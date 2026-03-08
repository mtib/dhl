pub mod complete;
pub mod db;
pub mod names;
pub mod repo;
pub mod workspace;

use anyhow::Result;
use std::path::PathBuf;

pub fn dhl_home() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let dhl = home.join(".dhl");
    std::fs::create_dir_all(&dhl)?;
    Ok(dhl)
}
