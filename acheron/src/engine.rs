use crate::{
    backend::{Backend, Container},
    config::{Paths, Resolved},
    state::{self, Receipt, State},
};
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Create,
    Sync,
    Rebuild,
    Conflict,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(match self {
            Self::Create => "create",
            Self::Sync => "sync config",
            Self::Rebuild => "rebuild required",
            Self::Conflict => "name conflict",
        })
    }
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub spec: Resolved,
    pub action: Action,
}

fn same_container_layout(a: &Resolved, b: &Resolved) -> bool {
    a.image == b.image && a.home == b.home && a.repo == b.repo && a.mounts == b.mounts
}

pub fn plan(desired: &[Resolved], containers: &[Container], state: &State) -> Vec<Plan> {
    desired
        .iter()
        .map(|spec| {
            let action = match containers.iter().find(|c| c.name == spec.name) {
                None => Action::Create,
                Some(c) => match state
                    .deployments
                    .iter()
                    .find(|r| r.spec.name == spec.name && r.container_id == c.id)
                {
                    None => Action::Conflict,
                    Some(receipt) if same_container_layout(&receipt.spec, spec) => Action::Sync,
                    Some(_) => Action::Rebuild,
                },
            };
            Plan {
                spec: spec.clone(),
                action,
            }
        })
        .collect()
}

pub fn check_mounts(spec: &Resolved) -> Result<()> {
    let homes = spec.home.parent().context("Invalid managed home")?;
    ensure!(
        !spec.repo.starts_with(homes),
        "Keep the dotfiles repository outside managed workspace homes"
    );
    for mount in &spec.mounts {
        ensure!(
            Path::new(&mount.source).exists(),
            "Mount source does not exist: {}. Create it or edit the workspace definition.",
            mount.source
        );
        let source = fs::canonicalize(&mount.source)?;
        let managed = if homes.exists() {
            fs::canonicalize(homes)?
        } else {
            homes.to_path_buf()
        };
        ensure!(
            !source.starts_with(&managed),
            "Project sources must be outside managed homes so factory reset can preserve them"
        );
    }
    Ok(())
}

fn check_home_path(paths: &Paths, spec: &Resolved) -> Result<()> {
    ensure!(
        spec.home == paths.data.join("homes").join(&spec.name),
        "Home is outside Acheron's managed homes"
    );
    for path in [&paths.data, &paths.data.join("homes"), &spec.home] {
        ensure!(
            !path.is_symlink(),
            "Managed home path must not be a symlink: {}",
            path.display()
        );
    }
    Ok(())
}

fn owned_home(paths: &Paths, spec: &Resolved) -> Result<()> {
    check_home_path(paths, spec)?;
    let marker = spec.home.join(".acheron-owner");
    ensure!(
        !marker.is_symlink(),
        "Home ownership marker must not be a symlink"
    );
    ensure!(
        fs::read_to_string(&marker).unwrap_or_default() == spec.name,
        "Refusing to modify an unowned home: {}",
        spec.home.display()
    );
    Ok(())
}

fn prepare_home(paths: &Paths, spec: &Resolved) -> Result<()> {
    check_home_path(paths, spec)?;
    if spec.home.join(".acheron-owner").exists() {
        return owned_home(paths, spec);
    }
    if spec.home.exists() {
        ensure!(
            fs::read_dir(&spec.home)?.next().is_none(),
            "Home already contains files but has no Acheron ownership marker: {}",
            spec.home.display()
        );
    }
    fs::create_dir_all(&spec.home)?;
    state::atomic_write(&spec.home.join(".acheron-owner"), spec.name.as_bytes())?;
    Ok(())
}

fn remember(
    paths: &Paths,
    state: &mut State,
    backend: &mut impl Backend,
    spec: &Resolved,
) -> Result<()> {
    let c = backend
        .list()?
        .into_iter()
        .find(|c| c.name == spec.name)
        .context(
            "Created container was not returned by distrobox list; inspect it before retrying",
        )?;
    state.deployments.retain(|r| r.spec.name != spec.name);
    state.deployments.push(Receipt {
        container_id: c.id,
        spec: spec.clone(),
    });
    state::write(paths, state)
}

pub fn apply(paths: &Paths, desired: &[Resolved], backend: &mut impl Backend) -> Result<String> {
    let mut state = state::read(paths)?;
    let steps = plan(desired, &backend.list()?, &state);
    ensure!(
        !steps.iter().any(|s| s.action == Action::Conflict),
        "A requested name belongs to an unmanaged or replaced container. No changes made; inspect the plan."
    );
    for step in &steps {
        if step.action != Action::Rebuild {
            check_mounts(&step.spec)?;
        }
    }
    let mut log = String::new();
    for step in steps {
        match step.action {
            Action::Create => {
                prepare_home(paths, &step.spec)?;
                backend.configure(&step.spec)?;
                backend.create(&step.spec)?;
                remember(paths, &mut state, backend, &step.spec)?;
                log.push_str(&format!(
                    "Created {}. First entry initializes Distrobox.\n",
                    step.spec.name
                ));
            }
            Action::Sync => {
                owned_home(paths, &step.spec)?;
                backend.configure(&step.spec)?;
                remember(paths, &mut state, backend, &step.spec)?;
                log.push_str(&format!(
                    "Synced {} configuration. Open a new shell for theme changes.\n",
                    step.spec.name
                ));
            }
            Action::Rebuild => log.push_str(&format!(
                "{} needs an explicit rebuild (image, home, repository, or mounts changed).\n",
                step.spec.name
            )),
            Action::Conflict => unreachable!(),
        }
    }
    Ok(log)
}

pub fn rebuild(
    paths: &Paths,
    spec: &Resolved,
    factory: bool,
    backend: &mut impl Backend,
) -> Result<String> {
    check_mounts(spec)?;
    let mut state = state::read(paths)?;
    let receipt = state
        .deployments
        .iter()
        .find(|r| r.spec.name == spec.name)
        .context("Only Acheron-managed containers can be rebuilt")?;
    ensure!(
        receipt.spec.home == spec.home,
        "Home location changed. Restore the original data directory before rebuilding; automatic migration is not supported."
    );
    owned_home(paths, &receipt.spec)?;
    let existing = backend.list()?;
    if let Some(c) = existing.iter().find(|c| c.name == spec.name) {
        ensure!(
            c.id == receipt.container_id,
            "Container identity changed outside Acheron; refusing to rebuild it"
        );
    }
    // Refuse Stow conflicts before removing a working container. Factory reset
    // deliberately clears this home, so it configures the replacement instead.
    if !factory {
        backend.configure(spec)?;
    }
    if existing.iter().any(|c| c.name == spec.name) {
        backend.remove(&spec.name)?;
    }
    if factory {
        // Never pass --rm-home to Distrobox. Only remove a verified owned directory.
        owned_home(paths, spec)?;
        fs::remove_dir_all(&spec.home)?;
    }
    prepare_home(paths, spec)?;
    if factory {
        backend.configure(spec)?;
    }
    backend.create(spec)?;
    remember(paths, &mut state, backend, spec)?;
    Ok(format!(
        "Rebuilt {}. {}",
        spec.name,
        if factory {
            "Workspace home was cleared. Project mounts were preserved."
        } else {
            "Workspace home and projects were preserved."
        }
    ))
}

pub fn delete(paths: &Paths, name: &str, backend: &mut impl Backend) -> Result<String> {
    ensure!(crate::config::valid_name(name), "Invalid workspace name");
    let mut state = state::read(paths)?;
    ensure!(
        backend.list()?.iter().any(|c| c.name == name),
        "Container not found: {name}"
    );
    backend.remove(name)?;
    state.deployments.retain(|r| r.spec.name != name);
    state::write(paths, &state)?;
    Ok(format!("Deleted {name}. Home and projects were preserved."))
}
