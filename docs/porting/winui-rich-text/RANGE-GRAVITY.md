# Independent range formatting gravity

`RichEditTextRange` has a typed `RangeGravity` property for choosing character
formatting at a collapsed visible UTF-16 boundary. The implementation shares
FerroUI's document, effective format resolution, editor transactions and typing
patches. It does not create a second document or use native COM objects.

The public names and values come from the pinned
[Microsoft.UI.Text.xml](https://github.com/microsoft/microsoft-ui-xaml/blob/fe244ca6ac4d482712b234a9de3cf771cc6481de/packaging/Intellisense/generated/Microsoft.UI.Text.xml).
Behavioral evidence is the immutable
[system-MSFTEDIT observation record](native-msftedit-links-gravity.json), with
engine, source and artifact provenance recorded in [TESTS.md](TESTS.md).
System MSFTEDIT observations are separate from the external WinUI text engine.

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

Two separate native contracts remain open. First, the measured insertion and
replacement fixtures show that native paired-range endpoint rebasing differs
from current FerroUI TextPointer tracking at carets, end boundaries and overlap
replacements. Formatting gravity does not change that tracking policy. Second,
the live selection is a distinct native object; its inherited Gravity cannot be
inferred from independent ITextRange2 observations. A separate 250-command,
18-control probe checks the actual selection's extended-interface identity,
typing, navigation, duplicates and readbacks. The immutable [selection record](native-msftedit-selection-gravity.json) now
preserves that actual native run, including interface identities and exact
readbacks. Production selection Gravity remains a separate implementation;
collecting the observations does not complete it.
