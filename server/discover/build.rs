use std::{env, path::PathBuf};

use anyhow::Result;

fn main() -> Result<()> {
    // Your code here
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let repo_root = manifest.parent().unwrap().parent().unwrap();
    let env_path = repo_root.join(".env");
    if !env_path.exists() {
        anyhow::bail!("The .env is not exist!")
    }
    dotenvy::from_path(env_path)?;
    Ok(())
}
