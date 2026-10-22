# Link fields in the shared document

Link fields use the same `DocumentSnapshot`, live model, editor history and
semantic `Hyperlink` controls as the rest of rich text. The projected service
adds an explicit coordinate system for the native field instruction and result.
Existing visible document, selection, layout, clipboard-range and accessibility
offsets keep their established UTF-16 meaning.

The pinned public contracts are `ITextRange.Link` and `RichEditTextRange.Link` in
[Microsoft.UI.Text.xml](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/packaging/Intellisense/generated/Microsoft.UI.Text.xml).
Their engine implementation is external to that WinUI source snapshot. The
behavior implemented here is bounded by the immutable
[system-MSFTEDIT record](native-msftedit-links-gravity.json): 137 commands across
59 fixtures and 274 before/after snapshots. The measured DLL is
`6.2.26100.33438`; this is separate evidence from the packaged WinUI engine.

## One field record across paragraph-owned controls

[`DocumentLinkField`](../../../src/FerroUI.Controls/documents/document_link_field.rs)
stores a stable nonzero ID, visible UTF-16 result range, unquoted target, optional
TargetName and the observed sentinel bit. `DocumentSnapshot::link_fields` is the
authoritative table; `next_link_field_id` starts at 1 and exceeds all stored IDs.
The table is part of the ordinary rich snapshot, so snapshots, history, exact
clipboard content and supported codecs use the same metadata.

A field may cover text in several paragraphs, empty paragraphs, or only real
paragraph separators. Each nonempty rendered segment remains a paragraph-owned
`Hyperlink` tagged with the same field ID. Empty paragraphs need no invisible
Hyperlink or automation focus target. The validator checks unique IDs, the
allocation watermark, disjoint scalar-aligned ranges, target/projection agreement
and complete result coverage. Targets are nonempty and cannot contain NUL or an
embedded quotation mark. TargetName is separate optional metadata.

Copies and history have different identity requirements:

| Operation | Field identity behavior |
|---|---|
| Snapshot or undo/redo restore | Retain the image's field IDs and watermark. The live allocator also retains its monotonic allocation floor. |
| Slice/copy | Intersect each field with the requested visible range, rebase its endpoints and retain its ID in the standalone fragment. Separator-only intersections remain fields. |
| Insert copied rich content | Allocate one fresh destination ID for each fragment field, shared by every paragraph segment. |
| Inherited plain edits, ChangeCase and character replacement | Rebase the existing field spans; Unicode expansion uses an explicit old-to-new UTF-16 boundary map. |
| Unrelated rich insertion inside a field | Preserve the surviving destination pieces with checked identities and remap inserted fields. |
| Filter a field to empty | Remove its record and turn its tagged wrappers into ordinary spans while preserving children/formatting. |

Compatible target changes reuse live interactive nodes. `HyperlinkBehavior`
retains the shared navigation/click handler lists for undo reconstruction without
retaining the old parented content tree. Those runtime handler lists do not
participate in content equality. Low-level semantic edits reconcile a consistent
new target; incompatible or disconnected projections retire that canonical
identity while retaining the remaining ordinary semantic links. This is an
explicit model policy, not a native paired-range tracking claim.

## Typed projected ranges

[`RichEditTextDocument::field_story()`](../../../src/FerroUI.Controls/rich_text/rich_edit_text_field_story.rs)
returns `RichEditTextFieldStory`. Its `FieldStoryOffset` values include field
instructions and a read-only synthetic final paragraph mark; visible offsets
remain ordinary model offsets. That final mark is never installed in the live
document or accepted as a Link mutation target. Explicit conversion
chooses `FieldInstructionBoundary::Before` or `After` at a result boundary.
`get_range_from_visible` chooses `Before` for both ends. Call `project_visible`
with `After` when selecting only the friendly result at an instruction boundary.

```rust
// `document` is a RichEditTextDocument with visible text in 0..5.
let story = document.field_story();
let range = story.get_range_from_visible(0..5)?;
range.set_link(Some("\"https://example.test/\""))?;
let argument = range.get_link()?;
```

The setter accepts the measured quoted Link argument, including its supported
sentinel form; it returns whether the shared content changed. The getter returns
`Option<String>` so absence stays distinct from a returned string. A successful
nonempty getter expands only the acted projected range to the whole field.
Ordinary `Clone` shares that Rust range; `duplicate()` creates independently
movable endpoints. Neither a getter nor a duplicate moves the editor selection.

The projected text accounts for the measured instruction and result UTF-16.
All 274 native snapshots report lengths equal to the returned strings. The
implementation does not infer three hidden marker positions from RTF syntax.
Raw `utf16` inspection may return a surrogate half; string reads and mutations
still validate representability. An instruction-only range has no visible range
and is never substituted with the current caret.

Visible endpoints follow existing text pointers. Instruction-interior endpoints
refer to an authoritative field ID. A projection of a legacy semantic Hyperlink
has provisional IDs only for that immutable query; any model-version change
stales such an instruction anchor. Raw surrogate-interior anchors are also
snapshot-bound. This prevents a later allocation with the same numeric ID from
being mistaken for the original field.

## Transactions and callback ownership

SetLink checks the argument, supported operation and candidate snapshot before
cancelling an original composition. It then resolves against committed content
again and installs the shared snapshot in one existing editor transaction.
The independent directed selection and typing patch are retained. Acted range
endpoints publish before callbacks, so a later callback's endpoint request wins.

Formatting reads and equality can run application brush code. The shared model
therefore checks its intrinsic mutation version after the old-snapshot/equality
read, including a would-be no-op, and after content-pool/control-parent preflight.
No intentional structural mutation has occurred at those fences. A failed
restore that observes an external write marks the editor transaction as external;
rollback cannot overwrite the callback's accepted document or recreate stale
history. Selection Gravity uses the same ownership discipline for snapshot
restoration and explicit history-clear requests.

Public model writes during active installation remain guarded by the model's
existing `installing` state. Arbitrary direct mutation of child Run properties
or collections during a reused-object setter inside structural rebuilding is a
stronger, pre-existing boundary and is not claimed to be solved here.

## RTF, transfer and policy filtering

The [RTF decoder](../../../src/FerroUI.Controls/rich_text/rtf.rs) creates one
canonical record for each supported field. The
[field encoder](../../../src/FerroUI.Controls/rich_text/rtf_link_fields.rs) writes
one field/result group across all of its paragraphs and separators. It closes
leaf-format groups before field boundaries and restores the current paragraph's
defaults after a field, preserving suffix metrics and formatting. Adjacent fields
with equal targets remain distinct. External RTF assigns fresh local IDs on
import; it retains field grouping and metadata, not process-local numeric IDs.

The reader bounds input, nesting, tokens, decoded UTF-16 and cumulative expanded
target/TargetName bytes. The text limit also applies independently to font-name
and instruction destinations. A structured `LineBreak` emits `\line` and counts
its actual platform length (CRLF on Windows, LF elsewhere). Literal CR, LF and
U+2028 inside Run text or instruction tokens use signed RTF Unicode escapes,
preserving their scalars without turning them into structured breaks. Quoted UNC
paths and quoted strings equal to switch names remain literal token data.

[`RichTextTransfer`](../../../src/FerroUI.Controls/rich_text/rich_text_transfer.rs)
validates an advertised exact snapshot before considering external formats.
Malformed exact content fails instead of silently selecting RTF or plain text.
In-process content retains exact IDs, watermark, TargetName and sentinel state.
RTF import/export keeps supported cross-paragraph fields; sentinel export and
other unrepresentable fields return an explicit error. Plain text remains an
available external representation when an outgoing exact snapshot has no RTF
encoding.

The [control filter](../../../src/FerroUI.Controls/rich_edit_fragment_filter.rs)
uses the actual whole-run Unicode casing result, with scalar-safe UTF-16 boundary
maps for expansion, contextual lowercase, tabs, line separators, paragraph joins
and embedded content. It rebases fields and validates the transformed snapshot.
After callback-bearing filtering, an editor/model/selection ownership fence
rejects a stale paste and preserves newer callback content and typing state.

## Evidence, regressions and open contracts

The production core's standalone std-only gate passed 23 tests, including the
137 literal transitions. Framework integration now has 24 live consumer cases,
four model-install preflight cases, 13 transfer/filter cases and 15 RTF field
cases (five strengthened existing cases and ten new cases). The live replay
reseeds the observed before range for each command. It does not establish native
physical tracking of paired live ranges. The combined full Controls gate passed
all 5,251 tests, including these consumers and the additional character-history
ordering regression. Exact source/execution records and earlier failures are in
[TESTS.md](TESTS.md); native parity limits below remain unchanged.

The following behavior remains explicit unsupported or uncharacterized:

- Instruction-only/interior Link commands, a getter beginning in a plain prefix
  before a field, and surrogate-interior Link mutation.
- Sentinel-empty arguments, nested fields, synthetic terminal-paragraph-mark
  mutation, and same-target right-adjacent prepend or equal-target gap merging.
  The measured left-first append behavior is implemented.
- Raw projected instruction spelling for TargetName. Shared snapshots and RTF
  retain TargetName, but the projected Link service rejects those stories.
- Sentinel RTF encoding. Exact in-process transfer retains its metadata.
- A Link result that cuts through a structured LineBreak atom, including the
  middle of its CRLF representation on Windows.
- Native paired-range physical tracking, arbitrary independently formatted
  paragraph marks, and every native Link visual-formatting side effect.
- The external WinUI runtime and complete native TOM ABI/content equivalence.

These boundaries keep both Link API entries `partial` under RT-I34. They do not
change the overall full-port acceptance requirements.
