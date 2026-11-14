use std::{env, fs, path::Path};

pub struct Environment {
    pub os: String,
    pub in_container: bool,
    pub immutable: bool,
}

impl Environment {
    pub fn detect() -> Self {
        let os = fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                s.lines().find_map(|l| {
                    l.strip_prefix("PRETTY_NAME=")
                        .map(|v| v.trim_matches('"').to_owned())
                })
            })
            .unwrap_or_else(|| env::consts::OS.into());
        Self {
            os,
            in_container: env::var_os("CONTAINER_ID").is_some()
                || env::var_os("container").is_some()
                || Path::new("/run/.containerenv").exists()
                || Path::new("/.dockerenv").exists(),
            immutable: Path::new("/run/ostree-booted").exists(),
        }
    }
    pub fn require_host(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            cfg!(target_os = "linux"),
            "Workspace management requires Linux"
        );
        anyhow::ensure!(
            !self.in_container,
            "Run Acheron on the host, outside your dev container. Use --demo for a preview here."
        );
        Ok(())
    }
}
