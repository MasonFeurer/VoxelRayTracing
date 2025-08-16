//// FOR DEV PURPOSES ONLY

use blockworld_common::resources::loader;

pub fn main() -> anyhow::Result<()> {
    let voxels = include_str!("../../stdrespack/voxelpack.ron");
    let features = include_str!("../../stdrespack/features.ron");
    let world_presets = include_str!("../../stdrespack/worldpresets.ron");
    let stylepack = include_str!("../../stdrespack/voxelstylepack.ron");

    let voxels = loader::parse_voxelpack(voxels)?;
    // println!("{voxels:#?}");

    let features = loader::parse_world_features(features, &voxels)?;
    // println!("{features:#?}");

    let world_presets = loader::parse_world_presets(&world_presets, &voxels, &features)?;
    // println!("{world_presets:#?}");

    let stylepack = loader::parse_voxel_stylepack(&stylepack, &voxels)?;
    // println!("{stylepack:#?}");

    Ok(())
}
