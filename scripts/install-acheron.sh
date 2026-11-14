#!/usr/bin/env bash
set -euo pipefail

release_tag="${1:-acheron-preview}"
[[ "$release_tag" =~ ^[a-zA-Z0-9_.-]+$ ]] || { echo "Invalid release tag" >&2; exit 1; }
[[ "$(uname -s)" == Linux ]] || { echo "Acheron currently supports Linux hosts." >&2; exit 1; }
case "$(uname -m)" in
    x86_64) target=x86_64-unknown-linux-musl ;;
    aarch64|arm64) target=aarch64-unknown-linux-musl ;;
    *) echo "Unsupported architecture" >&2; exit 1 ;;
esac
command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
archive="acheron-$target.tar.gz"
download_dir="$(mktemp -d)"
trap 'rm -rf -- "$download_dir"' EXIT
release_url="https://github.com/euphorlc/.dotfiles/releases/download/$release_tag"
curl --proto '=https' --tlsv1.2 -fSL "$release_url/$archive" -o "$download_dir/$archive"
curl --proto '=https' --tlsv1.2 -fSL "$release_url/$archive.sha256" -o "$download_dir/$archive.sha256"
(
    cd "$download_dir"
    sha256sum --check "$archive.sha256"
    tar -xzf "$archive" acheron
    ./acheron --version
)
install -D -m 0755 "$download_dir/acheron" "$HOME/.local/bin/acheron"
echo "Installed only the Acheron binary to $HOME/.local/bin/acheron"
