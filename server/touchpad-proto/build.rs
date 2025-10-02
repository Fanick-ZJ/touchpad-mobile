use std::{
    io::Result,
    path::{Path, PathBuf},
};

use cargo_metadata::{CargoOpt, MetadataCommand};

fn main() -> Result<()> {
    let meta = MetadataCommand::new()
        .manifest_path("Cargo.toml")
        .features(CargoOpt::AllFeatures)
        .exec()
        .expect("cargo metadata");

    let package = meta
        .packages
        .iter()
        .find(|pkg| pkg.name == "touchpad-proto")
        .expect("touchpad-proto package not found");

    let protos_dir = package
        .metadata
        .get("touchpad-proto")
        .expect("touchpad-proto metadata not found")
        .get("protos_dir")
        .expect("protos_dir not found")
        .as_str()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid protos_dir")
        })?;

    let base = Path::new(env!("CARGO_MANIFEST_DIR")); // 包根

    let protos_dir = if Path::new(protos_dir).is_relative() {
        base.join(protos_dir)
            .canonicalize()
            .expect("Failed to canonicalize protos_dir")
    } else {
        PathBuf::from(protos_dir)
    };

    if !protos_dir.exists() {
        eprintln!("Proto directory {} does not exist", protos_dir.display());
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Proto directory not found",
        ));
    }

    let protos_list = protos_dir
        .read_dir()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().map_or(false, |ext| ext == "proto") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    eprintln!("proto files:{:?}", protos_list);
    prost_build::compile_protos(&protos_list, &[protos_dir])?;
    Ok(())
}
