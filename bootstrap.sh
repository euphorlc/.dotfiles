#!/usr/bin/env bash
set -euo pipefail

# Run after cloning. No package installation and no container creation/deletion.
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
if [[ -f /run/.containerenv || -f /.dockerenv || -n "${CONTAINER_ID:-}" ]]; then
    echo "Run bootstrap on the host. Build Acheron inside your dev container instead." >&2
    exit 1
fi
command -v stow >/dev/null || { echo "GNU Stow is required on the host." >&2; exit 1; }
if [[ "${1:-}" == "--download" && $# -le 2 ]]; then
    bash "$repo_dir/scripts/install-acheron.sh" "${2:-acheron-preview}"
elif [[ "${1:-}" == "--binary" && $# == 2 ]]; then
    binary_path="$2"
    [[ -f "$binary_path" && -x "$binary_path" ]] || { echo "Not an executable: $binary_path" >&2; exit 1; }
    "$binary_path" --version
    install -D -m 0755 "$binary_path" "$HOME/.local/bin/acheron"
elif [[ $# != 0 ]]; then
    echo "Usage: bash bootstrap.sh [--binary /path/to/acheron | --download [release-tag]]" >&2
    exit 1
fi
stow --no-folding --restow --dir "$repo_dir/stow" --target "$HOME" host
echo "Host configuration stowed. No containers were changed."
echo "Add ~/.local/bin to your host shell's PATH, then run:"
echo "  acheron --repo '$repo_dir' doctor"
echo "  acheron --repo '$repo_dir' plan"
echo "  acheron --repo '$repo_dir' apply"
