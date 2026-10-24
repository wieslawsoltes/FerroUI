# Built-in editor background renderers

This checkpoint ports AvaloniaEdit's column rulers, current-line highlight and
replaceable-snippet field decorations through FerroUI's live background renderer
collection. It builds on [background geometry and layer contracts](editor-backgrounds.md)
and the [snippet insertion and lifetime port](editor-snippets.md).

The source baseline is AvaloniaEdit commit
`7277e36366d754219d7ede16b6613786b32caf4c`. No managed rendering package, separate
text shaper or graphics abstraction is introduced.

## Source mapping

| Pinned AvaloniaEdit source | FerroUI destination | Behavior |
| --- | --- | --- |
| [`Rendering/ColumnRulerRenderer.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/ColumnRulerRenderer.cs) | `src/FerroUI.Edit/column_ruler_renderer.rs` | Ordered column guides, native pen, measured space advances, horizontal scroll and device-pixel alignment |
| [`Rendering/CurrentLineHighlightRenderer.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/CurrentLineHighlightRenderer.cs) | `src/FerroUI.Edit/current_line_highlight_renderer.rs` | Full visible-line rectangle, native measured height, default translucent fill and border, explicit current logical line |
| [`Rendering/TextView.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Rendering/TextView.cs) and [`TextEditorOptions.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/TextEditorOptions.cs) | `src/FerroUI.Edit/text_area_backgrounds.rs` and Core `EditorOptions` | Automatic built-in renderer registration, current-caret line, enabled state, ruler positions and native styled brush/pen properties |
| [`Utils/PixelSnapHelpers.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Utils/PixelSnapHelpers.cs) and [`Utils/ExtensionMethods.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Utils/ExtensionMethods.cs) | Ruler pixel alignment and native `Visual::transform_to_visual` / `TopLevel::render_scaling` | Away-from-zero midpoint rounding followed by transformed device-pixel center snapping |
| [`Snippets/SnippetReplaceableTextElement.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Snippets/SnippetReplaceableTextElement.cs) | `src/FerroUI.Edit/snippet_background_renderer.rs`, `snippet_controller.rs` | Background field fills, active source and bound-field outlines, caret/document invalidation and deterministic removal |
| [`Snippets/SnippetBoundElement.cs`](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Snippets/SnippetBoundElement.cs) field-target relationship | `SnippetSession::fields` and `SnippetField` in `FerroUI.Edit.Core` | UTF-16 range snapshots preserve the exact source-to-bound relationship, including transformed lengths and occurrences preceding their source |
| View attachment and renderer state invalidation | `src/FerroUI.Edit/background_renderer_connections.rs` | Weak native view connections, shared renderer identity and invalidation outside collection borrows |

## Native ownership and view integration

The two general renderers are ordinary Rust objects behind `Rc`. Register their
`BackgroundRendererRef` values in `TextArea::background_renderers()`. Collection
membership invokes `ITextViewConnect` and removal disconnects the identity.
Renderers can be shared between views; their state setters invalidate the
background layer in every connected view. The connection list stores weak
references and does not retain an editor after its owner is released.
Disconnection also runs during `TextArea` destruction. Built-in connection
helpers compare `to_weak()` identities there; reconstructing a strong `to_ref()`
would panic after the native object's last strong reference has gone away. The
same fix is applied to `FoldingElementGenerator`, and `ITextViewConnect` now
documents this lifecycle constraint for application extensions.

`TextArea` owns its normal current-line and ruler renderer instances and registers
them automatically. `highlight_current_line`, `show_column_rulers` and
`column_ruler_positions` in `EditorOptions` drive those instances using the
current paint frame after line-construction callbacks have completed. Defaults
are both visibility flags false and ruler positions `[80]`. Accessors
`current_line_highlight_renderer()` and `column_ruler_renderer()` expose the
built-in objects. Independently positioned or styled additional decorations can
be registered through the same collection:

```rust
use ferroui_edit::{ColumnRulerRenderer, CurrentLineHighlightRenderer};
use std::rc::Rc;

let rulers = Rc::new(ColumnRulerRenderer::new());
rulers.set_columns([80, 120]);
editor.text_area().background_renderers().add(rulers.clone().into());

let current_line = Rc::new(CurrentLineHighlightRenderer::new());
current_line.set_enabled(true);
current_line.set_line(3); // One-based logical document line.
editor.text_area().background_renderers().add(current_line.clone().into());
```

The general renderer API leaves current-line and option policy with the host.
`CurrentLineHighlightRenderer::set_line` receives a one-based logical line and
zero suppresses the highlight; `set_enabled` controls visibility. The existing
editor `highlight_current_line` option drives the built-in instance. The renderer itself
does not install a competing caret subscription. Rulers default to no columns
and accept an owned ordered `Vec<i32>` snapshot, preserving negative and repeated
columns from the upstream enumerable. `set_ruler` changes columns and pen with
one invalidation; an absent pen hides their strokes.

Both renderers run in `KnownLayer::Background`, through the actual host paint
phase before text-element backgrounds, selection and glyphs. State changes call
`TextArea::invalidate_layer` and preserve existing shaped text. The host's content
clip keeps ruler strokes and full-width highlights out of the line-number gutter.

## Column ruler geometry

The ruler position is the requested column multiplied by
`TextArea::wide_space_width()`. It is aligned to a pixel center using the pinned
away-from-zero rule, translated by the content origin and horizontal scroll
offset, then snapped using the actual native transform to the root visual and
its render scaling. This preserves fractional scroll behavior and device scale.

Lines extend from the content viewport's top through the greater of the native
scroll extent and viewport height. They do not depend on the visible document
line count or an assumed monospace glyph advance. The pen is FerroUI's real
`IPen`; the default is one logical unit in LightGray (`#FFD3D3D3`).
The host's `ColumnRulerPen` styled property replaces that standalone default with
the pinned TextView stroke, ARGB `#5A808080`, also one logical unit. Changing the
host pen does not override the ruler visibility option.

The upstream device snap dereferences its root and inverts its transform. The
Rust implementation permits an unattached view or a singular transform: it
keeps the already column-aligned local point. Nonfinite ruler coordinates are
reported as a `VisualLineError` rather than sent to the native renderer. These
fallbacks preserve native headless layout and avoid panics for transient visual
tree states. Finite negative transformed coordinates retain upstream truncation
toward zero before adding the half-pixel center.

## Current-line geometry

`TextArea::visual_line_bounds` supplies the visible owner's full content-width
rectangle. Its vertical origin includes scroll offset and its height comes from
the native formatted layout. A wrapped line therefore highlights all its rows;
a line within a folded or generated multiline span shares its owner's highlight.
Offscreen or invalid line numbers produce no geometry.

The renderer passes that rectangle to `BackgroundGeometryBuilder` and draws its
native `StreamGeometry`. It retains the pinned defaults: background ARGB
`#1614DCE0`, border ARGB `#3400FF6E`, and a one-unit native pen. Background and pen
are separately optional and configurable. `set_default_colors` restores both
defaults in one invalidation. No fixed-row-height approximation is used.

The native host `CurrentLineBackground` and `CurrentLineBorder` styled properties
retain the upstream `None` defaults. They update the renderer on effective
property changes, including clearing a previously set brush or pen. They do not
overwrite the renderer's initial colors merely because the highlight is enabled.
`TextArea::set_default_highlight_line_colors()` restores the renderer defaults
without rewriting the styled property values, matching the pinned reset helper.

## Snippet decorations

`SnippetController::insert` installs two collection entries for a session with
built-in replaceable fields. The background entry fills every source field with
LimeGreen at 0.4 opacity. The text-layer entry outlines only source fields whose
inclusive range contains the native editor caret, together with each such
field's bound occurrences. The stroke is the native dotted black one-unit pen.
Merely entering a bound occurrence does not activate its source outline.

`SnippetSession::fields()` returns owned `SnippetField` snapshots containing the
field ID, its current anchored source range and its bound ranges. All offsets and
lengths are UTF-16; transformed bound text may have a different length. The query
keeps insertion order, omits invalid anchors and closed sessions, and does not
call extension code while borrowing session collections. Named anchors and
custom active elements are not mistaken for built-in bound occurrences. Custom
elements can supply their own native background renderer.

Each range is projected through `TextArea::segment_rectangles`, so decorations
use the same bidi fragments, soft-wrap boundaries, folds, preedit projection and
empty-field markers as the live view. A fresh geometry builder for each source
field preserves overlapping-field compositing. Whole-pixel alignment and border
thickness match the upstream snippet builder for fills and outlines. Source and
bound fragments are appended to one foreground geometry for each active field.

The controller's native text-change subscription invalidates both layers after
anchor changes, and its native selection-change subscription invalidates the
foreground after caret movement. Active placeholder traversal moves the native
selection and follows the same path. `field_background_renderer()` and
`active_field_renderer()` expose the live instances for styling while active.
When changing border thickness on an installed pair, set it on both instances
to preserve matching pixel alignment.

Deactivation, insertion undo, document replacement, errors and controller drop
remove both identities and dispose the event subscriptions. The registration
owns only a weak view reference, and each renderer owns only a weak session
reference. Retaining a removed renderer does not retain a live snippet. A
collection observer may synchronously deactivate during registration; the local
registration then removes the first inserted entry before returning, without
leaving a partially installed pair. Renderer drawing also checks session and
document identity so a stale external reference cannot paint another document.
If building a previously unseen line runs an extension that deactivates the
snippet, drawing stops before submitting geometry. If that callback instead
moves document anchors or the caret, the old range snapshot is rejected with
`ConstructionInvalidated` so the host can paint the new state on its next frame.

## Verification

The combined checkpoint passed **323 Core tests** and **169 native control
tests**, including both new Core field-snapshot regressions, all twelve focused
renderer regressions and the existing snippet controller tests. Formatting and
`git diff --check` also passed. The native gate caught an incorrect geometry
reference conversion; both renderers now use FerroUI's borrowed
`Ref::upcast_ref::<Geometry>()`, and the passing gate includes that correction.

The focused native tests live in
`src/FerroUI.Edit/builtin_background_renderer_tests.rs`. They draw through
FerroUI's `DrawingGroup` and the live editor paint path with the native test
render interface. They cover scroll/gutter ruler positions, duplicate columns,
negative and transformed device snapping, default colors, disabled/unseen lines,
wrapped and tall-font line geometry, folded owners, native snippet fills and
dotted linked outlines, caret boundaries, UTF-16 anchor changes, bidi/wrapped
fields, empty fields, reentrant registration teardown and weak ownership.
They also check deactivation from a native line-construction callback before a
retained renderer can draw an obsolete field.

Two Core regressions in `snippets/snippet_extension_tests.rs` verify the field
snapshot relationship across transformed-length changes, numeric versus
insertion order, named anchors, prefix edits and bound ranges before their
source. Existing snippet keyboard and undo tests remain part of the native gate.

The renderer implementation uses the production native geometry and drawing
contracts. The deterministic test backend verifies geometry, pens, brush opacity
and ordering; it does not establish raster antialiasing parity across platform
graphics backends or replace the project's separate browser conformance gate.
