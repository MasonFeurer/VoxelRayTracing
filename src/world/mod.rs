pub mod open_simplex;

use crate::math::{aabb::Aabb, BitField};
use glam::{ivec3, vec2, vec3, IVec3, Vec2, Vec3};
use open_simplex::{init_gradients, MultiNoiseMap, NoiseMap};

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
    Material::new(0, [0.10, 0.70, 0.10], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // FIRE
    Material::new(0, [1.00, 0.90, 0.20], 2.0, 0.0, 0.0, [1.0; 3], 0.0),
    // MAGMA
    Material::new(0, [0.75, 0.18, 0.01], 1.0, 1.0, 0.2, [1.0; 3], 0.0),
    // WATER
    Material::new(0, [0.00, 0.00, 1.00], 0.0, 0.0, 0.5, [1.0; 3], 0.0),
    // WOOD
    Material::new(0, [0.00, 0.00, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // BARK
    Material::new(0, [0.86, 0.85, 0.82], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
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
    Material::new(0, [0.90, 0.40, 0.40], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // ORANGE_LEAVES
    Material::new(0, [0.95, 0.20, 0.00], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
    // YELLOW_LEAVES
    Material::new(0, [0.90, 0.90, 0.10], 0.0, 1.0, 0.0, [1.0; 3], 0.0),
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

pub trait WorldPopulator {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) -> Result<(), ()>;
}

/// The structure that holds the entire interactable world, representing all voxels via a SVO.
#[derive(Clone, Default)]
pub struct World {
    pub size: u32,
    max_depth: u32,
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
        self.nodes.clear();
        self.nodes.push(Node::new_leaf(Voxel::AIR));
        self.start_search = 1;
    }
}

struct FoundNode {
    idx: u32,
    depth: u32,
}

/// Find and mutate the SVO nodes that make up the world.
impl World {
    fn find_node(&self, pos: IVec3, max_depth: u32) -> FoundNode {
        let mut center = IVec3::splat(self.size as i32 / 2);
        let mut size = self.size;
        let mut node_idx = 0;
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
    fn get_node(&self, idx: u32) -> Node {
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

    fn new_nodes(&mut self, voxel: Voxel) -> Result<u32, ()> {
        static NEW_NODES: [Node; 1024] = [Node::ZERO; 1024];

        let mut result = self.start_search;
        if result + 8 >= self.nodes.len() as u32 {
            // result can at most be nodes.len(),
            // so adding 8 more nodes should make result valid,
            // but I add 1024 nodes here because it's very likely
            // to want to allocate more nodes very soon after
            self.nodes.extend(&NEW_NODES);
        }

        while self.get_node(result).is_used() {
            result += 8;
            if result + 8 >= self.nodes.len() as u32 {
                self.nodes.extend(&NEW_NODES);
            }
        }
        self.start_search = result + 8;

        for idx in result..result + 8 {
            self.nodes[idx as usize] = Node::new_leaf(voxel);
        }
        if result - 8 > self.last_used_node {
            self.last_used_node = result + 8;
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

    pub fn fill_voxels(&mut self, a: IVec3, b: IVec3, voxel: Voxel) -> Result<(), ()> {
        let min = ivec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = ivec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    self.set_voxel(ivec3(x, y, z), voxel)?;
                }
            }
        }
        Ok(())
    }

    /// Similar to `fill_voxels`, but the voxels are all placed on top of the surface of the world.
    pub fn lay_voxels(&mut self, a: IVec3, b: IVec3, voxel: Voxel) -> Result<(), ()> {
        let min = ivec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = ivec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

        let h = max.y - min.y + 1;
        // println!("min: {min:?} max: {max:?} h: {h}");

        for x in min.x..=max.x {
            for z in min.z..=max.z {
                // self.set_voxel(ivec3(x, min.y + 5, z), Voxel::MUD)?;
                let surface = self.surface_at(x, z);
                // println!("surface at {x} {z} = {surface}");
                for y in 0..h {
                    let pos = ivec3(x, surface + y, z);
                    self.set_voxel(pos, voxel)?;
                }
            }
        }
        Ok(())
    }

    pub fn surface_at(&self, x: i32, z: i32) -> i32 {
        for y in 5..self.size as i32 {
            if self.get_voxel(ivec3(x, y, z)).unwrap().is_empty() {
                return y;
            }
        }
        69
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
}

pub struct DebugWorldGen;
impl WorldPopulator for DebugWorldGen {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) -> Result<(), ()> {
        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in 0..3 {
                    world.set_voxel(ivec3(x, y, z), Voxel::STONE)?;
                }
            }
        }
        for x in min.x..max.x {
            for z in min.z..max.z {
                for y in 0..3 {
                    world.set_voxel(ivec3(x, y, z), Voxel::DIRT)?;
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
    pub tree_freq: f32,
    pub tree_height: [u32; 2],
    pub tree_decay: f32,
}
impl DefaultWorldGen {
    pub fn clone_w_seed(&self, seed: i64) -> Self {
        Self::new(
            seed,
            self.scale,
            self.freq,
            self.tree_freq,
            self.tree_height,
            self.tree_decay,
        )
    }

    pub fn new(
        seed: i64,
        scale: f32,
        freq: f32,
        tree_freq: f32,
        tree_height: [u32; 2],
        tree_decay: f32,
    ) -> Self {
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
            tree_freq,
            tree_height,
            tree_decay,
        }
    }

    fn get_terrain_h(&self, pos: Vec2) -> f32 {
        let height_scale = self.height_scale_map.get(pos) * self.scale;
        let height_freq = self.height_freq_map.get(pos) * self.freq;
        self.height_map.get(pos * height_freq) * height_scale + 10.0
    }

    fn spawn_tree(&self, world: &mut World, surface: IVec3) -> Result<(), ()> {
        let h = fastrand::u32(self.tree_height[0]..self.tree_height[1]) as i32;
        let leaves = Voxel::ALL_LEAVES[fastrand::usize(0..Voxel::ALL_LEAVES.len())];
        self.sphere(world, surface + ivec3(0, h as i32, 0), 5, leaves)?;

        // only create a branch if the tree is tall
        let branch_count = if h < 11 { 0 } else { fastrand::i32(0..4) };

        for _ in 0..branch_count {
            // create a branch
            let branch_h = h - fastrand::i32(5..(h - 3));
            let branch_len = fastrand::u32(8..24) as f32 * 0.5;

            let branch_dir = rand_hem_dir(Vec3::Y);
            let start = ivec3(surface.x, surface.y + branch_h, surface.z);
            let end = (start.as_vec3() + branch_dir * branch_len).as_ivec3();

            self.sphere(world, end, 3, leaves)?;

            let line = crate::math::walk_line(start, end);
            for pos in line {
                world.set_voxel(pos, Voxel::BARK)?;
            }
        }

        for i in 0..h {
            world.set_voxel(surface + ivec3(0, i, 0), Voxel::BARK)?;
        }

        Ok(())
    }

    fn sphere(&self, world: &mut World, pos: IVec3, r: u32, voxel: Voxel) -> Result<(), ()> {
        let pos_center = pos.as_vec3() + Vec3::splat(0.5);
        let min = pos - IVec3::splat(r as i32);
        let max = pos + IVec3::splat(r as i32);
        let r_sq = r as f32 * r as f32;

        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq {
                        continue;
                    }

                    let decay_chance = dist_sq / (r_sq * self.tree_decay);
                    if fastrand::f32() <= decay_chance {
                        continue;
                    }

                    world.set_voxel(ivec3(x, y, z), voxel)?;
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
                let noise_pos = vec2(x as f32, z as f32) + Vec2::splat(0.5);

                let y = self.get_terrain_h(noise_pos) as i32;
                let surface_pos = ivec3(x, y, z);

                // set stone
                world.fill_voxels(ivec3(x, 0, z), ivec3(x, y - 4, z), Voxel::STONE)?;

                // set dirt
                world.fill_voxels(ivec3(x, y - 3, z), ivec3(x, y - 1, z), Voxel::DIRT)?;

                // set surface
                world.set_voxel(surface_pos, Voxel::GRASS)?;

                if fastrand::f32() < self.tree_freq {
                    self.spawn_tree(world, surface_pos)?;
                }
            }
        }
        Ok(())
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
