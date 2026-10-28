# Canvas guides, rulers and snapping

The native **Guides** button on the canvas shelf opens a staged editor for exact guide
positions, ruler/grid visibility and snapping. These are local canvas aids. Applying them
leaves the XAML buffer, application properties, preview generation, undo and collaboration
history untouched. Viewers can use them while inspecting a document.

## Native interaction

| Action | Result |
| --- | --- |
| Shift+R on the canvas | Show or hide rulers |
| Drag down from the top ruler | Create a horizontal guide |
| Drag right from the left ruler | Create a vertical guide |
| Drag a guide | Move its preview; release accepts the final pointer position |
| Alt+drag a guide | Create a copy; Alt also bypasses grid snapping |
| Drag a guide back over its corresponding ruler | Remove it on release |
| Select a guide, then Delete or Backspace | Remove only the selected guide |
| Arrow keys / Shift+arrow keys | Nudge the selected guide along its own axis |
| Escape | Discard an unfinished drag and clear guide selection |
| Shift+G on the canvas | Show or hide guides |

Ruler ticks and guide hit targets remain the same screen size at every zoom. Their numbers
refer to the same artboard coordinates used by XAML layout and the canvas camera. A pan,
zoom or viewport resize retires an unfinished guide drag before the coordinate frame
changes. Source/scope/settings changes, capture loss and focus leaving the canvas also
retire the draft. No partially dragged coordinate is accepted.

The Pan tool, held Space, middle-button panning and Interact mode retain their established
routing. Text inputs keep their normal editing shortcuts.

## Exact positions and snap preferences

The editor allows horizontal and vertical guides to be added, removed or cleared. Each
position can be typed exactly, including fractional and negative coordinates. All values
are checked together when **Apply** is pressed. Invalid values keep the editor open with
an explanation; **Cancel** or dismissal changes nothing. A retained editor cannot apply
onto another document, source revision, preview scope or newer guide model.

Visibility and snapping are independent. Hidden guides can remain snap targets. Grid,
guide and object snapping can each be disabled separately. Alt temporarily bypasses snapping
while manipulating controls. Snap distance is measured in screen logical pixels and is
converted using the actual zoom before querying the accepted snap index.

Control movement snaps to guide lines using the selection's leading edge, center and
trailing edge. Axis-aligned side resize handles snap only along the dimension being resized;
corner handles can snap on both axes. Rotated or skewed side handles retain their existing
local resize behavior. The visible grid is clipped to the artboard and becomes less dense
when zoomed out; this display adaptation does not change the configured snap spacing.

Small and large keyboard nudge amounts also apply to authored control movement. Ctrl/Cmd
with an arrow uses one tenth of the small step. Guide positions are limited to ±1,000,000
logical units, with at most 256 guides in one canvas scope. Grid spacing accepts 0.1–100,000
units; snap distance accepts 0–32 screen pixels. Rendering uses bounded tick/line counts,
including at extreme camera offsets and zoom levels.

## Document and definition ownership

Each retained document tab keeps its own guide sets and snapping preferences. A template or
other definition canvas has an independent set; returning to the document restores the
document's settings, and reopening that definition restores its guides. Save As retains the
current presentation. Explicitly opening a new document resets the guide state even when
its URI and source text equal the previous document.

These settings are retained for the workspace's open tabs. They are not serialized into
application XAML or an on-disk preferences file. Arbitrary nested controls do not own
additional frame guides; the active document or definition canvas is the coordinate scope.

## Embedding APIs

`CanvasGuide`, `CanvasGuideAxis` and `CanvasGuideSettings` are public serializable value
models with finite validation. `CanvasGuidesEditor` is an embeddable staged native editor.

`DesignerWorkspace` exposes:

- `canvas_guides` / `set_canvas_guides` for a complete validated model.
- `add_canvas_guide`, `move_canvas_guide` and `remove_canvas_guide` for individual edits.
- `selected_canvas_guide` for local guide selection.
- `canvas_guide_snapshot` / `apply_canvas_guides` for source- and scope-checked drafts.
- `canvas_guide_view_snapshot` / `restore_canvas_guide_view` for a matching document view.

`DesignSurface::set_canvas_guides` installs the same snapping settings for standalone
hosts, and cancels its pending authored gesture before changing the accepted snap index.
The older `DesignSurface::set_grid` remains an independent compatibility override accepting
any finite nonnegative spacing; applying `CanvasGuideSettings` replaces that override.

## Reference decisions

[Figma's rulers and guides](https://help.figma.com/hc/en-us/articles/360040449713-Add-guides-to-the-canvas-or-frames)
provide ruler creation, guide movement, duplication and removal. Its
[layout guide controls](https://help.figma.com/hc/en-us/articles/360040450513-Create-layout-guides)
separate visibility from guide usefulness. The FerroUI implementation adapts those workflows
to one native XAML artboard or definition scope.

[Visual Studio's XAML element workflow](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/working-with-elements-in-xaml-designer?view=visualstudio)
uses alignment snaplines alongside authored margins and native layout containers. FerroUI
keeps those container semantics and adds manual guide coordinates to its existing prepared
snap index.

## Native validation

Regression coverage exercises real pointer capture, final release coordinates, Escape,
source/camera/resize invalidation, hidden-guide snapping of a native Canvas child, staged
validation, viewer access, pending source, scope/tab restoration, explicit document
replacement, and guide-only keyboard deletion. Ruler arithmetic is tested across the
workspace's zoom range and very large camera offsets.

The native render fixture produces `designer-canvas-guides.png` and
`designer-canvas-guides-editor.png` with the same WinUI theme, HarfBuzz shaping and Vello
CPU backend as the existing canvas navigation evidence.
