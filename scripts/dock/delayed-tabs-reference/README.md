# Original delayed tab layout reference

Compiles the unchanged original `DelayedUniformTabPanel.cs` and its actual
`VisualRootCompatExtensions.cs` helper with Avalonia/Headless 12.1.3. Captures desired
extent and every child's x/width for 24 layouts: empty/single/multiple tabs, finite
and infinite measurement, fractional rounding, minimum/maximum constraints,
negative spacing, hidden children and child minima. No panel body or formula is
copied into this harness, and no reflection is used.

Regenerate from this directory so its SDK selection applies:

```sh
dotnet run -- ../../../tests/Dock/data/delayed-tab-layout.tsv
```

The committed corpus records .NET 10.0.13. Ordinary Rust tests do not require .NET.
Separate native tests run real asynchronous dispatcher timers for close-burst delay,
restart and viewport expansion, and use the actual DoubleTransition/CubicEaseOut.
The corpus itself covers geometry without animation or a visual root; broader
animation-frame and attached DPI behavior must be validated separately.

Derived from Dock, copyright (c) Wiesław Šoltés, under the MIT license in the
FerroDock workspace root. Original source files remain unchanged.
