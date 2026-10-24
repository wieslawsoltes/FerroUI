# Completion and insight controls

This report tracks the `CodeCompletion` domain from AvaloniaEdit revision
`7277e36366d754219d7ede16b6613786b32caf4c`. The derived Rust sources retain the
copyright and MIT license in `src/FerroUI.Edit/NOTICE.md`. The implementation
uses FerroUI's class model, native properties, content presenters, list
virtualization, popup positioning, routed input, and notification subscriptions.

## Source map

Paths in the first column are relative to upstream `src/AvaloniaEdit/CodeCompletion/`.
Rust paths are relative to `src/FerroUI.Edit/` unless noted.

| Upstream source | Rust source and public contract | Adaptation |
| --- | --- | --- |
| `CompletionWindowBase.cs` | `completion_window_base.rs`: `CompletionWindowBase`, `CompletionWindowBaseImpl` | Shared popup lifecycle, tracked UTF-16 range, editor input forwarding, focus/scroll/document dismissal, attached editor ownership. Rust extension hooks use native virtual dispatch. |
| `CompletionWindow.cs` | `completion_window.rs`: `CompletionWindow` | Existing `show_for`, `active_for`, `selected_item`, `request_insertion`, and `refresh` APIs remain. `show_with_data` adds rich/custom items. |
| `CompletionList.cs` | `completion_list.rs`: `CompletionList`, `CompletionInsertionRequestedEventArgs` | Native content control composed around `CompletionListBox`; observable `FerroList<CompletionDataRef>` data; configurable keyboard and pointer acceptance; native empty-content template. Core ranking remains in `FerroUI.Edit.Core/completion/completion_list.rs`. |
| `CompletionListBox.cs` | `completion_list_box.rs`: `CompletionListBox` | Uses the native `ListBox` theme, item containers, scroll view and virtualization; selection clamping and visible-page helpers. |
| `CompletionAcceptAction.cs` | `completion_list.rs`: `CompletionAcceptAction` | `PointerPressed`, `PointerReleased`, `DoubleTapped`; the default is pointer press. |
| `CompletionTipContentControl.cs` | `completion_tip_content_control.rs`: `CompletionTipContentControl` | Native content control accepts strings, controls, or template-bound data, with wrapping and bounded width. |
| `ICompletionData.cs` | `completion_data.rs`: `ICompletionData`, `CompletionData`, `CompletionDataRef` | Image/content/description/priority and custom completion callback. The UI-independent core `ICompletionData` remains available in the core namespace. |
| `IOverloadProvider.cs` | `overload_provider.rs`: `IOverloadProvider`, `OverloadProviderRef`, `OverloadProvider`, `OverloadItem` | Uses FerroUI `INotifyPropertyChanged`; convenience provider clamps selection and notifies collection/content changes. |
| `InsightWindow.cs` | `insight_window.rs`: `InsightWindow` | Native content, `CloseAutomatically`, editor range/focus lifecycle and theme subscription. |
| `OverloadInsightWindow.cs` | `overload_insight_window.rs`: `OverloadInsightWindow` | Observable provider, Up/Down navigation while editor retains focus, provider detachment on close. |
| `OverloadViewer.cs` | `overload_viewer.rs`: `OverloadViewer`; `collapse_if_single_overload_converter.rs`: `CollapseIfSingleOverloadConverter` | Rich header/body, observable selection text, navigation buttons, zero/one-count behavior, explicit one-way converter error. |
| `PopupWithCustomPosition.cs` | `popup_with_custom_position.rs`: `PopupWithCustomPosition` | A native popup exposing a point offset through the framework's horizontal/vertical offsets. |

The upstream completion/insight XAML styles are adapted as native control
composition and existing FerroUI list/content templates. Their literal XAML is
not copied. All public native classes have runtime type and compiled-XAML Rust
path registration. `CompletionListBox` explicitly inherits the `ListBox` style
key so native themes and virtualization continue to apply.

## Ownership and event ordering

`CompletionWindowBase::attach_to_editor` validates the supplied UTF-16 segment
before changing ownership. The editor's `ActiveWindows` attached property
retains active windows; each window retains only a weak editor reference. A
new window closes the previous instance of the same concrete type. Insight and
completion windows may coexist, and the most recently attached window receives
editor input first. After it closes, the previous window resumes handling input.

Document-changing notifications map both segment boundaries through each edit.
The start normally stays before an insertion at its boundary; the end follows
insertions. `ExpectInsertionBeforeStart` changes start affinity for the next
insertion exactly at that start, then resets. Deleting immediately before the
start dismisses the window. A document replacement always dismisses it.

The popup uses `TextArea::rectangle_for_offset` at its tracked start. This is
viewport geometry and does not move the caret. Scrolling the anchor out of view
or into a collapsed fold dismisses the popup. Unarranged editor tests can attach
a session before a viewport exists. Screen constraints remain the responsibility
of the native popup positioner. Parent window deactivation/closure and document
replacement close the session; focus checks are posted through FerroUI's
dispatcher so a focus transfer into the popup can finish before evaluation.

Input is forwarded with fresh routed-event arguments. Cloning native event
arguments would share routing state and change the editor's source while it is
still dispatching. Escape closes assistance, completion keys navigate/accept,
and ordinary keys continue to the editor. The completion window also forwards
text input and wheel input to its native content/scroll view. Native list pointer
handlers accept only real item containers, and release acceptance also checks
that the release lies within the item.

All editor, document, scroll, parent-window, theme and provider subscriptions
have explicit disposers. Closing detaches editor subscriptions immediately,
even when an application retains the closed popup object. Overload provider
subscriptions are removed on provider replacement, popup close and viewer drop.
The observable completion collection is a shared native handle; edits through
that handle refresh the model while preserving a selected item's identity when
it remains visible.

## Rich completion and insertion

Existing code may continue using plain core items:

```rust,ignore
let window = CompletionWindow::show_for(
    &editor, start, end,
    vec![ferroui_edit::core::completion::CompletionItem::new("ReadLine")],
)?;
```

For rich content, supply `Rc<dyn ferroui_edit::ICompletionData>` through
`show_with_data`. `Content` and `Description` accept `BoxedValue`, preserving
native controls or data templates. The item template also presents an `IImage`
when supplied. `CompletionData` is a convenience implementation, and custom
types may implement the trait directly. A `CompletionAction` receives the editor,
the tracked `SimpleSegment`, and the originating key or pointer event.

Before calling a completion action, the window preflights its replacement using
`EditorSession::validate_document_edits`. This preserves editor read-only and
protected-range behavior. A rejected operation does not call custom code.
After successful preflight, the popup closes and disposes its input handlers
**before** invoking custom code, allowing that code to start snippets or other
assistance without the old window removing their handlers. Default insertion
uses `apply_document_edits` and moves the caret in UTF-16 units; it participates
in the document's normal undo history. Custom actions that write additional
ranges should submit those edits through the editor session too.

`selected_completion_data()` preserves the exact selected object. The older
`selected_item()` returns a plain metadata snapshot; arbitrary rich descriptions
remain available on the rich object rather than being coerced to a string.

## Differences and remaining validation boundaries

* Completion descriptions are presented in the same native popup below the
  list, rather than in the upstream secondary popup beside the selected row.
  Rich content is preserved; exact upstream tooltip geometry is not claimed.
* The native content composition exposes actual FerroUI template/content APIs;
  it does not reproduce upstream template-part names or promise binary/source
  compatibility with the upstream XAML style files.
* Overload indices use `usize` and empty providers expose index zero with count
  zero. No negative-index sentinel is needed; the viewer handles zero count
  without indexing or division by zero.
* `CollapseIfSingleOverloadConverter::convert_back` returns a descriptive
  `BindingError`, replacing the upstream throwing one-way conversion contract.
* Programmatic control/provider construction is implemented. Arbitrary custom
  completion actions and provider implementations are Rust contracts, not
  serializable XAML objects.
* Ranking remains the core port's documented Unicode/string algorithm. These
  controls do not add a locale-sensitive .NET comparison layer.
* Native focused tests cover the contracts listed below. Browser and OS-specific
  popup appearance, IME interaction, and every upstream template permutation
  require separate platform integration coverage; a native unit-test pass does
  not establish their equivalence.

## Verification

The completion test cases cover original insertion/undo/document-replacement
behavior, immediate caret dismissal, read-only preflight before custom actions,
rich content identity, original event delivery, closing before custom action
dispatch, stacked-popup input ownership, unchanged-prefix navigation, observable
collection mutation/disposal, configurable accept keys, empty-list selection,
runtime XAML construction, overload navigation/provider replacement/disposal,
and insight segment/focus/manual-lifetime behavior. The existing snippet Tab
priority regression remains part of the full UI suite.

The integrated native gate completed successfully on 2026-10-22:

| Package | Result |
| --- | --- |
| `ferroui-edit-core` | 257 passed, 0 failed |
| `ferroui-edit` | 61 passed, 0 failed; includes 14 new completion/insight tests and all 47 existing editor tests |
| `editor-demo` | 2 passed, 0 failed |

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 \
cargo test --offline --no-default-features \
  -p ferroui-edit-core -p ferroui-edit -p editor-demo --lib
```

The gate uses the repository's installed dependency cache and existing native
feature union. `--no-default-features` intentionally avoids compiling the demo's
default desktop rendering backend as part of this library-test run. This is a
native behavior and XAML metadata check, not a browser rendering claim.
