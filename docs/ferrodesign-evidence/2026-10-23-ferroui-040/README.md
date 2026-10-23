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
preserves the exact observations. **Selected editor interactions on this
runtime remain pending**; build success is separate from runtime validation.

The full browser workflow at `c7dbf26f` now searches the independent browser-build cache
prefix, including the root lockfile hash, before its older fallback prefixes.
It also saves the target and its verified source-timestamp snapshot together
after build/artifact publication, before lengthy browser checks. The existing
cancellation/probe guards and safe partial-build behavior remain intact; no
later browser step runs Cargo. Assertions and the 120-minute job limit are
unchanged. This is a configured cache path and preservation change, with no
subsequent full-workflow cache hit or measured speedup claimed. The recorded
`1c` build had no source-timestamp manifest to restore and then snapshotted
10,468 tracked files. The later `c549` reuse was observed in the independent
workflow, not through this newly configured full-workflow cache bridge.

[The current source-delivery audit](source-delivery-c549/README.md) verifies the
tracked framework dependencies, embedded fonts, source links and all five
active Vello/no-Skia graphs. It records exact `c549` build-input equivalence to
the documentation checkpoint used for the initial audit. No independently
downloaded source archive was built for that audit.

The verified `08e29e4f` runtime uses the preceding framework and does not
establish a pass for this merge.
The sample's [validation record](../../../samples/FerroDesign/docs/VALIDATION.md)
tracks later results without erasing these source boundaries.
