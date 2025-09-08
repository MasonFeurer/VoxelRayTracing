use crate::Resources;
pub use common::world::*;
use glam::{uvec3, IVec3, UVec3};
use std::collections::HashMap;

pub struct ServerWorld {
    pub chunks: HashMap<IVec3, ServerChunk>,
}
impl ServerWorld {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    pub fn get_chunk(&self, pos: IVec3) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }

    pub fn create_chunk(&mut self, pos: IVec3, res: &Resources) {
        let chunk = ServerChunk::new();
        self.chunks.insert(pos, chunk);
    }

    pub fn create_dev_chunk(&mut self, pos: IVec3, res: &Resources) {
        let mut chunk = ServerChunk::with_capacity(16384 * 2 * 2);
        let mut alloc = &mut chunk.node_alloc;
        let mut svo = SvoMut {
            nodes: &mut chunk.nodes,
            root: 0,
            size: CHUNK_SIZE,
        };

        let mut set_vox = |pos: UVec3, name: &str| {
            set_svo_voxel(
                &mut svo,
                pos,
                res.voxelpack.by_name(name).unwrap(),
                CHUNK_DEPTH,
                alloc,
            );
        };

        if pos.y == 0 {
            for x in 1..(CHUNK_SIZE - 1) {
                for z in 1..(CHUNK_SIZE - 1) {
                    set_vox(uvec3(x, 0, z), "stone");
                    set_vox(uvec3(x, 1, z), "dirt");
                    set_vox(uvec3(x, 2, z), "grass");

                    if fastrand::u8(0..10) == 0 {
                        set_vox(uvec3(x, 3, z), "snow");
                    }
                }
            }

            for y in 0..pos.x as u32 {
                set_vox(uvec3(3, y + 3, 3), "clay");
            }
            for y in 0..pos.z as u32 {
                set_vox(uvec3(4, y + 3, 3), "mud");
            }
        }

        // if pos.x == 1 && pos.z == 2 {
        //     set_vox(uvec3(3, 4, 3), "stone")
        // }

        self.chunks.insert(pos, chunk);
    }
}

pub struct ServerChunk {
    pub nodes: Vec<Node>,
    pub node_alloc: NodeAlloc,
}
impl std::fmt::Debug for ServerChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        struct Chunk<'a> {
            nodes: &'a [Node],
        }
        let c = Chunk {
            nodes: self.used_nodes().clone(),
        };
        std::fmt::Debug::fmt(&c, f)
    }
}
impl ServerChunk {
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    pub fn with_capacity(cap: u32) -> Self {
        let mut nodes = vec![Node::ZERO; cap as usize];
        nodes[0] = Node::new(Voxel::from_data(0));
        Self {
            nodes,
            node_alloc: NodeAlloc::new(0..1, 1..cap),
        }
    }

    pub fn used_nodes(&self) -> &[Node] {
        &self.nodes[0..=self.node_alloc.last_used_addr() as usize]
    }
}
