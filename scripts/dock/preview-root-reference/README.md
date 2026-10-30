# Preview coordinate reference

This test-only probe runs the frozen `GetManagedPosition` body on actual Avalonia
12.1.3 Headless windows, the dependency pinned by the original repository. The generator
copies the original method body verbatim. Its `ManagedWindowLayer` parameter uses a
`Visual` type alias because the method calls only base Visual APIs; an actual Border
serves as the coordinate surface. No replacement dock/layer/engine is implemented.
The project compiles the original `VisualRootCompatExtensions.cs` directly, preserving
its `GetPresentationSource()?.RootVisual` result. It does not use the older root API.

From this directory, with an installed .NET 10 SDK:

```sh
python3 generate.py --check
dotnet run --verbosity quiet
```

The committed generated method must match the generator output. Package versions are
explicit and the local SDK selector uses the latest installed .NET 10 feature band.
Save the four-case TSV output to `../../../tests/Dock/data/preview-root.tsv` after reviewing
any change. Normal Rust tests read that fixture without requiring .NET.

On the pinned dependency, attached windows at render scales 1 and 2 and a hidden window
report `TopLevelHost`; the original direct TopLevel cast fails and the method returns
(0,0). A detached visual also returns (0,0). The Rust regression runs those four cases
against the actual native ManagedWindowLayer and compares root names/scales/coordinates.
This resolves the previously presumed Rust-specific root-coordinate difference for these
Headless cases. Preserve the original fallback. It does not establish full desktop,
managed-layer public Show/discovery, positive TopLevel-root or adorner/host parity.
The probe neither compiles the entire original DragPreviewHelper nor runs a C# sample.
