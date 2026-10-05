#!/bin/bash
# Builds a browser application and assembles its site.
#
#   scripts/build-browser.sh <application> [--debug] [--out <directory>]
#
# <application> is either an example of the browser crate (src/Browser/FerroUI.Browser/examples/
# <application>, with its host page in wwwroot/) or a binary package of the workspace with its host
# page in the wwwroot/ directory of the package (control-catalog-browser). The site is written to
# target/browser/<application> (or --out): the host page, the script modules of the platform
# (ferroui.js and storage.js, built from webapp/ with esbuild), the service worker (ferroui-sw.js, at
# the root of the site, which its scope and the save picker polyfill need) and the WebAssembly module
# with its script. Serve the directory with any static web server.
#
# The module is built with the `browser` profile of the workspace (optimised for size, see
# docs/porting/browser-platform.md, section 18), or with the `dev` profile with --debug.
#
# Needs: the Emscripten SDK activated in the shell (emsdk 6.0.10: `source emsdk_env.sh`), the Rust
# target wasm32-unknown-emscripten, the wasm-bindgen command-line tool of the version of the
# wasm-bindgen crate on PATH, node and npm. See docs/porting/browser-platform.md.
set -euo pipefail

APPLICATION=""
PROFILE="browser"
OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --debug) PROFILE="debug";;
    --out) shift; OUT="$1";;
    -*) echo "unknown option: $1" >&2; exit 2;;
    *) APPLICATION="$1";;
  esac
  shift
done
if [ -z "$APPLICATION" ]; then
  echo "usage: scripts/build-browser.sh <application> [--debug] [--out <directory>]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/src/Browser/FerroUI.Browser"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
OUT="${OUT:-$TARGET_DIR/browser/$APPLICATION}"

command -v node >/dev/null || { echo "node is not on PATH" >&2; exit 1; }
# An example of the browser crate, or else the binary of the workspace package of that name.
if [ -d "$CRATE/examples/$APPLICATION" ]; then
  KIND="example"
  WWWROOT="$CRATE/examples/$APPLICATION/wwwroot"
  CARGO_SELECTION=(-p ferroui-browser --example "$APPLICATION")
else
  KIND="bin"
  MANIFEST="$(cd "$ROOT" && cargo metadata --no-deps --format-version 1 --locked | node -e '
    let text = ""; process.stdin.on("data", (chunk) => text += chunk).on("end", () => {
      const found = JSON.parse(text).packages.find((p) => p.name === process.argv[1]
        && p.targets.some((t) => t.kind.includes("bin") && t.name === process.argv[1]));
      if (found) console.log(found.manifest_path);
    });' "$APPLICATION")"
  [ -n "$MANIFEST" ] || { echo "$APPLICATION is neither an example of the browser crate nor a binary package of the workspace" >&2; exit 2; }
  WWWROOT="$(dirname "$MANIFEST")/wwwroot"
  CARGO_SELECTION=(-p "$APPLICATION" --bin "$APPLICATION")
fi

command -v emcc >/dev/null || { echo "emcc is not on PATH: activate the Emscripten SDK first" >&2; exit 1; }
command -v em++ >/dev/null || { echo "em++ (the linker of the target) is not on PATH: activate the Emscripten SDK first" >&2; exit 1; }
command -v wasm-bindgen >/dev/null || { echo "wasm-bindgen is not on PATH" >&2; exit 1; }
[ -d "$WWWROOT" ] || { echo "no host page at $WWWROOT" >&2; exit 1; }

# The output directory is deleted before it is written: refuse anything that is not a directory of
# its own below the repository or the cargo target directory.
# The real path of a directory that may not exist yet (nothing is created).
abspath() {
  node -e 'const fs = require("fs"), path = require("path");
    let p = path.resolve(process.argv[1]), rest = [];
    while (!fs.existsSync(p) && path.dirname(p) !== p) { rest.unshift(path.basename(p)); p = path.dirname(p); }
    console.log(path.join(fs.realpathSync(p), ...rest));' "$1"
}
[ -n "$OUT" ] || { echo "the output directory is empty" >&2; exit 2; }
ROOT_REAL="$(cd "$ROOT" && pwd -P)"
TARGET_REAL="$(abspath "$TARGET_DIR")"
OUT_REAL="$(abspath "$OUT")"
case "$OUT_REAL" in
  /|"$ROOT_REAL"|"$TARGET_REAL"|"$ROOT_REAL/src"*|"$ROOT_REAL/.git"*)
    echo "refusing to use $OUT_REAL as the output directory" >&2; exit 2;;
esac
case "$OUT_REAL/" in
  "$ROOT_REAL"/*|"$TARGET_REAL"/*) ;;
  *) echo "the output directory $OUT_REAL is outside the repository and the target directory" >&2; exit 2;;
esac
OUT="$OUT_REAL"

# The version of the command-line tool has to be the version of the crate.
WANTED="$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' "$ROOT/Cargo.toml")"
FOUND="$(wasm-bindgen --version | awk '{print $2}')"
if [ "$WANTED" != "$FOUND" ]; then
  echo "wasm-bindgen $FOUND is on PATH but the workspace uses $WANTED" >&2
  exit 1
fi

# Skia's prebuilt binaries leave symbols to the linker of the application. The setting is added
# to flags that are already set, not put in their place.
REQUIRED_EMCC_FLAG="-s ERROR_ON_UNDEFINED_SYMBOLS=0"
case " ${EMCC_CFLAGS:-} " in
  *"ERROR_ON_UNDEFINED_SYMBOLS=0"*) ;;
  *) export EMCC_CFLAGS="${EMCC_CFLAGS:+$EMCC_CFLAGS }$REQUIRED_EMCC_FLAG";;
esac

echo "== script module"
(cd "$CRATE/webapp" && npm ci --no-audit --no-fund && npm run typecheck && npm run lint && npm run build)

echo "== WebAssembly module ($PROFILE)"
FLAGS=()
[ "$PROFILE" = "browser" ] && FLAGS+=(--profile browser)
(cd "$ROOT" && cargo build --locked --target wasm32-unknown-emscripten "${CARGO_SELECTION[@]}" "${FLAGS[@]}")

echo "== site"
BUILT="$TARGET_DIR/wasm32-unknown-emscripten/$PROFILE"
[ "$KIND" = "example" ] && BUILT="$BUILT/examples"
# The script of the module names the WebAssembly file it loads: the name of the target with `-`
# replaced by `_` for a binary, the name of the target for an example.
WASM="$(sed -n 's/.*locateFile("\([^"]*\.wasm\)".*/\1/p; s/.*new URL("\([^"]*\.wasm\)".*/\1/p' "$BUILT/$APPLICATION.js" | head -n 1)"
[ -n "$WASM" ] || WASM="$APPLICATION.wasm"
[ -f "$BUILT/$WASM" ] || { echo "the script of the module loads $WASM, which the build did not write" >&2; exit 1; }
rm -rf -- "$OUT"
mkdir -p "$OUT"
cp -R "$WWWROOT"/. "$OUT"/
# The main script module, and the storage bundle it imports on first use from the same directory.
cp "$CRATE/dist/ferroui.js" "$CRATE/dist/ferroui.js.map" "$CRATE/dist/storage.js" "$CRATE/dist/storage.js.map" "$OUT"/
# The service worker, registered by the application with `register_ferro_service_worker`. It is
# scoped to its own directory and found by the polyfill through the address of the document, so it
# has to sit at the root of the site, next to the host page.
cp "$CRATE/dist/ferroui-sw.js" "$CRATE/dist/ferroui-sw.js.map" "$OUT"/
cp "$BUILT/$APPLICATION.js" "$BUILT/$WASM" "$OUT"/
ls -la "$OUT"
echo "site written to $OUT"
