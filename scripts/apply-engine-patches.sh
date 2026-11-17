#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="${1:-$ROOT_DIR/engine/horizon-upstream}"
PATCH_DIR="${2:-$ROOT_DIR/patches}"

if [[ ! -d "$ENGINE_DIR/.git" && ! -f "$ENGINE_DIR/.git" ]]; then
  echo "Horizon upstream submodule is missing: $ENGINE_DIR" >&2
  exit 2
fi

while IFS= read -r patch_file; do
  [[ -z "$patch_file" ]] && continue
  echo "Checking $patch_file"
  git -C "$ENGINE_DIR" apply --check "$patch_file"
  git -C "$ENGINE_DIR" apply "$patch_file"
done < <(find "$PATCH_DIR" -maxdepth 1 -type f -name '*.patch' | sort)
