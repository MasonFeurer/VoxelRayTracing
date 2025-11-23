use crate::world::ServerChunk;
use common::math::rand_cardinal_dir;
use common::resources::{Biome, Feature, Source, WorldFeatures, WorldPreset};
use common::world::noise::{Map, MappedNoise, RawNoise};
use common::world::{inchunk_to_world_pos, Voxel, CHUNK_SIZE};
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
    *seed <<= 4 + *seed;
    *seed += 890189034;
    *seed *= 917834;
    *seed <<= 9;
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
    // (only usd in the constructor to generate the following noise generators)
    pub seed: i64,

    pub features: WorldFeatures,
    pub biomes: Vec<Biome>,
    pub biome_lookup: [[u32; 20]; 4],
    pub earth: Voxel,
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

            features: features,
            biomes: preset.biomes.clone(),
            biome_lookup: preset.biome_lookup.clone(),
            earth: preset.earth,
            height_map: value_gen_from_src(&preset.height, &mut seed),
            temp_map: value_gen_from_src(&preset.temp, &mut seed),
            humidity_map: value_gen_from_src(&preset.humidity, &mut seed),
            weird_map: value_gen_from_src(&preset.weirdness, &mut seed),
            vegetation: RawNoise::new(transmute_seed(&mut seed)),
            feat_map: MappedNoise::new(transmute_seed(&mut seed), Map::new(0.06, 1.0, 0.0)),
        }
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

    pub fn generate_chunk(
        &self,
        chunk_pos: IVec3,
        out_chunk: &mut ServerChunk,
        out_features: &mut Vec<BuiltFeature>,
    ) {
        for x in 0..CHUNK_SIZE {
            'a: for z in 0..CHUNK_SIZE {
                let world_pos = inchunk_to_world_pos(chunk_pos, uvec3(x, 0, z));

                let biome = self.biome_at(world_pos.x, world_pos.z).clone();
                let h = self.height_map.eval(world_pos.x as f32, world_pos.z as f32) as i32;

                let start_y = world_pos.y;
                let end_y = (world_pos.y + CHUNK_SIZE as i32).min(h + 1).max(start_y);
                for world_y in start_y..end_y {
                    let y_in_chunk = (world_y - world_pos.y) as u32;
                    let layer = h - world_y;

                    let vox = *biome.layers.get(layer as usize).unwrap_or(&self.earth);

                    _ = out_chunk.set_voxel(uvec3(x, y_in_chunk, z), vox);
                }
                if (h - world_pos.y < 0) || (h - world_pos.y >= 32) {
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
                if (fastrand::u32(0..=1000).max(50) as f32)
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
                out_features.push(build_feature(ivec3(world_pos.x, h, world_pos.z), feature));
            }
        }
    }
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
