pub mod gen;

use common::resources::{Biome, WorldFeatures, WorldPreset};
use common::world::{
    world_to_chunk_pos, world_to_inchunk_pos, Node, NodeAlloc, SetVoxelErr, Svo, Voxel,
    CHUNK_DEPTH, CHUNK_SIZE,
};
use gen::{BuiltFeature, WorldGen};
use glam::{ivec3, IVec3, UVec3};
use std::collections::{HashMap, HashSet};

pub struct ServerWorld {
    pub chunks: HashMap<IVec3, ServerChunk>,
    unplaced_features: Vec<BuiltFeature>,
    gen: WorldGen,
}
impl ServerWorld {
    pub fn new(preset: &WorldPreset, features: WorldFeatures, seed: i64) -> Self {
        Self {
            chunks: HashMap::new(),
            unplaced_features: Vec::new(),
            gen: WorldGen::new(preset, features, seed),
        }
    }

    pub fn create_chunk(&mut self, chunk_pos: IVec3) {
        let chunk = self
            .gen
            .generate_chunk(chunk_pos, &mut self.unplaced_features);
        self.chunks.insert(chunk_pos, chunk);
    }

    pub fn place_features(&mut self) -> Vec<IVec3> {
        let mut out = HashSet::new();

        'f: for feature_idx in (0..self.unplaced_features.len()).rev() {
            let feature = &self.unplaced_features[feature_idx];

            let min_chunk = world_to_chunk_pos(feature.min());
            let max_chunk = world_to_chunk_pos(feature.max());

            // make sure all chunks covered by the feature exist before placing.
            for x in min_chunk.x..=max_chunk.x {
                for y in min_chunk.y..=max_chunk.y {
                    for z in min_chunk.z..=max_chunk.z {
                        if self.get_chunk(ivec3(x, y, z)).is_none() {
                            continue 'f;
                        }
                    }
                }
            }

            for (pos, voxel) in feature.voxel_placements() {
                match set_voxel_w_chunks(&mut self.chunks, pos, voxel) {
                    Ok(()) => _ = out.insert(world_to_chunk_pos(pos)),
                    Err(err) => {
                        eprintln!(
                            "Failed to place voxel for feature {feature_idx} at {pos:?} : {err:?}"
                        );
                        continue 'f;
                    }
                }
            }
            _ = self.unplaced_features.remove(feature_idx);
        }
        out.into_iter().collect()
    }

    pub fn biome_at(&self, x: i32, z: i32) -> &Biome {
        self.gen.biome_at(x, z)
    }

    pub fn get_chunk(&self, pos: IVec3) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel: Voxel) -> Result<(), SetVoxelErr> {
        set_voxel_w_chunks(&mut self.chunks, pos, voxel)
    }
}

pub fn set_voxel_w_chunks(
    chunks: &mut HashMap<IVec3, ServerChunk>,
    pos: IVec3,
    voxel: Voxel,
) -> Result<(), SetVoxelErr> {
    let chunk_pos = world_to_chunk_pos(pos);
    let pos_in_chunk = world_to_inchunk_pos(pos);
    let chunk = chunks
        .get_mut(&chunk_pos)
        .ok_or(SetVoxelErr::PosOutOfBounds)?;
    chunk.set_voxel(pos_in_chunk, voxel)
}

pub struct ServerChunk {
    pub nodes: Vec<Node>,
    pub node_alloc: NodeAlloc,
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

    pub fn set_voxel(&mut self, pos: UVec3, voxel: Voxel) -> Result<(), SetVoxelErr> {
        match self.node_alloc.peek() {
            None => {
                self.nodes.extend(&[Node::ZERO; 128]);
                self.node_alloc.move_end(self.nodes.len() as u32);
            }
            Some(addr) => {
                if (self.nodes.len() as u32 - addr) < 128 {
                    self.nodes.extend(&[Node::ZERO; 128]);
                    self.node_alloc.move_end(self.nodes.len() as u32);
                }
            }
        }
        Svo::new(0, CHUNK_SIZE).set_node(
            &mut self.nodes,
            pos,
            voxel,
            CHUNK_DEPTH,
            &mut self.node_alloc,
        )
    }
}
