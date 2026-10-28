# Canvas navigation and designer icons

The designer keeps canvas navigation separate from application layout and source editing.
Pan, zoom, fit and wheel preferences change the local view of the artboard. They do not
author transforms, resize controls, add undo entries or send collaboration edits.

## Mouse and keyboard

The default wheel behavior follows Figma: the wheel pans the canvas and Ctrl/Cmd+wheel
zooms around the pointer. The zoom controls also expose a Blend-style wheel preference.
The preference changes the meaning of the wheel; it does not change document content.

| Input | Wheel pans, the default | Wheel zooms, the Blend-style option |
| --- | --- | --- |
| Wheel or two-axis scroll | Pan along the reported axes | Zoom around the pointer |
| Ctrl/Cmd+wheel | Zoom around the pointer | Pan |
| Shift+vertical wheel | Pan horizontally | Pan horizontally |
| Middle-button drag | Pan in either Select or Interact mode | Pan in either Select or Interact mode |
| Space+left-button drag | Temporarily pan | Temporarily pan |

Fractional wheel deltas are retained. A small trackpad or high-resolution wheel delta
produces a correspondingly small change instead of being rounded to a complete notch.
Zoom is anchored at the pointer, including when it reaches a supported zoom limit.
The artboard point beneath the pointer stays in the same viewport position.

The persistent **Pan** tool on the canvas shelf also pans with the left mouse button.
Choose **Select** (`V`) or **Interact** (`I`) to leave it. The existing `H` shortcut
continues to toggle layout helpers.

The canvas keyboard commands are:

| Command | Shortcut |
| --- | --- |
| Fit the artboard | Shift+1 |
| Fit the current selection | Shift+2 |
| Actual size, 100% | Ctrl/Cmd+0 |
| Zoom in / out | + / - |

Zoom ranges from **1% to 1600%**. Fit selection uses the measured native bounds of the
selected authored controls. Fit artboard uses the artboard dimensions and the current
measured canvas area. Both retain room for the canvas shelf and center the fitted bounds,
subject to the supported zoom range.

Open **…** beside **Fit** for the zoom options. Enter a percentage, optionally followed by
`%`, and press Enter or Apply. Invalid or out-of-range values display an inline error and
leave the accepted view unchanged. The same flyout provides actual size, fit artboard,
fit selection and the two wheel preferences. The percentage/reset button remains a quick
way to return to 100%.

## Input ownership and retained views

Navigation is scoped to the canvas. Wheel events over property editors, the source editor,
the timeline or other panels keep their own behavior. In **Interact** mode, unmodified
wheel input reaches application controls, so a preview `ScrollViewer` or list can scroll.
Ctrl/Cmd+wheel explicitly addresses the designer camera, using the selected wheel mode.
Middle-button drag remains available for navigation in Interact mode.

Canvas shortcuts and temporary Space panning do not take keyboard input from a focused
text editor or an interactive application control. Starting a camera operation retires an
unfinished authored canvas gesture. It cannot turn an earlier resize, drag or insertion
draft into a source edit under a different coordinate mapping.

An active pan captures its mouse pointer and checks the document, revision, preview
generation and definition scope before applying subsequent movement. Escape restores the
view from the start of the pan. Releasing the button accepts the current view; lost capture
or focus stops the gesture. These operations leave source drafts and undo history intact.

Each retained document tab stores its camera alongside the document and layer state.
Returning to that tab restores its pan and zoom. Entering a template/theme definition
starts a new view and retains the document camera for the return path. The wheel preference
belongs to the workspace. This state is local to the running editor and is not serialized
into XAML or collaboration operations.

When dock or window resizing changes the canvas dimensions, the camera shifts by half of
the viewport-size delta. The authored point at the old viewport center therefore remains
at the new center instead of jumping toward an edge.

## One coordinate transform

`CanvasViewport` represents a uniform positive zoom and a translation in logical viewport
pixels. The artboard-to-viewport mapping is:

```text
viewport = artboard * zoom + offset
artboard = (viewport - offset) / zoom
```

For a viewport anchor `a`, changing zoom from `z` to `z2` computes:

```text
offset2 = a + (offset - a) * (z2 / z)
```

The calculation uses the clamped final zoom, so reaching a limit does not introduce a
translation jump. Panning changes `offset` directly in logical viewport pixels. Its
screen distance therefore remains independent of magnification.

The native workspace applies this mapping once as a render transform inside a clipped
canvas. Application controls continue to measure and arrange in artboard coordinates;
changing zoom does not remeasure them at a different available size. Native coordinate
conversion handles selection, manipulation, drop targets and source-linked overlays
through the same transformed visual tree.

The finite-value boundary belongs to `CanvasViewport`, not to a particular toolbar.
Construction rejects non-finite or non-positive zoom and non-finite or excessively large
offsets. Positive finite zoom is clamped to the supported range. Fitting rejects invalid
bounds, padding and unavailable viewport dimensions. A host can use the value type without
constructing the workspace UI.

| API | Purpose |
| --- | --- |
| `CanvasViewport::new` | Validate a zoom and logical-pixel offset |
| `zoom`, `offset`, `transform` | Read the accepted mapping |
| `artboard_to_viewport`, `viewport_to_artboard` | Convert points through the mapping |
| `zoom_at` | Change magnification while preserving a viewport anchor |
| `pan` | Translate by a logical viewport delta |
| `fit` | Center finite native bounds inside a measured viewport and padding |
| `CanvasWheelMode::Pan` / `CanvasWheelMode::Zoom` | Choose the two supported wheel conventions |

`DesignerWorkspace` exposes the same operations used by its controls:

| Workspace API | Purpose |
| --- | --- |
| `canvas_viewport` / `set_canvas_viewport` | Capture or restore a validated camera |
| `zoom_canvas_at` / `pan_canvas` | Navigate in logical canvas coordinates |
| `fit_artboard` / `fit_selection` | Frame the document or measured selection |
| `set_zoom_percent` | Validate and apply a percentage using the canvas center as anchor |
| `artboard_to_canvas` / `canvas_to_artboard` | Convert points through the native camera |
| `canvas_wheel_mode` / `set_canvas_wheel_mode` | Read or change the wheel preference |
| `canvas_pan_tool` / `set_canvas_pan_tool` | Read or change the persistent hand tool |

Existing `zoom` and `set_zoom` callers share this camera. Hosts should use the conversion
methods when positioning their own canvas UI and the viewport value when retaining a view.
Setting a host-owned camera does not apply pending source or mutate document history.

## Icons at compact sizes

Designer icons use native vector geometry and a scoped `designer-icon` template. The
template fits the complete geometry uniformly within the requested drawing box, centers
it and reserves one logical pixel of padding. Its fill follows the icon's foreground.
This handles paths whose authored coordinates are larger than the compact toolbar box
without depending on an installed symbol font.

Layout rounding is disabled on the template's inner `Border` and `Path`. Uniform fitting
can produce fractional geometry dimensions; rounding the inner desired size before
centering can shift the rendered geometry away from the center of its frame. Using the
true fractional dimensions keeps the geometry centered. The outer icon and button frames
retain their normal layout rounding and stay aligned.

Compact icon buttons also use centered content, a two-pixel padding budget and explicit
minimum-size overrides. Previously, the shared six-pixel button padding could leave too
little space for an icon inside a small action button. Correct path fitting and sufficient
button content space are both required to avoid clipping.

The audit covers the workspace command bar and canvas shelf, asset cards, Layers/Project
rows, component library, property and layout actions, and timeline controls. Small actions
such as More, add/remove, up/down, back, close and property-value markers now use native
geometry. Search uses closed circular contours and a separate handle. Built-in overlapping
contours have an explicit nonzero fill rule; custom asset geometry keeps its declared rule.

The style applies to designer chrome. Application preview icons retain their own control
templates and theme behavior. Icon visuals do not receive pointer input; their containing
buttons, menus and rows remain the command targets.

The shared `vector_icon` and `geometry_icon` helpers are internal implementation details.
Hosts continue to supply custom asset geometry through `AssetIcon::CustomPath`; this change
does not introduce a second public icon ABI.

The native icon audit covers **103 unique production paths** and **two custom paths** with
offset and negative-coordinate origins. Each path is exercised at **14 and 24 logical
pixels**, on both **dark and light backgrounds**. Geometry assertions verify complete
bounds, the one-pixel gutter and centering; native raster checks at **1×, 1.25× and 2×**
verify visible output at each display scale. The fixture also checks the actual workspace's
compact button and command-bar constraints and separation from preview-owned templates.

## Why these conventions

[Figma's zoom and view controls][figma-zoom] expose custom percentages, fit-all and
fit-selection commands, Ctrl/Cmd+wheel zoom and file-tab-local zoom. Its
[canvas navigation guide][figma-pan] describes Space-drag and two-finger panning.
These suit navigating a layout while retaining the selection and tool.

[Blend's shortcuts][blend-keys] define a different wheel default: the wheel zooms,
Shift+wheel pans horizontally, and Ctrl+wheel pans vertically. Offering the two wheel
modes makes both established workflows explicit. FerroUI's keyboard commands remain
consistent when changing the wheel preference.

[Qt Widgets Designer's preview workflow][qt-preview] emphasizes running the real form
and inspecting alternate styles. FerroUI already provides native preview and scoped
preview environments; canvas navigation is local authoring state alongside those features.

[Visual Studio's icon guidance][vs-icons] emphasizes centered silhouettes, consistent
visual weight, negative space and padding at compact sizes. The designer applies those
principles using its existing native geometry and scoped theme system.

[figma-zoom]: https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options
[figma-pan]: https://help.figma.com/hc/en-us/articles/30925881896727-FD4B-Navigate-Figma-Design-files
[blend-keys]: https://learn.microsoft.com/en-us/visualstudio/xaml-tools/keyboard-shortcuts-in-blend?view=visualstudio
[qt-preview]: https://doc.qt.io/qt-6/designer-preview.html
[vs-icons]: https://learn.microsoft.com/en-us/visualstudio/extensibility/ux-guidelines/images-and-icons-for-visual-studio?view=visualstudio
