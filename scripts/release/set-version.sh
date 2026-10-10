#!/bin/sh
# set-version.sh <version>: the version of every crate of the workspace, in the one place it is
# stated (`[workspace.package]` of the root manifest) and in the pins of the crates of the workspace
# to one another (`version = "=<version>"` in `[workspace.dependencies]`), then verified with
# `cargo metadata`, which also brings Cargo.lock up to date. See docs/release.md, "Version scheme".
set -eu
if [ "$#" -ne 1 ]; then
  echo "usage: scripts/release/set-version.sh <version>   (for example 0.1.0-preview.2)" >&2
  exit 2
fi
exec python3 "$(dirname "$0")/crates.py" set-version "$1"
