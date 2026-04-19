const FEATURES_RON: &str = include_str!("../../stdrespack/features.ron");
const META_RON: &str = include_str!("../../stdrespack/meta.ron");
const VOXELPACK_RON: &str = include_str!("../../stdrespack/voxelpack.ron");
const STYLEPACK_RON: &str = include_str!("../../stdrespack/voxelstylepack.ron");
const WORLDPRESETS_RON: &str = include_str!("../../stdrespack/worldpresets.ron");
// const WORLDMETA_RON: &str = include_str!("../../stdrespack/worldmeta.ron");

fn write(path: impl AsRef<std::path::Path>, contents: &str) -> Result<(), std::io::Error> {
    if !path.as_ref().exists() {
        std::fs::write(path, contents)?
    }
    Ok(())
}

fn main() {
    println!("Hello, world!");

    let path = dirs::config_dir().unwrap().join("blockworld");

    println!("Using assets directory at {:?}", path.display());
    println!("Setting up default assets...");
    std::fs::create_dir_all(&path).unwrap();
    std::fs::create_dir_all(path.join("worlds")).unwrap();
    std::fs::create_dir_all(path.join("datapacks/vanilla")).unwrap();
    std::fs::create_dir_all(path.join("stylepacks/vanilla")).unwrap();

    write(path.join("datapacks/vanilla/meta.ron"), META_RON).unwrap();
    write(path.join("datapacks/vanilla/voxels.ron"), VOXELPACK_RON).unwrap();
    write(path.join("datapacks/vanilla/world_features.ron"), FEATURES_RON).unwrap();
    write(path.join("datapacks/vanilla/world_gen.ron"), WORLDPRESETS_RON).unwrap();
    write(path.join("stylepacks/vanilla/meta.ron"), META_RON).unwrap();
    write(path.join("stylepacks/vanilla/voxel_styles.ron"), STYLEPACK_RON).unwrap();

    println!("Done setting up default assets");
    println!("Building server-cli.....");

    std::process::Command::new("cargo").arg("build").arg("--release").arg("--bin").arg("blockworld-server-cli").status().unwrap();
    println!("Done building server-cli");

    std::fs::copy("target/release/blockworld-server-cli", &path.join("blockworld-server-cli")).unwrap();

    println!("Done setting everything up!");
    println!("You can now run blockworld-client-desktop to play!");
}
