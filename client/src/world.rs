use crate::common::math::Aabb;
use crate::common::resources::VoxelPack;
use crate::common::world::*;
use glam::{uvec3, IVec3, UVec3};

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

    pub fn new(root: NodeAddr, used: NodeRange, free: NodeRange) -> Self {
        Self {
            range: (root + used.start)..(root + free.end),
            alloc: NodeAlloc::new(used, free),
        }
    }

    pub fn set_voxel(
        &mut self,
        nodes: &mut [Node],
        pos: VoxelPosInChunk,
        voxel: Voxel,
    ) -> Result<(), SetVoxelErr> {
        let nodes = &mut nodes[self.range.start as usize..self.range.end as usize];
        Svo::new(0, CHUNK_SIZE).set_node(nodes, *pos, voxel, CHUNK_DEPTH, &mut self.alloc)
    }

    pub fn get_voxel(&self, nodes: &[Node], pos: VoxelPosInChunk) -> Result<Voxel, SetVoxelErr> {
        let nodes = &nodes[self.range.start as usize..self.range.end as usize];
        let node = Svo::new(0, CHUNK_SIZE).find_node(nodes, *pos, CHUNK_DEPTH);
        Ok(nodes[node.idx as usize].voxel())
    }
}

pub struct ChunkGrid {
    min: ChunkPos,
    chunks: Box<[Option<Chunk>]>,
    size_in_chunks: u32,
}
impl ChunkGrid {
    #[inline(always)]
    const fn local_pos_to_idx(pos: UVec3, grid_size: u32) -> usize {
        (pos.x + pos.y * grid_size + pos.z * grid_size * grid_size) as usize
    }

    pub fn new(min: ChunkPos, size_in_chunks: u32) -> Self {
        let volume = (size_in_chunks * size_in_chunks * size_in_chunks) as usize;
        let chunks = vec![<Option<Chunk>>::None; volume].into_boxed_slice();

        Self { min, chunks, size_in_chunks }
    }

    pub fn local_pos_for(&self, pos: ChunkPos) -> Option<UVec3> {
        if pos.cmplt(*self.min).any() {
            None
        } else {
            Some((*pos - *self.min).as_uvec3())
        }
    }
    #[inline(always)] pub fn unlocal_pos_for(&self, pos: UVec3) -> ChunkPos {
        ChunkPos::new(pos.as_ivec3() + *self.min)
    }

    #[inline(always)] pub const fn chunk_count(&self) -> usize { self.chunks.len() }
    #[inline(always)] pub const fn size_in_voxels(&self) -> u32 { self.size_in_chunks * CHUNK_SIZE }
    #[inline(always)] pub const fn size_in_chunks(&self) -> u32 { self.size_in_chunks }

    #[inline(always)] pub const fn min_voxel(&self) -> VoxelPos { self.min.min() }
    #[inline(always)] pub const fn max_voxel(&self) -> VoxelPos { self.max_chunk().max() }

    #[inline(always)] pub const fn min_chunk(&self) -> ChunkPos { self.min }
    #[inline(always)] pub const fn max_chunk(&self) -> ChunkPos {
        ChunkPos(
            self.min.0.x + self.size_in_chunks as i32,
            self.min.0.y + self.size_in_chunks as i32,
            self.min.0.z + self.size_in_chunks as i32,
        )
    }

    pub fn shift_chunks(&mut self, offset: IVec3, removed_chunks: &mut Vec<(ChunkPos, Chunk)>) {
        let mut new_chunks = vec![None; self.chunks.len()].into_boxed_slice();

        let min = IVec3::ZERO;
        let max = IVec3::splat(self.size_in_chunks as i32);

        for x in 0..self.size_in_chunks {
            for y in 0..self.size_in_chunks {
                for z in 0..self.size_in_chunks {
                    let pos = uvec3(x, y, z);

                    let idx = Self::local_pos_to_idx(pos, self.size_in_chunks);

                    let dst_pos = pos.as_ivec3() - offset;
                    if dst_pos.cmplt(min).any() || dst_pos.cmpge(max).any() {
                        if let Some(chunk) = self.chunks[idx].clone() {
                            removed_chunks.push((self.unlocal_pos_for(pos), chunk));
                        }
                        continue;
                    }
                    let dst_idx = Self::local_pos_to_idx(dst_pos.as_uvec3(), self.size_in_chunks);
                    new_chunks[dst_idx] = self.chunks[idx].clone();
                }
            }
        }
        self.chunks = new_chunks;
    }

    pub fn chunk_roots(&self) -> Vec<NodeAddr> {
        self.chunks
            .iter()
            .map(|chunk| chunk.as_ref().map(|c| c.range.start).unwrap_or(0))
            .collect()
    }

    pub fn populated_count(&self) -> usize {
        let mut r = 0;
        for chunk in &self.chunks {
            r += chunk.is_some() as usize;
        }
        r
    }

    pub fn empty_chunks(&self) -> Vec<ChunkPos> {
        let mut chunks = Vec::new();
        for x in 0..self.size_in_chunks {
            for y in 0..self.size_in_chunks {
                for z in 0..self.size_in_chunks {
                    let pos = uvec3(x, y, z);
                    let idx = Self::local_pos_to_idx(pos, self.size_in_chunks);
                    if self.chunks[idx].is_none() {
                        chunks.push(ChunkPos::new(pos.as_ivec3() + *self.min));
                    }
                }
            }
        }
        chunks
    }

    pub fn set_chunk(&mut self, pos: ChunkPos, chunk: Chunk) -> Option<()> {
        let local_pos = self.local_pos_for(pos)?;
        let idx = Self::local_pos_to_idx(local_pos, self.size_in_chunks);
        self.chunks[idx] = Some(chunk);
        Some(())
    }
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        let local_pos = self.local_pos_for(pos)?;
        let idx = Self::local_pos_to_idx(local_pos, self.size_in_chunks);
        self.chunks.get(idx)?.as_ref()
    }
    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        let local_pos = self.local_pos_for(pos)?;
        let idx = Self::local_pos_to_idx(local_pos, self.size_in_chunks);
        self.chunks.get_mut(idx)?.as_mut()
    }
}

pub struct ChunkAlloc {
    free_mem: Vec<NodeRange>,
    max_nodes: u32,
}
impl ChunkAlloc {
    pub fn new(max_nodes: u32) -> Self {
        Self {
            free_mem: vec![1..max_nodes],
            max_nodes,
        }
    }

    pub fn status(&self) -> (u32, u32) {
        let mut total_free = 0;
        for free in &self.free_mem {
            total_free += free.len();
        }
        (total_free as u32, self.max_nodes)
    }

    pub fn free_chunk(&mut self, root: u32, size: u32) {
        let range = root..root + size;
        // check if this span can be extended from an existing free memory span
        for free in &mut self.free_mem {
            if free.start == range.end {
                free.start -= size;
                return;
            }
            if free.end == root {
                free.end += size;
                return;
            }
        }
        self.free_mem.push(range);
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
        let chunk_space = space.start..(space.start + req_space);
        space.start = chunk_space.end;
        Chunk::new(chunk_space.start, 0..size, size..req_space)
    }
}

pub struct ClientWorld {
    chunks: ChunkGrid,
    nodes: Box<[Node]>,
    chunk_alloc: ChunkAlloc,
}
impl std::ops::Deref for ClientWorld {
    type Target = ChunkGrid;
    fn deref(&self) -> &ChunkGrid { &self.chunks }
}
impl std::ops::DerefMut for ClientWorld {
    fn deref_mut(&mut self) -> &mut ChunkGrid { &mut self.chunks }
}
impl ClientWorld {
    pub fn new(min_chunk: ChunkPos, max_nodes: u32, size: u32) -> Self {
        let mut nodes = vec![Node::EMPTY; max_nodes as usize].into_boxed_slice();
        nodes[0] = Node::new(Voxel::EMPTY); // 0 = air
        Self {
            chunks: ChunkGrid::new(min_chunk, size),
            nodes,
            chunk_alloc: ChunkAlloc::new(max_nodes),
        }
    }

    pub fn free_chunk(&mut self, chunk: Chunk) -> Option<()> {
        self.chunk_alloc
            .free_chunk(chunk.range.start, chunk.range.len() as u32);
        Some(())
    }

    pub fn chunk_alloc_status(&self) -> (u32, u32) {
        self.chunk_alloc.status()
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
}
impl ClientWorld {
    pub fn center_chunks(&mut self, anchor: ChunkPos, removed_chunks: &mut Vec<(ChunkPos, Chunk)>) {
        let curr_min_chunk = self.min;
        let new_min_chunk = ChunkPos::new(*anchor - IVec3::splat(self.size_in_chunks() as i32 / 2));

        if curr_min_chunk == new_min_chunk {
            return
        }
        self.min = new_min_chunk;

        let chunk_offset = *new_min_chunk - *curr_min_chunk;
        self.chunks.shift_chunks(chunk_offset, removed_chunks);
    }

    pub fn create_chunk(&mut self, pos: ChunkPos, nodes: &[Node]) -> Result<NodeAddr, SetVoxelErr> {
        if pos.cmplt(*self.min_chunk()).any() || pos.cmpge(*self.max_chunk()).any() {
            return Err(SetVoxelErr::PosOutOfBounds);
        }

        if let Some(chunk) = self.chunks.get_chunk_mut(pos) {
            if chunk.range.len() >= nodes.len() {
                let start = chunk.range.start as usize;
                let end = start + nodes.len();

                self.nodes[start..end].copy_from_slice(&nodes);

                // The addresses used in NodeAlloc are relative to the chunk root.
                chunk.alloc = NodeAlloc::new(0..nodes.len() as u32, nodes.len() as u32..chunk.range.len() as u32);

                return Ok(start as u32);
            }
        }

        let chunk = self.chunk_alloc.alloc_chunk(nodes.len() as u32);
        let range = chunk.range.start..(chunk.range.start + nodes.len() as u32);

        self.nodes[(range.start as usize)..(range.end as usize)].copy_from_slice(&nodes);
        self.chunks.set_chunk(pos, chunk);
        Ok(range.start)
    }

    fn check_bounds(&self, pos: VoxelPos) -> Result<(), SetVoxelErr> {
        if pos.cmplt(*self.min_voxel()).any() || pos.cmpge(*self.max_voxel()).any() {
            return Err(SetVoxelErr::PosOutOfBounds);
        }
        Ok(())
    }

    pub fn set_voxel(&mut self, pos: VoxelPos, voxel: Voxel) -> Result<&Chunk, SetVoxelErr> {
        self.check_bounds(pos)?;
        let (chunk_pos, pos_in_chunk) = pos.chunk();
        let chunk = self.chunks.get_chunk_mut(chunk_pos).ok_or(SetVoxelErr::NoChunk)?;
        chunk.set_voxel(&mut self.nodes, pos_in_chunk, voxel)?;
        Ok(chunk)
    }

    pub fn get_voxel(&self, pos: VoxelPos) -> Result<Voxel, SetVoxelErr> {
        self.check_bounds(pos)?;
        let (chunk_pos, pos_in_chunk) = pos.chunk();
        let chunk = self.chunks.get_chunk(chunk_pos).ok_or(SetVoxelErr::NoChunk)?;
        chunk.get_voxel(&self.nodes, pos_in_chunk)
    }

    pub fn highest_vox_at(&self, pos: VoxelPos) -> Option<i32> {
        for y in (self.min_voxel().y..self.max_voxel().y).rev() {
            if self.get_voxel(VoxelPos(pos.x, y, pos.z)).map(Voxel::is_empty) == Ok(false) {
                return Some(y);
            }
        }
        None
    }
}
impl ClientWorld {
    pub fn get_collisions_w(&self, aabb: &Aabb, voxelpack: &VoxelPack) -> Vec<Aabb> {
        let mut aabbs = Vec::new();

        let from = aabb.from.floor().as_ivec3();
        let to = aabb.to.ceil().as_ivec3();

        for x in from.x..to.x {
            for y in from.y..to.y {
                for z in from.z..to.z {
                    let pos = VoxelPos(x, y, z);

                    let voxel = self.get_voxel(pos).unwrap_or(Voxel::EMPTY);

                    if voxelpack.get(voxel).unwrap().is_solid() {
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
