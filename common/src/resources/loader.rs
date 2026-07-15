use super::{Biome, Feature, Source, Version, VoxelData, VoxelPack, VoxelStyle, VoxelStylepack, WorldFeatures, WorldPreset};
use crate::world::noise::Map;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub enum LoaderErr {
    Ron(ron::error::SpannedError),
    FeatureNotFound(String),
    VoxelNotFound(String),
    DuplicateVoxel(String),
}
impl std::fmt::Display for LoaderErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for LoaderErr {}
trait LoaderErrImpl: Sized {
    type Return;
    fn anyhow(self) -> Self::Return;
    fn context(self, msg: impl Into<String>) -> Self::Return;
}
impl LoaderErrImpl for LoaderErr {
    type Return = anyhow::Error;
    fn anyhow(self) -> anyhow::Error {
        anyhow::anyhow!("{}", self)
    }
    fn context(self, msg: impl Into<String>) -> anyhow::Error {
        self.anyhow().context(msg.into())
    }
}
impl<T> LoaderErrImpl for Result<T, LoaderErr> {
    type Return = Result<T, anyhow::Error>;
    fn anyhow(self) -> Result<T, anyhow::Error> {
        self.map_err(LoaderErr::anyhow)
    }
    fn context(self, msg: impl Into<String>) -> Result<T, anyhow::Error> {
        self.map_err(|e| e.context(msg))
    }
}

#[derive(Deserialize, Debug)]
pub struct RawMeta {
    pub name: String,
    pub version: Version,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RawWorldMeta {
    pub name: String,
    pub version: Version,
    pub datapack: String,
    pub stylepack: String,
    pub seed: i64,
}

#[derive(Deserialize)]
struct RawLayer {
    voxel: String,
    depth: u32,
}

#[derive(Deserialize)]
enum RawFeature {
    Tree {
        trunk_voxel: String,
        branch_voxel: String,
        leaf_voxel: String,

        height: (u32, u32),
        leaf_decay: f32,
        branch_count: (u32, u32),
        branch_height: (f32, f32),
        branch_len: (u32, u32),
    },
    CanopyTree {
        trunk_voxel: String,
        leaf_voxel: String,
        height: (u32, u32),
        slope_offset: (u32, u32),
    },
    Evergreen {
        trunk_voxel: String,
        leaf_voxel: String,
        height: (u32, u32),
        bottom_branch: (u32, u32),
    },
    Cactus {
        voxel: String,
        height: (u32, u32),
    },
    Spike {
        voxel: String,
        height: (u32, u32),
        width: (u32, u32),
    },
    Lake {
        voxel: String,
        size: (u32, u32),
        depth: (u32, u32),
    }
}
impl RawFeature {
    pub fn construct(&self, voxels: &VoxelPack) -> Result<Feature, LoaderErr> {
        Ok(match self {
            Self::Tree {
                trunk_voxel,
                branch_voxel,
                leaf_voxel,
                height,
                leaf_decay,
                branch_count,
                branch_height,
                branch_len,
            } => Feature::Tree {
                trunk_voxel: voxels
                    .by_name(trunk_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(trunk_voxel.clone()))?,
                branch_voxel: voxels
                    .by_name(branch_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(branch_voxel.clone()))?,
                leaf_voxel: voxels
                    .by_name(leaf_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(leaf_voxel.clone()))?,
                height: height.0..height.1,
                leaf_decay: *leaf_decay,
                branch_count: branch_count.0..branch_count.1,
                branch_height: branch_height.0..branch_height.1,
                branch_len: branch_len.0..branch_len.1,
            },
            Self::CanopyTree {
                trunk_voxel,
                leaf_voxel,
                height,
                slope_offset,
            } => Feature::CanopyTree {
                trunk_voxel: voxels
                    .by_name(trunk_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(trunk_voxel.clone()))?,
                leaf_voxel: voxels
                    .by_name(leaf_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(leaf_voxel.clone()))?,
                height: height.0..height.1,
                slope_offset: slope_offset.0..slope_offset.1,
            },
            Self::Evergreen {
                trunk_voxel,
                leaf_voxel,
                height,
                bottom_branch,
            } => Feature::Evergreen {
                trunk_voxel: voxels
                    .by_name(trunk_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(trunk_voxel.clone()))?,
                leaf_voxel: voxels
                    .by_name(leaf_voxel)
                    .ok_or(LoaderErr::VoxelNotFound(leaf_voxel.clone()))?,
                height: height.0..height.1,
                bottom_branch: bottom_branch.0..bottom_branch.1,
            },
            Self::Cactus { voxel, height } => Feature::Cactus {
                voxel: voxels.by_name(voxel).ok_or(LoaderErr::VoxelNotFound(voxel.clone()))?,
                height: height.0..height.1,
            },
            Self::Spike {
                voxel,
                height,
                width,
            } => Feature::Spike {
                voxel: voxels.by_name(voxel).ok_or(LoaderErr::VoxelNotFound(voxel.clone()))?,
                height: height.0..height.1,
                width: width.0..width.1,
            },
            Self::Lake {
                voxel,
                size,
                depth,
            } => Feature::Lake {
                voxel: voxels.by_name(voxel).ok_or(LoaderErr::VoxelNotFound(voxel.clone()))?,
                size: size.0..size.1,
                depth: depth.0..depth.1,
            },
        })
    }
}

#[derive(Deserialize)]
pub struct RawBiome {
    name: String,
    vegetation: Map,
    layers: Vec<RawLayer>,
    features: Vec<String>,
}
impl RawBiome {
    pub fn construct(
        &self,
        voxels: &VoxelPack,
        features: &WorldFeatures,
    ) -> Result<Biome, LoaderErr> {
        let mut layers = Vec::with_capacity(self.layers.len());
        for idx in 0..self.layers.len() {
            let name = &self.layers[idx].voxel;
            layers.extend(&vec![
                voxels
                    .by_name(name)
                    .ok_or(LoaderErr::VoxelNotFound(name.clone()))?;
                self.layers[idx].depth as usize
            ])
        }
        for feature in &self.features {
            _ = features
                .get(feature)
                .ok_or(LoaderErr::FeatureNotFound(feature.clone()))?;
        }

        Ok(Biome {
            name: self.name.clone(),
            vegetation: self.vegetation.clone(),
            layers,
            features: self.features.clone(),
        })
    }
}

#[derive(Deserialize)]
struct RawWorldPreset {
    name: String,

    temp: Source,
    humidity: Source,
    height: Source,
    weirdness: Source,

    sea_level: i32,
    earth: String,
    water: String,
    biome_lookup: [[u32; 20]; 8],
    biomes: Vec<RawBiome>,
}
impl RawWorldPreset {
    fn construct(
        &self,
        voxels: &VoxelPack,
        features: &WorldFeatures,
    ) -> anyhow::Result<WorldPreset> {
        let mut biomes = Vec::with_capacity(self.biomes.len());
        for biome in &self.biomes {
            biomes.push(
                biome
                    .construct(voxels, features)
                    .context(&format!("Failed to construct biome {:?}", biome.name))?
            );
        }
        let earth = voxels.by_name(&self.earth).ok_or(
            LoaderErr::VoxelNotFound(self.earth.clone())
                .context("Failed to load 'earth' field")
        )?;
        Ok(WorldPreset {
            name: self.name.clone(),
            temp: self.temp.clone(),
            humidity: self.humidity.clone(),
            weirdness: self.weirdness.clone(),
            height: self.height.clone(),
            sea_level: self.sea_level,
            earth,
            water: voxels.by_name(&self.water).ok_or(
                LoaderErr::VoxelNotFound(self.water.clone())
                    .context("Failed to load 'water' field")
            )?,
            biome_lookup: self.biome_lookup.clone(),
            biomes,
        })
    }
}

pub fn parse_world_presets(
    src: &str,
    voxels: &VoxelPack,
    features: &WorldFeatures,
) -> anyhow::Result<Vec<WorldPreset>> {
    use anyhow::Context;

    let parsed: Vec<RawWorldPreset> = anyhow::Context::context(
        ron::de::from_str(src).map_err(LoaderErr::Ron),
        "Failed to parse RON",
    )?;
    let mut constructs = Vec::with_capacity(parsed.len());
    for idx in 0..parsed.len() {
        constructs.push(
            parsed[idx]
                .construct(voxels, features)
                .context(format!("Failed to construct WorldGen {}", parsed[idx].name))?,
        );
    }
    Ok(constructs)
}
pub fn parse_world_features<'a>(src: &str, voxels: &VoxelPack) -> anyhow::Result<WorldFeatures> {
    let parsed: HashMap<String, RawFeature> = ron::de::from_str(src)
        .map_err(LoaderErr::Ron)
        .context("failed to parse RON")?;
    let mut compiled = HashMap::with_capacity(parsed.len());
    for (id, feature) in parsed {
        compiled.insert(
            id.clone(),
            feature
                .construct(voxels)
                .context(&format!("Failed to construct feature {id:?}"))?
        );
    }
    Ok(WorldFeatures(compiled))
}

pub fn parse_meta(src: &str) -> Result<RawMeta, LoaderErr> {
    let parsed: RawMeta = ron::de::from_str(src)
        .map_err(LoaderErr::Ron)?;
    Ok(parsed)
}
pub fn parse_world_meta(src: &str) -> Result<RawWorldMeta, LoaderErr> {
    let parsed: RawWorldMeta = ron::de::from_str(src)
        .map_err(LoaderErr::Ron)?;
    Ok(parsed)
}

pub fn parse_voxelpack(src: &str) -> Result<VoxelPack, LoaderErr> {
    let parsed: Vec<VoxelData> = ron::de::from_str(src)
        .map_err(LoaderErr::Ron)?;
    for (idx, vox) in parsed.iter().enumerate() {
        if parsed.iter().enumerate().any(|(i, v)| i != idx && v.name == vox.name) {
            return Err(LoaderErr::DuplicateVoxel(vox.name.clone()));
        }
    }
    Ok(VoxelPack::new(parsed))
}

pub fn parse_voxel_stylepack(src: &str) -> Result<VoxelStylepack, LoaderErr> {
    let parsed: Vec<(String, VoxelStyle)> = ron::de::from_str(src)
        .map_err(LoaderErr::Ron)?;
    let mut styles = HashMap::with_capacity(parsed.len());

    for (vox_name, style) in parsed {
        if styles.contains_key(&vox_name) {
            return Err(LoaderErr::DuplicateVoxel(vox_name.clone()));
        }
        styles.insert(vox_name, style);
    }
    Ok(VoxelStylepack { styles })
}
