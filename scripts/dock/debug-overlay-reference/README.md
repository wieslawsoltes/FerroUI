# Original debug overlay reference

From this directory, run:

```sh
dotnet run -- ../../../tests/Dock/data/debug-overlay-render.tsv
```

The harness links the unchanged original adorner, helper, manager and extensions,
with the actual original native Dock control project. Avalonia Headless is pinned to
12.1.3. It captures 16 combinations of the original three area flags and hover state,
including rectangle order, colors, thickness and bounds from actual DrawingGroup
output. It also checks F9 toggling, dynamic DockControl discovery/removal and explicit
cleanup. Rust tests consume the fixture without a .NET dependency.

If SourceLink rejects a configured package-cache path without its trailing slash, pass
that same path as a global property:
`dotnet run -p:NuGetPackageRoot=/path/to/package-cache/ -- ../../../tests/Dock/data/debug-overlay-render.tsv`.
No original source/build file is changed. The harness copies no rendering or docking
algorithm and uses no reflection.
