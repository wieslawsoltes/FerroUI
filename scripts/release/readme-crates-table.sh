#!/bin/sh
# readme-crates-table.sh [--check]: regenerates the table of the published crates in README.md,
# between its two marker comments, from `cargo metadata`: one row a crate that is not
# `publish = false`, with its description and its badges, grouped by the
# `[package.metadata.release] group` of its manifest. With --check nothing is written and a stale
# table is an error (the continuous integration runs that). See docs/release.md.
set -eu
exec python3 "$(dirname "$0")/crates.py" readme "$@"
