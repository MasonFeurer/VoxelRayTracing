use serde::Deserialize;

use super::{FeatureUid, Source};
use crate::world::Voxel;

#[derive(Deserialize, Debug)]
pub struct Biome {
    pub name: String,
    pub temp: (f32, f32),
    pub humidity: (f32, f32),
    pub weird: (f32, f32),
    pub surface: Vec<(Voxel, u32)>,
    pub surface_features: Vec<FeatureUid>,
}

#[derive(Deserialize, Debug)]
pub struct WorldPreset {
    pub name: String,
    pub temp: Source,
    pub humidity: Source,
    pub height: Source,
    pub sea_level: u32,
    pub earth: Voxel,
    pub biomes: Vec<Biome>,
}

#[derive(Deserialize, Debug)]
pub struct WorldFeature {
    pub name: String,
    pub freq: f32,
}
