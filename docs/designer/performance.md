# Designer infrastructure review: performance evidence

These measurements cover individual engines on one Linux x86-64 development environment with
Rust 1.89.0. They are microbenchmarks of the workloads below, not application frame-rate or
cross-platform latency guarantees. Correctness tests run separately; timing is not a CI gate.

## Document and syntax engine

The baseline is `0ca7b20386d60c8f0a1847e96326e419713b98d6`. Both variants use the same benchmark
source, pinned dependencies, release optimization level 3, thin LTO, debug information disabled,
incremental compilation disabled, and CPU affinity 0. Five sequential samples follow one warm-up
operation. Input cloning and result destruction are included; constructing each test fixture is
outside the measured operation. The document has 20,000 child controls and 1,277,853 source bytes.

| Operation | Baseline median | Review median | Baseline / review |
| --- | ---: | ---: | ---: |
| Parse and drop the document | 52.308 ms | 41.716 ms | 1.25× |
| Change one attribute, reconcile IDs and compute changes | 163.562 ms | 89.491 ms | 1.83× |
| Submit an identical source buffer | 175.756 ms | 0.054 ms | 3,278× |
| Find the syntax node at a source offset | 280.566 µs | 1.714 µs | 164× |
| Parse a markup extension with 2,000 named arguments | 3.336 ms | 0.518 ms | 6.44× |

The identical-buffer path compares bytes and returns an unchanged revision without parsing,
reconciling or building history. Hit testing descends ordered child spans rather than scanning
every node. Attribute edits still validate a complete parsed document, but contiguous attribute
batches, pinned identity fast paths and an ordered change merge reduce repeated work. The markup
parser walks a cursor and bounds nesting depth; it no longer rescans each remaining argument list.

Run the checked-in harness with:

```sh
cargo bench --locked -p ferroui-designer-core --bench document_engine
```

`FERROUI_BENCH_NODES` and `FERROUI_BENCH_SAMPLES` increase the workload above its default
20,000 controls and five samples. To compare an older checkout, use the same harness and Cargo
profile against that checkout's core crate; the harness only calls APIs available in the baseline.
[Raw samples and exact profile](performance/core-review.json) include the compiler version and
harness digest used for the comparison.

## Source coordinates and batched edits

The source-engine harness uses a repeated `aα🦀` line containing 48,000 Unicode scalars and
112,000 UTF-8 bytes. It performs 32,000 byte/UTF-16 coordinate queries, then 16,000 non-overlapping
replacements. Values are medians of five process runs, using Rust 1.89.0 and release thin LTO
for both the same baseline commit and the reviewed code.

| Operation | Baseline median | Review median |
| --- | ---: | ---: |
| Build the initial line index | 0.096 ms | 0.255 ms |
| Complete all coordinate queries | 926.523 ms | 3.612 ms |
| Apply the replacement batch | 1,033.595 ms | 4.469 ms |

Sparse Unicode checkpoints trade a slightly more expensive initial index for bounded local
coordinate scans. Incremental editing splices affected blocks and coalesces small neighbors,
including repeated insertion into one line. A batch validates all source ranges before copying
unchanged slices once into the destination. CR, LF, CRLF, supplementary Unicode scalars and
invalid UTF-16 positions have independent correctness coverage.

```sh
cargo run --locked --release -p ferroui-designer-language --example source_engine_benchmark
```

[Raw samples](performance/language-review.json) also include smaller 3,000- and 12,000-scalar
workloads. Initial-index cost is reported explicitly because it increased.

## Pointer snapping

The reusable `SnapIndex` sorts and deduplicates measured peer edges and centers once per drag.
Each pointer update examines only neighboring guides for each moving anchor. It preserves the
one-off `SnapSettings::snap` tie-breaking and validation behavior, including live changes to grid
size and zoom tolerance. Native design-surface gestures now retain this index and their selection
bounds rather than rebuilding both for every pointer movement.

The snapping harness compares the two public APIs in the same optimized process. With 10,000
peer rectangles, preparing the index took 1.553 ms. Across five alternating samples, 2,000 queries
took a median 470.817 ms through the one-off API and 0.378 ms through the prepared index.
Every query result is checked for equality before timing. Preparing an index only pays off when
the geometry is reused; one-off callers can continue to use the original API.

```sh
cargo bench --locked -p ferroui-designer-tools --bench snapping
```

[Raw samples](performance/snapping-review.csv) contain the preparation cost and every timed run.
The sorted arrays use memory proportional to the number of peer rectangles. A gesture also pins
its immutable syntax snapshot and revision so it cannot commit into an unrelated or changed
document that happens to contain the same numeric node IDs.

## Replication engine

A separate comparison executable links baseline and reviewed collaboration crates against the
same reviewed document core. This isolates the collaboration changes from the AST improvements.
Both variants therefore use the same Rust 1.89.0 release build, optimization level 3, no LTO and
16 codegen units. Three alternating samples run in one process. These illustrative workloads
exercise causal admission, cached contiguous version summaries and local transaction projection.

| Operation | Baseline median | Review median |
| --- | ---: | ---: |
| Merge operations from 4,000 distinct writers | 95.550 ms | 9.442 ms |
| Read the version vector 2,000 times after 4,000 single-writer operations | 675.277 ms | 0.129 ms |
| Apply 500 local edits to a 100-control document | 159.211 ms | 67.853 ms |

[Raw samples and profile](performance/collaboration-review.json) record the comparison setup.
The checked-in `cargo bench -p ferroui-designer-collaboration --bench replication` harness can
measure the current engine under the repository's release profile. It does not use the standalone
comparison executable's profile, so its absolute timings should not be mixed with this table.
Network latency, persistence and UI-thread preview costs are outside these measurements.

## Runtime UI work

Hierarchy and property models are presented through TreeDataGrid. Property control templates are
created for realized cells, and hierarchy selection can update without replacing the row model.
These changes remove the previous eager construction of one button or editor subtree for every
entry. Native render and interaction tests verify the controls and authoring behavior; no UI
throughput claim is inferred from the portable engine measurements above.
