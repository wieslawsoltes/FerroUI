# Task: the colour picker library

Branch: `color-picker`. Pull request title: `Controls: the colour picker library`.

Upstream `src/Avalonia.Controls.ColorPicker` (46 source files, about 7,800 lines, and 12 theme documents) has no counterpart in the port. It blocks `Pages/ColorPickerPage.xaml` and the colour picker styles of `App.xaml` in the ControlCatalog (`samples/ControlCatalog/excluded.txt`, `temporary.rs`, `GAPS.md`) and the `ColorSpectrumAutomationPeerTests` of the core port. Rows 19 and 23a of `CRITICAL-PATH.md` are the rows this task advances; `docs/porting/tracking/Avalonia.Controls.ColorPicker.md` is its tracking page.

Do, one commit per item:

1. A new crate `src/FerroUI.Controls.ColorPicker` (package `ferroui-controls-color-picker`), a workspace member, laid out as the other library crates are (`register_types.rs`, `NOTICE.md`, markup metadata). Port every source file exactly, file by file: the controls (`ColorPicker`, `ColorView`, `ColorSpectrum`, `ColorSlider`, `ColorPreviewer`), the palettes, the converters, the helpers and the automation peers.
2. The theme documents of the library for both themes, converted as the other theme documents are (`scripts/convert_theme_xaml.py`, the sync scripts of the themes), wired the way upstream wires them (the application includes the styles of the library; they are not part of the base theme).
3. The upstream tests of the library, under upstream's names, and `Automation/ColorSpectrumAutomationPeerTests.cs` of the controls tests (6 tests), which `CONTINUATION.md` lists as blocked on this crate.
4. The ControlCatalog: take `Pages/ColorPickerPage.xaml` off `excluded.txt`, restore the colour picker styles of `App.xaml` that `temporary.rs` drops, port the page class, and update `GAPS.md`. The sample then depends on the new crate.
5. Update the tracking page, rows 19 and 23a of `CRITICAL-PATH.md`, and `CONTINUATION.md`.

Colour conversion code that already exists in `ferroui-base` (`media/color.rs`, `hsl_color.rs`, `hsv_color.rs`) is used, not duplicated. Another worker is porting `Media/ColorTests.cs` on the branch `core-port-12` and may fix those files: do not change them here; if a test of this task needs a fix there, leave the test in place, say so in the pull request, and rebase once that branch has merged.

End the pull request description with a "Continuation" section.
