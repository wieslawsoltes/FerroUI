# A popup opened while its target is moved by a render transform (October 2026)

**Decided (2026-10-15):** the owner chose the recommendation: record the difference as a deviation and fix it after the first preview. Fix 1 and 2 are in `primitives/popup.rs` (DEVIATIONS.md, "Popup"), with the reposition per frame; the ignored test below is active and passes, and the test of case (a) checks only the place given at the opening. The rest of this document is the analysis as it was written before the decision.

## What was seen

The picture run of the ControlCatalog on Wayland opened a combo box while its page was still sliding in (`wayland-platform.md`, section 11). The popup was configured a page width to the right of where the combo box came to rest, and stayed there.

## What the host test shows

`src/FerroUI.Controls/popup_moving_target_tests.rs` (the headless mock window with native popups placed by the managed positioner; the frames are render passes of the media context, as in `native_control_host_tests.rs`). A window of 400 by 300, a `NavigationPage` with a horizontal `PageSlide`, a page with a target of 100 by 50 that rests at (20, 30), and a `Popup` of 60 by 40 placed below the target (`PlacementMode::Bottom`, so centred: (40, 80) at rest). A positioner that records its parameters stands in front of the managed one.

| Case | The anchor rectangle the positioner is given | Where the popup's host is when the transition has ended | Test |
|---|---|---|---|
| (b) opened after the transition | (20, 30, 100 by 50) | (40, 80): below its target | `popup_opened_after_the_transition_is_placed_at_its_target` (passes) |
| (a) opened in the middle of a transition of 1.5 seconds | the rectangle of the moving target: for example (299.2, 30, 100 by 50) | (319, 80): where the target was when the popup opened; the positioner is not called again, and the popup stays open | `popup_opened_during_the_transition_keeps_the_place_its_target_had_then_known_difference` (passes: it asserts the behaviour as it is) |
| (a) as a fix would have it | (20, 30, 100 by 50) at the end | (40, 80) | `popup_opened_during_the_transition_ends_at_its_target` (ignored, with the reason; run with `--include-ignored` it fails with the anchor rectangle at (299.2, 30)) |

**Is (a) reachable by a user? Yes.** The second test asserts it: during the transition the hit test at the visual position of the target finds the target (it is the topmost visual below the empty presenters the test template lays over the pages), and at its place at rest it is not found. Input is hit-tested against the composition visuals, which carry the render transform, so a press on a control of a page that is still moving reaches that control and opens its popup. With the default transition of a navigation page the window of time is short; with a slow transition, or a control near the leading edge of the page, it is easy to meet.

## The port against upstream, file by file

| Upstream (tracked commit `17350180`) | Port | Result |
|---|---|---|
| `Primitives/Popup.cs`: `UpdateHostPosition` is called at open (line 483), from `HandlePositionChange` (the window's `PositionChanged`, lines 506 to 508; a parent popup root's `PositionChanged`, lines 521 to 523) and from `PlacementTargetLayoutUpdated` (the target's `LayoutUpdated`, lines 513 to 515, and only when the bounds of the target differ from the last ones, lines 1038 to 1050) | `primitives/popup.rs`: `update_host_position` at open, from `handle_position_change` (`platform_position_changed` of the top level, `position_changed` of the parent popup root) and from `placement_target_layout_updated` with the same comparison of bounds | The same three triggers and no other |
| `Primitives/PopupRoot.cs`: `UpdatePosition` from `ConfigurePosition` (line 150) and when the arranged size of the root changes (lines 212 to 219) | `primitives/popup_root.rs`: `update_position` from `configure_position` and when the size changes | The same |
| `Primitives/OverlayPopupHost.cs`: the same two places (lines 100 and 124) | `primitives/overlay_popup_host.rs` | The same |
| `Primitives/PopupPositioning/IPopupPositioner.cs`: the anchor rectangle is the bounds of the target (or the placement rectangle) intersected with the bounds and transformed with `target.TransformToVisual(rootVisual)` at the time of the call (line 578) | `primitives/popup_positioning/i_popup_positioner.rs`, `calculate_anchor_rect` | The same: the place of the target at the time of the request, render transforms included |
| `ManagedPopupPositioner.cs`, `PopupPositionRequest.cs` | the modules of the same names | Hold no state about the target; nothing to compare |
| The placement target (`Popup.PlacementTarget`, `SetPlacementTarget`) | the same | No subscription to a transform on either side |

**The port matches upstream.** An animated render transform of an ancestor moves no window, raises `LayoutUpdated` without a change of the target's bounds (the bounds are relative to the parent), and moves no parent popup: upstream leaves the popup where the control was as well, on every platform. The one thing that differs between platforms is what the platform's positioner then does with a rectangle near the edge: the compositor of a Wayland session slides a popup that would leave the output back into it, which is why the pictures had it at the edge of the output.

A popup opened in (a) is positioned again later only by accident: when its own size changes, or when a layout pass changes the bounds of its target.

## The smallest correct fix

Position an open popup again when the transform of its target to the root changes, as `NativeControlHost` does since its own fault of this class (`native_control_host.rs`, `subscribe_render_transforms`; DEVIATIONS.md, "Native control host"):

1. In the open state of `Popup` (where the three subscriptions are made, and only when the placement is not `Pointer`): walk from the placement target to the root; for every visual subscribe to a change of its `RenderTransform` and `RenderTransformOrigin` properties, and to the `changed` event of a mutable transform (an animation changes one in place). Subscribe again when a `RenderTransform` of the chain is replaced. The subscriptions go into the clean-up list of the open state, so they end when the popup closes.
2. A change queues one update per frame (a flag and a dispatcher job after the render, as `enqueue_for_move_resize` of the native control host), and the update calls `update_host_position` only when the rectangle of the target in the root differs from the last one given (kept beside `last_placement_target_bounds`).

About 60 lines in `primitives/popup.rs` and no change of a contract. The ignored test is its acceptance test.

What it costs, and why it is a decision:

- **It is a deviation from upstream** (a second one of the same kind as the native control host's), to be recorded in `DEVIATIONS.md`. Upstream has the fault and has not fixed it at the tracked commit.
- **A popup then follows its target through the whole animation**, one reposition per frame. On Windows, macOS and X11 that is a window move. On Wayland it is `xdg_popup.reposition` with a shell of version 3 or later, and before that (sway 1.9, the compositor of the tests) the popup is destroyed and created again for every new place (`wayland-platform.md`, section 7.1), so it flickers for the length of the transition. A variant that waits until the rectangle has been the same for one frame before it moves the popup avoids that at the price of a popup that jumps once at the end; it needs a timer or a second frame and is not smaller.
- The alternative that changes nothing in the popup: do not let input reach a page while it slides (`NavigationPage` would disable hit testing of the incoming page for the transition). That removes this way of meeting the fault, not the fault (any animated transform above an open popup's target shows it), and it is also not upstream's behaviour.

## Recommendation

Leave the behaviour as it is for the first preview (it is upstream's, and the tests pin it), and take fix 1 and 2 as a recorded deviation afterwards, with the reposition per frame: it is the behaviour a user expects, it is what the native control host already does, and the cost on an old Wayland shell is confined to a popup that is open while its target moves.

## Addition: ancestors moved by layout (2026-11-27)

The fix follows render transforms only. A layout pass that moves an ancestor of the target (a scroll of the editor, a reflow above it) leaves the bounds of the target itself unchanged, so neither upstream's `PlacementTargetLayoutUpdated` nor the transform subscriptions position the popup again. The code editor anchors its completion list, insight and search popups to the caret inside a scrolled text area, so these popups opt in: `Popup::set_follows_placement_target_layout(true)` adds the `Bounds` of every element of the chain to the same subscription and the same once-per-frame job. It is off by default, so every other popup, flyout, tool tip and menu keeps the triggers above; a popup that does not opt in pays one flag test per property change of the chain while it is open. Tests: `popup_moving_target_tests.rs` (`a_popup_follows_an_ancestor_moved_by_layout_only_when_it_opts_in`).
