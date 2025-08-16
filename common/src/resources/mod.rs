use serde::Deserialize;

pub mod loader;
pub mod world_preset;

use crate::Voxel;
use world_preset::{WorldFeature, WorldPreset};

#[derive(Deserialize, Clone, Debug)]
pub enum Source {
    Value(f32),
    Map { scale: f32, freq: f32 },
}

pub type FeatureUid = usize;

pub struct ClientResources {
    pub voxelpack: VoxelPack,
    pub voxel_stylepacks: Vec<VoxelStylePack>,
}

pub struct ServerResources {
    pub world_presets: Vec<WorldPreset>,
    pub world_features: WorldFeatures,
    pub voxelpack: VoxelPack,
}

#[derive(Debug)]
pub struct WorldFeatures(Vec<WorldFeature>);
impl WorldFeatures {
    pub fn by_name(&self, name: &str) -> Option<FeatureUid> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, d)| d.name.as_str() == name)
            .map(|(idx, _)| idx)
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
            .map(|(idx, _)| Voxel(idx as u8))
    }

    pub fn get(&self, v: Voxel) -> Option<&VoxelData> {
        self.voxels.get(v.0 as usize)
    }
}
#[derive(Deserialize, Debug)]
pub struct VoxelData {
    name: String,
    empty: bool,
}

#[derive(Debug)]
pub struct VoxelStylePack {
    pub styles: Vec<VoxelStyle>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
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
