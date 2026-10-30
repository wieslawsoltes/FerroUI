# Selector title ordering reference

This test-only .NET 10 probe evaluates the original DockControl selector's
`StringComparer.OrdinalIgnoreCase.Compare` expression. It records 21 string pairs
as UTF-8 hex and comparison signs, including case equivalence, punctuation,
non-expanding casing, Greek final sigma, embedded nulls, prefixes and supplementary
characters versus BMP characters. It does not compile DockControl, the selector UI
or any engine/host, and does not establish behavior on older runtimes or strings
containing unpaired UTF-16 surrogates (Rust strings cannot contain those).

Run from this directory so the local SDK selection overrides the parent pin:

```sh
dotnet run --project SelectorReference.csproj --verbosity quiet > ../../../tests/Dock/data/selector-title-order.tsv
```

The native comparer uses the already captured .NET 10 simple-uppercase scalar map
from scripts/dock/workspace-reference. It compares the resulting Unicode scalars, not
UTF-16 units: the probe shows a supplementary scalar sorts after a BMP scalar in
the original ignore-case comparer. Rust full uppercase can expand scalars and
change case equivalence; neither full uppercase nor UTF-16 ordinal sorting is an
appropriate substitute here. Normal Cargo tests read the fixture without .NET.
