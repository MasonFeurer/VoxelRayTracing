use serde::Deserialize;
use std::collections::HashMap;

pub mod loader;

use crate::world::noise::Map;
use crate::world::Voxel;

#[derive(Clone, Debug)]
pub struct WorldPreset {
    pub name: String,
    pub temp: Source,
    pub humidity: Source,
    pub weirdness: Source,
    pub height: Source,
    pub sea_level: u32,
    pub earth: Voxel,

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

#[derive(Deserialize, Debug)]
pub struct VoxelData {
    pub name: String,
    pub empty: bool,
}

#[derive(Debug)]
pub struct VoxelStylePack {
    pub styles: Vec<VoxelStyle>,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct VoxelStyle {
    pub empty: bool,
    pub color: [f32; 3],
}
impl VoxelStyle {
    pub const ZERO: Self = Self {
        empty: false,
        color: [0.0; 3],
    };
}
