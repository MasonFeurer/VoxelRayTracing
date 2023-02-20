use crate::aabb::Aabb;
use crate::math::{Vec2f, Vec3i};
use crate::{vec2f, vec3f, vec3i};
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Voxel(pub u8);
impl Voxel {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const DIRT: Self = Self(2);
    pub const GRASS: Self = Self(3);
    pub const FIRE: Self = Self(4);
    pub const MAGMA: Self = Self(5);
    pub const WATER: Self = Self(6);
    pub const WOOD: Self = Self(7);
    pub const BARK: Self = Self(8);
    pub const LEAVES: Self = Self(9);
    pub const SAND: Self = Self(10);
    pub const MUD: Self = Self(11);
    pub const CLAY: Self = Self(12);
    pub const IRON: Self = Self(13);

    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self == Self::AIR || self == Self::WATER
    }
    #[inline(always)]
    pub fn is_solid(self) -> bool {
        self != Self::AIR && self != Self::WATER
    }

    pub fn display(self) -> &'static str {
        &VOXEL_DISPLAY_NAMES[self.0 as usize]
    }
}

#[rustfmt::skip]
static VOXEL_DISPLAY_NAMES: &[&str] = &[
    "air",
    "stone",
    "dirt",
    "grass",
    "fire",
    "magma",
    "water",
    "wood",
    "bark",
    "leaves",
    "sand",
    "mud",
    "clay",
    "iron",
];

pub const CHUNK_W: i32 = 32;
pub const CHUNK_H: i32 = 32;
pub const CHUNK_VOLUME: i32 = CHUNK_W * CHUNK_W * CHUNK_H; // 32768
pub const CHUNK_SIZE: Vec3i = vec3i!(CHUNK_W, CHUNK_H, CHUNK_W);

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Chunk {
    solid_voxels_count: u32,
    minmax: [u32; 6],
    voxels: [Voxel; CHUNK_VOLUME as usize],
}
impl Chunk {
    pub fn get_voxel(&self, pos: Vec3i) -> Voxel {
        let idx = (pos.x + pos.y * CHUNK_W + pos.z * CHUNK_W * CHUNK_H) as usize;
        self.voxels[idx]
    }
    pub fn set_voxel(&mut self, pos: Vec3i, voxel: Voxel) -> Option<usize> {
        if pos.x >= CHUNK_W || pos.y >= CHUNK_H || pos.z >= CHUNK_W {
            return None;
        }

        let idx = (pos.x + pos.y * CHUNK_W + pos.z * CHUNK_W * CHUNK_H) as usize;
        if self.voxels[idx] == voxel {
            return None;
        }
        match voxel.is_solid() {
            b if b == self.voxels[idx].is_solid() => {}
            false => self.solid_voxels_count -= 1,
            true => self.solid_voxels_count += 1,
        }
        self.voxels[idx] = voxel;
        Some(idx)
    }
}

pub const WORLD_W: i32 = 8;
pub const WORLD_H: i32 = 8;
pub const WORLD_CHUNKS_COUNT: i32 = WORLD_W * WORLD_W * WORLD_H;

pub struct VoxelChunkPos {
    pub chunk: Vec3i,
    pub in_chunk: Vec3i,
}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct World {
    pub min_chunk_pos: [u32; 3],
    pub chunks: [Chunk; WORLD_CHUNKS_COUNT as usize],
    _padding0: [u32; 1],
}
impl World {
    pub fn voxel_chunk_pos(&self, pos: Vec3i) -> Option<VoxelChunkPos> {
        if pos.x < 0 || pos.y < 0 || pos.z < 0 {
            return None;
        }
        let chunk = pos / CHUNK_SIZE;
        let in_chunk = pos % CHUNK_SIZE;
        Some(VoxelChunkPos { chunk, in_chunk })
    }

    pub fn get_voxel(&self, pos: Vec3i) -> Option<Voxel> {
        let pos = self.voxel_chunk_pos(pos)?;
        if pos.chunk.x >= WORLD_W || pos.chunk.y >= WORLD_H || pos.chunk.z >= WORLD_W {
            return None;
        }

        let chunk_idx =
            (pos.chunk.x + pos.chunk.y * WORLD_W + pos.chunk.z * WORLD_W * WORLD_H) as usize;
        Some(self.chunks[chunk_idx].get_voxel(pos.in_chunk))
    }
    pub fn set_voxel(&mut self, pos: Vec3i, voxel: Voxel) -> Option<(usize, usize)> {
        let pos = self.voxel_chunk_pos(pos)?;
        if pos.chunk.x >= WORLD_W || pos.chunk.y >= WORLD_W || pos.chunk.z >= WORLD_W {
            return None;
        }

        let chunk_idx =
            (pos.chunk.x + pos.chunk.y * WORLD_W + pos.chunk.z * WORLD_W * WORLD_H) as usize;
        Some((
            chunk_idx,
            self.chunks[chunk_idx].set_voxel(pos.in_chunk, voxel)?,
        ))
    }
    pub fn set_voxels(&mut self, min: Vec3i, max: Vec3i, voxel: Voxel) {
        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    self.set_voxel(vec3i!(x, y, z), voxel);
                }
            }
        }
    }

    pub fn surface_at(&self, x: i32, z: i32) -> i32 {
        for y in 0..WORLD_H {
            if self.get_voxel(vec3i!(x, y, z)).unwrap().is_empty() {
                return y;
            }
        }
        0
    }

    pub fn populate(&mut self) {
        let seed = fastrand::i64(..);
        let mut gen = WorldGen::new(seed);
        gen.populate([0, WORLD_W * CHUNK_W], [0, WORLD_W * CHUNK_W], self);
    }

    pub fn get_collisions_w(&self, aabb: &Aabb) -> Vec<Aabb> {
        let mut aabbs = Vec::new();

        let from = aabb.from.floor();
        let to = aabb.to.ceil();

        for x in from.x..to.x {
            for y in from.y..to.y {
                for z in from.z..to.z {
                    let pos = vec3i!(x, y, z);
                    let voxel = self.get_voxel(pos).unwrap_or(Voxel::AIR);

                    if !voxel.is_empty() {
                        let min = vec3f!(x as f32, y as f32, z as f32);
                        let max = min + 1.0;
                        aabbs.push(Aabb::new(min, max));
                    }
                }
            }
        }
        aabbs
    }
}

use crate::open_simplex::{init_gradients, MultiNoiseMap, NoiseMap};
pub struct WorldGen {
    pub seed: i64,

    pub height_map: MultiNoiseMap,
    pub height_scale_map: MultiNoiseMap,
    pub height_freq_map: MultiNoiseMap,
}
impl WorldGen {
    pub fn new(seed: i64) -> Self {
        init_gradients();
        let height_scale_map =
            MultiNoiseMap::new(&[NoiseMap::new(seed.wrapping_mul(47828974), 0.005, 2.0)]);
        let height_freq_map = MultiNoiseMap::new(&[
            NoiseMap::new(seed.wrapping_mul(479389189), 0.0003, 3.4),
            NoiseMap::new(seed.wrapping_mul(77277342), 0.0001, 4.4),
        ]);
        let height_map = MultiNoiseMap::new(&[
            NoiseMap::new(seed.wrapping_mul(2024118), 0.004, 200.0),
            NoiseMap::new(seed.wrapping_mul(55381728), 0.1, 6.0),
            NoiseMap::new(seed.wrapping_mul(8282442), 0.01, 20.0),
            NoiseMap::new(seed.wrapping_mul(7472824), 0.008, 100.0),
        ]);
        Self {
            seed,
            height_map,
            height_scale_map,
            height_freq_map,
        }
    }

    pub fn get_terrain_h(&self, pos: Vec2f) -> f32 {
        let height_scale = self.height_scale_map.get(pos);
        let height_freq = self.height_freq_map.get(pos);
        self.height_map.get(pos * height_freq) * height_scale
    }

    pub fn populate(&mut self, x: [i32; 2], z: [i32; 2], world: &mut World) {
        for x in x[0]..x[1] {
            for z in z[0]..z[1] {
                let y = self.get_terrain_h(vec2f!(x as f32, z as f32)) as i32;

                // set stone
                world.set_voxels(vec3i!(x, 0, z), vec3i!(x + 1, y - 3, z + 1), Voxel::STONE);

                // set dirt
                world.set_voxels(vec3i!(x, y - 3, z), vec3i!(x + 1, y, z + 1), Voxel::DIRT);

                let mut surface = Voxel::GRASS;
                if y < 30 {
                    world.set_voxels(vec3i!(x, y, z), vec3i!(x + 1, 31, z + 1), Voxel::WATER);
                    surface = Voxel::SAND;
                    if y <= 26 {
                        surface = Voxel::DIRT;
                    }
                }

                // set surface
                world.set_voxel(vec3i!(x, y, z), surface);
            }
        }
    }
}
