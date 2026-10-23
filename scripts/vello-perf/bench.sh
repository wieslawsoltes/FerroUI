#!/bin/zsh
# Builds the frame benchmarks of the catalog once and runs the test binary for each renderer and
# benchmark, one process each, so every renderer is measured from the same build in one session.
#
#   scripts/vello-perf/bench.sh <outdir> <renderers> [benchmarks...]
#
#   <renderers>   comma separated: skia, vello-hybrid, vello-gpu, vello-cpu. Put skia first and
#                 last (for example skia,vello-hybrid,vello-gpu,skia2) to see how much the
#                 machine drifted during the session; a renderer named <name>2 runs <name> again.
#   benchmarks    default: the buttons page, the text page, the effects page and the catalog tour.
#
# Environment:
#   CARGO_TARGET_DIR      where to build (default: target of the checkout)
#   FERROUI_BENCH_SURFACE metal (default; macOS only) or raster (software)
#   FERROUI_BENCH_SCALING 2 (default)
#
# Run the binary rather than `cargo test`: cargo builds the sample crates again on every run,
# which loads the machine and slows the first renderer measured (docs/porting/vello-backend.md,
# section 14.5). Logs go to <outdir>/<renderer>-<benchmark>.log; table.py summarizes them.
set -u
root=${0:A:h:h:h}
out=$1; renderers=(${(s:,:)2}); shift 2
tests=("$@")
[ ${#tests} -eq 0 ] && tests=(frame_benchmark_buttons_page frame_benchmark_text_page frame_benchmark_effects frame_benchmark_catalog_tour)
surface=${FERROUI_BENCH_SURFACE:-metal}
scaling=${FERROUI_BENCH_SCALING:-2}
mkdir -p $out
cd $root
bin=$(cargo test -p control-catalog --release --lib --no-run --message-format=json 2>/dev/null | python3 -c '
import sys, json
for line in sys.stdin:
    try: m = json.loads(line)
    except Exception: continue
    if m.get("reason") == "compiler-artifact" and m.get("target", {}).get("name") == "control_catalog" and m.get("profile", {}).get("test") and m.get("executable"):
        print(m["executable"])')
[ -z "$bin" ] && { echo "build failed"; exit 1; }
echo "binary $bin"
for label in $renderers; do
  r=${label%2}
  for t in $tests; do
    if [ $surface = raster ]; then
      (cd $root/samples/ControlCatalog && FERROUI_TEST_RENDERER=$r FERROUI_BENCH_SCALING=$scaling $bin $t --ignored --nocapture --test-threads=1 > $out/$label-$t.log 2>&1)
    else
      (cd $root/samples/ControlCatalog && FERROUI_TEST_RENDERER=$r FERROUI_BENCH_SURFACE=$surface FERROUI_BENCH_SCALING=$scaling $bin $t --ignored --nocapture --test-threads=1 > $out/$label-$t.log 2>&1)
    fi
    echo "$label $t exit $?"
  done
done
