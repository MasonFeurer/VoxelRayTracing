use client::common::math::Aabb;
use glam::{ivec3, uvec3, IVec3, UVec3, Vec3};
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

pub type NodeAddr = u32;
pub type NodeRange = Range<NodeAddr>;

/// The voxel-width of a chunk.
pub const CHUNK_SIZE: u32 = 32;

/// The depth in a Chunk's SVO where nodes are the same size as voxels.
/// Derived from "2^(CHUNK_DEPTH) = CHUNK_SIZE"
pub const CHUNK_DEPTH: u32 = 5;

/// The maximum number of nodes a chunk could need to represent it's state.
/// Derived from "1 + 2^3 + 4^3 + 8^3 + 16^3 + 32^3"
pub const NODES_PER_CHUNK: u32 = 37_449;

/// When adding a chunk to the world, this is the number of extra nodes the chunk makes room for.
/// When a chunk needs to use more than this amount more of extra memory for storing nodes,
/// The chunk will have to be re-located in memory.
pub const CHUNK_INIT_FREE_MEM: u32 = 256;

#[inline(always)]
pub fn vox_to_chunk_pos(pos: IVec3) -> IVec3 {
    pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32))
}

struct FoundNode {
    idx: NodeAddr,
    depth: u32,
    center: UVec3,
    size: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Voxel(pub u16);
impl Voxel {
    pub const EMPTY: Self = Self(0);

    pub fn is_empty(self) -> bool {
        self.0 == 0
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

pub enum SetVoxelErr {
    PosOutOfBounds,
    OutOfMemory,
    NoChunk,
}

#[derive(Clone)]
pub struct Chunk {
    // The range in the Node list that this chunk occupies.
    pub range: NodeRange,
    pub alloc: NodeAlloc,
}
impl Chunk {
    pub fn empty() -> Self {
        Self {
            range: 0..1,
            alloc: NodeAlloc::new(0..1, 1..1),
        }
    }

    pub fn new(used: NodeRange, free: NodeRange) -> Self {
        Self {
            range: used.start..free.end,
            alloc: NodeAlloc::new(used, free),
        }
    }

    pub fn find_node(
        &self,
        nodes: &[Node],
        pos: UVec3,
        max_depth: u32,
    ) -> Result<FoundNode, SetVoxelErr> {
        let mut center = UVec3::splat(CHUNK_SIZE / 2);
        let mut size = CHUNK_SIZE;
        let mut idx = self.range.start;
        let mut depth: u32 = 0;

        loop {
            let node = nodes[idx as usize];
            if !node.is_split() || depth == max_depth {
                return Ok(FoundNode {
                    idx,
                    depth,
                    center,
                    size,
                });
            }
            size /= 2;

            let gt = uvec3(
                (pos.x >= center.x) as u32,
                (pos.y >= center.y) as u32,
                (pos.z >= center.z) as u32,
            );
            let child_idx = (gt.x << 0) | (gt.y << 1) | (gt.z << 2);
            idx = node.child_idx() + child_idx;
            let child_dir = gt.as_ivec3() * 2 - IVec3::ONE;
            center = (center.as_ivec3() + IVec3::splat(size as i32 / 2) * child_dir).as_uvec3();
            depth += 1;
        }
    }

    pub fn set_voxel(
        &mut self,
        nodes: &mut [Node],
        pos: UVec3,
        voxel: Voxel,
    ) -> Result<(), SetVoxelErr> {
        let FoundNode {
            mut idx,
            mut center,
            mut size,
            depth,
            ..
        } = self.find_node(nodes, pos, CHUNK_DEPTH)?;

        let parent_voxel = nodes[idx as usize].voxel();
        // on_change(idx..idx + 1);

        // If depth is less than target_depth,
        // the SVO doesn't go to desired depth, so we must split until it does
        for _ in depth..CHUNK_DEPTH {
            let first_child = self.alloc.alloc()?;
            nodes[first_child as usize..(first_child as usize + 8)]
                .copy_from_slice(&[Node::new(parent_voxel); 8]);

            nodes[idx as usize] = Node::new_split(first_child);
            // on_change(first_child..first_child + 8);
            size /= 2;

            let gt = uvec3(
                (pos.x >= center.x) as u32,
                (pos.y >= center.y) as u32,
                (pos.z >= center.z) as u32,
            );
            let child_idx = (gt.x << 0) | (gt.y << 1) | (gt.z << 2);
            idx = first_child + child_idx;
            let child_dir = gt.as_ivec3() * 2 - IVec3::ONE;
            center = (center.as_ivec3() + IVec3::splat(size as i32 / 2) * child_dir).as_uvec3();
        }
        // SVO now goes to desired depth, so we can mutate the node now.
        nodes[idx as usize] = Node::new(voxel);
        Ok(())
    }
    pub fn get_voxel(&mut self, nodes: &[Node], pos: UVec3, voxel: Voxel) {
        todo!()
    }
}

#[derive(Clone)]
pub struct NodeAlloc {
    // The range in the Node list that this chunk occupies.
    range: NodeRange,

    // Spans of free memory where this allocator is able to place new nodes.
    pub free_mem: Vec<NodeRange>,
}
impl NodeAlloc {
    pub fn new(used: NodeRange, free: NodeRange) -> Self {
        Self {
            range: used.start..free.end,
            free_mem: vec![free],
        }
    }

    pub fn alloc(&mut self) -> Result<NodeAddr, SetVoxelErr> {
        // Assuming theres only ever one free_mem NodeRange for now.
        let mut free = &mut self.free_mem[0];

        if free.end - free.start < 8 {
            return Err(SetVoxelErr::OutOfMemory);
        }
        let result_addr = free.start;
        free.start += 8;
        Ok(result_addr)
    }
}

pub struct ChunkGrid {
    pub chunks: Box<[Option<Chunk>]>,
    pub chunk_count: usize,
    pub size: u32,
}
impl ChunkGrid {
    // the distance from the center to the edge of the grid.
    // the grids width is derived from (`size` * 2 + 1).
    pub fn new(size: u32) -> Self {
        let width = size * 2 + 1;
        let area = (width * width) as usize;
        let chunks = vec![<Option<Chunk>>::None; area].into_boxed_slice();

        Self {
            chunks,
            chunk_count: area,
            size: width,
        }
    }

    pub fn put_chunk(&mut self, pos: UVec3, chunk: Chunk) {
        let idx = pos.x + pos.y * self.size + pos.z * self.size * self.size;
        self.chunks[idx as usize] = Some(chunk);
    }
    pub fn get_chunk(&self, pos: UVec3) -> Option<&Chunk> {
        let idx = pos.x + pos.y * self.size + pos.z * self.size * self.size;

        self.chunks.get(idx as usize)?.as_ref()
    }
    pub fn get_chunk_mut(&mut self, pos: UVec3) -> Option<&mut Chunk> {
        let idx = pos.x + pos.y * self.size + pos.z * self.size * self.size;
        self.chunks.get_mut(idx as usize)?.as_mut()
    }

    pub fn resize(&mut self, new_size: u32) {
        todo!()
    }

    pub fn clear(&mut self) {
        todo!()
    }
}

pub struct ChunkAlloc {
    free_mem: Vec<NodeRange>,
}
impl ChunkAlloc {
    pub fn new(max_nodes: u32) -> Self {
        Self {
            free_mem: vec![1..max_nodes],
        }
    }

    pub fn alloc_chunk(&mut self, size: u32) -> Chunk {
        let req_space = size + CHUNK_INIT_FREE_MEM;

        let mut space: Option<&mut NodeRange> = None;
        for available in &mut self.free_mem {
            let space_size = available.end - available.start;
            if space_size >= req_space {
                space = Some(available);
                break;
            }
        }
        let Some(space) = space else {
            panic!("No available memory for allocating chunk");
        };
        let chunk_space = space.start..(space.start + req_space as u32);
        space.start = chunk_space.end;
        let used_mem = chunk_space.start..(chunk_space.start + size as u32);
        let free_mem = (chunk_space.start + size as u32)..chunk_space.end;
        Chunk::new(used_mem, free_mem)
    }
}

pub struct World {
    pub origin: IVec3,
    pub size_in_chunks: u32,
    pub chunks: ChunkGrid,
    pub nodes: Box<[Node]>,
    pub chunk_alloc: ChunkAlloc,
}
impl World {
    pub fn new(origin: IVec3, max_nodes: u32, size: u32) -> Self {
        let mut nodes = vec![Node::ZERO; max_nodes as usize].into_boxed_slice();
        nodes[0] = Node::new(Voxel(0)); // 0 = air
        Self {
            origin,
            size_in_chunks: size,
            chunks: ChunkGrid::new(size),
            nodes,
            chunk_alloc: ChunkAlloc::new(max_nodes),
        }
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn size_in_chunks(&self) -> u32 {
        self.size_in_chunks
    }
    pub fn size(&self) -> u32 {
        self.size_in_chunks * CHUNK_SIZE
    }
    pub fn min(&self) -> IVec3 {
        self.origin
    }

    pub fn chunk_roots(&self) -> Vec<NodeAddr> {
        self.chunks
            .chunks
            .iter()
            .map(|chunk| chunk.as_ref().map(|c| c.range.start).unwrap_or(0))
            .collect()
    }

    pub fn put_chunk(&mut self, pos: UVec3, nodes: &[Node]) {
        let chunk = self.chunk_alloc.alloc_chunk(nodes.len() as u32);
        let range = chunk.range.start..(chunk.range.start + nodes.len() as u32);

        self.nodes[(range.start as usize)..(range.end as usize)].copy_from_slice(&nodes);
        self.chunks.put_chunk(pos, chunk);
    }

    pub fn get_chunk(&self, pos: IVec3) -> Option<&[Node]> {
        todo!()
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<(), SetVoxelErr> {
        let pos = (pos - self.origin).as_uvec3();
        let chunk_pos = vox_to_chunk_pos(pos.as_ivec3()).as_uvec3();
        let pos_in_chunk = pos - (chunk_pos * CHUNK_SIZE);

        self.chunks
            .get_chunk_mut(chunk_pos)
            .ok_or(SetVoxelErr::NoChunk)?
            .set_voxel(&mut self.nodes, pos_in_chunk, voxel)
    }

    pub fn get_voxel(&self, pos: IVec3) -> Option<Voxel> {
        todo!()
    }
}
