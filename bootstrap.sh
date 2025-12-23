#!/bin/bash

set -e  # Exit on error.
set -u  # Error on undefined variables.

DOTFILES_DIR="$HOME/dotfiles"
REPO_URL="https://github.com/euphorlc/dotfiles.git"

install() {
    local dependency=$1
    if ! command -v $dependency >/dev/null 2>&1; then
        echo "[*] Installing ${dependency^}..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            brew install $dependency
        elif [[ -f /etc/debian_version ]]; then
            sudo apt update && sudo apt install -y $dependency 1>/de v/null
        elif [[ -f /etc/redhat-release ]]; then
            sudo dnf install -y -q $dependency
        else
        echo "Please install ${dependency^} manually."
        exit 1
        fi
    fi
}

echo "[!] Starting Bootstrap..."

install "stow"

# Clone the dotfiles repo if it doesn't exist.
if [[ ! -d "$DOTFILES_DIR" ]]; then
    echo "[*] Cloning dotfiles into $DOTFILES_DIR"
    git clone -q "$REPO_URL" "$DOTFILES_DIR"
fi

# Run stow to create symlinks.
cd "$DOTFILES_DIR"
echo "[*] Stowing Dotfiles"
stow .

# Install and configure Fish shell.
install "fish"
echo "[*] Setting Fish as default shell..."
chsh -s "$(which fish)" "$USER"

# Install and configure tmux.
install "tmux"
git clone -q https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm
~/.tmux/plugins/tpm/bin/install_plugins

# Install keychain.
install "keychain"

# Install Starship.
echo "[*] Installing Starship..."
curl -sS https://starship.rs/install.sh | sh -s -- -y 1>/dev/null

echo "[✓] Bootstrap complete!"
fish
