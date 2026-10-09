#!/bin/bash
# Builds the page of the HTML transport of designer support.
#
#   scripts/build-designer-webapp.sh [--check]
#
# The page is the web application of src/FerroUI.DesignerSupport/remote/html_transport/webapp: its
# sources (src/, TypeScript and the host page) are bundled with esbuild into build/, which is
# checked in and embedded in the crate (html_transport.rs), so that building the crate needs
# neither Node nor a bundler. Run this script after every change of the sources and commit what it
# writes:
#
#   build/index.html     the host page (src/index.html as it is)
#   build/index.js       the bundle of src/index.ts, minified
#   build/sources.sha1   the digest of the sources the two were built from; a test of the crate
#                        (the_built_page_is_built_from_its_sources) computes it again from the
#                        sources and fails when they changed without a build
#
# Before it builds, the script checks the types of the sources (tsc) and runs the tests of the
# input messages of the page (tests/, with the test runner of Node).
#
# --check builds into a temporary directory and compares the result with build/: it fails when
# the checked-in files are not what the sources build to.
#
# Needs node on PATH, and esbuild and tsc. The two are taken from the packages of the script module
# of the browser platform, which pins their versions (src/Browser/FerroUI.Browser/webapp: run
# `npm ci` there once), or from the directory FERROUI_WEBAPP_TOOLS names (a node_modules/.bin).
# The bundle depends on the version of esbuild: build with the pinned one.
set -euo pipefail

CHECK=""
while [ $# -gt 0 ]; do
  case "$1" in
    --check) CHECK="1";;
    *) echo "usage: scripts/build-designer-webapp.sh [--check]" >&2; exit 2;;
  esac
  shift
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEBAPP="$ROOT/src/FerroUI.DesignerSupport/remote/html_transport/webapp"
TOOLS="${FERROUI_WEBAPP_TOOLS:-$ROOT/src/Browser/FerroUI.Browser/webapp/node_modules/.bin}"

command -v node >/dev/null || { echo "node is not on PATH" >&2; exit 1; }
for tool in esbuild tsc; do
  [ -x "$TOOLS/$tool" ] || { echo "$tool is not in $TOOLS: run 'npm ci' in src/Browser/FerroUI.Browser/webapp or set FERROUI_WEBAPP_TOOLS" >&2; exit 1; }
done
if command -v shasum >/dev/null; then
  SHA1=(shasum -a 1)
elif command -v sha1sum >/dev/null; then
  SHA1=(sha1sum)
else
  echo "neither shasum nor sha1sum is on PATH" >&2; exit 1
fi

# The digest of the sources: for tsconfig.json and then every file of src/ in the order of the
# bytes of its path, the path, a line feed and the bytes of the file. Files whose name starts with
# a dot are not sources. The test of the crate computes the same.
sources_hash() {
  (cd "$WEBAPP" && { echo tsconfig.json; find src -type f ! -name '.*' | LC_ALL=C sort; } | while IFS= read -r file; do
    printf '%s\n' "$file"
    cat "$file"
  done | "${SHA1[@]}" | cut -d' ' -f1)
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "== types"
"$TOOLS/tsc" --noEmit -p "$WEBAPP/tsconfig.json"

echo "== tests"
"$TOOLS/esbuild" "$WEBAPP/tests/Models/InputEventTests.ts" --bundle --format=esm --platform=node \
  --log-level=warning --outfile="$WORK/tests/InputEventTests.mjs"
node --test "$WORK/tests/InputEventTests.mjs"

echo "== page"
OUT="$WEBAPP/build"
[ -n "$CHECK" ] && OUT="$WORK/build"
mkdir -p "$OUT"
(cd "$WEBAPP" && "$TOOLS/esbuild" src/index.ts --bundle --tree-shaking=true --minify --format=esm --target=es6 \
  --platform=browser --log-level=warning --tsconfig=tsconfig.json --outfile="$OUT/index.js")
cp "$WEBAPP/src/index.html" "$OUT/index.html"
sources_hash > "$OUT/sources.sha1"

if [ -n "$CHECK" ]; then
  diff -r "$WEBAPP/build" "$OUT" || { echo "build/ is not what the sources build to: run scripts/build-designer-webapp.sh" >&2; exit 1; }
  echo "build/ is up to date"
else
  echo "wrote $OUT"
fi
