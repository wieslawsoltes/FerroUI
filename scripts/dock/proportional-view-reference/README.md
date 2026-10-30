# Original proportional-view construction reference

This executable compiles the unchanged original `ProportionalStackPanelView.axaml`
and its code-behind, plus the actual proportional controls, with Avalonia/Headless/
Fluent 12.1.3. The original name generator supplies `InitializeComponent`; no view
or control implementation is copied or replaced. It asserts the source constructor's
`NullReferenceException`. The initial unhandled probe located the failure at source
markup line 50: the first child added to the null `ItemsControl.ItemsSource`.

Run from this directory so its SDK selection is used:

```sh
dotnet run
```

Expected output: `construction` followed by a tab and `NullReferenceException`.

The corresponding unchanged-structure Rust markup is rejected earlier by the shared
compiler with `FRN3000: XamlPropertyAssignmentNode: Add: not a declared member`.
The compiler finds the source `IList.Add` operation through `IEnumerable`, but its
native emitter cannot emit that interface member. Adding explicit collections makes
different behavior and is not the port of this view. The view remains pending until
the source operation and failure timing can be preserved; it is absent from the Rust
application. The original sample does not reference it during normal startup.

This reference is derived from Dock, copyright (c) Wiesław Šoltés, under the MIT
license in the FerroDock workspace root. Original C# and markup inputs stay unchanged.
