# SVG performance evidence

The [performance report](../../../docs/porting/svg-performance.md) explains the changes, results
and limits. This directory retains the measurements used to make those decisions, including
the earlier regression and its follow-up experiments.

## Primary final comparison

| Dataset | Source | Contents |
| --- | --- | --- |
| `baseline-paired/` | `7e2d2cd899cac0b6a1a6f887e3e5ebba15ffb2f6` | 15 CSVs from the corrected baseline executable, paired with final exports. |
| `optimized/` | `553b2e75b930079c2c8535e6256c8c4263b05aaa` | 15 CSVs from the final optimized executable. |
| `paired-final-metadata.json` | Both executables | Exact execution order, source/binary fingerprints, commands and process timestamps. |
| `environment.json` | Both executables | Compiler/profile, CPU affinity, native archive, tool versions and aggregation method. |

`paired-final-metadata.json` is copied byte-for-byte from the original run. Its relative CSV
paths beginning `baseline/` map to `baseline-paired/` here, while `optimized/` is unchanged;
the original serial baseline retains the `baseline/` directory. This relocation is also recorded
in [paired-final-path-map.json](paired-final-path-map.json). Absolute commands and timestamps
remain exactly as recorded.

There are three fresh-process pairs per workload. Each process uses 40 warm samples, eight
record/fresh/cache-cold samples and five warmups for `warm`, `multi_view` and `no_dedup`.
The gzip phase has 40 samples and no separate warmup. Baseline executes first in repetitions
1 and 3; repetition 2 reverses the order. Every process is pinned to logical CPU 0, with no
concurrent local compilation, renderer or validation job.

The CSV files are preserved exactly as the benchmark emitted them. They contain **phase
distribution summaries**, not individual timing samples. With eight samples, nearest-rank p95
is the observed maximum. Ranges of process p95 values are not pooled percentiles or confidence
intervals.

`summarize.py` verifies scene, byte-count and report-counter consistency and computes both the
median of three process medians per executable and the median of the three within-pair speed
ratios. The baseline schema lacks `prepared_vector_cache_hits`, because that cache did not
exist; the summarizer interprets the missing field as zero without modifying the baseline data.
Prepared hits are a subset of `vector_cache_hits` and must not be added to it.

```sh
python3 samples/SvgExport/performance/summarize.py
python3 samples/SvgExport/performance/summarize.py --json
```

To repeat the matched experiment, first build and preserve one Release benchmark executable
from each source revision, then use a new result directory:

```sh
python3 samples/SvgExport/performance/run_pairs.py \
  --baseline-executable "$baseline_benchmark" \
  --optimized-executable "$optimized_benchmark" \
  --baseline-revision 7e2d2cd899cac0b6a1a6f887e3e5ebba15ffb2f6 \
  --optimized-revision 553b2e75b930079c2c8535e6256c8c4263b05aaa \
  --results "$new_results"
```

The runner uses Linux `taskset` and Python 3.11 or newer. The Rust benchmark itself is portable.
The report documents asset preparation, timed boundaries and fixture-generation commands.

## Earlier rounds and diagnostics

These datasets are retained for auditability. They are not substituted into the final tables.

| Directory / file | Purpose |
| --- | --- |
| `baseline/` | Original serial three-process baseline round, preserved unchanged. |
| `optimized-initial/` | Original serial round at `447cef96e659fe5d579fd69d2d17afad2b631761`, before boxing cache storage. |
| `environment-initial.json` | Original source, executable and profile fingerprints. |
| `raster-paired-initial/` | Three alternating baseline/initial-optimized raster pairs that confirmed a small full-export regression. |
| `xml-diagnostic/` | Same-process XML isolation: four implementations, three inputs, 800 individual samples per combination. |
| `raster-boxing/` | Three rotated-order baseline/initial/boxed raster triples. |
| `boxing-vector-controls/` | Three initial/boxed pairs each for geometry and text, including the observed timing variation. |
| `boxing-build.json` | The candidate binary and exact two-file patch fingerprints before the same source was committed as `553b2e75`. |

For the original serial comparison, select its directories explicitly:

```sh
python3 samples/SvgExport/performance/summarize.py \
  --baseline samples/SvgExport/performance/baseline \
  --optimized samples/SvgExport/performance/optimized-initial
```

Corresponding run numbers in that original serial comparison were **not** adjacent process
pairs; its optional same-run ratios must not be interpreted as the matched final protocol.

The XML diagnostic is described in its [README](xml-diagnostic/README.md). It retains 9,600
individual nanosecond timings; this differs from the main benchmark's phase-summary CSVs.
Its result does not reproduce whole-executable code layout or surrounding allocation history.

## Output invariants

`baseline-fixtures.json`, `optimized-initial-fixtures.json` and `optimized-fixtures.json` record
the 30 fixture SVG, PNG and decoded-RGBA hashes. All three versions have identical bytes.
The corresponding renderer logs retain XML/reference checks and RGB/alpha comparisons against
Skia's native replay. The report explains the dense-text rasterizer distinction explicitly.

`aeris-fixtures.json` records the final-source regeneration of all five committed Aeris SVG
views and the normal SVGZ; all six artifacts remain unchanged. Large generated benchmark
SVG/PNG sets are reproducible using `export_bench --fixtures` and are not duplicated here.
The [validation summary](validation/summary.json) and adjacent logs record the final source's
112 default tests, 112 `media3d` tests, strict Clippy, validator tests and example invariants.

Memory summaries use the maximum observed `/proc/self/status` `VmHWM` per process, then the
median of those three values. They are approximate whole-process KiB, including native assets,
temporary output and allocator retention, not phase-local allocations or exact cache size.

Validation command logs in `validation/` retain their diagnostic output; trailing blank lines
at the end of three logs are normalized to satisfy the repository whitespace check. Raw CSVs
and process metadata are unchanged.
