# Render tests

`tests/FerroUI.RenderTests` (crate `ferroui-render-tests`) is the port of upstream's render tests (`tests/Avalonia.RenderTests`, compiled for Skia by `tests/Avalonia.Skia.RenderTests`): scenes built in code are rendered offscreen and compared with upstream's expected images. It is the proof that the port draws what upstream draws.

## The harness and how it matches upstream

| Upstream | Port |
|---|---|
| `TestBase.cs` | `test_base.rs`: `TestBase::new(r"Shapes\Rectangle")`, `render_to_file`, `render_to_file_with_dpi`, `compare_images`, `compare_images_with` (`skipImmediate`, `skipCompositor`), `compare_images_no_renderer`, `test_font_family()` |
| `TestRenderHelper.cs` | `test_render_helper.rs` |
| `TestRenderRoot.cs` | `test_render_root.rs` |
| `ManualRenderTimer.cs` | `manual_render_timer.rs` |
| `CrossTestBase.cs`, `CrossUI/` | `cross_test_base.rs`, `cross_ui/` |
| `Mesa/` (software OpenGL and Vulkan of Linux and Windows) | not ported |

- **Two renderings of every scene**, as upstream: the immediate renderer (a `RenderTargetBitmap` of the size of the control at the DPI of the test, `<Name>.immediate.out.png`) and the compositor (a `CompositingRenderer` over a `TestRenderRoot` whose surface is a writeable bitmap of the default pixel format of the backend, with a manual render timer, one synchronous paint, `<Name>.composited.out.png`). The cross tests render through the compositor only (`<Name>.skia.out.png`).
- **The measure** is upstream's `TestRenderHelper.CompareImages`: the root mean square error over the four channels of all pixels, the colour channels multiplied by alpha, each channel in 0..1. **The threshold** is upstream's `AllowedError`, 0.022, for every output. Both images are decoded by the `png` crate, not by the render backend (upstream decodes with a library of its own too).
- **The services** are the ones upstream's static constructor installs: the Skia platform (the real font manager of the backend, on the CPU: the surfaces are raster, as upstream's), the standard asset loader, the HarfBuzz text shaper and a cursor factory that does nothing. No application, no theme: a test that needs a theme adds it to its control, as upstream's test does. The test fonts are the resources `resm:FerroUI.Skia.RenderTests.Assets?assembly=ferroui-render-tests` (the font files of `src/Skia/FerroUI.Skia/test_assets/assets`, the assets of upstream's render tests).
- **Differences**, none of which changes what is drawn or how it is compared: the services belong to the thread of a test (upstream: the process) and a test holds the dispatcher of its thread; the outputs are written under the build directory, never next to the expected images; every output of a comparison is measured before the test fails, so that the record of a run has all of them (upstream stops at the first); upstream's optional Mesa GL and Vulkan outputs (`gpuAllowedError`, `skipGpu`) do not exist.
- **Tests upstream runs on Windows only** (`Win32Fact`, `Win32Theory`: text in the default font of the system) are ported completely and carry `#[cfg_attr(not(windows), ignore = "<upstream's message>")]`. `#if AVALONIA_SKIA` code is in, `#if !AVALONIA_SKIA` code is out. `RuntimeInformation` conditions follow the macOS branch on macOS.

The expected images (`TestFiles/Skia`, `TestFiles/CrossTests`, `TestFiles/PixelFormats`: 360 files, 13 MB, unmodified PNG, JPEG and raw pixel files) and `Assets/` are upstream's; `NOTICE.md` of the crate has the attribution.

## Running

```sh
cargo test -p ferroui-render-tests                    # the Skia backend
cargo test -p ferroui-render-tests --features vello   # the Vello backend, CPU mode, same expected images
cargo test -p ferroui-render-tests -- shapes::path    # one suite
```

The suite takes 4 to 5 seconds on the Skia backend and about 14 seconds on the Vello backend (debug build, Apple M3 Pro, 393 test functions); the crate compiles in about 20 seconds once its dependencies are built. It is a member of the workspace, so the macOS job of the CI (`cargo test --workspace --lib --tests`) runs the Skia configuration; the Vello configuration is not run by the CI.

## Outputs and how to inspect a failure

The outputs go to `<build directory>/render-tests-output/<skia|vello>/` (the build directory is the one the test binary is in; `FERROUI_RENDER_TESTS_OUTPUT` names another place), in the layout of `TestFiles`: `Skia/Shapes/Path/Line_Absolute.immediate.out.png`, `.composited.out.png`, `CrossTests/Media/Geometry/<Name>.skia.out.png`. Nothing is written into the repository.

A failing test names its output files, their measured errors and the expected image:

```
.../render-tests-output/skia/Skia/Shapes/Path/Line_Absolute.immediate.out.png: Error = 0.0658...; ...composited.out.png: Error = 0.0658... (expected: .../TestFiles/Skia/Shapes/Path/Line_Absolute.expected.png)
```

Every comparison appends a line to `results.tsv` of the output directory (output file, error, `pass` or `fail`); `scripts/render_tests_report.py <results.tsv> [--failing] [--groups]` prints the last result of every output as a table. Delete the file before a run whose table is to hold that run alone. The tables of 2026-10-10 are `docs/porting/data/render-tests-skia.md` and `render-tests-vello.md`.

A failing test is a difference between the port and upstream. Look at the two images, find where they differ (bounding box, the pixels with the largest difference), and find the cause in the port against the upstream statement; the threshold is never changed. A difference that is not fixed at once is `#[ignore = "<the difference and the measured error>"]` and a row of the table below.

## Results on the Skia backend (2026-10-10)

393 test functions for 301 upstream tests (a theory is one test upstream and a function per row here): 370 pass, 23 are ignored by upstream's own condition, none is ignored for a difference of the port. 318 comparisons of an image with its expected image are made; the largest error is 0.0207.

| Ignored tests | Count | Reason |
|---|---|---|
| `Controls/BorderTests.cs`: `Border_Centers_Content_Horizontally`, `_Vertically`, `Border_Left_Aligns_Content`, `Border_Right_Aligns_Content`, `Border_Top_Aligns_Content`, `Border_Bottom_Aligns_Content` | 6 | `Win32Fact("Has text")` |
| `Controls/TextBlockTests.cs`: the tests with text in a system font, and the rows of their theories | 14 | `Win32Fact("Has text")`, `Win32Theory("Has text")`, `Win32Theory("Depends on the backend")` |
| `Media/GlyphRunTests.cs`: three of four | 3 | `Win32Fact("For consistent results")` |

Waived (`docs/porting/data/test-aliases.toml`, counted by `scripts/port-status/test_gaps.py` as project `Avalonia.RenderTests`: 310 tests, 0 missing, 9 waived): the eight tests of `Composition/OpenGlCompositionInteropTests.cs`, which need the software OpenGL of upstream's test project, and `Should_Properly_CloseFigure` of `CrossTests/CrossGeometryTests.cs`, which is not compiled into upstream's Skia render tests.

### Differences found and fixed

| Tests | Difference | Cause and fix |
|---|---|---|
| `Shapes/PathTests.cs`: `Line_Absolute`, `Line_Relative`, `HorizontalLine_*`, `VerticalLine_*` (6; error 0.048 to 0.067) | the line was drawn shifted | The path data ends in `M0,0M200,200` to give the shape its extent. Upstream reads `SKPath.TightBounds`, which for a path of lines are the bounds of all its points in the Skia upstream links; the Skia the port links leaves trailing moves out of the bounds. `tight_bounds` in `skia_sharp_extensions.rs` computes them from the points; every reader of tight bounds in the backend uses it. The Vello backend had the same difference for every path (`VelloPath::tight_bounds`) |
| `CrossTests/Media/ImageScalingTests.cs` (2; a panic) | "ImmutableBitmap has been disposed" | The control draws a bitmap it disposes at the end of its render. Upstream records `source.Clone()`, a counted reference; the port recorded the platform bitmap without its count. `DrawingContext::draw_bitmap` takes the counted reference and the render data holds one of its own |
| `Controls/PipsPagerTests.cs` (2; passing at 0.0111) | the characters of the two buttons were missing | The template binds the text of a text block to the content of the button with an indexer binding. Upstream publishes the value as an object and the property takes it if it is of its type; the port published it in the declared type of the source property. `IndexerBindingExpression` casts the value to the type of the bound property |

### Passing tests whose error is close to the threshold

Looked at one by one; none is a difference of the port.

| Tests | Error | What differs |
|---|---|---|
| `Media/DrawingContextTests.cs`: `Should_Render_LinesAndText` (composited) | 0.0207 | the expected image has sub-pixel (LCD) text, which Skia does not draw on macOS: the text is grey-scale |
| `Controls/BorderTests.cs`: `Border_Stretches_Content_Horizontally`, `_Vertically` | 0.0195 | the text asks for the family "Segoe UI", which macOS does not have: another font, three pixels higher |
| `Shapes/PathTests.cs`: `Line_Relative` | 0.0199 | the expected image of upstream is the one of `Line_Absolute` (the line ends at 190,10); the data of the test ends the line at 200,0, where the port draws it. Upstream's own output is as far from its expected image |
| `Controls/TabbedPageTests.cs`: `TabbedPage_WithIcons_TopPlacement`, `_BottomPlacement` | 0.0145 | the expected images show the icons as paths; since upstream's change of the icon template of a page a geometry given as an icon is content without a template and is shown as the name of its type, by upstream and by the port |
| `Shapes/PathTests.cs`: `Line_Absolute`, `GetWidenedPathGeometry_Line_Dash`, `Path_Tick_Scaled`; `Shapes/LineTests.cs`: `Line_1px_Stroke`, `_Reversed` | 0.012 to 0.018 | a diagonal line one pixel wide: the expected images spread it over three pixels, the Skia of the port draws it sharper |
| `Media/VisualBrushTests.cs`: `VisualBrush_InTree_Visual`, `*_NoTile`; `Media/ImageBrushTests.cs`: `ImageBrush_Should_Render_With_Transform` | 0.008 to 0.017 | the edge row of a rectangle at a half pixel: half coverage in the expected image, a quarter in the output, or the other way round |

## The Vello configuration

The same tests with `--features vello`: the harness installs the Vello platform in its CPU mode instead of the Skia platform; everything else is the same, the expected images included. The results are a table of the design document of the Vello backend (`docs/porting/vello-backend.md`, section 8, "Upstream's render tests") and `docs/porting/data/render-tests-vello.md`; nothing of the Skia configuration is ignored for them.
