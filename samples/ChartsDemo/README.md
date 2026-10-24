# ChartsDemo

The Charts public-contract port in a compiled-XAML FerroUI view. The default library is independent of any desktop backend and exposes `create_view()` and `ChartsDemo` for browser, mobile and desktop hosts. Rendering uses the Charts control and the framework drawing, text, resource and automation services.

```sh
cargo test -p charts-demo --lib
cargo run -p charts-demo --features desktop

# Optional offscreen verification with real Vello CPU rendering and HarfBuzz text.
FERROUI_CHARTS_RENDER_OUTPUT=/tmp/ferroui-charts-render \
  cargo test -p charts-demo --features render-smoke charts_demo_renders_native -- --nocapture
```

`MainView.xaml` is compiled by `ferroui-build` against the Charts metadata exported by its build script. The runtime-loader test loads that same document independently. The optional desktop feature installs the native FerroUI WinUI theme and opens the view in a window; another host can install its normal theme and set `Control::boxed(charts_demo::create_view())` as its content.

Four charts demonstrate line/area/grouped bar series, circle/diamond/square/triangle markers, dashed strokes, per-point immutable label/marker overrides, a date axis and shared categories. The line, area and bar controls share the same `Samples` objects. All four charts share the actual-values source. Buttons append to the existing `FerroList` sources, replace the source objects, switch light/dark resources, toggle legends and rotate bars. Replacement retains each shared `Samples` identity and subscribes to the new source.

This is a FerroUI integration sample for the audited public WinUI contract. It is not a copy of the unavailable internal WinUI ChartApp implementation. See [the source audit and full case mapping](../../docs/porting/winui-charts.md).

The optional render test writes `charts-light.png`, `charts-dark.png` and
`charts-live-mutation.png` using `RenderTargetBitmap`; it asserts colored chart
pixels and changes after native theme/data/orientation/legend updates. It uses
the real Vello CPU backend, HarfBuzz and installed fonts, with an in-memory
window host and a manual animation clock to finish theme transitions before
capture. These images verify FerroUI output, not pixel equality with the
unavailable Microsoft implementation.
