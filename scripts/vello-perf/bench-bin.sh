#!/bin/zsh
# Runs prebuilt catalog test binaries in one session: an A/B of two builds (copy the binary of
# the first build aside before building the second; bench.sh prints its path).
#
#   scripts/vello-perf/bench-bin.sh <outdir> <label>=<binary>=<renderer> ...
#
# Example:
#   bench-bin.sh out skia=before=skia gpu-before=before=vello-gpu gpu-after=after=vello-gpu skia2=before=skia
#
# Environment: TESTS (space separated benchmarks), FERROUI_BENCH_SURFACE (metal), FERROUI_BENCH_SCALING (2).
set -u
root=${0:A:h:h:h}
out=$1; shift
tests=(${(s: :)${TESTS:-frame_benchmark_buttons_page frame_benchmark_text_page frame_benchmark_effects frame_benchmark_catalog_tour}})
mkdir -p $out
for spec in "$@"; do
  parts=(${(s:=:)spec}); label=$parts[1]; bin=${parts[2]:A}; r=$parts[3]
  for t in $tests; do
    (cd $root/samples/ControlCatalog && FERROUI_TEST_RENDERER=$r FERROUI_BENCH_SURFACE=${FERROUI_BENCH_SURFACE:-metal} FERROUI_BENCH_SCALING=${FERROUI_BENCH_SCALING:-2} $bin $t --ignored --nocapture --test-threads=1 > $out/$label-$t.log 2>&1)
    echo "$label $t exit $?"
  done
done
