use serde::Deserialize;

use super::world_preset::{Biome, WorldPreset};
use super::{
    Source, VoxelData, VoxelPack, VoxelStyle, VoxelStylePack, WorldFeature, WorldFeatures,
};

#[derive(Debug)]
pub struct VoxelNotFoundError;
impl std::fmt::Display for VoxelNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for VoxelNotFoundError {}

#[derive(Debug)]
pub struct FeatureNotFoundError;
impl std::fmt::Display for FeatureNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for FeatureNotFoundError {}

#[derive(Deserialize)]
pub struct BiomeSource {
    name: String,
    temp: (f32, f32),
    humidity: (f32, f32),
    weird: (f32, f32),
    surface: Vec<(String, u32)>,
    surface_features: Vec<String>,
}
impl BiomeSource {
    pub fn construct(&self, voxels: &VoxelPack, features: &WorldFeatures) -> anyhow::Result<Biome> {
        let mut surface = Vec::with_capacity(self.surface.len());
        for idx in 0..self.surface.len() {
            surface.push((
                voxels
                    .by_name(&self.surface[idx].0)
                    .ok_or(VoxelNotFoundError)?,
                self.surface[idx].1,
            ))
        }
        let mut surface_features = Vec::with_capacity(self.surface_features.len());
        for idx in 0..self.surface_features.len() {
            surface_features.push(
                features
                    .by_name(&self.surface_features[idx])
                    .ok_or(FeatureNotFoundError)?,
            );
        }

        Ok(Biome {
            name: self.name.clone(),
            temp: self.temp,
            humidity: self.humidity,
            weird: self.weird,
            surface,
            surface_features,
        })
    }
}

#[derive(Deserialize)]
pub struct WorldPresetSource {
    name: String,

    temp: Source,
    humidity: Source,
    height: Source,

    sea_level: u32,
    earth: String,
    biomes: Vec<BiomeSource>,
}
impl WorldPresetSource {
    pub fn construct(
        &self,
        voxels: &VoxelPack,
        features: &WorldFeatures,
    ) -> anyhow::Result<WorldPreset> {
        let mut biomes = Vec::with_capacity(self.biomes.len());
        for idx in 0..self.biomes.len() {
            biomes.push(self.biomes[idx].construct(voxels, features)?);
        }
        Ok(WorldPreset {
            name: self.name.clone(),
            temp: self.temp.clone(),
            humidity: self.humidity.clone(),
            height: self.height.clone(),
            sea_level: self.sea_level,
            earth: voxels.by_name(&self.earth).ok_or(VoxelNotFoundError)?,
            biomes,
        })
    }
}

pub fn parse_world_presets(
    src: &str,
    voxels: &VoxelPack,
    features: &WorldFeatures,
) -> anyhow::Result<Vec<WorldPreset>> {
    let parsed: Vec<WorldPresetSource> = ron::de::from_str(src)?;
    let mut constructs = Vec::with_capacity(parsed.len());
    for idx in 0..parsed.len() {
        constructs.push(parsed[idx].construct(voxels, features)?);
    }
    Ok(constructs)
}
pub fn parse_world_features(src: &str, _voxels: &VoxelPack) -> anyhow::Result<WorldFeatures> {
    let parsed: Vec<WorldFeature> = ron::de::from_str(src)?;
    Ok(WorldFeatures(parsed))
}

pub fn parse_voxelpack(src: &str) -> anyhow::Result<VoxelPack> {
    let parsed: Vec<VoxelData> = ron::de::from_str(src)?;
    Ok(VoxelPack::new(parsed))
}

pub fn parse_voxel_stylepack(src: &str, voxels: &VoxelPack) -> anyhow::Result<VoxelStylePack> {
    let parsed: Vec<(String, VoxelStyle)> = ron::de::from_str(src)?;
    let mut styles = vec![VoxelStyle::ZERO; parsed.len()];

    for (vox_name, style) in parsed {
        let vox_id = voxels
            .by_name(&vox_name)
            .ok_or(VoxelNotFoundError)?
            .as_data();
        styles[vox_id as usize] = style;
    }

    Ok(VoxelStylePack { styles })
}
