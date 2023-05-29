pub mod open_simplex;

use crate::math::{aabb::Aabb, BitField};
use glam::{IVec3, Vec2, Vec3};
use open_simplex::{init_gradients, MultiNoiseMap, NoiseMap};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Voxel(pub u8);

impl Voxel {
    pub const AIR: u8 = 0;
    pub const STONE: u8 = 1;
    pub const DIRT: u8 = 2;
    pub const GRASS: u8 = 3;
    pub const FIRE: u8 = 4;
    pub const MAGMA: u8 = 5;
    pub const WATER: u8 = 6;
    pub const WOOD: u8 = 7;
    pub const BARK: u8 = 8;
    pub const LEAVES: u8 = 9;
    pub const SAND: u8 = 10;
    pub const MUD: u8 = 11;
    pub const CLAY: u8 = 12;
    pub const GOLD: u8 = 13;
    pub const MIRROR: u8 = 14;
    pub const BRIGHT: u8 = 15;
    pub const ORANGE_TILE: u8 = 16;
    pub const POLISHED_BLACK_TILES: u8 = 17;
    pub const SMOOTH_ROCK: u8 = 18;
    pub const WOOD_FLOORING: u8 = 19;
    pub const POLISHED_BLACK_FLOORING: u8 = 20;
}

impl Voxel {
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.0 == Self::AIR || self.0 == Self::WATER
    }
    #[inline(always)]
    pub fn is_solid(self) -> bool {
        self.0 != Self::AIR && self.0 != Voxel::WATER
    }

    pub fn display_name(self) -> &'static str {
        match self.0 {
            Self::AIR => "air",
            Self::STONE => "stone",
            Self::DIRT => "dirt",
            Self::GRASS => "grass",
            Self::FIRE => "fire",
            Self::MAGMA => "magma",
            Self::WATER => "water",
            Self::WOOD => "wood",
            Self::BARK => "bark",
            Self::LEAVES => "leaves",
            Self::SAND => "sand",
            Self::MUD => "mud",
            Self::CLAY => "clay",
            Self::GOLD => "gold",
            Self::MIRROR => "mirror",
            Self::BRIGHT => "bright",
            Self::ORANGE_TILE => "orange tile",
            Self::POLISHED_BLACK_TILES => "polished black tiles",
            Self::SMOOTH_ROCK => "smooth rock",
            Self::WOOD_FLOORING => "wood flooring",
            Self::POLISHED_BLACK_FLOORING => "polished black flooring",
            _ => "{unknown}",
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Material {
    pub color: [f32; 3],
    pub empty: u32,
    pub scatter: f32,
    pub emission: f32,
    pub polish_bounce_chance: f32,
    pub _padding0: u32,
    pub polish_color: [f32; 3],
    pub polish_scatter: f32,
}
impl Material {
    pub const ZERO: Self = Self {
        color: [0.0; 3],
        empty: 0,
        scatter: 0.0,
        emission: 0.0,
        polish_bounce_chance: 0.0,
        _padding0: 0,
        polish_color: [0.0; 3],
        polish_scatter: 0.0,
    };

    pub const fn new(
        empty: u32,
        color: [f32; 3],
        emission: f32,
        scatter: f32,
        polish_bounce_chance: f32,
        polish_color: [f32; 3],
        polish_scatter: f32,
    ) -> Self {
        Self {
            empty,
            color,
            emission,
            scatter,
            polish_bounce_chance,
            polish_color,
            polish_scatter,
            _padding0: 0,
        }
    }
}

#[rustfmt::skip]
pub static DEFAULT_VOXEL_MATERIALS: &[Material] = &[
    Material::new(1, [0.00, 0.00, 0.00], 0.0, 0.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.40, 0.40, 0.40], 0.0, 0.8, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.40, 0.20, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.10, 0.70, 0.10], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [1.00, 0.90, 0.20], 2.0, 0.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.75, 0.18, 0.01], 1.0, 1.0, 0.2, [1.0; 3], 0.0),
    Material::new(0, [0.00, 0.00, 1.00], 0.0, 0.0, 0.5, [1.0; 3], 0.0),
    Material::new(0, [0.00, 0.00, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.86, 0.85, 0.82], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.23, 0.52, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.99, 0.92, 0.53], 0.0, 0.9, 0.0, [1.0; 3], 0.0),
    Material::new(0, [0.22, 0.13, 0.02], 0.0, 0.8, 0.4, [1.0; 3], 0.0),
    Material::new(0, [0.35, 0.30, 0.25], 0.0, 0.8, 0.4, [1.0; 3], 0.0),
    Material::new(0, [0.83, 0.68, 0.22], 0.0, 0.3, 0.0, [1.0; 3], 0.0),
    Material::new(0, [1.00, 1.00, 1.00], 0.0, 0.0, 0.0, [1.0; 3], 0.0),
    // BRIGHT
    Material::new(0, [1.00, 1.00, 1.00], 5.0, 1.0, 0.0, [1.0; 3], 0.0),
    // ORANGE_TILE
    Material::new(0, [0.87, 0.41, 0.01], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // POLISHED_BLACK_TILES
    Material::new(0, [0.10, 0.10, 0.10], 0.0, 0.1, 0.9, [1.0; 3], 0.0),
    // SMOOTH_ROCK
    Material::new(0, [0.60, 0.60, 0.60], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // WOOD_FLOORING
    Material::new(0, [1.00, 0.00, 1.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // POLISHED_BLACK_FLOORING
    Material::new(0, [0.07, 0.07, 0.07], 0.0, 0.1, 0.8, [1.0; 3], 0.0),
];

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Node {
    data: BitField,
    first_child: u32,
}
impl Node {
    pub fn new(voxel: Voxel, first_child: u32, is_split: bool) -> Self {
        let mut rs = Self {
            data: BitField::ZERO,
            first_child,
        };
        rs.set_split_flag(is_split);
        rs.set_voxel(voxel);
        rs
    }

    #[inline(always)]
    pub fn get_voxel(self) -> Voxel {
        Voxel(self.data.get(8, 8) as u8)
    }
    #[inline(always)]
    pub fn set_voxel(&mut self, voxel: Voxel) {
        self.data.set(voxel.0 as u32, 8, 8)
    }

    #[inline(always)]
    pub fn set_free_flag(&mut self, f: bool) {
        self.data.set(f as u32, 1, 1)
    }
    #[inline(always)]
    pub fn is_free(self) -> bool {
        self.data.get(1, 1) == 1
    }

    #[inline(always)]
    pub fn set_split_flag(&mut self, f: bool) {
        self.data.set(f as u32, 1, 0)
    }
    #[inline(always)]
    pub fn is_split(self) -> bool {
        self.data.get(1, 0) == 1
    }

    #[inline(always)]
    pub fn get_child(self, idx: u32) -> u32 {
        self.first_child + idx
    }

    pub fn can_simplify(self, world: &World) -> bool {
        // If this node isn't split, this node can't be simplified
        if !self.is_split() {
            return false;
        }
        // If any childeren are split, this node can't be simplified
        for idx in 0..8 {
            if world.get_node(self.get_child(idx)).is_split() {
                return false;
            }
        }
        // If any childeren are not the same voxel, this node can't be simplified
        for child_idx in 0..7 {
            if world.get_node(self.get_child(child_idx)).get_voxel()
                != world.get_node(self.get_child(child_idx + 1)).get_voxel()
            {
                return false;
            }
        }
        // Otherwise, this node can be simplified
        true
    }

    pub fn split(&mut self, first_child: u32) {
        self.set_split_flag(true);
        self.first_child = first_child;
        self.set_voxel(Voxel(Voxel::MAGMA));
    }

    pub fn simplify(&mut self, result: Voxel) {
        self.set_split_flag(false);
        self.set_voxel(result);
    }
}

pub trait WorldPopulator {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) -> Result<(), ()>;
}

const MAX_NODES: usize = 300_000_000;

#[derive(Clone)]
#[repr(C)]
pub struct World {
    pub root_idx: u32,
    pub size: u32,
    pub max_depth: u32,
    pub start_search: u32,
    pub nodes: [Node; MAX_NODES],
}

/// Create and clear worlds
impl World {
    pub fn new_boxed(max_depth: u32) -> Box<Self> {
        let mut world = unsafe { Box::<World>::new_zeroed().assume_init() };
        world.set_max_depth(max_depth);
        world.clear();
        world
    }

    pub fn set_max_depth(&mut self, max_depth: u32) {
        self.max_depth = max_depth;
        self.size = 1 << max_depth;
    }

    pub fn clear(&mut self) {
        self.root_idx = 0;
        self.start_search = 1;
        self.nodes[0] = Node::new(Voxel(Voxel::AIR), 0, false);
        for node in &mut self.nodes[1..] {
            node.set_free_flag(true);
        }
    }
}

struct FoundNode {
    idx: u32,
    depth: u32,
}

/// Find and mutate the SVO nodes that make up this world.
impl World {
    fn find_node(&self, pos: IVec3, max_depth: u32) -> FoundNode {
        let mut center = IVec3::splat(self.size as i32 / 2);
        let mut size = self.size;
        let mut node_idx = self.root_idx;
        let mut depth: u32 = 0;

        loop {
            let node = self.get_node(node_idx);
            if !node.is_split() || depth == max_depth {
                return FoundNode {
                    idx: node_idx,
                    depth,
                };
            }
            size /= 2;

            let gt = IVec3::new(
                (pos.x >= center.x) as i32,
                (pos.y >= center.y) as i32,
                (pos.z >= center.z) as i32,
            );
            let child_idx = (gt.x as u32) << 0 | (gt.y as u32) << 1 | (gt.z as u32) << 2;
            node_idx = node.get_child(child_idx);
            let child_dir = gt * 2 - IVec3::ONE;
            center += IVec3::splat(size as i32 / 2) * child_dir;
            depth += 1;
        }
    }

    pub fn count_nodes(&self) -> u32 {
        fn count_node(world: &World, node: u32) -> u32 {
            let node = world.get_node(node);
            if !node.is_split() {
                return 1;
            }
            count_node(world, node.get_child(0))
                + count_node(world, node.get_child(1))
                + count_node(world, node.get_child(2))
                + count_node(world, node.get_child(3))
                + count_node(world, node.get_child(4))
                + count_node(world, node.get_child(5))
                + count_node(world, node.get_child(6))
                + count_node(world, node.get_child(7))
        }
        count_node(self, self.root_idx)
    }

    fn get_node(&self, idx: u32) -> Node {
        self.nodes[idx as usize]
    }

    fn mut_node(&mut self, idx: u32) -> &mut Node {
        &mut self.nodes[idx as usize]
    }

    fn free_nodes(&mut self, start: u32) {
        if start < self.start_search {
            self.start_search = start;
        }
        for idx in start..start + 8 {
            self.nodes[idx as usize].set_free_flag(true);
        }
    }

    fn new_nodes(&mut self, voxel: Voxel) -> Result<u32, ()> {
        let mut result = self.start_search;
        if result + 8 >= self.nodes.len() as u32 {
            return Err(());
        }

        while !self.get_node(result).is_free() {
            result += 8;
            if result + 8 >= self.nodes.len() as u32 {
                return Err(());
            }
        }
        self.start_search = result + 8;

        for idx in result..result + 8 {
            self.nodes[idx as usize] = Node::new(voxel, 0, false);
        }
        Ok(result)
    }
}

/// High-level voxel-based manipulation.
impl World {
    pub fn get_voxel(&self, pos: IVec3) -> Option<Voxel> {
        let FoundNode { idx, .. } = self.find_node(pos, self.max_depth);
        Some(self.get_node(idx).get_voxel())
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<(), ()> {
        let FoundNode { idx, depth, .. } = self.find_node(pos, self.max_depth);
        let node = self.get_node(idx);

        if node.get_voxel() == voxel {
            return Ok(());
        }
        if depth == self.max_depth {
            self.mut_node(idx).set_voxel(voxel);

            let mut parent_depth = depth - 1;
            let mut parent_idx = self.find_node(pos, parent_depth).idx;

            while self.get_node(parent_idx).can_simplify(self) {
                let first_child = self.get_node(parent_idx).get_child(0);
                let reduce_to = self.get_node(first_child).get_voxel();
                self.mut_node(parent_idx).simplify(reduce_to);
                self.free_nodes(first_child);

                parent_depth -= 1;
                parent_idx = self.find_node(pos, parent_depth).idx;
            }
            return Ok(());
        }
        let Ok(new_first_child) = self.new_nodes(node.get_voxel()) else {
            return Err(());
        };

        self.mut_node(idx).split(new_first_child);
        self.set_voxel(pos, voxel)
    }

    pub fn set_voxels(&mut self, min: IVec3, max: IVec3, voxel: Voxel) -> Result<(), ()> {
        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    self.set_voxel(IVec3 { x, y, z }, voxel)?;
                }
            }
        }
        Ok(())
    }

    pub fn surface_at(&self, x: i32, z: i32) -> i32 {
        for y in 0..self.size as i32 {
            if self.get_voxel(IVec3 { x, y, z }).unwrap().is_empty() {
                return y;
            }
        }
        0
    }

    pub fn populate_with<P: WorldPopulator>(&mut self, p: &P) -> Result<(), ()> {
        let min = IVec3::ZERO;
        let max = IVec3::splat(self.size as i32);
        p.populate(min, max, self)
    }

    pub fn get_collisions_w(&self, aabb: &Aabb) -> Vec<Aabb> {
        let mut aabbs = Vec::new();

        let from = aabb.from.floor().as_ivec3();
        let to = aabb.to.ceil().as_ivec3();

        for x in from.x..to.x {
            for y in from.y..to.y {
                for z in from.z..to.z {
                    let pos = IVec3 { x, y, z };

                    let voxel = self.get_voxel(pos).unwrap_or(Voxel(Voxel::AIR));

                    if !voxel.is_empty() {
                        let min = pos.as_vec3();
                        let max = min + 1.0;
                        aabbs.push(Aabb::new(min, max));
                    }
                }
            }
        }
        aabbs
    }
}

pub struct DebugWorldGen;
impl WorldPopulator for DebugWorldGen {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) -> Result<(), ()> {
        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in 0..3 {
                    world.set_voxel(IVec3 { x, y, z }, Voxel(Voxel::STONE))?;
                }
            }
        }
        for x in min.x..max.x {
            for z in min.z..max.z {
                for y in 0..3 {
                    world.set_voxel(IVec3 { x, y, z }, Voxel(Voxel::DIRT))?;
                }
            }
        }
        Ok(())
    }
}

pub struct DefaultWorldGen {
    pub seed: i64,

    pub height_map: MultiNoiseMap,
    pub height_scale_map: MultiNoiseMap,
    pub height_freq_map: MultiNoiseMap,
    pub scale: f32,
    pub freq: f32,
}
impl DefaultWorldGen {
    pub fn new(seed: i64, scale: f32, freq: f32) -> Self {
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
            scale,
            freq,
        }
    }

    fn get_terrain_h(&self, pos: Vec2) -> f32 {
        let height_scale = self.height_scale_map.get(pos) * self.scale;
        let height_freq = self.height_freq_map.get(pos) * self.freq;
        self.height_map.get(pos * height_freq) * height_scale
    }

    fn spawn_tree(&self, world: &mut World, surface: IVec3) -> Result<(), ()> {
        let h = fastrand::u32(6..14) as i32;
        for i in 0..h {
            world.set_voxel(surface + IVec3::new(0, i, 0), Voxel(Voxel::BARK))?;
        }
        self.sphere(
            world,
            surface + IVec3::new(0, h as i32, 0),
            4,
            Voxel(Voxel::LEAVES),
        )
    }

    fn sphere(&self, world: &mut World, pos: IVec3, r: u32, voxel: Voxel) -> Result<(), ()> {
        let pos_center = pos.as_vec3() + Vec3::splat(0.5);
        let min = pos - IVec3::splat(r as i32);
        let max = pos + IVec3::splat(r as i32);
        let r_sq = r as f32 * r as f32;

        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    let block_center = IVec3 { x, y, z }.as_vec3() + Vec3::splat(0.5);
                    if (block_center - pos_center).length_squared() >= r_sq {
                        continue;
                    }
                    world.set_voxel(IVec3 { x, y, z }, voxel)?;
                }
            }
        }
        Ok(())
    }
}
impl WorldPopulator for DefaultWorldGen {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) -> Result<(), ()> {
        for x in min.x..max.x {
            for z in min.z..max.z {
                let noise_pos = Vec2::new(x as f32, z as f32) + Vec2::splat(0.5);

                let y = self.get_terrain_h(noise_pos) as i32;
                let surface_pos = IVec3 { x, y, z };

                // set stone
                world.set_voxels(
                    IVec3::new(x, 0, z),
                    IVec3::new(x + 1, y - 3, z + 1),
                    Voxel(Voxel::STONE),
                )?;

                // set dirt
                world.set_voxels(
                    IVec3::new(x, y - 3, z),
                    IVec3::new(x + 1, y, z + 1),
                    Voxel(Voxel::DIRT),
                )?;

                // set surface
                world.set_voxel(surface_pos, Voxel(Voxel::GRASS))?;

                if fastrand::u32(0..300) == 0 {
                    self.spawn_tree(world, surface_pos)?;
                }
            }
        }
        Ok(())
    }
}
