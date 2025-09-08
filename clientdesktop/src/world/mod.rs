use client::common::math::Aabb;
use client::common::world::*;
use glam::{ivec3, uvec3, IVec3, UVec3};
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub struct ChunkPtr(usize, UVec3, u32);
impl ChunkPtr {
    pub fn idx(self) -> usize {
        self.0
    }

    pub fn pos(self) -> UVec3 {
        self.1
    }

    pub fn world_size(self) -> u32 {
        self.2
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
        let mut svo = SvoMut {
            nodes,
            root: self.range.start,
            size: CHUNK_SIZE,
        };

        set_svo_voxel(&mut svo, pos, voxel, CHUNK_DEPTH, &mut self.alloc)
    }

    pub fn get_voxel(&self, nodes: &[Node], pos: UVec3) -> Result<Voxel, SetVoxelErr> {
        // TODO: bounds checks

        let svo = SvoRef {
            nodes,
            root: self.range.start,
            size: CHUNK_SIZE,
        };

        let idx = find_svo_node(&svo, pos, CHUNK_DEPTH).idx;
        Ok(nodes[idx as usize].voxel())
    }
}

pub struct ChunkGrid {
    pub chunks: Box<[Option<Chunk>]>,
    // A table that stores the local-position of every chunk at the given indices.
    pub chunk_pos_map: Box<[UVec3]>,
    pub chunk_count: usize,
    pub size: u32,
}
impl ChunkGrid {
    pub fn new(size: u32) -> Self {
        let volume = (size * size * size) as usize;
        let chunks = vec![<Option<Chunk>>::None; volume].into_boxed_slice();
        let mut chunk_pos_map = vec![UVec3::ZERO; volume].into_boxed_slice();
        for x in 0..size {
            for y in 0..size {
                for z in 0..size {
                    let idx = x + y * size + z * size * size;
                    chunk_pos_map[idx as usize] = UVec3 { x, y, z };
                }
            }
        }

        Self {
            chunks,
            chunk_pos_map,
            chunk_count: volume,
            size,
        }
    }

    pub fn empty_chunks<'a>(&'a self) -> impl Iterator<Item = ChunkPtr> + 'a {
        self.chunks
            .iter()
            .enumerate()
            .filter(|(_idx, chunk)| chunk.is_some())
            .map(|(idx, _)| ChunkPtr(idx, self.chunk_pos_map[idx], self.size))
    }

    pub fn chunk_at(&self, pos: UVec3) -> Option<ChunkPtr> {
        // TODO: bounds checks
        let idx = pos.x + pos.y * self.size + pos.z * self.size * self.size;
        Some(ChunkPtr(idx as usize, pos, self.size))
    }

    pub fn put_chunk(&mut self, ptr: ChunkPtr, chunk: Chunk) {
        // TODO: make sure ptr is valid (world may have been resized since ptr was created)
        self.chunks[ptr.idx()] = Some(chunk);
    }
    pub fn get_chunk(&self, ptr: ChunkPtr) -> Option<&Chunk> {
        // TODO: make sure ptr is valid (world may have been resized since ptr was created)
        self.chunks.get(ptr.idx())?.as_ref()
    }
    pub fn get_chunk_mut(&mut self, ptr: ChunkPtr) -> Option<&mut Chunk> {
        // TODO: make sure ptr is valid (world may have been resized since ptr was created)
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
impl std::ops::Deref for World {
    type Target = ChunkGrid;
    fn deref(&self) -> &ChunkGrid {
        &self.chunks
    }
}
impl World {
    pub fn new(origin: IVec3, max_nodes: u32, size: u32) -> Self {
        let mut nodes = vec![Node::ZERO; max_nodes as usize].into_boxed_slice();
        nodes[0] = Node::new(Voxel::EMPTY); // 0 = air
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

    pub fn create_chunk(&mut self, pos: UVec3, nodes: &[Node]) -> Result<(), SetVoxelErr> {
        let chunk_ptr = self.chunk_at(pos).ok_or(SetVoxelErr::PosOutOfBounds)?;

        let chunk = self.chunk_alloc.alloc_chunk(nodes.len() as u32);
        let range = chunk.range.start..(chunk.range.start + nodes.len() as u32);

        self.nodes[(range.start as usize)..(range.end as usize)].copy_from_slice(&nodes);
        self.chunks.put_chunk(chunk_ptr, chunk);
        Ok(())
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<(), SetVoxelErr> {
        // TODO: check bounds
        let pos = (pos - self.origin).as_uvec3();
        let chunk_pos = vox_to_chunk_pos(pos.as_ivec3()).as_uvec3();
        let pos_in_chunk = pos - (chunk_pos * CHUNK_SIZE);

        let chunk = self.chunk_at(chunk_pos).unwrap();
        let chunk = self.chunks.chunks[chunk.idx()]
            .as_mut()
            .ok_or(SetVoxelErr::NoChunk)?;
        chunk.set_voxel(&mut self.nodes, pos_in_chunk, voxel)
    }

    pub fn get_voxel(&self, pos: IVec3) -> Result<Voxel, SetVoxelErr> {
        // TODO: check bounds
        let pos = (pos - self.origin).as_uvec3();
        let chunk_pos = vox_to_chunk_pos(pos.as_ivec3()).as_uvec3();
        let pos_in_chunk = pos - (chunk_pos * CHUNK_SIZE);

        let chunk = self.chunk_at(chunk_pos).unwrap();
        let chunk = self.chunks.chunks[chunk.idx()]
            .as_ref()
            .ok_or(SetVoxelErr::NoChunk)?;
        chunk.get_voxel(&self.nodes, pos_in_chunk)
    }
}
impl World {
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
