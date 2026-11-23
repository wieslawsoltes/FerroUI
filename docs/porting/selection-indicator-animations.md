# Selection indicator composition animations

## Upstream investigation

The source is WinUI's [NavigationView.cpp](https://github.com/microsoft/microsoft-ui-xaml/blob/388069e3f18abafdd74d68b0b3cd1b3fa4dbb097/src/controls/dev/NavigationView/NavigationView.cpp#L1992), particularly `AnimateSelectionChanged`, `PlayIndicatorAnimations`, `PlayIndicatorNonSameLevelAnimations`, `PlayIndicatorNonSameLevelTopPrimaryAnimation`, and `OnAnimationComplete`. The [current source](https://github.com/microsoft/microsoft-ui-xaml/blob/main/controls/dev/NavigationView/NavigationView.cpp) still uses these keyframes.

Ferro's NavigationView port already supplied `SelectionIndicator` template parts and selected/collapsed-ancestor opacity, but omitted the composition animation. Fluent TabItem used `PART_SelectedPipe` and collapsed unselected pipes, so there was no unselected geometry to animate. WinUI-theme vertical TabItems reuse NavigationView's indicator appearance.

WinUI's [TabView.cpp](https://github.com/microsoft/microsoft-ui-xaml/blob/main/controls/dev/TabView/TabView.cpp) does not implement this underline transition. Ferro's WinUI-theme top/bottom tabs use TabView's tab shape and have no selection line. They keep that appearance. TabControl now shares NavigationView's motion whenever its item template supplies `SelectionIndicator` or `PART_SelectedPipe`, including Fluent tabs in all four placements.

## Motion and ownership

Two existing indicators participate, each in its own item template. Coordinates come from the laid-out elements in their common control's coordinate space. Both indicators become opaque before starting the animation.

For items at the same depth:

- Duration: 600 ms; the pivot changes at 200 ms.
- The scale grows from the source length to the distance between indicators plus their length at one third of the progress, then contracts to the destination length. Horizontal indicators account for different tab widths.
- For indicators up to 80 logical pixels apart, the stretch uses cubic Bezier control points `(0.9, 0.1), (1, 0.2)`; contraction and outgoing fade use `(0.1, 0.9), (0.2, 1)`. Farther apart the control points move, in proportion to the distance, towards `(0.4, 0), (0.6, 1)` for both, which they reach at 240 pixels (see "Differences from WinUI").
- A single-step easing switches the position and the centre of scaling together, making the leading edge stretch first and the trailing edge catch up. Both are keyed at one third of the same 600 ms timeline.
- The outgoing indicator stays opaque through the stretch, then fades out. The incoming indicator stays opaque.

At different hierarchy depths, the bars shrink/grow in place instead of crossing the content diagonally. Vertical bars pivot about the edge facing the destination; horizontal bars pivot about their centre. The horizontal pivot uses `CenterPoint.X`: the upstream top-primary helper writes Y while scaling X, which would not centre a horizontal scale.

All motion uses the same composition API design as WinUI: scalar keyframe animations of `Offset.X/Y`, `Scale.X/Y`, `CenterPoint.X/Y` and outgoing `Opacity`, grouped in a `CompositionScopedBatch`. There are no UI-thread animation ticks, layout-property animations, XAML transitions, or `RenderTransform` animations.

`ResetElementAnimationProperties` in the upstream source uses ordinary visual property assignments. Ferro now follows that contract in its generated visual setters: an assignment replaces a whole-property or subchannel animation even when the API-side value is unchanged. The assignment is serialized with the property stream, so subsequent `StartAnimation` in the same commit wins. Implicit animations still require a changed value, as [Microsoft documents](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.implicitanimationcollection). The element renderer forwards actual framework changes instead of redundantly assigning every property during an unrelated repaint.

WinUI's indicator wrapper makes its local offset zero. Ferro templates can place the indicator at a nonzero local layout origin, particularly Fluent tab indicators. The offset keyframes therefore add that origin to WinUI's relative positions; completion restores the element's layout offset, unit scale, zero centre and selected/unselected opacity using normal setters. No extra composition reset API or queued stop workaround is used.

## Differences from WinUI

Two details of `PlayIndicatorAnimations` are not reproduced.

- **One timeline for the pivot.** WinUI switches the centre of scaling at the end of a 200 ms animation and the position at key 0.333 of the 600 ms one, which is 199.8 ms. Between the two the indicator, stretched over the whole travel, is drawn at its new position with its old centre: a frame sampled there shows it displaced by the travel distance. The centre is keyed at one third of the 600 ms timeline, the key of the position, so one step moves both (`pivot_keeps_both_indicators_inside_the_travel_span` samples around 200 ms in both directions and orientations).
- **Easing of a distant selection.** The stretch curve of WinUI `(0.9, 0.1), (1, 0.2)` ends, and its contraction `(0.1, 0.9), (0.2, 1)` begins, at a speed that grows with the distance: between items far apart an edge of the indicator covers most of the travel in one frame. Up to 80 pixels of travel the curves are WinUI's, so the motion between neighbouring items is unchanged; from there to 240 pixels they are blended towards `(0.4, 0), (0.6, 1)`, which starts and ends at rest. The duration and the keys are the same (`distant_selection_bounds_edge_motion_at_60_and_120_hz`: over 360 pixels no edge moves 72 pixels or more between two frames at 60 Hz or 120 Hz).

## Layout and lifetime

Each attached control observes `LayoutUpdated`, so target lookup happens after item styles, template application, selection, expansion and arrangement. Selection changes invalidate arrange even when they only change opacity. Collapsing a selected descendant targets its first collapsed ancestor; expansion restores the descendant's indicator.

Repeated notifications for the same target preserve its running animation. A new target completes the preceding transition before starting another. A generation check and disposed batch subscription prevent an earlier completion from resetting a later animation. Resizing/repositioning the target or changing strip orientation completes the old motion. Detach, template replacement and drop release animation state; layout and completion callbacks hold their owners weakly.

First/empty selections, missing parts, zero dimensions, unrelated visual trees, absent composition visuals and disabled platform animations settle immediately. Fluent's unselected pipes remain laid out at zero opacity so both geometry and composition visuals are available. Themes without a named line receive no new visual element.

## Validation

`cargo test -p ferroui-controls --lib selection_indicator` exercises the real compositing renderer and render-side visual values. Composition animation controllers sample keyframes without depending on wall-clock timing. Coverage includes vertical stretch and pivot, the pivot around its switch, the edge motion of a distant selection per frame, renderer invalidation during motion, horizontal unequal widths in both directions, different depths, repeated targets, rapid interruption, natural scoped-batch completion, fallback behaviour and disabled motion, target resize/orientation changes and the controls' selection/layout/detach integration. The base composition tests verify restoring an unchanged base value and reset/start ordering within one commit.


## Native desktop visual check (2026-11-07)

Built `control-catalog-desktop` with `classic-themes` in this worktree and launched
`target/ControlCatalogIndicators.app`, backed by this worktree's executable and native
macOS/Metal renderer. The application was exercised through its native UI, including inline
TabItems and tabs generated from a data template.

| Default theme/control | Resting indicator size (logical pixels) | Desktop result |
| --- | --- | --- |
| WinUI NavigationView, left and compact | 3 × 16 | Correct final item and no stranded indicator after rapid changes and compact-pane toggling |
| WinUI NavigationView, top | 16 × 3, centred | Matches the upstream top-primary template; the short pill is intentional |
| WinUI TabbedPage, horizontal | 16 × 3 | Uses NavigationView's design; rapid Home/Search/Settings changes settle correctly |
| WinUI TabControl, top/bottom | TabView selected-tab shape, no accent underline | Existing appearance preserved |
| WinUI TabControl, left/right | 3 × 16 | Correct selected indicator after rapid Arch/Leaf changes in both inline and data-template samples |
| Fluent TabControl, top/bottom | Header width × 2 | Underline follows each header's width and settles without retaining the stretch |
| Fluent TabControl, left/right | 2 × 24 | Correct resting geometry after rapid changes |

The native click sequences included eight left-navigation selections in 940 ms, eight top
selections in 864 ms and eight compact selections in 924 ms. These interrupt the 600 ms
composition animation repeatedly. Visual inspection found the final selected item and indicator
consistent, with no duplicate or stranded indicators. Desktop inspection supplements the
render-side keyframe tests; it is not a frame-rate measurement. WinUI was restored and the
catalog was left running on the NavigationView page.

The authoritative top-primary rectangle is in
[WinUI's NavigationView template](https://github.com/microsoft/microsoft-ui-xaml/blob/main/controls/dev/NavigationView/NavigationView_themeresources.xaml#L903):
16 logical pixels wide, 3 high, with radius 2, in a centred wrapper. At 2× display scaling it
occupies a nominal 32 × 6 screen pixels. Its resting width is independent of the header length;
the composition scale changes only during the transition.
