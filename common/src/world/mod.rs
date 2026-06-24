pub mod noise;

use bincode::{Decode, Encode};
use glam::{uvec3, IVec3, UVec3, Vec3};

pub type NodeAddr = u32;
pub type NodeRange = std::ops::Range<NodeAddr>;

/// The voxel-width of a chunk.
pub const CHUNK_SIZE: u32 = 32;

/// The depth in a Chunk's SVO where nodes are the same size as voxels.
/// Derived from "2^(CHUNK_DEPTH) = CHUNK_SIZE"
pub const CHUNK_DEPTH: u32 = 5;

/// The maximum number of nodes a chunk could need to represent it's state.
/// Derived from "1 + 2^3 + 4^3 + 8^3 + 16^3 + 32^3"
pub const NODES_PER_CHUNK: u32 = 37449;

/// When adding a chunk to the world, this is the number of extra nodes the chunk makes room for.
/// When a chunk needs to use more than this amount more of extra memory for storing nodes,
/// The chunk will have to be re-located in memory.
pub const CHUNK_INIT_FREE_MEM: u32 = 2048;

pub const REGION_SIZE: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct GlobalPos<const SCALE: u32>(pub IVec3);
impl<const SCALE: u32> GlobalPos<SCALE> {
    #[inline(always)] pub const fn new(pos: IVec3) -> Self { Self(pos) }
}
impl<const SCALE: u32> std::ops::Deref for GlobalPos<SCALE> {
    type Target = IVec3;
    #[inline(always)] fn deref(&self) -> &Self::Target { &self.0 }
}
impl<const SCALE: u32> From<IVec3> for GlobalPos<SCALE> {
    #[inline(always)] fn from(value: IVec3) -> Self { Self(value) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Hash)]
pub struct LocalPos<const SCALE: u32, const CAP: u32>(pub UVec3);
impl<const SCALE: u32, const CAP: u32> LocalPos<SCALE, CAP> {
    #[inline(always)]
    pub const fn new(pos: UVec3) -> Option<Self> {
        if pos.x >= CAP || pos.y >= CAP || pos.z >= CAP {
            None
        } else {
            Some(Self(pos))
        }
    }
    #[inline(always)] pub const fn new_unchecked(pos: UVec3) -> Self { Self(pos) }
    #[inline(always)] pub const fn center() -> Self { Self::new_unchecked(UVec3::splat(CAP / 2)) }
}
impl<const SCALE: u32, const CAP: u32> std::ops::Deref for LocalPos<SCALE, CAP> {
    type Target = UVec3;
    #[inline(always)] fn deref(&self) -> &Self::Target { &self.0 }
}
impl<const SCALE: u32, const CAP: u32> From<UVec3> for LocalPos<SCALE, CAP> {
    #[inline(always)] fn from(value: UVec3) -> Self { Self(value) }
}

pub type VoxelPos = GlobalPos<1>;
#[allow(non_snake_case)] // this is to mimic a constructor
pub const fn VoxelPos(x: i32, y: i32, z: i32) -> VoxelPos { VoxelPos::new(IVec3::new(x, y, z)) }

pub type ChunkPos = GlobalPos<CHUNK_SIZE>;
#[allow(non_snake_case)] // this is to mimic a constructor
pub const fn ChunkPos(x: i32, y: i32, z: i32) -> ChunkPos { ChunkPos::new(IVec3::new(x, y, z)) }

pub type RegionPos = GlobalPos<REGION_SIZE>;
#[allow(non_snake_case)] // this is to mimic a constructor
pub const fn RegionPos(x: i32, y: i32, z: i32) -> RegionPos { RegionPos::new(IVec3::new(x, y, z)) }

pub type VoxelPosInChunk = LocalPos<1, CHUNK_SIZE>;
#[allow(non_snake_case)] // this is to mimic a constructor
pub const fn VoxelPosInChunk(x: u32, y: u32, z: u32) -> Option<VoxelPosInChunk> { VoxelPosInChunk::new(UVec3::new(x, y, z)) }

pub type ChunkPosInRegion = LocalPos<CHUNK_SIZE, REGION_SIZE>;
#[allow(non_snake_case)] // this is to mimic a constructor
pub const fn ChunkPosInRegion(x: u32, y: u32, z: u32) -> Option<ChunkPosInRegion> { ChunkPosInRegion::new(UVec3::new(x, y, z)) }

impl VoxelPos {
    #[inline(always)]
    pub fn chunk(self) -> (ChunkPos, VoxelPosInChunk) {
        let pos = self.0.div_euclid(IVec3::splat(CHUNK_SIZE as i32));
        let in_chunk = (self.0 - pos * CHUNK_SIZE as i32).as_uvec3();
        (ChunkPos::new(pos), VoxelPosInChunk::new_unchecked(in_chunk))
    }
}
impl ChunkPos {
    #[inline(always)]
    pub fn region(self) -> (RegionPos, ChunkPosInRegion) {
        let pos = self.0.div_euclid(IVec3::splat(REGION_SIZE as i32));
        let in_chunk = (self.0 - pos * REGION_SIZE as i32).as_uvec3();
        (RegionPos::new(pos), ChunkPosInRegion::new_unchecked(in_chunk))
    }

    #[inline(always)]
    pub const fn min(self) -> VoxelPos {
        VoxelPos(
            self.0.x * CHUNK_SIZE as i32,
            self.0.y * CHUNK_SIZE as i32,
            self.0.z * CHUNK_SIZE as i32
        )
    }
    #[inline(always)]
    pub const fn max(self) -> VoxelPos {
        VoxelPos(
            self.0.x * CHUNK_SIZE as i32 + CHUNK_SIZE as i32 - 1,
            self.0.y * CHUNK_SIZE as i32 + CHUNK_SIZE as i32 - 1,
            self.0.z * CHUNK_SIZE as i32 + CHUNK_SIZE as i32 - 1
        )
    }
}
impl VoxelPosInChunk {
    #[inline(always)]
    pub fn global(self, chunk_pos: ChunkPos) -> VoxelPos {
        VoxelPos::new((*chunk_pos * CHUNK_SIZE as i32) + self.as_ivec3())
    }
}
impl ChunkPosInRegion {
    #[inline(always)]
    pub fn global(self, region_pos: RegionPos) -> ChunkPos {
        ChunkPos::new((*region_pos * REGION_SIZE as i32) + self.as_ivec3())
    }
}


#[derive(Debug, PartialEq)]
pub enum SetVoxelErr {
    PosOutOfBounds,
    OutOfMemory,
    NoChunk,
    NoChange,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
///  The first byte is ignored.
pub struct Voxel(u16);
impl Voxel {
    pub const EMPTY: Self = Self(0);
    pub const MAX_VALUE: u16 = u16::MAX / 2;
    
    pub const fn as_data(self) -> u16 { self.0 }
    pub const fn from_data(byte: u16) -> Self { Self(byte) }
    pub const fn is_empty(self) -> bool { self.0 == 0 }
}

/// Represents a node in the sparse voxel octree (SVO) for each chunk.
///
/// # States
/// ## 0xxxxxxxxxxxxxxx
/// Entire node is occupied by voxel `x`.
///
/// ## 1yyyyyyyyyyyyyyy
/// Node splits into 8 nodes of half-size at `y`.
#[repr(transparent)]
#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Node(u16);
impl Node {
    pub const EMPTY: Self = Self::new(Voxel::EMPTY);
    
    const SPLIT_MASK: u16 = 0x8000;
    const DATA_MASK: u16 = 0x7FFF;

    #[inline(always)] pub const fn new(vox: Voxel) -> Self {
        Self(vox.as_data() & Self::DATA_MASK)
    }
    #[inline(always)] pub const fn new_split(child_idx: u16) -> Self {
        Self(child_idx | Self::SPLIT_MASK)
    }

    #[inline(always)] pub const fn voxel(self) -> Voxel {
        Voxel::from_data(self.0 & Self::DATA_MASK)
    }
    #[inline(always)] pub const fn set_voxel(&mut self, voxel: Voxel) {
        self.0 = (self.0 & Self::SPLIT_MASK) | (voxel.as_data() & Self::DATA_MASK);
    }

    #[inline(always)] pub const fn is_split(self) -> bool {
        (self.0 & Self::SPLIT_MASK) != 0
    }
    #[inline(always)] pub const fn set_split(&mut self, split: bool) {
        self.0 = (self.0 & Self::DATA_MASK) | ((split as u16) << 15);
    }

    #[inline(always)] pub const fn child_idx(self) -> u16 {
        self.0 & Self::DATA_MASK
    }
    #[inline(always)] pub const fn set_child_idx(&mut self, idx: u16) {
        self.0 = (self.0 & Self::SPLIT_MASK) | (idx & Self::DATA_MASK);
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(dead_code)] // Fields used by #derive Debug
        enum NodeEnum {
            Split(u16),
            Voxel(u16),
        }
        let e = if self.is_split() {
            NodeEnum::Split(self.child_idx())
        } else {
            NodeEnum::Voxel(self.voxel().as_data())
        };
        f.write_str(&format!("{e:?}"))
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeAlloc {
    // The range in the Node list that this chunk occupies.
    pub range: NodeRange,

    // Spans of free memory where this allocator is able to place new nodes.
    pub free_mem: Vec<NodeRange>,

    pub last_used_addr: NodeAddr,
}
impl NodeAlloc {
    pub fn new(used: NodeRange, free: NodeRange) -> Self {
        assert_eq!(used.end, free.start);
        Self {
            range: used.start..free.end,
            free_mem: vec![free],
            last_used_addr: used.end - 1,
        }
    }

    // `new_idx` should greater than `self.last_used_addr()`
    pub fn move_end(&mut self, new_end: NodeAddr) {
        let free = self
            .free_mem
            .iter_mut()
            .find(|free| free.end == self.range.end)
            .unwrap();
        free.end = new_end;
        self.range.end = new_end;
    }

    pub fn total_free_mem(&self) -> u32 {
        let mut total_free = 0;
        for free in &self.free_mem {
            total_free += free.len();
        }
        total_free as u32
    }
    pub fn total_used_mem(&self) -> u32 {
        self.range.end - self.total_free_mem()
    }

    fn find_next(&self) -> Option<usize> {
        let mut earliest_free = 0;
        let mut earliest_free_addr = u32::MAX;

        for (idx, free) in self.free_mem.iter().enumerate() {
            if free.end.saturating_sub(free.start) < 8 {
                continue;
            }
            if free.start < earliest_free_addr {
                earliest_free_addr = free.start;
                earliest_free = idx;
            }
        }
        if earliest_free_addr == u32::MAX {
            None
        } else {
            Some(earliest_free)
        }
    }

    pub fn next(&mut self) -> Option<NodeAddr> {
        let earliest_free = self.find_next()?;

        let free = &mut self.free_mem[earliest_free];
        let result = free.start.clone();
        free.start += 8;
        if free.start + 1 == free.end {
            _ = self.free_mem.remove(earliest_free);
        }
        self.last_used_addr = self.last_used_addr.max(result + 7);
        Some(result)
    }

    pub fn peek(&self) -> Option<NodeAddr> {
        let earliest_free = self.find_next()?;
        Some(self.free_mem[earliest_free].start)
    }

    pub fn free(&mut self, addr: NodeAddr) {
        let range = addr..addr + 8;
        // check if this span can be extended from an existing free memory span
        for free in &mut self.free_mem {
            if free.start == range.end {
                free.start -= 8;
                return;
            }
            if free.end == addr {
                free.end += 8;
                return;
            }
        }
        self.free_mem.push(range);
    }

    // The highest-most address that is being used to store data.
    pub fn last_used_addr(&self) -> NodeAddr {
        self.last_used_addr
    }
}

#[derive(Clone, Debug)]
pub struct FoundNode {
    pub idx: NodeAddr,
    pub depth: u32,
    pub center: Vec3,
    pub size: u32,
}

pub struct Svo {
    pub root: NodeAddr,
    pub size: u32,
}
impl Svo {
    pub const fn new(root: NodeAddr, size: u32) -> Self {
        Self { root, size }
    }

    pub fn node_parent(&self, nodes: &[Node], node_in: &FoundNode) -> Option<FoundNode> {
        if node_in.depth == 0 {
            return None;
        }
        let mut size = self.size;
        let mut idx = self.root;
        let mut center = Vec3::splat(size as f32 * 0.5);
        let mut depth: u32 = 0;

        loop {
            let node = nodes[idx as usize];
            if !node.is_split() || depth == node_in.depth - 1 {
                return Some(FoundNode {
                    idx,
                    depth,
                    center,
                    size,
                });
            }
            size /= 2;

            let gt = uvec3(
                (node_in.center.x >= center.x) as u32,
                (node_in.center.y >= center.y) as u32,
                (node_in.center.z >= center.z) as u32,
            );
            let child_idx = (gt.x << 0) | (gt.y << 1) | (gt.z << 2);
            idx = node.child_idx() as u32 + child_idx;
            let child_dir = gt.as_ivec3() * 2 - IVec3::ONE;
            center += Vec3::splat(size as f32) * 0.5 * child_dir.as_vec3();
            depth += 1;
        }
    }

    pub fn find_node(&self, nodes: &[Node], pos: UVec3, max_depth: u32) -> FoundNode {
        let mut size = self.size;
        let mut idx = self.root;
        let mut center = Vec3::splat(size as f32 * 0.5);
        let mut depth: u32 = 0;

        loop {
            let node = nodes[idx as usize];
            if !node.is_split() || depth == max_depth {
                return FoundNode {
                    idx,
                    depth,
                    center,
                    size,
                };
            }
            size /= 2;

            let gt = uvec3(
                (pos.x as f32 >= center.x) as u32,
                (pos.y as f32 >= center.y) as u32,
                (pos.z as f32 >= center.z) as u32,
            );
            let child_idx = (gt.x << 0) | (gt.y << 1) | (gt.z << 2);
            idx = node.child_idx() as u32 + child_idx;
            let child_dir = gt.as_ivec3() * 2 - IVec3::ONE;
            center += Vec3::splat(size as f32) * 0.5 * child_dir.as_vec3();
            depth += 1;
        }
    }

    pub fn set_node(
        &self,
        nodes: &mut [Node],
        pos: UVec3,
        voxel: Voxel,
        target_depth: u32,
        alloc: &mut NodeAlloc,
    ) -> Result<(), SetVoxelErr> {
        let mut node = self.find_node(nodes, pos, target_depth);
        let parent_voxel = nodes[node.idx as usize].voxel();
        // No need to break down the SVO because the highest node has the same voxel type.
        if parent_voxel == voxel {
            return Ok(());
        }

        // If depth is less than target_depth,
        // the SVO doesn't go to the desired depth, so we must split until it does
        while node.depth < target_depth {
            let first_child = alloc.next().ok_or(SetVoxelErr::OutOfMemory)?;
            assert!(first_child < Voxel::MAX_VALUE as u32);

            nodes[first_child as usize..(first_child as usize + 8)]
                .copy_from_slice(&[Node::new(parent_voxel); 8]);

            nodes[node.idx as usize] = Node::new_split(first_child as u16);
            node.size /= 2;

            let gt = uvec3(
                (pos.x as f32 >= node.center.x) as u32,
                (pos.y as f32 >= node.center.y) as u32,
                (pos.z as f32 >= node.center.z) as u32,
            );
            let child_dir = gt.as_ivec3() * 2 - IVec3::ONE;
            let child_idx = (gt.x << 0) | (gt.y << 1) | (gt.z << 2);
            node.idx = first_child + child_idx;
            node.center += Vec3::splat(node.size as f32) * 0.5 * child_dir.as_vec3();
            node.depth += 1;
        }
        // SVO now goes to the desired depth, so we can mutate the node now.
        nodes[node.idx as usize] = Node::new(voxel);

        // if the SVO's depth was already at the target depth,
        // we should check if this voxel is being set to the same value
        // as the adjacent nodes, and if so, set the parent and remove
        // this set of nodes to simplify the SVO.
        loop {
            if let Some(parent_node) = self.node_parent(nodes, &node) {
                node = parent_node;
            } else {
                break;
            }
            let parent_idx = node.idx;
            let idx = nodes[parent_idx as usize].child_idx();
            let child_nodes = &nodes[idx as usize..idx as usize + 8];
            if nodes_are_eq(&child_nodes) {
                alloc.free(idx as u32);
                nodes[parent_idx as usize] = Node::new(voxel);
            } else {
                break;
            }
        }
        Ok(())
    }
}

#[inline(always)]
fn nodes_are_eq(n: &[Node]) -> bool {
    n[0] == n[1]
        && n[0] == n[2]
        && n[0] == n[3]
        && n[0] == n[4]
        && n[0] == n[5]
        && n[0] == n[6]
        && n[0] == n[7]
}
