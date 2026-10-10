#!/bin/sh
# Copies the sources of one commit to the folder a Windows virtual machine reads them from, for
# scripts/windows/vm-smoke.ps1 (docs/porting/win32-platform.md, section 10.4).
#
#   scripts/windows/vm-sync.sh [<commit>] [<base>]
#
#   <commit>  what to copy: a commit, a branch or a tag. Default: HEAD.
#   <base>    the directory on the shared volume the copies are made in. Default:
#             /Volumes/1TB-macOS/ferroui-vm-windows
#
# What is copied is the tree of the commit, not the working tree: changes that are not committed
# are not part of it, and neither is anything Git ignores.
#
# Every sync makes a NEW directory, `<base>/src-<the first twelve digits of the commit>`, and
# removes the directories of earlier syncs (`src-*`, and `src` of the first version of this
# script). A machine that was suspended while the host replaced files of a directory it had read
# served the old contents of those files afterwards (a run built sources of the commit before,
# with the mark of the new one): a path the machine has never seen has nothing cached. The price
# is that cargo in the machine sees every file as new, so a run after a sync builds everything.
#
# The copy is marked and listed. `.vm-sync-in-progress` exists in the directory from its first
# file to its last; `.vm-sync-manifest` lists every file with its size and its SHA-256;
# `.vm-sync-commit` is written last and names the commit. vm-smoke.ps1 refuses a tree with the
# first mark or without the last, compares every file with the manifest and refuses a tree that
# differs from it, and prints the commit.
#
# The script prints the directory to start vm-smoke.ps1 from. It starts nothing in the machine
# and does not touch the machine itself.
set -eu

commit=${1:-HEAD}
base=${2:-/Volumes/1TB-macOS/ferroui-vm-windows}

root=$(git rev-parse --show-toplevel)
hash=$(git -C "$root" rev-parse --verify "$commit^{commit}")
subject=$(git -C "$root" log -1 --format=%s "$hash" | cut -c 1-100)
name=src-$(printf %s "$hash" | cut -c 1-12)
destination=$base/$name

echo "vm-sync: commit $hash ($subject)"
echo "vm-sync: to $destination"

mkdir -p "$base"
# The directories of earlier syncs, and one of this commit (a sync of the same commit again is a
# new copy too, under a name that says so).
if [ -e "$destination" ]; then
    name=$name-$(date +%H%M%S)
    destination=$base/$name
    echo "vm-sync: the commit was synced before; this copy is $destination"
fi
for old in "$base"/src "$base"/src-*; do
    if [ -d "$old" ]; then
        rm -rf "$old"
        echo "vm-sync: removed $old"
    fi
done

mkdir "$destination"
date "+%Y-%m-%d %H:%M:%S $hash" > "$destination/.vm-sync-in-progress"
git -C "$root" archive --format=tar "$hash" | tar -x -C "$destination"

# The manifest: "<SHA-256>  <size>  <path>" for every file, the paths relative to the directory.
(
    cd "$destination"
    find . -type f ! -name '.vm-sync-*' | sed 's|^\./||' | LC_ALL=C sort | while IFS= read -r file; do
        sum=$(shasum -a 256 "$file" | cut -d ' ' -f 1)
        size=$(wc -c < "$file" | tr -d ' ')
        printf '%s  %s  %s\n' "$sum" "$size" "$file"
    done > .vm-sync-manifest
)
echo "$hash" > "$destination/.vm-sync-commit"
rm -f "$destination/.vm-sync-in-progress"

echo "vm-sync: done: $(wc -l < "$destination/.vm-sync-manifest" | tr -d ' ') file(s) listed in the manifest; the mark names $hash"
printf '%s\n' "vm-sync: start the script of the machine from $name/scripts/windows/vm-smoke.ps1 (under the drive letter of the share)"
