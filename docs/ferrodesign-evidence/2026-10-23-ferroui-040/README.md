# FerroUI 0.4.0 integration

The continuation merges upstream `934b9ce16e2ada890ee31534f1a98808f483a14f`
into branch checkpoint `0f0732784bd6fdf1cb2477a61dd332d9a007fec2`.
Both parents are retained. The sample lockfile changes only the 26 local
FerroUI package versions; external dependencies and checksums are unchanged.

[Local gate metadata](integration-gates.json) and the [complete core log](core-tests.log)
record **533 passed / 0 failed**, **4.24 s**, with Rust **1.99.0**. Five active
application target graphs contain FerroUI Vello and exclude Skia. The graph
hashes identify the exact local audit; the independent browser build publishes
its full active graph with its runtime manifest.

The additive conflicts preserve both paragraph test modules and both mock
drawing-context fields. Integration review adds `TextLineBounds` forwarding
through styled paragraphs and conservative native GPU fragment abandonment
before target-bounded layers or non-default fills close outer clips. Seven
GPU fragment regressions require a compute adapter in native feedback CI.

The merged source at `1c7151f5` subsequently passed
[core CI](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194410)
with **533/533 tests in 2.41 s**. Its
[generated-reference workflow](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194402)
passed every generator and drift check: all five generated Rust/XAML files
match the committed SHA-256 hashes and the generated patch is empty.
[CI evidence](ci/) preserves exact source metadata and decoded logs.

Native feedback at `1c7151f5` was rejected by YAML parsing before any job
started. The `gpu::fragment_tests::` filter appeared in a plain scalar command;
`2d6284d9` changes it to a literal block. All five FerroDesign workflows parse
after that fix. The corrected
[native run 39950436412](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950436412/job/120403288368)
at `2d6284d9` **passed**: **239/239 application tests in 26.03 s**, all
framework filters (**8 unhinted / 17 Base paragraph / 8 line-bounds / 16 Controls
paragraph / 174 TextBox / 5 rich-history**), and **7/7 mandatory GPU fragment
tests in 7.14 s**. None failed or was ignored; overlapping framework counts are
not additive. This run precedes the later hover/pressed diagnostic checkpoint.

The subsequent exact `c549f6bc`
[native job 120410483091](https://github.com/wieslawsoltes/FerroUI/actions/runs/39952473854/job/120410483091)
also **passed completely**: **239/239 application tests in 43.60 s**, all six
other framework filters, and **7/7 mandatory GPU fragment tests in 9.09 s** with
`FERROUI_VELLO_REQUIRE_GPU=1`. All eight invocations had zero failed and zero
ignored tests. The native projection regression includes the new hover/pressed
fields. Its core source tree is identical to the recorded 533-test `1c` tree;
this native job did not rerun core tests. [Native evidence](native-c549/README.md)
preserves exact source/tree, outcomes and the complete decoded log.

The `1c7151f5`
[browser build job 120403095053](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194298/job/120403095053)
completed the real WebAssembly build step in **28 min 17 s**. The downloaded
artifact contains a verified **53,785,479-byte WASM** and **16 payload files
plus its provenance manifest**. Archive digest/size, ZIP CRCs and entry paths,
every payload hash/size, exact source/toolchain, Vello/no-Skia graph and local
resource/module closure passed verification. Seven modules were parsed,
including generated inline snippets and an explicit check of dynamic
`storage.js`. [Browser-build evidence](browser-build-1c/README.md) preserves the
manifest and complete build log. No editor interaction or performance run was
made against this `1c` binary.

The exact `c549f6bc`
[build job 120415321864](https://github.com/wieslawsoltes/FerroUI/actions/runs/39952473917/job/120415321864)
then passed with a **10 min 10 s** build step. Its delivered **53,785,830-byte
WASM**, **17 payload files plus manifest**, source/toolchain identity and
complete archive/resource/module closure were verified. The copied web host,
relative favicon link and SVG bytes match exact `c549` source. The independent
workflow restored **10,459 matching source timestamps** and skipped **nine
changed files**, then rebuilt the application. [Current build evidence](browser-build-c549/README.md)
preserves the exact observations. The subsequent [selected editor replay](browser-runtime-c549/README.md)
completed **50 Software2D / 38 WebGL2 checks but failed overall**. Software2D
gradients passed all 17 groups; both modes failed the newly reached polygon,
first-preedit and guide checks, and WebGL2 failed a later gradient insertion.
The original reports, exports, screenshots and diagnoses remain unchanged.

The separate [strict text-pan benchmark](browser-text-pan-c549/README.md)
passed **36 normal plus six separate trace observations per mode**, with empty
error lists and graceful process exit. Its long timing tails remain recorded;
an exact trace join shows a two-callback settled marker 829.460 ms before its
matching display swap completes. Neither presentation observation nor settling
is a display-completion or FPS measurement.

The [polygon reference diagnosis](polygon-coverage-c549/README.md) reproduced
the 212 direct-raster edge differences, found zero against the existing
converged area-coverage method, and rejected a deliberately one-pixel-shifted
control. The [double-click correction record](input-corrections-c549/README.md)
retains the real input semantics and unchanged native 500 ms threshold.
Published harness checkpoint `0aab9d7c` also uses actual native zoom so guide
fixtures respect the existing screen-space attraction radius. All 53 helper
checks passed; new editor replay remains a separate gate.

The [full native c7 workflow](full-native-c7/README.md) passed 533 core and 239
application tests, launcher compilation and configured public API/pixel gates.
Its production matches c549 and does not cover the new first-preedit defect.

The [first-preedit native checkpoint at 4d3](native-4d3/README.md) compiled but
**failed three new caret/selection/template tests**. All 13 new functions ran;
ten passed, including styled Unicode first-clause/cancellation/history and
reentrant clause/restoration cases. Its full TextBox substring filter passed
184/187, while the separate 239 application and seven required GPU fragment
tests passed. The [matching browser build](browser-build-4d3/README.md)
passed archive/source verification with a 53,794,092-byte WASM and 17 payloads;
it was not replayed. The distinct `dc361996` correction preserves the failed
expectations, synchronizes provisional Text before native caret movement,
and adds a five-case reentry regression. Its source review is not an execution
pass; the validation record distinguishes later results.

The [completed sequential c7 browser job](full-browser-c7/README.md) was
**canceled after its 120-minute budget with failed and incomplete coverage**.
It continued WebGL2 work through guide bindings; six later scenarios were not
reached and no final browser report was produced. The last retained guide
capture completed its guarded retry before cancellation. The evidence does
not support a cleanup-stall diagnosis or certify unreported scenario passes.

This run did exercise the full-workflow cache bridge: it restored the
independent c549 build cache and **10,464 matching source timestamps**, skipped
**five** changed inputs, and completed its browser build step in **9m54s**.
The build artifact, source snapshot and target-cache save completed before
interactions. The exact cache keys, terminal job, incomplete coverage, capture
timings and lossless log remain in the c7 evidence. Successful build reuse is
separate from browser correctness and does not establish a controlled speedup.

[The current source-delivery audit](source-delivery-c549/README.md) verifies the
tracked framework dependencies, embedded fonts, source links and all five
active Vello/no-Skia graphs. It records exact `c549` build-input equivalence to
the documentation checkpoint used for the initial audit. No independently
downloaded source archive was built for that audit.

The verified `08e29e4f` runtime uses the preceding framework and does not
establish a pass for this merge.
The sample's [validation record](../../../samples/FerroDesign/docs/VALIDATION.md)
tracks later results without erasing these source boundaries.
