#!/bin/bash
# Builds a browser application and assembles its site.
#
#   scripts/build-browser.sh <example> [--debug | --profile <name>] [--out <directory>]
#
# <example> is an example of the browser crate (src/Browser/FerroUI.Browser/examples/<example>,
# with its host page in wwwroot/). The site is written to target/browser/<example> (or --out):
# the host page, the script module of the platform (ferroui.js, built from webapp/ with esbuild)
# and the WebAssembly module with its script, and for the browser profile .gz and .br copies of
# them. Serve the directory with any static web server; the module must be served as
# application/wasm, which lets the browser compile it while it downloads.
# The module is built with the cargo profile `browser` (the release profile optimised for size, see
# docs/porting/browser-size.md); --debug builds the dev profile, --profile any other profile.
#
# Needs: the Emscripten SDK activated in the shell (emsdk 6.0.10: `source emsdk_env.sh`), the Rust
# target wasm32-unknown-emscripten, the wasm-bindgen command-line tool of the version of the
# wasm-bindgen crate on PATH, node and npm. See docs/porting/browser-platform.md.
set -euo pipefail

EXAMPLE=""
PROFILE="browser"
OUT=""
while [ $# -gt 0 ]; do
  case "$1" in
    --debug) PROFILE="dev";;
    --profile) shift; PROFILE="$1";;
    --out) shift; OUT="$1";;
    -*) echo "unknown option: $1" >&2; exit 2;;
    *) EXAMPLE="$1";;
  esac
  shift
done
if [ -z "$EXAMPLE" ]; then
  echo "usage: scripts/build-browser.sh <example> [--debug | --profile <name>] [--out <directory>]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/src/Browser/FerroUI.Browser"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
OUT="${OUT:-$TARGET_DIR/browser/$EXAMPLE}"
WWWROOT="$CRATE/examples/$EXAMPLE/wwwroot"

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

if [ "$PROFILE" = "browser" ]; then
  echo "== tools of the browser build"
  (cd "$ROOT/scripts/browser" && npm ci --no-audit --no-fund)
fi

echo "== script module"
(cd "$CRATE/webapp" && npm ci --no-audit --no-fund && npm run typecheck && npm run lint && npm run build)

echo "== WebAssembly module ($PROFILE)"
# rustc has emcc link at -Oz for opt-level "z". At -Os and -Oz Emscripten leaves out the cache of
# getWasmTableEntry, so that every call that can unwind (invoke_* in the script) looks the function
# up with wasmTable.get, which delays the first frame of themed_view by a third. The browser
# profile links at -O2 instead (the last -O flag wins); wasm-opt -Oz runs on the module afterwards.
LINK_ARGS=()
[ "$PROFILE" = "browser" ] && LINK_ARGS=(-- -Clink-arg=-O2)
(cd "$ROOT" && cargo rustc --target wasm32-unknown-emscripten -p ferroui-browser --example "$EXAMPLE" --profile "$PROFILE" "${LINK_ARGS[@]}")

echo "== site"
# The dev profile builds into debug/, every other profile into the directory of its name.
PROFILE_DIR="$PROFILE"
[ "$PROFILE" = "dev" ] && PROFILE_DIR="debug"
BUILT="$TARGET_DIR/wasm32-unknown-emscripten/$PROFILE_DIR/examples"
rm -rf -- "$OUT"
mkdir -p "$OUT"
cp -R "$WWWROOT"/. "$OUT"/
cp "$CRATE/dist/ferroui.js" "$CRATE/dist/ferroui.js.map" "$OUT"/
cp "$BUILT/$EXAMPLE.js" "$BUILT/$EXAMPLE.wasm" "$OUT"/

if [ "$PROFILE" = "browser" ]; then
  # emcc linked at -O2 (see above); wasm-opt of the pinned Binaryen (scripts/browser/package.json, the
  # version of Emscripten 6.0.10) optimises the module for size with the settings and the features
  # emcc uses for its own wasm-opt run. The JavaScript build of wasm-opt takes several minutes.
  echo "== wasm-opt -Oz"
  "$ROOT/scripts/browser/node_modules/.bin/wasm-opt" -Oz \
    --low-memory-unused --zero-filled-memory --pass-arg=directize-initial-contents-immutable \
    --mvp-features --enable-threads --enable-bulk-memory --enable-bulk-memory-opt \
    --enable-call-indirect-overlong --enable-multivalue --enable-mutable-globals \
    --enable-nontrapping-float-to-int --enable-reference-types --enable-sign-ext \
    "$OUT/$EXAMPLE.wasm" -o "$OUT/$EXAMPLE.wasm"

  # Precompressed copies (.gz, .br) next to the files, for servers that serve them with
  # Content-Encoding instead of compressing on every request.
  echo "== precompressed files"
  node "$ROOT/scripts/browser/compress.mjs" "$OUT"
fi
ls -la "$OUT"
echo "site written to $OUT"
