use crate::Resources;
pub use common::world::*;
use glam::IVec3;
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
            nodes: &self.nodes[0..=self.node_alloc.last_used_addr() as usize],
        };
        std::fmt::Debug::fmt(&c, f)
    }
}
impl ServerChunk {
    pub fn new() -> Self {
        let cap: u32 = 256;

        let mut nodes = vec![Node::ZERO; cap as usize];
        nodes[0] = Node::new(Voxel::from_data(1));
        Self {
            nodes,
            node_alloc: NodeAlloc::new(0..1, 1..cap),
        }
    }
}
