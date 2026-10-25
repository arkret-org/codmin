#!/usr/bin/env bash
# Reproduce the semantic UI dependency layout used by local Cargo paths.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ui_dir="$(dirname "$(dirname "$repo_dir")")/glory-rs"
mkdir -p "$ui_dir"
checkout() {
    local name="$1" revision="$2" destination="$ui_dir/$1"
    if [[ -e "$destination" ]]; then
        test -f "$destination/Cargo.toml" || { echo "Invalid existing UI dependency: $destination" >&2; exit 1; }
        echo "Reusing $destination at $(git -C "$destination" rev-parse HEAD)"
        return
    fi
    git clone --no-checkout "https://github.com/glory-rs/$name.git" "$destination"
    git -C "$destination" checkout --detach "$revision"
}
# Local aecbe360 adds rendering optimizations to this published API-compatible parent.
checkout glory b660dea9fc182270d2eb13bbf00a08b4dab1a49b
checkout sapid fd64d78edf9f91f9dc63bfa2be6e59ec1f739a58
