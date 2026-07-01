use crate::world::ServerChunk;
use common::math::rand_cardinal_dir;
use common::resources::{Biome, Feature, Source, WorldFeatures, WorldPreset};
use common::world::noise::{Map, MappedNoise, RawNoise};
use common::world::{ChunkPos, Node, NodeAlloc, Svo, Voxel, VoxelPos, VoxelPosInChunk, CHUNK_DEPTH, CHUNK_SIZE};
use glam::{ivec3, uvec3, vec2, IVec3, Vec3};
use std::collections::HashMap;

fn randf32(range: std::ops::Range<f32>) -> f32 {
    let size = range.end - range.start;
    fastrand::f32() * size + range.start
}

enum ValueGen {
    Constant(f32),
    Noise(MappedNoise),
    ComplexNoise {
        freq: MappedNoise,
        scale: MappedNoise,
        base: MappedNoise,
        layers: Vec<MappedNoise>,
    },
}
impl ValueGen {
    #[inline(always)]
    fn eval(&self, x: f32, z: f32) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Noise(noise) => noise.sample(vec2(x, z)),
            Self::ComplexNoise {
                freq,
                scale,
                base,
                layers,
            } => {
                let freq = freq.sample(vec2(x, z));
                let scale = scale.sample(vec2(x, z));
                let mut height = base.sample(vec2(x, z) * freq) * scale;
                for layer in layers {
                    height += layer.sample(vec2(x, z));
                }
                height
            }
        }
    }
}

fn transmute_seed(seed: &mut i64) -> i64 {
    *seed = seed.wrapping_add(890189034);
    *seed = seed.wrapping_mul(917834);
    *seed <<= 1;
    *seed = seed.wrapping_add(6478912);
    *seed = seed.wrapping_mul(0891247);
    *seed
}
fn map_from_src(src: &Map, seed: &mut i64) -> MappedNoise {
    MappedNoise::new(transmute_seed(seed), *src)
}
fn value_gen_from_src(src: &Source, seed: &mut i64) -> ValueGen {
    match src {
        Source::Value(v) => ValueGen::Constant(*v),
        Source::Noise(noise) => ValueGen::Noise(map_from_src(noise, seed)),
        Source::ComplexNoise {
            freq,
            scale,
            base,
            layers,
        } => ValueGen::ComplexNoise {
            freq: map_from_src(freq, seed),
            scale: map_from_src(scale, seed),
            base: map_from_src(base, seed),
            layers: layers
                .into_iter()
                .map(|src| map_from_src(src, seed))
                .collect(),
        },
    }
}

pub struct WorldGen {
    // (only used in the constructor to generate the following noise generators)
    pub seed: i64,

    pub features: WorldFeatures,
    pub biomes: Vec<Biome>,
    pub biome_lookup: [[u32; 20]; 8],
    pub earth: Voxel,
    pub water: Voxel,
    pub sea_level: i32,
    height_map: ValueGen,
    temp_map: ValueGen,
    humidity_map: ValueGen,
    weird_map: ValueGen,
    vegetation: RawNoise,
    feat_map: MappedNoise,
}
impl WorldGen {
    pub fn new(preset: &WorldPreset, features: WorldFeatures, mut seed: i64) -> Self {
        let seed_copy = seed;
        Self {
            seed: seed_copy,

            features,
            biomes: preset.biomes.clone(),
            biome_lookup: preset.biome_lookup.clone(),
            earth: preset.earth,
            water: preset.water,
            sea_level: preset.sea_level,
            height_map: value_gen_from_src(&preset.height, &mut seed),
            temp_map: value_gen_from_src(&preset.temp, &mut seed),
            humidity_map: value_gen_from_src(&preset.humidity, &mut seed),
            weird_map: value_gen_from_src(&preset.weirdness, &mut seed),
            vegetation: RawNoise::new(transmute_seed(&mut seed)),
            feat_map: MappedNoise::new(transmute_seed(&mut seed), Map::new(0.15, 1.0, 0.0)),
        }
    }

    #[inline(always)]
    pub fn terrain_h_at(&self, x: i32, z: i32) -> i32 {
        self.height_map.eval(x as f32, z as f32) as i32
    }
    
    pub fn find_land_near(&self, x: i32, z: i32) -> Option<VoxelPos> {
        let search_gap = 10;
        let search_steps =  100;
        for x in (x - search_steps)..(x + search_steps) {
            for z in (z - search_steps)..(z + search_steps) {
                let xx = x * search_gap;
                let zz = z * search_gap;
                
                let h = self.terrain_h_at(xx, zz);
                if h > self.sea_level {
                    return Some(VoxelPos(xx, h, zz));
                }
            }
        }
        None
    }

    pub fn biome_at(&self, x: i32, z: i32) -> &Biome {
        // 0.0f..1.0f
        let temp = self.temp_map.eval(x as f32, z as f32);
        // 0.0f..1.0f
        let humidity = self.humidity_map.eval(x as f32, z as f32);
        // 0.0f..1.0f
        let weird = self.weird_map.eval(x as f32, z as f32);

        let temp_idx = ((temp * 20.0).floor() as usize).min(19);
        let weird_idx = (weird.round() as usize).min(1) * 4;
        let humidity_idx = ((humidity * 4.0).floor() as usize).min(3);
        let biome_idx = self.biome_lookup[humidity_idx + weird_idx][temp_idx];
        &self.biomes[biome_idx as usize]
    }

    // SANITIZATION: `buffer` should be `MAX_NODES` in length, and be all zeros.
    // We use an inputed buffer to avoid allocating a node list on the heap
    // every time a chunk needs to be generated. Only nodes that are allocated
    // on the heap are the ones actually needed to represent the final chunk.
    pub fn generate_chunk(
        &self,
        buffer: &mut [Node],
        chunk_pos: ChunkPos,
        out_features: &mut Vec<BuiltFeature>,
    ) -> ServerChunk {
        let mut node_alloc = NodeAlloc::new(0..1, 1..buffer.len() as u32);

        {
            let (min, max) = (chunk_pos.min(), chunk_pos.max());
            let surface_samples = [
                // sample the world surface at the 4 corners and the center of the chunk.
                self.height_map.eval(min.x as f32, min.z as f32),
                self.height_map.eval(max.x as f32, min.z as f32),
                self.height_map.eval(min.x as f32, max.z as f32),
                self.height_map.eval(max.x as f32, max.z as f32),
                self.height_map.eval(
                    min.x as f32 + CHUNK_SIZE as f32 * 0.5,
                    min.z as f32 + CHUNK_SIZE as f32 * 0.5,
                ),
            ];
            // get the min
            let surface_min = surface_samples.into_iter().reduce(f32::min).unwrap();
            // if the lowest surface value is above this chunk ceiling, we can assume most-if-not-all
            // of this chunk will be composed of the `earth` voxel (`stone` in the default gen).
            // Because of this assumption, we can change the chunk to be a single node consisting of the
            // `earth` voxel. This saves the permormance cost of writing `earth` to most of the chunk
            // starting from Air.
            if surface_min > max.y as f32 {
                buffer[0] = Node::new(self.earth);
            }
        }

        for x in 0..CHUNK_SIZE {
            'a: for z in 0..CHUNK_SIZE {
                let world_pos = VoxelPosInChunk(x, 0, z).unwrap().global(chunk_pos);

                let biome = self.biome_at(world_pos.x, world_pos.z).clone();
                let h = self.height_map.eval(world_pos.x as f32, world_pos.z as f32) as i32;

                let start_y = world_pos.y;
                let end_y = (world_pos.y + CHUNK_SIZE as i32).min(h + 1).max(start_y);
                for world_y in start_y..end_y {
                    let y_in_chunk = (world_y - world_pos.y) as u32;
                    let layer = h - world_y;

                    let vox = *biome.layers.get(layer as usize).unwrap_or(&self.earth);

                    _ = Svo::new(0, CHUNK_SIZE).set_node(
                        buffer,
                        uvec3(x, y_in_chunk, z),
                        vox,
                        CHUNK_DEPTH,
                        &mut node_alloc,
                    );
                }
                for world_y in end_y..self.sea_level.min(world_pos.y + CHUNK_SIZE as i32) {
                    let y_in_chunk = (world_y - world_pos.y) as u32;
                    _ = Svo::new(0, CHUNK_SIZE).set_node(
                        buffer,
                        uvec3(x, y_in_chunk, z),
                        self.water,
                        CHUNK_DEPTH,
                        &mut node_alloc,
                    );
                }

                if (h - world_pos.y < 0) || (h - world_pos.y >= CHUNK_SIZE as i32) || h < self.sea_level {
                    continue;
                }

                // ---- Vegetation/Features ----
                // ### determine if this is a peak in the noise map
                let get_veg = |x: i32, z: i32| self.feat_map.sample(vec2(x as f32, z as f32));

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
                // Remove an increasing number of features as `self.vegetation` produces lower results.
                if (fastrand::u32(0..=1000) as f32)
                    >= self
                        .vegetation
                        .map_sample(vec2(x as f32, z as f32), &biome.vegetation)
                        * 1000.0
                {
                    continue 'a;
                }

                // ### this is a peak, so place a feature here
                // randomly choose one of the features in the biome:
                let Some(feature) = fastrand::choice(&biome.features) else {
                    continue 'a;
                };
                let feature = self.features.get(feature).unwrap().clone();
                out_features.push(build_feature(VoxelPos(world_pos.x, h, world_pos.z), feature));
            }
        }
        let used_voxels = node_alloc.last_used_addr() + 64;
        let nodes = buffer[0..=used_voxels as usize].to_vec();
        node_alloc.move_end(used_voxels);

        ServerChunk { nodes, node_alloc }
    }
}

#[derive(Clone)]
pub struct BuiltFeature {
    voxels: HashMap<VoxelPos, Voxel>,
    bounds: (VoxelPos, VoxelPos),
}
impl BuiltFeature {
    pub fn new() -> Self {
        Self {
            voxels: HashMap::new(),
            bounds: (VoxelPos::new(IVec3::MAX), VoxelPos::new(IVec3::MIN)),
        }
    }

    #[inline(always)] pub fn min(&self) -> VoxelPos { self.bounds.0 }
    #[inline(always)] pub fn max(&self) -> VoxelPos { self.bounds.1 }

    pub fn voxel_placements<'a>(&'a self) -> impl Iterator<Item = (VoxelPos, Voxel)> + 'a {
        self.voxels.iter().map(|(pos, vox)| (*pos, *vox))
    }
    pub fn into_voxel_placements(self) -> impl Iterator<Item = (VoxelPos, Voxel)> {
        self.voxels.into_iter()
    }

    pub fn set_voxel(&mut self, pos: VoxelPos, v: Voxel) {
        self.voxels.insert(pos, v);
        self.bounds.0 = VoxelPos::new(self.bounds.0.min(*pos));
        self.bounds.1 = VoxelPos::new(self.bounds.1.max(*pos));
    }

    pub fn place_line(&mut self, start: VoxelPos, end: VoxelPos, v: Voxel) {
        for pos in common::math::walk_line(*start, *end) {
            self.set_voxel(VoxelPos::new(pos), v);
        }
    }

    pub fn place_sphere(&mut self, center: VoxelPos, r: u32, v: Voxel) {
        let pos_center = center.as_vec3() + Vec3::splat(0.5);
        let min = *center - IVec3::splat(r as i32);
        let max = *center + IVec3::splat(r as i32);
        let r_sq = r as f32 * r as f32;

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq {
                        continue;
                    }
                    self.set_voxel(VoxelPos(x, y, z), v);
                }
            }
        }
    }

    pub fn place_disc(&mut self, center: VoxelPos, r: f32, height: u32, v: Voxel) {
        let pos_center = center.as_vec3() + Vec3::splat(0.5);
        let min = *center - ivec3(r as i32, 0, r as i32);
        let max = *center + ivec3(r as i32, height as i32 - 1, r as i32);
        let r_sq = r * r;

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq {
                        continue;
                    }
                    self.set_voxel(VoxelPos(x, y, z), v);
                }
            }
        }
    }
}

pub fn build_feature(surface: VoxelPos, feature: Feature) -> BuiltFeature {
    let mut out = BuiltFeature::new();
    match feature {
        Feature::Tree {
            trunk_voxel,
            branch_voxel,
            leaf_voxel,
            height,
            leaf_decay: _,
            branch_count,
            branch_height,
            branch_len,
        } => {
            let height = fastrand::u32(height);
            let top = VoxelPos::new(*surface + ivec3(0, height as i32, 0));

            let branch_count = match height {
                ..=8 => 0,
                _ => fastrand::u32(branch_count),
            };
            out.place_sphere(top, 5, leaf_voxel);

            for _ in 0..branch_count {
                let branch_h = (randf32(branch_height.clone()) * height as f32) as u32;
                let branch_len = fastrand::u32(branch_len.clone());

                let branch_dir = common::math::rand_hem_dir(Vec3::Y);
                let start = VoxelPos(surface.x, surface.y + branch_h as i32, surface.z);
                let end = VoxelPos::new((start.as_vec3() + branch_dir * branch_len as f32).as_ivec3());

                out.place_sphere(end, 3, leaf_voxel);
                out.place_line(start, end, branch_voxel);
            }
            out.place_line(surface, top, trunk_voxel);
        }
        Feature::CanopyTree {
            trunk_voxel,
            leaf_voxel,
            height,
            slope_offset: _,
        } => {
            // TODO: add slant
            let r = fastrand::u32(5..11) as f32 - 0.1;

            let height = fastrand::u32(height);
            let top = VoxelPos::new(*surface + ivec3(0, height as i32, 0));

            out.place_line(surface, top, trunk_voxel);
            out.place_disc(top, r, 1, leaf_voxel);

            let branch_count = fastrand::u32(1..4);
            for _ in 0..branch_count {
                let branch_h = fastrand::u32(4..height);
                let branch_len = fastrand::u32(3..6);

                let branch_dir = common::math::rand_hem_dir(Vec3::Y);
                let start = VoxelPos::new(ivec3(surface.x, surface.y + branch_h as i32, surface.z));
                let end = VoxelPos::new((start.as_vec3() + branch_dir * branch_len as f32).as_ivec3());

                out.place_line(start, end, trunk_voxel);
                out.place_disc(end, 4.0, 1, leaf_voxel);
            }
        }
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
                let c = VoxelPos::new(*surface + IVec3::Y * y);
                out.place_disc(c, r as f32 - 0.1, 1, leaf_voxel);
                r += 1;
                y -= 2;
            }
            let top = VoxelPos::new(*surface + ivec3(0, height - 1, 0));
            out.place_line(surface, top, trunk_voxel);
        }
        Feature::Cactus { voxel, height } => {
            let pos = VoxelPos::new(*surface + IVec3::Y);
            let height = fastrand::u32(height) as i32;
            let splits = if height > 3 { fastrand::u32(0..4) } else { 0 };

            out.place_line(pos, VoxelPos::new(*pos + IVec3::Y * height), voxel);
            for _ in 0..splits {
                let split_h = fastrand::i32(1..height);
                let split_len = fastrand::i32(1..4);
                let dir = rand_cardinal_dir();

                out.set_voxel(VoxelPos::new(*pos + IVec3::Y * split_h + dir), voxel);
                let branch_min = VoxelPos::new(*pos + IVec3::Y * split_h + dir * 2);
                let branch_max = VoxelPos::new(*branch_min + IVec3::Y * split_len);
                out.place_line(branch_min, branch_max, voxel);
            }
        }
        Feature::Spike {
            voxel,
            height,
            width,
        } => {
            let height = fastrand::u32(height) as i32;
            let width = fastrand::u32(width);
            for y in 0..height {
                let delta = 1.0 - (y as f32 / height as f32);
                let w = (delta * width as f32).floor() as u32;
                out.place_disc(VoxelPos::new(*surface + IVec3::Y * y), (w as f32 * 0.5) - 0.1, 1, voxel);
            }
        }
        Feature::Lake { voxel, size, depth } => {
            println!("TODO: lake");
        }
    }
    out
}
