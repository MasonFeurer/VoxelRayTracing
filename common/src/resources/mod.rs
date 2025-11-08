use serde::Deserialize;

pub mod loader;
pub mod world_preset;

use crate::world::Voxel;
pub use world_preset::{WorldFeature, WorldPreset};

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct Noise {
    pub scale: f32,
    pub freq: f32,
    pub offset: f32,
}

#[derive(Deserialize, Clone, Debug)]
pub enum Source {
    Value(f32),
    Noise(Noise),
    ComplexNoise {
        freq: Noise,
        scale: Noise,
        base: Noise,
        layers: Vec<Noise>,
    },
}

pub type FeatureUid = usize;

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
