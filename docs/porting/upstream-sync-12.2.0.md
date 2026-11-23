# Upstream sync to 12.2.0

The record of the sync from upstream commit `66b7273b9a` (the tracked commit since 2026-10-17)
to upstream release `12.2.0` (`5d9e6d1b0e`, published 2026-11-23): 182 commits, none of them a
merge commit. The inventory has one entry per commit, in
[`data/upstream-sync-12.2.0.json`](data/upstream-sync-12.2.0.json). The tables below are
generated from it by `scripts/port-status/upstream_sync.py`, which also refuses a commit that
is missing, listed twice or without a class.

This work targets milestone 0.7.0 as a stack of draft pull requests. Nothing in it is merged
or marked ready.

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
| `port` (69 done, 45 planned) | 114 |
| `already` | 16 |
| `skip-performance` | 0 |
| `skip-better` | 1 |
| `not-applicable` | 27 |
| `deferred` | 24 |
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
| input | 5 |  |  |  |  | 1 | 6 |
| layout | 1 |  |  |  |  |  | 1 |
| markup | 2 |  |  |  | 3 |  | 5 |
| platform | 3 |  |  |  |  |  | 3 |
| platform-android | 9 |  |  |  |  | 1 | 10 |
| platform-browser | 3 | 2 |  |  | 1 |  | 6 |
| platform-ios | 5 |  |  |  |  |  | 5 |
| platform-linux | 6 | 1 |  |  |  |  | 7 |
| platform-macos | 8 | 1 |  |  |  | 1 | 10 |
| platform-win32 | 6 | 2 |  |  |  | 1 | 9 |
| platform-x11 | 6 |  |  |  |  |  | 6 |
| rendering | 11 | 3 |  |  | 2 | 2 | 18 |
| samples | 1 |  |  |  |  | 2 | 3 |
| tests |  |  |  |  | 2 |  | 2 |
| text | 7 | 2 |  |  | 1 | 8 | 18 |
| themes | 6 | 1 |  |  |  | 8 | 15 |
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
| Fluent control resources merged into the theme dictionary | `719787dc69` | Deferred. Lookup order of a custom theme could change |

## New upstream features and their state

Features merged in this range:

| Feature | Upstream PR | State |
|---|---|---|
| Fluent 2 theme with its ColorPicker themes | #21771, #22569, #22611 | `deferred`: owner decision below |
| Variable fonts: axes, instances, gvar, HVAR, MVAR, VVAR, `FontVariations` | #22179 to #22185 | `deferred`: owner decision below |
| Backdrop effects | #21826 | `deferred`: needs a GPU path in Skia and Vello |
| Key text, consumed modifiers, native key data | #22628 | `deferred` |
| Opt-in EGL partial presentation | #22501 | `deferred` |
| `Win32PlatformOptions.UseCompositionSwapchain` | #22115 | `deferred` |
| Android desktop drag and drop | #22451 | `deferred` |
| Platform-simulated typefaces | #22773 | `deferred`: follows the web fonts stack |
| Compositor output rotation | #22499 | `port` |
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
| `dc0105179b` | #22519 | 2026-10-20 | platform-macos |  | Reference-counting refactor of the whole native layer (1,844 lines). Needs smart-pointer output from the FerroUI IDL generator and a macOS hand check. |
| `78316701f3` | #22451 | 2026-10-22 | platform-android |  | Desktop drag and drop on Android (549 lines of JNI). Needs a device or emulator with a pointer for validation. |
| `bd835340fb` | #21771 | 2026-10-22 | themes |  | Fluent 2 theme (16,105 lines). Owner decision: no new theme crate. Fixes that apply to the existing Fluent or WinUI themes are ported; the rest gets a recorded class. |
| `74b4e97962` | #22400 | 2026-10-23 | samples |  | Catalog ColorPicker page redesign. Rerun `scripts/sync-control-catalog.sh` after the ColorPicker theme work. |
| `db0a9a9cdb` | #22115 | 2026-10-27 | platform-win32 |  | `UseCompositionSwapchain` (769 lines). Needs the Windows VM and a frame-pacing measurement. |
| `f3eca207d0` | #22307 | 2026-10-27 | samples |  | Catalog theme switching. Depends on the Fluent 2 decision. |
| `5d6b56563b` | #22630 | 2026-10-28 | themes |  | Fluent 2 resource providers; follows the Fluent 2 decision. |
| `725b5975e4` | #22611 | 2026-10-28 | themes |  | Fluent 2 brushes; follows the Fluent 2 decision. |
| `1c82a11d87` | #22636 | 2026-10-28 | themes |  | Fluent 2 resources; follows the Fluent 2 decision. |
| `23dce09edc` | #22569 | 2026-10-29 | themes |  | Fluent 2 ColorPicker themes; follows the Fluent 2 decision. |
| `719787dc69` | #22637 | 2026-10-29 | themes |  | Theme resources merged into one dictionary. Changes lookup order and generated theme code; needs the theme size and start-up check first. |
| `eae84c1403` | #22179 | 2026-10-30 | text |  | Variable fonts: fvar and avar axis API. `FontVariationSettings` and backend variation exist; needs a design on top of them and PR 1252. |
| `d2ee79eb46` | #22180 | 2026-10-30 | text |  | Variable fonts: `GlyphTypeface` variation clones. |
| `64b21ba1cc` | #22181 | 2026-10-30 | text |  | Variable fonts: gvar outlines for managed glyph geometry. Skia and Vello already apply variations when drawing. |
| `9ce8ffda39` | #22182 | 2026-10-30 | text |  | Variable fonts: HVAR advances. |
| `ee4f35bfdd` | #22183 | 2026-10-30 | text |  | Variable fonts: MVAR metrics. |
| `408f2494f0` | #22184 | 2026-10-30 | text |  | Variable fonts: VVAR advances. |
| `dfee7f224b` | #22185 | 2026-10-30 | text |  | Variable fonts: `FontVariations` property, markup and animation. |
| `37279ab420` | #22650 | 2026-10-30 | themes |  | Fluent 2 embedded root; follows the Fluent 2 decision. |
| `9186fcf61f` | #22756 | 2026-11-09 | themes |  | Fluent 2 calendar; follows the Fluent 2 decision. |
| `8dae868308` | #22628 | 2026-11-10 | input |  | Key text, consumed modifiers and native key data (1,258 lines on every platform). Public event API; needs each platform checked. |
| `070c3be990` | #22501 | 2026-11-12 | rendering |  | Opt-in EGL partial presentation (1,390 lines). Needs GPU validation on Linux and Android. |
| `e08cd25212` | #21826 | 2026-11-13 | rendering |  | Backdrop effects (1,786 lines). Needs a GPU path in Skia and in Vello; must reuse the WinUI composition backdrop code. |
| `80659c4c3e` | #22773 | 2026-11-13 | text |  | Platform-simulated typefaces. Touches font matching beside the open web fonts stack; follows that stack. |
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
| `612785a8e2` | #22468 | 2026-10-19 | platform-browser | `port` | browser | Command-based hotkeys on Apple platforms in the browser; shared Apple gesture tables. |
| `f5807681dc` | #22488 | 2026-10-19 | controls | `port` (done) | controls2 | Window content measured again when the platform constrains the size. |
| `d931166834` | #22337 | 2026-10-19 | controls | `port` (done) | controls2 | TabbedPage leaves arrow keys to a focused child. |
| `edc20c924d` | #22412 | 2026-10-19 | platform-ios | `port` | ios | Touch drags and trackpad momentum at the display refresh rate. |
| `82c96fe15d` | #22419 | 2026-10-19 | platform-win32 | `already` |  | `get_dpi_for_monitor` in `interop/unmanaged_methods.rs` already declares the 32-bit HRESULT. |
| `33371cbfb7` | #22517 | 2026-10-19 | platform-macos | `port` (done) | macos | `automation.mm`: menu role for context menus. |
| `19952840c5` | #22478 | 2026-10-19 | platform-browser | `not-applicable` |  | Queue between a managed UI thread and the DOM thread. FerroUI dispatches input inside the DOM event (`browser-render-worker.md`, `browser_input_handler.rs`). |
| `da7ea04e6d` | #22497 | 2026-10-19 | controls | `already` |  | `ClassicDesktopStyleApplicationLifetime::start` takes the token after showing the window and skips the loop when it is gone. |
| `b72f1fd4d8` | #22470 | 2026-10-19 | platform-browser | `port` | browser | Blob stream: seeking, bounded reads, write position. |
| `b6ed89b8d9` | #22457 | 2026-10-19 | controls | `port` (done) | controls | `Popup`: the Y scale read M11/M12. |
| `b13194be25` | #22453 | 2026-10-19 | platform-x11 | `port` | x11 | X11 dispatcher: explicit background processing. |
| `f547369621` | #22496 | 2026-10-19 | threading | `already` |  | `Dispatcher::replace_implementation` already moves the due times of the queue and of every timer by the difference of the two clocks. |
| `ee85f0604a` | #22504 | 2026-10-19 | rendering | `port` | rendering | Skia geometry: path bounds computed before sharing; stroke cache kept valid on failure. |
| `9b3e9c881b` | #22499 | 2026-10-19 | rendering | `port` | rendering | Compositor output rotation through `RenderTargetProperties`. |
| `be0c42891f` | #22522 | 2026-10-19 | build | `not-applicable` |  | Upstream repository workflow. |
| `b515c3ae17` | #22523 | 2026-10-20 | controls | `port` (done) | controls2 | Per-area log verbosity for `log_to_trace` and `log_to_delegate`. |
| `ba4fedb31e` | #22529 | 2026-10-20 | platform-ios | `port` | ios | Keyboard stays up when focus moves between text boxes. |
| `ae55790316` | #22526 | 2026-10-20 | platform-ios | `port` | ios | Touches in progress are cancelled when the view leaves its window. |
| `45693b1713` | #22196 | 2026-10-20 | build | `not-applicable` |  | .NET 11 SDK and .NET WebAssembly runtime. |
| `fa6d6f069e` | #22532 | 2026-10-20 | platform-macos | `port` (done) | macos | `automation.mm`: AXFocused settable only on keyboard-focusable elements. |
| `dc0105179b` | #22519 | 2026-10-20 | platform-macos | `deferred` |  | Reference-counting refactor of the whole native layer (1,844 lines). Needs smart-pointer output from the FerroUI IDL generator and a macOS hand check. |
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
| `72ebf9aad9` | #22438 | 2026-10-21 | platform-android | `port` | android | Cursors on Android (`cursor_factory.rs` is a stub). |
| `0d6c2aaa81` | #22433 | 2026-10-21 | text | `already` |  | `TextLayout::hit_test_point` already passes a point relative to the line origin and subtracts `start`; upstream's cases are added as tests in `text_layout_tests.rs`. |
| `26b4ed5b35` | #22491 | 2026-10-21 | layout | `port` (done) | controls | Layout pass tolerates a control moved to another top level. |
| `9135170769` | #22557 | 2026-10-21 | rendering | `port` | rendering | `ICustomHitTest` documentation: local coordinates. |
| `74c62ffd26` | #22552 | 2026-10-22 | input | `port` (done) | input | Gesture unit tests that replace the removed integration tests. |
| `78316701f3` | #22451 | 2026-10-22 | platform-android | `deferred` |  | Desktop drag and drop on Android (549 lines of JNI). Needs a device or emulator with a pointer for validation. |
| `103cf72903` | #22520 | 2026-10-22 | platform-win32 | `port` (done) | win32 | Alt+Space opens the system menu unless handled. |
| `52c057b1cd` | #22556 | 2026-10-22 | themes | `port` | themes | CommandBar overflow scrolls in Fluent and Simple. |
| `f874d44497` | #22435 | 2026-10-22 | platform-android | `port` (done) | android | `ACTION_SET_TEXT` replaces the text. |
| `e7cd2df08e` | #22489 | 2026-10-22 | controls | `port` (done) | controls2 | Keyboard works inside the date, time and calendar picker popups. |
| `d8f2fc7b9f` | #22554 | 2026-10-22 | text | `port` (done) | text | Bounded lookahead (4096) on the enumerator and on the break table of a run, so a long unbroken run does not rescan the rest of the text per line. |
| `e4452b5bdb` | #22429 | 2026-10-22 | input | `port` (done) | input | Double tap needs the same button for both clicks. |
| `bd835340fb` | #21771 | 2026-10-22 | themes | `deferred` |  | Fluent 2 theme (16,105 lines). Owner decision: no new theme crate. Fixes that apply to the existing Fluent or WinUI themes are ported; the rest gets a recorded class. |
| `136efefc7d` | #22378 | 2026-10-22 | automation | `port` (done) | automation | ContentControl name no longer falls back to a type name. |
| `7fda9c36a5` | #22562 | 2026-10-23 | automation | `port` | win32 | UI Automation Invoke returns immediately. |
| `74b4e97962` | #22400 | 2026-10-23 | samples | `deferred` |  | Catalog ColorPicker page redesign. Rerun `scripts/sync-control-catalog.sh` after the ColorPicker theme work. |
| `6a25cf4d9b` | #22399 | 2026-10-23 | controls | `port` (done) | controls2 | A delayed submenu action no longer closes a reopened menu. |
| `c1e4fab24c` | #22577 | 2026-10-23 | collections | `not-applicable` |  | `FerroList::contains` and `index_of` take `&T`; an invalid item cannot be passed. |
| `1def5f851b` | #22426 | 2026-10-25 | controls | `port` (done) | controls2 | PipsPager sizing and scroll reset. |
| `b88f813a95` | #22595 | 2026-10-25 | controls | `port` (done) | controls | VirtualizingStackPanel: condition that was always true. |
| `13c99cfe42` | #22594 | 2026-10-25 | platform-linux | `already` |  | `DBusTextInputMethodBase::dispose` takes the name first and logs a failed disconnect. |
| `b7a70cd06b` | #22591 | 2026-10-25 | themes | `port` | themes | Fluent DropDownButton padding. Its Fluent 2 half follows commit bd835340fb. |
| `584ae6f3a7` | #22596 | 2026-10-25 | platform | `port` (done) | controls2 | `FallbackStorageProvider` under concurrent calls. |
| `6b6c573a03` | #22579 | 2026-10-25 | controls | `port` (done) | controls2 | ColorPicker fixes. |
| `99086ac183` | #22356 | 2026-10-26 | controls | `port` (done) | controls2 | AutoCompleteBox keeps focus when an item is picked. |
| `fb506057ea` | #22608 | 2026-10-26 | automation | `port` (done) | automation | Automation peers use `set_current_value`, so bindings survive. |
| `e61863dff7` | #22612 | 2026-10-27 | samples | `port` | samples | Catalog date picker sample validation. |
| `0a728569fc` | #22619 | 2026-10-27 | tests | `not-applicable` |  | Dispatcher leak in the upstream xUnit fixtures. |
| `c67550d9ec` | #22604 | 2026-10-27 | base | `already` |  | Rust `str` comparisons are ordinal (`iteration_count.rs`, `classes.rs`). |
| `6b41bcb0be` | #22593 | 2026-10-27 | platform-win32 | `already` |  | `query_active_display_paths` in `interop/unmanaged_methods.rs` passes a null topology. |
| `db0a9a9cdb` | #22115 | 2026-10-27 | platform-win32 | `deferred` |  | `UseCompositionSwapchain` (769 lines). Needs the Windows VM and a frame-pacing measurement. |
| `75ea7e186c` | #22549 | 2026-10-27 | build | `not-applicable` |  | Roslyn analyzer. Rust has no unchecked cast to warn about. |
| `951ee3f067` | #22609 | 2026-10-27 | controls | `port` (done) | controls | Calendar waits for a whole wheel notch. |
| `626c94d77c` | #22545 | 2026-10-27 | text | `already` |  | `bidi_algorithm.rs` already resolves W2 in one forward pass. |
| `f3eca207d0` | #22307 | 2026-10-27 | samples | `deferred` |  | Catalog theme switching. Depends on the Fluent 2 decision. |
| `b6fa4b12ea` | #22621 | 2026-10-27 | controls | `port` (done) | controls2 | DrawerPage and NavigationPage template parts relaxed to Decorator. |
| `7e8b76db3c` | #22379 | 2026-10-27 | platform-x11 | `port` | x11 | AT-SPI windows added on the UI thread. |
| `a9d935b5c7` | #22423 | 2026-10-27 | controls | `port` (done) | controls | `ComboBox.WrapSelection` with nothing to select. |
| `ca9149f2fa` | #22625 | 2026-10-28 | platform-linux | `port` (done) | linux | IBus expects evdev keycodes (X11 keycode minus 8). |
| `da1dafcdfb` | #22629 | 2026-10-28 | tests | `not-applicable` |  | Removal of the upstream Appium suite. |
| `0e0dad08ca` | #22631 | 2026-10-28 | themes | `already` |  | `SystemAccentColors::try_get_resource` matches typed resource keys; no culture comparison. |
| `b0ad044a65` | #22626 | 2026-10-28 | text | `not-applicable` |  | Pooled-buffer disposal. `ShapedBuffer` halves are `Rc` handles released by ownership. |
| `5d6b56563b` | #22630 | 2026-10-28 | themes | `deferred` |  | Fluent 2 resource providers; follows the Fluent 2 decision. |
| `5865a87387` | #22632 | 2026-10-28 | build | `not-applicable` |  | Upstream test projects removed. |
| `1efe0dfabb` | #22633 | 2026-10-28 | build | `not-applicable` |  | Upstream Xcode target. FerroUI sets its own minimum in `src/FerroUI.Native/build.rs`; raising it is an owner decision. |
| `725b5975e4` | #22611 | 2026-10-28 | themes | `deferred` |  | Fluent 2 brushes; follows the Fluent 2 decision. |
| `d1cc67f3ae` | #22634 | 2026-10-28 | build | `not-applicable` |  | Upstream solution filter. |
| `1c82a11d87` | #22636 | 2026-10-28 | themes | `deferred` |  | Fluent 2 resources; follows the Fluent 2 decision. |
| `c85e221c60` | #22565 | 2026-10-28 | automation | `port` (done) | automation | TreeViewItem expand and collapse provider. |
| `23dce09edc` | #22569 | 2026-10-29 | themes | `deferred` |  | Fluent 2 ColorPicker themes; follows the Fluent 2 decision. |
| `c9c7b90053` | #22641 | 2026-10-29 | platform-linux | `port` (done) | linux | AT-SPI: popups report the window role. |
| `5aa47e4f7f` | #22642 | 2026-10-29 | platform-linux | `port` (done) | linux | AT-SPI: active state only for the active window. |
| `1968b08e18` | #22578 | 2026-10-29 | themes | `port` | themes | ColorPicker fixes backported to the Fluent and Simple themes. |
| `719787dc69` | #22637 | 2026-10-29 | themes | `deferred` |  | Theme resources merged into one dictionary. Changes lookup order and generated theme code; needs the theme size and start-up check first. |
| `55129e0659` | #22229 | 2026-10-30 | platform-win32 | `port` | win32 | Move and resize drags work with touch. |
| `2a38dc63eb` | #22624 | 2026-10-30 | controls | `port` (done) | controls2 | Popup with pointer placement resizes with its content. |
| `eae84c1403` | #22179 | 2026-10-30 | text | `deferred` |  | Variable fonts: fvar and avar axis API. `FontVariationSettings` and backend variation exist; needs a design on top of them and PR 1252. |
| `d2ee79eb46` | #22180 | 2026-10-30 | text | `deferred` |  | Variable fonts: `GlyphTypeface` variation clones. |
| `64b21ba1cc` | #22181 | 2026-10-30 | text | `deferred` |  | Variable fonts: gvar outlines for managed glyph geometry. Skia and Vello already apply variations when drawing. |
| `9ce8ffda39` | #22182 | 2026-10-30 | text | `deferred` |  | Variable fonts: HVAR advances. |
| `ee4f35bfdd` | #22183 | 2026-10-30 | text | `deferred` |  | Variable fonts: MVAR metrics. |
| `408f2494f0` | #22184 | 2026-10-30 | text | `deferred` |  | Variable fonts: VVAR advances. |
| `dfee7f224b` | #22185 | 2026-10-30 | text | `deferred` |  | Variable fonts: `FontVariations` property, markup and animation. |
| `37279ab420` | #22650 | 2026-10-30 | themes | `deferred` |  | Fluent 2 embedded root; follows the Fluent 2 decision. |
| `432480ae82` | #22574 | 2026-10-30 | controls | `port` (done) | controls2 | `TabStripItem.Icon` and `IconTemplate`; icon presenter in the Fluent tab templates (two more elements per tab: start-up check owed). |
| `5b36bbe0ad` | #22683 | 2026-11-03 | platform-ios | `port` | ios | Accessibility traversal skips interop peers. |
| `99b0962977` | #22686 | 2026-11-03 | rendering | `already` |  | `immutable_bitmap.rs` returns an error from `Codec::from_data` on null. |
| `0a53cc4730` | #22687 | 2026-11-03 | platform-linux | `port` (done) | linux | AT-SPI: X11 window id attribute. |
| `c3b6f5cf61` | #22710 | 2026-11-05 | markup | `port` | markup | XamlX update: inline generics in `x:Type` and `x:Static`; compare with the FerroUI compiler. |
| `c9188ebe5f` | #22724 | 2026-11-05 | markup | `not-applicable` |  | IL emitter overload choice on Mono. |
| `330e602179` | #22652 | 2026-11-05 | input | `port` (done) | input | Alt with another modifier does not open the menu. |
| `60cb9d62b2` | #22731 | 2026-11-06 | build | `not-applicable` |  | Upstream package feeds. |
| `f5af0ebf98` | #22707 | 2026-11-06 | controls | `port` (done) | controls2 | TableViewColumn minimum and maximum widths. |
| `c4d24e9dab` | #22709 | 2026-11-06 | automation | `port` | automation | TableView row automation name from visible cells. |
| `727fea6125` | #22741 | 2026-11-07 | platform-browser | `already` |  | `webapp/modules/ferroui/dom.ts` already returns left, top, right, bottom. |
| `a81ceeaf3c` | #22656 | 2026-11-09 | platform-browser | `port` | browser | Clipboard in non-secure contexts. |
| `48132c0500` | #22716 | 2026-11-09 | controls | `port` (done) | controls | `IsActive` set before `Activated` is raised. |
| `69be03931f` | #22736 | 2026-11-09 | platform | `port` (done) | controls | Documentation of glob patterns. |
| `c9b5ae7b21` | #22746 | 2026-11-09 | controls | `already` |  | `Span::on_property_changed` compares the property, so `on_inlines_changed` runs. |
| `ae5ae6124e` | #22757 | 2026-11-09 | rendering | `port` | rendering | Native memory and resource checks (bitmap copies and others); each checked against the Rust code. |
| `dc061bc3ef` | #22698 | 2026-11-09 | rendering | `already` |  | `sk_round_rect_cache.rs` passes the radii without a heap array. |
| `1b53d56525` | #22688 | 2026-11-09 | platform-android | `port` | android | Storage cursors closed. |
| `5bbaa8e331` | #22743 | 2026-11-09 | rendering | `already` |  | `LockGuard` in `vulkan_khr_surface_render_target.rs` releases the lock on every exit. |
| `9186fcf61f` | #22756 | 2026-11-09 | themes | `deferred` |  | Fluent 2 calendar; follows the Fluent 2 decision. |
| `66f1f2a9dd` | #22763 | 2026-11-09 | rendering | `port` | rendering | `ImmutableRadialGradientBrush` copied `radius_x` into `radius_y`. |
| `c93b0d6c52` | #22764 | 2026-11-09 | rendering | `port` | rendering | Visual cache layer: the vertical scale was the horizontal one. |
| `45ddd45380` | #22659 | 2026-11-09 | rendering | `not-applicable` |  | Finalizer race of the .NET Skia binding. Rust ownership keeps the stream alive. |
| `b6a5fbc191` | #22740 | 2026-11-09 | rendering | `not-applicable` |  | `GC.KeepAlive`. Rust borrows keep the bitmap alive. |
| `b490bc6733` | #22748 | 2026-11-10 | platform-x11 | `port` (done) | x11 | Two-finger scrolling with the synaptics driver. |
| `2061fb5090` | #22760 | 2026-11-10 | text | `port` (done) | text | Wrap opportunity between shaped runs, decided from a window of both runs. The per-run break tables stay, so multi-run lines do not fall back to the enumerator. |
| `bdeeeef0a7` | #22693 | 2026-11-10 | platform-ios | `port` | ios | Embedded native controls in accessibility. |
| `80c4f31848` | #22462 | 2026-11-10 | platform-x11 | `port` (done) | x11 | ConfigureNotify older than our last resize ignored. |
| `0dc28102b4` | #22566 | 2026-11-10 | platform-linux | `port` | linux | DRM: no block on the page flip just queued. |
| `8dae868308` | #22628 | 2026-11-10 | input | `deferred` |  | Key text, consumed modifiers and native key data (1,258 lines on every platform). Public event API; needs each platform checked. |
| `5ef345810e` | #22782 | 2026-11-12 | platform-win32 | `port` | win32 | Virtual key mapping behind the keyboard layout tests. |
| `3ecf2a3fd8` | #22777 | 2026-11-12 | build | `not-applicable` |  | Empty upstream CI commit. |
| `b04cec5ef5` | #22801 | 2026-11-12 | build | `not-applicable` |  | .NET SDK update. |
| `dc2253658c` | #22674 | 2026-11-12 | markup | `port` | markup | Compact IL names do not apply; the enum whitespace fix in its XamlX update does. |
| `27b1179321` | #22558 | 2026-11-12 | platform-linux | `port` (done) | linux | StatusNotifierItem tooltip text. |
| `22e9912b75` | #22725 | 2026-11-12 | markup | `not-applicable` |  | Exact signature lookups in the IL compiler. |
| `050d3fb941` | #22798 | 2026-11-12 | rendering | `port` | rendering | Skia: one layer less for each opacity mask. Performance gain. |
| `ee79f65f26` | #22685 | 2026-11-12 | platform-android | `port` (done) | android | Input type no longer forced to a password variation. |
| `84f262c218` | #22676 | 2026-11-12 | markup | `not-applicable` |  | Function-pointer factories for IL-compiled property info. FerroUI generates Rust. |
| `24b6c7b410` | #22610 | 2026-11-12 | platform-macos | `port` | macos | `updateLayer` flushes the render pass synchronously. |
| `070c3be990` | #22501 | 2026-11-12 | rendering | `deferred` |  | Opt-in EGL partial presentation (1,390 lines). Needs GPU validation on Linux and Android. |
| `ca61365d75` | #22538 | 2026-11-12 | rendering | `port` | rendering | Animated value written even when an expression has read it. |
| `61c3aaa876` | #22810 | 2026-11-13 | rendering | `port` | rendering | Remove `CompositorDrawingContextProxy`: on main only its own tests construct it. |
| `6350ac83e6` | #22774 | 2026-11-13 | text | `port` (done) | text | `TextLineImpl::collapse` gives the collapsed line its own references and keeps the source line valid after a split; the layout releases the replaced line. |
| `a4f832f5d6` | #22795 | 2026-11-13 | input | `port` (done) | input | Tab navigation skips disabled elements. |
| `e73a976afd` | #22796 | 2026-11-13 | platform-macos | `already` |  | `trayicon.mm` already tests the text for null. |
| `60b1719596` | #22797 | 2026-11-13 | controls | `port` (done) | controls | No empty `SelectionChanged` during initialization. |
| `53e461e07a` | #22466 | 2026-11-13 | controls | `port` (done) | controls2 | Tab strip stays reachable with Tab. |
| `ab36f90feb` | #22701 | 2026-11-13 | controls | `port` (done) | controls | Grid: debug assertion allows a rounding residual. |
| `5ea0586b32` | #22699 | 2026-11-13 | themes | `port` | themes | Fluent ButtonSpinner outer corner. |
| `a05d9d0046` | #22812 | 2026-11-13 | themes | `port` | themes | Simple ComboBox and Slider show validation errors. |
| `d0a3041701` | #22475 | 2026-11-13 | controls | `port` (done) | controls | RelativePanel chain with infinite available size. |
| `ad90a07d03` | #22671 | 2026-11-13 | automation | `port` | win32 | `LabeledBy` in Windows UI Automation. |
| `10a2fb0ac5` | #22682 | 2026-11-13 | platform-win32 | `port` | win32 | Clipboard read retried while another process holds it. |
| `e08cd25212` | #21826 | 2026-11-13 | rendering | `deferred` |  | Backdrop effects (1,786 lines). Needs a GPU path in Skia and in Vello; must reuse the WinUI composition backdrop code. |
| `80659c4c3e` | #22773 | 2026-11-13 | text | `deferred` |  | Platform-simulated typefaces. Touches font matching beside the open web fonts stack; follows that stack. |
| `02d7bc35e3` | #22662 | 2026-11-13 | text | `port` (done) | text | Runs of different scripts shaped in separate buffers. |
| `9d6cb1ea62` | #22015 | 2026-11-13 | rendering | `port` | rendering | ContentPresenter and acrylic clip to rounded bounds. |
| `d379b92b87` | #22694 | 2026-11-13 | platform-macos | `port` (done) | macos | `automation.mm`: tab button subrole. |
| `500e78afef` | #22679 | 2026-11-13 | data | `port` (done) | data | TwoWay binding does not write a just-published value back. |
| `8035f14fd3` | #22690 | 2026-11-13 | platform-macos | `port` (done) | macos | `FrnWindow.mm`: only the view tracking area resets the cursor. |
| `4ae41027c0` | #22820 | 2026-11-16 | platform-win32 | `port` (done) | win32 | `dcomp.idl`: `BOOL`, not `bool`, for `CreateTargetForHwnd`. |
| `7011f50c27` | #22305 | 2026-11-16 | controls | `port` (done) | controls2 | `AutoCompleteBox.UseFloatingPlaceholder`. |
| `9f6a652216` | #22846 | 2026-11-16 | platform-browser | `already` |  | `webapp/modules/ferroui/timer.ts` already passes the timestamp. |
| `16d8678a44` | #22835 | 2026-11-16 | rendering | `port` | rendering | Skia: multiply opacity applied once below an opacity mask. |
| `ab578c3ea0` | #22793 | 2026-11-17 | platform | `port` (done) | controls2 | Assembly resolved by exact name. |
| `75497e3634` | #22857 | 2026-11-17 | themes | `port` | themes | Simple theme: missing resources. |
| `a49cbefe8b` | #22864 | 2026-11-17 | automation | `port` (done) | automation | Peer parent kept after a move. |
| `281e59e138` | #22866 | 2026-11-17 | build | `not-applicable` |  | Upstream contribution bot. |
| `fda56217a5` | #22872 | 2026-11-18 | build | `not-applicable` |  | Upstream workflows. |
| `251fda6155` |  | 2026-11-18 | build | `not-applicable` |  | Upstream contribution bot. |
| `a096f9bbde` | #22862 | 2026-11-18 | platform-android | `port` | android | Accessibility node position. |
| `92dc2b7207` | #22849 | 2026-11-18 | platform-android | `port` | android | Accessibility focus changes reported. |
| `7007223002` | #22871 | 2026-11-18 | controls | `port` (done) | controls | FlexPanel relative basis is a fraction. Parsed percentages were divided by 100 twice; code passing `50` must pass `0.5`. |
| `192e7a0ba5` | #22880 | 2026-11-18 | platform-macos | `port` (done) | macos | Leaf nodes are not reported as expandable. |
| `2d34086103` | #22883 | 2026-11-19 | build | `not-applicable` |  | Upstream contribution guide. |
| `ea6f0daf9b` | #22888 | 2026-11-19 | platform-android | `port` | android | Start-up does not wait for the Choreographer thread. |
| `5032926e38` | #22865 | 2026-11-19 | build | `not-applicable` |  | Upstream readme. |
| `3f984fa0d9` | #22870 | 2026-11-19 | text | `port` (done) | text | `ParagraphIndent` and first-line `Indent` honoured by the formatter, beside the paragraph styles of `TextLayout`. |
| `debc815ba9` | #22914 | 2026-11-21 | build | `not-applicable` |  | Upstream build services package. |
| `605e161019` | #22738 | 2026-11-23 | platform-android | `port` | android | Render call synchronized with surface create and destroy. |
| `51dae7305e` | #22681 | 2026-11-23 | platform-android | `port` | android | TalkBack roles. |
| `5d9e6d1b0e` |  | 2026-11-23 | build | `not-applicable` |  | Upstream version number. |
<!-- /generated:inventory -->
