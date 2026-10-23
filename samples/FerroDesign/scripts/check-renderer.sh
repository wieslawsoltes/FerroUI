#!/usr/bin/env bash
# Verify the resolved application graph for each supported target without
# compiling or requiring cross-compilation toolchains to be installed.
set -euo pipefail

FERRODESIGN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ "$#" -eq 0 ]; then
    set -- wasm32-unknown-emscripten x86_64-unknown-linux-gnu \
        x86_64-pc-windows-msvc aarch64-apple-darwin x86_64-apple-darwin
fi
for FERRODESIGN_TARGET in "$@"; do
    FERRODESIGN_GRAPH="$(cd "$FERRODESIGN_ROOT" && cargo tree --locked \
        --target "$FERRODESIGN_TARGET" -p ferrodesign-app -e normal,build)"
    if [[ "$FERRODESIGN_GRAPH" =~ ferroui-skia|skia-safe|skia-bindings ]]; then
        printf 'Skia entered the active FerroDesign graph for %s.\n' "$FERRODESIGN_TARGET" >&2
        exit 1
    fi
    if [[ ! "$FERRODESIGN_GRAPH" =~ ferroui-vello ]]; then
        printf 'FerroUI Vello is missing from the graph for %s.\n' "$FERRODESIGN_TARGET" >&2
        exit 1
    fi
    if [ -n "${FERRODESIGN_GRAPH_DIR:-}" ]; then
        mkdir -p "$FERRODESIGN_GRAPH_DIR"
        printf '%s\n' "$FERRODESIGN_GRAPH" > "$FERRODESIGN_GRAPH_DIR/$FERRODESIGN_TARGET.txt"
    fi
    printf 'Verified FerroUI Vello without Skia: %s\n' "$FERRODESIGN_TARGET"
done
