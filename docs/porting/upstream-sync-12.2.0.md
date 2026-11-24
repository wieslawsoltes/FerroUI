# Upstream sync to 12.2.0

The record of the sync from upstream commit `66b7273b9a` (the tracked commit since 2026-10-17)
to upstream release `12.2.0` (`5d9e6d1b0e`, published 2026-11-23): 182 commits, none of them a
merge commit. The inventory has one entry per commit, in
[`data/upstream-sync-12.2.0.json`](data/upstream-sync-12.2.0.json). The tables below are
generated from it by `scripts/port-status/upstream_sync.py`, which also refuses a commit that
is missing, listed twice or without a class.

This work targets milestone 0.7.0 as a stack of draft pull requests (#1329), with two
exceptions in 0.6.0: the security fix of #1317 and the variation axis API of #1376, which the
web fonts stack maps onto. Nothing in it is merged or marked ready.

`TRACKING.md` and `REMAINING.md` are generated from an extraction of the upstream sources
(`scripts/port-status/run.sh`, which needs a checkout of the tracked commit and the .NET
extractor). They still describe `66b7273b9a`; they are regenerated at `12.2.0` once the stack
lands, so that their numbers describe what is merged.

## How a commit is classified

| Class | Meaning |
|---|---|
| `port` | Ported in this work, with its tests. `planned` until its part is pushed, then `done` |
| `already` | FerroUI has the behaviour; the note says where |
| `skip-performance` | It would make FerroUI slower, larger or use more memory |
| `skip-better` | FerroUI's code is already better; the note says why |
| `not-applicable` | .NET only, upstream's build or repository, or tests of removed code |
| `deferred` | Belongs in FerroUI, too large for this pass; the note says what it needs |

What decides between them:

- A change that slows start-up, grows hello world or adds work on a hot path is not taken as
  written. Its behaviour is ported another way, or it is classified `skip-performance`.
- FerroUI's code stays where it is better. A real bug upstream fixed in code FerroUI rewrote is
  fixed in FerroUI's code.
- Nothing drawn on the GPU moves to the CPU. A new rendering feature has a GPU path on each
  backend, or it is `deferred`.
- Every built-in feature is usable from the default XML namespace, without a prefix.

## Status

<!-- generated:summary -->
| Class | Commits |
|---|---:|
| `port` (126 done, 0 planned) | 126 |
| `already` | 21 |
| `skip-performance` | 0 |
| `skip-better` | 5 |
| `not-applicable` | 30 |
| `deferred` | 0 |
| **Total** | **182** |
<!-- /generated:summary -->

Where a port could add
work on a hot path (the animated value write of `ca61365d75`, the TwoWay check of
`500e78afef`), its part keeps the behaviour without the cost or moves the commit to
`skip-performance`, with the reason.

### By area

<!-- generated:areas -->
| Area | `port` | `already` | `skip-performance` | `skip-better` | `not-applicable` | `deferred` | Total |
|---|---:|---:|---:|---:|---:|---:|---:|
| automation | 7 |  |  | 1 |  |  | 8 |
| base |  | 1 |  |  |  |  | 1 |
| build |  |  |  |  | 17 |  | 17 |
| collections |  |  |  |  | 1 |  | 1 |
| controls | 26 | 2 |  |  |  |  | 28 |
| data | 2 |  |  |  |  |  | 2 |
| input | 6 |  |  |  |  |  | 6 |
| layout | 1 |  |  |  |  |  | 1 |
| markup | 2 |  |  |  | 3 |  | 5 |
| platform | 3 |  |  |  |  |  | 3 |
| platform-android | 9 | 1 |  |  |  |  | 10 |
| platform-browser | 3 | 2 |  |  | 1 |  | 6 |
| platform-ios | 5 |  |  |  |  |  | 5 |
| platform-linux | 6 | 1 |  |  |  |  | 7 |
| platform-macos | 9 | 1 |  |  |  |  | 10 |
| platform-win32 | 7 | 2 |  |  |  |  | 9 |
| platform-x11 | 5 | 1 |  |  |  |  | 6 |
| rendering | 13 | 3 |  |  | 2 |  | 18 |
| samples | 2 |  |  |  | 1 |  | 3 |
| tests |  |  |  |  | 2 |  | 2 |
| text | 11 | 2 |  | 4 | 1 |  | 18 |
| themes | 9 | 4 |  |  | 2 |  | 15 |
| threading |  | 1 |  |  |  |  | 1 |
<!-- /generated:areas -->

## Breaking changes upstream made

Upstream's own API compatibility baseline records two removals in this range:

| Upstream change | Commit | Meaning for FerroUI |
|---|---|---|
| `ITopLevelImpl.Paint` removed, with the offscreen override | `24b6c7b410` | Rendering is flushed from `updateLayer`. The part for macOS decides whether the FerroUI member goes too; a removal is listed in that PR |
| Generated Android `Resource` type removed | `4ae41027c0` | Does not apply: FerroUI has no generated .NET resource type |

Changes of behaviour that the baseline does not record:

| Upstream change | Commit | Meaning for FerroUI |
|---|---|---|
| `FlexBasisKind.Relative` is a fraction (`0.5`), no longer a percentage (`50`) | `7007223002` | Follows upstream. A parsed `50%` was divided by 100 twice and now works; code that builds the value with `50` must pass `0.5` |
| `Window.IsActive` is true when `Activated` is raised | `48132c0500` | Follows upstream |
| `PART_TopBar` and `PART_Backdrop` of DrawerPage, and the matching parts of NavigationPage, are `Decorator` | `b6fa4b12ea` | Widening; existing templates keep working |
| A masked `TextBox` reports its mask to accessibility, not its text | `51f5c8f123` | FerroUI reports no value at all for a password field, also while revealed. Its remaining paths are fixed in PR 1317 |
| Space key up clicks a button only if the button was pressed | `c4dfde67c3` | Follows upstream |
| TwoWay bindings do not write a just-published value back | `500e78afef` | Follows upstream |
| Fluent and Simple control resources merged into the theme dictionary | `719787dc69` | Follows upstream (PR 1408). `Controls/FluentControls.xaml` and `Controls/SimpleControls.xaml` are resource dictionaries: an application that included one with `StyleInclude` uses `ResourceInclude` or `MergeResourceInclude`. The theme units of the trimming stay one per control document (the merge records the innermost origin, as for the WinUI theme) |
| Key events carry key text, alternate text, consumed modifiers and platform data | `8dae868308` | Follows upstream (PR 1409); `KeyEventArgs` has four more public fields |
| `SwapChainGlSurface` is gone; the WinUI swap chain panel renders through the composition swap chain render target | `db0a9a9cdb` | Follows upstream. `ferroui_win32::open_gl::angle::SwapChainGlSurface` is removed: a host uses `ferroui_win32::direct_x::CompositionSwapchainSurface` over its own `ISwapchainVisualHost`. `SwapChainTopLevelImpl` reports `Transparent` |
| The region dirty rectangle tracker inflates by a pixel inside the surface | `070c3be990` | Follows upstream: the inflation was undone by intersecting with the rectangle itself. A target that clips with regions repaints one more pixel around each rectangle. `RenderTargetProperties` has a new field (`supports_partial_presentation`) |

## New upstream features and their state

Features merged in this range:

| Feature | Upstream PR | State |
|---|---|---|
| Fluent 2 theme with its ColorPicker themes | #21771, #22569, #22611 | No new theme (owner decision below). Audited: the fixes that apply to the existing themes are ported (`ButtonSpinner` directions, `EmbeddableControlRoot` transparency fallback in Fluent and WinUI, `ColorSpectrum` tooltip, #22578); the rest is `already` or `not-applicable` |
| Variable fonts: axes, instances, gvar, HVAR, MVAR, VVAR, `FontVariations` | #22179 to #22185 | axes, instances and instance clones `port` (PR 1376, 0.6.0); gvar, HVAR, MVAR, VVAR `skip-better`; `FontVariations` `port` (PR 1397): Skia and Vello, design defaults on Quartz and Direct2D |
| Backdrop effects | #21826 | `deferred`: needs a GPU path in Skia and Vello |
| Key text, consumed modifiers, native key data | #22628, #22782 | `port` (PR 1409): Windows, macOS, X11, Wayland |
| Opt-in EGL partial presentation | #22501 | `port` (part `egl-partial`): X11, Wayland and DRM with Skia on EGL; Vello GL presents whole buffers; to validate on Mesa and a DRM device |
| `Win32PlatformOptions.UseCompositionSwapchain` | #22115 | `port` (part `composition-swapchain`): DirectComposition, Windows.UI.Composition and the WinUI swap chain panel; to validate in the Windows VM |
| Android desktop drag and drop | #22451 | `port` (part `android-dnd`): drops from other applications and drags of the application; to validate on a device with a pointer |
| Platform-simulated typefaces | #22773 | `deferred`: follows the web fonts stack |
| Compositor output rotation | #22499 | `port` (PR 1398) |
| Android cursors and TalkBack roles | #22438, #22681 | `port` |
| Paragraph and first-line indents | #22870 | `port` |
| TableView column limits and row automation name | #22707, #22709 | `port` |
| `AutoCompleteBox.UseFloatingPlaceholder` | #22305 | `port` |
| `TabStripItem.Icon` and tab icons in the themes | #22574 | `port` |
| Per-area log verbosity | #22523 | `port` |
| TreeViewItem expand and collapse provider | #22565 | `port` |
| `LabeledBy` in Windows UI Automation | #22671 | `port` |
| iOS touch delivery at the display refresh rate | #22412 | `port` |

The 12.2.0 release notes also list 52 pull requests merged before `66b7273b9a` (FlexPanel,
composition brushes, typed binding expressions, visual geometry hit testing and others). They
are part of the base that the previous sync tracked, so they are not inventoried here.

## Deferred, and what each needs

<!-- generated:deferred -->
| Commit | PR | Date | Area | Part | Note |
|---|---:|---|---|---|---|
<!-- /generated:deferred -->

## Decisions of the owner (2026-11-23)

1. **Fluent 2.** FerroUI has its Fluent 2 theme, so there is no new theme crate. Each Fluent 2
   commit is checked for fixes that apply to the existing Fluent or WinUI themes.
2. **Variable fonts.** The public API (`FontVariations`, axes, named instances, animation) is
   added on the existing Skia, Vello and HarfBuzz support, with one variation model shared with
   the web fonts stack. Table readers are added only where a backend cannot answer.
3. **Minimum macOS version.** FerroUI keeps its minimum (x86_64 10.13, arm64 11.0). A newer API
   is guarded at run time with a working fallback.

`deferred` is not final: the deferred commits are worked through after the ports, and one is
left out only with a recorded performance or better-code reason.

## Inventory

<!-- generated:inventory -->
| Commit | PR | Date | Area | Class | Part | Note |
|---|---:|---|---|---|---|---|
| `dd91d9dc44` | #22448 | 2026-10-19 | platform-x11 | `port` (done) | x11 | Read _NET_WORKAREA through the typed property helper; `update_work_area` still asks for any type. |
| `612785a8e2` | #22468 | 2026-10-19 | platform-browser | `port` (done) | browser2 | `PlatformHotkeyConfiguration::apple` and `KeyGestureFormatInfo::apple`; the browser on Apple platforms and iOS use Command. |
| `f5807681dc` | #22488 | 2026-10-19 | controls | `port` (done) | controls2 | Window content measured again when the platform constrains the size. |
| `d931166834` | #22337 | 2026-10-19 | controls | `port` (done) | controls2 | TabbedPage leaves arrow keys to a focused child. |
| `edc20c924d` | #22412 | 2026-10-19 | platform-ios | `port` (done) | ios2 | Touch display link at the display rate, inertia scaled by elapsed time, 60 Hz warning per screen. |
| `82c96fe15d` | #22419 | 2026-10-19 | platform-win32 | `already` |  | `get_dpi_for_monitor` in `interop/unmanaged_methods.rs` already declares the 32-bit HRESULT. |
| `33371cbfb7` | #22517 | 2026-10-19 | platform-macos | `port` (done) | macos | `automation.mm`: menu role for context menus. |
| `19952840c5` | #22478 | 2026-10-19 | platform-browser | `not-applicable` |  | Queue between a managed UI thread and the DOM thread. FerroUI dispatches input inside the DOM event (`browser-render-worker.md`, `browser_input_handler.rs`). |
| `da7ea04e6d` | #22497 | 2026-10-19 | controls | `already` |  | `ClassicDesktopStyleApplicationLifetime::start` takes the token after showing the window and skips the loop when it is gone. |
| `b72f1fd4d8` | #22470 | 2026-10-19 | platform-browser | `port` (done) | browser2 | Blob streams seek and read in place; the write, seek and length fixes were already in FerroUI. |
| `b6ed89b8d9` | #22457 | 2026-10-19 | controls | `port` (done) | controls | `Popup`: the Y scale read M11/M12. |
| `b13194be25` | #22453 | 2026-10-19 | platform-x11 | `port` (done) | x11b | The X11 run loop implements explicit background processing, with the restructured wait of upstream. |
| `f547369621` | #22496 | 2026-10-19 | threading | `already` |  | `Dispatcher::replace_implementation` already moves the due times of the queue and of every timer by the difference of the two clocks. |
| `ee85f0604a` | #22504 | 2026-10-19 | rendering | `port` (done) | rendering | Skia geometry: path bounds computed before sharing; stroke cache kept valid on failure. |
| `9b3e9c881b` | #22499 | 2026-10-19 | rendering | `port` (done) | rotation | Compositor output rotation: `RenderTargetProperties::required_rotation_for_orientation`; a rotated target draws the layer and the overlays with the quarter-turn transform and redraws on a change; the DRM output reports its orientation and drops its own rotation framebuffer and shaders; Skia and Vello GL targets forward it. |
| `be0c42891f` | #22522 | 2026-10-19 | build | `not-applicable` |  | Upstream repository workflow. |
| `b515c3ae17` | #22523 | 2026-10-20 | controls | `port` (done) | controls2 | Per-area log verbosity for `log_to_trace` and `log_to_delegate`. |
| `ba4fedb31e` | #22529 | 2026-10-20 | platform-ios | `port` (done) | platforms2 | Keyboard stays up when focus moves between text boxes. |
| `ae55790316` | #22526 | 2026-10-20 | platform-ios | `port` (done) | ios2 | Touches in progress are cancelled when the view leaves its window or is disposed. |
| `45693b1713` | #22196 | 2026-10-20 | build | `not-applicable` |  | .NET 11 SDK and .NET WebAssembly runtime. |
| `fa6d6f069e` | #22532 | 2026-10-20 | platform-macos | `port` (done) | macos | `automation.mm`: AXFocused settable only on keyboard-focusable elements. |
| `dc0105179b` | #22519 | 2026-10-20 | platform-macos | `port` (done) | native-com | microcom-codegen gets the smart pointer wrapper mode of MicroCom 0.11.7 (raw `impl_*` slots, `ComPtr` wrappers, `MicroComImpl<I>` adapters with `@cpp-call-preamble`), reproduced from the text templates of the published 0.11.7 generator package (read as data, not committed) and the upstream sources; ferroui-native generates in that mode. The native sources took the commit by a three-way merge (8 files with conflicts, resolved), and the FerroUI additions moved to `MicroComImpl` and `ComPtr` signatures. Not compiled locally (`ferroui-native` has a Skia edge): CI is the first compile. |
| `1fd3c0830e` | #22227 | 2026-10-20 | data | `port` (done) | data | Indexer bindings refresh on `Item[]` and `Item[key]`. |
| `06c2072375` | #22531 | 2026-10-20 | platform-win32 | `port` (done) | win32 | OLE data object accepts a combination of media. |
| `5eedd4afe5` | #22463 | 2026-10-20 | platform-macos | `port` (done) | macos | `automation.mm`: no announcement for an empty live region. |
| `fa9c3c1572` | #22540 | 2026-10-20 | build | `not-applicable` |  | Upstream NuGet publishing action. |
| `a9c52297f5` | #22548 | 2026-10-21 | platform-macos | `port` (done) | macos | `automation.mm`: accessibility activation point. |
| `51f5c8f123` | #22458 | 2026-10-21 | automation | `skip-better` |  | FerroUI discloses less: providers report no value for a password field, also while revealed (`text_box_automation_peer.rs`). The cached and input-method paths were fixed in PR 1317. |
| `6cef02f424` | #22427 | 2026-10-21 | input | `port` (done) | input | Tap and hold compare root positions, so a moving target still taps. |
| `565b1cf0b1` | #22428 | 2026-10-21 | controls | `port` (done) | controls | ComboBox clears `:pressed` on capture loss. |
| `d3fcf74a91` | #22553 | 2026-10-21 | text | `port` (done) | text | Paragraph end at the paragraph level (rule L1); runs report their direction from the bidi level; hit testing and the line break read the logical end of the line. |
| `88a7967b3e` | #22487 | 2026-10-21 | text | `port` (done) | text | `Typeface` tolerates repeated whitespace in family names. |
| `4573d1f752` | #22455 | 2026-10-21 | controls | `port` (done) | controls | NUL characters removed from window titles. |
| `c4dfde67c3` | #22456 | 2026-10-21 | controls | `port` (done) | controls | Space key up clicks only a pressed button. |
| `c2a6e2916e` | #22555 | 2026-10-21 | platform-x11 | `port` (done) | x11 | Window position read again after reparenting. |
| `72ebf9aad9` | #22438 | 2026-10-21 | platform-android | `port` (done) | android2 | Pointer icons for standard and bitmap cursors (API 24). |
| `0d6c2aaa81` | #22433 | 2026-10-21 | text | `already` |  | `TextLayout::hit_test_point` already passes a point relative to the line origin and subtracts `start`; upstream's cases are added as tests in `text_layout_tests.rs`. |
| `26b4ed5b35` | #22491 | 2026-10-21 | layout | `port` (done) | controls | Layout pass tolerates a control moved to another top level. |
| `9135170769` | #22557 | 2026-10-21 | rendering | `port` (done) | rendering | `ICustomHitTest` documentation: local coordinates. |
| `74c62ffd26` | #22552 | 2026-10-22 | input | `port` (done) | input | Gesture unit tests that replace the removed integration tests. |
| `78316701f3` | #22451 | 2026-10-22 | platform-android | `port` (done) | android-dnd | Desktop drag and drop on Android: `FerroDragListener` (Java) forwards the drag events of the surface view; `AndroidDragDrop` raises enter, over, leave and drop (the description's formats before the drop, the clip data with the drop permissions held during the drop), `AndroidDragSource` starts a drag with `View.startDragAndDrop`; the clip data building moved to `ClipDataHelper`. Written without a device: the drag needs an emulator or device with a pointer. |
| `103cf72903` | #22520 | 2026-10-22 | platform-win32 | `port` (done) | win32 | Alt+Space opens the system menu unless handled. |
| `52c057b1cd` | #22556 | 2026-10-22 | themes | `port` (done) | themes2 | CommandBar overflow scrolls in Fluent and Simple. |
| `f874d44497` | #22435 | 2026-10-22 | platform-android | `port` (done) | android | `ACTION_SET_TEXT` replaces the text. |
| `e7cd2df08e` | #22489 | 2026-10-22 | controls | `port` (done) | controls2 | Keyboard works inside the date, time and calendar picker popups. |
| `d8f2fc7b9f` | #22554 | 2026-10-22 | text | `port` (done) | text | Bounded lookahead (4096) on the enumerator and on the break table of a run, so a long unbroken run does not rescan the rest of the text per line. |
| `e4452b5bdb` | #22429 | 2026-10-22 | input | `port` (done) | input | Double tap needs the same button for both clicks. |
| `bd835340fb` | #21771 | 2026-10-22 | themes | `port` (done) | themes4 | Fluent 2 theme: no new theme crate (owner). Ported to the existing code: the `ButtonSpinner` spin buttons spin in the direction of their part. Not taken: the template, resources and samples of the new theme; the `Grid.Column` removal in the Fluent `ButtonSpinner` template (no effect inside a `DockPanel`). |
| `136efefc7d` | #22378 | 2026-10-22 | automation | `port` (done) | automation | ContentControl name no longer falls back to a type name. |
| `7fda9c36a5` | #22562 | 2026-10-23 | automation | `port` (done) | platforms2 | UI Automation `Invoke` checks the provider and the enabled state, then posts the click, so a modal dialog no longer blocks the client. |
| `74b4e97962` | #22400 | 2026-10-23 | samples | `port` (done) | themes4 | Catalog ColorPicker page redesign: the upstream 12.2.0 document converted by `scripts/convert_catalog_xaml.py`; the code-behind adds no picker any more. |
| `6a25cf4d9b` | #22399 | 2026-10-23 | controls | `port` (done) | controls2 | A delayed submenu action no longer closes a reopened menu. |
| `c1e4fab24c` | #22577 | 2026-10-23 | collections | `not-applicable` |  | `FerroList::contains` and `index_of` take `&T`; an invalid item cannot be passed. |
| `1def5f851b` | #22426 | 2026-10-25 | controls | `port` (done) | controls2 | PipsPager sizing and scroll reset. |
| `b88f813a95` | #22595 | 2026-10-25 | controls | `port` (done) | controls | VirtualizingStackPanel: condition that was always true. |
| `13c99cfe42` | #22594 | 2026-10-25 | platform-linux | `already` |  | `DBusTextInputMethodBase::dispose` takes the name first and logs a failed disconnect. |
| `b7a70cd06b` | #22591 | 2026-10-25 | themes | `already` |  | The Fluent DropDownButton already has its padding on the content presenter; the Fluent half of the commit is a comment, the rest is its Fluent 2 theme. |
| `584ae6f3a7` | #22596 | 2026-10-25 | platform | `port` (done) | controls2 | `FallbackStorageProvider` under concurrent calls. |
| `6b6c573a03` | #22579 | 2026-10-25 | controls | `port` (done) | controls2 | ColorPicker fixes. |
| `99086ac183` | #22356 | 2026-10-26 | controls | `port` (done) | controls2 | AutoCompleteBox keeps focus when an item is picked. |
| `fb506057ea` | #22608 | 2026-10-26 | automation | `port` (done) | automation | Automation peers use `set_current_value`, so bindings survive. |
| `e61863dff7` | #22612 | 2026-10-27 | samples | `port` (done) | samples2 | The calendar date picker page has its own view model; `[Required]` is still not ported. |
| `0a728569fc` | #22619 | 2026-10-27 | tests | `not-applicable` |  | Dispatcher leak in the upstream xUnit fixtures. |
| `c67550d9ec` | #22604 | 2026-10-27 | base | `already` |  | Rust `str` comparisons are ordinal (`iteration_count.rs`, `classes.rs`). |
| `6b41bcb0be` | #22593 | 2026-10-27 | platform-win32 | `already` |  | `query_active_display_paths` in `interop/unmanaged_methods.rs` passes a null topology. |
| `db0a9a9cdb` | #22115 | 2026-10-27 | platform-win32 | `port` (done) | composition-swapchain | `Win32PlatformOptions::use_composition_swap_chain`: `CompositionSwapchainRenderTarget` renders into a flip-model swap chain for composition shown by an `ISwapchainVisualHost` (the DirectComposition window, the Windows.UI.Composition window through `CreateCompositionSurfaceForSwapChain`, the WinUI swap chain panel); `SwapChainGlSurface` removed, the panel renders through `CompositionSwapchainSurface`; `SwapChainTopLevelImpl` reports `Transparent`. The Windows.UI.Composition brush of a swap chain does not stretch (issue 476). Windows VM validation owed. |
| `75ea7e186c` | #22549 | 2026-10-27 | build | `not-applicable` |  | Roslyn analyzer. Rust has no unchecked cast to warn about. |
| `951ee3f067` | #22609 | 2026-10-27 | controls | `port` (done) | controls | Calendar waits for a whole wheel notch. |
| `626c94d77c` | #22545 | 2026-10-27 | text | `already` |  | `bidi_algorithm.rs` already resolves W2 in one forward pass. |
| `f3eca207d0` | #22307 | 2026-10-27 | samples | `not-applicable` |  | Catalog switching to the Fluent 2 theme: owner decision, no Fluent 2 theme crate. |
| `b6fa4b12ea` | #22621 | 2026-10-27 | controls | `port` (done) | controls2 | DrawerPage and NavigationPage template parts relaxed to Decorator. |
| `7e8b76db3c` | #22379 | 2026-10-27 | platform-x11 | `already` |  | `X11AtSpiAccessibility` starts its server and adds the tracked windows in a task on the UI thread (`spawn_local_with_priority`). |
| `a9d935b5c7` | #22423 | 2026-10-27 | controls | `port` (done) | controls | `ComboBox.WrapSelection` with nothing to select. |
| `ca9149f2fa` | #22625 | 2026-10-28 | platform-linux | `port` (done) | linux | IBus expects evdev keycodes (X11 keycode minus 8). |
| `da1dafcdfb` | #22629 | 2026-10-28 | tests | `not-applicable` |  | Removal of the upstream Appium suite. |
| `0e0dad08ca` | #22631 | 2026-10-28 | themes | `already` |  | `SystemAccentColors::try_get_resource` matches typed resource keys; no culture comparison. |
| `b0ad044a65` | #22626 | 2026-10-28 | text | `not-applicable` |  | Pooled-buffer disposal. `ShapedBuffer` halves are `Rc` handles released by ownership. |
| `5d6b56563b` | #22630 | 2026-10-28 | themes | `already` |  | Ordinal key comparison in the Fluent 2 resource providers: FerroUI compares resource keys as Rust strings, which is ordinal. |
| `5865a87387` | #22632 | 2026-10-28 | build | `not-applicable` |  | Upstream test projects removed. |
| `1efe0dfabb` | #22633 | 2026-10-28 | build | `not-applicable` |  | Upstream Xcode target. FerroUI sets its own minimum in `src/FerroUI.Native/build.rs`; raising it is an owner decision. |
| `725b5975e4` | #22611 | 2026-10-28 | themes | `not-applicable` |  | Brush resources of the Fluent 2 theme, which FerroUI does not add (owner decision); the Fluent and WinUI themes keep their own brushes. |
| `d1cc67f3ae` | #22634 | 2026-10-28 | build | `not-applicable` |  | Upstream solution filter. |
| `1c82a11d87` | #22636 | 2026-10-28 | themes | `not-applicable` |  | Light-theme caption button and notification brushes of the Fluent 2 theme only; the Fluent and WinUI themes have their own resources for them. |
| `c85e221c60` | #22565 | 2026-10-28 | automation | `port` (done) | automation | TreeViewItem expand and collapse provider. |
| `23dce09edc` | #22569 | 2026-10-29 | themes | `port` (done) | themes4 | Fluent 2 ColorPicker themes: not added (owner). Ported: the `ColorSpectrum` templates of Fluent and Simple keep the code-behind tooltip as a comment, and the `Rgb`/`Hsv` doc typo; the other fixes came with `1968b08e18`. |
| `c9c7b90053` | #22641 | 2026-10-29 | platform-linux | `port` (done) | linux | AT-SPI: popups report the window role. |
| `5aa47e4f7f` | #22642 | 2026-10-29 | platform-linux | `port` (done) | linux | AT-SPI: active state only for the active window. |
| `1968b08e18` | #22578 | 2026-10-29 | themes | `port` (done) | themes3 | ColorPicker Fluent 2 fixes in the Fluent and Simple colour picker themes (theme dictionaries for the base resources). |
| `719787dc69` | #22637 | 2026-10-29 | themes | `port` (done) | merge-nested | Compiler: a merged document is flattened before it is inlined, recursively, relative sources against its own URI, circular merges reported; entries recorded with the innermost origin, so the theme units stay per control document. Themes: the Fluent and Simple controls documents are resource dictionaries merged into the theme dictionary (as the WinUI theme already does, so not against trimming); duplicate `MenuFlyoutScrollerMargin` and the Fluent `PipsPager` `Light` dictionary removed. The Fluent 2 palettes have no theme crate. |
| `55129e0659` | #22229 | 2026-10-30 | platform-win32 | `port` (done) | win32b | Move and resize drags wait for the emulated left button of touch and pen. |
| `2a38dc63eb` | #22624 | 2026-10-30 | controls | `port` (done) | controls2 | Popup with pointer placement resizes with its content. |
| `eae84c1403` | #22179 | 2026-10-30 | text | `port` (done) | variation-axes | Public axis API (`FontVariationAxis`, `FontVariationInstance`, `GlyphTypeface::variation_axes`, `named_instances`) on the Skia and Vello axis queries; PR 1376 (0.6.0). No managed fvar or avar parser. |
| `d2ee79eb46` | #22180 | 2026-10-30 | text | `port` (done) | variation-axes | `GlyphTypeface::with_variation` on `IPlatformTypeface::with_variation_settings`; PR 1376 (0.6.0). |
| `64b21ba1cc` | #22181 | 2026-10-30 | text | `skip-better` |  | Managed gvar outlines: Skia and Vello draw variable outlines natively at the position of the typeface. |
| `9ce8ffda39` | #22182 | 2026-10-30 | text | `skip-better` |  | Managed HVAR advances: HarfBuzz, Skia and Vello apply them natively. |
| `ee4f35bfdd` | #22183 | 2026-10-30 | text | `skip-better` |  | Managed MVAR metrics: the backends read varied metrics natively. |
| `408f2494f0` | #22184 | 2026-10-30 | text | `skip-better` |  | Managed VVAR advances: HarfBuzz applies them natively. |
| `dfee7f224b` | #22185 | 2026-10-30 | text | `port` (done) | font-variations | `FontVariations`: inherited attached property like `FontFeatures` (same owners), folded into the `Typeface` (one pointer), resolved by the font manager to a bounded per-typeface instance cache, carried through fallback and `FormattedText`; markup value type; animator selected by type and `FontVariationSettingsTransition` with CSS interpolation. Quartz and Direct2D make no instances: design defaults there. |
| `37279ab420` | #22650 | 2026-10-30 | themes | `port` (done) | themes4 | `EmbeddableControlRoot` sets `TransparencyBackgroundFallback` in the Fluent and WinUI themes (to the value of their `Window` theme), as the Fluent 2 fix does. |
| `432480ae82` | #22574 | 2026-10-30 | controls | `port` (done) | controls2 | `TabStripItem.Icon` and `IconTemplate`; icon presenter in the Fluent tab templates (two more elements per tab: start-up check owed). |
| `5b36bbe0ad` | #22683 | 2026-11-03 | platform-ios | `port` (done) | platforms2 | Accessibility traversal skips interop peers. |
| `99b0962977` | #22686 | 2026-11-03 | rendering | `already` |  | `immutable_bitmap.rs` returns an error from `Codec::from_data` on null. |
| `0a53cc4730` | #22687 | 2026-11-03 | platform-linux | `port` (done) | linux | AT-SPI: X11 window id attribute. |
| `c3b6f5cf61` | #22710 | 2026-11-05 | markup | `port` (done) | xamlx | Inline generic arguments and nullable indicators in `x:Type` and `x:Static`, Nullable checks, in the Rust port of XamlX. |
| `c9188ebe5f` | #22724 | 2026-11-05 | markup | `not-applicable` |  | IL emitter overload choice on Mono. |
| `330e602179` | #22652 | 2026-11-05 | input | `port` (done) | input | Alt with another modifier does not open the menu. |
| `60cb9d62b2` | #22731 | 2026-11-06 | build | `not-applicable` |  | Upstream package feeds. |
| `f5af0ebf98` | #22707 | 2026-11-06 | controls | `port` (done) | controls2 | TableViewColumn minimum and maximum widths. |
| `c4d24e9dab` | #22709 | 2026-11-06 | automation | `port` (done) | automation2 | `TableViewRowAutomationPeer`: a data item named after its visible cells. |
| `727fea6125` | #22741 | 2026-11-07 | platform-browser | `already` |  | `webapp/modules/ferroui/dom.ts` already returns left, top, right, bottom. |
| `a81ceeaf3c` | #22656 | 2026-11-09 | platform-browser | `port` (done) | platforms2 | Browser clipboard works without `navigator.clipboard` (non-secure contexts); file items of a paste event are copied before the event ends. |
| `48132c0500` | #22716 | 2026-11-09 | controls | `port` (done) | controls | `IsActive` set before `Activated` is raised. |
| `69be03931f` | #22736 | 2026-11-09 | platform | `port` (done) | controls | Documentation of glob patterns. |
| `c9b5ae7b21` | #22746 | 2026-11-09 | controls | `already` |  | `Span::on_property_changed` compares the property, so `on_inlines_changed` runs. |
| `ae5ae6124e` | #22757 | 2026-11-09 | rendering | `port` (done) | rendering | Bitmap copy guards, pixel writer reset and EDID length (`rendering`); Win32 icon mask layout (`win32b`). GDI handle lifetimes in OLE were already owned by `ScreenContexts`. |
| `dc061bc3ef` | #22698 | 2026-11-09 | rendering | `already` |  | `sk_round_rect_cache.rs` passes the radii without a heap array. |
| `1b53d56525` | #22688 | 2026-11-09 | platform-android | `already` |  | `StorageHelper.java` runs every query in try-with-resources, so the cursors are closed. |
| `5bbaa8e331` | #22743 | 2026-11-09 | rendering | `already` |  | `LockGuard` in `vulkan_khr_surface_render_target.rs` releases the lock on every exit. |
| `9186fcf61f` | #22756 | 2026-11-09 | themes | `already` |  | Week numbers in the wrong column of the Fluent 2 calendar: the Fluent and WinUI `CalendarItem` templates of FerroUI already put the week numbers in a column of their own. |
| `66f1f2a9dd` | #22763 | 2026-11-09 | rendering | `port` (done) | rendering | `ImmutableRadialGradientBrush` copied `radius_x` into `radius_y`. |
| `c93b0d6c52` | #22764 | 2026-11-09 | rendering | `port` (done) | rendering | Visual cache layer: the vertical scale was the horizontal one. |
| `45ddd45380` | #22659 | 2026-11-09 | rendering | `not-applicable` |  | Finalizer race of the .NET Skia binding. Rust ownership keeps the stream alive. |
| `b6a5fbc191` | #22740 | 2026-11-09 | rendering | `not-applicable` |  | `GC.KeepAlive`. Rust borrows keep the bitmap alive. |
| `b490bc6733` | #22748 | 2026-11-10 | platform-x11 | `port` (done) | x11 | Two-finger scrolling with the synaptics driver. |
| `2061fb5090` | #22760 | 2026-11-10 | text | `port` (done) | text | Wrap opportunity between shaped runs, decided from a window of both runs. The per-run break tables stay, so multi-run lines do not fall back to the enumerator. |
| `bdeeeef0a7` | #22693 | 2026-11-10 | platform-ios | `port` (done) | ios2 | Native control views listed for VoiceOver; `accessibilityElements`. |
| `80c4f31848` | #22462 | 2026-11-10 | platform-x11 | `port` (done) | x11 | ConfigureNotify older than our last resize ignored. |
| `0dc28102b4` | #22566 | 2026-11-10 | platform-linux | `port` (done) | x11b | DRM output: the next frame renders while the flip just queued waits for vblank; the wait happens before the next flip, or at once when no free buffer is left. |
| `8dae868308` | #22628 | 2026-11-10 | input | `port` (done) | key-info | `KeyEventArgs::key_text`, `alternate_key_text`, `consumed_modifiers`, `platform_key_info` (`PlatformKeyInfo`, `KnownPlatformKeyInfoFormats`) on Windows (paired `WM_CHAR`, layout table built lazily per layout instead of at start-up, dead keys), X11 (GTK consumed rules), Wayland (`WaylandKeyInfo`, held key changes) and macOS (native patch 0009, IDL `FrnKeyEventInfo`). |
| `5ef345810e` | #22782 | 2026-11-12 | platform-win32 | `port` (done) | key-info | Win32 layout table: none while a dead key is pending (Windows 10 1607 and later), a table for a layout added after start-up. Its CI and integration test changes have no FerroUI counterpart; the layouts are checked in the Windows VM. |
| `3ecf2a3fd8` | #22777 | 2026-11-12 | build | `not-applicable` |  | Empty upstream CI commit. |
| `b04cec5ef5` | #22801 | 2026-11-12 | build | `not-applicable` |  | .NET SDK update. |
| `dc2253658c` | #22674 | 2026-11-12 | markup | `port` (done) | xamlx | The enum whitespace trim of its XamlX update is ported; compact IL identifiers do not apply (the Rust emitter writes no IL). |
| `27b1179321` | #22558 | 2026-11-12 | platform-linux | `port` (done) | linux | StatusNotifierItem tooltip text. |
| `22e9912b75` | #22725 | 2026-11-12 | markup | `not-applicable` |  | Exact signature lookups in the IL compiler. |
| `050d3fb941` | #22798 | 2026-11-12 | rendering | `port` (done) | rendering | Skia: one layer less for each opacity mask. Performance gain. |
| `ee79f65f26` | #22685 | 2026-11-12 | platform-android | `port` (done) | android | Input type no longer forced to a password variation. |
| `84f262c218` | #22676 | 2026-11-12 | markup | `not-applicable` |  | Function-pointer factories for IL-compiled property info. FerroUI generates Rust. |
| `24b6c7b410` | #22610 | 2026-11-12 | platform-macos | `port` (done) | render-flush | `updateLayer` runs the render pass with dispatcher processing disabled (`MediaContext::paint_with_render_pass`); not skip-better: FerroUI ran the same jobs, and the cooperative resize (#476) keeps its frame. Native patch 0008. |
| `070c3be990` | #22501 | 2026-11-12 | rendering | `port` (done) | egl-partial | Opt-in EGL partial presentation: `IPartialPresentationFeature`/`IPartialPresentationBuffer`, `RenderTargetProperties::supports_partial_presentation`, `PartialPresentationDirtyRectHistory` in the composition target (replay of the frames a buffer missed, rotated output rectangles), tracker `copy_rects`/`copy_uninflated_rects` and the region tracker inflation fix; EGL buffer age, `eglSetDamageRegionKHR`, swap with damage (KHR/EXT) in `EglPlatformSurfaceRenderTargetBase` and the DRM output; Skia GL forwards the feature; `use_partial_presentation` of X11, Wayland (aborted frame callback) and DRM. Vello GL targets do not take it. Linux/Android GPU validation owed. |
| `ca61365d75` | #22538 | 2026-11-12 | rendering | `port` (done) | rendering | Animated value written even when an expression has read it. |
| `61c3aaa876` | #22810 | 2026-11-13 | rendering | `port` (done) | rendering | Remove `CompositorDrawingContextProxy`: on main only its own tests construct it. |
| `6350ac83e6` | #22774 | 2026-11-13 | text | `port` (done) | text | `TextLineImpl::collapse` gives the collapsed line its own references and keeps the source line valid after a split; the layout releases the replaced line. |
| `a4f832f5d6` | #22795 | 2026-11-13 | input | `port` (done) | input | Tab navigation skips disabled elements. |
| `e73a976afd` | #22796 | 2026-11-13 | platform-macos | `already` |  | `trayicon.mm` already tests the text for null. |
| `60b1719596` | #22797 | 2026-11-13 | controls | `port` (done) | controls | No empty `SelectionChanged` during initialization. |
| `53e461e07a` | #22466 | 2026-11-13 | controls | `port` (done) | controls2 | Tab strip stays reachable with Tab. |
| `ab36f90feb` | #22701 | 2026-11-13 | controls | `port` (done) | controls | Grid: debug assertion allows a rounding residual. |
| `5ea0586b32` | #22699 | 2026-11-13 | themes | `port` (done) | themes2 | Fluent ButtonSpinner outer corner. |
| `a05d9d0046` | #22812 | 2026-11-13 | themes | `port` (done) | themes2 | Simple ComboBox and Slider are wrapped in `DataValidationErrors`. One more element per control in the Simple theme: theme start-up check owed. |
| `d0a3041701` | #22475 | 2026-11-13 | controls | `port` (done) | controls | RelativePanel chain with infinite available size. |
| `ad90a07d03` | #22671 | 2026-11-13 | automation | `port` (done) | platforms2 | `LabeledBy` in Windows UI Automation. |
| `10a2fb0ac5` | #22682 | 2026-11-13 | platform-win32 | `port` (done) | platforms2 | OLE `GetData` retried for 100 ms while another process holds the clipboard, then a timeout error instead of no data. |
| `e08cd25212` | #21826 | 2026-11-13 | rendering | `port` (done) | backdrop | Visual.BackdropEffect, BackdropEffectBoundsSource and BackdropEffectCache (default XML namespace), composition fields, own layout bounds, catalog page. Drawn with the WinUI backdrop replay (`render_scope::draw_backdrop`) into `push_effect` of the backend, on the GPU in Skia and Vello (Quartz and Direct2D through their effects too), read beyond the bounds by the sampling radius and cut to them; the dirty area grows over it as for the backdrop brush. Not ported: the retained snapshot (`IDrawingContextImplWithBackdropSupport`, `BackdropVolatilePass`, snapshot dirty tracking): the replay reads no pixels back, works on every backend and is bounded by the dirty area, so the cache mode is a hint (DEVIATIONS.md). |
| `80659c4c3e` | #22773 | 2026-11-13 | text | `port` (done) | typeface-sim | Split by the web fonts stack: ported there (PR 1417): no bold or oblique simulation on a colour face. Already: an oblique face answers an italic request, and fallback faces are not matched again. Skip-better: emboldening a real Bold face for ExtraBold (CSS synthesizes bold only for a face below 600). Deferred to the web fonts work: platform typeface simulation without copying the font (`IPlatformTypefaceSimulator`, the `FontCollectionBase` and `SystemFontCollection` changes). Ported here (backends): Skia family matches and character fallbacks share one wrapper that computes the simulations of the request (a fallback keeps them, oblique as italic), the simulations DirectWrite or fontconfig applied inside a typeface are detected from its OS/2 table and its variation axes, reported and not applied twice, and a simulated face reports the requested weight or style; Direct2D reports the simulations DirectWrite applies inside a system face (`d2_font_simulations`) and no longer applies them again. |
| `02d7bc35e3` | #22662 | 2026-11-13 | text | `port` (done) | text | Runs of different scripts shaped in separate buffers. |
| `9d6cb1ea62` | #22015 | 2026-11-13 | rendering | `port` (done) | rendering | `ContentPresenter` and acrylic clip to rounded bounds. A presenter pays for a border visual only when it clips with a corner radius. |
| `d379b92b87` | #22694 | 2026-11-13 | platform-macos | `port` (done) | macos | `automation.mm`: tab button subrole. |
| `500e78afef` | #22679 | 2026-11-13 | data | `port` (done) | data | TwoWay binding does not write a just-published value back. |
| `8035f14fd3` | #22690 | 2026-11-13 | platform-macos | `port` (done) | macos | `FrnWindow.mm`: only the view tracking area resets the cursor. |
| `4ae41027c0` | #22820 | 2026-11-16 | platform-win32 | `port` (done) | win32 | `dcomp.idl`: `BOOL`, not `bool`, for `CreateTargetForHwnd`. |
| `7011f50c27` | #22305 | 2026-11-16 | controls | `port` (done) | controls2 | `AutoCompleteBox.UseFloatingPlaceholder`. |
| `9f6a652216` | #22846 | 2026-11-16 | platform-browser | `already` |  | `webapp/modules/ferroui/timer.ts` already passes the timestamp. |
| `16d8678a44` | #22835 | 2026-11-16 | rendering | `port` (done) | rendering | Skia: multiply opacity applied once below an opacity mask. |
| `ab578c3ea0` | #22793 | 2026-11-17 | platform | `port` (done) | controls2 | Assembly resolved by exact name. |
| `75497e3634` | #22857 | 2026-11-17 | themes | `port` (done) | themes2 | Simple theme: missing resources. |
| `a49cbefe8b` | #22864 | 2026-11-17 | automation | `port` (done) | automation | Peer parent kept after a move. |
| `281e59e138` | #22866 | 2026-11-17 | build | `not-applicable` |  | Upstream contribution bot. |
| `fda56217a5` | #22872 | 2026-11-18 | build | `not-applicable` |  | Upstream workflows. |
| `251fda6155` |  | 2026-11-18 | build | `not-applicable` |  | Upstream contribution bot. |
| `a096f9bbde` | #22862 | 2026-11-18 | platform-android | `port` (done) | android2 | The view position on the screen translates accessibility bounds. |
| `92dc2b7207` | #22849 | 2026-11-18 | platform-android | `port` (done) | platforms3 | The root peer reports focus changes to TalkBack: the focused virtual view is invalidated and takes the keyboard focus. An interop peer counts as no focus. |
| `7007223002` | #22871 | 2026-11-18 | controls | `port` (done) | controls | FlexPanel relative basis is a fraction. Parsed percentages were divided by 100 twice; code passing `50` must pass `0.5`. |
| `192e7a0ba5` | #22880 | 2026-11-18 | platform-macos | `port` (done) | macos | Leaf nodes are not reported as expandable. |
| `2d34086103` | #22883 | 2026-11-19 | build | `not-applicable` |  | Upstream contribution guide. |
| `ea6f0daf9b` | #22888 | 2026-11-19 | platform-android | `port` (done) | platforms2 | The Choreographer timer no longer blocks start-up: the looper thread posts the first callback once it has the choreographer. |
| `5032926e38` | #22865 | 2026-11-19 | build | `not-applicable` |  | Upstream readme. |
| `3f984fa0d9` | #22870 | 2026-11-19 | text | `port` (done) | text | `ParagraphIndent` and first-line `Indent` honoured by the formatter, beside the paragraph styles of `TextLayout`. |
| `debc815ba9` | #22914 | 2026-11-21 | build | `not-applicable` |  | Upstream build services package. |
| `605e161019` | #22738 | 2026-11-23 | platform-android | `port` (done) | android2 | One render lock for ticks and surface changes; EGL without a window is not ready. The software framebuffer already discarded such frames. |
| `51dae7305e` | #22681 | 2026-11-23 | platform-android | `port` (done) | platforms3 | `android_access_role_helper.rs` maps controls to Android widget classes, so TalkBack announces their role. |
| `5d9e6d1b0e` |  | 2026-11-23 | build | `not-applicable` |  | Upstream version number. |
<!-- /generated:inventory -->
