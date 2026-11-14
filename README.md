# Acheron dotfiles and workspaces

Acheron is a Rust terminal application for deploying themed Distrobox workspaces.
Your Git repository describes the desired workspaces; Acheron plans changes,
creates missing containers, and links the shared configuration into separate,
persistent workspace homes. Rust and development dependencies stay in containers.

## Test this branch on Fedora Silverblue

Run these commands in a **host terminal**, outside Distrobox:

```bash
cd ~/.dotfiles
git fetch origin
git switch feat/acheron-workspace-deployment
```

Wait for the **Checks**, **Acheron binaries**, and **Workspace images** workflows
on this branch to succeed. The first image build compiles Starship and can take
several minutes. Then download the prebuilt binary and stow the host package:

```bash
bash bootstrap.sh --download acheron-preview
mkdir -p ~/Projects
~/.local/bin/acheron --demo
~/.local/bin/acheron doctor
~/.local/bin/acheron plan
~/.local/bin/acheron apply
~/.local/bin/acheron enter rust-dev
```

Only Acheron's binary is installed on the host by the download step. Bootstrap
also links the small host configuration package; it never installs system
packages, changes the login shell, or creates/deletes containers. The host must
already have Distrobox, its configured engine (normally Podman), and GNU Stow.
For bare `acheron` commands, ensure `~/.local/bin` is on your host shell's PATH.

GHCR packages may initially be private. After the first image publication, make
`acheron-base` and `acheron-dev` public in your GitHub package settings, or
authenticate your host to GHCR with `podman login ghcr.io`. Confirm image access:

```bash
podman pull ghcr.io/euphorlc/acheron-dev:preview
```

The published workspace images currently target **x86_64**. Release binaries are
built for x86_64 and aarch64; ARM users must provide a compatible workspace image.
If published images are unavailable, build the same images locally using Podman:

```bash
bash scripts/build-images.sh preview
```

That builds development tools inside images, not on the host. No host Rust
installation is needed. Existing containers named `dev`, `mars`, and `vulcan`
are left alone; the default workspace is named `rust-dev` to avoid a collision.

## New computer workflow

```bash
git clone https://github.com/euphorlc/.dotfiles.git ~/.dotfiles
cd ~/.dotfiles
# While testing, select feat/acheron-workspace-deployment here.
bash bootstrap.sh --download acheron-preview
mkdir -p ~/Projects
~/.local/bin/acheron --profile laptop doctor
~/.local/bin/acheron --profile laptop plan
~/.local/bin/acheron --profile laptop apply
```

Select the same profile on subsequent commands, including the TUI. The default
profile contains `rust-dev`; `laptop` replaces that entry with a Vulcan theme;
`desktop` adds an `experiments` workspace. Profiles are full-entry overlays:
matching workspace names replace the entire default entry, including mounts.
They do not automatically remove defaults. Edit profiles to match your machines.
Paths beginning with `~/` resolve against the host home on the current computer.

## Repository layout

```text
acheron/                 Rust CLI, TUI, deployment engine, tests
workspaces/default.toml  Desired workspaces, committed to Git
workspaces/machines/     Optional machine profile overlays
stow/host/               Minimal host configuration
stow/terminal/           Shared Fish, Starship, and tmux configuration
images/base/             Common terminal tools and preinstalled tmux plugins
images/dev/              Rust, rust-analyzer, and native compiler dependencies
scripts/                 Binary installation and local image builds
.github/workflows/       Checks, workspace images, and portable release binaries
```

Host configuration is stowed with `stow --no-folding --dir stow --target "$HOME"
host`. Acheron stows the `terminal` package into each workspace home, using
`--no-folding` so generated configuration and shell state do not end up in Git.
Root `.config` and tmux symlinks retain the paths referenced by earlier Stow
installations. Existing host terminal settings remain usable; new installations
only stow the `host` package unless you explicitly choose `terminal` as well.
Do not use plain `stow .` with the new package layout; use bootstrap or name the
package explicitly.

## Workspace definitions and data

```toml
version = 1

[[workspace]]
name = "rust-dev"
image = "ghcr.io/euphorlc/acheron-dev:preview"
theme = "mars"

[[workspace.mounts]]
source = "~/Projects"
target = "/workspace/projects"
read_only = false
```

Themes are independent of workspace names and images. Multiple workspaces can
use the same theme. Image and mount changes require an explicit rebuild; theme
changes need only `apply` followed by a new shell. Linked dotfile content updates
are immediately visible when you pull Git changes; applications may need reloads.

| Data | Default location | Lifetime |
| --- | --- | --- |
| Desired definitions | `~/.dotfiles/workspaces/` | Commit and restore with Git |
| Workspace homes | `~/.local/share/acheron/homes/NAME/` | Survive rebuild and deletion |
| Deployment receipts and lock | `~/.local/state/acheron/` | Local to this computer |
| Projects | `~/Projects`, mounted at `/workspace/projects` | Survive factory reset |
| System tools | Workspace image/container | Replaced on rebuild |

`XDG_DATA_HOME` and `XDG_STATE_HOME` override the data and state base paths.
Keep the same values when managing existing workspaces. Use `--repo PATH` or
`ACHERON_REPO` if your checkout is not `~/.dotfiles`. Paths currently support
ASCII letters, digits, `/`, `.`, `_`, and `-`; spaces and shell metacharacters
are rejected because Distrobox processes some arguments through shell commands.
Mount destinations must be below `/workspace`. Project directories must exist
and must be outside all managed workspace homes.

Workspace home separation prevents accidental sharing of Cargo caches,
configuration, and shell history. Distrobox still integrates host resources;
these are trusted development environments, not security sandboxes. Secrets
and personal data are never committed by Acheron. Use host SSH-agent forwarding
or configure credentials explicitly; private keys are not copied into homes.

## Lifecycle and safety

```bash
acheron                   # Interactive dashboard
acheron --demo            # Read-only preview, including inside containers
acheron doctor            # Check requirements and mount sources
acheron plan              # Read-only desired-vs-current comparison
acheron plan --offline    # Definitions preview without Distrobox
acheron apply             # Create missing workspaces, synchronize configuration
acheron apply rust-dev    # Apply one definition
acheron enter rust-dev    # Distrobox entry using Fish
acheron rebuild rust-dev  # Type name to confirm; preserve home and projects
acheron factory-reset rust-dev # Type name to confirm; DELETE workspace home
acheron delete rust-dev   # Type name to confirm; preserve home and projects
```

Scripts may pass `--confirm rust-dev` to destructive commands. Acheron does not
pass `--rm-home` to Distrobox. Factory reset verifies its home ownership marker
and path before removing that directory. Files you store directly inside that
home, including uncommitted projects, are deleted by factory reset; keep projects
in the explicit external project mount.

In the TUI, `n` saves workspace definitions, `a` applies the plan, `x` rebuilds,
`F` factory-resets, `d` deletes, `r` refreshes, and `q` quits. Arrow keys select a
workspace; PageUp/PageDown scroll activity. Dialogs use Tab to move fields,
Left/Right to select a theme, Enter to submit, and Esc to cancel. New definitions
are written into the selected profile in your checkout; commit them to Git to
deploy them on another computer. Operation output appears after completion.

Apply never automatically replaces or deletes containers. Layout differences
are reported as `rebuild required`. Existing same-name containers without a
matching deployment receipt/container ID are conflicts, never silently adopted.
Other containers are left alone. Rebuild can recover after a failed creation;
ordinary apply can recreate a missing container while retaining its home.
Operations are sequential, with a process lock. A failed batch can leave earlier
workspaces created; refresh the plan and retry. There is no automatic rollback.

Delete preserves the Git definition. A subsequent apply recreates that workspace.
To retire it permanently, also remove its definition from the applicable profile.
Legacy Orbit state and existing Distrobox containers are not automatically
migrated; use distinct names or explicitly retire the old containers yourself.
Do not copy local deployment receipts between computers.

## Images and release automation

The base image includes Fish, Starship, tmux, GNU Stow, Git, keychain, and tmux
plugins. The development image adds Rust 1.100, Cargo, rustfmt, Clippy,
rust-analyzer, Rust sources, GCC, Make, pkg-config, and OpenSSL headers. The Rust
toolchain is managed in the image under `/opt`; writable Cargo caches and
`cargo install` binaries live in the separate persistent home. Update the image
to change its preinstalled toolchain. Plugin versions are likewise image-managed.
No network installers run as container initialization hooks.

Image workflows publish `preview` on this branch, `latest` on main, and
`sha-COMMIT` tags. Preview definitions deliberately use `preview` during testing.
For reproducible deployments, replace a mutable image tag with its published
`@sha256:...` digest in your definitions. Mutable tags are not automatically
resolved or updated by Acheron. Pull a refreshed tag before rebuilding to use it.
Images use pinned Fedora/Rust/Starship release versions, but distro package
updates and plugin repository heads can change between image builds; the image
digest is the exact deployment reference.

Binary automation builds static musl executables and publishes an
`acheron-preview` prerelease for this branch, with per-architecture SHA256 files.
The preview release notes identify the build commit. Pushing an `acheron-v*` tag
publishes a versioned release. Downloads verify checksums before installation.
The GitHub Actions settings must permit workflow package/release publishing.

## Build inside a development container

```bash
cd ~/.dotfiles/acheron
cargo build --release --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

The locally built binary is `acheron/target/release/acheron`. Copy only that
binary to the host if using a local build; GNU builds require compatible host
libc. The CI musl binaries avoid that requirement. Acheron detects container
execution and rejects host management there; `--demo` and `plan --offline` work
inside the dev container. No host installation is performed by building/tests.
