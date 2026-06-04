#!/bin/bash

set -e  # Exit on error
set -u  # Error on undefined variables

DOTFILES_DIR="$HOME/dotfiles"
REPO_URL="https://github.com/euphorlc/dotfiles.git"

# ─── Detect machine identity ──────────────────────────────────────────────────
MACHINE="unknown"
if [[ -f /etc/os-release ]]; then
    if grep -qi "kali" /etc/os-release; then
        MACHINE="ares"
    elif grep -qi "ubuntu" /etc/os-release; then
        MACHINE="hephaestus"
    fi
fi
echo "[*] Detected machine: ${MACHINE}"

# ─── Package installer helper ─────────────────────────────────────────────────
install() {
    local dependency=$1
    if ! command -v "$dependency" >/dev/null 2>&1; then
        echo "[*] Installing ${dependency^}..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            brew install "$dependency"
        elif [[ -f /etc/debian_version ]]; then
            sudo apt-get update -qq && sudo apt-get install -y -qq "$dependency"
        elif [[ -f /etc/redhat-release ]]; then
            sudo dnf install -y -q "$dependency"
        else
            echo "[!] Please install ${dependency^} manually."
            exit 1
        fi
    fi
}

echo "[!] Starting Bootstrap..."

# ─── Core dependencies ────────────────────────────────────────────────────────
install "stow"
install "fish"
install "tmux"
install "keychain"
install "curl"
install "git"

# ─── Detect WSL ───────────────────────────────────────────────────────────────
IS_WSL=false
if grep -qi "microsoft\|wsl" /proc/version 2>/dev/null; then
    IS_WSL=true
    echo "[*] WSL environment detected — skipping Alacritty and font install."
fi

# ─── Nerd Font installer helper ──────────────────────────────────────────────
# Downloads a zip release from Nerd Fonts GitHub, extracts .ttf files,
# installs them to ~/.local/share/fonts, and refreshes the font cache.
install_nerd_font() {
    local font_name="$1"    # Nerd Fonts release name e.g. "JetBrainsMono"
    local nf_version="v3.4.0"   # Pinned release — bump here to upgrade

    if fc-list | grep -qi "$font_name"; then
        echo "[✓] Font '${font_name}' already installed, skipping."
        return
    fi

    echo "[*] Installing Nerd Font: ${font_name}..."
    local font_dir="$HOME/.local/share/fonts"
    local tmp_zip="/tmp/${font_name}.zip"
    local tmp_dir="/tmp/${font_name}_nf"
    local zip_url="https://github.com/ryanoasis/nerd-fonts/releases/download/${nf_version}/${font_name}.zip"

    mkdir -p "$font_dir"
    curl -fsSL "$zip_url" -o "$tmp_zip"
    mkdir -p "$tmp_dir"
    unzip -q -o "$tmp_zip" "*.ttf" -d "$tmp_dir"
    cp "$tmp_dir"/*.ttf "$font_dir"/
    fc-cache -f "$font_dir"
    rm -rf "$tmp_zip" "$tmp_dir"
    echo "[✓] Font '${font_name}' installed."
}

if [[ "$IS_WSL" == false ]]; then
    # ─── Machine-specific font ────────────────────────────────────────────────────
    install "unzip"       # required to extract font zips
    install "fontconfig"  # provides fc-list and fc-cache

    if [[ "$MACHINE" == "hephaestus" ]]; then
        install_nerd_font "JetBrainsMono"
    elif [[ "$MACHINE" == "ares" ]]; then
        install_nerd_font "Terminus"
    else
        echo "[!] Unknown machine — skipping font install."
    fi

    # ─── Alacritty ────────────────────────────────────────────────────────────────
    if ! command -v alacritty >/dev/null 2>&1; then
        echo "[*] Installing Alacritty..."
        if [[ "$OSTYPE" == "darwin"* ]]; then
            brew install --cask alacritty
        elif [[ -f /etc/debian_version ]]; then
            # Alacritty is not in default apt repos on all distros; install via cargo or snap
            if command -v snap >/dev/null 2>&1; then
                sudo snap install alacritty --classic
            elif command -v cargo >/dev/null 2>&1; then
                cargo install alacritty
            else
                echo "[*] Installing Alacritty build dependencies..."
                sudo apt-get update -qq
                sudo apt-get install -y -qq \
                    cmake pkg-config libfreetype6-dev libfontconfig1-dev \
                    libxcb-xfixes0-dev libxkbcommon-dev python3 cargo
                cargo install alacritty
            fi
        else
            echo "[!] Please install Alacritty manually: https://alacritty.org"
        fi
    fi
fi

# ─── Starship ─────────────────────────────────────────────────────────────────
if ! command -v starship >/dev/null 2>&1; then
    echo "[*] Installing Starship..."
    curl -sS https://starship.rs/install.sh | sh -s -- -y 1>/dev/null
fi

# ─── Clone dotfiles ───────────────────────────────────────────────────────────
if [[ ! -d "$DOTFILES_DIR" ]]; then
    echo "[*] Cloning dotfiles into $DOTFILES_DIR..."
    git clone -q "$REPO_URL" "$DOTFILES_DIR"
fi

# ─── TMUX plugin manager ──────────────────────────────────────────────────────
if [[ ! -d "$HOME/.tmux/plugins/tpm" ]]; then
    echo "[*] Installing TMUX Plugin Manager..."
    git clone -q https://github.com/tmux-plugins/tpm ~/.tmux/plugins/tpm
fi

# ─── Stow dotfiles ────────────────────────────────────────────────────────────
cd "$DOTFILES_DIR"
echo "[*] Stowing Dotfiles..."
stow --adopt .
git restore .

# ─── Symlink machine-specific Alacritty config ────────────────────────────────
# Alacritty on Linux reads ~/.config/alacritty/alacritty.toml as its entry point.
# We symlink the machine-specific profile to that path so the correct theme and
# shell config loads automatically without any runtime detection needed.
if [[ "$IS_WSL" == false ]]; then
    ALACRITTY_CFG_DIR="$HOME/.config/alacritty"
    ALACRITTY_ENTRY="$ALACRITTY_CFG_DIR/alacritty.toml"

    if [[ "$MACHINE" != "unknown" ]]; then
        echo "[*] Linking Alacritty profile for ${MACHINE}..."

        # The base config was stowed to ~/.config/alacritty/alacritty.toml by stow.
        # We need to replace it with a symlink to the machine profile, which itself
        # imports the base config under a different key to avoid a circular reference.
        # Strategy: rename base → _base.toml, entry point → <machine>.toml symlink.

        if [[ -f "$ALACRITTY_CFG_DIR/_base.toml" || -L "$ALACRITTY_CFG_DIR/_base.toml" ]]; then
            : # already set up on a previous run
        else
            # Rename the stow-managed base to _base.toml
            mv "$ALACRITTY_ENTRY" "$ALACRITTY_CFG_DIR/_base.toml"
        fi

        # Point alacritty.toml → machine profile (profile imports _base.toml)
        ln -sf "$ALACRITTY_CFG_DIR/${MACHINE}.toml" "$ALACRITTY_ENTRY"
        echo "[✓] Alacritty entry point → ${MACHINE}.toml"
    else
        echo "[!] Unknown machine — skipping Alacritty profile symlink."
        echo "    Manually symlink ~/.config/alacritty/alacritty.toml to hephaestus.toml or ares.toml"
    fi
fi

# ─── Default shell → fish ─────────────────────────────────────────────────────
echo "[*] Setting Fish as default shell..."
if [[ "$SHELL" != "$(which fish)" ]]; then
    chsh -s "$(which fish)" "$USER"
fi

echo "[✓] Bootstrap complete! Restart your terminal or type 'fish' to apply."
