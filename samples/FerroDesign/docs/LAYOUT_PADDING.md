# Four-side auto-layout padding

Select an auto-layout frame, or one of its direct children, and open **Layout and grid placement…**. The **Frame padding** section appears before child placement controls, including on an empty frame. Top, Right, Bottom and Left are native FerroUI `TextBox` controls in the existing compiled-XAML editor shell. Their values are document units.

## Linked and independent sides

**Link all four sides for the next edit** is an explicit editing mode. It starts enabled when the four stored numbers are equal. Opening the dialog or changing this checkbox does not rewrite padding, create an override, reflow the document or add history. In particular, linking an asymmetric frame retains all four displayed values until a field is edited.

| Mode | Committed field edit |
| --- | --- |
| Unlinked | Changes the edited sides and preserves the exact numbers on untouched sides. |
| Linked | Uses the most recently edited dirty side as the value for all four sides. |

The compact inspector's single **Padding** field displays **Mixed** for asymmetric values. Entering a number there remains an explicit uniform edit. The detailed fields use the inspector's existing two-decimal display for fractional numbers; an untouched field retains its full stored precision when another side changes. A valid newly entered fractional value is stored without rounding to its display text.

## Commit, discard and saving

Enter, focus loss, **Apply padding**, **Done**, **Save design**, and the existing save/pagehide draft flush use the same group commit. All four visible strings must parse as finite, nonnegative numbers before any padding mutation occurs. Invalid input remains visible with an inline error. It blocks partial changes, relinking and dialog closure until corrected or discarded. Rejected Enter retains the field's focus.

Escape in a padding field discards all four pending padding drafts and restores their accepted display values without changing selection or the document. During IME composition, Enter/Escape retain the native composition behavior. The shared inspector lifecycle restores the last confirmed text before releasing IME focus for saving; preedit text is not an authored number.

**Save design** calls the existing native Save command and exports the complete editable design. Pagehide uses the production persistence host and IndexedDB lifecycle. The padding feature does not introduce a separate storage path or history stack.

## Validation and document behavior

`Editor::set_layout_padding(parent, Padding)` validates all four sides, rejects an active edit transaction and a locked frame or locked ancestor, and requires an existing auto-layout frame. It constructs and validates a candidate document, runs the existing complete layout arrangement, validates the result, and commits it in one history step. Invalid input or a rejected arrangement leaves the live document, selection, history and redo unchanged.

Reflow uses the existing row, column, wrapping, grid-track and nested Hug/Fill layout engine. Other layout settings and child placement metadata are preserved. Changes to a component instance's padding record the existing `layout` override, which protects that layout from subsequent component synchronization. Padding has no new serialization schema or renderer implementation.

An equal numeric padding value returns before arrangement or transaction creation. It therefore preserves redo, render tokens, instance metadata and even unsettled imported geometry. A different value may move children, resize Hug ancestors and move their siblings; undo and redo restore that complete document change.

Padding and grid-track controls share an accepted dialog baseline. The baseline includes the editor's render-change identity, parent node, selection and page, and advances after either control group commits. An intervening document edit or replacement editor rejects the old draft with a reopen message rather than overwriting the newer state. Ordinary child placement or ordering continues to rebuild the layout dialog through its existing route.

## Coverage and limits

The source includes eight core padding regressions in [`layout_padding_tests.rs`](../crates/ferrodesign-core/src/layout_padding_tests.rs), five native UI regressions in [`shell/layout_padding_tests.rs`](../crates/ferrodesign-app/src/shell/layout_padding_tests.rs), and the [`layout-padding-browser.mjs`](../tests/layout-padding-browser.mjs) scenario. They cover independent fractional values, linked editing, invalid groups and Escape, exact history and real pending redo, empty frames, nested reflow, explicit grid tracks, instance overrides, locks, transaction ownership, stale dialogs, confirmed IME input, pagehide persistence and native Save downloads. Browser screenshots use the common native-frame capture helper.

All eight core padding tests passed at source `7f81367248e26dfb2d07cfa73619646fa4de6ad1`, within the 511-test core suite. The [preserved core evidence](../../../docs/ferrodesign-evidence/2026-10-22-7f813672-core/) records that checkpoint. The five native UI tests and six browser checks remain authored coverage awaiting execution; this document does not claim an application or browser pass for them. Consult [VALIDATION.md](VALIDATION.md) for later results tied to a specific source checkpoint.

This is four-side numeric padding authoring; it does not add on-canvas padding handles, padding variables, per-side component override keys, or a measured Figma visual-parity guarantee. Linked mode belongs to the current dialog and is not saved in the document.
