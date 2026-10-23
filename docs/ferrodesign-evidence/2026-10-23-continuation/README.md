# FerroDesign continuation, 23 October 2026

This evidence distinguishes the shipped source, completed native/core checks,
and browser work that still needs exact-source execution. It does not establish
complete Figma compatibility or matched Figma visual parity.

## Published checkpoints

| Commit | Change |
| --- | --- |
| [`3be16cb7`](https://github.com/wieslawsoltes/FerroUI/commit/3be16cb735c9636673fc43e95692503f7cac0144) | Coordinated padding Escape discard, missing read-only native control diagnostics, and explicit connected-browser/process shutdown ownership |
| [`dfab0ed9`](https://github.com/wieslawsoltes/FerroUI/commit/dfab0ed9a0146a892b40d2db0590b0193c80f672) | Native radial/angular gradient handles and six typed auto-layout spacing bindings, with exact transaction/override ownership |
| [`1637a093`](https://github.com/wieslawsoltes/FerroUI/commit/1637a093723a4f899b7f4e371fd53f0b91914a98) | Atomic prototype spacing reflow/rollback, browser input/coverage/crop corrections, and early browser artifact publication with verified source timestamp reuse |
| [`08e29e4f`](https://github.com/wieslawsoltes/FerroUI/commit/08e29e4f530c0ae65f6e2ac072f381605e23ed0c) | Independent browser-build feedback and passive angular-input expectations; the production core/application source is unchanged from `1637a093` |

All application rendering continues through public FerroUI APIs and Vello.
The active target dependency checks reject Skia. Browser Canvas2D SVG decoding
in the independent comparison is a test oracle, not the editor's renderer.

## Completed checks

[`completed-gates.json`](completed-gates.json) records exact source IDs, run/job
URLs, decoded-log byte counts and SHA-256 hashes. The linked excerpt files
retain selected original line numbers and result text. Full decoded CI logs
were inspected; the excerpts do not pretend to contain the entire logs.

| Source | Gate | Result |
| --- | --- | --- |
| `3be16cb7` | [Native feedback](https://github.com/wieslawsoltes/FerroUI/actions/runs/39941952822/job/120375277944) | **230/230 application**, **27.27 s**; five framework filters passed **8 / 15 / 16 / 174 / 5** |
| `3be16cb7` | [Full-workflow native job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39941952796/job/120375278062) | **511 core / 230 application**, application **19.43 s**; native compiler check, configured public text/geometry/compositing gates and all five Vello/no-Skia target graphs passed |
| `dfab0ed9` | [Core](https://github.com/wieslawsoltes/FerroUI/actions/runs/39942848092/job/120378251111) | **528/528**, **1.72 s** |
| `dfab0ed9` | [Native feedback](https://github.com/wieslawsoltes/FerroUI/actions/runs/39942848091/job/120381540661) | **238/238 application**, **28.42 s**; same five framework filters passed |
| `1637a093` | [Core](https://github.com/wieslawsoltes/FerroUI/actions/runs/39944253726/job/120382892496) | **533/533**, **2.50 s**, including all five prototype-spacing regressions |
| `1637a093` | [Native feedback](https://github.com/wieslawsoltes/FerroUI/actions/runs/39944253747/job/120382892411) | **239/239 application**, **43.57 s**, including clipped selection/radius-control priority and Present button diagnostics; same five framework filters passed |
| `1637a093` | Local locked core suite, Rust 1.99.0 | **533/533**, **5.91 s**, including all five new prototype-spacing cases |
| `1637a093` | Local pure JavaScript suites | **34/34** coverage arithmetic, input observations/radius oracle, trace analysis and browser/process ownership cases |
| `08e29e4f` | Local pure JavaScript suites | **39/39**, including five additional angular-input cases for inverse-affine movement, seam accumulation and stop interpolation |

The local core execution preceded connector publication. The local commit and
published commit have the same verified Git tree,
`30d4ef3b1db917318a11163e7d92aa493b3d0203`. Core tests do not execute native
controls or browser rendering. The framework filters overlap and must not be
added together as unique test counts.

The native feedback at `dfab0ed9` executed five new radial/angular tests and three
spacing-variable UI tests. It predates the later prototype integration, native
clipped-corner control-priority regression and Present button diagnostic.
The subsequent complete `1637a093` core/native jobs cover those changes.

## Recovered browser baseline and corrections

The actual `b7b0b885` WebAssembly build is from
[run 39869066118](https://github.com/wieslawsoltes/FerroUI/actions/runs/39869066118).
Its complete report was recovered from the official validation artifact.
[`b7-browser-outcome.json`](b7-browser-outcome.json) preserves that report's
digest, scope, per-scenario status/check count and complete primary errors.
The overall report is **failed**.

| Presentation mode | Completed checks | Passed scenarios | Failed scenarios |
| --- | ---: | ---: | ---: |
| Software2D | 161 | 28 | 9 |
| WebGL2 | 155 | 27 | 10 |

Each mode requested all 37 then-existing scenarios. Partial checks within a
failed scenario remain partial, and ordinary common gates also contribute to
the completed-check count. Ten linear-gradient checks passed in each mode;
that binary does not contain the new radial/angular handles or spacing bindings.

The recovered failures drove specific corrections:

- Padding, guide, export and modal tests were missing read-only names in the
  native diagnostic map. The underlying controls remain real FerroUI controls.
- Fractional radius/width input was rounded before native delivery. The revised
  expectations use bounded, trusted browser input with independent affine
  geometry, preserving the original numeric tolerances and exact history checks.
- The clipped selection probe overlapped a newly available radius handle. The
  new point is still inside the rounded cutout and explicitly clear of native
  radius/resize targets; a native regression verifies both priorities.
- A text selection's conservative 448×208 paint bounds were mistaken for a
  440×200 crop. The paragraph export now uses an explicit clipped artboard;
  authored text and complete history checks are retained.
- The 8×/16× independent SVG reference failed its own four-alpha convergence
  gate. The fixed 16×/32× strip experiment retains both convergence and native
  PNG thresholds and bounds explicit raster/readback allocations. Its exact
  source, inputs and scalar results are in the
  [browser harness evidence](../2026-10-23-browser-harness/README.md).

Those changes remain subject to real editor execution. Old-runtime replays and
isolated decoder checks are not substituted for a current-source browser pass.

The later [prototype-motion investigation](../2026-10-23-browser-harness/prototype-motion.md)
preserves the original failing CI trace and exact b7-runtime replay scope. The
corrected harness requires actual source-frame pixels before timed input and
records bounded progress samples from the existing rAF wait. The original
2,000 ms duration, `(0.2, 0.8)` midpoint range and 10-second timeout remain
unchanged. The focused WebGL2 replay passed all three motion checks; an isolated
Software2D module replay also passed, while its normal shared-gate run remained
blocked in text-entry preparation. Those are separate, explicitly scoped results.

The [native text-entry evidence](../2026-10-23-browser-harness/native-text-entry.md)
records a requested double-click whose native press timestamps were 636 ms
apart, beyond the existing 500 ms window. The unchanged timestamp path preserves
DOM event age; the historical trace does not contain the DOM timestamps. The
common harness keeps its first real double-click assertion and records passive
input evidence. Later text/IME setup selects the actual text layer and uses the
public native Enter command, preserving all text, composition and history gates.
The focused b7-runtime WebGL2 replay passed **26/26** checks: 16 common gates,
seven radius checks and three responsive/persistence gates. Its first native
double-click was 183 ms apart, compared with 182.9 ms in the passive DOM record;
all five later native Enter reopens passed. It does not validate the new
application features or replace a complete two-mode browser suite.

## Current native document-engine measurement

The existing release benchmark ran at `08e29e4f` with **10,000 nodes**, Rust
1.99.0, locked/offline dependencies and one build job. Its core tree is identical
to `1637a093`; source files were unchanged during execution. All paired document,
history, component-ID and serialization equivalence assertions passed.
The [complete result and provenance](performance/README.md) retain the workload,
sample counts and original measurements.

| Operation | Scoped path p95 | Whole-document reference p95 |
| --- | ---: | ---: |
| Nudge | 0.1250 ms | 34.9949 ms |
| Undo | 0.0878 ms | 16.7329 ms |
| Redo | 0.0813 ms | 8.6996 ms |
| Gesture update | 0.0023 ms | 11.7940 ms |

The dense row layout containing 9,999 text children measured 14.50 ms at its
five-sample maximum/p95. Compact serialization of the 12,802,055-byte document
measured 69.06 ms at its five-sample maximum/p95. These are native CPU-engine
measurements on a shared host. They exclude browser input, native-control
layout, artwork rendering and presentation, and establish no frame-rate claim.

## Build cache and performance boundary

The `dfab0ed9` native job restored 9,846 exact source files and skipped 20 with
changed content. Its application compilation took **41.49 s**, followed by the
**28.42 s** test execution recorded above. The previous correction job's cache
had no source manifest and therefore made a conservative miss. These are
observed CI runs with different cache/source conditions, not a controlled
build-speed comparison or an editor runtime benchmark.

The browser workflow now applies the same content-verified source timestamp
helper, runs its filesystem and changed-input Cargo probes, and snapshots only
after source/build activity finishes. A successful WebAssembly/static-import
build publishes the complete distribution immediately. Browser interactions,
pixels, persistence and text-pan measurements retain their separate gates.

The substantial historical WebGL2 automation/presentation tail is not claimed
to be fixed. No source-inspection, unit-test or cache-reuse result establishes
display FPS, physical-GPU speed or end-to-end parity with Figma.
