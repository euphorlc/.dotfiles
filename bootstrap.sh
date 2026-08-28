#!/bin/bash
set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────────────
DOTFILES_DIR="$HOME/.dotfiles"
REPO_URL="https://github.com/euphorlc/.dotfiles.git"
MARS_HOME="$HOME/containers/mars"
VULCAN_HOME="$HOME/containers/vulcan"

# Colors for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[*]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[x]${NC} $1" >&2
}

print_info() {
    echo -e "${BLUE}[i]${NC} $1"
}

# ─── Step 1: Check for Stow ────────────────────────────────────────────────
if ! command -v stow >/dev/null 2>&1; then
    print_error "GNU Stow is not installed on this system."
    echo "Please install 'stow' using your system's package manager and run bootstrap.sh again."
    exit 1
fi

# ─── Step 2: Clone or Update Dotfiles Repository ─────────────────────────
if [[ ! -d "$DOTFILES_DIR" ]]; then
    print_status "Cloning dotfiles repository into $DOTFILES_DIR..."
    git clone "$REPO_URL" "$DOTFILES_DIR"
else
    print_status "Dotfiles directory found at $DOTFILES_DIR."
    if [[ -d "$DOTFILES_DIR/.git" ]]; then
        print_status "Pulling latest changes..."
        git -C "$DOTFILES_DIR" pull --rebase
    fi
fi

cd "$DOTFILES_DIR"

# ─── Step 3: Stow to Host ──────────────────────────────────────────────────
print_status "Stowing dotfiles to host home directory..."
stow .

# ─── Step 4: Create Isolated Container Homes ─────────────────────────────
print_status "Creating isolated home directories for containers..."
mkdir -p "$MARS_HOME" "$VULCAN_HOME"

# ─── Step 5: Stow Configs to Isolated Homes ──────────────────────────────
print_status "Stowing dotfiles to Mars container home..."
stow -t "$MARS_HOME" .

print_status "Stowing dotfiles to Vulcan container home..."
stow -t "$VULCAN_HOME" .

# ─── Step 6: Write Environment Marker Files ──────────────────────────────
print_status "Writing environment marker files..."
echo "mars" > "$MARS_HOME/.env"
echo "vulcan" > "$VULCAN_HOME/.env"

# ─── Step 7: Check for Distrobox ──────────────────────────────────────────
if ! command -v distrobox >/dev/null 2>&1; then
    print_warning "Distrobox is not installed on this host."
    echo "Container homes have been prepared at:"
    echo "  - $MARS_HOME"
    echo "  - $VULCAN_HOME"
    exit 0
fi

# ─── Step 8: Create Distrobox Containers ──────────────────────────────────
# Note: We use TMUX_TMPDIR to isolate sockets instead of fighting /tmp mounts
print_status "Creating Mars container (penetration testing environment)..."
distrobox create \
    --image registry.fedoraproject.org/fedora:41 \
    --name mars \
    --home "$MARS_HOME" \
    --env TMUX_TMPDIR=/tmp/mars-tmux \
    --additional-packages "git,fish,tmux,stow" \
    --init-hooks "curl -sS https://starship.rs/install.sh | sh -s -- -y; chsh -s /usr/bin/fish"

print_status "Creating Vulcan container (development environment)..."
distrobox create \
    --image registry.fedoraproject.org/fedora:41 \
    --name vulcan \
    --home "$VULCAN_HOME" \
    --env TMUX_TMPDIR=/tmp/vulcan-tmux \
    --additional-packages "git,fish,tmux,stow" \
    --init-hooks "curl -sS https://starship.rs/install.sh | sh -s -- -y; chsh -s /usr/bin/fish"

# ─── Step 9: Post-Setup Instructions ──────────────────────────────────────
echo ""
print_status "✅ Deployment complete!"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "To enter your environments:"
echo "  ${GREEN}distrobox enter mars${NC}     # Penetration testing"
echo "  ${GREEN}distrobox enter vulcan${NC}   # Development"
echo ""
echo "Tmux sessions are isolated via TMUX_TMPDIR:"
echo "  Mars uses   : /tmp/mars-tmux"
echo "  Vulcan uses : /tmp/vulcan-tmux"
echo ""
echo "Install additional tools manually inside each container:"
echo "  ${BLUE}distrobox enter mars${NC}  → sudo dnf install nmap python3-pip ..."
echo "  ${BLUE}distrobox enter vulcan${NC} → sudo dnf install gcc gcc-c++ make ..."
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
