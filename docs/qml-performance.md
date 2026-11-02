# Measuring QML performance

These Linux measurements compare the same synthetic applications with revision `df705afb` and the optimized implementation. Each pair uses the same workload, build profile and machine. The rows measure different operations and must be read independently. A real application's gains depend on its object graph, bindings, delegate reuse, JavaScript and native layout.

## Compiler and engine

| Operation | Before | After |
|---|---:|---:|
| Resolve and link 256 composite instances | 3.41 ms | 1.45 ms |
| Generate Rust for those instances, including linking | 147.5 ms | 26.1 ms |
| Generator peak RSS for full output | 41,040 KiB | 15,032 KiB |
| Size of generated Rust | 1,231,698 bytes | 790,960 bytes |
| Instantiate a small root with a 128-type registry | 188.4 µs | 2.2 µs |
| Construct a 256-child tree | 649 µs | 427 µs |
| Incubate and publish the 256 children | 923 µs | 691 µs |
| Reevaluate 512 dependent bindings | 416 µs | 248 µs |
| Deliver a signal to one interested receiver among 513 Connections | 37.0 µs | 0.98 µs |
| Retarget one receiver among 514 Connections | 5.11 µs | 0.75 µs |
| Reevaluate 512 bindings with the profiler enabled | 346 µs | 303 µs |

Compiler timings are medians of nine warmed release runs. Code generation includes emitting the QML IR and native evaluators. The emitted module still goes through rustc. In five alternating dev builds of that Rust module, median rustc CPU time fell from 3.150 s to 2.664 s, while rustc’s own peak RSS rose from 490,664 KiB to 506,492 KiB. Compiler validation can now skip Rust formatting and output generation. Module lookup caches last for one load, so new builds still discover changed files.

The runtime comparison used pinned optimized binaries and medians of three runs, each containing seven timed batches. Construction shares immutable registry data. Transactions journal changed slots and update dependency edges when reads change. The profiler benchmark uses a separate feature build, so its absolute times should only be compared within that row.

## Models, native layout and clocks

| Operation | Before | After |
|---|---:|---:|
| Change a role unrelated to section grouping in 8,000 rows | 1,539 µs | 3.53 µs |
| Change the grouping role of one row | 1,891 µs | 285 µs |
| Attach and detach a typed 8,000-row model before layout | 2,504 µs | 14.1 µs |
| Scroll a retained 20-row viewport by one pixel | 60.1 µs | 20.6 µs |
| Scroll far enough to recycle delegates | 286 µs | 261 µs |
| Append and remove a row with 300 eager Repeater delegates | 1,102 µs | 542 µs |
| Adjust an anchor margin among 302 native peers | 1,163 µs | 8.22 µs |
| Tick one active timer alongside 2,000 stopped timers | 9.45 µs | 0.48 µs |

These pairs measure CPU time on the simulation thread. Both revisions' QML and QML Controls code used optimization level 3 with the same native dependencies. The comparison alternated baseline and candidate processes on one CPU, used ten warmups, then took the median of seven blocks in each process and the median of three process results per revision. The checked-in portable harness reports elapsed time. On a busy host the 400-track active-animation case showed variable elapsed times, so it does not establish a general animation throughput gain.

A single role edit can update neighboring section boundaries. Structural changes and new grouping rules can still require a rebuild. Models with unknown or null-only roles retain a discovery path. Incremental native layout retains a full validation path for topology changes and custom controls.

## JavaScript, browser and editor

| Operation | Before | After |
|---|---:|---:|
| Load an additional root with 32 already-installed 8 KiB modules in native WebScene | 34.8 ms | 0.114 ms |
| Warm V8 method call in a 1,024-property scope, no host reads | 6.06 ms | 0.395 ms |
| Same scope, one host property read | 6.44 ms | 0.478 ms |
| Check a browser subscription after an unrelated model edit | 2.78 ms | 0.131 ms |
| Publish a nested edit in the watched model | 5.29 ms | 1.40 ms |
| Local hover with 800 local and 800 inherited properties | 16.0 ms | 0.066 ms |
| Go to an inherited declaration in the same fixture | 15.6 ms | 0.115 ms |
| Return full completion results for the same fixture | 27.2 ms | 9.91 ms |
| Apply a one-character edit and publish diagnostics | 12.0 ms | 8.96 ms |

The V8 round-trip comparison uses one dev-built Rust client, alternately paired with the previous worker and the descriptor-capable worker. It includes the process boundary and validates every return value. Measured warm evaluation requests for the wide scope fell from about 43 KB to 237 bytes. Scope and compiled-function descriptors are private to a worker generation. The sender uses 128 replaceable program slots with a 64 KiB signature limit per slot. Nested calls and larger signatures send the script in full. A restart resends them. Unrecognized workers use the full protocol. Small scopes with a host read showed overlapping timings in this comparison.

The subscription fixture runs compiled Wasm in Chromium with 256 rows and 256 nested models. It takes nine warmed medians over 20 updates and excludes application startup. The editor fixture drives a dev-built server over real stdio and includes UTF-16 edits, diagnostics and response decoding. The reported pairs average two process medians per revision in alternating order on the same CPU set. The editor retains parsed disk components up to 16 files and 2 MiB of source text. It checks the current content on every lookup, including when a buffer is closed or a file is replaced with equally long text.

A separate 12,000-sibling stress test compares the source indexes with the first editor optimization in this round (`4804e0e3`). With the editor crate optimized against identical dependencies, median warmed sibling definition fell from 85 to 44 µs and lookup inside a method with 12,000 reads from 93 to 46 µs. On this workload, elapsed time to open the file ranged from 471 ms before to 553 ms after in one comparison, while process CPU varied by less than 3% in a separate comparison. These results support faster repeated queries. Cold file opening and first-query latency remain sensitive to parsing and semantic index construction.

## Follow-up measurements against `65304945`

This pass changes how the compiler represents repeated objects, how the runtime finds references during destruction, and how the native adapter applies model edits. Each pair below compares the same fixture against `65304945` on Linux.

| Operation | Before | After |
|---|---:|---:|
| Generated Rust for 256 composite instances | 790,960 B | 381,523 B |
| Downstream dev rustc CPU, full rlib code generation | 3.01 s | 0.99 s |
| Downstream dev rustc peak RSS | 518.5 MiB | 232.2 MiB |
| Materialize and drop the compiled component | 249 µs | 187 µs |
| Create and drop its 257-object instance | 1.83 ms | 1.81 ms |
| Create and retire a child beside 8,192 scalar properties | 103.6 µs | 2.24 µs |
| Insert and remove a section in the middle of 8,000 rows | 8.11 ms | 63 µs |
| Append and remove a section at the end of the same model | 3.97 ms | 58 µs |
| Reorder siblings with 320 unrelated native peers | 855 µs | 9 µs |
| Transfer a child between two panels in that tree | 831 µs | 44 µs |
| Page a mounted ListView with delegate pooling | 1.21 ms | 141 µs |
| First hover in a file with 12,000 sibling items | 451 ms | 121 ms |
| First definition inside its method with 12,000 reads | 0.088 ms | 37.2 ms |
| Subsequent definitions inside the same method | 0.060 ms | 0.061 ms |
| Warm V8 method call with 1,024 properties, no host reads | 315 µs | 290 µs |
| The same call with one host property read | 415 µs | 386 µs |

The generator emits typed records for common members and shares a metadata factory across objects with identical shapes. The public component still materializes owned IR. Timing the generator itself gave 27.39 ms before and 27.76 ms after. The downstream rustc measurements are medians of five alternating pairs with the same runtime rlib at optimization level 0 and debug info disabled. Component and instance timings use thirteen alternating release pairs pinned to the same core.

The runtime discovers reference-bearing slots on first retirement, then tracks writes, aliases and binding targets in declaration order. The create-and-retire fixture measures repeated retirement and includes creation on every iteration. The first retirement still visits the slot arena, and later retirements scale with the number of candidate slots. Across three alternating release runs a write to an unrelated slot took 481–482 ns, and constructing the separate 256-child tree took 441 µs before and 459 µs after.

ListView now receives the changed row range. It preserves the unaffected prefix of the section index and updates the boundaries at the edit. A long move still scans the moved interval, and shifting a suffix still costs time proportional to the number of affected boundaries. The native adapter validates edits to stable child collections together, so it can move a child in either update order. Lifetime changes, reentrant updates and custom hosts retain the full validation path. The controls rows use simulation-thread CPU time with identical native dependencies, ten warmups and three alternating runs per revision.

The editor indexes declarations once per document revision. For complex scripts, the first hover or definition inside the script resolves its JavaScript names and caches them. This moves the cost of resolving the 12,000-read method to its first navigation. Warm sibling hovers took 0.060 ms before and 0.065 ms after. These stdio timings use six alternating pairs of pinned server processes with `--properties 12 --siblings 12000 --iterations 21`.

The native WebScene host now uses one I/O thread per child for a bounded request/reply exchange. It waits once per frame under the original deadline, including when a host callback reenters the worker. WorkerScript cancellation also interrupts the owner's wait when the child has stopped reading its pipe. The V8 measurements use eight paired runs of the dev client on two dedicated cores, swapping affinities between runs, with the same worker executable and native library. In the eight-property scope the no-read call took 322 µs before and 297 µs after, and a single read took 382 µs before and 354 µs after. Elapsed times varied more when the clients competed with unrelated builds.

## Reproduce

Run these commands from the workspace root. For comparisons with an earlier revision, use the same harness and Cargo bench target declarations in both checkouts. Keep build flags, toolchain, CPU affinity and background load consistent. The measurements above used rustc 1.99.0.

```sh
cargo bench --locked -p ferroui-qml-build --bench compiler
cargo bench --locked -p ferroui-qml --bench runtime
cargo bench --locked -p ferroui-qml --bench runtime --features diagnostics
cargo bench --locked -p ferroui-qml-controls --bench visual_collections

cargo build --locked -p ferroui-designer-language --features qml --bin ferroui-qml-lsp
python3 scripts/qml/bench-editor.py target/debug/ferroui-qml-lsp
python3 scripts/qml/bench-editor.py target/debug/ferroui-qml-lsp --properties 12 --siblings 12000 --iterations 151
node scripts/qml/bench-channel.mjs
```

Configure the native WebScene library and worker as in the [QML setup guide](qml.md#modern-javascript-with-webscene-v8). To reproduce the worker comparison, change `FERROUI_WEBSCENE_WORKER` between runs of the same client.

```sh
cargo run --locked -p ferroui-qml-webscene --example root_catalog_bench
cargo run --locked -p ferroui-qml-webscene --example bridge_bench
```

For browser timings, build and package the `qml-browser-tests` Wasm application using the Chromium job in the [conformance workflow](../.github/workflows/qml.yml). Run its subscription benchmark against that site.

```sh
python3 tests/QmlBrowser/run.py --benchmark-subscriptions --chrome google-chrome --site /path/to/site
```

To measure the downstream compiler separately, save each revision's generated Rust module. Use the same benchmark harness in both checkouts. The commands below assume Cargo's default target directory. For a paired comparison, point both rustc invocations at the same current runtime build.

```sh
FERROUI_QML_BENCH_OUTPUT=/tmp/generated.rs cargo bench --locked -p ferroui-qml-build --bench compiler
cargo build --locked -p ferroui-qml
/usr/bin/time -v rustc --edition=2021 --crate-name qml_fixture --crate-type rlib \
  -C opt-level=0 -C debuginfo=0 -L dependency=target/debug/deps \
  --extern ferroui_qml=target/debug/libferroui_qml.rlib /tmp/generated.rs -o /tmp/libqml_fixture.rlib

cargo build --locked --release -p ferroui-qml
FERROUI_QML_BENCH_MODULE=/tmp/generated.rs rustc --edition=2021 -C opt-level=3 \
  -L dependency=target/release/deps --extern ferroui_qml=target/release/libferroui_qml.rlib \
  scripts/qml/bench-generated.rs -o /tmp/qml-consumer
/tmp/qml-consumer
```
