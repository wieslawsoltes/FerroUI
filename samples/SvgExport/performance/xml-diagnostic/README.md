# Isolated XML diagnostic

This diagnostic investigated the small raster warm-export regression in the initial optimized
executable. It compares the corrected baseline XML source with the initial optimized XML source,
plus two variants that separately restore the old escaper or identity-rewrite implementation.
The production XML source is unchanged between the initial and final optimized revisions.

All four variants execute in one Release binary with CPU 0 affinity, thin LTO, 16 code-generation
units and panic abort. Every combination uses eight warmups and 800 individual samples; the
first variant rotates between samples. Input/options preparation, exact-output checks, output
destruction and CSV bookkeeping are outside each timed `xml::optimize` call.

The inputs use the exact embedded PNG from the real raster fixture. The expanded input places
one equivalent image definition before each of its eight semantic layer groups. Deduplication
must reproduce the original 26,840-byte SVG exactly and remove seven aliases. Disabling dedup
produces the same 198,354-byte output in every implementation. The third input is the already
normalized single-resource SVG.

`timings-run1.csv` contains 9,600 individual timing records. `summary-run1.txt` and
`measurement-metadata.json` retain the measured medians, binary fingerprint and limits.
`input-metadata.json` records source/input hashes and the original fixture provenance.

The current XML implementation is faster in all three isolated cases. Restoring either old
implementation does not explain or fix the whole-export result. This is an isolation experiment;
it does not reproduce the complete executable's layout, Skia/render integration or surrounding
allocation history, and therefore does not assign the full-pipeline discrepancy to a mechanism.

## Reproduction

Generate the baseline raster fixture with the main benchmark, then reconstruct this crate in
a **new directory outside the repository workspace**. `prepare.py` reads both pinned XML sources
from Git, constructs the two restoration variants and recreates the exact input. Its preparation
was checked against all four recorded source hashes and both recorded input hashes.

```sh
python3 samples/SvgExport/performance/xml-diagnostic/prepare.py \
  --fixture "$baseline_fixtures/raster-normal.svg" \
  --output "$diagnostic_directory"
cd "$diagnostic_directory"
cargo build --release --locked
taskset -c 0 target/release/ferroui-svg-xml-diagnostic timings.csv
```

`Cargo.toml.template`, `Cargo.lock.template` and `main.rs` are copied into the generated crate;
the template filenames keep it outside the main repository's Cargo workspace.
