mod ui;

use acheron::{
    backend::{self, Backend, Distrobox},
    config::{self, Paths},
    engine,
    environment::Environment,
    state,
};
use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use std::{
    io::{self, IsTerminal, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Reproducible, themed Distrobox workspaces")]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Dotfiles repository (default: ACHERON_REPO or ~/.dotfiles)"
    )]
    repo: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        default_value = "default",
        help = "Machine profile overlay"
    )]
    profile: String,
    #[arg(long, help = "Preview the TUI without commands or filesystem access")]
    demo: bool,
    #[command(subcommand)]
    command: Option<Operation>,
}

#[derive(Subcommand)]
enum Operation {
    /// Check host prerequisites, manifests, and mount paths
    Doctor,
    /// Preview actions; never creates containers or homes
    Plan {
        #[arg(long, help = "Preview definitions without querying Distrobox")]
        offline: bool,
    },
    /// Create missing workspaces and sync configuration; never rebuilds automatically
    Apply { name: Option<String> },
    /// Enter a workspace using Fish
    Enter { name: String },
    /// Replace the container, preserving its home and projects
    Rebuild {
        name: String,
        #[arg(long)]
        confirm: Option<String>,
    },
    /// Replace the container AND delete its managed home; project mounts survive
    FactoryReset {
        name: String,
        #[arg(long)]
        confirm: Option<String>,
    },
    /// Delete a container, preserving its home and projects
    Delete {
        name: String,
        #[arg(long)]
        confirm: Option<String>,
    },
}

fn confirm(name: &str, provided: Option<&str>, factory: bool) -> Result<()> {
    if let Some(value) = provided {
        ensure!(value == name, "--confirm must exactly match {name}");
        return Ok(());
    }
    ensure!(
        io::stdin().is_terminal(),
        "Destructive operation requires --confirm {name} in noninteractive mode"
    );
    if factory {
        eprintln!(
            "Factory reset permanently deletes {name}'s workspace home, including toolchains, history, and files stored there. Project mounts are preserved."
        );
    } else {
        eprintln!(
            "This removes {name}'s container and installed system packages. Its home is preserved."
        );
    }
    print!("Type {name} to continue: ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    ensure!(
        answer.trim() == name,
        "Cancelled: workspace name did not match"
    );
    Ok(())
}

fn doctor(paths: &Paths, profile: &str) -> Result<()> {
    let environment = Environment::detect();
    println!(
        "Acheron {}\nOS: {}\nLocation: {}\nOSTree: {}\nRepository: {}",
        env!("CARGO_PKG_VERSION"),
        environment.os,
        if environment.in_container {
            "container (management disabled)"
        } else {
            "host"
        },
        environment.immutable,
        paths.repo.display()
    );
    let mut problems = Vec::new();
    for program in ["distrobox", "stow"] {
        let found = backend::available(program);
        println!("{} {program}", if found { "OK" } else { "MISSING" });
        if !found {
            problems.push(format!("Install {program} on the host"));
        }
    }
    let manager = std::env::var("DBX_CONTAINER_MANAGER").ok();
    let managers: Vec<&str> = manager
        .as_deref()
        .map(|m| vec![m])
        .unwrap_or_else(|| vec!["podman", "docker", "lilipod"]);
    if !managers.iter().any(|m| backend::available(m)) {
        problems.push("No configured container engine found (usually Podman)".into());
    }
    for spec in config::load(paths, profile)? {
        println!(
            "Workspace: {} / {} / {}",
            spec.name,
            spec.theme,
            spec.home.display()
        );
        if let Err(e) = engine::check_mounts(&spec) {
            problems.push(e.to_string());
        }
    }
    if environment.in_container {
        problems.push("Run management commands from your Silverblue host terminal".into());
    }
    for problem in &problems {
        eprintln!("- {problem}");
    }
    ensure!(
        problems.is_empty(),
        "Doctor found {} issue(s)",
        problems.len()
    );
    println!("Ready. Use acheron plan to review deployment actions.");
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.demo {
        ensure!(
            cli.command.is_none(),
            "--demo cannot be combined with a subcommand"
        );
        return ui::run(None, cli.profile, true);
    }
    let paths = Paths::discover(cli.repo)?;
    let Some(command) = cli.command else {
        let _lock = state::lock(&paths)?;
        return ui::run(Some(paths), cli.profile, false);
    };
    if matches!(command, Operation::Doctor) {
        return doctor(&paths, &cli.profile);
    }
    if let Operation::Plan { offline } = command {
        let specs = config::load(&paths, &cli.profile)?;
        let live = if offline {
            vec![]
        } else {
            Environment::detect().require_host()?;
            Distrobox.list()?
        };
        let state = if offline {
            state::State::default()
        } else {
            state::read(&paths)?
        };
        if offline {
            println!("OFFLINE: showing creation definitions; existing containers are not checked.");
        }
        for step in engine::plan(&specs, &live, &state) {
            println!(
                "{:<18} {:<20} {}\n  image: {}\n  home:  {}",
                step.spec.name,
                step.action,
                step.spec.theme,
                step.spec.image,
                step.spec.home.display()
            );
            for mount in &step.spec.mounts {
                println!(
                    "  mount: {} → {} ({})",
                    mount.source,
                    mount.target,
                    if mount.read_only { "ro" } else { "rw" }
                );
            }
        }
        for c in live
            .iter()
            .filter(|c| !specs.iter().any(|s| s.name == c.name))
        {
            println!("{:<18} external/unselected (left alone)", c.name);
        }
        return Ok(());
    }
    Environment::detect().require_host()?;
    if let Operation::Enter { ref name } = command {
        ensure!(config::valid_name(name), "Invalid workspace name");
        return backend::enter(name);
    }
    let _lock = state::lock(&paths)?;
    let desired = config::load(&paths, &cli.profile)?;
    match command {
        Operation::Apply { name } => {
            let specs = if let Some(name) = name {
                vec![
                    desired
                        .iter()
                        .find(|s| s.name == name)
                        .context("Workspace not found in profile")?
                        .clone(),
                ]
            } else {
                desired
            };
            print!("{}", engine::apply(&paths, &specs, &mut Distrobox)?);
        }
        Operation::Rebuild {
            name,
            confirm: answer,
        } => {
            let spec = desired
                .iter()
                .find(|s| s.name == name)
                .context("Workspace needs a Git-backed definition before rebuilding")?;
            confirm(&name, answer.as_deref(), false)?;
            println!("{}", engine::rebuild(&paths, spec, false, &mut Distrobox)?);
        }
        Operation::FactoryReset {
            name,
            confirm: answer,
        } => {
            let spec = desired
                .iter()
                .find(|s| s.name == name)
                .context("Workspace needs a Git-backed definition before rebuilding")?;
            confirm(&name, answer.as_deref(), true)?;
            println!("{}", engine::rebuild(&paths, spec, true, &mut Distrobox)?);
        }
        Operation::Delete {
            name,
            confirm: answer,
        } => {
            ensure!(config::valid_name(&name), "Invalid workspace name");
            confirm(&name, answer.as_deref(), false)?;
            println!("{}", engine::delete(&paths, &name, &mut Distrobox)?);
            println!(
                "Its definition was retained. Remove it from Git if it should stay absent on the next apply."
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destructive_confirmation_must_match_exactly() {
        assert!(confirm("dev", Some("de"), false).is_err());
        assert!(confirm("dev", Some("dev"), true).is_ok());
    }
}
