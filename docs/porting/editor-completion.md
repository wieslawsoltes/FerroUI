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
Closing an older popup also detaches later layers above it.

Document-changing notifications map both segment boundaries through each edit.
The start normally stays before an insertion at its boundary; the end follows
insertions. `ExpectInsertionBeforeStart` changes start affinity for the next
insertion exactly at that start, then resets. Deleting immediately before the
start dismisses the window. A document replacement always dismisses it.

The popup projects its tracked start into the text area's viewport. Opening at
the caret keeps its soft-wrap affinity and virtual cells. Later caret movement
leaves the opening anchor in place. The anchor follows edits to the tracked
segment and uses virtual cells while its offset is at a logical line end.
Scrolling the anchor out of view or into a collapsed fold dismisses the popup.
Unarranged editor tests can attach a session before a viewport exists. Screen
constraints remain the responsibility
of the native popup positioner. Parent window deactivation/closure and document
replacement close the session; focus checks are posted through FerroUI's
dispatcher so a focus transfer into the popup can finish before evaluation.
By default, completion and insight leave the editor and surrounding controls
available to pointer input. A click within a matching completion prefix can
move the caret and keep the list open. Scrolling keeps the list open until
its anchor leaves the viewport. A click on a focusable control outside the
popup reaches that control and then dismisses assistance on focus loss.
`CloseAutomatically` controls the caret and focus policies independently of
FerroUI's `IsLightDismissEnabled` popup property.

Input is forwarded with fresh routed-event arguments. Cloning native event
arguments would share routing state and change the editor's source while it is
still dispatching. Popup preview handlers are temporary layers in the text
area's input stack. Detaching a layer closes its popup, including when an
application replaces the primary input handler. Escape closes assistance, completion keys navigate/accept,
and ordinary keys continue to the editor. The completion window also forwards
text input and wheel input to its native content/scroll view. Native list pointer
handlers accept only real item containers, and release acceptance also checks
that the release lies within the item.

`TextEditor.CompletionAcceptAction` sets the gesture for new completion windows.
It is a styled property available in Rust and XAML. The popup captures the value
when it attaches to the editor. The list can override it for that window. The
Windows regression sends press and release through the native popup's realized
row. It verifies that insertion waits for release and one Undo restores the
document.

All editor, document, scroll, parent-window, theme and provider subscriptions
have explicit disposers. Closing detaches editor subscriptions immediately,
even when an application retains the closed popup object. Overload provider
subscriptions are removed on provider replacement, popup close and viewer drop.
The observable completion collection is a shared native handle; edits through
that handle refresh the model while preserving a selected item's identity when
it remains visible.

## Filtering and selection

The list keeps matching rows in provider order and ranks them to choose the
selection. A full match ignoring case ranks above a case-sensitive prefix.
Short camel-case matches rank above substring matches. Longer camel-case
matches rank below substrings. An equally good selected row stays selected.
Otherwise the highest priority wins, with ties resolved in provider order.

Camel-case matching uses the first character and subsequent uppercase letters
in the Basic Multilingual Plane as initials. The pinned editor's results
confirm that a supplementary letter, Roman numeral or circled capital inside
an identifier does not consume an initial. Full, prefix and substring matches
continue to accept supplementary characters. Eight of the reference queries
are reproduced by the core regression, including case-insensitive prefixes
and an emoji at the start of an identifier.

The native list takes its locale from the editor thread and uses the
formatting culture's registered comparison rules for full, prefix and
substring matches. A repeated query is evaluated again when the locale
or its comparison rules change. Standalone completion and native controls
without a provider use canonical normalization and ICU4X simple case folding,
including Turkic folds for Turkish and Azerbaijani. Comparison keys skip the
zero-weight scalars in ICU's root collation. Displayed and inserted text retain
their original characters, and verbatim full matches still rank first.
[ICU4X case folding](https://docs.rs/icu_casemap/latest/icu_casemap/struct.CaseMapperBorrowed.html#method.simple_fold)

The portable matcher agreed with all 80 pinned list states across English,
Turkish, Azerbaijani and Greek. Five additional queries verified original
suggestion identity, a combining mark separated from its starter, emoji with
and without a joiner, and the canonical decomposition of a musical note.
The Windows reference comparison agreed on 959 of the 962 singleton zero
weights across seven locales. U+10EFB, U+113E1 and U+113E2 have zero weights in
the pinned Unicode 17 table and remain significant to that Windows runtime.
[ICU 78.1 root collation table](https://github.com/unicode-org/icu/blob/release-78.1/icu4c/source/data/unidata/FractionalUCA.txt)

Portable prefix queries must include adjacent secondary-only accents. A
zero-weight scalar permits a cut before such an accent. Substrings start and end
at the original grapheme boundaries, with individual CR and LF search positions
preserved. Comparison keys keep original grapheme boundaries after zero-weight
characters are removed. [ICU string
search](https://unicode-org.github.io/icu/userguide/collation/string-search.html)

Verify the generated table with the pinned `FractionalUCA.txt` input:

```sh
python3 scripts/edit-port/generate_collation_ignorables.py FractionalUCA.txt --check
```

With filtering disabled, all entries stay visible in provider order. An
unmatched query clears the selection. Down selects the first row, and accepting
while the selection is empty closes the popup without editing the document or
creating an undo entry. [Pinned completion
list](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/CodeCompletion/CompletionList.cs)

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

Set `CompletionList.ItemTemplate` in runtime or compiled XAML to customize the
rows. In a Rust-created popup, use `window.completion_list().set_item_template`.
The template receives the original item's `CompletionDataRef`, with readable
`Text`, `Content`, `Description`, `Image` and `Priority` properties. Set the
template to null or `None` to restore the built-in image and content row.

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

The selected item's description opens in a secondary native popup beside the
list. It aligns with the realized row and updates after layout or scrolling.
While a selected row is awaiting realization, the description uses the top or
bottom edge of the list according to its index. The native positioner can flip
the popup across the list when the preferred side lacks space. Descriptions
keep their native content and theme, share the editor window's overlay, and
leave keyboard and pointer input with the underlying controls. An item without
a description hides the secondary popup. Closing completion also closes its
description and releases the presented content before invoking custom insertion.
This follows the pinned [completion window](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/CodeCompletion/CompletionWindow.cs).

While the input method displays preedit, routed keys leave completion and
insight selection alone. Tab and the arrow keys also leave editor focus in
place. The text area receives the IME commit directly and refreshes the
suggestions from the committed document. The native input connection test
exercises these transitions and verifies separate undo steps for the commit
and subsequent completion acceptance.

## Differences and remaining validation boundaries

* The native content composition exposes actual FerroUI template/content APIs;
  it does not reproduce upstream template-part names or promise binary/source
  compatibility with the upstream XAML style files.
* Overload indices use `usize` and empty providers expose index zero with count
  zero. No negative-index sentinel is needed; the viewer handles zero count
  without indexing or division by zero.
* `CollapseIfSingleOverloadConverter::convert_back` returns a descriptive
  `BindingError`, replacing the upstream throwing one-way conversion contract.
* Applications implement custom completion actions and overload providers in
  Rust. XAML can bind the viewer's nullable `Provider` to a host-supplied
  `OverloadProviderRef`. The handle exposes `SelectedIndex`, `Count`,
  `CurrentIndexText`, `CurrentHeader` and `CurrentContent` to bindings and
  forwards the provider's property change notifications.
* The portable matcher covers canonical equivalence, root collation ignorables
  and context-independent case folding. Locale-specific contractions and
  expansions depend on the registered comparison provider.
* Native focused tests cover the contracts listed below. Browser and OS-specific
  popup appearance, IME interaction, and every upstream template permutation
  require separate platform integration coverage; a native unit-test pass does
  not establish their equivalence.

## Tests

The completion tests cover insertion, undo and document replacement, caret
dismissal, read-only preflight before custom actions, rich content identity,
event delivery, stacked-popup input ownership, observable collection mutation
and disposal, configurable accept keys, runtime XAML construction, overload
navigation and insight lifetime. They are native behavior and XAML metadata
checks, not a browser rendering claim.

```sh
cargo test --no-default-features \
  -p ferroui-edit-core -p ferroui-edit -p editor-demo --lib
```
