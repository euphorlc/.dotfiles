#!/bin/bash

set -e  # Exit on error
set -u  # Error on undefined variables

DOTFILES_DIR="$HOME/dotfiles"
REPO_URL="https://github.com/euphorlc/dotfiles.git"

install() {
    local dependency=$1
    if ! command -v $dependency >/dev/null 2>&1; then
        echo "[*] Installing ${dependency^}..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            brew install $dependency
        elif [[ -f /etc/debian_version ]]; then
            sudo apt update && sudo apt install -y $dependency 1>/dev/null
        elif [[ -f /etc/redhat-release ]]; then
            sudo dnf install -y -q $dependency
        else
        echo "Please install ${dependency^} manually."
        exit 1
        fi
    fi
}

echo "[!] Starting Bootstrap..."

# Install all dependencies
install "stow"
install "fish"
install "tmux"
install "keychain"
install "curl"
install "git"

# Install starship
if ! command -v starship >/dev/null 2>&1; then
    echo "[*] Installing Starship..."
    curl -sS https://starship.rs/install.sh | sh -s -- -y 1>/dev/null
fi

# Clone the dotfiles repo if it doesn't exist
if [[ ! -d "$DOTFILES_DIR" ]]; then
    echo "[*] Cloning dotfiles into $DOTFILES_DIR..."
    git clone -q "$REPO_URL" "$DOTFILES_DIR"
fi

# Install tmux plugin manager
if [[ ! -d "$HOME/.tmux/plugins/tpm" ]]; then
    echo "[*] Installing TMUX Plugin Manager..."
    git clone -q https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm
fi

# Stow dotfiles
cd "$DOTFILES_DIR"
echo "[*] Stowing Dotfiles..."
stow --adopt .
git restore .

# Change default shell to fish
echo "[*] Setting Fish as default shell (You may be prompted for your password)..."
if [[ "$SHELL" != "$(which fish)" ]]; then
    chsh -s "$(which fish)" "$USER"
fi

echo "[✓] Bootstrap complete! Restart your terminal or type 'fish' to apply."
