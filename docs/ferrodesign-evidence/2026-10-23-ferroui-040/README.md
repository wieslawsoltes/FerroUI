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
not additive. Actual browser execution and the later hover/pressed diagnostic
checkpoint retain separate gates.
The verified `08e29e4f` runtime uses the preceding framework and does not
establish a pass for this merge.
The sample's [validation record](../../../samples/FerroDesign/docs/VALIDATION.md)
tracks later results without erasing these source boundaries.
