#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
image_tag="${1:-preview}"
[[ "$image_tag" =~ ^[a-zA-Z0-9_.-]+$ ]] || { echo "Invalid image tag" >&2; exit 1; }
podman build -f "$repo_dir/images/base/Containerfile" -t "ghcr.io/euphorlc/acheron-base:$image_tag" "$repo_dir"
podman build -f "$repo_dir/images/dev/Containerfile" \
    --build-arg "BASE_IMAGE=ghcr.io/euphorlc/acheron-base:$image_tag" \
    -t "ghcr.io/euphorlc/acheron-dev:$image_tag" "$repo_dir"
