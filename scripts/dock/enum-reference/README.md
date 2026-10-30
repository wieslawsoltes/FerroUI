# Original enum display reference

Links the actual frozen source `Dock.Model/Core/DockingWindowState.cs` and
captures its runtime `Enum.ToString()` for every valid flag combination and
unknown positive/negative values. No copied enum or runtime type discovery is
used. Regenerate from the Cargo workspace with:

```sh
cd scripts/dock/enum-reference
dotnet run -- ../../../tests/Dock/data/docking-window-state-display.tsv
```

Normal Rust tests read the committed corpus without requiring .NET. This harness
is derived from Dock, copyright (c) Wiesław Šoltés, under the MIT license in the
workspace root. The linked original C# file remains unchanged.
