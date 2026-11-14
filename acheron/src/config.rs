use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub workspace: Vec<Workspace>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub name: String,
    pub image: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub mounts: Vec<Mount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Mount {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Resolved {
    pub name: String,
    pub image: String,
    pub theme: String,
    pub home: PathBuf,
    pub repo: PathBuf,
    pub mounts: Vec<Mount>,
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub repo: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub host_home: PathBuf,
}

fn default_theme() -> String {
    "vulcan".into()
}
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.as_bytes()[0].is_ascii_alphanumeric()
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

// Distrobox interpolates volume/flag arguments into its own shell commands.
// Reject metacharacters even though Acheron invokes it without a shell.
pub fn safe_path(path: &Path) -> Result<()> {
    let text = path.to_str().context("Paths must be valid UTF-8")?;
    ensure!(
        path.is_absolute() && !path.components().any(|c| c == Component::ParentDir),
        "Use absolute paths without '..': {text}"
    );
    ensure!(
        text.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/._-".contains(&c)),
        "Paths may contain only letters, digits, /, ., _, and -: {text}"
    );
    Ok(())
}

pub fn expand(value: &str, home: &Path) -> Result<PathBuf> {
    let path = if value == "~" {
        home.to_path_buf()
    } else if let Some(rest) = value.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(value)
    };
    safe_path(&path)?;
    Ok(path)
}

impl Paths {
    pub fn discover(repo: Option<PathBuf>) -> Result<Self> {
        let home = PathBuf::from(env::var_os("HOME").context("HOME is required")?);
        let repo = repo
            .or_else(|| env::var_os("ACHERON_REPO").map(PathBuf::from))
            .unwrap_or_else(|| home.join(".dotfiles"));
        let repo = repo.canonicalize().with_context(|| {
            format!("Repository not found: {}. Pass --repo PATH", repo.display())
        })?;
        let data = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("acheron");
        let state = env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"))
            .join("acheron");
        for path in [&repo, &data, &state, &home] {
            safe_path(path)?;
        }
        ensure!(
            repo.join("stow/terminal").is_dir(),
            "{} is not an Acheron dotfiles repository",
            repo.display()
        );
        Ok(Self {
            repo,
            data,
            state,
            host_home: home,
        })
    }
    pub fn manifest(&self, profile: &str) -> Result<PathBuf> {
        ensure!(valid_name(profile), "Invalid profile name");
        Ok(if profile == "default" {
            self.repo.join("workspaces/default.toml")
        } else {
            self.repo
                .join("workspaces/machines")
                .join(format!("{profile}.toml"))
        })
    }
}

impl Workspace {
    pub fn resolve(&self, paths: &Paths) -> Result<Resolved> {
        ensure!(
            valid_name(&self.name),
            "Invalid workspace name: {}",
            self.name
        );
        ensure!(
            !self.image.is_empty()
                && !self.image.starts_with('-')
                && self
                    .image
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"/.:_-@".contains(&c)),
            "Invalid image reference"
        );
        ensure!(
            ["mars", "vulcan"].contains(&self.theme.as_str()),
            "Theme must be mars or vulcan"
        );
        let home = paths.data.join("homes").join(&self.name);
        let mut targets = BTreeSet::new();
        let mounts = self
            .mounts
            .iter()
            .map(|mount| {
                let source = expand(&mount.source, &paths.host_home)?;
                let target = PathBuf::from(&mount.target);
                safe_path(&target)?;
                ensure!(
                    target.starts_with("/workspace") && target != Path::new("/workspace"),
                    "Project mount targets must be below /workspace"
                );
                ensure!(
                    targets.insert(target.clone()),
                    "Duplicate mount target: {}",
                    target.display()
                );
                Ok(Mount {
                    source: source.to_string_lossy().into(),
                    target: mount.target.clone(),
                    read_only: mount.read_only,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Resolved {
            name: self.name.clone(),
            image: self.image.clone(),
            theme: self.theme.clone(),
            home,
            repo: paths.repo.clone(),
            mounts,
        })
    }
}

pub fn load(paths: &Paths, profile: &str) -> Result<Vec<Resolved>> {
    fn read(path: &Path) -> Result<Manifest> {
        let text =
            fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
        let manifest: Manifest = toml::from_str(&text)
            .with_context(|| format!("Invalid manifest {}", path.display()))?;
        ensure!(
            manifest.version == 1,
            "Unsupported manifest version: {}",
            manifest.version
        );
        let mut names = BTreeSet::new();
        for w in &manifest.workspace {
            ensure!(names.insert(&w.name), "Duplicate workspace: {}", w.name);
        }
        Ok(manifest)
    }
    let mut base = read(&paths.manifest("default")?)?;
    if profile != "default" {
        for workspace in read(&paths.manifest(profile)?)?.workspace {
            if let Some(existing) = base.workspace.iter_mut().find(|w| w.name == workspace.name) {
                *existing = workspace;
            } else {
                base.workspace.push(workspace);
            }
        }
    }
    base.workspace.iter().map(|w| w.resolve(paths)).collect()
}

pub fn add_workspace(paths: &Paths, profile: &str, workspace: Workspace) -> Result<()> {
    workspace.resolve(paths)?;
    ensure!(
        !load(paths, profile)?
            .iter()
            .any(|w| w.name == workspace.name),
        "Workspace already exists in the selected profile"
    );
    let path = paths.manifest(profile)?;
    let mut manifest: Manifest = if path.exists() {
        toml::from_str(&fs::read_to_string(&path)?)?
    } else {
        Manifest {
            version: 1,
            workspace: vec![],
        }
    };
    manifest.workspace.push(workspace);
    fs::create_dir_all(path.parent().unwrap())?;
    crate::state::atomic_write(&path, toml::to_string_pretty(&manifest)?.as_bytes())
}
