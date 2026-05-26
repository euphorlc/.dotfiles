# Euphorlc's Dotfiles

My personal dotfiles, managed with [GNU Stow](https://www.gnu.org/software/stow/) and bootstrapped via a single shell script.

---

## Quick Start

On a fresh machine, run the following command to clone the repo and set everything up automatically:

```bash
bash <(curl -sL https://raw.githubusercontent.com/euphorlc/dotfiles/main/bootstrap.sh)
```

This will:

1. Install GNU Stow if it isn't already present
2. Clone this repository into `~/dotfiles`
3. Run `stow .` to symlink all dotfiles into your home directory

---

## Prerequisites

- **Git** — required to clone the repository
- **curl** — required to fetch the bootstrap script
- **A supported OS** — the bootstrap script handles package installation for:
  - macOS (via [Homebrew](https://brew.sh))
  - Debian/Ubuntu (via `apt`)
  - Fedora/RHEL (via `dnf`)

---

## How It Works

This repo uses GNU Stow to manage symlinks. Stow mirrors the directory structure of this repo into your home directory, so a file at `~/dotfiles/.config/nvim/init.lua` becomes symlinked to `~/.config/nvim/init.lua`.

To manually re-stow after pulling changes:

```bash
cd ~/dotfiles
stow .
```

To unstow (remove all symlinks):

```bash
cd ~/dotfiles
stow -D .
```

---

## Manual Setup

If you prefer to set things up yourself without the bootstrap script:

```bash
# 1. Install GNU Stow
brew install stow          # macOS
sudo apt install -y stow   # Debian/Ubuntu
sudo dnf install -y stow   # Fedora/RHEL

# 2. Clone the repo
git clone https://github.com/euphorlc/dotfiles.git ~/dotfiles

# 3. Apply symlinks
cd ~/dotfiles
stow .
```
