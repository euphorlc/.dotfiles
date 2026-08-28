#!/bin/bash
set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────────────
DOTFILES_DIR="$HOME/.dotfiles"
REPO_URL="https://github.com/euphorlc/.dotfiles.git"
MARS_HOME="$HOME/containers/mars"
VULCAN_HOME="$HOME/containers/vulcan"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() { echo -e "${GREEN}[*]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[!]${NC} $1"; }
print_error() { echo -e "${RED}[x]${NC} $1" >&2; }
print_info() { echo -e "${BLUE}[i]${NC} $1"; }

# ─── Step 1: Check for Stow ────────────────────────────────────────────────
if ! command -v stow >/dev/null 2>&1; then
    print_error "GNU Stow is not installed."
    echo "Please install 'stow' and rerun."
    exit 1
fi

# ─── Step 2: Clone or Update Repository ──────────────────────────────────
if [[ ! -d "$DOTFILES_DIR" ]]; then
    print_status "Cloning dotfiles..."
    git clone "$REPO_URL" "$DOTFILES_DIR"
else
    print_status "Updating dotfiles..."
    git -C "$DOTFILES_DIR" pull --rebase
fi
cd "$DOTFILES_DIR"

# ─── Step 3: Stow to Host ──────────────────────────────────────────────────
print_status "Stowing to host..."
stow .

# ─── Step 4: Create Isolated Homes ──────────────────────────────────────
print_status "Creating container home directories..."
mkdir -p "$MARS_HOME" "$VULCAN_HOME"

# ─── Step 5: Stow to Each Home ───────────────────────────────────────────
print_status "Stowing to Mars home..."
stow -t "$MARS_HOME" .
print_status "Stowing to Vulcan home..."
stow -t "$VULCAN_HOME" .

# ─── Step 6: Write Environment Markers ──────────────────────────────────
echo "mars" > "$MARS_HOME/.env"
echo "vulcan" > "$VULCAN_HOME/.env"

# ─── Step 7: Check Distrobox ─────────────────────────────────────────────
if ! command -v distrobox >/dev/null 2>&1; then
    print_warning "Distrobox not installed – skipping container creation."
    exit 0
fi

# ─── Step 8: Clean up existing containers ────────────────────────────────
print_status "Removing existing containers (if any)..."
distrobox rm -f mars vulcan 2>/dev/null || true

# ─── Step 9: Create Containers ────────────────────────────────────────────
print_status "Creating Mars container..."
distrobox create \
    --image registry.fedoraproject.org/fedora:41 \
    --name mars \
    --home "$MARS_HOME" \
    --additional-flags "--env TMUX_TMPDIR=/tmp/mars-tmux" \
    --additional-packages "git fish tmux stow" \
    --init-hooks "curl -sS https://starship.rs/install.sh | sh -s -- -y; chsh -s /usr/bin/fish"

print_status "Creating Vulcan container..."
distrobox create \
    --image registry.fedoraproject.org/fedora:41 \
    --name vulcan \
    --home "$VULCAN_HOME" \
    --additional-flags "--env TMUX_TMPDIR=/tmp/vulcan-tmux" \
    --additional-packages "git fish tmux stow" \
    --init-hooks "curl -sS https://starship.rs/install.sh | sh -s -- -y; chsh -s /usr/bin/fish"

# ─── Step 10: Done ─────────────────────────────────────────────────────────
echo ""
print_status "✅ Deployment complete!"
echo ""
echo "  Enter environments:"
echo "    distrobox enter mars"
echo "    distrobox enter vulcan"
echo ""
echo "  Tmux sockets isolated via TMUX_TMPDIR."
echo "  Install extra tools manually inside each container."
