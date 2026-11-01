#!/usr/bin/env bash
# Preserve the path layout; require clean actual GitHub main sources.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ui_dir="$(dirname "$(dirname "$repo_dir")")/glory-rs"
mkdir -p "$ui_dir"
resolve_main() {
    local remote="$1" result revision
    result="$(git ls-remote --exit-code --refs "$remote" refs/heads/main)" || return 1
    [[ "$result" != *$'\n'* && "$result" == *$'\trefs/heads/main' ]] || {
        echo "UI main must resolve to exactly one advertised ref" >&2; return 1;
    }
    revision="${result%%$'\t'*}"
    [[ "$revision" =~ ^[0-9a-f]{40}$ ]] || {
        echo "UI main has no exact commit identity" >&2; return 1;
    }
    printf '%s' "$revision"
}
checkout() {
    local name="$1" remote="$2" expected="$3" destination="$ui_dir/$1" revision actual_root actual_remote source_status actual_head
    revision="$(resolve_main "$remote")" || return 1
    [[ -z "$expected" || "$expected" == "$revision" ]] || {
        echo "Provided UI source ref no longer matches GitHub main" >&2; return 1;
    }
    [[ ! -L "$destination" ]] || { echo "UI checkout cannot be a symlink" >&2; return 1; }
    if [[ -e "$destination" ]]; then
        [[ -d "$destination" && -f "$destination/Cargo.toml" ]] || {
            echo "Invalid existing UI checkout" >&2; return 1;
        }
        actual_root="$(git -C "$destination" rev-parse --show-toplevel)" || return 1
        [[ "$actual_root" == "$(cd "$destination" && pwd -P)" ]] || {
            echo "UI destination is not its own repository root" >&2; return 1;
        }
        actual_remote="$(git -C "$destination" remote get-url --all origin)" || return 1
        [[ "$actual_remote" == "$remote" ]] || {
            echo "UI checkout origin differs from its required repository" >&2; return 1;
        }
    else
        git clone --no-checkout --branch main --single-branch "$remote" "$destination"
        git -C "$destination" checkout --detach "$revision"
    fi
    source_status="$(git -C "$destination" status --porcelain=v1 --untracked-files=all)" || return 1
    [[ -z "$source_status" ]] || {
        echo "UI checkout has local source changes" >&2; return 1;
    }
    actual_head="$(git -C "$destination" rev-parse HEAD)" || return 1
    [[ "$actual_head" == "$revision" ]] || {
        echo "Existing UI checkout is not the resolved main commit; keep it unchanged" >&2; return 1;
    }
    printf -v "${name}_revision" '%s' "$revision"
}
checkout glory https://github.com/glory-rs/glory.git "${GLORY_REF:-}"
checkout sapid https://github.com/sapid-ui/sapid.git "${SAPID_REF:-}"
# These are real path siblings consumed by the current Glory/Apace manifests.
checkout apace https://github.com/glory-rs/apace.git "${APACE_REF:-}"
checkout takia https://github.com/glory-rs/takia.git "${TAKIA_REF:-}"
checkout wingy https://github.com/glory-rs/wingy.git "${WINGY_REF:-}"
[[ "$(resolve_main https://github.com/glory-rs/glory.git)" == "$glory_revision" &&
   "$(resolve_main https://github.com/sapid-ui/sapid.git)" == "$sapid_revision" &&
   "$(resolve_main https://github.com/glory-rs/apace.git)" == "$apace_revision" &&
   "$(resolve_main https://github.com/glory-rs/takia.git)" == "$takia_revision" &&
   "$(resolve_main https://github.com/glory-rs/wingy.git)" == "$wingy_revision" ]] || {
    echo "UI main changed while preparing the source closure" >&2; exit 1;
}
output="${UI_SOURCE_REFS_OUTPUT:-$repo_dir/artifacts/ui-source-refs.json}"
mkdir -p "$(dirname "$output")"
printf '{"glory":"%s","sapid":"%s","apace":"%s","takia":"%s","wingy":"%s"}\n' "$glory_revision" "$sapid_revision" "$apace_revision" "$takia_revision" "$wingy_revision" > "$output"
printf 'Verified UI source refs: Glory %s; Sapid %s; Apace %s; Takia %s; Wingy %s\n' "$glory_revision" "$sapid_revision" "$apace_revision" "$takia_revision" "$wingy_revision"
