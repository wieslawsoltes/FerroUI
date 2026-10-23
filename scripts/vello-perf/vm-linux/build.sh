#!/bin/bash
# In the Linux virtual machine: builds the desktop catalog with the Vello backend in release from a
# branch of origin, in a worktree of its own, in the background (log: /tmp/vbuild.log; wait with
# wait-build.sh). Targets a directory on the shared volume of the host.
#
#   build.sh <branch> [<checkout, default ~/GitHub/FerroUI>] [<target dir>]
set -u
branch=${1:?branch}; repo=${2:-$HOME/GitHub/FerroUI}
export CARGO_TARGET_DIR=${3:-/media/psf/1TB-macOS/ferroui-vm-linux}
source "$HOME/.cargo/env" 2>/dev/null
pkill -f "cargo build --release -p control-catalog-desktop"; sleep 2
cd "$repo" || exit 1
git fetch -q origin "+refs/heads/$branch:refs/remotes/origin/$branch"
wt="$repo-vello"
if [ ! -d "$wt" ]; then git worktree add -q --detach "$wt" "origin/$branch"; else git -C "$wt" checkout -q --detach "origin/$branch"; fi
cd "$wt" && git log --oneline -1
nohup cargo build --release -p control-catalog-desktop --features vello > /tmp/vbuild.log 2>&1 &
echo started
