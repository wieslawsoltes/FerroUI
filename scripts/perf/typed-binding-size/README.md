# Sharing typed binding code across models and values

This experiment changes one FerroUI.Base source file in a disposable checkout. The Catalog retains its normal XAML, Skia backend, runtime metadata hooks and production Cargo profile.

## What landed

PR #1023 landed a narrower form of this experiment. It shares the cold operations of a binding: start, stop, disposal, the data context change and error logging, with source resolution in `SourceSlot<S>` and the unboxed cache in `TypedValueSlot<V>`. The path that produces a value stays compiled for each expression, from the change notification of the source to the sink.

The experiment's candidate shared that path too. On the benchmark gate it took 6.3% more instructions for each value a typed binding produces and 2.6% more for its setup, so it was not landed in that form.

Stripped macOS executables in MB, PR #1022 and PR #1023 together against `44a5f9094`. PR #1022 (the casts of compiled markup) was not measured alone, so these figures are not the saving of the binding change by itself.

| Application | `release` | Saved | `dist` | Saved |
| --- | ---: | ---: | ---: | ---: |
| Control catalog | 75.25 -> 74.25 | 1.00 | 61.29 -> 60.24 | 1.05 |
| WinUI theme gallery | 33.92 -> 33.79 | 0.13 | 26.82 -> 26.67 | 0.15 |
| TodoMvc | 22.79 -> 22.70 | 0.09 | 18.73 -> 18.67 | 0.06 |
| Hello world | 18.49 -> 18.46 | 0.03 | 15.62 -> 15.60 | 0.02 |

Measured the same way against the same baseline, the two PRs with the experiment's candidate saved 2.16 MB of the catalog in `release` and 1.72 MB in `dist`.

## Measured Catalog results of the experiment's candidate

Everything from here on describes the candidate that shared the value path as well, on its own Catalog revision. **These are not the figures of what landed.**

| Binding implementation | Complete executable bytes | Typed binding instructions | Total machine instructions |
| --- | ---: | ---: | ---: |
| Stock | 65,859,504 | 4,514,656 | 51,347,192 |
| PR #496 tip binding file | 65,132,000 | 3,887,880 | 50,624,376 |
| Separate model and value operations | 63,839,040 | 2,684,244 | 49,303,800 |

The experiment's candidate saves **2,020,464 bytes (3.07%) against the matched stock build**, including **1,292,960 bytes beyond the PR-tip binding file**. The relocatable constant section grows by 55,904 bytes against stock. The archived 65,876,016-byte Catalog has identical stock instruction totals. Its extra 16,512 file bytes come from one 16,384-byte text-segment page and 128 bytes of link/signing data. The only section-length difference is 2,066 C-string bytes.

On the linked 64-pair fixture, stock measures 3,643,008 bytes, PR tip 3,610,016, historical controller 3,460,576 and candidate 3,394,432. That is 66,144 bytes beyond the older controller on this probe. Restoring per-pair source subscription replacement increases the candidate fixture to 3,410,960 bytes.

The reporting unit for notifying i32 propagation is nanoseconds per change. The uninstrumented 50,000-update probe gave stock 29/37 and candidate 31/38 for source-to-target/target-to-source across 31 randomized rounds. After four million warmup updates, one-million-update windows gave stock 42/51 and candidate 45/57. Short-window medians varied across campaigns with host scheduling. The allocation-counted probe recorded zero allocations over 50,000 changes in either direction on every arm. The first attach used ten allocations on each. The candidate expression occupies 200 bytes for this model, up from 192. The candidate passed 681 binding-related tests, including hot reload. Matched 1900×1360 ListBox, Data Validation and TreeDataGrid screenshots each had zero differing pixels.

The desktop startup and navigation runs use fresh native Skia/Metal Catalog processes with the WinUI Light theme. In 31 alternating pairs, process spawn to the after-setup theme-variant marker measured 163.13 ms stock and 166.42 ms candidate. The median paired ratio was 1.0164 with a 95% bootstrap interval of 0.9583–1.0464. The marker is emitted before the main window is constructed. In 11 further pairs the first TreeDataGrid navigation to the UI Loaded event measured 18.421 and 19.188 ms, with revisits at 17.200 and 17.480 ms. The other pages moved in both directions. The Loaded endpoint observes UI readiness.

A separate seven-pair frame run recorded 2,342 stock and 2,294 candidate draws during the same page sequence. The median of per-process median draw times was 8.832 ms stock and 8.833 ms candidate. The median of per-process 95th percentiles was 9.012 ms and 8.997 ms. The paired p95 ratio was 0.997 with a 95% bootstrap interval of 0.971–1.032. The built-in compositor timer brackets the wall time of successful `render_frame()` calls after the first page selection, which includes any backend wait within that call. Display presentation and physical input latency require different instrumentation.

The exact binary allocations and hashes are in [results.json](results.json). [Raw microbenchmark samples](timings.json), [desktop summaries](desktop-results.json) and [desktop samples](desktop-samples.json) retain the separate workloads and process pairs.

## Source comparisons

| Arm | Binding implementation on ControlCatalog revision `933fd4632` |
| --- | --- |
| Stock | Unmodified source at that revision |
| PR tip | Only `data/core/typed_binding_expression.rs` from `8678e15b8` |
| Historical controller | Only the same file from `86159449b` |
| Separate model and value operations | The PR-tip file plus `controller.patch` |

PR #496 tried an erased target lifecycle controller in `86159449b`, removed it in `dcabf29ca`, then shared narrower target helpers in `b18def031`. Commit `484189ae2` split untyped owner lookup and value conversion by the one type each uses. The separate [Linux editable WinUI report](https://github.com/wieslawsoltes/FerroUI/blob/8678e15b8ae738931237c0e70ef66f61a6ecbfd2/docs/compiled-theme-shared-runtime.md) measures larger cohorts. An earlier combined observable and target lifecycle trial reduced the full Linux image from 28,324,712 to 27,534,728 bytes while slowing boxed-style notification and read by 39.8%. The PR does not isolate the typed controller’s contribution to either change. The boxed-style workload measures a different delivery path from these notifying typed properties.

The candidate combines the PR-tip helpers with the historical controller design. It also moves source resolution, weak-handle replacement and notification subscription into `SourceSlot<S>`. The strong source handle returns to the expression for the initial read or write, and the caller keeps its owned data context through that call. `TypedValueSlot<V>` owns the unboxed cache, cached-value reads, conversion of the target value, getter-error handling and cache replacement. A model-specific accessor still takes the original typed source and value. No additional heap object is needed to hold either slot. The public `TypedBindingExpression<S, V>` retains its concrete type identity.

The previous metadata-removal experiment measured 28 → 126 ns source-to-target and 36 → 137 ns target-to-source on its notifying integer workload. Its size and timings are in [the compact-spike results](../../themes/compact-spike/results.json).

## Reproduction

Use macOS arm64, Xcode command-line tools, the repository's Skia build prerequisites and a checkout containing the PR commits. See the recorded toolchain in `results.json`. Run from the repository root. Supply a new output directory on a volume with room for a complete Catalog build and its intermediate files.

```sh
python3 scripts/perf/typed-binding-size/build.py /path/to/new/output
python3 scripts/perf/typed-binding-size/sample.py /path/to/new/output > /path/to/new/output/timings.json
python3 scripts/perf/typed-binding-size/sample.py /path/to/new/output --warm > /path/to/new/output/warm-timings.json
python3 scripts/perf/typed-binding-size/catalog_perf.py startup /path/to/new/output/stock /path/to/new/output/axes /path/to/new/output/startup --pairs 31
python3 scripts/perf/typed-binding-size/catalog_perf.py navigation /path/to/new/output/stock /path/to/new/output/axes /path/to/new/output/navigation --pairs 11
python3 scripts/perf/typed-binding-size/catalog_perf.py frames /path/to/new/output/stock /path/to/new/output/axes /path/to/new/output/frames --pairs 7
```

The driver creates a detached worktree, compiles each version with the same `dist` profile, records the Apple link map and locates the exact Cargo-built Base library for the standalone probes. The linker map option collects measurements. The production build configuration is identical across arms. The default command also builds a historical-controller Catalog for further investigation. The recorded historical comparison here uses the 64-pair probe. The optional `--arms value_only` ablation applies `without-source-slot.patch` to retain the shared value operations while putting model subscription replacement back in each pair.

`cohort.rs` constructs 64 concrete expressions across 16 distinct model types and four property value types. The complete file size measures the resulting linked probe. `map_families.py` counts only live instructions whose demangled name begins in the typed binding module. Generic drop code, read-only data and padding affect the complete file separately.

`sample.py` shuffles the process order within each round with a fixed seed. After three warm-up rounds it records every sample from the uninstrumented notifying i32 propagation probe and the separate allocation-counted lifecycle probe. Each process checks two-way updates. Lifecycle measurements alternate two distinct data contexts and repeatedly attach and dispose a binding. Using `--warm` gives each process four million warmup updates and times one million more per direction. The allocation counters cover 50,000 changes per direction and 10,000 iterations per lifecycle operation.

`ownership.rs` replaces the target's data context from inside the old model's getter. It records callback order, strong-reference counts during and after reentry and the final value of the outer read. It checks subsequent writes to the new model and source disconnection on disposal. The driver also runs the candidate's binding-related Base tests with development hot reload enabled.

## Related areas in PR #496

| Catalog allocation at the stock revision | Prior implementation and evidence | Remaining experiment |
| --- | --- | --- |
| 4,429,040 bytes in the property store and styled/direct properties | Demand-rooted coercion and inheritance plus restored typed boxed-style delivery saved 426,648 bytes in the PR's matched Linux x86-64 editable WinUI executable. The current branch already carries related coercion, inheritance and state sharing in `a0d3bcfab`, `e6a202ce5`, `b67f642f1` and `2e36aa428`. Later `c137e8a55` specialized writes for registered coercers. | Review the PR's separate `5268d4dc2` borrowed typed write and notification frames against the current source. The PR tip points a source-validation workflow at that commit. It contains no matched shipping result for those frames. The stock Catalog's 364 `set_and_raise_core` bodies occupy 539,960 instruction bytes in total. |
| 7,997,836 bytes of generated XAML instructions | The PR's matched Linux editable WinUI comparison of metadata receiver preparation, 18 whole ControlTheme programs and 34 Style-region programs saved 115,656 file bytes and 167,568 instruction bytes. Current HEAD already has related metadata preparation in `1598dfe8a` and per-theme control tables in `4f1fa3b24`. | Measure the remaining Catalog page/template bodies and incremental regions against the existing tables under one link configuration. |
| Rust-generated call adapters and destruction distributed across the instruction categories | The PR's Linux stage-4 symbol census attributed 3,107,916 bytes to callable entries and trait shims and 1,022,527 bytes to compiler-generated destruction, including associated unwind data. HEAD already emits compile-time class vtables and shares several metadata adapters. | Resolve the macOS Catalog's remaining shims and drop glue to their owning typed values. The PR proposes per-module descriptors for built-ins plus typed leaves for ownership and invocation, preserving dynamic registration for user types. These gross Linux categories have no measured macOS saving. |

The exact full Linux production pairs and their correctness/performance limitations are in the [shared runtime report](https://github.com/wieslawsoltes/FerroUI/blob/8678e15b8ae738931237c0e70ef66f61a6ecbfd2/docs/compiled-theme-shared-runtime.md). The separate mechanism census and proposals are in the [Rust architecture report](https://github.com/wieslawsoltes/FerroUI/blob/8678e15b8ae738931237c0e70ef66f61a6ecbfd2/docs/compiled-theme-rust-architecture.md) and [size architecture report](https://github.com/wieslawsoltes/FerroUI/blob/8678e15b8ae738931237c0e70ef66f61a6ecbfd2/docs/compiled-theme-size-architecture.md).
