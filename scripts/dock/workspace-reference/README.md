# Workspace ordinal comparison reference

The source DockWorkspaceManager uses StringComparer.OrdinalIgnoreCase. FerroUI
System 0.3.0 has no corresponding string comparer. Rust full Unicode uppercase
can expand a character into several characters and would change ID identity.

This test-only .NET 10 program scans all Unicode scalar values and writes the
simple-uppercase mappings actually accepted by OrdinalIgnoreCase to the native
helper table. It independently captures 3,005 actual comparisons, including both
upper/lower candidates, dotless/dotted I, long S, Kelvin sign, sharp S, ligatures,
combining characters, supplementary characters, mixed strings and embedded nulls.
The ordinary Rust tests read the corpus without requiring .NET.

Regenerate from this directory so its SDK selection takes precedence over the
parent repository's exact pin:

```sh
dotnet run -- ../../../src/Dock/FerroDock.Model/workspace_ordinal_data.rs ../../../tests/Dock/data/workspace-ordinal.tsv
```

The checked-in data was produced by .NET 10.0.13 on macOS with default globalization.
The helper represents this runtime's ordinal equivalence for valid Unicode scalar
strings. CLR strings with unpaired UTF-16 surrogates are not represented by Rust
String. Equivalence across other ICU/NLS versions and globalization modes remains
unverified; inspect changes before regenerating. This is not culture-sensitive
folding, Unicode normalization, or a case-insensitive replacement for the manager's
separate exact active-ID removal check.

The implementation can move to FerroUI.System when that shared comparer exists.
See the [.NET ordinal implementation](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Private.CoreLib/src/System/Globalization/Ordinal.cs)
and [ordinal casing implementation](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Private.CoreLib/src/System/Globalization/OrdinalCasing.Icu.cs)
for the underlying runtime behavior.
