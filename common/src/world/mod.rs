pub mod noise;

use glam::{uvec3, IVec3, UVec3, Vec3};

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
#[allow(dead_code)] // Linter claims Voxel.0 is unused
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
pub fn world_to_chunk_pos(pos: IVec3) -> IVec3 {
    pos.div_euclid(IVec3::splat(CHUNK_SIZE as i32))
}

#[inline(always)]
pub fn world_to_inchunk_pos(pos: IVec3) -> UVec3 {
    (pos - (world_to_chunk_pos(pos) * CHUNK_SIZE as i32)).as_uvec3()
}

#[inline(always)]
pub fn chunk_to_world_pos(pos: IVec3) -> IVec3 {
    pos * CHUNK_SIZE as i32
}

#[inline(always)]
pub fn inchunk_to_world_pos(chunk: IVec3, pos: UVec3) -> IVec3 {
    (chunk * CHUNK_SIZE as i32) + pos.as_ivec3()
}

/// Represents a node in the sparse voxel octree (SVO) for each chunk.
///
/// # States
/// ## 0_______________xxxxxxxxxxxxxxxx
/// Entire node is occupied by voxel `x`.
///
/// ## 1yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy
/// Node splits into 8 nodes of half size at `y`.
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
        #[allow(dead_code)] // Fields used by #derive Debug
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
        assert!(used.end == free.start);
        Self {
            range: used.start..free.end,
            free_mem: vec![free],
            last_used_addr: used.end - 1,
        }
    }

    // `new_idx` should greater then `self.last_used_addr()`
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

    pub fn next(&mut self) -> Option<NodeAddr> {
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
            return None;
        }

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
            return None;
        }
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
    pub fn new(root: NodeAddr, size: u32) -> Self {
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
            idx = node.child_idx() + child_idx;
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
            idx = node.child_idx() + child_idx;
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
        // No need to break-down the SVO because the highest node has the same voxel type.
        if parent_voxel == voxel {
            return Ok(());
        }

        // If depth is less than target_depth,
        // the SVO doesn't go to desired depth, so we must split until it does
        while node.depth < target_depth {
            let first_child = alloc.next().ok_or(SetVoxelErr::OutOfMemory)?;
            nodes[first_child as usize..(first_child as usize + 8)]
                .copy_from_slice(&[Node::new(parent_voxel); 8]);

            nodes[node.idx as usize] = Node::new_split(first_child);
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
        // SVO now goes to desired depth, so we can mutate the node now.
        nodes[node.idx as usize] = Node::new(voxel);

        // if the SVO's depth was already at the target depth,
        // we should check if this voxel is being set to the same value
        // as the adjacent nodes, and if so, set the parent and remove
        // this set of nodes to simplify the SVO.
        loop {
            if let Some(parent_node) = self.node_parent(nodes, &node) {
                // println!("node {found_node:?} has parent {node:?}");
                node = parent_node;
            } else {
                break;
            }
            let parent_idx = node.idx;
            let idx = nodes[parent_idx as usize].child_idx();
            let child_nodes = &nodes[idx as usize..idx as usize + 8];
            if nodes_are_eq(&child_nodes) {
                alloc.free(idx);
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
    assert_eq!(n.len(), 8);
    n[0] == n[1]
        && n[0] == n[2]
        && n[0] == n[3]
        && n[0] == n[4]
        && n[0] == n[5]
        && n[0] == n[6]
        && n[0] == n[7]
}
