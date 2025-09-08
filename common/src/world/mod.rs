use glam::{uvec3, IVec3, UVec3};

pub type NodeAddr = u32;
pub type NodeRange = std::ops::Range<NodeAddr>;

#[derive(Debug)]
pub enum SetVoxelErr {
    PosOutOfBounds,
    OutOfMemory,
    NoChunk,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Voxel(u16);
impl Voxel {
    pub const EMPTY: Self = Self(0);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn as_data(self) -> u16 {
        self.0
    }

    pub fn from_data(byte: u16) -> Self {
        Self(byte)
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

/// When adding a chunk to the world, this is the number of extra nodes the chunk makes room for.
/// When a chunk needs to use more than this amount more of extra memory for storing nodes,
/// The chunk will have to be re-located in memory.
pub const CHUNK_INIT_FREE_MEM: u32 = 256;

#[inline(always)]
pub fn vox_to_chunk_pos(pos: IVec3) -> IVec3 {
    pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32))
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
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Node(u32);
impl Node {
    // same as Node::new(Voxel::EMPTY)
    pub const ZERO: Self = Self(0);

    const SPLIT_MASK: u32 = 0x8000_0000;
    const DATA_MASK: u32 = 0x7FFF_FFFF;

    pub fn new(vox: Voxel) -> Self {
        Self((vox.as_data() as u32) & Self::DATA_MASK)
    }
    pub fn new_split(child_idx: u32) -> Self {
        Self(child_idx | Self::SPLIT_MASK)
    }

    pub fn voxel(self) -> Voxel {
        Voxel::from_data((self.0 & 0xFFFF) as u16)
    }
    pub fn set_voxel(&mut self, voxel: Voxel) {
        self.0 = (self.0 & Self::SPLIT_MASK) | voxel.as_data() as u32;
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

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        enum NodeEnum {
            Split(u32),
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

pub trait NodeAllocImpl {
    fn next(&mut self) -> Option<NodeAddr>;
    fn free(&mut self, addr: NodeAddr);
}

#[derive(Clone, Debug)]
pub struct NodeAlloc {
    // The range in the Node list that this chunk occupies.
    pub range: NodeRange,

    // Spans of free memory where this allocator is able to place new nodes.
    pub free_mem: Vec<NodeRange>,
}
impl NodeAlloc {
    pub fn new(used: NodeRange, free: NodeRange) -> Self {
        assert!(used.end == free.start);
        Self {
            range: used.start..free.end,
            free_mem: vec![free],
        }
    }

    pub fn next(&mut self) -> Option<NodeAddr> {
        // Assuming theres only ever one free_mem NodeRange for now.
        let free = &mut self.free_mem[0];

        if free.end - free.start < 8 {
            return None;
        }
        let result_addr = free.start;
        free.start += 8;
        Some(result_addr)
    }

    pub fn free(&mut self, _addr: NodeAddr) {
        unimplemented!()
    }

    pub fn available_space(&self) -> u32 {
        // Assuming theres only ever one free_mem NodeRange for now.
        self.free_mem[0].end - self.free_mem[0].start
    }

    // The highest-most address that is being used to store data.
    pub fn last_used_addr(&self) -> u32 {
        // Assuming theres only ever one free_mem NodeRange for now.
        self.free_mem[0].start - 1
    }
}

pub struct SvoRef<'a> {
    pub nodes: &'a [Node],
    pub root: NodeAddr,
    pub size: u32,
}
pub struct SvoMut<'a> {
    pub nodes: &'a mut [Node],
    pub root: NodeAddr,
    pub size: u32,
}
impl<'a> SvoMut<'a> {
    pub fn as_ref(&'a self) -> SvoRef<'a> {
        SvoRef {
            nodes: &*self.nodes,
            root: self.root,
            size: self.size,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FoundNode {
    pub idx: NodeAddr,
    pub depth: u32,
    pub center: UVec3,
    pub size: u32,
}

pub fn find_svo_node(svo: &SvoRef, pos: UVec3, max_depth: u32) -> FoundNode {
    let mut size = svo.size;
    let mut idx = svo.root;
    let mut center = UVec3::splat(size / 2);
    let mut depth: u32 = 0;

    loop {
        let node = svo.nodes[idx as usize];
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

pub fn set_svo_voxel(
    svo: &mut SvoMut,
    pos: UVec3,
    voxel: Voxel,
    target_depth: u32,
    alloc: &mut NodeAlloc,
) -> Result<(), SetVoxelErr> {
    let FoundNode {
        mut idx,
        mut center,
        mut size,
        depth,
        ..
    } = find_svo_node(&svo.as_ref(), pos, target_depth);

    let parent_voxel = svo.nodes[idx as usize].voxel();

    // If depth is less than target_depth,
    // the SVO doesn't go to desired depth, so we must split until it does
    for _ in depth..target_depth {
        let first_child = alloc.next().ok_or(SetVoxelErr::OutOfMemory)?;
        svo.nodes[first_child as usize..(first_child as usize + 8)]
            .copy_from_slice(&[Node::new(parent_voxel); 8]);

        svo.nodes[idx as usize] = Node::new_split(first_child);
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
    svo.nodes[idx as usize] = Node::new(voxel);
    Ok(())
}
