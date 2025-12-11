use super::{
    Biome, Feature, Source, VoxelData, VoxelPack, VoxelStyle, VoxelStylePack, WorldFeatures,
    WorldPreset,
};
use crate::world::noise::Map;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug)]
pub enum ErrType {
    Ron(ron::error::SpannedError),
    FeatureNotFound(String),
    VoxelNotFound(String),
}

#[derive(Debug)]
pub struct LoaderErr {
    ty: ErrType,
    context: Option<String>,
}
impl LoaderErr {
    fn new(ty: ErrType) -> Self {
        Self { ty, context: None }
    }

    fn ron(err: ron::error::SpannedError) -> Self {
        Self::new(ErrType::Ron(err))
    }
    fn voxel_nf(name: &str) -> Self {
        Self::new(ErrType::VoxelNotFound(String::from(name)))
    }
    fn feature_nf(name: &str) -> Self {
        Self::new(ErrType::FeatureNotFound(String::from(name)))
    }

    pub fn context(mut self, c: &str) -> Self {
        self.context = Some(String::from(c));
        self
    }
    pub fn ty(&self) -> &ErrType {
        &self.ty
    }
}
impl std::fmt::Display for LoaderErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for LoaderErr {}

#[derive(Deserialize)]
pub struct LayerSource {
    voxel: String,
    depth: u32,
}

#[derive(Deserialize)]
pub enum FeatureSource {
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
}
impl FeatureSource {
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
                    .ok_or(LoaderErr::voxel_nf(&trunk_voxel))?,
                branch_voxel: voxels
                    .by_name(branch_voxel)
                    .ok_or(LoaderErr::voxel_nf(&branch_voxel))?,
                leaf_voxel: voxels
                    .by_name(leaf_voxel)
                    .ok_or(LoaderErr::voxel_nf(&leaf_voxel))?,
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
                    .ok_or(LoaderErr::voxel_nf(&trunk_voxel))?,
                leaf_voxel: voxels
                    .by_name(leaf_voxel)
                    .ok_or(LoaderErr::voxel_nf(&leaf_voxel))?,
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
                    .ok_or(LoaderErr::voxel_nf(&trunk_voxel))?,
                leaf_voxel: voxels
                    .by_name(leaf_voxel)
                    .ok_or(LoaderErr::voxel_nf(&leaf_voxel))?,
                height: height.0..height.1,
                bottom_branch: bottom_branch.0..bottom_branch.1,
            },
            Self::Cactus { voxel, height } => Feature::Cactus {
                voxel: voxels.by_name(voxel).ok_or(LoaderErr::voxel_nf(&voxel))?,
                height: height.0..height.1,
            },
            Self::Spike {
                voxel,
                height,
                width,
            } => Feature::Spike {
                voxel: voxels.by_name(voxel).ok_or(LoaderErr::voxel_nf(&voxel))?,
                height: height.0..height.1,
                width: width.0..width.1,
            },
        })
    }
}

#[derive(Deserialize)]
pub struct BiomeSource {
    name: String,
    vegetation: Map,
    layers: Vec<LayerSource>,
    features: Vec<String>,
}
impl BiomeSource {
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
                    .ok_or(LoaderErr::voxel_nf(&name))?;
                self.layers[idx].depth as usize
            ])
        }
        for feature in &self.features {
            _ = features
                .get(feature)
                .ok_or(LoaderErr::feature_nf(&feature))?;
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
pub struct WorldPresetSource {
    name: String,

    temp: Source,
    humidity: Source,
    height: Source,
    weirdness: Source,

    sea_level: i32,
    earth: String,
    water: String,
    biome_lookup: [[u32; 20]; 4],
    biomes: Vec<BiomeSource>,
}
impl WorldPresetSource {
    pub fn construct(
        &self,
        voxels: &VoxelPack,
        features: &WorldFeatures,
    ) -> Result<WorldPreset, LoaderErr> {
        let ctx = format!("Constructing WorldPreset {:?}", self.name);
        let mut biomes = Vec::with_capacity(self.biomes.len());
        for biome in &self.biomes {
            biomes.push(
                biome
                    .construct(voxels, features)
                    .map_err(|e| e.context(&format!("Constructing biome {:?}", biome.name)))
                    .map_err(|e| e.context(&ctx))?,
            );
        }
        Ok(WorldPreset {
            name: self.name.clone(),
            temp: self.temp.clone(),
            humidity: self.humidity.clone(),
            weirdness: self.weirdness.clone(),
            height: self.height.clone(),
            sea_level: self.sea_level,
            earth: voxels.by_name(&self.earth).ok_or(
                LoaderErr::voxel_nf(&self.earth)
                    .context("`earth` field")
                    .context(&ctx),
            )?,
            water: voxels.by_name(&self.water).ok_or(
                LoaderErr::voxel_nf(&self.water)
                    .context("`water` field")
                    .context(&ctx),
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
) -> Result<Vec<WorldPreset>, LoaderErr> {
    let parsed: Vec<WorldPresetSource> = ron::de::from_str(src)
        .map_err(LoaderErr::ron)
        .map_err(|e| e.context("world_presets file"))?;
    let mut constructs = Vec::with_capacity(parsed.len());
    for idx in 0..parsed.len() {
        constructs.push(parsed[idx].construct(voxels, features)?);
    }
    Ok(constructs)
}
pub fn parse_world_features(src: &str, voxels: &VoxelPack) -> Result<WorldFeatures, LoaderErr> {
    let parsed: HashMap<String, FeatureSource> = ron::de::from_str(src)
        .map_err(LoaderErr::ron)
        .map_err(|e| e.context("world_features file"))?;
    let mut compiled = HashMap::with_capacity(parsed.len());
    for (id, feature) in parsed {
        compiled.insert(
            id,
            feature
                .construct(voxels)
                .map_err(|e| e.context("world_features file"))?,
        );
    }
    Ok(WorldFeatures(compiled))
}

pub fn parse_voxelpack(src: &str) -> Result<VoxelPack, LoaderErr> {
    let parsed: Vec<VoxelData> = ron::de::from_str(src)
        .map_err(LoaderErr::ron)
        .map_err(|e| e.context("voxelpack file"))?;
    Ok(VoxelPack::new(parsed))
}

pub fn parse_voxel_stylepack(src: &str, voxels: &VoxelPack) -> Result<VoxelStylePack, LoaderErr> {
    let parsed: Vec<(String, VoxelStyle)> = ron::de::from_str(src)
        .map_err(LoaderErr::ron)
        .map_err(|e| e.context("stylepack file"))?;
    let mut styles = vec![VoxelStyle::ZERO; parsed.len()];

    for (vox_name, style) in parsed {
        let vox_id = voxels
            .by_name(&vox_name)
            .ok_or(LoaderErr::voxel_nf(&vox_name))?
            .as_data();
        styles[vox_id as usize] = style;
    }

    Ok(VoxelStylePack { styles })
}
