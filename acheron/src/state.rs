use crate::config::{Paths, Resolved};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub container_id: String,
    pub spec: Resolved,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub deployments: Vec<Receipt>,
}

pub fn read(paths: &Paths) -> Result<State> {
    let path = paths.state.join("deployments.json");
    if !path.exists() {
        return Ok(State::default());
    }
    serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("Invalid deployment state {}", path.display()))
}

pub fn write(paths: &Paths, state: &State) -> Result<()> {
    fs::create_dir_all(&paths.state)?;
    atomic_write(
        &paths.state.join("deployments.json"),
        &serde_json::to_vec_pretty(state)?,
    )
}

pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    ensure!(
        !path.is_symlink(),
        "Refusing to replace symlink {}",
        path.display()
    );
    let tmp = path.with_extension("tmp");
    ensure!(
        !tmp.is_symlink(),
        "Refusing temporary symlink {}",
        tmp.display()
    );
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn lock(paths: &Paths) -> Result<File> {
    fs::create_dir_all(&paths.state)?;
    let path = paths.state.join("acheron.lock");
    ensure!(!path.is_symlink(), "Refusing lock symlink");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    file.try_lock()
        .context("Another Acheron process may be managing these workspaces")?;
    Ok(file)
}
