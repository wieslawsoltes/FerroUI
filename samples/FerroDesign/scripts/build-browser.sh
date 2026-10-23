#!/usr/bin/env bash
# Build the FerroUI/Vello application and assemble a self-contained static distribution.
# Run setup.sh once, then source its env.sh before invoking this script.
set -euo pipefail

FERRODESIGN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FERROUI_ROOT="$(cd "$FERRODESIGN_ROOT/../.." && pwd)"
FERRODESIGN_BROWSER="$FERROUI_ROOT/src/Browser/FerroUI.Browser"
FERRODESIGN_PROFILE="browser"
FERRODESIGN_OUT="$FERRODESIGN_ROOT/dist"
FERRODESIGN_SKIP_WEBAPP=""

while [ $# -gt 0 ]; do
    case "$1" in
        --debug) FERRODESIGN_PROFILE="debug" ;;
        --out) shift; FERRODESIGN_OUT="${1:?--out requires a directory}" ;;
        --skip-webapp) FERRODESIGN_SKIP_WEBAPP="1" ;;
        -h|--help)
            printf '%s\n' 'Usage: scripts/build-browser.sh [--debug] [--out directory] [--skip-webapp]'
            exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done

for FERRODESIGN_COMMAND in cargo emcc wasm-bindgen node npm; do
    command -v "$FERRODESIGN_COMMAND" >/dev/null || {
        printf '%s is not on PATH. Run scripts/setup.sh, then source its env.sh.\n' "$FERRODESIGN_COMMAND" >&2
        exit 1
    }
done
if [ "$(wasm-bindgen --version)" != "wasm-bindgen 0.2.129" ]; then
    printf '%s\n' 'This pinned FerroUI revision requires wasm-bindgen-cli 0.2.129.' >&2
    exit 1
fi

# A newly exported source tree can create its first lockfile; subsequent builds are locked.
if [ ! -f "$FERRODESIGN_ROOT/Cargo.lock" ]; then
    (cd "$FERRODESIGN_ROOT" && cargo generate-lockfile)
fi

# Check the active target graph, not optional dependencies that merely appear in Cargo.lock.
FERRODESIGN_GRAPH="$(cd "$FERRODESIGN_ROOT" && cargo tree --locked --target wasm32-unknown-emscripten -p ferrodesign-app -e normal,build)"
if [[ "$FERRODESIGN_GRAPH" =~ ferroui-skia|skia-safe|skia-bindings ]]; then
    printf '%s\n' 'Refusing to build: a Skia dependency entered the application dependency graph.' >&2
    exit 1
fi
if [[ ! "$FERRODESIGN_GRAPH" =~ ferroui-vello ]]; then
    printf '%s\n' 'Refusing to build: the application dependency graph does not contain the FerroUI Vello backend.' >&2
    exit 1
fi

if [ -z "$FERRODESIGN_SKIP_WEBAPP" ]; then
    (
        cd "$FERRODESIGN_BROWSER/webapp"
        npm ci --no-audit --no-fund
        npm run typecheck
        npm run test:input
        npm run build
    )
fi

FERRODESIGN_TARGET="${CARGO_TARGET_DIR:-$FERRODESIGN_ROOT/target}"
mkdir -p "$FERRODESIGN_TARGET"
FERRODESIGN_TARGET="$(cd "$FERRODESIGN_TARGET" && pwd)"
FERRODESIGN_MESSAGES="$(mktemp)"
trap 'rm -f -- "$FERRODESIGN_MESSAGES"' EXIT
FERRODESIGN_FLAGS=()
if [ "$FERRODESIGN_PROFILE" = "browser" ]; then FERRODESIGN_FLAGS+=(--profile browser); fi

# The repository's .cargo/config.toml supplies the Emscripten/wasm-bindgen ABI options.
# No undefined-symbol suppression or Skia compatibility flags are used. Serialization keeps
# the framework's generated controls and metadata within ordinary CI memory limits.
(
    cd "$FERRODESIGN_ROOT"
    # Cargo's job limit does not bound the post-link worker pools. Keep bindgen's
    # Rayon pool and Binaryen within the same budget unless explicitly overridden.
    export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-${FERRODESIGN_BUILD_JOBS:-1}}"
    export BINARYEN_CORES="${BINARYEN_CORES:-${FERRODESIGN_BUILD_JOBS:-1}}"
    cargo build --locked --target wasm32-unknown-emscripten --target-dir "$FERRODESIGN_TARGET" \
        -p ferrodesign-app --bin ferrodesign-app --jobs "${FERRODESIGN_BUILD_JOBS:-1}" \
        "${FERRODESIGN_FLAGS[@]}" --message-format=json-render-diagnostics > "$FERRODESIGN_MESSAGES"
)

FERRODESIGN_BUILT="$FERRODESIGN_TARGET/wasm32-unknown-emscripten/$FERRODESIGN_PROFILE"
FERRODESIGN_SCRIPT="$FERRODESIGN_BUILT/ferrodesign-app.js"
FERRODESIGN_WASM="$(sed -n 's/.*locateFile("\([^"]*\.wasm\)".*/\1/p; s/.*new URL("\([^"]*\.wasm\)".*/\1/p' "$FERRODESIGN_SCRIPT" | head -n 1)"
FERRODESIGN_WASM="${FERRODESIGN_WASM:-ferrodesign_app.wasm}"
test -f "$FERRODESIGN_BUILT/$FERRODESIGN_WASM"
mkdir -p "$FERRODESIGN_OUT"
cp -R "$FERRODESIGN_ROOT/web/." "$FERRODESIGN_OUT/"
cp "$FERRODESIGN_SCRIPT" "$FERRODESIGN_BUILT/$FERRODESIGN_WASM" "$FERRODESIGN_OUT/"
# wasm-bindgen emits inline_js modules beside the linker output. Cargo can copy the
# application module from deps/ to the profile root without copying its snippets.
# Preserve both possible output locations so every generated static import ships.
for FERRODESIGN_SNIPPETS in "$FERRODESIGN_BUILT/deps/snippets" "$FERRODESIGN_BUILT/snippets"; do
    if [ -d "$FERRODESIGN_SNIPPETS" ]; then
        mkdir -p "$FERRODESIGN_OUT/snippets"
        cp -R "$FERRODESIGN_SNIPPETS/." "$FERRODESIGN_OUT/snippets/"
    fi
done
for FERRODESIGN_FILE in ferroui.js storage.js ferroui-sw.js; do
    cp "$FERRODESIGN_BROWSER/dist/$FERRODESIGN_FILE" "$FERRODESIGN_OUT/"
    if [ -f "$FERRODESIGN_BROWSER/dist/$FERRODESIGN_FILE.map" ]; then
        cp "$FERRODESIGN_BROWSER/dist/$FERRODESIGN_FILE.map" "$FERRODESIGN_OUT/"
    fi
done
cp "$FERROUI_ROOT/scripts/browser/threads/ferroui-loader.js" "$FERRODESIGN_OUT/"

node --experimental-vm-modules - "$FERRODESIGN_MESSAGES" "$FERRODESIGN_OUT" <<'NODE'
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { fileURLToPath, pathToFileURL } = require("node:url");
for (const line of fs.readFileSync(process.argv[2], "utf8").split("\n")) {
    if (!line.startsWith("{")) continue;
    const message = JSON.parse(line);
    if (message.reason !== "build-script-executed" || !message.out_dir) continue;
    const directory = path.join(message.out_dir, "browser-site");
    if (fs.existsSync(directory)) fs.cpSync(directory, process.argv[3], { recursive: true });
}

// Parse the static module graph without executing browser code. A missing inline
// dependency otherwise appears in the browser as a failure to import the main JS.
const output = path.resolve(process.argv[3]);
const pending = ["main.js", "ferrodesign-app.js"].map(name => path.join(output, name));
const checked = new Set();
while (pending.length) {
    const file = pending.pop();
    if (checked.has(file)) continue;
    if (!fs.existsSync(file) || !fs.statSync(file).isFile()) {
        throw new Error(`Browser distribution is missing a required module: ${path.relative(output, file)}`);
    }
    checked.add(file);
    const identifier = pathToFileURL(file).href;
    const module = new vm.SourceTextModule(fs.readFileSync(file, "utf8"), { identifier });
    for (const specifier of module.dependencySpecifiers) {
        if (!specifier.startsWith("./") && !specifier.startsWith("../")) {
            throw new Error(`Browser module ${path.relative(output, file)} has a nonlocal import: ${specifier}`);
        }
        const dependency = fileURLToPath(new URL(specifier, identifier));
        const relative = path.relative(output, dependency);
        if (relative === ".." || relative.startsWith(".." + path.sep) || path.isAbsolute(relative)) {
            throw new Error(`Browser module import leaves the distribution: ${specifier}`);
        }
        pending.push(dependency);
    }
}
console.log(`Verified ${checked.size} required JavaScript modules in the browser distribution.`);
NODE

printf '%s\n' "$FERRODESIGN_GRAPH" > "$FERRODESIGN_OUT/dependencies.txt"
cp "$FERRODESIGN_ROOT/Cargo.lock" "$FERRODESIGN_OUT/Cargo.lock"
printf 'Browser distribution: %s\nRenderer: FerroUI Vello (WebGL2 hybrid, CPU fallback)\n' "$FERRODESIGN_OUT"
printf 'Serve with: python3 -m http.server 8080 --directory "%s"\n' "$FERRODESIGN_OUT"
