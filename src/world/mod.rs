pub mod data;
pub mod gen;
pub mod noise;

use crate::math::aabb::Aabb;
use glam::{ivec3, IVec3, UVec3, Vec3};
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

pub type NodeAddr = u32;

#[inline(always)]
pub fn vox_to_chunk_pos(pos: IVec3) -> IVec3 {
    pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32))
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Voxel(pub u16);
impl Voxel {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const DIRT: Self = Self(2);
    pub const GRASS: Self = Self(3);
    pub const SNOW: Self = Self(4);
    pub const DEAD_GRASS: Self = Self(5);
    pub const MOIST_GRASS: Self = Self(6);
    pub const SAND: Self = Self(7);
    pub const MUD: Self = Self(8);
    pub const CLAY: Self = Self(9);
    pub const FIRE: Self = Self(10);
    pub const MAGMA: Self = Self(11);
    pub const WATER: Self = Self(12);
    pub const OAK_WOOD: Self = Self(13);
    pub const OAK_LEAVES: Self = Self(14);
    pub const BIRCH_WOOD: Self = Self(15);
    pub const BIRCH_LEAVES: Self = Self(16);
    pub const SPRUCE_WOOD: Self = Self(17);
    pub const SPRUCE_LEAVES: Self = Self(18);
    pub const CACTUS: Self = Self(19);
    pub const GOLD: Self = Self(20);
    pub const MIRROR: Self = Self(21);
    pub const BRIGHT: Self = Self(22);

    #[inline(always)]
    pub fn display_name(&self) -> &'static str {
        &data::VOXEL_NAMES[self.0 as usize]
    }

    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self == Self::AIR || self == Self::WATER
    }
    #[inline(always)]
    pub fn is_solid(self) -> bool {
        match self {
            Self::AIR => false,
            Self::WATER => false,
            Self::MAGMA => false,
            Self::FIRE => false,
            Self::MUD => false,
            _ => true,
        }
    }

    #[inline(always)]
    pub fn viscosity(self) -> f32 {
        match self {
            Self::AIR => 1.0,
            Self::WATER => 0.6,
            Self::MAGMA => 0.2,
            Self::FIRE => 1.0,
            Self::MUD => 0.2,
            _ => 0.0,
        }
    }
}

/// Represents a node in the sparse voxel octree (SVO) for each chunk.
///
/// # States
/// ## 0_______________xxxxxxxxxxxxxxxx
/// Entire node is occupied by voxel `x`.
///
/// ## 1yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
/// Node splits into 8 nodes of half size at `y`.

/// Note: By storing the nodes in a specific way, we can remove the need for a Node to point to it's child.
///
/// chunk: 16^3 = 4_096
/// max nodes: 16^3+8^3+4^3+2^3+1^3 = 4_681
/// bitlen: 13
///
/// chunk: 32^3 = 32_768
/// max nodes: 32^3+16^3+8^3+4^3+2^3+1^3 = 37_449
/// bitlen: 16
///
/// chunk: 64^3 = 262_144
/// max nodes: 64^3+32^3+16^3+8^3+4^3+2^3+1^3 = 299_593
/// bitlen: 19
///
/// 299_583 bytes ~= 300Mb

/// 299_593

/// 15^3 chunks in world = 3375 chunks
/// meaning 1011126375 nodes
/// meaning 1_011_126_375 (~1Gb) byte world
///
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Node(u32);
impl Node {
    pub const ZERO: Self = Self(0);
    const SPLIT_MASK: u32 = 0x8000_0000;
    const DATA_MASK: u32 = 0x7FFF_FFFF;

    pub fn new(vox: Voxel) -> Self {
        Self((vox.0 as u32) & Self::DATA_MASK)
    }
    pub fn new_split(child_idx: u32) -> Self {
        Self(child_idx | Self::SPLIT_MASK)
    }

    pub fn voxel(self) -> Voxel {
        Voxel((self.0 & 0xFFFF) as u16)
    }
    pub fn set_voxel(&mut self, voxel: Voxel) {
        self.0 = (self.0 & Self::SPLIT_MASK) | voxel.0 as u32;
    }

    pub fn is_split(self) -> bool {
        (self.0 >> 31) != 0
    }
    pub fn set_split(&mut self, split: bool) {
        self.0 = (self.0 & Self::DATA_MASK) | ((split as u32) << 31);
    }

    pub fn child_idx(self) -> u32 {
        self.0 & Self::DATA_MASK
    }
    pub fn set_child_idx(&mut self, idx: u32) {
        self.0 = (self.0 & Self::SPLIT_MASK) | idx;
    }
}

#[derive(Clone, Debug)]
pub enum WorldErr {
    Oob,
    ChunkOob,
    NodeAllocLimit,
}

struct FoundNode {
    idx: NodeAddr,
    depth: u32,
    center: IVec3,
    size: u32,
}

#[derive(Default, Clone)]
pub struct NodeAlloc {
    pub range: Range<NodeAddr>,
    pub next: NodeAddr,
}
impl NodeAlloc {
    fn new(range: Range<NodeAddr>) -> Self {
        Self {
            next: range.start,
            range,
        }
    }

    fn next(&mut self) -> Result<NodeAddr, WorldErr> {
        // note: self.range.end is exclusive
        if self.next + 7 >= self.range.end {
            return Err(WorldErr::NodeAllocLimit);
        }
        let idx = self.next;
        self.next += 8;
        Ok(idx)
    }

    pub fn reset(&mut self) {
        self.next = self.range.start;
    }
}

/// The voxel-width of a chunk.
pub const CHUNK_SIZE: u32 = 32;

/// The depth in a Chunk's SVO where nodes are the same size as voxels.
/// Derived from "2^(CHUNK_DEPTH) = CHUNK_SIZE"
pub const CHUNK_DEPTH: u32 = 5;

/// The maximum number of nodes a chunk could need to represent it's state.
/// Derived from "1 + 2^3 + 4^3 + 8^3 + 16^3 + 32^3"
pub const NODES_PER_CHUNK: u32 = 37_449;

#[repr(C)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ChunkHeader {
    pub root: NodeAddr,
    pub alloc: u32,
}
impl ChunkHeader {
    pub const ZERO: Self = Self { root: 0, alloc: 0 };
}

pub struct World {
    min: IVec3,
    size_in_chunks: u32,
    chunk_count: u32,
    prev_anchor_chunk: IVec3,

    pub chunks: Box<[ChunkHeader]>,
    pub region_locks: Box<[AtomicBool]>,
    pub allocs: Box<[NodeAlloc]>,
    pub nodes: Box<[Node]>,
}
/// Create and clear worlds
impl World {
    pub fn new(max_nodes: u32, size_in_chunks: u32) -> Self {
        let chunk_count = size_in_chunks * size_in_chunks * size_in_chunks;

        let nodes = vec![Node::ZERO; max_nodes as usize].into_boxed_slice();

        let allocs = (0..chunk_count)
            .into_iter()
            .map(|idx| {
                let start = idx * NODES_PER_CHUNK;
                let end = start + NODES_PER_CHUNK;
                NodeAlloc::new((start + 1)..end)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let chunks = (0..chunk_count)
            .into_iter()
            .map(|i| {
                let root = i * NODES_PER_CHUNK;
                ChunkHeader { root, alloc: i }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let region_locks = (0..chunk_count)
            .into_iter()
            .map(|_| AtomicBool::new(false))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            min: IVec3::ZERO,
            size_in_chunks,
            chunk_count,
            prev_anchor_chunk: IVec3::ZERO,

            chunks,
            allocs,
            nodes,
            region_locks,
        }
    }

    #[inline(always)]
    pub fn size(&self) -> u32 {
        self.size_in_chunks * CHUNK_SIZE
    }

    #[inline(always)]
    pub fn size3(&self) -> UVec3 {
        UVec3::splat(self.size_in_chunks * CHUNK_SIZE)
    }

    #[inline(always)]
    pub fn size_in_chunks(&self) -> u32 {
        self.size_in_chunks
    }

    #[inline(always)]
    pub fn size_in_chunks3(&self) -> UVec3 {
        UVec3::splat(self.size_in_chunks)
    }

    #[inline(always)]
    pub fn min(&self) -> IVec3 {
        self.min
    }
    #[inline(always)]
    pub fn max(&self) -> IVec3 {
        self.min + self.size3().as_ivec3()
    }

    #[inline(always)]
    pub fn min_chunk_pos(&self) -> IVec3 {
        vox_to_chunk_pos(self.min())
    }

    #[inline(always)]
    pub fn max_chunk_pos(&self) -> IVec3 {
        vox_to_chunk_pos(self.max())
    }

    #[inline(always)]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    #[inline(always)]
    pub fn chunk_idx(&self, mut pos: IVec3) -> Option<u32> {
        pos -= self.min_chunk_pos();
        let w = self.size_in_chunks;
        if pos.cmplt(IVec3::ZERO).any() || pos.cmpge(IVec3::splat(w as i32)).any() {
            return None;
        }
        Some(pos.x as u32 + pos.y as u32 * w + pos.z as u32 * w * w)
    }

    #[inline(always)]
    pub fn chunk_nodes(&self, chunk_idx: u32) -> &[Node] {
        let min = self.chunk_nodes_offset(chunk_idx) as usize;
        let max = self.allocs[self.chunks[chunk_idx as usize].alloc as usize]
            .range
            .end;
        &self.nodes[min as usize..max as usize]
    }

    #[inline(always)]
    pub fn chunk_nodes_offset(&self, chunk_idx: u32) -> NodeAddr {
        self.chunks[chunk_idx as usize].root
    }

    #[inline(always)]
    pub fn chunk_count(&self) -> u32 {
        self.chunk_count
    }

    #[inline(always)]
    pub fn check_bounds(&self, pos: IVec3) -> Result<(), WorldErr> {
        let in_bounds = (pos.cmpge(self.min())).all() && (pos.cmplt(self.max())).all();
        in_bounds.then(|| ()).ok_or(WorldErr::Oob)
    }

    pub fn lock_chunk(&mut self, idx: u32) {
        let start_time = SystemTime::now();
        while self.region_locks[idx as usize]
            .compare_exchange(false, true, Ordering::Acquire, Ordering::SeqCst)
            .is_err()
        {
            if SystemTime::now()
                .duration_since(start_time)
                .unwrap_or(Duration::ZERO)
                .as_millis()
                > 1000
            {
                panic!("THREAD LOCK NOT BEING UNLOCKED : {idx}");
            }
        }
    }
    pub fn unlock_chunk(&mut self, idx: u32) {
        self.region_locks[idx as usize].store(false, Ordering::Release)
    }
}
/// Manage chunks
impl World {
    pub fn rotate_chunks(&mut self, offset: IVec3) -> Vec<IVec3> {
        let w = self.size_in_chunks;
        let pos_as_idx =
            |pos: IVec3| (pos.x + pos.y * w as i32 + pos.z * w as i32 * w as i32) as usize;
        let pos_oob =
            |pos: IVec3| pos.cmplt(IVec3::ZERO).any() || pos.cmpge(IVec3::splat(w as i32)).any();

        let mut new_chunks = vec![ChunkHeader::ZERO; self.chunk_count as usize].into_boxed_slice();
        let mut rebuild = vec![];
        let min_chunk = self.min_chunk_pos();

        for x in 0..w {
            for y in 0..w {
                for z in 0..w {
                    let pos = ivec3(x as i32, y as i32, z as i32);
                    let idx = pos_as_idx(pos);
                    let dst_pos = pos - offset;
                    let dst_pos = dst_pos.rem_euclid(IVec3::splat(w as i32));
                    let dst_idx = pos_as_idx(dst_pos);

                    new_chunks[dst_idx] = self.chunks[idx].clone();

                    if pos_oob(pos + offset) {
                        rebuild.push(pos + min_chunk);
                    }
                }
            }
        }
        self.chunks = new_chunks;
        rebuild
    }

    pub fn update(&mut self, anchor: IVec3) -> Vec<IVec3> {
        let chunk_size = IVec3::splat(CHUNK_SIZE as i32);
        let w = self.size_in_chunks;

        let anchor_chunk = vox_to_chunk_pos(anchor);
        if anchor_chunk == self.prev_anchor_chunk {
            return vec![];
        }
        let prev_min_chunk = self.min / chunk_size;
        let min_chunk = anchor_chunk - IVec3::splat(w as i32 / 2);

        if prev_min_chunk == min_chunk {
            return vec![];
        }
        self.min = min_chunk * chunk_size;

        let chunk_offset = min_chunk - prev_min_chunk;
        self.rotate_chunks(chunk_offset)
    }
}
/// Find and mutate the SVO nodes that make up the world.
impl World {
    fn find_chunk_node(
        &self,
        pos: IVec3,
        max_depth: u32,
        chunk: ChunkHeader,
    ) -> Result<FoundNode, WorldErr> {
        let mut center = IVec3::splat(CHUNK_SIZE as i32 / 2);
        let mut size = CHUNK_SIZE;
        let mut idx = chunk.root;
        let mut depth: u32 = 0;

        loop {
            let node = self.get_node(idx);
            if !node.is_split() || depth == max_depth {
                return Ok(FoundNode {
                    idx,
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
            let child_idx = ((gt.x as u32) << 0) | ((gt.y as u32) << 1) | ((gt.z as u32) << 2);
            idx = node.child_idx() + child_idx;
            let child_dir = gt * 2 - IVec3::ONE;
            center += IVec3::splat(size as i32 / 2) * child_dir;
            depth += 1;
        }
    }

    fn find_node(&self, pos: IVec3, max_depth: u32) -> Result<FoundNode, WorldErr> {
        let chunk_coords = vox_to_chunk_pos(pos);
        let chunk_idx = self.chunk_idx(chunk_coords).ok_or(WorldErr::ChunkOob)?;
        let chunk = self.chunks[chunk_idx as usize].clone();
        let pos = pos - chunk_coords * CHUNK_SIZE as i32;
        self.find_chunk_node(pos, max_depth, chunk)
    }

    fn alloc_nodes(&mut self, alloc_idx: u32, voxel: Voxel) -> Result<u32, WorldErr> {
        let first_idx = self.allocs[alloc_idx as usize].next()?;
        for i in first_idx..(first_idx + 8) {
            *self.mut_node(i) = Node::new(voxel);
        }
        Ok(first_idx)
    }

    #[inline(always)]
    pub fn reset_alloc(&mut self, alloc_idx: u32) {
        self.allocs[alloc_idx as usize].reset()
    }

    #[inline(always)]
    pub fn get_voxel(&self, pos: IVec3) -> Result<Voxel, WorldErr> {
        let FoundNode { idx, .. } = self.find_node(pos, CHUNK_DEPTH)?;
        Ok(self.get_node(idx).voxel())
    }

    pub fn set_voxel_in_chunk(
        &mut self,
        chunk: ChunkHeader,
        pos: IVec3,
        voxel: Voxel,
        mut on_change: impl FnMut(Range<NodeAddr>),
    ) -> Result<(), WorldErr> {
        let alloc_idx = chunk.alloc;
        let FoundNode {
            mut idx,
            mut center,
            mut size,
            depth,
            ..
        } = self.find_chunk_node(pos, CHUNK_DEPTH, chunk)?;
        // If `idx` is outside the Node region of this chunk,
        // mutating it could cause data races.
        assert!(idx < self.allocs[alloc_idx as usize].range.end);

        let parent_voxel = self.get_node(idx).voxel();
        on_change(idx..idx + 1);

        // If depth is less than target_depth,
        // the SVO doesn't go to desired depth, so we must split until it does
        for _ in depth..CHUNK_DEPTH {
            // note: allocators don't move, so alloc_idx will always be valid
            let first_child = self.alloc_nodes(alloc_idx, parent_voxel)?;

            assert!(idx < self.allocs[alloc_idx as usize].range.end);
            *self.mut_node(idx) = Node::new_split(first_child);
            on_change(first_child..first_child + 8);
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
        *self.mut_node(idx) = Node::new(voxel);
        Ok(())
    }

    #[inline(always)]
    pub fn set_voxel(
        &mut self,
        pos: IVec3,
        voxel: Voxel,
        on_change: impl FnMut(Range<NodeAddr>),
    ) -> Result<(), WorldErr> {
        let chunk_coords = vox_to_chunk_pos(pos);
        let chunk_idx = self.chunk_idx(chunk_coords).ok_or(WorldErr::ChunkOob)?;
        let chunk = self.chunks[chunk_idx as usize].clone();
        let pos = pos - chunk_coords * CHUNK_SIZE as i32;
        self.set_voxel_in_chunk(chunk, pos, voxel, on_change)
    }

    #[inline(always)]
    pub fn set_voxel_collected(
        &mut self,
        pos: IVec3,
        voxel: Voxel,
    ) -> Result<Vec<Range<NodeAddr>>, WorldErr> {
        let mut result = vec![];
        self.set_voxel(pos, voxel, |range| result.push(range))?;
        Ok(result)
    }

    #[inline(always)]
    pub fn get_node(&self, idx: u32) -> Node {
        self.nodes[idx as usize]
    }

    #[inline(always)]
    pub fn mut_node(&mut self, idx: u32) -> &mut Node {
        &mut self.nodes[idx as usize]
    }
}
/// High-level voxel-based manipulation.
impl World {
    pub fn set_voxels(&mut self, a: IVec3, b: IVec3, voxel: Voxel) {
        let min = ivec3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = ivec3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    _ = self.set_voxel(ivec3(x, y, z), voxel, |_| {});
                }
            }
        }
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

        for x in min.x..=max.x {
            for y in min.y..=max.y {
                for z in min.z..=max.z {
                    let block_center = ivec3(x, y, z).as_vec3() + Vec3::splat(0.5);
                    let dist_sq = (block_center - pos_center).length_squared();

                    if dist_sq >= r_sq || fastrand::f32() <= decay {
                        continue;
                    }

                    _ = self.set_voxel(ivec3(x, y, z), voxel, |_| {});
                }
            }
        }
    }
}
