#!/bin/bash
# Builds the Skia binaries of the threaded browser build: the published binaries of the pinned
# skia-bindings crate with the bindings shim compiled again, with threads.
#
#   scripts/browser/skia-threads-shim.sh [<tools directory>]     default: <repository>/.tools
#   scripts/browser/skia-threads-shim.sh --id                    prints the name of the result
#
# Run by `scripts/browser/setup.sh --threads`; by hand it needs the environment of the browser
# build (`source <tools directory>/env.sh`). `scripts/build-browser.sh --threads` uses the result;
# the build without threads does not know about it.
#
# Why. The skia-bindings crate does not compile anything for this target: its build script downloads
# one archive (skia-binaries-<key>.tar.gz) with libskia.a (Skia, built by Skia's own build, whose
# configuration for WebAssembly passes -pthread), libskia-bindings.a (the shim: the crate's
# src/bindings.cpp, gl.cpp, gpu.cpp and ganesh.cpp, compiled by the `cc` crate on the machine that
# published the binaries, without -pthread) and bindings.rs (generated from the same sources). The
# flags `scripts/build-browser.sh --threads` exports for C++ never reach the shim, because nothing
# is compiled here. Without atomics the reference counts its inline code touches are not atomic, the
# guard of a local static is not thread-safe and a `thread_local` is one global for all threads:
# three of the four objects carry the mark LLVM leaves when it has stripped thread-local storage
# (`-shared-mem` in their target features). See docs/porting/browser-render-worker.md, "Skia shim".
#
# What. The four sources are compiled here with -pthread, against the headers of the Skia revision
# the crate names and with the preprocessor definitions of the Skia build, and put in place of the
# shim in a copy of the published archive. libskia.a and bindings.rs stay the published files.
# The crate reads the copy through SKIA_BINARIES_URL (a file:// address), which is its own way to
# take binaries from elsewhere. Steps:
#   1. the crate file of the version in Cargo.lock, from crates.io, checked against the checksum of
#      Cargo.lock (the shim sources, the revision of the crate, the tag of Skia);
#   2. the published binaries for the key the crate would ask for;
#   3. the Skia source of that tag (headers; nothing of Skia is compiled);
#   4. the definitions: `gn gen` with the arguments the crate's build script passes for this target
#      and feature set, then the `defines` of obj/skia.ninja and obj/gpu.ninja, which is how the
#      crate gets them in a build from source (build_support/skia_bindgen.rs, from_ninja_features);
#   5. the compile, with the flags the `cc` crate and the build script give (-O3, sections, C++20,
#      no RTTI, the include directories of the sysroot) plus -pthread;
#   6. checks: every function the published shim defines is defined by the new one and no other,
#      and every object of both archives uses atomics and bulk memory.
# The result is kept in <tools directory>/skia-threads/<id>/ and built once: the id names the
# version of the crate, the feature set, the version of Emscripten and the format below. About
# 200 MB are downloaded and unpacked in a work directory that is deleted at the end; the compile
# itself is four files.
#
# Needs: em++, emar and llvm-nm of the activated Emscripten SDK (EMSDK set), python3 (Skia's
# bin/fetch-gn and gn itself), curl, tar, and network access to crates.io, github.com and
# chrome-infra-packages.appspot.com (the gn binary Skia pins).
set -euo pipefail

# Changed when this script changes what it produces: a new id, and so a new build of the result.
FORMAT="1"
TARGET="wasm32-unknown-emscripten"
# The features of skia-bindings in the browser build, as they appear in the key of the binaries
# (build_support/features.rs, to_key): Ganesh on GL, JPEG decode and encode, PDF. CI checks that the
# workspace resolves to them ("Skia features of the target" in .github/workflows/ci.yml). With
# another feature set the crate asks for another key, does not find it here and starts a build of
# Skia from source: change this pin and the sources below together with the features.
SKIA_FEATURES_KEY="ganesh-gl-jpegd-jpege-pdf"
# The sources of the shim for those features (build_support/skia_bindgen.rs, Configuration::new).
SHIM_SOURCES="bindings gl gpu ganesh"

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# A field of the skia-bindings package in Cargo.lock.
lock_field() {
  awk -v field="$1" '
    $0 == "name = \"skia-bindings\"" { found = 1; next }
    found && /^\[\[package\]\]/ { exit }
    found && index($0, field " = \"") == 1 { value = substr($0, length(field) + 5); sub(/"$/, "", value); print value; exit }
  ' "$ROOT/Cargo.lock"
}
VERSION="$(lock_field version)"
CHECKSUM="$(lock_field checksum)"
[ -n "$VERSION" ] && [ -n "$CHECKSUM" ] || { echo "cannot read the version and the checksum of skia-bindings from Cargo.lock" >&2; exit 1; }

command -v emcc >/dev/null || { echo "emcc is not on PATH: activate the Emscripten SDK first (source <tools directory>/env.sh)" >&2; exit 1; }
EMSCRIPTEN_VERSION="$(emcc -dumpversion)"
ID="skia-bindings-$VERSION-$SKIA_FEATURES_KEY-emscripten-$EMSCRIPTEN_VERSION-$FORMAT"
if [ "${1:-}" = "--id" ]; then
  echo "$ID"
  exit 0
fi

TOOLS="${1:-$ROOT/.tools}"
mkdir -p "$TOOLS"
TOOLS="$(cd "$TOOLS" && pwd)"
OUT="$TOOLS/skia-threads/$ID"
WORK="$TOOLS/skia-threads/work"

if [ -f "$OUT/complete" ] && compgen -G "$OUT/skia-binaries-*.tar.gz" >/dev/null; then
  echo "up to date: $OUT"
  exit 0
fi

[ -n "${EMSDK:-}" ] || { echo "EMSDK is not set: activate the Emscripten SDK first" >&2; exit 1; }
for TOOL in em++ emar python3 curl tar; do
  command -v "$TOOL" >/dev/null || { echo "$TOOL is not on PATH" >&2; exit 1; }
done
NM="$EMSDK/upstream/bin/llvm-nm"
[ -x "$NM" ] || { echo "no llvm-nm at $NM" >&2; exit 1; }
NODE="${EMSDK_NODE:-node}"
command -v "$NODE" >/dev/null || { echo "node is not available (EMSDK_NODE, or node on PATH)" >&2; exit 1; }

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi
}
download() {
  echo "   $1"
  curl --fail --location --silent --show-error --retry 3 --output "$2" "$1"
}

rm -rf "$OUT" "$WORK"
mkdir -p "$WORK/crate" "$WORK/prebuilt" "$WORK/skia" "$WORK/objects"

echo "-- the crate skia-bindings $VERSION"
download "https://static.crates.io/crates/skia-bindings/skia-bindings-$VERSION.crate" "$WORK/crate.tar.gz"
FOUND="$(sha256 "$WORK/crate.tar.gz")"
[ "$FOUND" = "$CHECKSUM" ] || { echo "the crate file has the checksum $FOUND, Cargo.lock has $CHECKSUM" >&2; exit 1; }
tar -xzf "$WORK/crate.tar.gz" -C "$WORK/crate" --strip-components=1
for SOURCE in $SHIM_SOURCES; do
  [ -f "$WORK/crate/src/$SOURCE.cpp" ] || { echo "the crate has no src/$SOURCE.cpp" >&2; exit 1; }
done
# The key of the binaries: the first twenty digits of the revision the crate was packaged from, the
# target and the features (build_support/binary_cache/binaries.rs, key).
REVISION="$(sed -n 's/.*"sha1": *"\([0-9a-f]*\)".*/\1/p' "$WORK/crate/.cargo_vcs_info.json")"
[ "${#REVISION}" -ge 20 ] || { echo "cannot read the revision of the crate from .cargo_vcs_info.json" >&2; exit 1; }
KEY="${REVISION:0:20}-$TARGET-$SKIA_FEATURES_KEY"
# The tag of Skia the crate was written against ([package.metadata] of its Cargo.toml).
SKIA_TAG="$(sed -n 's/^skia = "\(.*\)"$/\1/p' "$WORK/crate/Cargo.toml")"
[ -n "$SKIA_TAG" ] || { echo "cannot read the tag of Skia from the Cargo.toml of the crate" >&2; exit 1; }

echo "-- the published binaries $KEY"
download "https://github.com/rust-skia/skia-binaries/releases/download/$VERSION/skia-binaries-$KEY.tar.gz" "$WORK/prebuilt.tar.gz"
tar -xzf "$WORK/prebuilt.tar.gz" -C "$WORK/prebuilt"
PREBUILT="$WORK/prebuilt/skia-binaries"
for FILE in libskia.a libskia-bindings.a bindings.rs key.txt; do
  [ -f "$PREBUILT/$FILE" ] || { echo "the published binaries have no $FILE" >&2; exit 1; }
done
[ "$(cat "$PREBUILT/key.txt")" = "$KEY" ] || { echo "the published binaries name the key $(cat "$PREBUILT/key.txt"), expected $KEY" >&2; exit 1; }

echo "-- the Skia source $SKIA_TAG"
# The address the crate downloads from in a build from source (build_support/binary_cache/
# download.rs); `infra` is left out there too.
download "https://codeload.github.com/rust-skia/skia/tar.gz/$SKIA_TAG" "$WORK/skia.tar.gz"
tar -xzf "$WORK/skia.tar.gz" -C "$WORK/skia" --strip-components=1 --exclude="skia-$SKIA_TAG/infra"
[ -f "$WORK/skia/include/core/SkTypes.h" ] || { echo "the Skia source did not unpack to $WORK/skia" >&2; exit 1; }

echo "-- the definitions of the Skia build"
(cd "$WORK/skia" && python3 bin/fetch-gn)
# The arguments of build_support/skia/config.rs (FinalBuildConfiguration::from_build_configuration)
# and build_support/platform/emscripten.rs for the features above, with no debug build, no system
# libraries and the release optimisation level of the published binaries. Nothing is built from
# them; they decide the `defines` of the ninja files.
GN_ARGS="is_skia_standalone=false is_official_build=true skia_use_partition_alloc=false is_debug=false"
GN_ARGS="$GN_ARGS skia_enable_svg=false skia_enable_ganesh=true skia_enable_graphite=false"
GN_ARGS="$GN_ARGS skia_enable_skottie=false skia_enable_pdf=true skia_use_gl=true skia_use_egl=false"
GN_ARGS="$GN_ARGS skia_use_x11=false skia_use_system_libpng=false skia_use_libwebp_encode=false"
GN_ARGS="$GN_ARGS skia_use_libwebp_decode=false skia_use_system_zlib=false skia_use_xps=false"
GN_ARGS="$GN_ARGS skia_use_dng_sdk=false skia_use_libjpeg_turbo_decode=true skia_use_libjpeg_turbo_encode=true"
GN_ARGS="$GN_ARGS cc=\"emcc\" cxx=\"em++\" skia_use_icu=false skia_use_harfbuzz=false skia_use_freetype=true"
GN_ARGS="$GN_ARGS skia_use_freetype_woff2=false skia_use_system_freetype2=false"
GN_ARGS="$GN_ARGS skia_use_system_libjpeg_turbo=false skia_use_expat=true skia_use_system_expat=false"
GN_ARGS="$GN_ARGS skia_gl_standard=\"webgl\" skia_use_webgl=true target_cpu=\"wasm\" skia_emsdk_dir=\"$EMSDK\""
GN_ARGS="$GN_ARGS skia_enable_fontmgr_custom_embedded=false skia_enable_fontmgr_custom_empty=true"
GN_ARGS="$GN_ARGS extra_cflags=[\"-O3\",\"--target=$TARGET\"] extra_asmflags=[\"--target=$TARGET\"]"
(cd "$WORK/skia" && bin/gn gen "$WORK/gn" --script-executable=python3 "--args=$GN_ARGS")
# One definition per line, without the -D: the `defines` line of each file, split at white space,
# the escapes of ninja ($) and of the shell (\) removed, the first definition of a name kept
# (build_support/skia_bindgen.rs: from_ninja_files, from_defines_str, unescape_ninja, combine).
python3 - "$WORK/gn/obj/skia.ninja" "$WORK/gn/obj/gpu.ninja" > "$WORK/skia-defines.txt" <<'PYTHON'
import sys

def unescape(text, escape):
    result, characters = [], iter(text)
    for character in characters:
        if character == escape:
            following = next(characters, None)
            if following is not None:
                result.append(following)
        else:
            result.append(character)
    return "".join(result)

names = set()
for path in sys.argv[1:]:
    with open(path, encoding="utf-8") as file:
        line = next((line for line in file.read().splitlines() if line.startswith("defines = ")), None)
    if line is None:
        sys.exit(f"no line starting with `defines = ` in {path}")
    for definition in line[len("defines = "):].split():
        if not definition.startswith("-D"):
            sys.exit(f"a definition without -D in {path}: {definition}")
        name, separator, value = definition[2:].partition("=")
        if name in names:
            continue
        names.add(name)
        print(name + separator + unescape(unescape(value, "$"), "\\"))
PYTHON
[ -s "$WORK/skia-defines.txt" ] || { echo "no definitions were read from the ninja files" >&2; exit 1; }
DEFINES=()
while IFS= read -r DEFINITION; do
  DEFINES+=("-D$DEFINITION")
done < "$WORK/skia-defines.txt"
echo "   ${#DEFINES[@]} definitions"

echo "-- the shim, with -pthread"
# The flags of the `cc` crate for the target (the optimisation level of a release build, a section
# per function and per datum, no position-independent code), then those of the build script
# (skia_bindgen.rs: the standard, no RTTI; platform/emscripten.rs: the include directories of the
# sysroot of the SDK instead of the compiler's own), then the one this script exists for.
FLAGS=(-O3 -ffunction-sections -fdata-sections -std=c++20 -fno-rtti -nobuiltininc -fvisibility=default)
SYSROOT_INCLUDE="$EMSDK/upstream/emscripten/cache/sysroot/include"
[ -d "$SYSROOT_INCLUDE" ] || { echo "no sysroot of the SDK at $SYSROOT_INCLUDE" >&2; exit 1; }
[ -d "$SYSROOT_INCLUDE/c++/v1" ] && FLAGS+=("-isystem$SYSROOT_INCLUDE/c++/v1")
FLAGS+=("-isystem$SYSROOT_INCLUDE")
[ -d "$SYSROOT_INCLUDE/compat" ] && FLAGS+=("-isystem$SYSROOT_INCLUDE/compat")
FLAGS+=(-pthread)
OBJECTS=()
for SOURCE in $SHIM_SOURCES; do
  echo "   src/$SOURCE.cpp"
  (cd "$WORK/crate" && em++ "${FLAGS[@]}" -I "$WORK/skia" "${DEFINES[@]}" -c "src/$SOURCE.cpp" -o "$WORK/objects/$SOURCE.o")
  OBJECTS+=("$WORK/objects/$SOURCE.o")
done
emar crs "$WORK/libskia-bindings.a" "${OBJECTS[@]}"

echo "-- checks"
# The functions an archive defines. The shim leaves out and puts in functions by the definitions
# (the codecs, the font managers, PDF, WebGL), so the same list is the evidence that the definitions
# that decide what the shim contains are those of the published build.
functions() {
  "$NM" --defined-only --extern-only "$1" | awk 'NF == 3 && $2 == "T" { print $3 }' | LC_ALL=C sort -u
}
functions "$PREBUILT/libskia-bindings.a" > "$WORK/functions-published.txt"
functions "$WORK/libskia-bindings.a" > "$WORK/functions-threads.txt"
[ -s "$WORK/functions-published.txt" ] || { echo "llvm-nm found no functions in the published shim" >&2; exit 1; }
if ! diff "$WORK/functions-published.txt" "$WORK/functions-threads.txt" > "$WORK/functions.diff"; then
  echo "the shim compiled here does not define the functions of the published shim (< published, > here):" >&2
  cat "$WORK/functions.diff" >&2
  echo "the work directory is kept: $WORK" >&2
  exit 1
fi
echo "   $(wc -l < "$WORK/functions-threads.txt" | tr -d ' ') functions, the same as the published shim"
"$NODE" "$ROOT/scripts/browser/wasm-features.mjs" --summary --require atomics,bulk-memory \
  "$PREBUILT/libskia.a" "$WORK/libskia-bindings.a"

echo "-- the archive"
cp "$WORK/libskia-bindings.a" "$PREBUILT/libskia-bindings.a"
mkdir -p "$OUT"
# The name and the layout the crate expects: one directory `skia-binaries` (binaries.rs, unpack).
# COPYFILE_DISABLE keeps the tar of macOS from adding a file of attributes for each file.
COPYFILE_DISABLE=1 tar -czf "$OUT/skia-binaries-$KEY.tar.gz" -C "$WORK/prebuilt" skia-binaries
cp "$WORK/skia-defines.txt" "$OUT/skia-defines.txt"
cp "$WORK/functions-threads.txt" "$OUT/functions.txt"
printf '%s\n' "skia-bindings $VERSION, key $KEY, Skia $SKIA_TAG, Emscripten $EMSCRIPTEN_VERSION" > "$OUT/complete"
rm -rf "$WORK"
echo "written: $OUT/skia-binaries-$KEY.tar.gz"
