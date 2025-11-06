use common::resources::world_preset::Biome;
use common::resources::{Source, WorldPreset};
pub use common::world::{noise::NoiseMap, *};
use glam::{uvec3, vec2, IVec3};
use std::collections::HashMap;

enum WorldValue {
    Constant(f32),
    Noise(NoiseMap, f32),
}
impl WorldValue {
    fn eval(&self, x: f32, z: f32) -> f32 {
        match self {
            Self::Constant(v) => *v,
            Self::Noise(n, offset) => n.get(vec2(x, z)) + offset,
        }
    }
}

pub struct ServerWorld {
    pub chunks: HashMap<IVec3, ServerChunk>,

    biomes: Vec<Biome>,
    biome_lookup: [[u32; 20]; 4],
    earth: Voxel,
    height_map: WorldValue,
    temp_map: WorldValue,
    humidity_map: WorldValue,
    weird_map: WorldValue,
}
impl ServerWorld {
    pub fn new(preset: &WorldPreset) -> Self {
        noise::init_gradients();

        let create_map = |src: Source| match src {
            Source::Value(v) => WorldValue::Constant(v),
            Source::Map {
                freq,
                scale,
                offset,
            } => WorldValue::Noise(
                NoiseMap::new(fastrand::i64(..), freq as f64, scale as f64),
                offset,
            ),
        };

        Self {
            chunks: HashMap::new(),

            biomes: preset.biomes.clone(),
            biome_lookup: preset.biome_lookup.clone(),
            earth: preset.earth,
            height_map: create_map(preset.height),
            temp_map: create_map(preset.temp),
            humidity_map: create_map(preset.humidity),
            weird_map: create_map(preset.weirdness),
        }
    }

    pub fn biome_at(&self, x: i32, z: i32) -> &Biome {
        // 0.0f..1.0f
        let temp = self.temp_map.eval(x as f32, z as f32);
        // 0.0f..1.0f
        let humidity = self.humidity_map.eval(x as f32, z as f32);

        let temp_idx = ((temp * 20.0).floor() as usize).min(19);
        let humidity_idx = ((humidity * 4.0).floor() as usize).min(3);
        let biome_idx = self.biome_lookup[humidity_idx][temp_idx];
        &self.biomes[biome_idx as usize]
    }

    pub fn get_chunk(&self, pos: IVec3) -> Option<&ServerChunk> {
        self.chunks.get(&pos)
    }

    pub fn create_chunk(&mut self, chunk_pos: IVec3) {
        let mut chunk = ServerChunk::with_capacity((CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) / 2);
        let alloc = &mut chunk.node_alloc;
        let mut svo = SvoMut {
            nodes: &mut chunk.nodes,
            root: 0,
            size: CHUNK_SIZE,
        };

        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let world_pos = inchunk_to_world_pos(chunk_pos, uvec3(x, 0, z));

                let biome = self.biome_at(world_pos.x, world_pos.z);
                let h = self.height_map.eval(world_pos.x as f32, world_pos.z as f32) as i32;

                let start_y = world_pos.y;
                let end_y = (world_pos.y + CHUNK_SIZE as i32).min(h + 1).max(start_y);
                for world_y in start_y..end_y {
                    let y_in_chunk = (world_y - world_pos.y) as u32;
                    let layer = h - world_y;

                    let vox = *biome.layers.get(layer as usize).unwrap_or(&self.earth);

                    _ = set_svo_voxel(&mut svo, uvec3(x, y_in_chunk, z), vox, CHUNK_DEPTH, alloc);
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
            #[allow(dead_code)] // used by  #derive Debug
            nodes: &'a [Node],
        }
        let c = Chunk {
            nodes: self.used_nodes(),
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
