# Native integration probe

This sample uses the ported RibbonUniformGridPanel and Core
adaptive layout algorithm, plus native RibbonItemContent presentation.

Main.xaml contains all view layout. ProbeViewModel uses ReactiveUI notifications;
LayoutPreview is a UI-independent service interface, SourceLayoutPreview invokes
the ported algorithm, and the application entry point wires the concrete service.
Markup metadata is separate from presentation logic. There are no application
action commands in this probe; the separate Headless test now exercises the native Core/ReactiveUI command bridge.

The first slider changes available width for three fixed group-width profiles.
The second changes the real grid's Columns property through compiled bindings.
Cells are native editable TextBoxes so edits and focus can be observed while the
grid reflows. These cells are test content, not replacement Ribbon input controls.
The Simple theme styles the framework controls only; original Ribbon themes have
not been replaced.

From `components/FerroRibbon`:

```sh
cargo run -p ferroribbon-native-probe
```

Desktop builds use the normal Skia/HarfBuzz/native backend.

The final presentation section edits a ReactiveUI label and toggles simplified
mode through compiled bindings to the actual ported RibbonItemContent. It uses the
source plain-character icon path, without requiring missing symbol-font assets.
The panel is passive presentation, not a replacement RibbonButton/command control.
Text splitting/cache and size formulas belong to the library. The classic/simplified
transition retains the original cache behavior.
