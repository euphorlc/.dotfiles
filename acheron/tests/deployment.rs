use acheron::{
    backend::{self, Backend, Container, Distrobox},
    config::{self, Mount, Paths, Resolved, Workspace},
    engine::{self, Action},
    state,
};
use anyhow::{Result, bail};
use std::{fs, path::PathBuf};
use tempfile::TempDir;

#[derive(Default)]
struct Fake {
    containers: Vec<Container>,
    creates: usize,
    removes: usize,
    configurations: usize,
    fail_create: bool,
}
impl Backend for Fake {
    fn list(&mut self) -> Result<Vec<Container>> {
        Ok(self.containers.clone())
    }
    fn create(&mut self, s: &Resolved) -> Result<()> {
        if self.fail_create {
            bail!("simulated image pull failure");
        }
        self.creates += 1;
        self.containers.push(Container {
            id: format!("id{}", self.creates),
            name: s.name.clone(),
            status: "Created".into(),
            image: s.image.clone(),
        });
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<()> {
        self.removes += 1;
        self.containers.retain(|c| c.name != name);
        Ok(())
    }
    fn configure(&mut self, _: &Resolved) -> Result<()> {
        self.configurations += 1;
        Ok(())
    }
}

fn fixture() -> (TempDir, Paths, Resolved) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let paths = Paths {
        repo: root.join("repo"),
        data: root.join("data"),
        state: root.join("state"),
        host_home: root.join("host"),
    };
    fs::create_dir_all(paths.repo.join("stow/terminal")).unwrap();
    fs::create_dir_all(paths.host_home.join("Projects")).unwrap();
    let spec = Workspace {
        name: "rust-dev".into(),
        image: "example.org/dev@sha256:abc".into(),
        theme: "mars".into(),
        mounts: vec![Mount {
            source: "~/Projects".into(),
            target: "/workspace/projects".into(),
            read_only: false,
        }],
    }
    .resolve(&paths)
    .unwrap();
    (tmp, paths, spec)
}

#[test]
fn apply_is_idempotent_and_does_not_rebuild_for_theme_changes() {
    let (_tmp, paths, mut spec) = fixture();
    let mut b = Fake::default();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    spec.theme = "vulcan".into();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    assert_eq!(b.creates, 1);
    assert_eq!(b.removes, 0);
    assert_eq!(b.configurations, 3);
    assert_eq!(
        state::read(&paths).unwrap().deployments[0].spec.theme,
        "vulcan"
    );
}

#[test]
fn layout_changes_require_explicit_rebuild_and_preserve_home() {
    let (_tmp, paths, mut spec) = fixture();
    let mut b = Fake::default();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    fs::write(spec.home.join("keep"), "cargo and project data").unwrap();
    spec.image = "example.org/dev:v2".into();
    let p = engine::plan(
        std::slice::from_ref(&spec),
        &b.containers,
        &state::read(&paths).unwrap(),
    );
    assert_eq!(p[0].action, Action::Rebuild);
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    assert_eq!(b.removes, 0);
    engine::rebuild(&paths, &spec, false, &mut b).unwrap();
    assert!(spec.home.join("keep").exists());
    assert_eq!(b.removes, 1);
    assert_eq!(b.creates, 2);
}

#[test]
fn factory_reset_clears_only_owned_home_and_preserves_projects() {
    let (_tmp, paths, spec) = fixture();
    let mut b = Fake::default();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    fs::write(spec.home.join("remove-me"), "cache").unwrap();
    let project = paths.host_home.join("Projects/main.rs");
    fs::write(&project, "work").unwrap();
    std::os::unix::fs::symlink(&project, spec.home.join("project-link")).unwrap();
    engine::rebuild(&paths, &spec, true, &mut b).unwrap();
    assert!(!spec.home.join("remove-me").exists());
    assert!(project.exists());
    assert!(spec.home.join(".acheron-owner").exists());
}

#[test]
fn deletion_keeps_home_and_apply_can_recreate_it() {
    let (_tmp, paths, spec) = fixture();
    let mut b = Fake::default();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    fs::write(spec.home.join("keep"), "data").unwrap();
    engine::delete(&paths, &spec.name, &mut b).unwrap();
    assert!(spec.home.join("keep").exists());
    assert!(state::read(&paths).unwrap().deployments.is_empty());
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    assert!(spec.home.join("keep").exists());
}

#[test]
fn external_or_replaced_containers_are_not_adopted() {
    let (_tmp, paths, spec) = fixture();
    let mut b = Fake::default();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    b.containers[0].id = "replaced-outside-acheron".into();
    assert_eq!(
        engine::plan(
            std::slice::from_ref(&spec),
            &b.containers,
            &state::read(&paths).unwrap()
        )[0]
        .action,
        Action::Conflict
    );
    assert!(engine::apply(&paths, std::slice::from_ref(&spec), &mut b).is_err());
    assert!(engine::rebuild(&paths, &spec, false, &mut b).is_err());
    assert_eq!(b.removes, 0);
}

#[test]
fn failed_rebuild_can_be_retried_without_losing_home() {
    let (_tmp, paths, spec) = fixture();
    let mut b = Fake::default();
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    fs::write(spec.home.join("keep"), "data").unwrap();
    b.fail_create = true;
    assert!(engine::rebuild(&paths, &spec, false, &mut b).is_err());
    assert!(spec.home.join("keep").exists());
    b.fail_create = false;
    engine::apply(&paths, std::slice::from_ref(&spec), &mut b).unwrap();
    assert_eq!(b.containers.len(), 1);
    assert!(spec.home.join("keep").exists());
}

#[test]
fn unowned_or_symlinked_homes_are_refused() {
    let (_tmp, paths, spec) = fixture();
    let mut b = Fake::default();
    fs::create_dir_all(&spec.home).unwrap();
    fs::write(spec.home.join("personal"), "important").unwrap();
    assert!(engine::apply(&paths, std::slice::from_ref(&spec), &mut b).is_err());
    assert_eq!(b.creates, 0);
    fs::remove_dir_all(&spec.home).unwrap();
    std::os::unix::fs::symlink(&paths.host_home, &spec.home).unwrap();
    assert!(engine::apply(&paths, &[spec], &mut b).is_err());
}

#[test]
fn project_sources_inside_managed_homes_are_refused() {
    let (_tmp, _paths, mut spec) = fixture();
    fs::create_dir_all(spec.home.join("Projects")).unwrap();
    spec.mounts[0].source = spec.home.join("Projects").display().to_string();
    assert!(engine::check_mounts(&spec).is_err());
}

#[test]
fn config_validation_rejects_shell_characters_and_ambiguous_mounts() {
    let (_tmp, paths, _) = fixture();
    for name in ["--all", "../home", "dev;id", "a b", ""] {
        assert!(!config::valid_name(name));
    }
    let mut workspace = Workspace {
        name: "valid".into(),
        image: "fedora:44".into(),
        theme: "mars".into(),
        mounts: vec![],
    };
    workspace.image = "fedora:44;id".into();
    assert!(workspace.resolve(&paths).is_err());
    workspace.image = "fedora:44".into();
    workspace.mounts = vec![Mount {
        source: "~/Projects".into(),
        target: "/home/user".into(),
        read_only: false,
    }];
    assert!(workspace.resolve(&paths).is_err());
    assert!(config::expand("~/../escape", &paths.host_home).is_err());
}

#[test]
fn create_command_explicitly_sets_home_mounts_and_socket_without_install_hooks() {
    let (_tmp, _paths, spec) = fixture();
    let args = backend::create_args(&spec);
    assert!(
        args.windows(2)
            .any(|p| p[0] == "--home" && p[1] == spec.home.display().to_string())
    );
    assert!(
        args.iter()
            .any(|a| a.contains("TMUX_TMPDIR=/tmp/acheron-rust-dev"))
    );
    assert!(!args.contains(&"--init-hooks".into()));
    assert!(!args.contains(&"--rm-home".into()));
}

#[test]
fn distrobox_parser_handles_statuses_and_fails_closed_on_bad_output() {
    let rows = backend::parse_list("ID | NAME | STATUS | IMAGE\na | dev | Up 1 hour | fedora:44\nb | web | Exited (0) | fedora:44\n").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].status, "Exited (0)");
    assert!(backend::parse_list("warning: unexpected output").is_err());
    assert!(backend::parse_list("").unwrap().is_empty());
}

#[test]
fn profile_overlay_replaces_full_entries_and_detects_duplicates() {
    let (_tmp, paths, _) = fixture();
    fs::create_dir_all(paths.repo.join("workspaces/machines")).unwrap();
    fs::write(
        paths.repo.join("workspaces/default.toml"),
        "version=1\n[[workspace]]\nname='dev'\nimage='fedora:44'\ntheme='mars'\n",
    )
    .unwrap();
    fs::write(
        paths.repo.join("workspaces/machines/laptop.toml"),
        "version=1\n[[workspace]]\nname='dev'\nimage='fedora:44'\ntheme='vulcan'\n",
    )
    .unwrap();
    let specs = config::load(&paths, "laptop").unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].theme, "vulcan");
    fs::write(paths.repo.join("workspaces/default.toml"), "version=1\n[[workspace]]\nname='dev'\nimage='fedora:44'\n[[workspace]]\nname='dev'\nimage='fedora:44'\n").unwrap();
    assert!(config::load(&paths, "default").is_err());
}

#[test]
fn stow_deploys_symlinks_and_theme_without_folding_or_adopting_conflicts() {
    if !backend::available("stow") {
        eprintln!("stow not installed; CI installs it for this integration test");
        return;
    }
    let (_tmp, paths, spec) = fixture();
    let source = paths.repo.join("stow/terminal/.config/fish");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("config.fish"), "# shared config").unwrap();
    fs::create_dir_all(&spec.home).unwrap();
    Distrobox.configure(&spec).unwrap();
    Distrobox.configure(&spec).unwrap();
    assert!(!spec.home.join(".config").is_symlink());
    assert!(spec.home.join(".config/fish/config.fish").is_symlink());
    assert!(
        fs::read_to_string(spec.home.join(".config/acheron/workspace.fish"))
            .unwrap()
            .contains("WORKSPACE_THEME mars")
    );
    fs::remove_file(spec.home.join(".config/fish/config.fish")).unwrap();
    fs::write(
        spec.home.join(".config/fish/config.fish"),
        "personal config",
    )
    .unwrap();
    assert!(Distrobox.configure(&spec).is_err());
    assert_eq!(
        fs::read_to_string(spec.home.join(".config/fish/config.fish")).unwrap(),
        "personal config"
    );
}

#[test]
fn state_lock_prevents_concurrent_mutation() {
    let (_tmp, paths, _) = fixture();
    let guard = state::lock(&paths).unwrap();
    assert!(state::lock(&paths).is_err());
    drop(guard);
    assert!(state::lock(&paths).is_ok());
}

#[test]
fn repository_profiles_resolve_with_no_host_specific_paths() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let (_tmp, mut paths, _) = fixture();
    paths.repo = repo;
    for profile in ["default", "laptop", "desktop"] {
        assert!(!config::load(&paths, profile).unwrap().is_empty());
    }
}
