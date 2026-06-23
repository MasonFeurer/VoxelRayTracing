pub mod gen;

use common::resources::{Biome, WorldFeatures, WorldPreset};
use common::world::{ChunkPos, Node, NodeAlloc, SetVoxelErr, Svo, Voxel, VoxelPos, VoxelPosInChunk, CHUNK_DEPTH, CHUNK_SIZE};
use gen::{BuiltFeature, WorldGen};
use std::collections::HashMap;
use std::sync::{Arc};
use common::log::warn;

pub trait WorldFsExt {
    fn read_chunk(&self, pos: ChunkPos) -> Option<ServerChunk>;
}

pub struct ServerWorld {
    pub chunks: HashMap<ChunkPos, ServerChunk>,
    pub unplaced_features: Vec<BuiltFeature>,
    pub gen: Arc<WorldGen>,
}
impl ServerWorld {
    pub fn new(preset: &WorldPreset, features: WorldFeatures, seed: i64) -> Self {
        Self {
            chunks: HashMap::new(),
            unplaced_features: Vec::new(),
            gen: Arc::new(WorldGen::new(preset, features, seed)),
        }
    }

    pub fn place_features(&mut self, mut dirty_chunk: impl FnMut(ChunkPos)) {
        'f: for feature_idx in (0..self.unplaced_features.len()).rev() {
            let feature = &self.unplaced_features[feature_idx];

            let (min_chunk, max_chunk) = (feature.min(), feature.max());

            // make sure all chunks covered by the feature exist before placing.
            for x in min_chunk.x..=max_chunk.x {
                for y in min_chunk.y..=max_chunk.y {
                    for z in min_chunk.z..=max_chunk.z {
                        if self.get_chunk(ChunkPos(x, y, z)).is_none() {
                            continue 'f;
                        }
                    }
                }
            }

            for (pos, voxel) in feature.voxel_placements() {
                match set_voxel_w_chunks(&mut self.chunks, pos, voxel) {
                    Ok(()) => _ = dirty_chunk(pos.chunk().0),
                    Err(err) => {
                        warn!("Failed to place voxel for feature {feature_idx} at {pos:?} : {err:?}");
                    }
                }
            }
            _ = self.unplaced_features.remove(feature_idx);
        }
    }

    pub fn biome_at(&self, x: i32, z: i32) -> &Biome {
        self.gen.biome_at(x, z)
    }

    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }

    pub fn set_voxel(&mut self, pos: VoxelPos, voxel: Voxel) -> Result<(), SetVoxelErr> {
        set_voxel_w_chunks(&mut self.chunks, pos, voxel)
    }
}

pub fn set_voxel_w_chunks(
    chunks: &mut HashMap<ChunkPos, ServerChunk>,
    pos: VoxelPos,
    voxel: Voxel,
) -> Result<(), SetVoxelErr> {
    let (chunk_pos, pos_in_chunk) = pos.chunk();
    let chunk = chunks
        .get_mut(&chunk_pos)
        .ok_or(SetVoxelErr::PosOutOfBounds)?;
    chunk.set_voxel(pos_in_chunk, voxel)
}

#[derive(Clone)]
pub struct ServerChunk {
    pub nodes: Vec<Node>,
    pub node_alloc: NodeAlloc,
}
/// Constructors
impl ServerChunk {
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        Self {
            node_alloc: NodeAlloc::new(0..nodes.len() as u32, nodes.len() as u32..nodes.len() as u32 + 256),
            nodes,
        }
    }

    pub fn with_capacity(cap: u32) -> Self {
        let mut nodes = vec![Node::EMPTY; cap as usize];
        nodes[0] = Node::new(Voxel::from_data(0));
        Self {
            nodes,
            node_alloc: NodeAlloc::new(0..1, 1..cap),
        }
    }
}
/// 
impl ServerChunk {
    pub fn used_nodes(&self) -> &[Node] {
        &self.nodes[0..=self.node_alloc.last_used_addr() as usize]
    }

    pub fn set_voxel(&mut self, pos: VoxelPosInChunk, voxel: Voxel) -> Result<(), SetVoxelErr> {
        match self.node_alloc.peek() {
            None => {
                self.nodes.extend(&[Node::EMPTY; 128]);
                self.node_alloc.move_end(self.nodes.len() as u32);
            }
            Some(addr) => {
                if (self.nodes.len() as u32 - addr) < 128 {
                    self.nodes.extend(&[Node::EMPTY; 128]);
                    self.node_alloc.move_end(self.nodes.len() as u32);
                }
            }
        }
        Svo::new(0, CHUNK_SIZE).set_node(
            &mut self.nodes,
            *pos,
            voxel,
            CHUNK_DEPTH,
            &mut self.node_alloc,
        )
    }
}
