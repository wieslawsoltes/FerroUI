# Original selection lookup reference

Run from this directory:

```sh
dotnet run -- ../../../tests/Dock/data/selection-overlay-lookup.tsv
```

The harness links the unchanged original `SelectionOverlayHelper` and
`SelectionOverlayAdorner`, with actual Dock native controls, ReactiveUI factory and
model through project references. It does not copy the lookup algorithm or reflect
over private members. Avalonia Headless is pinned to 12.1.3; .NET is selected by
`global.json`. Normal Rust tests read the committed fixture without requiring .NET.

The 216 combinations cover six states in each visible/pinned/tab map pair: absent,
control only, root only, invalid root, valid control/root, and valid root with a
non-Control `IDockableControl`. The last case succeeds with a null control in the
original and must stop later fallbacks. Names identify the actual returned objects.

If the local NuGet configuration supplies a cache path without a trailing slash,
the original SourceLink targets reject it. Pass the same cache path with the trailing
slash as a global build property, for example
`dotnet run -p:NuGetPackageRoot=/path/to/package-cache/ -- ../../../tests/Dock/data/selection-overlay-lookup.tsv`.
No original source or build file needs editing.
