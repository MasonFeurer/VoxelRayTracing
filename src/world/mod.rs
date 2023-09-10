pub mod open_simplex;

use crate::math::{aabb::Aabb, BitField};
use glam::{ivec3, vec2, vec3, IVec3, Vec3};
use open_simplex::NoiseMap;
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq)]
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
    pub const GREEN_LEAVES: Self = Self(9);
    pub const PINK_LEAVES: Self = Self(21);
    pub const ORANGE_LEAVES: Self = Self(22);
    pub const YELLOW_LEAVES: Self = Self(23);
    pub const RED_LEAVES: Self = Self(24);
    pub const SAND: Self = Self(10);
    pub const MUD: Self = Self(11);
    pub const CLAY: Self = Self(12);
    pub const GOLD: Self = Self(13);
    pub const MIRROR: Self = Self(14);
    pub const BRIGHT: Self = Self(15);
    pub const ORANGE_TILE: Self = Self(16);
    pub const POLISHED_BLACK_TILES: Self = Self(17);
    pub const SMOOTH_ROCK: Self = Self(18);
    pub const WOOD_FLOORING: Self = Self(19);
    pub const POLISHED_BLACK_FLOORING: Self = Self(20);

    pub const ALL_LEAVES: &[Self] = &[
        Self::GREEN_LEAVES,
        Self::PINK_LEAVES,
        Self::ORANGE_LEAVES,
        Self::YELLOW_LEAVES,
        Self::RED_LEAVES,
    ];
}

impl Voxel {
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self == Self::AIR || self == Self::WATER
    }
    #[inline(always)]
    pub fn is_solid(self) -> bool {
        self != Self::AIR && self != Voxel::WATER
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::AIR => "air",
            Self::STONE => "stone",
            Self::DIRT => "dirt",
            Self::GRASS => "grass",
            Self::FIRE => "fire",
            Self::MAGMA => "magma",
            Self::WATER => "water",
            Self::WOOD => "wood",
            Self::BARK => "bark",
            Self::GREEN_LEAVES => "green leaves",
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
            Self::PINK_LEAVES => "pink leaves",
            Self::ORANGE_LEAVES => "orange leaves",
            Self::YELLOW_LEAVES => "yellow leaves",
            Self::RED_LEAVES => "red leaves",
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
    // AIR
    Material::new(1, [0.00, 0.00, 0.00], 0.0, 0.0, 0.0, [1.0; 3], 0.0),
    // STONE
    Material::new(0, [0.40, 0.40, 0.40], 0.0, 0.8, 0.0, [1.0; 3], 0.0),
    // DIRT
    Material::new(0, [0.40, 0.20, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // GRASS
    Material::new(0, [0.011, 0.58, 0.11], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // FIRE
    Material::new(0, [1.00, 0.90, 0.20], 2.0, 0.0, 0.0, [1.0; 3], 0.0),
    // MAGMA
    Material::new(0, [0.75, 0.18, 0.01], 1.0, 1.0, 0.2, [1.0; 3], 0.0),
    // WATER
    Material::new(0, [0.076, 0.563, 0.563], 0.0, 0.0, 0.5, [1.0; 3], 0.0),
    // WOOD
    Material::new(0, [0.00, 0.00, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // BARK
    Material::new(0, [1.00, 1.00, 1.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // GREEN_LEAVES
    Material::new(0, [0.23, 0.52, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // SAND
    Material::new(0, [0.99, 0.92, 0.53], 0.0, 0.9, 0.0, [1.0; 3], 0.0),
    // MUD
    Material::new(0, [0.22, 0.13, 0.02], 0.0, 0.8, 0.4, [1.0; 3], 0.0),
    // CLAY
    Material::new(0, [0.35, 0.30, 0.25], 0.0, 0.8, 0.4, [1.0; 3], 0.0),
    // GOLD
    Material::new(0, [0.83, 0.68, 0.22], 0.0, 0.3, 0.0, [1.0; 3], 0.0),
    // MIRROR
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
    // PINK_LEAVES
    Material::new(0, [0.95, 0.45, 0.60], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // ORANGE_LEAVES
    Material::new(0, [0.95, 0.20, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // YELLOW_LEAVES
    Material::new(0, [1.00, 0.92, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // RED_LEAVES
    Material::new(0, [0.95, 0.10, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
];

/// Represents a node in the sparse voxel octree (SVO) that is the world.
///
/// ## Underlying Implementation
/// There are a lot of nodes in a world,
/// so I've tried to make them use as little memory as I could.
/// Each node consumes 4 bytes of memory, a single 32-bit integer.
/// Here are the different states of the bits:
///
/// ```
/// 00______________________________
/// ```
/// Node is not used.
///
/// ```
/// 10______________________________
/// ```
/// Invalid state.
///
/// ```
/// 01______________________xxxxxxxx
/// ```
/// Node is a single voxel where x = voxel type.
///
/// ```
/// 11xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
/// ```
/// Node is split into 8 nodes of half size where x points to first child.
/// All 8 child nodes will be sequential in memory so only the position of the first one is needed.
/// NOTE: the index of the first child will always be one more than a multiple of 8,
/// so x actually represrents `(child_index - 1) / 8`.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Node(BitField);
impl Node {
    pub const ZERO: Self = Self(BitField::ZERO);

    pub fn new_leaf(voxel: Voxel) -> Self {
        let mut rs = Self::ZERO;
        rs.set_voxel(voxel);
        rs.set_used_flag(true);
        rs
    }

    pub fn new_split(first_child: u32) -> Self {
        let mut rs = Self::ZERO;
        rs.set_first_child(first_child);
        rs.set_split_flag(true);
        rs.set_used_flag(true);
        rs
    }

    #[inline(always)]
    pub fn set_used_flag(&mut self, f: bool) {
        self.0.set(f as u32, 1, 30)
    }
    #[inline(always)]
    pub fn is_used(self) -> bool {
        self.0.get(1, 30) == 1
    }

    #[inline(always)]
    pub fn set_split_flag(&mut self, f: bool) {
        self.0.set(f as u32, 1, 31)
    }
    #[inline(always)]
    pub fn is_split(self) -> bool {
        self.0.get(1, 31) == 1
    }

    #[inline(always)]
    pub fn get_voxel(self) -> Voxel {
        Voxel(self.0.get(8, 0) as u8)
    }
    #[inline(always)]
    pub fn set_voxel(&mut self, voxel: Voxel) {
        self.0.set(voxel.0 as u32, 8, 0)
    }

    #[inline(always)]
    pub fn set_first_child(&mut self, first_child: u32) {
        debug_assert!(first_child == 0 || ((first_child - 1) % 8) == 0);
        let first_child = (first_child - 1) / 8; // TODO replace with bitshift

        self.0.set(first_child, 30, 0)
    }
    #[inline(always)]
    pub fn first_child(self) -> u32 {
        self.0.get(30, 0) * 8 + 1
    }

    #[inline(always)]
    pub fn get_child(self, idx: u32) -> u32 {
        self.first_child() + idx
    }

    /// Tests if this is a node with childeren that can be simplified into a node representing a single voxel,
    /// requiring all child nodes to represent the same voxel type.
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
        let child_voxel = world.get_node(self.get_child(0)).get_voxel();
        for child in 1..8 {
            if world.get_node(self.get_child(child)).get_voxel() != child_voxel {
                return false;
            }
        }
        // Otherwise, this node can be simplified
        true
    }

    pub fn split(&mut self, first_child: u32) {
        self.set_split_flag(true);
        self.set_first_child(first_child);
    }

    /// Call if `Self::can_simplify` returns `true`.
    pub fn simplify(&mut self, result: Voxel) {
        self.set_split_flag(false);
        self.set_voxel(result);
    }
}

#[derive(Clone, Debug)]
pub enum WorldErr {
    OutOfBounds,
}

/// The structure that holds the entire interactable world, representing all voxels via a SVO.
#[derive(Clone, Default)]
pub struct World {
    pub size: u32,
    pub max_depth: u32,
    start_search: u32,
    last_used_node: u32,

    // Note: Removing items from the Vec is not good since
    // some nodes may point to other nodes by index.
    nodes: Vec<Node>,
}

/// Create and clear worlds
impl World {
    pub fn new(max_depth: u32, alloc_nodes: u32) -> Self {
        let mut nodes = vec![Node::ZERO; alloc_nodes as usize];
        nodes[0] = Node::new_leaf(Voxel::AIR);
        Self {
            size: 1 << max_depth,
            max_depth,
            start_search: 1,
            last_used_node: 0,
            nodes,
        }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes[0..=self.last_used_node as usize]
    }

    pub fn set_max_depth(&mut self, max_depth: u32) {
        self.max_depth = max_depth;
        self.size = 1 << max_depth;
    }

    pub fn clear(&mut self) {
        for node in &mut self.nodes {
            node.set_used_flag(false);
        }
        self.nodes[0] = Node::new_leaf(Voxel::AIR);
        self.start_search = 1;
        self.last_used_node = 0;
    }
}

struct FoundNode {
    idx: u32,
    depth: u32,
    center: IVec3,
    size: u32,
}

/// Find and mutate the SVO nodes that make up the world.
impl World {
    pub fn check_bounds(&self, pos: IVec3) -> Result<(), WorldErr> {
        let in_bounds =
            (pos.cmpge(IVec3::ZERO)).all() && (pos.cmplt(IVec3::splat(self.size as i32))).all();
        in_bounds.then(|| ()).ok_or(WorldErr::OutOfBounds)
    }

    fn find_node(&self, pos: IVec3, max_depth: u32) -> Result<FoundNode, WorldErr> {
        self.check_bounds(pos)?;

        let mut center = IVec3::splat(self.size as i32 / 2);
        let mut size = self.size;
        let mut node_idx = 0;
        let mut depth: u32 = 0;

        loop {
            let node = self.get_node(node_idx);
            if !node.is_split() || depth == max_depth {
                return Ok(FoundNode {
                    idx: node_idx,
                    depth,
                    center,
                    size,
                });
            }
            size /= 2;

            let gt = ivec3(
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

    #[inline(always)]
    pub fn get_node(&self, idx: u32) -> Node {
        self.nodes[idx as usize]
    }

    #[inline(always)]
    fn mut_node(&mut self, idx: u32) -> &mut Node {
        &mut self.nodes[idx as usize]
    }

    fn free_nodes(&mut self, start: u32) {
        if start < self.start_search {
            self.start_search = start;
        }
        for idx in start..start + 8 {
            self.nodes[idx as usize].set_used_flag(false);
        }
    }

    fn new_nodes(&mut self, voxel: Voxel) -> u32 {
        static NEW_NODES: [Node; 1024] = [Node::ZERO; 1024];

        let mut result = self.start_search;
        if result + 7 >= self.nodes.len() as u32 {
            // result can at most be nodes.len(),
            // so adding 8 more nodes should make result valid,
            // but I add 1024 nodes here because it's very likely
            // to want to allocate more nodes very soon after
            self.nodes.extend(&NEW_NODES);
        }

        while self.get_node(result).is_used() {
            result += 8;
            if result + 7 >= self.nodes.len() as u32 {
                self.nodes.extend(&NEW_NODES);
            }
        }
        self.start_search = result + 8;

        for idx in result..result + 8 {
            self.nodes[idx as usize] = Node::new_leaf(voxel);
        }
        if result > self.last_used_node.saturating_sub(7) {
            self.last_used_node = result + 7;
        }
        result
    }
}

#[derive(Clone, Copy)]
pub struct NodeSeq {
    pub idx: u32,
    pub count: u8,
}

/// High-level voxel-based manipulation.
impl World {
    pub fn get_voxel(&self, pos: IVec3) -> Result<Voxel, WorldErr> {
        let FoundNode { idx, .. } = self.find_node(pos, self.max_depth)?;
        Ok(self.get_node(idx).get_voxel())
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<Vec<NodeSeq>, WorldErr> {
        let target_depth = self.max_depth;
        let FoundNode {
            mut idx,
            depth,
            mut center,
            mut size,
            ..
        } = self.find_node(pos, target_depth)?;
        let old_voxel = self.get_node(idx).get_voxel();

        let mut result: Vec<NodeSeq> = vec![];
        result.push(NodeSeq { idx, count: 1 });

        // If depth is less than target_depth,
        // the SVO doesn't go to desired depth, so we must split until it does
        for _ in depth..target_depth {
            let first_child = self.new_nodes(old_voxel);

            self.mut_node(idx).set_split_flag(true);
            self.mut_node(idx).set_first_child(first_child);
            result.push(NodeSeq {
                idx: first_child,
                count: 8,
            });

            size /= 2;

            let gt = ivec3(
                (pos.x >= center.x) as i32,
                (pos.y >= center.y) as i32,
                (pos.z >= center.z) as i32,
            );
            let child_idx = (gt.x as u32) << 0 | (gt.y as u32) << 1 | (gt.z as u32) << 2;
            idx = first_child + child_idx;
            let child_dir = gt * 2 - IVec3::ONE;
            center += IVec3::splat(size as i32 / 2) * child_dir;
        }
        // SVO now goes to desired depth, so we can mutate the node now.
        self.mut_node(idx).set_voxel(voxel);
        self.mut_node(idx).set_split_flag(false);
        Ok(result)
    }

    pub fn fill_voxels(&mut self, a: IVec3, b: IVec3, voxel: Voxel) {
        let min = ivec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = ivec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    _ = self.set_voxel(ivec3(x, y, z), voxel);
                }
            }
        }
    }

    pub fn surface_at(&self, x: i32, z: i32) -> Result<i32, WorldErr> {
        for y in 0..self.size as i32 {
            if self.get_voxel(ivec3(x, y, z))?.is_empty() {
                return Ok(y);
            }
        }
        Err(WorldErr::OutOfBounds)
    }

    pub fn get_collisions_w(&self, aabb: &Aabb) -> Vec<Aabb> {
        let mut aabbs = Vec::new();

        let from = aabb.from.floor().as_ivec3();
        let to = aabb.to.ceil().as_ivec3();

        for x in from.x..to.x {
            for y in from.y..to.y {
                for z in from.z..to.z {
                    let pos = ivec3(x, y, z);

                    let voxel = self.get_voxel(pos).unwrap_or(Voxel::AIR);

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

    pub fn sphere(&mut self, pos: IVec3, r: u32, voxel: Voxel, decay: f32) {
        let pos_center = pos.as_vec3() + Vec3::splat(0.5);
        let min = pos - IVec3::splat(r as i32);
        let max = pos + IVec3::splat(r as i32);
        let r_sq = r as f32 * r as f32;

        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq || fastrand::f32() <= decay {
                        continue;
                    }

                    _ = self.set_voxel(ivec3(x, y, z), voxel);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct NoiseMaps {
    height: NoiseMap,
    freq: NoiseMap,
    scale: NoiseMap,
    bumps: NoiseMap,
    mountains: NoiseMap,
    temp: NoiseMap,
    moisture: NoiseMap,
    vegetation: NoiseMap,
}
impl NoiseMaps {
    pub fn from_seed(seed: i64) -> Self {
        Self {
            height: NoiseMap::new(seed.wrapping_mul(4326742), 0.003, 2.5),
            freq: NoiseMap::new(seed.wrapping_mul(927144), 0.0001, 7.0),
            scale: NoiseMap::new(seed.wrapping_mul(43265), 0.003, 40.0),
            bumps: NoiseMap::new(seed.wrapping_mul(76324), 0.15, 4.0),
            mountains: NoiseMap::new(seed.wrapping_mul(72316423), 0.001, 40.0),
            temp: NoiseMap::new(seed.wrapping_mul(83226), 0.001, 1.0),
            moisture: NoiseMap::new(seed.wrapping_mul(2345632), 0.0001, 1.0),
            vegetation: NoiseMap::new(seed.wrapping_mul(53252), 0.001, 1.0),
        }
    }

    pub fn terrain_height(&self, x: f32, z: f32) -> f32 {
        let freq = self.freq.get(vec2(x, z));
        let scale = self.scale.get(vec2(x, z));
        self.height.get(vec2(x * freq, z * freq)) * scale
            + self.bumps.get(vec2(x, z))
            + self.mountains.get(vec2(x, z))
    }

    pub fn temp(&self, x: f32, z: f32) -> f32 {
        self.temp.get(vec2(x, z))
    }

    pub fn moisture(&self, x: f32, z: f32) -> f32 {
        self.moisture.get(vec2(x, z))
    }

    pub fn vegetation(&self, x: f32, z: f32) -> f32 {
        self.vegetation.get(vec2(x, z))
    }
}

pub struct WorldGen {
    pub maps: NoiseMaps,
    pub tree_gen: TreeGen,
}
impl WorldGen {
    pub fn new(seed: i64, tree_gen: TreeGen) -> Self {
        let maps = NoiseMaps::from_seed(seed);
        Self { maps, tree_gen }
    }

    pub fn populate(&self, min: IVec3, max: IVec3, world: &mut World) {
        for x in min.x..max.x {
            for z in min.z..max.z {
                let y = self.maps.terrain_height(x as f32, z as f32) as i32;
                let surface_pos = ivec3(x, y, z);

                world.fill_voxels(ivec3(x, 0, z), ivec3(x, y - 4, z), Voxel::STONE);
                world.fill_voxels(ivec3(x, y - 3, z), ivec3(x, y - 1, z), Voxel::DIRT);

                if y < 26 {
                    _ = world.set_voxel(surface_pos, Voxel::SAND);
                    world.fill_voxels(surface_pos + IVec3::Y, ivec3(x, 26, z), Voxel::WATER);
                    continue;
                }

                // let temp = self.maps.temp(x, z);
                let moisture = self.maps.moisture(x as f32, z as f32);
                let vegetation = self.maps.vegetation(x as f32, z as f32);

                let surface = if moisture < 0.2 || y < 27 {
                    Voxel::SAND
                } else {
                    Voxel::GRASS
                };
                _ = world.set_voxel(surface_pos, surface);

                if y < 26 {
                    world.fill_voxels(surface_pos + IVec3::Y, ivec3(x, 26, z), Voxel::WATER);
                    continue;
                }

                if fastrand::f32() < 0.005 * vegetation {
                    spawn_tree(world, surface_pos, &self.tree_gen);
                }
            }
        }
    }
}

pub struct TreeGen {
    pub height: Range<u32>,
    pub bark: Voxel,
    pub leaves: Vec<Voxel>,
    pub leaves_decay: f32,
    pub branch_count: Range<u32>,
    pub branch_height: Range<f32>,
    pub branch_len: Range<f32>,
}

fn spawn_tree(world: &mut World, surface: IVec3, tree: &TreeGen) {
    let height = fastrand::u32(tree.height.clone());
    let leaves = tree.leaves[fastrand::usize(0..tree.leaves.len())];
    let randf32 = |range: Range<f32>| -> f32 {
        let size = range.end - range.start;
        fastrand::f32() * size + range.start
    };

    // only create branches if the tree is tall
    let branch_count = if height < 11 {
        0
    } else {
        fastrand::u32(tree.branch_count.clone())
    };

    world.sphere(
        surface + ivec3(0, height as i32, 0),
        5,
        leaves,
        tree.leaves_decay,
    );

    for _ in 0..branch_count {
        let branch_h = (randf32(tree.branch_height.clone()) * height as f32) as u32;
        let branch_len = randf32(tree.branch_len.clone());

        let branch_dir = rand_hem_dir(Vec3::Y);
        let start = ivec3(surface.x, surface.y + branch_h as i32, surface.z);
        let end = (start.as_vec3() + branch_dir * branch_len).as_ivec3();

        world.sphere(end, 3, leaves, tree.leaves_decay);

        let line = crate::math::walk_line(start, end);
        for pos in line {
            _ = world.set_voxel(pos, tree.bark);
        }
    }

    for i in 0..height as i32 {
        _ = world.set_voxel(surface + ivec3(0, i, 0), tree.bark);
    }
}

pub fn rand_cardinal_dir() -> IVec3 {
    [
        ivec3(-1, 0, 0),
        ivec3(1, 0, 0),
        ivec3(0, 0, -1),
        ivec3(0, 0, 1),
    ][fastrand::usize(0..4)]
}

pub fn rand_dir() -> Vec3 {
    fn rand_norm() -> f32 {
        let theta = 2.0 * 3.14159265 * fastrand::f32();
        let rho = (-2.0 * fastrand::f32().ln()).sqrt();
        return rho * theta.cos();
    }

    let x = rand_norm();
    let y = rand_norm();
    let z = rand_norm();
    vec3(x, y, z).normalize()
}

pub fn rand_hem_dir(norm: Vec3) -> Vec3 {
    let dir = rand_dir();
    dir * norm.dot(dir).signum()
}
