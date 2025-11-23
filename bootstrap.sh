#!/bin/bash

set -e  # Exit on error
set -u  # Error on undefined variables

# --- Configuration ---
DOTFILES_DIR="$HOME/dotfiles"
REPO_URL="https://github.com/euphorlc/dotfiles.git"

echo "[!] Starting Bootstrap..."

# --- Install GNU Stow ---
if ! command -v stow >/dev/null 2>&1; then
  echo "[*] Installing GNU Stow..."
  if [[ "$OSTYPE" == "darwin"* ]]; then
    brew install stow
  elif [[ -f /etc/debian_version ]]; then
    sudo apt update && sudo apt install -y stow 1>/dev/null
  elif [[ -f /etc/redhat-release ]]; then
    sudo dnf install -y -q stow
  else
    echo "Please install GNU Stow manually."
    exit 1
  fi
fi

# --- Clone the dotfiles repo if it doesn't exist ---
if [[ ! -d "$DOTFILES_DIR" ]]; then
  echo "[*] Cloning dotfiles into $DOTFILES_DIR"
  git clone -q "$REPO_URL" "$DOTFILES_DIR"
fi

# --- Run stow to create symlinks ---
cd "$DOTFILES_DIR"
echo "[*] Stowing Dotfiles"
stow .

# --- Install Fish Shell ---
if ! command -v fish >/dev/null 2>&1; then
  echo "[*] Installing Fish Shell..."
  if [[ "$OSTYPE" == "darwin"* ]]; then
    brew install fish
  elif [[ -f /etc/debian_version ]]; then
    sudo apt update && sudo apt install -y fish 1>/dev/null
    echo "[*] Setting Fish as default shell..."
    chsh -s "$(which fish)" "$USER"
  elif [[ -f /etc/redhat-release ]]; then
    sudo dnf install -y -q fish
  else
    echo "Please install Fish Shell manually."
    exit 1
  fi
fi

# --- Install Starship
echo "[*] Installing Starship..."
curl -sS https://starship.rs/install.sh | sh -s -- -y 1>/dev/null

echo "[✓] Bootstrap complete!"
fish