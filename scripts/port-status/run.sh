#!/usr/bin/env bash
# Regenerates the port tracking documents.
#
#   scripts/port-status/run.sh            # upstream checkout at ../Avalonia (or $UPSTREAM)
#   scripts/port-status/run.sh --force    # always re-run the upstream extractor
#   scripts/port-status/run.sh --check    # exit 1 when the generated documents are out of date (TRACKING.md,
#                                         # REMAINING.md, the pages, port-status.json); runs no extraction
#
# Step 1 (only when stale): scripts/api-extract -> docs/porting/data/upstream-api.json
# Step 2: scripts/port-status/port_status.py -> docs/porting/TRACKING.md, docs/porting/REMAINING.md,
#         docs/porting/tracking/*.md, docs/porting/data/port-status.json
#
# The extractor reads the TRACKED commit of upstream, not the HEAD of the checkout (which is usually ahead):
# the commit recorded in upstream-api.json, or $TRACKED_COMMIT to move the port to another one. The tree of
# that commit, and of each of its submodules at the commit the tree names, is exported with `git archive`
# into a temporary directory (below $PORT_STATUS_TMP, else $TMPDIR) that is removed afterwards; nothing is
# checked out and the checkout is not touched. Files of the checkout that upstream ignores (build outputs
# such as the generated native header) are therefore not part of the extraction.
# When $UPSTREAM is not a git checkout it is taken to be such an export and read as it is.
#
# The build output of the extractor goes to $API_EXTRACT_ARTIFACTS when set (dotnet --artifacts-path),
# else to scripts/api-extract/bin and obj.
#
# The Rust tree and the upstream checkout are only read.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
upstream="${UPSTREAM:-$repo/../Avalonia}"
data="$repo/docs/porting/data"
json="$data/upstream-api.json"
extract="$repo/scripts/api-extract"
dotnet="${DOTNET:-}"
if [[ -z "$dotnet" ]]; then
    if command -v dotnet >/dev/null 2>&1; then dotnet=dotnet; else dotnet="$HOME/.dotnet/dotnet"; fi
fi
python="${PYTHON:-python3}"

force=0
check=0
pass=()
for arg in "$@"; do
    case "$arg" in
        --force) force=1 ;;
        --check) check=1; pass+=("$arg") ;;
        *) pass+=("$arg") ;;
    esac
done

recorded=""
recorded_xamlx=""
if [[ -f "$json" ]]; then
    recorded="$(sed -n 's/^ *"upstreamCommit": "\([0-9a-f]*\)".*/\1/p' "$json" | head -1)"
    recorded_xamlx="$(sed -n 's/^ *"external\/XamlX": "\([0-9a-f]*\)".*/\1/p' "$json" | head -1)"
fi
tracked="${TRACKED_COMMIT:-$recorded}"

stale=0
reason=""
if [[ $force -eq 1 ]]; then
    stale=1; reason="--force"
elif [[ ! -f "$json" ]]; then
    stale=1; reason="no upstream-api.json"
elif [[ "$extract/Program.cs" -nt "$json" || "$extract/projects.json" -nt "$json" || "$extract/ApiExtract.csproj" -nt "$json" ]]; then
    stale=1; reason="extractor or project list changed"
elif [[ -n "$tracked" && "$tracked" != "$recorded" ]]; then
    stale=1; reason="tracked commit is $tracked"
fi

if [[ $check -eq 1 && $force -eq 0 ]]; then
    # A check writes nothing and needs neither the upstream checkout nor the .NET SDK: it compares the
    # documents with what the committed extraction and the Rust tree give.
    if [[ ! -f "$json" ]]; then
        echo "error: $json is missing" >&2
        exit 2
    fi
    echo "api-extract: not run by --check" >&2
elif [[ $stale -eq 1 ]]; then
    if [[ ! -d "$upstream" ]]; then
        echo "error: upstream checkout not found at $upstream (set UPSTREAM=/path/to/Avalonia)" >&2
        exit 2
    fi
    source_dir="$upstream"
    commit_args=()
    if git -C "$upstream" rev-parse --git-dir >/dev/null 2>&1 && [[ "$(cd "$(git -C "$upstream" rev-parse --show-toplevel)" && pwd)" == "$(cd "$upstream" && pwd)" ]]; then
        if [[ -z "$tracked" ]]; then
            tracked="$(git -C "$upstream" rev-parse HEAD)"
        fi
        tracked="$(git -C "$upstream" rev-parse "$tracked^{commit}")"
        export_dir="$(mktemp -d "${PORT_STATUS_TMP:-${TMPDIR:-/tmp}}/ferroui-upstream-XXXXXX")"
        trap 'rm -rf "$export_dir"' EXIT
        echo "api-extract: exporting upstream $tracked to $export_dir" >&2
        git -C "$upstream" archive "$tracked" | tar -x -C "$export_dir"
        commit_args=(--commit "$tracked")
        # submodules: the commit the tracked tree names, read from the checkout of the submodule
        while read -r _mode _type sha path; do
            if git -C "$upstream/$path" cat-file -e "$sha^{commit}" 2>/dev/null; then
                mkdir -p "$export_dir/$path"
                git -C "$upstream/$path" archive "$sha" | tar -x -C "$export_dir/$path"
                if [[ "$path" == "external/XamlX" ]]; then commit_args+=(--xamlx-commit "$sha"); fi
            else
                echo "warning: submodule $path at $sha is not available in $upstream/$path, skipped" >&2
            fi
        done < <(git -C "$upstream" ls-tree -r "$tracked" | awk '$1 == "160000" { print $1, $2, $3, $4 }')
        source_dir="$export_dir"
    else
        # a plain export of the tracked commit
        if [[ -n "$tracked" ]]; then commit_args=(--commit "$tracked"); fi
        if [[ -n "$recorded_xamlx" && -z "${TRACKED_COMMIT:-}" ]]; then commit_args+=(--xamlx-commit "$recorded_xamlx"); fi
    fi
    echo "api-extract: running ($reason)" >&2
    if [[ -n "${API_EXTRACT_ARTIFACTS:-}" ]]; then
        "$dotnet" build "$extract/ApiExtract.csproj" -c Release -nologo -v quiet -clp:NoSummary --artifacts-path "$API_EXTRACT_ARTIFACTS" >&2
        dll="$API_EXTRACT_ARTIFACTS/bin/ApiExtract/release/api-extract.dll"
    else
        "$dotnet" build "$extract/ApiExtract.csproj" -c Release -nologo -v quiet -clp:NoSummary >&2
        dll="$extract/bin/Release/net10.0/api-extract.dll"
    fi
    "$dotnet" "$dll" --upstream "$source_dir" --config "$extract/projects.json" --out "$data" ${commit_args[@]+"${commit_args[@]}"}
else
    echo "api-extract: upstream-api.json is up to date" >&2
fi

"$python" "$here/port_status.py" --repo "$repo" ${pass[@]+"${pass[@]}"}
