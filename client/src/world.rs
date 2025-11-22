use crate::common::math::Aabb;
use crate::common::world::*;
use glam::{ivec3, uvec3, IVec3, UVec3};

#[derive(Clone, Debug)]
pub struct ChunkPtr {
    cycle: u32,
    idx: usize,
    local_pos: UVec3,
}
impl ChunkPtr {
    fn new(grid_size: u32, cycle: u32, idx: usize) -> Self {
        Self {
            cycle,
            idx,
            local_pos: ChunkGrid::idx_to_local_pos(idx, grid_size),
        }
    }

    pub fn idx(&self) -> usize {
        self.idx
    }

    pub fn local_pos(&self) -> UVec3 {
        self.local_pos
    }
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

    pub fn set_voxel(
        &mut self,
        nodes: &mut [Node],
        pos: UVec3,
        voxel: Voxel,
    ) -> Result<(), SetVoxelErr> {
        let nodes = &mut nodes[self.range.start as usize..self.range.end as usize];
        let mut svo = SvoMut {
            nodes,
            root: self.range.start,
            size: CHUNK_SIZE,
        };

        set_svo_voxel(&mut svo, pos, voxel, CHUNK_DEPTH, &mut self.alloc)
    }

    pub fn get_voxel(&self, nodes: &[Node], pos: UVec3) -> Result<Voxel, SetVoxelErr> {
        // TODO: bounds checks
        let nodes = &nodes[self.range.start as usize..self.range.end as usize];
        let svo = SvoRef {
            nodes,
            root: 0,
            size: CHUNK_SIZE,
        };

        let idx = find_svo_node(&svo, pos, CHUNK_DEPTH).idx;
        Ok(nodes[idx as usize].voxel())
    }
}

pub struct ChunkGrid {
    chunks: Box<[Option<Chunk>]>,
    chunk_count: usize,
    size: u32,
    cycle: u32,
}
impl ChunkGrid {
    fn local_pos_to_idx(pos: UVec3, grid_size: u32) -> usize {
        (pos.x + pos.y * grid_size + pos.z * grid_size * grid_size) as usize
    }
    fn idx_to_local_pos(idx: usize, grid_size: u32) -> UVec3 {
        let mut idx = idx as u32;
        let z = idx / (grid_size * grid_size);
        idx -= z * grid_size * grid_size;
        let y = idx / grid_size;
        idx -= y * grid_size;
        let x = idx;
        uvec3(x, y, z)
    }

    pub fn new(size: u32) -> Self {
        let volume = (size * size * size) as usize;
        let chunks = vec![<Option<Chunk>>::None; volume].into_boxed_slice();

        Self {
            chunks,
            chunk_count: volume,
            size,
            cycle: 0,
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    pub fn shift_chunks(&mut self, offset: IVec3) {
        let mut new_chunks = vec![None; self.chunks.len()].into_boxed_slice();

        let min = IVec3::ZERO;
        let max = IVec3::splat(self.size as i32);

        for x in 0..self.size {
            for y in 0..self.size {
                for z in 0..self.size {
                    let pos = uvec3(x as u32, y as u32, z as u32);

                    let idx = Self::local_pos_to_idx(pos, self.size);

                    let dst_pos = pos.as_ivec3() - offset;
                    if dst_pos.cmplt(min).any() || dst_pos.cmpge(max).any() {
                        continue;
                    }
                    let dst_idx = Self::local_pos_to_idx(dst_pos.as_uvec3(), self.size);
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

    pub fn empty_chunks<'a>(&'a self) -> impl Iterator<Item = ChunkPtr> + 'a {
        self.chunks
            .iter()
            .enumerate()
            .filter(|(_idx, chunk)| chunk.is_none())
            .map(|(idx, _)| ChunkPtr::new(self.size, self.cycle, idx))
    }

    pub fn chunk_at(&self, pos: UVec3) -> Option<ChunkPtr> {
        // TODO: bounds checks
        let idx = pos.x + pos.y * self.size + pos.z * self.size * self.size;
        Some(ChunkPtr::new(self.size, self.cycle, idx as usize))
    }

    pub fn put_chunk(&mut self, ptr: &ChunkPtr, chunk: Chunk) {
        if ptr.cycle != self.cycle {
            return;
        }
        self.chunks[ptr.idx()] = Some(chunk);
    }
    pub fn get_chunk(&self, ptr: &ChunkPtr) -> Option<&Chunk> {
        if ptr.cycle != self.cycle {
            return None;
        }
        self.chunks.get(ptr.idx())?.as_ref()
    }
    pub fn get_chunk_mut(&mut self, ptr: &ChunkPtr) -> Option<&mut Chunk> {
        if ptr.cycle != self.cycle {
            return None;
        }
        self.chunks.get_mut(ptr.idx())?.as_mut()
    }

    pub fn resize(&mut self, _new_size: u32) {
        todo!()
    }

    pub fn clear(&mut self) {
        todo!()
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

pub struct ClientWorld {
    min_chunk: IVec3,
    size_in_chunks: u32,
    chunks: ChunkGrid,
    nodes: Box<[Node]>,
    chunk_alloc: ChunkAlloc,
}
impl std::ops::Deref for ClientWorld {
    type Target = ChunkGrid;
    fn deref(&self) -> &ChunkGrid {
        &self.chunks
    }
}
impl ClientWorld {
    pub fn new(min_chunk: IVec3, max_nodes: u32, size: u32) -> Self {
        let mut nodes = vec![Node::ZERO; max_nodes as usize].into_boxed_slice();
        nodes[0] = Node::new(Voxel::EMPTY); // 0 = air
        Self {
            min_chunk,
            size_in_chunks: size,
            chunks: ChunkGrid::new(size),
            nodes,
            chunk_alloc: ChunkAlloc::new(max_nodes),
        }
    }

    pub fn chunk_alloc_status(&self) -> (u32, u32) {
        self.chunk_alloc.status()
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

    pub fn min_voxel(&self) -> IVec3 {
        chunk_to_world_pos(self.min_chunk)
    }
    pub fn max_voxel(&self) -> IVec3 {
        self.min_voxel() + IVec3::splat(self.size() as i32)
    }

    pub fn min_chunk(&self) -> IVec3 {
        self.min_chunk
    }
    pub fn max_chunk(&self) -> IVec3 {
        self.min_chunk + IVec3::splat(self.size_in_chunks as i32)
    }
}
impl ClientWorld {
    pub fn center_chunks(&mut self, anchor: IVec3) {
        let anchor_chunk = world_to_chunk_pos(anchor);
        let curr_min_chunk = self.min_chunk();
        let new_min_chunk = anchor_chunk - IVec3::splat(self.size as i32 / 2);

        if curr_min_chunk == new_min_chunk {
            return;
        }
        self.min_chunk = new_min_chunk;

        let chunk_offset = new_min_chunk - curr_min_chunk;
        self.chunks.shift_chunks(chunk_offset)
    }

    pub fn create_chunk(&mut self, pos: IVec3, nodes: &[Node]) -> Result<NodeAddr, SetVoxelErr> {
        if pos.cmplt(self.min_chunk()).any() || pos.cmpge(self.max_chunk()).any() {
            return Err(SetVoxelErr::PosOutOfBounds);
        }
        let local_pos = (pos - self.min_chunk()).as_uvec3();
        let chunk_ptr = self
            .chunk_at(local_pos)
            .ok_or(SetVoxelErr::PosOutOfBounds)?;
        if let Some(chunk) = self.chunks.get_chunk_mut(&chunk_ptr) {
            if chunk.range.len() >= nodes.len() {
                let start = chunk.range.start as usize;
                let end = start + nodes.len();

                self.nodes[start..end].copy_from_slice(&nodes);
                chunk.alloc = NodeAlloc::new(start as u32..end as u32, end as u32..chunk.range.end);
                return Ok(start as u32);
            }
        }

        let chunk = self.chunk_alloc.alloc_chunk(nodes.len() as u32);
        let range = chunk.range.start..(chunk.range.start + nodes.len() as u32);

        self.nodes[(range.start as usize)..(range.end as usize)].copy_from_slice(&nodes);
        self.chunks.put_chunk(&chunk_ptr, chunk);
        Ok(range.start)
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<(), SetVoxelErr> {
        if pos.cmplt(self.min_voxel()).any() || pos.cmpge(self.max_voxel()).any() {
            return Err(SetVoxelErr::PosOutOfBounds);
        }

        let pos = (pos - self.min_voxel()).as_uvec3();
        let chunk_pos = world_to_chunk_pos(pos.as_ivec3()).as_uvec3();
        let pos_in_chunk = pos - (chunk_pos * CHUNK_SIZE);

        let chunk = self.chunk_at(chunk_pos).unwrap();
        let chunk = self.chunks.chunks[chunk.idx()]
            .as_mut()
            .ok_or(SetVoxelErr::NoChunk)?;
        chunk.set_voxel(&mut self.nodes, pos_in_chunk, voxel)
    }

    pub fn get_voxel(&self, pos: IVec3) -> Result<Voxel, SetVoxelErr> {
        if pos.cmplt(self.min_voxel()).any() || pos.cmpge(self.max_voxel()).any() {
            return Err(SetVoxelErr::PosOutOfBounds);
        }

        let pos = (pos - self.min_voxel()).as_uvec3();
        let chunk_pos = world_to_chunk_pos(pos.as_ivec3()).as_uvec3();
        let pos_in_chunk = pos - (chunk_pos * CHUNK_SIZE);

        let chunk = self.chunk_at(chunk_pos).unwrap();
        let chunk = self.chunks.chunks[chunk.idx()]
            .as_ref()
            .ok_or(SetVoxelErr::NoChunk)?;
        let vox = chunk.get_voxel(&self.nodes, pos_in_chunk);
        vox
    }
}
impl ClientWorld {
    pub fn get_collisions_w(&self, aabb: &Aabb) -> Vec<Aabb> {
        let mut aabbs = Vec::new();

        let from = aabb.from.floor().as_ivec3();
        let to = aabb.to.ceil().as_ivec3();

        for x in from.x..to.x {
            for y in from.y..to.y {
                for z in from.z..to.z {
                    let pos = ivec3(x, y, z);

                    let voxel = self.get_voxel(pos).unwrap_or(Voxel::EMPTY);

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
