#!/usr/bin/env bash
# Build Ferro Desktop with the Linux Graphite backend in this Cargo invocation.
set -euo pipefail

usage() {
    printf 'Usage: %s [build|run|check|test] [Cargo arguments] [-- program arguments]\n' "$0"
}
case ${1:-build} in
    -h|--help) usage; exit 0 ;;
    build|run|check|test) action=${1:-build};;
    *) usage >&2; exit 2 ;;
esac
if (($# > 0)); then shift; fi
if [[ $(uname -s) != Linux ]]; then
    echo 'The Graphite Vulkan desktop build requires a Linux host.' >&2
    exit 1
fi

root=$(cd -- "$(dirname -- "$0")/../.." && pwd)
tag=m153-0.101.2
digest=30c8b51a0a32fae63562c303bdc91605519a9598951422610e3fb8685b3c4f6f
valid_headers() {
    [[ -f $1/include/gpu/graphite/Context.h && -f $1/include/third_party/vulkan/vulkan/vulkan_core.h &&
       -f $1/modules/skcms/skcms.h ]]
}

if [[ -z ${FERROUI_SKIA_HEADERS:-} ]]; then
    cache=${XDG_CACHE_HOME:-$HOME/.cache}/ferroui/skia
    mkdir -p -- "$cache"
    dest=$cache/$tag
    if ! valid_headers "$dest"; then
        # The lock is held in this subshell only. Interruptions leave the published cache intact.
        (
            flock -x 9
            if valid_headers "$dest"; then exit 0; fi
            if [[ -e $dest ]]; then
                printf 'Incomplete Skia header cache at %s. Move it aside and retry.\n' "$dest" >&2
                exit 1
            fi
            stage=$(mktemp -d -- "$cache/.headers.XXXXXXXX")
            trap 'rm -rf -- "$stage"' EXIT
            url="https://codeload.github.com/rust-skia/skia/tar.gz/refs/tags/$tag"
            curl --fail --location --silent --show-error --retry 3 --output "$stage/skia.tar.gz" "$url"
            (cd -- "$stage" && printf '%s  skia.tar.gz\n' "$digest" | sha256sum --check --status) || {
                echo 'Checksum mismatch for the pinned Skia headers.' >&2
                exit 1
            }
            tar -xzf "$stage/skia.tar.gz" --directory="$stage" --wildcards \
                "skia-$tag/include/*" "skia-$tag/modules/skcms/*.h"
            valid_headers "$stage/skia-$tag" || { echo 'Incomplete pinned Skia archive.' >&2; exit 1; }
            mv -T -- "$stage/skia-$tag" "$dest"
        ) 9>"$cache/.$tag.lock"
    fi
    FERROUI_SKIA_HEADERS=$dest
fi
if ! valid_headers "$FERROUI_SKIA_HEADERS"; then
    printf 'FERROUI_SKIA_HEADERS must name a matching Skia source root with include/ and modules/skcms/: %s\n' "$FERROUI_SKIA_HEADERS" >&2
    exit 1
fi
# build.rs runs from the Skia crate. Resolve even a caller-supplied relative path before changing cwd.
FERROUI_SKIA_HEADERS=$(cd -- "$FERROUI_SKIA_HEADERS" && pwd)
export FERROUI_SKIA_HEADERS

# Cargo gives encoded flags priority over RUSTFLAGS. Preserve exact argument boundaries in that mode.
if [[ -v CARGO_ENCODED_RUSTFLAGS ]]; then
    sep=$'\x1f'
    export CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS}${CARGO_ENCODED_RUSTFLAGS:+$sep}--cfg${sep}ferro_skia_graphite_vulkan"
else
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg ferro_skia_graphite_vulkan"
fi
profile=(--release)
for arg in "$@"; do
    case $arg in
        --) break ;;
        -r|--release|--profile|--profile=*) profile=(); break ;;
    esac
done
cd -- "$root"
exec cargo "$action" --locked -p ferro-desktop "${profile[@]}" "$@"
