#!/bin/sh
# Copies the sources of one commit to the folder a Windows virtual machine reads them from, for
# scripts/windows/vm-smoke.ps1 (docs/porting/win32-platform.md, section 10.4).
#
#   scripts/windows/vm-sync.sh [<commit>] [<destination>]
#
#   <commit>       what to copy: a commit, a branch or a tag. Default: HEAD.
#   <destination>  the source directory the machine reads. Default:
#                  /Volumes/1TB-macOS/ferroui-vm-windows/src
#
# What is copied is the tree of the commit, not the working tree: changes that are not committed
# are not part of it, and neither is anything Git ignores. Files of an earlier copy that the
# commit does not have are removed.
#
# The copy is marked. `.vm-sync-in-progress` exists in the destination from the first changed
# file to the last; `.vm-sync-commit` is written when the copy is complete and names the commit.
# vm-smoke.ps1 refuses a tree with the first mark or without the second, and prints the commit:
# a run once built a tree that was half copied.
#
# The script starts nothing in the machine and does not touch the machine itself.
set -eu

commit=${1:-HEAD}
destination=${2:-/Volumes/1TB-macOS/ferroui-vm-windows/src}

root=$(git rev-parse --show-toplevel)
hash=$(git -C "$root" rev-parse --verify "$commit^{commit}")
subject=$(git -C "$root" log -1 --format=%s "$hash" | cut -c 1-100)

mkdir -p "$destination"
stage=$(mktemp -d "$(dirname "$destination")/.vm-sync-stage.XXXXXX")
trap 'rm -rf "$stage"' EXIT

echo "vm-sync: commit $hash ($subject)"
echo "vm-sync: to $destination"

# The tree of the commit, exported beside the destination (the same volume).
git -C "$root" archive --format=tar "$hash" | tar -x -C "$stage"

date "+%Y-%m-%d %H:%M:%S $hash" > "$destination/.vm-sync-in-progress"
rm -f "$destination/.vm-sync-commit"
# By content, and without the times of the export (every file of an export has the time of its
# commit): a file that changed gets the time of the copy, which is how cargo in the machine sees
# that it changed, and a file that did not change is not touched.
rsync -rlpD --checksum --delete \
    --exclude=/.vm-sync-in-progress --exclude=/.vm-sync-commit \
    "$stage/" "$destination/"
echo "$hash" > "$destination/.vm-sync-commit"
rm -f "$destination/.vm-sync-in-progress"

echo "vm-sync: done: $(find "$destination" -type f | wc -l | tr -d ' ') file(s); the mark names $hash"
