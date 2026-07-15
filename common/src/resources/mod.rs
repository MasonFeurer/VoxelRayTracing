use crate::world::noise::Map;
use crate::world::Voxel;
use anyhow::Context;
use log::warn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod loader;

pub type Version = (u8, u8);
pub const CURRENT_VERSION: Version = (0, 1);

#[derive(Debug)]
pub struct Resources {
    pub path: PathBuf,
    pub datapacks: HashMap<String, Datapack>,
    pub stylepacks: HashMap<String, Stylepack>,
    pub worlds: Vec<WorldInfo>,
}
impl<'a> Resources {
    pub fn load_from(data_folder: impl AsRef<Path>) -> anyhow::Result<Self> {
        let mut datapacks = HashMap::new();
        let mut stylepacks = HashMap::new();
        let mut worlds = Vec::new();

        for pack_folder in std::fs::read_dir(&data_folder.as_ref().join("datapacks"))? {
            let Ok(pack_folder) = pack_folder.map(|e| e.path()) else { continue };
            match Datapack::load_from(&pack_folder) {
                Ok(pack) => _ = datapacks.insert(pack.name.clone(), pack),
                Err(e) => {
                    warn!("Failed to load datapack {:?}: {}", pack_folder.display(), e);
                }
            }
        }
        for pack_folder in std::fs::read_dir(&data_folder.as_ref().join("stylepacks"))? {
            let Ok(pack_folder) = pack_folder.map(|e| e.path()) else { continue };
            match Stylepack::load_from(&pack_folder) {
                Ok(pack) => _ = stylepacks.insert(pack.name.clone(), pack),
                Err(e) => {
                    warn!("Failed to load stylepack {:?}: {}", pack_folder.display(), e);
                }
            }
        }
        for world_folder in std::fs::read_dir(&data_folder.as_ref().join("worlds"))? {
            let Ok(world_folder) = world_folder.map(|e| e.path()) else { continue };
            match WorldInfo::load_from(&world_folder) {
                Ok(world) => worlds.push(world),
                Err(e) => {
                    warn!("Failed to load world {:?}: {}", world_folder.display(), e);
                }
            }
        }

        Ok(Self {
            path: data_folder.as_ref().to_owned(),
            datapacks,
            stylepacks,
            worlds,
        })
    }
    
    pub fn reload_worlds(&mut self) -> anyhow::Result<()> {
        self.worlds.clear();
        for world_folder in std::fs::read_dir(&self.path.join("worlds"))? {
            let world_folder = world_folder?.path();
            let world = WorldInfo::load_from(world_folder)?;
            self.worlds.push(world);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub name: String,
    pub version: Version,
    pub datapack: String,
    pub stylepack: String,
}
impl WorldInfo {
    pub fn load_from(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let meta = std::fs::read_to_string(dir.as_ref().join("meta.ron"))?;
        let meta = loader::parse_world_meta(&meta)?;
        Ok(Self {
            name: meta.name,
            version: meta.version,
            datapack: meta.datapack,
            stylepack: meta.stylepack,
        })
    }
}

#[derive(Debug)]
pub struct Datapack {
    pub path: PathBuf,
    pub name: String,
    pub version: Version,
    pub voxels: VoxelPack,
    pub world_features: WorldFeatures,
    pub world_presets: Vec<WorldPreset>,
}
impl Datapack {
    pub fn load_from(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let meta = std::fs::read_to_string(dir.as_ref().join("meta.ron"))
            .context("Failed to read meta.ron")?;
        let meta = loader::parse_meta(&meta).context("Failed to parse meta.ron")?;

        let voxels = std::fs::read_to_string(dir.as_ref().join("voxels.ron"))
            .context("Failed to read voxels.ron")?;
        let voxels = loader::parse_voxelpack(&voxels).context("Failed to parse voxels.ron")?;

        let world_features = std::fs::read_to_string(dir.as_ref().join("world_features.ron"))
            .context("Failed to read world_features.ron")?;
        let world_features = loader::parse_world_features(&world_features, &voxels)
            .context("Failed to parse world_features.ron")?;

        let world_presets = std::fs::read_to_string(dir.as_ref().join("world_gen.ron"))
            .context("Failed to read world_gen.ron")?;
        let world_presets = loader::parse_world_presets(&world_presets, &voxels, &world_features)
            .context("Failed to parse world_gen.ron")?;

        Ok(Self {
            path: dir.as_ref().to_owned(),
            name: meta.name,
            version: meta.version,
            voxels,
            world_features,
            world_presets,
        })
    }
}

#[derive(Debug)]
pub struct Stylepack {
    pub name: String,
    pub version: Version,
    pub voxel_styles: HashMap<String, VoxelStyle>,
}
impl Stylepack {
    pub fn load_from(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let meta = std::fs::read_to_string(dir.as_ref().join("meta.ron"))?;
        let meta = loader::parse_meta(&meta)?;

        let stylepack = std::fs::read_to_string(dir.as_ref().join("voxel_styles.ron"))?;
        let stylepack = loader::parse_voxel_stylepack(&stylepack)?;
        Ok(Self {
            name: meta.name,
            version: meta.version,
            voxel_styles: stylepack.styles,
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

    pub biome_lookup: [[u32; 20]; 8],
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
    Lake {
        voxel: Voxel,
        size: std::ops::Range<u32>,
        depth: std::ops::Range<u32>,
    }
}

#[derive(Clone, Debug, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelPack {
    voxels: Vec<VoxelData>,
}
impl VoxelPack {
    pub fn new(voxels: Vec<VoxelData>) -> Self {
        assert!(voxels.len() < Voxel::MAX_VALUE as usize);
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
    
    pub fn voxel_idx_by_name(&self, name: &str) -> Option<usize> {
        self.voxels
            .iter()
            .enumerate()
            .find(|(_, d)| d.name.as_str() == name)
            .map(|(idx, _)| idx)
    }

    pub fn get(&self, v: Voxel) -> Option<&VoxelData> {
        self.voxels.get(v.as_data() as usize)
    }

    pub fn count(&self) -> usize {
        self.voxels.len()
    }

    pub fn get_all(&self) -> impl Iterator<Item = Voxel> {
        let total = self.voxels.len() as u16;
        (0..total).map(Voxel::from_data)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VoxelState {
    Solid,
    Liquid,
    Gas,
}
impl Default for VoxelState {
    fn default() -> Self {
        Self::Solid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoxelAttr {
    Color,
    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelData {
    pub name: String,
    #[serde(default)]
    pub state: VoxelState,
    #[serde(default)]
    pub attrs: Option<VoxelAttr>,
}
impl VoxelData {
    pub const fn is_solid(&self) -> bool {
        matches!(self.state, VoxelState::Solid)
    }
    pub const fn is_air(&self) -> bool {
        matches!(self.state, VoxelState::Gas)
    }
}

#[derive(Debug)]
pub struct VoxelStylepack {
    pub styles: HashMap<String, VoxelStyle>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
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
