# The children of a border with a box shadow in a render target bitmap of more than 96 DPI (October 2026)

**Decided** (the owner's standing policy, 2026-10-15: a defect of upstream is fixed in the port and recorded as a deviation, as for the popup and the native control host). The fix of section "The smallest correct fix" is in the Skia backend (`restore_canvas` in `src/Skia/FerroUI.Skia/drawing_context_impl.rs`) and recorded in `DEVIATIONS.md` (Skia backend); the acceptance test below is turned on and the test that asserted upstream's behaviour is removed (pull request 195). The rest of this page is the analysis as it was prepared for the decision.

## What was seen

The screenshot run of the ControlCatalog on macOS (scaling 2, a window of 1100 by 800; `FERROUI_SMOKE_SCREENSHOTS`, pictures in `<workspace>/ferroui-screenshots/macos-gl/catalog-metal/` and `catalog-opengl/`) showed, on the Buttons page, the selected navigation item "Buttons" as an empty rounded box at its place, and its content (the selection bar, the icon and the text) twice as large and away from its place, down and to the left, over the item "Slider". The same page at scaling 1 (the Linux framebuffer, DRM and headless VNC runs) is correct.

The item is a `SelectableButton` with the theme `SelectableItemButton` of the sample (`samples/ControlCatalog/CustomThemes.xaml`): selected (and under the pointer), its `Border` `PART_LayoutRoot` gets a background and the box shadow `NavigationItemShadow`. Nothing of the selection is animated; the transform that goes wrong is the one of the drawing context.

## Which picture is wrong, and which is right

The pictures of the run are drawn by `save_frame` of `samples/ControlCatalog.Desktop/program.rs`: a `RenderTargetBitmap` of the size of the client area in pixels and 96 times the render scaling as its DPI, into which the window is rendered (`RenderTargetBitmap::render`, the immediate renderer). That is not the path of the frames the window presents: those are drawn by the compositor on the render thread.

The screenshot run writes a second picture of every page since this investigation, `NN-<header>-composed.png`: a snapshot of the composition visual of the window at its render scaling (`Compositor::create_composition_visual_snapshot`), which the render thread draws from the same server visuals and through the same drawing code as the frames it presents. On macOS with Metal, at scaling 2:

| Picture | The selected item "Buttons" |
|---|---|
| `01-buttons.png` (render target bitmap of 192 DPI) | an empty box at its place; its content at twice its size and twice its distance from the origin of the window |
| `01-buttons-composed.png` (the compositor) | correct: the box with its shadow, the bar, the icon and the text in it |

Pictures: `<workspace>/ferroui-screenshots/macos-render-fixes/before/` (`01-buttons-navigation-bitmap-vs-composed.png` puts the navigation panes of the two side by side). **The window shows the item correctly; only a render target bitmap of more than 96 DPI is wrong.**

## What the host test shows

`src/Headless/FerroUI.Headless/unit_tests/render_scaling_tests.rs` (the headless platform with the Skia backend, as upstream's headless tests render): a window of 200 by 100 with a rounded `Border` with a box shadow at (10, 10), and in it a red `Border` of 20 by 10 at (40, 30) of the window.

| Case | Where the red pixels are | Test |
|---|---|---|
| The frame the window renders at scaling 2 (`capture_rendered_frame`, the compositor) | (80, 60) to (120, 80): in place | `shadowed_border_child_is_in_place_in_the_frame_at_scaling_2` (passes) |
| A render target bitmap of 192 DPI (400 by 200 pixels) | (160, 120) to (240, 160): twice as large, twice as far | `shadowed_border_child_is_scaled_twice_in_a_render_target_bitmap_of_192_dpi_known_difference` (asserted upstream's behaviour before the fix; removed with it) |
| The same, as a fix would have it | (80, 60) to (120, 80) | `shadowed_border_child_is_in_place_in_a_render_target_bitmap_of_192_dpi` (ignored before the fix, when it failed with the bounds above; passes with it) |

## The cause

1. A render target bitmap draws through a drawing context that scales to its DPI: `RenderTargetBitmapImpl` makes its `FramebufferRenderTarget` with `useScaledDrawing: true`, and the `DrawingContextImpl` of a scaled target keeps the scale apart as `_postTransform` (`Matrix.CreateScale(dpi / 96)`). Its `Transform` setter stores the logical transform in `_currentTransform` and gives the canvas `value * _postTransform`.
2. `DrawRectangle` with a box shadow saves the canvas, clips, draws the shadow and restores the canvas with `RestoreCanvas`, which sets `_currentTransform = null`. The canvas has the same matrix as before, but the logical transform is forgotten.
3. The `Transform` getter then reads it back from the canvas: `_currentTransform ??= Canvas.TotalMatrix44.ToAvaloniaMatrix()`. The matrix of the canvas includes `_postTransform`, so from now on the context reports the logical transform times the DPI scale.
4. The immediate renderer pushes a transform for every child of the border (`ImmediateRenderer.Render`, `context.PushTransform(transform)`), and `PlatformDrawingContext.PushTransformCore` computes it from the getter: `_impl.Transform = matrix * _impl.Transform`. The setter multiplies by `_postTransform` again: the children are drawn with the scale applied twice. When the border's own transform is popped, the transform saved before the shadow is set again, so the siblings that follow are in place.

At a DPI of 96 there is no post transform and nothing changes, which is why the pictures at scaling 1 are correct. The frames of a window are not affected because the compositor never reads the transform of the platform context: its `CompositorDrawingContextProxy` keeps the transform it reported and the one it gave the context itself (`_reportedTransform`, `_effectiveTransform`), and applies its own post transform, and the render targets of windows draw unscaled (`ScaleDrawingToDpi = false`).

Any restore of the canvas inside a draw call, or a pop followed by a push before the transform is set again, does the same in a scaled context. The two box shadow loops of `DrawRectangle` are the ones the controls meet; the immediate renderer itself always pops a transform after it pops a clip.

## The port against upstream, file by file

| Upstream (tracked commit `17350180`) | Port | Result |
|---|---|---|
| `src/Skia/Avalonia.Skia/DrawingContextImpl.cs`: `_postTransform` from `ScaleDrawingToDpi` (lines 202 to 207); `RestoreCanvas` sets `_currentTransform = null` (lines 687 to 691); the getter of `Transform` reads `Canvas.TotalMatrix44` (line 875); the setter multiplies by `_postTransform` (line 888); `DrawRectangle` restores the canvas after every shadow (lines 472 and 515) | `src/Skia/FerroUI.Skia/drawing_context_impl.rs`: `post_transform` from `scale_drawing_to_dpi`; `restore_canvas` stores the matrix of the canvas (`local_to_device`) as the transform at once (line 395), which is the value upstream's getter computes later; `transform()` returns it (line 1131); `set_transform` multiplies by the post transform; `draw_rectangle` restores the canvas after every shadow | The same values |
| `src/Skia/Avalonia.Skia/RenderTargetBitmapImpl.cs`: `new FramebufferRenderTarget(this, true)` (line 18) | `render_target_bitmap_impl.rs`: `FramebufferRenderTarget::from_render_target(.., true)` | The same |
| `src/Avalonia.Base/Media/PlatformDrawingContext.cs`: `PushTransformCore` reads `_impl.Transform` (line 82) | `src/FerroUI.Base/media/platform_drawing_context.rs`, `push_transform_core` | The same |
| `src/Avalonia.Base/Rendering/ImmediateRenderer.cs`, `RenderTargetBitmap.Render` | `src/FerroUI.Base/rendering/immediate_renderer.rs`, `render_target_bitmap.rs` | The same |
| `src/Avalonia.Base/Rendering/Composition/Server/DrawingContextProxy.cs`: the reported and the effective transform kept by the proxy (lines 41 to 70) | `src/FerroUI.Base/rendering/composition/server/drawing_context_proxy.rs` | The same; why the window is correct |

**The port matches upstream.** Upstream draws a border with a box shadow and children into a `RenderTargetBitmap` of more than 96 DPI the same way.

## The smallest correct fix

In the Skia drawing context, report the logical transform after a restore: the matrix of the canvas with the post transform taken off.

```rust
fn restore_canvas(&mut self) {
    let canvas = self.canvas.get();
    canvas.restore();
    let total = matrix44_to_matrix(&canvas.local_to_device());
    self.current_transform = Some(match self.post_transform {
        Some(post_transform) => total * post_transform.invert(),
        None => total,
    });
}
```

Verified on macOS (26.6, Metal, scaling 2): with it the ignored acceptance test passes and the known-difference test fails, as they should, the tests of the Skia backend and of the headless platform pass otherwise, and the render target bitmap of the catalog matches the compositor's picture (`<workspace>/ferroui-screenshots/macos-render-fixes/after-fix-branch/`: of the 3.5 million pixels of the Buttons page, 6,440 differed between the two pictures by more than 40 in a channel before the fix and 287 after it, about what the TextBlock page, without a selected item in view, differs by in both runs: 243). The post transform is a scale by the DPI over 96, so it is always invertible. Upstream's form would be the same correction in the getter (`Canvas.TotalMatrix44.ToAvaloniaMatrix() * _postTransform.Value.Invert()`). The Vello backend keeps the logical transform itself and never reads it back from a canvas, so it is not affected and needs nothing.

What it costs, and why it is a decision:

- **It is a deviation from upstream**, to be recorded in `DEVIATIONS.md` (Skia backend, Behaviour): a correction of an upstream fault, as the shared snapshot of a surface is. Upstream has the fault at the tracked commit.
- It changes what `IDrawingContextImpl::transform` returns after a restore in a scaled context, which only code holding the platform context directly could observe (the lease of the Skia API restores the canvas matrix itself, not the transform).
- The alternative that changes nothing in the framework: the screenshot run writes the compositor's picture (`-composed`) and is read from that. That is already there, but it leaves the fault to every application that renders a visual into a render target bitmap at a DPI other than 96 (a printout, an export at a higher resolution, the pictures of the catalog), which is how upstream behaves.

## Recommendation (taken)

Take the fix as a recorded deviation: a render target bitmap at the DPI of the screen is the ordinary way to capture a control on a high-density display, and its output is wrong with any box shadow in the tree. The `-composed` pictures of the screenshot run stay: they are what the window shows, whatever the bitmap does.
