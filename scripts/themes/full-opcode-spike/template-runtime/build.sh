#!/bin/sh
# Run from the root of an independently prepared checkout. Output and cache must be absolute.
set -eu
mode=${1:?select native, stock, single or fused}
out=${2:?output directory}
cache=${3:?Cargo build directory}
case "$mode" in native) backend=native;; stock|single|fused) backend=bytecode;; *) exit 2;; esac
case "$out" in /*) ;; *) exit 2;; esac
case "$cache" in /*) ;; *) exit 2;; esac
scripts=scripts/themes/full-opcode-spike
projects="$PWD/target/template-loop-probes"
mkdir -p "$out"
python3 "$scripts/prepare.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$scripts/template-runtime/prepare.py" "$projects"
for theme in winui fluent; do
    FERROUI_XAML_BACKEND="$backend" CARGO_TARGET_DIR="$cache" cargo build --offline --profile dist --manifest-path "$projects/$theme/Cargo.toml"
    cp "$cache/dist/template-$theme-probe" "$out/$theme-$mode-unstripped"
    cp "$out/$theme-$mode-unstripped" "$out/$theme-$mode"
    strip "$out/$theme-$mode"
    codesign -f -s - "$out/$theme-$mode"
    codesign --verify "$out/$theme-$mode"
done
