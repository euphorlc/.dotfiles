#!/bin/bash
set -euo pipefail

DOTFILES_DIR="$HOME/.dotfiles"
REPO_URL="https://github.com/euphorlc/.dotfiles.git"

# --- Step 1: Check for Stow ---
if ! command -v stow >/dev/null 2>&1; then
  echo "[x] Error: GNU Stow is not installed on this system." >&2
  echo "Please install 'stow' using your system's package manager and run bootstrap.sh again." >&2
  exit 1
fi

# --- Step 2: Clone Dotfiles Repository ---
if [[ ! -d "$DOTFILES_DIR" ]]; then
  echo "[*] Cloning dotfiles repository into $DOTFILES_DIR..."
  git clone "$REPO_URL" "$DOTFILES_DIR"
else
  echo "[*] Dotfiles directory found at $DOTFILES_DIR."
fi

# --- Step 3: Stow Configuration Files ---
cd "$DOTFILES_DIR"
echo "[*] Stowing dotfiles..."
stow .

# --- Step 4: Trigger Distrobox Assembly ---
if command -v distrobox >/dev/null 2>&1; then
  if [[ -f "$DOTFILES_DIR/distrobox.ini" ]]; then
    echo "[*] Assembling Distrobox containers..."
    distrobox assemble create --file "$DOTFILES_DIR/distrobox.ini"
  fi
else
  echo "[!] Warning: Distrobox is not installed on this host. Skipping container assembly."
fi

echo "[✓] Deployment complete!"
