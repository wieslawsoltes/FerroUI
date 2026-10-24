# Native Charts captures

Generated on 2026-10-21 from `samples/ChartsDemo/MainView.xaml` and its Rust
commands. Each PNG is 1120×880 and was produced by FerroUI
`RenderTargetBitmap` with the Vello CPU backend, HarfBuzz and installed Linux
fonts. The host window is a test window; the drawing and text services are
real. A manual animation clock finishes native theme transitions before each
capture.

| File | Scenario |
| --- | --- |
| [light.png](light.png) | Six samples; light theme; vertical grouped bars; legends shown |
| [dark.png](dark.png) | The same shared controls and samples after switching to the dark theme |
| [live-mutation.png](live-mutation.png) | Seven samples after appending to the live lists; horizontal bars; legends hidden |

Regenerate from the repository root:

```sh
FERROUI_CHARTS_RENDER_OUTPUT=/tmp/ferroui-charts-render \
  cargo test --locked -p charts-demo --features render-smoke \
  charts_demo_renders_native -- --nocapture
```

The test writes `charts-light.png`, `charts-dark.png` and
`charts-live-mutation.png`. These reviewed copies drop the `charts-` prefix.
The smoke gate verifies that colored chart pixels exist and that theme and
data/orientation/legend changes alter the rendered output. It does not require
an exact image hash; available fonts can change glyph rasterization.

These are examples of the native FerroUI implementation. No reference image
from Microsoft's unpublished Charts implementation was available, so they do
not establish pixel parity with WinUI. See [the port record](../../../winui-charts.md)
for the public-contract mapping and verification boundary.
