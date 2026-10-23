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

Framework tests, native application tests and the actual merged-source browser
build are separate pending gates at this checkpoint. The verified `08e29e4f`
runtime uses the preceding framework and does not establish a pass for this merge.
The sample's [validation record](../../../samples/FerroDesign/docs/VALIDATION.md)
tracks later results without erasing these source boundaries.
