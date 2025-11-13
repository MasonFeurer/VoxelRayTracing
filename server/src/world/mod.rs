use common::math::rand_cardinal_dir;
use common::resources::{Biome, Feature, Source, WorldFeatures, WorldPreset};
use common::world::noise::NoiseMap;
use common::world::{
    inchunk_to_world_pos, set_svo_voxel, world_to_chunk_pos, world_to_inchunk_pos, Node, NodeAlloc,
    SetVoxelErr, SvoMut, Voxel, CHUNK_DEPTH, CHUNK_SIZE,
};
use glam::{ivec3, uvec3, vec2, IVec3, Vec3};
use std::collections::{HashMap, HashSet};

enum WorldValue {
    Constant(f32),
    Noise(NoiseMap),
    ComplexNoise {
        freq: NoiseMap,
        scale: NoiseMap,
        base: NoiseMap,
        layers: Vec<NoiseMap>,
    },
}
impl WorldValue {
    fn eval(&self, x: f32, z: f32) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Noise(noise) => noise.get(vec2(x, z)),
            Self::ComplexNoise {
                freq,
                scale,
                base,
                layers,
            } => {
                let freq = freq.get(vec2(x, z));
                let scale = scale.get(vec2(x, z));
                let mut height = base.get(vec2(x, z) * freq) * scale;
                for layer in layers {
                    height += layer.get(vec2(x, z));
                }
                height
            }
        }
    }
}

pub struct ServerWorld {
    pub chunks: HashMap<IVec3, ServerChunk>,
    features: WorldFeatures,

    biomes: Vec<Biome>,
    biome_lookup: [[u32; 20]; 4],
    earth: Voxel,
    height_map: WorldValue,
    temp_map: WorldValue,
    humidity_map: WorldValue,
    weird_map: WorldValue,

    feat_map: NoiseMap,
    unplaced_features: Vec<BuiltFeature>,
}
impl ServerWorld {
    pub fn new(preset: &WorldPreset, features: WorldFeatures) -> Self {
        let noise_from_src = |src: &common::resources::Noise| -> NoiseMap {
            NoiseMap::new(
                fastrand::i64(..),
                src.freq as f64,
                src.scale as f64,
                src.offset as f64,
            )
        };
        let create_map = |src: &Source| match src {
            Source::Value(v) => WorldValue::Constant(*v),
            Source::Noise(noise) => WorldValue::Noise(noise_from_src(noise)),
            Source::ComplexNoise {
                freq,
                scale,
                base,
                layers,
            } => WorldValue::ComplexNoise {
                freq: noise_from_src(freq),
                scale: noise_from_src(scale),
                base: noise_from_src(base),
                layers: layers.into_iter().map(noise_from_src).collect(),
            },
        };

        Self {
            chunks: HashMap::new(),
            features,

            biomes: preset.biomes.clone(),
            biome_lookup: preset.biome_lookup.clone(),
            earth: preset.earth,
            height_map: create_map(&preset.height),
            temp_map: create_map(&preset.temp),
            humidity_map: create_map(&preset.humidity),
            weird_map: create_map(&preset.weirdness),

            feat_map: NoiseMap::new(fastrand::i64(..), 0.1, 1.0, 0.0),
            unplaced_features: Vec::new(),
        }
    }

    pub fn place_features(&mut self) -> Vec<IVec3> {
        let mut out = HashSet::new();

        'f: for feature_idx in (0..self.unplaced_features.len()).rev() {
            let feature = &self.unplaced_features[feature_idx];

            let min_chunk = world_to_chunk_pos(feature.min());
            let max_chunk = world_to_chunk_pos(feature.max());

            // make sure all chunks covered by the feature exist before placing.
            for x in min_chunk.x..=max_chunk.x {
                for y in min_chunk.y..=max_chunk.y {
                    for z in min_chunk.z..=max_chunk.z {
                        if self.get_chunk(ivec3(x, y, z)).is_none() {
                            continue 'f;
                        }
                    }
                }
            }

            for (pos, voxel) in feature.voxel_placements() {
                match set_voxel_w_chunks(&mut self.chunks, pos, voxel) {
                    Ok(()) => _ = out.insert(world_to_chunk_pos(pos)),
                    Err(err) => {
                        eprintln!(
                            "Failed to place voxel for feature {feature_idx} at {pos:?} : {err:?}"
                        );
                        continue 'f;
                    }
                }
            }
            _ = self.unplaced_features.remove(feature_idx);
        }
        out.into_iter().collect()
    }

    pub fn biome_at(&self, x: i32, z: i32) -> &Biome {
        // 0.0f..1.0f
        let temp = self.temp_map.eval(x as f32, z as f32);
        // 0.0f..1.0f
        let humidity = self.humidity_map.eval(x as f32, z as f32);

        let temp_idx = ((temp * 20.0).floor() as usize).min(19);
        let humidity_idx = ((humidity * 4.0).floor() as usize).min(3);
        let biome_idx = self.biome_lookup[humidity_idx][temp_idx];
        &self.biomes[biome_idx as usize]
    }

    pub fn get_chunk(&self, pos: IVec3) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<(), SetVoxelErr> {
        set_voxel_w_chunks(&mut self.chunks, pos, voxel)
    }
}

pub fn set_voxel_w_chunks(
    chunks: &mut HashMap<IVec3, ServerChunk>,
    pos: IVec3,
    voxel: Voxel,
) -> Result<(), SetVoxelErr> {
    let chunk_pos = world_to_chunk_pos(pos);
    let pos_in_chunk = world_to_inchunk_pos(pos);
    let chunk = chunks
        .get_mut(&chunk_pos)
        .ok_or(SetVoxelErr::PosOutOfBounds)?;
    let mut svo = SvoMut {
        nodes: &mut chunk.nodes,
        root: 0,
        size: CHUNK_SIZE,
    };
    set_svo_voxel(
        &mut svo,
        pos_in_chunk,
        voxel,
        CHUNK_DEPTH,
        &mut chunk.node_alloc,
    )
}

#[derive(Clone)]
pub struct BuiltFeature {
    voxels: HashMap<IVec3, Voxel>,
    bounds: (IVec3, IVec3),
}
impl BuiltFeature {
    pub fn new() -> Self {
        Self {
            voxels: HashMap::new(),
            bounds: (IVec3::MAX, IVec3::MIN),
        }
    }

    pub fn min(&self) -> IVec3 {
        self.bounds.0
    }
    pub fn max(&self) -> IVec3 {
        self.bounds.1
    }

    pub fn voxel_placements<'a>(&'a self) -> impl Iterator<Item = (IVec3, Voxel)> + 'a {
        self.voxels.iter().map(|(pos, vox)| (*pos, *vox))
    }
    pub fn into_voxel_placements(self) -> impl Iterator<Item = (IVec3, Voxel)> {
        self.voxels.into_iter()
    }

    pub fn set_voxel(&mut self, pos: IVec3, v: Voxel) {
        self.voxels.insert(pos, v);
        self.bounds.0 = self.bounds.0.min(pos);
        self.bounds.1 = self.bounds.1.max(pos);
    }

    pub fn place_line(&mut self, start: IVec3, end: IVec3, v: Voxel) {
        for pos in common::math::walk_line(start, end) {
            self.set_voxel(pos, v);
        }
    }

    pub fn place_sphere(&mut self, center: IVec3, r: u32, v: Voxel) {
        let pos_center = center.as_vec3() + Vec3::splat(0.5);
        let min = center - IVec3::splat(r as i32);
        let max = center + IVec3::splat(r as i32);
        let r_sq = r as f32 * r as f32;

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq {
                        continue;
                    }
                    self.set_voxel(ivec3(x, y, z), v);
                }
            }
        }
    }

    pub fn place_disc(&mut self, center: IVec3, r: u32, height: u32, v: Voxel) {
        let pos_center = center.as_vec3() + Vec3::splat(0.5);
        let min = center - ivec3(r as i32, 0, r as i32);
        let max = center + ivec3(r as i32, height as i32 - 1, r as i32);
        let r_sq = r as f32 * r as f32;

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq {
                        continue;
                    }
                    self.set_voxel(ivec3(x, y, z), v);
                }
            }
        }
    }
}

fn randf32(range: std::ops::Range<f32>) -> f32 {
    let size = range.end - range.start;
    fastrand::f32() * size + range.start
}

impl ServerWorld {
    pub fn build_feature(surface: IVec3, feature: Feature) -> BuiltFeature {
        let mut out = BuiltFeature::new();
        match feature {
            Feature::Tree {
                trunk_voxel,
                branch_voxel,
                leaf_voxel,
                height,
                leaf_decay,
                branch_count,
                branch_height,
                branch_len,
            } => {
                let height = fastrand::u32(height);
                let top = surface + ivec3(0, height as i32, 0);

                let branch_count = match height {
                    ..=8 => 0,
                    _ => fastrand::u32(branch_count),
                };
                out.place_sphere(top, 5, leaf_voxel);

                for _ in 0..branch_count {
                    let branch_h = (randf32(branch_height.clone()) * height as f32) as u32;
                    let branch_len = fastrand::u32(branch_len.clone());

                    let branch_dir = common::math::rand_hem_dir(Vec3::Y);
                    let start = ivec3(surface.x, surface.y + branch_h as i32, surface.z);
                    let end = (start.as_vec3() + branch_dir * branch_len as f32).as_ivec3();

                    out.place_sphere(end, 3, leaf_voxel);
                    out.place_line(start, end, trunk_voxel);
                }
                out.place_line(surface, top, trunk_voxel);
            }
            Feature::CanopyTree {
                trunk_voxel,
                leaf_voxel,
                height,
                slope_offset,
            } => {}
            Feature::Evergreen {
                trunk_voxel,
                leaf_voxel,
                height,
                bottom_branch,
            } => {
                let offset = fastrand::u32(bottom_branch) as i32;
                let height = offset + fastrand::u32(height) as i32;

                let mut y = height;
                let mut r: i32 = 1;
                while y > offset {
                    let c = surface + IVec3::Y * y;
                    out.place_disc(c, r as u32, 1, leaf_voxel);
                    r += 1;
                    y -= 2;
                }
                let top = surface + ivec3(0, height - 1, 0);
                out.place_line(surface, top, trunk_voxel);
            }
            Feature::Cactus { voxel, height } => {
                let pos = surface + IVec3::Y;
                let height = fastrand::u32(height) as i32;
                let splits = if height > 3 { fastrand::u32(0..4) } else { 0 };

                out.place_line(pos, pos + IVec3::Y * height, voxel);
                for _ in 0..splits {
                    let split_h = fastrand::i32(1..height);
                    let split_len = fastrand::i32(1..4);
                    let dir = rand_cardinal_dir();

                    out.set_voxel(pos + IVec3::Y * split_h + dir, voxel);
                    let branch_min = pos + IVec3::Y * split_h + dir * 2;
                    let branch_max = branch_min + IVec3::Y * split_len;
                    out.place_line(branch_min, branch_max, voxel);
                }
            }
            Feature::Spike {
                voxel,
                height,
                width,
            } => {}
        }
        out
    }

    pub fn create_chunk(&mut self, chunk_pos: IVec3) {
        let mut chunk = ServerChunk::with_capacity((CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) / 2);
        let alloc = &mut chunk.node_alloc;
        let mut svo = SvoMut {
            nodes: &mut chunk.nodes,
            root: 0,
            size: CHUNK_SIZE,
        };

        for x in 0..CHUNK_SIZE {
            'a: for z in 0..CHUNK_SIZE {
                let world_pos = inchunk_to_world_pos(chunk_pos, uvec3(x, 0, z));

                let biome = self.biome_at(world_pos.x, world_pos.z);
                let h = self.height_map.eval(world_pos.x as f32, world_pos.z as f32) as i32;

                let start_y = world_pos.y;
                let end_y = (world_pos.y + CHUNK_SIZE as i32).min(h + 1).max(start_y);
                for world_y in start_y..end_y {
                    let y_in_chunk = (world_y - world_pos.y) as u32;
                    let layer = h - world_y;

                    let vox = *biome.layers.get(layer as usize).unwrap_or(&self.earth);

                    _ = set_svo_voxel(&mut svo, uvec3(x, y_in_chunk, z), vox, CHUNK_DEPTH, alloc);
                }
                if (h - world_pos.y < 0) || (h - world_pos.y >= 32) {
                    continue;
                }
                let surface_y = (h - world_pos.y) as u32;

                // ---- Vegetation/Features ----
                // ### determine if this is a peak in the noise map
                let get_veg = |x: i32, z: i32| self.feat_map.get(vec2(x as f32, z as f32));

                let veg = get_veg(world_pos.x, world_pos.z);
                let veg_adj = [
                    get_veg(world_pos.x + 1, world_pos.z),
                    get_veg(world_pos.x - 1, world_pos.z),
                    get_veg(world_pos.x, world_pos.z + 1),
                    get_veg(world_pos.x, world_pos.z - 1),
                    get_veg(world_pos.x + 1, world_pos.z + 1),
                    get_veg(world_pos.x - 1, world_pos.z + 1),
                    get_veg(world_pos.x - 1, world_pos.z - 1),
                    get_veg(world_pos.x + 1, world_pos.z - 1),
                ];
                for veg_adj in veg_adj {
                    if veg_adj >= veg {
                        continue 'a;
                    }
                }

                // ### this is a peak, so place a feature here
                // randomly choose one of the features in the biome:
                let Some(feature) = fastrand::choice(&biome.features) else {
                    continue 'a;
                };
                // println!("Placing feature {feature:?} at {world_pos:?}");
                let feature = self.features.get(feature).unwrap().clone();

                let built_feature =
                    Self::build_feature(ivec3(world_pos.x, h, world_pos.z), feature);
                self.unplaced_features.push(built_feature);
            }
        }

        self.chunks.insert(chunk_pos, chunk);
    }
}

pub struct ServerChunk {
    pub nodes: Vec<Node>,
    pub node_alloc: NodeAlloc,
}
impl std::fmt::Debug for ServerChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Chunk<'a> {
            #[allow(dead_code)] // used by  #derive Debug
            nodes: &'a [Node],
        }
        let c = Chunk {
            nodes: self.used_nodes(),
        };
        std::fmt::Debug::fmt(&c, f)
    }
}
impl ServerChunk {
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    pub fn with_capacity(cap: u32) -> Self {
        let mut nodes = vec![Node::ZERO; cap as usize];
        nodes[0] = Node::new(Voxel::from_data(0));
        Self {
            nodes,
            node_alloc: NodeAlloc::new(0..1, 1..cap),
        }
    }

    pub fn used_nodes(&self) -> &[Node] {
        &self.nodes[0..=self.node_alloc.last_used_addr() as usize]
    }
}
