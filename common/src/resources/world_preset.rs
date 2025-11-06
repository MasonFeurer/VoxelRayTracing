use serde::Deserialize;

use super::{FeatureUid, Source};
use crate::world::Voxel;

#[derive(Deserialize, Debug, Clone)]
pub struct Biome {
    pub name: String,
    pub layers: Vec<Voxel>,
    pub surface_features: Vec<FeatureUid>,
}

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct WorldFeature {
    pub name: String,
    pub freq: f32,
}
