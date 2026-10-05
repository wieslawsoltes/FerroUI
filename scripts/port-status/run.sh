#!/usr/bin/env bash
# Regenerates the port tracking documents.
#
#   scripts/port-status/run.sh            # upstream checkout at ../Avalonia (or $UPSTREAM)
#   scripts/port-status/run.sh --force    # always re-run the upstream extractor
#   scripts/port-status/run.sh --check    # exit 1 when the generated documents are out of date
#
# Step 1 (only when stale): scripts/api-extract -> docs/porting/data/upstream-api.json
# Step 2: scripts/port-status/port_status.py -> docs/porting/TRACKING.md, docs/porting/tracking/*.md,
#         docs/porting/data/port-status.json
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

force=0
pass=()
for arg in "$@"; do
    case "$arg" in
        --force) force=1 ;;
        *) pass+=("$arg") ;;
    esac
done

stale=0
reason=""
if [[ $force -eq 1 ]]; then
    stale=1; reason="--force"
elif [[ ! -f "$json" ]]; then
    stale=1; reason="no upstream-api.json"
elif [[ "$extract/Program.cs" -nt "$json" || "$extract/projects.json" -nt "$json" || "$extract/ApiExtract.csproj" -nt "$json" ]]; then
    stale=1; reason="extractor or project list changed"
elif [[ -d "$upstream" ]]; then
    head="$(git -C "$upstream" rev-parse HEAD 2>/dev/null || echo unknown)"
    if ! grep -q "\"upstreamCommit\": \"$head\"" "$json"; then
        stale=1; reason="upstream is at $head"
    fi
fi

if [[ $stale -eq 1 ]]; then
    if [[ ! -d "$upstream" ]]; then
        echo "error: upstream checkout not found at $upstream (set UPSTREAM=/path/to/Avalonia)" >&2
        exit 2
    fi
    echo "api-extract: running ($reason)" >&2
    "$dotnet" build "$extract/ApiExtract.csproj" -c Release -nologo -v quiet -clp:NoSummary >&2
    "$dotnet" "$extract/bin/Release/net10.0/api-extract.dll" --upstream "$upstream" --config "$extract/projects.json" --out "$data"
else
    echo "api-extract: upstream-api.json is up to date" >&2
fi

exec python3 "$here/port_status.py" --repo "$repo" ${pass[@]+"${pass[@]}"}
