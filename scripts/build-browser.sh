#!/bin/bash
# Builds a browser application and assembles its site.
#
#   scripts/build-browser.sh <application> [--debug] [--out <directory>] [--threads]
#
# <application> is either an example of the browser crate (src/Browser/FerroUI.Browser/examples/
# <application>, with its host page in wwwroot/) or a binary package of the workspace with its host
# page in the wwwroot/ directory of the package (control-catalog-browser). The site is written to
# target/browser/<application> (or --out): the host page, the script modules of the platform
# (ferroui.js and storage.js, built from webapp/ with esbuild), the service worker (ferroui-sw.js, at
# the root of the site, which its scope and the save picker polyfill need) and the WebAssembly module
# with its script, plus the files that build scripts of the application leave for the site in
# `$OUT_DIR/browser-site/` (asset files the host page downloads). Serve the directory with any
# static web server.
#
# The module is built with the `browser` profile of the workspace (optimised for size, see
# docs/porting/browser-platform.md, section 18), or with the `dev` profile with --debug.
#
# Needs: the Emscripten SDK activated in the shell (emsdk 6.0.10: `source emsdk_env.sh`), the Rust
# target wasm32-unknown-emscripten, the wasm-bindgen command-line tool of the version of the
# wasm-bindgen crate on PATH, node and npm. See docs/porting/browser-platform.md.
#
# --threads (opt-in, docs/porting/browser-platform.md, "Threads (opt-in)") builds a module that can
# spawn threads: the nightly toolchain that `scripts/browser/setup.sh --threads` installs, a standard
# library rebuilt with atomics (-Zbuild-std) and the pthread options of Emscripten. The build goes to
# its own target directory (target/threads) and the site to target/browser-threads/<application>, so
# that neither replaces the output of a build without the option. The site also gets one file of
# scripts/browser/threads/, the check for cross-origin isolation (the service worker that provides
# the isolation on a host that cannot set headers is the one every site has, ferroui-sw.js); the
# other two are linked into the script of the module, for the web workers that run its threads. The
# memory of such a module does not grow: it has the size below from the start. Variables of the mode:
#   FERROUI_BROWSER_THREAD_POOL_SIZE   web workers created before the application starts (default 2);
#                                      a thread beyond the pool cannot start until the main thread
#                                      returns to the browser
#   FERROUI_BROWSER_THREAD_MEMORY_MB   the size of the memory of the module in megabytes (default
#                                      512): all the application can ever allocate
#   FERROUI_BROWSER_NIGHTLY            the nightly toolchain (default: the pin of setup.sh)
set -euo pipefail

APPLICATION=""
PROFILE="browser"
OUT=""
THREADS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --debug) PROFILE="debug";;
    --out) shift; OUT="$1";;
    --threads) THREADS="1";;
    -*) echo "unknown option: $1" >&2; exit 2;;
    *) APPLICATION="$1";;
  esac
  shift
done
if [ -z "$APPLICATION" ]; then
  echo "usage: scripts/build-browser.sh <application> [--debug] [--out <directory>] [--threads]" >&2
  exit 2
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/src/Browser/FerroUI.Browser"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
# Where cargo builds. The threaded mode has its own directory: its standard library and its flags
# differ, and sharing the directory would rebuild everything at each change of mode.
BUILD_DIR="$TARGET_DIR"
if [ -n "$THREADS" ]; then
  BUILD_DIR="$TARGET_DIR/threads"
  OUT="${OUT:-$TARGET_DIR/browser-threads/$APPLICATION}"
  THREAD_POOL_SIZE="${FERROUI_BROWSER_THREAD_POOL_SIZE:-2}"
  case "$THREAD_POOL_SIZE" in
    ''|*[!0-9]*) echo "FERROUI_BROWSER_THREAD_POOL_SIZE is not a number: $THREAD_POOL_SIZE" >&2; exit 2;;
  esac
  # Not measured yet: the default is a provisional size, to be replaced by the peak of the catalog
  # with a margin (docs/porting/browser-render-worker.md, "B2.2"). The stack alone is 8 MB.
  THREAD_MEMORY_MB="${FERROUI_BROWSER_THREAD_MEMORY_MB:-512}"
  case "$THREAD_MEMORY_MB" in
    ''|*[!0-9]*) echo "FERROUI_BROWSER_THREAD_MEMORY_MB is not a number: $THREAD_MEMORY_MB" >&2; exit 2;;
  esac
  if [ "$THREAD_MEMORY_MB" -lt 16 ] || [ "$THREAD_MEMORY_MB" -gt 2048 ]; then
    echo "FERROUI_BROWSER_THREAD_MEMORY_MB is not between 16 and 2048: $THREAD_MEMORY_MB" >&2; exit 2
  fi
  # The pin lives in setup.sh, next to the pin of the stable toolchain.
  NIGHTLY="${FERROUI_BROWSER_NIGHTLY:-$(sed -n 's/^RUST_NIGHTLY="\(.*\)"$/\1/p' "$ROOT/scripts/browser/setup.sh")}"
  [ -n "$NIGHTLY" ] || { echo "cannot read the nightly toolchain from scripts/browser/setup.sh" >&2; exit 1; }
  # Replaces the stable pin that env.sh exports, for every cargo and rustc call below.
  export RUSTUP_TOOLCHAIN="$NIGHTLY"
  cargo --version >/dev/null 2>&1 || { echo "the toolchain $NIGHTLY is not installed: run scripts/browser/setup.sh --threads" >&2; exit 1; }
fi
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
# Every object of a threaded link has to be compiled with threads: this reaches the C and C++ code
# that build scripts compile for the module (HarfBuzz, the setjmp bridge of ferroui-skia).
if [ -n "$THREADS" ]; then
  case " $EMCC_CFLAGS " in
    *" -pthread "*) ;;
    *) export EMCC_CFLAGS="$EMCC_CFLAGS -pthread";;
  esac
  # Build scripts that compile C or C++ without going through emcc's flags (the bindings of Skia)
  # read the compiler flags of the target: without atomics their objects cannot be linked into a
  # module with shared memory.
  export CFLAGS_wasm32_unknown_emscripten="${CFLAGS_wasm32_unknown_emscripten:+$CFLAGS_wasm32_unknown_emscripten }-pthread"
  export CXXFLAGS_wasm32_unknown_emscripten="${CXXFLAGS_wasm32_unknown_emscripten:+$CXXFLAGS_wasm32_unknown_emscripten }-pthread"
fi

echo "== script module"
(cd "$CRATE/webapp" && npm ci --no-audit --no-fund && npm run typecheck && npm run lint && npm run build)

echo "== WebAssembly module ($PROFILE${THREADS:+, threads})"
FLAGS=()
[ "$PROFILE" = "browser" ] && FLAGS+=(--profile browser)
if [ -n "$THREADS" ]; then
  # The standard library that ships with a toolchain is built without atomics and cannot be linked
  # into a module with shared memory: it is rebuilt from rust-src with the flags of the build.
  FLAGS+=(--target-dir "$BUILD_DIR" -Zbuild-std=std,panic_unwind)
  # Added to the flags of the target in .cargo/config.toml (an array given with --config is appended
  # to the one of the file; RUSTFLAGS in the environment would replace it). The environment of the
  # file is "web" alone, and a thread is a web worker that loads the script of the module.
  # --no-check-features: the prebuilt archive of the Skia bindings (libskia-bindings.a, four objects:
  # bindings, gl, gpu, ganesh) is compiled without atomics, unlike libskia.a beside it, and the
  # linker refuses such an object in a module with shared memory. The check is switched off for the
  # link. What that costs: in those four objects a `thread_local` is a plain global and the guard of
  # a local static is not atomic. They are thin forwarding functions; the proper fix is to compile
  # them here with -pthread (their sources are in the skia-bindings crate) or binaries built so.
  # PThread joins the exported runtime methods of the file (the setting given later replaces the
  # earlier one): the script side finds the web worker of a thread in its table when it transfers
  # a canvas to the thread that renders (WebRenderTargetRegistry.create).
  # The memory is fixed (-sALLOW_MEMORY_GROWTH=0 after the 1 of the file, and the whole size as
  # -sINITIAL_MEMORY). A shared memory that grows leaves every other thread with views that end
  # where the memory ended before. Emscripten's own script and the port's (FerroExports.heapU8)
  # look for the new buffer at each use, but the wasm-bindgen glue keeps a DataView that it only
  # replaces when Emscripten replaces its views, and writes through it without asking: after a
  # growth by another thread such a write fails. That cannot be repaired from outside the glue, so
  # the memory does not grow until the glue is right (docs/porting/browser-render-worker.md,
  # section 5 and "B2.2"). The build without threads keeps its growing memory.
  # The two scripts of scripts/browser/threads/ that are linked into the script of the module make
  # a web worker that runs a thread attach its own copy of ferroui.js to its own module, and start
  # the wasm-bindgen glue there (docs/porting/browser-render-worker.md, "B2.1"). The paths go into
  # a TOML string as they are: a repository path with a quote or a backslash in it would break it.
  THREADS_DIR="$ROOT/scripts/browser/threads"
  FLAGS+=(--config "target.wasm32-unknown-emscripten.rustflags=[\"-Ctarget-feature=+atomics,+bulk-memory\", \"-Clink-arg=-pthread\", \"-Clink-arg=-Wl,--no-check-features\", \"-Clink-arg=-sPTHREAD_POOL_SIZE=$THREAD_POOL_SIZE\", \"-Clink-arg=-sENVIRONMENT=web,worker\", \"-Clink-arg=-sEXPORTED_RUNTIME_METHODS=GL,HEAPU8,wasmMemory,PThread\", \"-Clink-arg=-sALLOW_MEMORY_GROWTH=0\", \"-Clink-arg=-sINITIAL_MEMORY=${THREAD_MEMORY_MB}MB\", \"-Clink-arg=--extern-pre-js=$THREADS_DIR/ferroui-worker-import.js\", \"-Clink-arg=--post-js=$THREADS_DIR/ferroui-worker-attach.js\"]")
fi
# The messages of the build name the output directories of the build scripts.
MESSAGES="$(mktemp)"
trap 'rm -f -- "$MESSAGES"' EXIT
(cd "$ROOT" && cargo build --locked --target wasm32-unknown-emscripten "${CARGO_SELECTION[@]}" "${FLAGS[@]}" \
  --message-format=json-render-diagnostics > "$MESSAGES")

echo "== site"
BUILT="$BUILD_DIR/wasm32-unknown-emscripten/$PROFILE"
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
# The service worker, registered by the application with `register_ferro_service_worker` and, in a
# threaded site on a host that does not send the headers of cross-origin isolation, by the check
# below (as `ferroui-sw.js?coi=1`, which makes it add them). It is scoped to its own directory and
# found by the polyfill through the address of the document, so it has to sit at the root of the
# site, next to the host page.
cp "$CRATE/dist/ferroui-sw.js" "$CRATE/dist/ferroui-sw.js.map" "$OUT"/
cp "$BUILT/$APPLICATION.js" "$BUILT/$WASM" "$OUT"/
# The threaded mode: the check for cross-origin isolation, which a host page written for threads
# imports before it creates the module.
if [ -n "$THREADS" ]; then
  cp "$ROOT/scripts/browser/threads/ferroui-threads.js" "$OUT"/
  # The worker of a thread loads the script of the module by the name the linker gave it, which
  # is the name of the module file: for an application with a hyphen in its name that is not the
  # name Cargo gives the script (control_catalog_browser.js against control-catalog-browser.js).
  # Without the file every worker fails to load and the module never finishes starting. The
  # script under the worker's name is the script of the module itself, imported.
  WORKER_SCRIPT="$(sed -n 's/.*new Worker(new URL("\([^"]*\.js\)",import\.meta\.url).*/\1/p' "$BUILT/$APPLICATION.js" | head -n 1)"
  if [ -n "$WORKER_SCRIPT" ] && [ "$WORKER_SCRIPT" != "$APPLICATION.js" ]; then
    printf 'import "./%s";\n' "$APPLICATION.js" > "$OUT/$WORKER_SCRIPT"
  fi
fi
# Files the build scripts of the application wrote for the site.
node -e 'const fs = require("fs"), path = require("path");
  const dirs = new Set();
  for (const line of fs.readFileSync(process.argv[1], "utf8").split("\n")) {
    if (!line.startsWith("{")) continue;
    const message = JSON.parse(line);
    if (message.reason === "build-script-executed" && message.out_dir) dirs.add(path.join(message.out_dir, "browser-site"));
  }
  for (const dir of dirs) if (fs.existsSync(dir)) console.log(dir);' "$MESSAGES" | while read -r SITE_FILES; do
  cp -R "$SITE_FILES"/. "$OUT"/
done
ls -la "$OUT"
echo "site written to $OUT"
