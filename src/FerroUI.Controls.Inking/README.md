# ferroui-controls-inking

The inking controls of FerroUI, a port of the inking controls of WinUI
(`controls/dev/InkCanvas` and `controls/dev/InkToolbar` of microsoft-ui-xaml, commit
`7e4b0c2096be402f51cc8f7d203ebf67cff183d0`):

- `InkCanvas` and its `InkPresenter`: pen, mouse and touch input, wet ink, erasing, selection,
  the ruler and protractor stencils and custom drying through `InkSynchronizer`;
- the stroke services: `InkStroke`, `InkStrokeBuilder`, `InkStrokeContainer`, the portable
  FERROINK format and the clipboard (`InkClipboard`);
- `InkToolbar` with its pen, eraser, stencil and custom buttons and the pen configuration flyout;
- on Windows, `WindowsInkServices`: the ink services of the system (ISF and GIF with ISF, the
  system ink clipboard, handwriting recognition).

Register the types of the crate once, before markup names them:

```rust
ferroui_controls_inking::register_types();
```

## The WinUI styles of the toolbar

The control themes of the toolbar are part of this crate, not of the WinUI theme. An application
on the WinUI theme that shows the toolbar includes them after the theme, in `App.xaml`:

```xml
<Application.Styles>
  <WinUITheme />
  <StyleInclude Source="ferres://FerroUI.Controls.Inking/Themes/WinUI.xaml" />
</Application.Styles>
```

The documents are compiled with the crate. With the feature `document-assets` they are also
embedded as assets, for markup that the run-time loader loads and that includes them.

The port, its source mappings, its deviations and its tests are described in
`docs/porting/winui-inking.md` in the repository.
