# SVG backend performance validation

This report compares the corrected SVG implementation at
[`7e2d2cd899`](https://github.com/wieslawsoltes/FerroUI/commit/7e2d2cd899cac0b6a1a6f887e3e5ebba15ffb2f6)
with the optimized implementation at
[`553b2e75b9`](https://github.com/wieslawsoltes/FerroUI/commit/553b2e75b930079c2c8535e6256c8c4263b05aaa).
The correctness round, including native image opacity and capture lifetime fixes, is already
present in the baseline. See [the review report](svg-review.md) for those findings.

## What changed

The optimization has two parts. XML processing now avoids creating replacement strings when a
CSS resource reference already has its canonical spelling, avoids redundant rewriting when
there are no resource aliases, and copies XML text between escape characters in spans. It
preserves validation, namespace handling, reference rewriting and empty-definition behavior.

Each immutable vector layer can also retain one prepared fragment alongside its existing
native SVG representation. A matching export reuses that fragment after its native-format
repair, identifier namespacing and inspection transformation. The key includes the native text
and path modes, fragment prefix and every effective inspection parameter, including signed-zero
float spelling. A changed native representation replaces its prepared fragment. The final
document is still composed, validated, optimized and serialized for each export; this is not a
cache of a completed document.

The cache holds at most one prepared view per vector layer. `cache_native_svg = false` bypasses
native and prepared caching; `clear_export_cache()` releases both and the raster cache. The
uncached path retains its owned fragment without converting it to an `Rc<str>`. For the five-view
sequence below, each vector layer has two prepared hits per batch, while all five exports avoid
native SVG generation. `prepared_vector_cache_hits` is therefore a subset of
`vector_cache_hits`, and the counters must not be added together.
This is a bound on retained variants, not a fixed byte budget: larger layer fragments retain
more memory. Applications exporting each recording once can disable caching, and applications
that finish inspecting a retained document can clear its export caches.

The optional native/prepared cache entry is boxed. Relative to the initial prepared-cache
implementation at `447cef96e6`, this x86-64 build reduces the inline `SvgLayer` representation
from 512 to 408 bytes (20.3%), so a layer without a native vector cache
does not carry that enlarged entry inline. The vector cache adds one allocation when created;
this does not change its retention policy. The measured performance effects are reported below
without attributing them to an unmeasured hardware or allocator mechanism.

## Measurement method

Both executables use the repository's actual `release` profile: optimization level 3, thin LTO,
the default 16 code-generation units, and panic abort. There are no package-level optimization
overrides. The build uses two Cargo jobs and the same unmodified pinned native Skia archive.
Both executables were copied to separate immutable files before measurements.

The host is Linux x86-64 on an Intel Xeon Platinum 8573C, with Rust 1.100.0, Cargo 1.100.0 and
Inkscape 1.4. Each measured process is pinned to logical CPU 0. The final comparison has three
fresh-process baseline/optimized pairs per workload, with no simultaneous Cargo, renderer or
validation jobs. Baseline runs first in repetitions 1 and 3; the order reverses in repetition 2.
Every final table uses only this matched dataset. Earlier serial rounds and focused diagnostics
are retained separately. The benchmark
uses embedded Inter text and the same default host font configuration in both runs. Full tool,
profile, archive, executable and source fingerprints are in
[environment.json](../../samples/SvgExport/performance/environment.json).

The benchmark is
[`export_bench.rs`](../../src/Svg/FerroUI.Svg/examples/export_bench.rs). Assets, font registration,
HarfBuzz shaping and input bitmap creation happen before measurement. Native font caches can
still initialize on the first recording, so these are prepared-asset, warm-process workloads,
not operating-system or application startup measurements. UI layout is excluded.

| Phase | Timed operation | Samples per process |
| --- | --- | ---: |
| `record` | Record a new document using the prepared assets. | 8 |
| `fresh_document` | Record a new document and export it, inside one timer. | 8 |
| `cold` | Export an existing recording after clearing its document export caches. | 8 |
| `warm` | Export the same recorded document with warm export caches. | 40 |
| `multi_view` | Export normal, x-ray, wireframe, cutaway and exploded views as one batch. | 40 |
| `no_dedup` | Warm normal export with resource deduplication disabled. | 40 |
| `gzip` | Compress an existing SVG into a fresh buffer at gzip level 6. No export is performed. | 40 |

The `warm`, `multi_view` and `no_dedup` phases have five untimed warmups; `gzip` has no separate
warmup. Output/document destruction, file I/O, XML checks and
fixture rendering are outside the timing windows. The fresh-document value is measured directly;
it is not the sum of separate recording and export medians. Multi-view times are for all five
exports, and its output size and report counters are also aggregated over the five outputs.

The latency columns use the **median of the three process medians** for each executable. A
ratio-of-medians column divides those two values. The **median paired speedup** instead divides
baseline by optimized median latency within each matched process pair, then takes the median
of those three ratios. These descriptive estimators can differ when timing varies between
repetitions; both are reported explicitly. No result is normalized using the gzip control.
Each CSV contains minimum, median
and nearest-rank p95 for that process and phase. With eight observations, p95 is the observed
maximum; it has limited statistical resolution. These CSVs are unmodified phase-distribution
summaries, not individual latency samples. A median or range of per-process p95 values is not
a pooled p95. The runs show timing variability; the observed spread is retained below.

The optimized CSV adds `prepared_vector_cache_hits`; its collection and formatting occur after
timing. The timed workload and arguments are unchanged. The original baseline CSVs are retained
without that column, which means zero prepared hits in comparisons because that cache did not
exist in the baseline.

Memory observations come from `/proc/self/status` after each phase. The report uses the maximum
observed `VmHWM` for each process, then the median of the three process values. These are approximate
whole-process snapshots, including assets, Skia, caches, temporary output and allocator retention;
they do not measure phase-local allocations or the prepared cache's exact size. In this host a few
successive `VmHWM` snapshots decreased slightly, so individual rows should not be interpreted as
exact monotonic memory accounting.
The `recording_bytes` column is Skia's approximate picture accounting, summed across layers;
it is not a measurement of total assets, document allocations or export-cache memory.

## Workloads

Every document is 1280 × 960 with eight named 640 × 240 semantic layers, positioned in two
columns and four rows. Repeated content is intentional and should be considered when using
these measurements: the resource workloads are favorable to deduplication where their inherited
contexts permit it. They do not represent every distribution of mixed application content.

| Workload | Recorded operations per document | Purpose |
| --- | --- | --- |
| Geometry | 1,536 rounded-rectangle, ellipse and line draws. | Large path payloads and repeated vector export. |
| Definitions | 1,536 draws, including repeated gradient background/border paint and curved paths, with 512 clips. | Both valid resource sharing and contexts that must remain separate; 512 definitions are deduplicated. |
| Text | 80 shaped text-line draws with Inter at 14 DIPs. | Outlined ligatures, combining marks, Polish text and dense path data. This is not a bidi or complex-script benchmark. |
| Bitmap | 128 cropped image draws at 0.85 opacity. | Native image encoding, alpha preservation, image references and view transformations. |
| Raster | 1,024 primitive draws inside isolated opacity groups. | Automatic whole-layer fallback and cached PNG reuse; identical layer images permit seven resource aliases. |

Normal exports use compact output, exact numeric values, absolute native paths and resource
deduplication. The first four workloads use `VectorOnly`, which prohibits added whole-layer raster
fallback; their existing bitmap inputs remain editable SVG image nodes. The raster workload uses
automatic fidelity. All use raster scale 1. Exploded-view labels are disabled so their font
selection does not enter the workload. Each workload verifies XML identifiers and cold/warm
byte equality for every inspection view outside the timing windows.

## Measured results

The final matched comparison shows the largest gains for **repeated exports of retained
documents**. Median within-pair warm speedups for the four vector workloads are **1.69–2.69×**;
their five-view batches improve **1.19–1.54×**. Cached raster output also improves in the final
implementation. All three matched pairs improve warm and five-view median latency for every
workload. These results do not imply an application-frame, UI-layout or general fresh-document
speedup.

### Repeated normal exports

| Workload | Baseline median, ms | Optimized median, ms | Ratio of medians | Median paired speedup |
| --- | ---: | ---: | ---: | ---: |
| Geometry | 38.9696 | 16.0647 | 2.426× | 2.433× |
| Definitions | 47.2995 | 28.0345 | 1.687× | 1.687× |
| Text | 31.8403 | 11.8056 | 2.697× | 2.686× |
| Bitmap | 3.9872 | 1.8283 | 2.181× | 2.133× |
| Raster | 0.8672 | 0.7871 | 1.102× | 1.076× |

### Five-view batches

Each sample below contains all five exports, not a single view.

| Workload | Baseline median, ms | Optimized median, ms | Ratio of medians | Median paired speedup |
| --- | ---: | ---: | ---: | ---: |
| Geometry | 221.4069 | 144.4457 | 1.533× | 1.535× |
| Definitions | 251.1634 | 207.7383 | 1.209× | 1.189× |
| Text | 156.9234 | 104.2154 | 1.506× | 1.506× |
| Bitmap | 22.1026 | 16.2292 | 1.362× | 1.363× |
| Raster | 4.0586 | 3.6834 | 1.102× | 1.102× |

The normal vector export reports eight native-cache hits, including eight prepared-fragment
hits. A five-view vector batch reports 40 native hits, including 16 prepared hits. Raster output
has no prepared-vector hits: normal export reuses eight cached PNGs, and a five-view batch
reuses 32 because wireframe suppresses image output.

### Fresh documents and cache-cold exports

| Workload | Fresh document baseline → optimized, ms | Median paired speedup | Cache-cold baseline → optimized, ms | Median paired speedup |
| --- | ---: | ---: | ---: | ---: |
| Geometry | 131.9126 → 125.9628 | 1.027× | 132.5829 → 124.5583 | 1.060× |
| Definitions | 65.3453 → 60.1510 | 1.013× | 64.8459 → 58.9246 | 1.051× |
| Text | 133.1239 → 131.3701 | 0.994× | 133.2190 → 126.3114 | 1.070× |
| Bitmap | 54.0555 → 52.4388 | 1.004× | 52.6592 → 52.8890 | 0.999× |
| Raster | 86.7740 → 85.1843 | 1.013× | 83.2096 → 83.8165 | 0.995× |

Fresh-document paired ratios range from 0.994× to 1.027×. First export must still replay the
native recording and prepare its first fragment; this measurement does not establish a general
fresh-document acceleration. Cache-cold changes are smaller and mixed. Individual cold/fresh
process pairs vary more than their aggregate values, and the original per-process results are
retained.

### Recording, dedup-disabled output and compression

Each cell gives baseline → optimized process-median aggregates in milliseconds, followed by
the median within-pair speed ratio in parentheses.

| Workload | Record, ms (paired ratio) | Dedup disabled, ms (paired ratio) | Gzip, ms (paired ratio) |
| --- | ---: | ---: | ---: |
| Geometry | 0.3014 → 0.2977 (0.997×) | 38.9341 → 16.1349 (2.433×) | 6.7451 → 6.7461 (1.021×) |
| Definitions | 1.0405 → 1.1562 (0.878×) | 38.8689 → 21.2595 (1.825×) | 4.0334 → 3.9373 (1.024×) |
| Text | 2.8593 → 2.7798 (1.018×) | 29.0977 → 13.9369 (2.088×) | 164.2050 → 166.2170 (0.986×) |
| Bitmap | 0.0317 → 0.0312 (1.016×) | 3.6316 → 1.5033 (2.419×) | 0.4602 → 0.4903 (0.932×) |
| Raster | 0.3263 → 0.3375 (0.967×) | 1.1077 → 0.8155 (1.318×) | 0.1515 → 0.1464 (1.038×) |

Recording is not an intended optimization target and is not a strictly unchanged-code control:
the layer/cache representation changed even though the recording workload did not. Its results
are mixed, including a paired slowdown for the definitions and raster cases. The gzip algorithm
is unchanged and its observed paired ratios range from 0.932× to 1.038×. No compression speedup
or universal latency improvement is claimed.

### Observed dispersion and tail limits

The ranges below contain three independent process statistics. The p95 ranges are the smallest
and largest **per-process p95**, not pooled percentiles or confidence intervals.

| Workload | Baseline warm process medians, ms | Optimized warm process medians, ms | Baseline warm p95 range, ms | Optimized warm p95 range, ms |
| --- | ---: | ---: | ---: | ---: |
| Geometry | 38.458–53.892 | 16.015–16.089 | 43.844–61.686 | 18.937–19.226 |
| Definitions | 46.481–52.361 | 27.184–44.622 | 59.929–100.228 | 30.157–55.222 |
| Text | 31.434–32.914 | 11.703–15.058 | 33.507–45.585 | 13.357–21.867 |
| Bitmap | 3.884–4.237 | 1.821–1.907 | 4.591–8.242 | 2.380–2.489 |
| Raster | 0.847–0.889 | 0.769–0.850 | 1.108–1.234 | 1.038–2.050 |

**Faster medians do not establish tighter tails.** In particular, the final raster warm p95
range is 1.0377–2.0503 ms, versus 1.1084–1.2339 ms for its matched baseline. The report retains
that upper-tail observation despite the improved median. The earlier serial baseline also
contained a definitions warm p95 of 200.600 ms against a 49.746 ms median; that latency outlier
remains in the historical dataset. These measurements do not provide a production tail-latency
guarantee or establish the cause of individual outliers.

### Output size and memory

| Workload | Normal SVG bytes | SVG bytes without dedup | Gzip bytes | Baseline median process HWM, KiB | Optimized median process HWM, KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| Geometry | 2,787,548 | 2,787,548 | 26,996 | 47,440 | 49,728 |
| Definitions | 555,252 | 656,483 | 23,530 | 48,788 | 47,916 |
| Text | 2,762,384 | 2,762,384 | 922,275 | 43,256 | 46,028 |
| Bitmap | 111,230 | 111,230 | 3,741 | 21,444 | 20,188 |
| Raster | 26,840 | 198,354 | 5,662 | 20,124 | 19,052 |

All byte counts are unchanged. Resource sharing was already present in the corrected baseline:
the definitions workload removes 512 duplicate definitions, and the raster workload replaces
seven duplicate image resources with references.

The final matched observations increase whole-process high water by about **2.2 MiB for geometry**
and **2.7 MiB for outlined text**; the other three cases have lower observed values. Retaining
prepared fragments is a speed/memory tradeoff. These approximate process observations include
native assets, temporary output and allocator retention, and do not attribute the entire
difference to the cache. `cache_native_svg = false` and `clear_export_cache()` give applications
control over retained export data.

### Regression investigation retained in the evidence

The first serial optimization round at `447cef96e6` improved vector reuse but increased raster
warm export from 0.8309 to 0.9734 ms. Three additional alternating process pairs still showed
0.8707 → 0.9593 ms (+10.2%). A same-process XML-only isolation instead improved the deduplication
operation from 0.8070 to 0.7644 ms, and restoring the old escaper or identity rewriting made that
isolated operation slower. This did not justify reverting those XML improvements or identify
the full-executable mechanism.

Boxing the optional cache entry reduced inline `SvgLayer` size from 512 to 408 bytes. The
bounded raster comparison then produced the following three-process median aggregates:

| Implementation in raster diagnostic | Warm, ms | Five-view batch, ms | Dedup disabled, ms |
| --- | ---: | ---: | ---: |
| Corrected baseline | 0.8752 | 3.7939 | 1.0778 |
| Initial optimization | 0.9214 | 4.2686 | 0.9778 |
| Boxed cache storage | 0.8055 | 3.6033 | 0.8151 |

Geometry controls favored boxing in all three warm/five-view pairs. Text controls were mixed:
two warm pairs improved and one increased slightly, amid substantial cross-phase variation.
That evidence led to the final interleaved baseline/final comparison reported above. All
historical CSVs and the XML diagnostic are retained in the [evidence directory](../../samples/SvgExport/performance/README.md);
none are substituted into the final tables. The report does not assign the improvement to an
unmeasured hardware-cache or allocator mechanism.

## Output and independent rendering invariants

All 30 benchmark SVG files are byte-identical before and after optimization. Their Inkscape
PNG files and decoded RGBA pixels are also identical. Checksums are recorded in
[baseline-fixtures.json](../../samples/SvgExport/performance/baseline-fixtures.json) and
[optimized-fixtures.json](../../samples/SvgExport/performance/optimized-fixtures.json).
The independent renderer checks XML structure, unique identifiers, local resource references,
normal output and every inspection view. Renderer logs are retained with the data.
The Aeris example was also rebuilt against the optimized source: all five SVG views and the
normal SVGZ remain byte-identical to the committed examples. Their hashes are recorded in
[aeris-fixtures.json](../../samples/SvgExport/performance/aeris-fixtures.json).

The following errors compare the SVG rendered by Inkscape with the same recordings replayed
through Skia's raster path, at 96 DPI. They are unchanged by the optimization.

| Workload | Mean absolute RGB error / 255 | Mean absolute alpha error / 255 |
| --- | ---: | ---: |
| Geometry | 0.8964 | 0.4671 |
| Definitions | 0.4968 | 0.0000 |
| Text | 6.5707 | 6.5707 |
| Bitmap | 0.2705 | 0.0000 |
| Raster | 0.0000 | 0.0000 |

Dense 14-DIP text makes the distinction between device-hinted raster glyphs and scalable
outlines visible. The SVG path data is identical when raster hinting/alignment options change.
At 384 DPI against a native 4× reference, text RGB error is 1.2355/255 with default hinting and
0.8428/255 with explicit unhinted/unaligned text. The positioned-glyph regression checks the
actual outline points against native glyph paths, including fractional advances, offsets and
baseline coordinates. These results do not claim pixel-identical small text between independent
rasterizers. `SvgFidelity::Raster` remains the policy for preserving native raster output.

The corrected baseline also fixes the image-opacity failure found by these workloads:
the earlier bitmap RGB error was 13.8267/255 and alpha error was 26.91/255. Native-only repair
of Skia's image paint encoding reduces those to 0.2705 and zero while retaining vector layers
and editable image nodes. This correctness improvement is **not** included in the performance
speedup calculation.

## Reproducing the measurements

Build the benchmark in a clean checkout of each recorded source revision, preserving the two
executables under different names:

```sh
cargo build --release --locked -p ferroui-svg --example export_bench
```

Run the two preserved executables as adjacent process pairs using a new result directory.
The runner reverses their order in repetition 2 and preserves the raw CSVs and exact process
metadata. Linux `taskset` provides the CPU affinity; the Rust benchmark itself is portable.

```sh
python3 samples/SvgExport/performance/run_pairs.py \
  --baseline-executable "$baseline_benchmark" \
  --optimized-executable "$optimized_benchmark" \
  --baseline-revision 7e2d2cd899cac0b6a1a6f887e3e5ebba15ffb2f6 \
  --optimized-revision 553b2e75b930079c2c8535e6256c8c4263b05aaa \
  --results "$new_results"
```

Recompute the tables and verify output-size, scene and report-counter consistency directly from
the retained CSVs:

```sh
python3 samples/SvgExport/performance/summarize.py
python3 samples/SvgExport/performance/summarize.py --json
```

Generate and validate fixtures separately from the measurement window:

```sh
"$benchmark" --iterations 1 --cold-iterations 1 --warmup 0 \
  --output "$results/fixture-check.csv" --fixtures "$results/fixtures"
python3 src/Svg/FerroUI.Svg/examples/validate_exports.py "$results/fixtures" \
  --preview-dir "$results/previews"
python3 -m unittest discover -s src/Svg/FerroUI.Svg/examples -p test_validate_exports.py
```

For the focused bitmap alpha gate, generate a separate fixture directory with `--only bitmap`
and pass `--max-mean-error 1 --max-alpha-error 1` to the validator. For the text diagnostic,
use `--only text --reference-scale 4`, optionally `--unhinted-text`, and render both the vector
and native-reference SVGs at 384 DPI. The reference-scale flag affects only diagnostic fixture
generation, not the benchmarked export options.

The final implementation passed 112 SVG tests in the default build and 112 with `media3d`,
strict all-target/all-feature SVG Clippy, three independent validator tests, formatting and the
API/convention checks. [Final validation logs and the source-stamped summary](../../samples/SvgExport/performance/validation/summary.json)
record the boxed implementation's gates. The correctness report records the broader Skia/compositor validation
and the controlled Fontconfig environment used for the full Skia suite.
