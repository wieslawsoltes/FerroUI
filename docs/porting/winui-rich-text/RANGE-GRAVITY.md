# Range and live selection formatting gravity

`RichEditTextRange` has a typed `RangeGravity` property for choosing character
formatting at a collapsed visible UTF-16 boundary. The implementation shares
FerroUI's document, effective format resolution, editor transactions and typing
patches. It does not create a second document or use native COM objects.

The public names and values come from the pinned
[Microsoft.UI.Text.xml](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/packaging/Intellisense/generated/Microsoft.UI.Text.xml).
Behavioral evidence is the immutable
[system-MSFTEDIT observation record](native-msftedit-links-gravity.json), with
engine, source and artifact provenance recorded in [TESTS.md](TESTS.md).
The separately measured [actual-selection record](native-msftedit-selection-gravity.json)
backs the live-selection consumer below. System MSFTEDIT observations are
separate from the external WinUI text engine.

## Public values and accepted modes

| Public value | Numeric value | Collapsed formatting source |
|---|---:|---|
| UIBehavior | 0 | The preceding run within a paragraph; the current paragraph at its start. |
| Backward | 1 | The preceding character, including the preceding paragraph at a paragraph boundary. |
| Forward | 2 | The following run. |
| Inward | 3 | The following run for a collapsed range. |
| Outward | 4 | The measured setter accepts this as UIBehavior; the getter therefore returns 0. |

All five constants retain their native values even though the setter canonicalizes
Outward. The enum participates in FerroUI's ordinary typed markup and nullable
value registry. `TryFrom<i32>` and `set_gravity_value` reject unknown numbers
without changing the range; `set_gravity` accepts only the checked Rust enum.

```rust
use ferroui_controls::rich_text::RangeGravity;

let caret = document.get_range(2, 2)?;
caret.set_gravity(RangeGravity::Forward);
let format = caret.character_format()?;
caret.set_text("X")?;
```

`Clone` shares the Rust range object and its metadata. `get_clone()` creates an
independent range and resets its mode to UIBehavior, matching native Duplicate.
Explicit endpoint setters retain the accepted mode. Changing it advances the
range generation, including a change away and back, so pending clipboard or
callback-bearing operations cannot mistake the newer range for their original
request. Assigning the same accepted value does not retire an operation.

## One resolver for queries and insertion

Collapsed character-format queries, ordinary `set_text`, plain-text paste and
Unlink insertion use the same character-format resolver. The range's explicit
typing patch overlays the resolved source. Explicit rich fragments keep their
own formatting. Paragraph-format queries continue to use the current paragraph;
Backward character affinity does not move the caret's paragraph alignment.

Nonempty character-format queries keep the existing uniform/mixed result. The
literal paragraph-boundary fixtures model paragraph marks with the preceding
run's style. The document does not independently format a paragraph-mark
character, so those fixtures do not establish arbitrary paragraph-mark format
parity.

The visible document excludes the native synthetic terminal EOP. Forward and
Inward at its final position therefore use the final paragraph's effective
defaults, or the editor's insertion defaults when no paragraph exists. Public
positions remain scalar-safe `usize` offsets with checked errors; native signed
endpoint clamping and extra terminal positions are not introduced here.

## Callback, clipboard and history ownership

The private range typing snapshot is shared through `Rc` while RangeData is
borrowed. Deep brush and collection operations happen only after releasing that
borrow, because a service can run application code. Read guards check both the
range generation and the model's intrinsic mutation version, including changes
inside held notification batches.

Grouped range edits temporarily install the complete resolved insertion format
and then restore the independent editor typing snapshot exactly. They do not
merge that temporary format into the user's patch when the rebased caret happens
to equal the resulting insertion caret. The replacement helper is restricted to
active grouped range edits and checks generation around copying and disposal.

SetText, direct rich snapshots and clipboard edits publish their acted endpoints
before editor notifications. A handler's newer endpoints or mode therefore
survive. A second guard after clipboard typing restoration catches callbacks
that occur late in the operation. Proven independent model writes retain the
existing external-change behavior and are not overwritten by stale rollback.

Gravity is range metadata rather than document content. A successful undo keeps
the range's current mode. If an outer text group fails, document content, history
and selection roll back while an explicitly accepted mode change remains. That
last policy is a stated FerroUI transaction translation, not an inference from
the native fixtures.

## Verification and remaining contracts

The `rich_range_gravity_` family contains 23 regression definitions, including
literal replay of 45 native formatting/insertion fixtures, 190 commands from ten
retention fixtures, and 24 invalid setters. Additional cases exercise independent
and shared clones, read-only mode, composition and redo preservation, rich and
plain insertion, exact typing restoration, grouped failure, custom brush
callbacks, endpoint publication and asynchronous cut/paste. Actual execution
results belong to the checkpoint records in [TESTS.md](TESTS.md).

The measured insertion and replacement fixtures show that native paired-range
endpoint rebasing differs from current FerroUI TextPointer tracking at carets,
end boundaries and overlap replacements. Formatting gravity does not change
that tracking policy. The actual selection is a distinct native object; its
consumer below is based on its own observations, not inferred from independent
ITextRange2 behavior.

## Live selection consumer

[`RichEditTextSelection::{gravity,set_gravity,set_gravity_value}`](../../../src/FerroUI.Controls/rich_text/rich_edit_text_selection_gravity.rs)
share the editor's mode across selection handles. All five enum constants keep
their pinned values. The actual selection retains Inward as 3; accepted Outward
reads back as UIBehavior (0). Invalid numeric setters leave state unchanged.
Changing this metadata works during read-only state, composition and publication
without editing content, raising a text event or creating an undo unit. An
accepted same-value setter records newer intent and invalidates an older read.
That last detail differs intentionally from the independent range's unchanged-
value policy described above.

Collapsed format queries and actual typed insertion use the same shared resolver.
Previous/following/current-paragraph choice follows the selected mode. Explicit
typing patches override the transient resolved source, but the source is never
promoted into that patch. Default typing therefore keeps normal inherited format
behavior after later movement. Nonempty range queries retain uniform/mixed
semantics. Live-caret legacy composition and exact input consume the affinity;
remote exact replacements keep their existing range inheritance policy.

The guarded read captures the model mutation identity, directed selection,
mode/options intent generations, navigation generation and editor checkpoint.
Callback-bearing format copying happens outside RefCell borrows. Newer mode,
selection, defaults or external model writes reject the stale operation.
Composition restoration preflights format reads before mutating the model, and
explicit history clearing cannot resurrect an old preview or a popped undo unit.

Successful undo/redo retains the current selection mode. A failed outer edit group
restores the group's starting selection mode with its other editor state. This
is a documented FerroUI policy; independent range metadata instead retains an
explicit accepted mode on failed groups. Explicit typing patches, IME transaction
rules and these history/error policies are framework choices, not native fixture
conclusions.

## Actual-selection evidence and limits

The immutable actual-selection run contains **250 commands in 18 fresh-control
fixtures**, with 18 canonical IUnknown/interface identity records. It uses system
MSFTEDIT `6.2.26100.33648`, separately from the independent-range record's build.
The matrix contains 80 setup commands and 170 target commands. Of the 250 calls,
248 have a Rust API representation; two null-BSTR TypeText observations remain
transport evidence because the Rust method accepts `&str`.

The 29 `rich_selection_gravity_` regression definitions replay the representable
commands, mode readbacks, duplicates, selected navigation units and projected
visible endpoints. Ten ordinary/paragraph formatting fixtures include 75 target
format checks and ten inserted-range checks. Five nonempty replacement fixtures
now assert the measured trailing-paragraph alignment. The later paragraph-boundary
matrix below supplies the additional evidence that permitted this correction.
The literal generated data keeps
all 250 native observations, including the two null-BSTR calls. New framework
runtime results belong to the current central gate in [TESTS.md](TESTS.md).

Remaining partials are the synthetic terminal EOP, independent duplicate cached
collapsed formatting/physical rebasing, initial native caret caches after external
format setup, automatic AT_END_OF_LINE updates, and the remaining unmeasured
paragraph-format survival rules. Native flags are retained in the literal data;
replay asserts
START_ACTIVE and REPLACE without pretending automatic line-affinity parity.
The stronger direct-child structural-install boundary in [LINK-FIELDS.md](LINK-FIELDS.md)
also remains. No system-MSFTEDIT record establishes the external WinUI engine's
complete behavior. These differences are RT-I32 and RT-I35.

## Measured TypeText paragraph boundaries and alignment

The expanded actual-selection oracle has executed on both x64 and ARM64. Its
**490 commands / 34 fixtures** include the historical 250-command matrix and
16 additional paragraph replacement fixtures containing **240 commands**. The
[x64 record](native-msftedit-e5a0778a-x64.json) and
[ARM64 record](native-msftedit-e5a0778a-arm64.json) preserve their separate engine,
checkout, workflow and verified archive provenance. Their fixture, command,
endpoint, format, HRESULT and interface observations agree. The earlier
250-command JSON remains immutable.

These measurements distinguish two cases that a plain string splice cannot:

| Source, visible UTF-16 selection and TypeText | Visible result | Alignment |
|---|---|---|
| `ab\ncd`, `1..4`, `"X"` | `aXd` | Trailing paragraph |
| `ab\ncd`, `2..3`, `"X"` | `abX\ncd` | Existing first and second paragraphs |
| `ab\ncd`, `1..3`, `"X"` | `aX\ncd` | Existing first and second paragraphs |
| `ab\ncd`, `0..3`, `"X"` | `X\ncd` | Existing first and second paragraphs |
| `ab\ncd`, `1..5`, `"X"` | `aX` | Trailing paragraph |
| `ab\ncd`, `1..4`, `"\r\n"` | `a\n\nd` | Trailing alignment on all resulting paragraphs |

The table uses the model's visible LF separators; the record keeps native CR and
its synthetic terminal EOP. Forward and reversed selections give the same text
and alignment. Empty TypeText remains a no-op with the directed selection intact.
The matrix also checks emoji replacement, opposite alignments and three source
paragraphs.

`RichTextEditor::prepare_typing_range` now excludes the last selected structural
paragraph mark when the selection ends exactly at the next block's start. It
uses actual block boundaries, so a literal LF inside a Run and an inline
`LineBreak` do not acquire paragraph protection. The accepted replacement range
is computed before overtype and MaxLength planning; a retained mark cannot create
false insertion capacity or cause rejected Unicode input to delete the selection.
Ordinary caret typing keeps its existing fast path. The original directed
selection stays in the undo snapshot. OVERTYPE also uses the original selection
to decide whether its first scalar replaces selected content, even when mark
protection has shortened the effective replacement range to zero.

A nonempty typed replacement crossing the remaining paragraph boundary retains
the trailing paragraph's **alignment** through the existing rich model splice.
This applies to all paragraphs created by typed CR/LF, with the existing TypeText
rule that CR and LF are separate typed breaks. An inherited alignment remains
unassigned in the resulting snapshot instead of being frozen to the current
value. Character formatting, semantic inlines, Link IDs and all other paragraph
properties keep their existing policies. Bulk SetText, Delete, paste and IME
replacement do not use the new typed endpoint/alignment policy.

The literal Rust replay executes all **240 new commands** and checks all
**64 external paragraph alignment queries**, eleven printable inserted-range
Bold/Italic queries, projected visible text, selection
direction, accepted Gravity and supported flags. The five older reversed emoji
replacement fixtures now agree on alignment as well. Twelve independent framework
tests cover scalar-safe truncation, both selection directions, exact undo/redo,
inherited alignment, rejected length-limited input, soft/literal breaks, atomic
publication, failed grouped edits and independent metadata. A combined OVERTYPE
regression checks retained marks, supplementary scalars, length limits and history.
Native observations independently establish OVERTYPE stopping at a mark and the
selected-mark retention rule; their combination remains a framework regression,
not a separately measured native fixture. These test definitions supplement the
native observations; execution results belong in [TESTS.md](TESTS.md).

This closes the measured TypeText endpoint/alignment discrepancy, not every
paragraph-format contract. Tabs, indents, line/paragraph spacing, arbitrary
paragraph inheritance, post-break cached character formatting, independent
paragraph-mark styling, bulk deletion/paste/IME policies and automatic
AT_END_OF_LINE affinity still need their own native comparisons. System-MSFTEDIT
results do not establish the packaged WinUI engine's complete behavior. The
relevant public API entries therefore remain partial.

## Measured endpoint evidence: Stage A

The separate native `range_tracking` source family adds 15 fresh-control
fixtures and 90 commands without extending or rewriting the immutable
independent-range Gravity or actual-selection records. Each fixture configures
an independent range, sets requested UIBehavior or Forward, creates its
duplicate, replaces text through a separate range and reads both modes. The
new relationships examine shared endpoints, exact/covering/right-overlap
replacements and carets, including identical text and retained prefixes or
suffixes. These cases distinguish an edit's original span from a minimal
visible-string difference. Their 90-command / 15-fixture native results are now
preserved in the separate x64 and ARM64 records linked above; both agree.

Raw HRESULTs, full story and primary/duplicate states are retained; neither the
probe nor its parser predicts a readback or endpoint law. The executed 490-call
actual-selection matrix and historical 250-call record remain separate.
The 1,577 independent Gravity and 137 Link observations stay immutable.

Because these snapshots read GetFont/GetPara, they can populate formatting
caches. They cannot resolve the remaining independent-duplicate cached-format
contract. Stage B needs a distinct experiment that controls when the first font
read occurs. The completed 50-test extractor gate checks the observation protocol
only. No production tracking or format-cache policy is changed by this
measurement alone, and the relevant API mappings remain partial.
