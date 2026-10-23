#!/usr/bin/env bash
# Install the pinned browser toolchain without changing shell profiles or existing Rust installs.
# Usage: samples/FerroDesign/scripts/setup.sh [tool directory]
set -euo pipefail

FERRODESIGN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FERRODESIGN_TOOLS="${1:-$FERRODESIGN_ROOT/.tools}"
mkdir -p "$FERRODESIGN_TOOLS"
FERRODESIGN_TOOLS="$(cd "$FERRODESIGN_TOOLS" && pwd)"
FERRODESIGN_RUST_VERSION="1.99.0"
FERRODESIGN_EMSDK_VERSION="6.0.10"
FERRODESIGN_BINDGEN_VERSION="0.2.129"

export CARGO_HOME="$FERRODESIGN_TOOLS/cargo"
export RUSTUP_HOME="$FERRODESIGN_TOOLS/rustup"
export PATH="$CARGO_HOME/bin:$FERRODESIGN_TOOLS/bin:$PATH"
mkdir -p "$FERRODESIGN_TOOLS/bin"

if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
    curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
        https://sh.rustup.rs -o "$FERRODESIGN_TOOLS/rustup-init.sh"
    sh "$FERRODESIGN_TOOLS/rustup-init.sh" -y --no-modify-path --profile minimal \
        --default-toolchain "$FERRODESIGN_RUST_VERSION"
fi
rustup toolchain install "$FERRODESIGN_RUST_VERSION" --profile minimal --target wasm32-unknown-emscripten
export RUSTUP_TOOLCHAIN="$FERRODESIGN_RUST_VERSION"

if [ ! -x "$FERRODESIGN_TOOLS/emsdk/emsdk" ]; then
    git clone --depth 1 https://github.com/emscripten-core/emsdk.git "$FERRODESIGN_TOOLS/emsdk"
fi
"$FERRODESIGN_TOOLS/emsdk/emsdk" install "$FERRODESIGN_EMSDK_VERSION"
"$FERRODESIGN_TOOLS/emsdk/emsdk" activate "$FERRODESIGN_EMSDK_VERSION"

if [ "$("$FERRODESIGN_TOOLS/bin/wasm-bindgen" --version 2>/dev/null || true)" != "wasm-bindgen $FERRODESIGN_BINDGEN_VERSION" ]; then
    FERRODESIGN_BINDGEN_TRIPLE=""
    FERRODESIGN_BINDGEN_SHA256=""
    # Official wasm-bindgen 0.2.129 release asset SHA-256 digests. Keep these
    # coupled to the version above; verify before extracting or running code.
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64)
            FERRODESIGN_BINDGEN_TRIPLE="x86_64-unknown-linux-musl"
            FERRODESIGN_BINDGEN_SHA256="82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e" ;;
        Linux-aarch64|Linux-arm64)
            FERRODESIGN_BINDGEN_TRIPLE="aarch64-unknown-linux-musl"
            FERRODESIGN_BINDGEN_SHA256="2ed4351c35dd9440308bbb02767d47ea278efe851a52465300f3c94f5b6c2a87" ;;
        Darwin-arm64|Darwin-aarch64)
            FERRODESIGN_BINDGEN_TRIPLE="aarch64-apple-darwin"
            FERRODESIGN_BINDGEN_SHA256="81d4a23d56b3c3eb8187658329116d50e0b228a93b343825fb71f70179051cd1" ;;
        Darwin-x86_64)
            FERRODESIGN_BINDGEN_TRIPLE="x86_64-apple-darwin"
            FERRODESIGN_BINDGEN_SHA256="7e028879a68ec53dae14048b55cc5215f6df75b17a9c1426fac8fa086d3202fd" ;;
    esac
    if [ -n "$FERRODESIGN_BINDGEN_TRIPLE" ]; then
        FERRODESIGN_BINDGEN_TEMP="$(mktemp -d "$FERRODESIGN_TOOLS/wasm-bindgen.XXXXXX")"
        trap 'rm -rf "$FERRODESIGN_BINDGEN_TEMP"' EXIT
        FERRODESIGN_BINDGEN_RELEASE="wasm-bindgen-$FERRODESIGN_BINDGEN_VERSION-$FERRODESIGN_BINDGEN_TRIPLE"
        FERRODESIGN_BINDGEN_ARCHIVE="$FERRODESIGN_BINDGEN_TEMP/$FERRODESIGN_BINDGEN_RELEASE.tar.gz"
        if curl --fail --location --retry 3 --proto '=https' --proto-redir '=https' --tlsv1.2 \
            --connect-timeout 15 --max-time 180 \
            "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$FERRODESIGN_BINDGEN_VERSION/$FERRODESIGN_BINDGEN_RELEASE.tar.gz" \
            -o "$FERRODESIGN_BINDGEN_ARCHIVE"; then
            if command -v sha256sum >/dev/null 2>&1; then
                FERRODESIGN_BINDGEN_DIGEST="$(sha256sum < "$FERRODESIGN_BINDGEN_ARCHIVE")"
            elif command -v shasum >/dev/null 2>&1; then
                FERRODESIGN_BINDGEN_DIGEST="$(shasum -a 256 < "$FERRODESIGN_BINDGEN_ARCHIVE")"
            else
                printf '%s\n' 'Cannot verify wasm-bindgen: sha256sum or shasum is required.' >&2
                exit 1
            fi
            if [ "${FERRODESIGN_BINDGEN_DIGEST%% *}" != "$FERRODESIGN_BINDGEN_SHA256" ]; then
                printf 'SHA-256 mismatch for %s; refusing to extract or execute it.\n' "$FERRODESIGN_BINDGEN_RELEASE.tar.gz" >&2
                exit 1
            fi
            tar -xzf "$FERRODESIGN_BINDGEN_ARCHIVE" -C "$FERRODESIGN_BINDGEN_TEMP"
            FERRODESIGN_BINDGEN_EXTRACTED="$FERRODESIGN_BINDGEN_TEMP/$FERRODESIGN_BINDGEN_RELEASE"
            if [ "$("$FERRODESIGN_BINDGEN_EXTRACTED/wasm-bindgen" --version 2>/dev/null || true)" != "wasm-bindgen $FERRODESIGN_BINDGEN_VERSION" ]; then
                printf '%s\n' 'Verified wasm-bindgen release does not report the required version.' >&2
                exit 1
            fi
            for FERRODESIGN_BINDGEN_BINARY in wasm-bindgen wasm-bindgen-test-runner wasm2es6js; do
                if [ -f "$FERRODESIGN_BINDGEN_EXTRACTED/$FERRODESIGN_BINDGEN_BINARY" ]; then
                    install -m 755 "$FERRODESIGN_BINDGEN_EXTRACTED/$FERRODESIGN_BINDGEN_BINARY" "$FERRODESIGN_TOOLS/bin/$FERRODESIGN_BINDGEN_BINARY"
                fi
            done
        else
            printf '%s\n' 'wasm-bindgen release download unavailable; using locked Cargo installation.' >&2
        fi
        rm -rf "$FERRODESIGN_BINDGEN_TEMP"
        trap - EXIT
    fi
    if [ "$("$FERRODESIGN_TOOLS/bin/wasm-bindgen" --version 2>/dev/null || true)" != "wasm-bindgen $FERRODESIGN_BINDGEN_VERSION" ]; then
        CARGO_TARGET_DIR="$FERRODESIGN_TOOLS/target-install" cargo install wasm-bindgen-cli \
            --version "$FERRODESIGN_BINDGEN_VERSION" --locked --root "$FERRODESIGN_TOOLS" --force --jobs 1
    fi
fi

cat > "$FERRODESIGN_TOOLS/env.sh" <<ENV
# Generated by FerroDesign/scripts/setup.sh.
export CARGO_HOME="$FERRODESIGN_TOOLS/cargo"
export RUSTUP_HOME="$FERRODESIGN_TOOLS/rustup"
export RUSTUP_TOOLCHAIN="$FERRODESIGN_RUST_VERSION"
export PATH="$FERRODESIGN_TOOLS/bin:$FERRODESIGN_TOOLS/cargo/bin:\$PATH"
source "$FERRODESIGN_TOOLS/emsdk/emsdk_env.sh" >/dev/null 2>&1
ENV

source "$FERRODESIGN_TOOLS/env.sh"
rustc --version
emcc --version | head -n 1
wasm-bindgen --version
printf '\nReady. Run: source "%s/env.sh"\n' "$FERRODESIGN_TOOLS"
