use crate::Resources;
pub use common::world::{noise::NoiseMap, *};
use glam::{uvec3, vec2, IVec3, UVec3};
use std::collections::HashMap;

pub struct ServerWorld {
    pub chunks: HashMap<IVec3, ServerChunk>,
    pub noise: NoiseMap,
}
impl ServerWorld {
    pub fn new() -> Self {
        noise::init_gradients();
        Self {
            chunks: HashMap::new(),
            noise: NoiseMap::new(fastrand::i64(..), 0.03, 50.0),
        }
    }

    pub fn get_chunk(&self, pos: IVec3) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }

    pub fn create_chunk(&mut self, pos: IVec3, res: &Resources) {
        let chunk = ServerChunk::new();
        self.chunks.insert(pos, chunk);
    }

    pub fn create_dev_chunk(&mut self, chunk_pos: IVec3, res: &Resources) {
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

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_pos = inchunk_to_world_pos(chunk_pos, uvec3(x, 0, z));
                let h = self.noise.get(vec2(world_pos.x as f32, world_pos.z as f32)) as u32;

                for y in 0u32..CHUNK_SIZE {
                    if y as i32 + world_pos.y < h as i32 {
                        set_vox(uvec3(x, y, z), "stone");
                    } else if y as i32 + world_pos.y == h as i32 {
                        set_vox(uvec3(x, y, z), "grass");
                    }
                }
            }
        }
        self.chunks.insert(chunk_pos, chunk);
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
