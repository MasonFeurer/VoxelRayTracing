use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::world::noise::Map;
use crate::world::Voxel;

pub mod loader;

#[derive(Deserialize, Debug)]
pub struct Meta {
    pub name: String,
    pub version: (u8, u8),
}

#[derive(Debug)]
pub struct Datapack {
    pub meta: Meta,
    pub voxels: VoxelPack,
    pub world_features: WorldFeatures,
    pub world_presets: Vec<WorldPreset>,
}
impl Datapack {
    pub fn load_from(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let meta = std::fs::read_to_string(dir.as_ref().join("meta.ron"))?;
        let meta = loader::parse_meta(&meta)?;

        let voxels = std::fs::read_to_string(dir.as_ref().join("voxels.ron"))?;
        let voxels = loader::parse_voxelpack(&voxels)?;

        let world_features = std::fs::read_to_string(dir.as_ref().join("world_features.ron"))?;
        let world_features = loader::parse_world_features(&world_features, &voxels)?;

        let world_presets = std::fs::read_to_string(dir.as_ref().join("world_gen.ron"))?;
        let world_presets = loader::parse_world_presets(&world_presets, &voxels, &world_features)?;

        Ok(Self {
            meta,
            voxels,
            world_features,
            world_presets,
        })
    }
}

#[derive(Debug)]
pub struct Stylepack {
    pub meta: Meta,
    pub voxel_styles: VoxelStylePack,
}
impl Stylepack {
    pub fn load_from(datapack: &Datapack, dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let meta = std::fs::read_to_string(dir.as_ref().join("meta.ron"))?;
        let meta = loader::parse_meta(&meta)?;

        let stylepack = std::fs::read_to_string(dir.as_ref().join("voxel_styles.ron"))?;
        let stylepack = loader::parse_voxel_stylepack(&stylepack, &datapack.voxels)?;
        Ok(Self {
            meta,
            voxel_styles: stylepack,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WorldPreset {
    pub name: String,
    pub temp: Source,
    pub humidity: Source,
    pub weirdness: Source,
    pub height: Source,
    pub sea_level: i32,
    pub earth: Voxel,
    pub water: Voxel,

    pub biome_lookup: [[u32; 20]; 4],
    pub biomes: Vec<Biome>,
}

#[derive(Clone, Debug)]
pub struct Biome {
    pub name: String,
    pub vegetation: Map,
    pub layers: Vec<Voxel>,
    pub features: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Feature {
    Tree {
        trunk_voxel: Voxel,
        branch_voxel: Voxel,
        leaf_voxel: Voxel,

        height: std::ops::Range<u32>,
        leaf_decay: f32,
        branch_count: std::ops::Range<u32>,
        branch_height: std::ops::Range<f32>,
        branch_len: std::ops::Range<u32>,
    },
    CanopyTree {
        trunk_voxel: Voxel,
        leaf_voxel: Voxel,
        height: std::ops::Range<u32>,
        slope_offset: std::ops::Range<u32>,
    },
    Evergreen {
        trunk_voxel: Voxel,
        leaf_voxel: Voxel,
        height: std::ops::Range<u32>,
        bottom_branch: std::ops::Range<u32>,
    },
    Cactus {
        voxel: Voxel,
        height: std::ops::Range<u32>,
    },
    Spike {
        voxel: Voxel,
        height: std::ops::Range<u32>,
        width: std::ops::Range<u32>,
    },
}

#[derive(Deserialize, Clone, Debug)]
pub enum Source {
    Value(f32),
    Noise(Map),
    ComplexNoise {
        freq: Map,
        scale: Map,
        base: Map,
        layers: Vec<Map>,
    },
}

#[derive(Debug, Clone)]
pub struct WorldFeatures(HashMap<String, Feature>);
impl WorldFeatures {
    pub fn get(&self, id: &str) -> Option<&Feature> {
        self.0.get(id)
    }
}

/// Lists all voxels that can exist in the world,
/// and gives them properties
#[derive(Debug)]
pub struct VoxelPack {
    voxels: Vec<VoxelData>,
}
impl VoxelPack {
    pub fn new(voxels: Vec<VoxelData>) -> Self {
        Self { voxels }
    }

    pub fn by_name(&self, name: &str) -> Option<Voxel> {
        assert!(self.voxels.len() < 256);
        self.voxels
            .iter()
            .enumerate()
            .find(|(_, d)| d.name.as_str() == name)
            .map(|(idx, _)| Voxel::from_data(idx as u16))
    }

    pub fn get(&self, v: Voxel) -> Option<&VoxelData> {
        self.voxels.get(v.as_data() as usize)
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum VoxelState {
    Solid,
    Liquid,
    Gas,
}

#[derive(Deserialize, Debug)]
pub struct VoxelData {
    pub state: VoxelState,
    pub name: String,
}
impl VoxelData {
    pub fn is_solid(&self) -> bool {
        self.state == VoxelState::Solid
    }
    pub fn is_air(&self) -> bool {
        self.state == VoxelState::Gas
    }
}

#[derive(Debug)]
pub struct VoxelStylePack {
    pub styles: Vec<VoxelStyle>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct VoxelStyle {
    pub state: VoxelState,
    pub color: [f32; 3],
}
impl VoxelStyle {
    pub const ZERO: Self = Self {
        state: VoxelState::Gas,
        color: [0.0; 3],
    };
}
