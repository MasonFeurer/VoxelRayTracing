pub mod open_simplex;

use crate::math::aabb::Aabb;
use glam::{IVec3, Vec2, Vec3};
use open_simplex::{init_gradients, MultiNoiseMap, NoiseMap};

pub mod voxel {
    use super::Voxel;
    pub const AIR: Voxel = Voxel(0);
    pub const STONE: Voxel = Voxel(1);
    pub const DIRT: Voxel = Voxel(2);
    pub const GRASS: Voxel = Voxel(3);
    pub const FIRE: Voxel = Voxel(4);
    pub const MAGMA: Voxel = Voxel(5);
    pub const WATER: Voxel = Voxel(6);
    pub const WOOD: Voxel = Voxel(7);
    pub const BARK: Voxel = Voxel(8);
    pub const LEAVES: Voxel = Voxel(9);
    pub const SAND: Voxel = Voxel(10);
    pub const MUD: Voxel = Voxel(11);
    pub const CLAY: Voxel = Voxel(12);
    pub const GOLD: Voxel = Voxel(13);
    pub const MIRROR: Voxel = Voxel(14);
    pub const BRIGHT: Voxel = Voxel(15);
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Voxel(pub u8);
impl Voxel {
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self == voxel::AIR || self == voxel::WATER
    }
    #[inline(always)]
    pub fn is_solid(self) -> bool {
        self != voxel::AIR && self != voxel::WATER
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
    "gold",
    "mirror",
    "bright",
];

#[derive(Clone, Copy)]
#[repr(C)]
// color: vec3<f32>,
// pass_chance: f32,
// emission: f32,
// reflect_chance: f32,
pub struct VoxelProps {
    color: [f32; 3],
    pass_chance: f32,
    emission: f32,
    reflect_chance: f32,
}
impl VoxelProps {
    const DEFAULT: Self = Self {
        color: [0.0; 3],
        pass_chance: 0.0,
        emission: 0.0,
        reflect_chance: 0.0,
    };

    const fn color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }

    const fn emit(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }

    const fn pass(mut self, pass_chance: f32) -> Self {
        self.pass_chance = pass_chance;
        self
    }

    const fn reflect(mut self, reflect_chance: f32) -> Self {
        self.reflect_chance = reflect_chance;
        self
    }
}

pub static mut VOXEL_PROPS: [VoxelProps; 256] = [VoxelProps::DEFAULT; 256];

pub fn load_default_props(props: &mut [VoxelProps]) {
    const DEFAULT: VoxelProps = VoxelProps::DEFAULT;
    use voxel::*;

    props[AIR.0 as usize] = DEFAULT.pass(1.0);
    props[STONE.0 as usize] = DEFAULT.color([0.4; 3]);
    props[DIRT.0 as usize] = DEFAULT.color([0.4, 0.2, 0.0]);
    props[GRASS.0 as usize] = DEFAULT.color([0.1, 0.7, 0.1]);
    props[FIRE.0 as usize] = DEFAULT.color([1.0, 0.9, 0.2]).emit(0.5);
    props[MAGMA.0 as usize] = DEFAULT.color([0.75, 0.18, 0.01]).emit(0.8).reflect(0.5);
    props[WATER.0 as usize] = DEFAULT.color([0.0, 0.0, 1.0]).pass(0.5).reflect(0.5);
    props[WOOD.0 as usize] = DEFAULT;
    props[BARK.0 as usize] = DEFAULT.color([0.86, 0.85, 0.82]);
    props[LEAVES.0 as usize] = DEFAULT.color([0.23, 0.52, 0.0]);
    props[SAND.0 as usize] = DEFAULT.color([0.99, 0.92, 0.53]).reflect(0.2);
    props[MUD.0 as usize] = DEFAULT.color([0.22, 0.13, 0.02]).reflect(0.4);
    props[CLAY.0 as usize] = DEFAULT.color([0.35, 0.30, 0.25]).reflect(0.4);
    props[GOLD.0 as usize] = DEFAULT.color([0.83, 0.68, 0.22]).reflect(0.7);
    props[MIRROR.0 as usize] = DEFAULT.reflect(1.0);
    props[BRIGHT.0 as usize] = DEFAULT.color([1.0; 3]).emit(1.0);
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct BitField(u32);
impl BitField {
    fn set(&mut self, data: u32, len: u32, offset: u32) {
        let mask = !(!0 << len) << offset;
        self.0 = (self.0 & !mask) | (data << offset);
    }

    fn get(self, len: u32, offset: u32) -> u32 {
        let mask = !(!0 << len) << offset;
        (self.0 & mask) >> offset
    }
}
#[test]
fn test_bitfield() {
    let mut a = BitField(0b0);

    a.set(1, 1, 1);
    assert_eq!(a.0, 0b00000000_00000010);

    a.set(1, 1, 2);
    assert_eq!(a.0, 0b00000000_00000110);

    a.set(0b101, 3, 5);
    assert_eq!(a.0, 0b00000000_10100110);

    a.set(0b11011101, 8, 8);
    assert_eq!(a.0, 0b11011101_10100110);

    assert_eq!(a.get(1, 0), 0);
    assert_eq!(a.get(1, 1), 1);
    assert_eq!(a.get(2, 0), 0b10);
    assert_eq!(a.get(2, 1), 0b11);

    a.set(0, 8, 0);
    assert_eq!(a.0, 0b11011101_00000000);
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Node {
    data: BitField,
    first_child: u32,
}
impl Node {
    pub fn new(voxel: Voxel, first_child: u32, is_split: bool) -> Self {
        let mut rs = Self {
            data: BitField(0),
            first_child,
        };
        rs.set_split_flag(is_split);
        rs.set_voxel(voxel);
        rs
    }

    pub fn get_voxel(self) -> Voxel {
        Voxel(self.data.get(8, 8) as u8)
    }

    pub fn set_voxel(&mut self, voxel: Voxel) {
        self.data.set(voxel.0 as u32, 8, 8)
    }

    pub fn set_free_flag(&mut self, f: bool) {
        self.data.set(f as u32, 1, 1)
    }
    pub fn is_free(self) -> bool {
        self.data.get(1, 1) == 1
    }

    pub fn set_split_flag(&mut self, f: bool) {
        self.data.set(f as u32, 1, 0)
    }
    pub fn is_split(self) -> bool {
        self.data.get(1, 0) == 1
    }

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
        self.set_voxel(voxel::MAGMA);
    }

    pub fn simplify(&mut self, result: Voxel) {
        self.set_split_flag(false);
        self.set_voxel(result);
    }
}

pub trait WorldPopulator {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World);
}

const MAX_NODES: usize = 20_000_000;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct World {
    pub root_idx: u32,
    pub size: u32,
    pub max_depth: u32,
    pub start_search: u32,
    pub nodes: [Node; MAX_NODES],
}
impl World {
    pub fn init(&mut self, max_depth: u32) {
        self.root_idx = 0;
        self.max_depth = max_depth;
        self.size = 1 << max_depth;
        self.start_search = 1;
        self.nodes[0] = Node::new(voxel::AIR, 0, false);
        for node in &mut self.nodes[1..] {
            node.set_free_flag(true);
        }
    }
}

struct FoundNode {
    idx: u32,
    depth: u32,
}

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

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) {
        let FoundNode { idx, depth, .. } = self.find_node(pos, self.max_depth);
        let node = self.get_node(idx);

        if node.get_voxel() == voxel {
            return;
        }
        if depth == self.max_depth {
            self.mut_node(idx).set_voxel(voxel);

            let mut parent_depth = depth - 1;
            let mut parent_idx = self.find_node(pos, parent_depth).idx;

            while self.get_node(parent_idx).can_simplify(self) {
                //
                let first_child = self.get_node(parent_idx).get_child(0);
                let reduce_to = self.get_node(first_child).get_voxel();
                self.mut_node(parent_idx).simplify(reduce_to);
                self.free_nodes(first_child);

                parent_depth -= 1;
                parent_idx = self.find_node(pos, parent_depth).idx;
            }
            return;
        }
        let new_first_child = self.new_nodes(node.get_voxel());

        self.mut_node(idx).split(new_first_child);
        self.set_voxel(pos, voxel);
    }

    fn count_nodes_impl(&self, node: u32) -> u32 {
        let node = self.get_node(node);
        if !node.is_split() {
            return 1;
        }
        self.count_nodes_impl(node.get_child(0))
            + self.count_nodes_impl(node.get_child(1))
            + self.count_nodes_impl(node.get_child(2))
            + self.count_nodes_impl(node.get_child(3))
            + self.count_nodes_impl(node.get_child(4))
            + self.count_nodes_impl(node.get_child(5))
            + self.count_nodes_impl(node.get_child(6))
            + self.count_nodes_impl(node.get_child(7))
    }

    pub fn count_nodes(&self) -> u32 {
        self.count_nodes_impl(self.root_idx)
    }

    pub fn get_node(&self, idx: u32) -> Node {
        self.nodes[idx as usize]
    }

    pub fn mut_node(&mut self, idx: u32) -> &mut Node {
        &mut self.nodes[idx as usize]
    }

    fn free_nodes(&mut self, start: u32) {
        if start < self.start_search {
            self.start_search = start;
        }
        for idx in start..start + 8 {
            self.nodes[idx as usize].set_free_flag(false);
        }
    }

    fn new_nodes(&mut self, voxel: Voxel) -> u32 {
        let mut result = self.start_search;
        if result + 8 >= self.nodes.len() as u32 {
            panic!("FAILED TO CREATE NEW NODE - OUT OF SPACE");
        }

        while !self.get_node(result).is_free() {
            result += 8;
            if result + 8 >= self.nodes.len() as u32 {
                panic!("FAILED TO CREATE NEW NODE - OUT OF SPACE");
            }
        }
        self.start_search = result + 8;

        for idx in result..result + 8 {
            self.nodes[idx as usize] = Node::new(voxel, 0, false);
        }
        result
    }
}

impl World {
    pub fn get_voxel(&self, pos: IVec3) -> Option<Voxel> {
        let FoundNode { idx, .. } = self.find_node(pos, self.max_depth);
        Some(self.get_node(idx).get_voxel())
    }
    pub fn set_voxels(&mut self, min: IVec3, max: IVec3, voxel: Voxel) {
        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    self.set_voxel(IVec3 { x, y, z }, voxel);
                }
            }
        }
    }

    pub fn surface_at(&self, x: i32, z: i32) -> i32 {
        for y in 0..self.size as i32 {
            if self.get_voxel(IVec3 { x, y, z }).unwrap().is_empty() {
                return y;
            }
        }
        0
    }

    pub fn populate_with<P: WorldPopulator>(&mut self, p: &P) {
        let min = IVec3::ZERO;
        let max = IVec3::splat(self.size as i32);
        p.populate(min, max, self);
    }

    pub fn get_collisions_w(&self, aabb: &Aabb) -> Vec<Aabb> {
        let mut aabbs = Vec::new();

        let from = aabb.from.floor().as_ivec3();
        let to = aabb.to.ceil().as_ivec3();

        for x in from.x..to.x {
            for y in from.y..to.y {
                for z in from.z..to.z {
                    let pos = IVec3 { x, y, z };

                    let voxel = self.get_voxel(pos).unwrap_or(voxel::AIR);

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
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) {
        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in 0..3 {
                    world.set_voxel(IVec3 { x, y, z }, voxel::STONE);
                }
            }
        }
        for x in min.x..max.x {
            for z in min.z..max.z {
                for y in 0..3 {
                    world.set_voxel(IVec3 { x, y, z }, voxel::DIRT);
                }
            }
        }
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

    pub fn get_terrain_h(&self, pos: Vec2) -> f32 {
        let height_scale = self.height_scale_map.get(pos) * self.scale;
        let height_freq = self.height_freq_map.get(pos) * self.freq;
        self.height_map.get(pos * height_freq) * height_scale
    }

    pub fn spawn_tree(&self, world: &mut World, surface: IVec3) {
        let h = fastrand::u32(6..14) as i32;
        for i in 0..h {
            world.set_voxel(surface + IVec3::new(0, i, 0), voxel::BARK);
        }
        self.sphere(
            world,
            surface + IVec3::new(0, h as i32, 0),
            4,
            voxel::LEAVES,
        );
    }

    pub fn sphere(&self, world: &mut World, pos: IVec3, r: u32, voxel: Voxel) {
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
                    world.set_voxel(IVec3 { x, y, z }, voxel);
                }
            }
        }
    }
}
impl WorldPopulator for DefaultWorldGen {
    fn populate(&self, min: IVec3, max: IVec3, world: &mut World) {
        for x in min.x..max.x {
            for z in min.z..max.z {
                let noise_pos = Vec2::new(x as f32, z as f32) + Vec2::splat(0.5);

                let y = self.get_terrain_h(noise_pos) as i32;
                let surface_pos = IVec3 { x, y, z };

                // set stone
                world.set_voxels(
                    IVec3::new(x, 0, z),
                    IVec3::new(x + 1, y - 3, z + 1),
                    voxel::STONE,
                );

                // set dirt
                world.set_voxels(
                    IVec3::new(x, y - 3, z),
                    IVec3::new(x + 1, y, z + 1),
                    voxel::DIRT,
                );

                // set surface
                world.set_voxel(surface_pos, voxel::GRASS);

                if fastrand::u32(0..300) == 0 {
                    self.spawn_tree(world, surface_pos);
                }
            }
        }
    }
}
