# Geometry reference values

This test-only program compiles DockPoint.cs and DockRect.cs directly from the
frozen C# source baseline. It captures the actual invariant-culture output used
by their ToString methods and checks their NaN/signed-zero value equality.

From the FerroDock directory, regenerate the fixture with an installed .NET 10
SDK:

```sh
cd scripts/dock/geometry-reference
dotnet run -- ../../../tests/Dock/data/invariant-double.tsv
```

Run from this subdirectory to use its SDK selection (10.0, latest installed
feature band) rather than the parent repository's stricter SDK pin. No UI or
NuGet package dependency is needed. Commit fixture changes only after checking
the source baseline and understanding any changed output.

The fixture covers signed zero, NaNs/infinities, the finite extrema, subnormal
values, both neighbors of positive and negative decimal powers, and 4,096
deterministic pseudorandom IEEE-754 bit patterns. The current fixture has 7,895
distinct values and was generated with .NET 10.0.13.

The normal Rust test run reads the checked-in TSV; it does not need .NET. This
test protects general-format exponent thresholds, exponent spelling, round-to-even
shortest digits and invariant separators. It does not establish geometry
serialization or end-to-end document/layout behavior.
