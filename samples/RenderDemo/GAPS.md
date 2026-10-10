# RenderDemo: gaps

What the port of the sample found in the framework, and what it found in the sample itself. Every gap has a reproduction in `tests/gaps.rs`, or in the test of the shell that is named with it; a gap that is fixed keeps its test, which then asserts what the framework does now.

```sh
cargo test -p render-demo --lib
```

Status: 17 documents, all compiled by the build, all load with the run-time loader too and show their class; 20 pages, all selected, laid out and rendered through the compositor with Skia by `tests/shell.rs`.

## Gaps of the framework

| Id | What the sample does | What the framework did | Status |
|---|---|---|---|
| R001 | `Pages/CustomAnimatorPage.xaml` gives the setters of its key frames a custom animator: `<Animation.Animator><pages:CustomStringAnimator/></Animation.Animator>`. Upstream: `Animation.SetAnimator(IAnimationSetter setter, CustomAnimatorBase value)`, which the compiler of markup finds as the setter of an attached property. | The markup metadata of `Animation` had no static setter `SetAnimator`, and the contract of the custom animators was no type of markup (no handle, no equality), so the compiler refused the document (`Unable to resolve suitable regular or attached property Animator`) and a class of a sample could not be cast to the contract. | Fixed: `SetAnimator` is declared on `Animation` (`scripts/markup_types_overrides.py`, generated into `src/FerroUI.Base/markup_types/classes.rs`), the contract is published as `CustomAnimatorBase` (`markup_types/contracts.rs`) and compares by identity (`animation/i_custom_animator.rs`). `tests/gaps.rs`: `r001_a_setter_of_a_key_frame_takes_a_custom_animator`; in the base crate `the_animator_of_a_setter_is_an_attached_property_of_the_animation`. |

## Findings in the sample

| Id | What the sample does | What happens | Status |
|---|---|---|---|
| R002 | `CustomStringAnimator.Interpolate` takes `newValue.Substring(0, length + 1)` with `length = (int)(progress / step)`. | At a progress of exactly one that is one character more than the text has, and `Substring` throws. The framework asks an animator for its value at a progress of one when it ends an animation whose element leaves the visual tree (`AnimationInstance.DoComplete(false)` from the handler of `DetachedFromVisualTree`, which calls `FindEdgeValue`), so the managed original throws when the page is left or the window is closed. | Listed, not a gap of the framework: the port of the animator takes at most the whole text (`Pages/custom_string_animator.rs`; `docs/porting/DEVIATIONS.md`, RenderDemo sample). `tests/shell.rs`: `the_custom_animator_page_animates_its_text_with_the_animator_of_the_sample` leaves the page. |
| R003 | `Pages/AnimationSpeedPage.xaml` binds its sliders with `Mode=OneWayToSource`. | A slider writes its value, zero, to the view model when it is bound, after the constructor of the view model set a speed ratio of one: the texts of the page stand still until a slider is moved. The managed original does the same. | Listed, as upstream. `tests/shell.rs`: `the_animation_speed_page_rotates_its_texts_at_the_speed_of_the_slider`. |

## Bindings that report an error

The tour of the pages reports three binding errors, once each, and the desktop host prints the same three when it starts. The managed original makes each of them: the template of the hamburger menu binds members of the selected item of the menu (`SelectedItem.(ScrollViewer.HorizontalScrollBarVisibility)`, `SelectedItem.(ScrollViewer.VerticalScrollBarVisibility)`, `SelectedItem.(TabItem.Header)`), the menu has no selected item while it sorts its pages when it is loaded (`Items.Clear()`), and a null in the middle of a path is an error of a binding. `tests/shell.rs` holds the list (`ACCEPTED_BINDING_REPORTS`); a report that is not in it fails `every_page_is_laid_out_and_rendered`, and so does an entry that is no longer reported.

## What the tests do not reach

- The timers of `LineBoundsDemoControl` (60 ticks a second) and of the two glyph run controls (one tick a second) run on the clock of the dispatcher, which the tests do not advance: the pages are drawn as they are at their first frame.
- The statistics of the hit testing page are refreshed once a second of the same clock: the test reads the line of the first frame.
- The window options of the upstream entry point (`OverlayPopups` of the Windows platform) and the developer tools of a debug build have no counterpart (`program.rs`).

## Differences of the port that are not gaps

- Handlers of timers, pointer events and composition updates hold their control weakly where the managed original captures `this`: a cycle a collector frees, and here a leak.
- The stopwatches of the pages that draw by the time are the clock of the dispatcher (milliseconds); the custom draw operation of the Skia page, which draws on the render thread, measures with the monotonic clock of the standard library.
- `Random` of the glyph run controls is the subtractive generator of the managed original, unseeded.
