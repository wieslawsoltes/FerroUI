# Snippet expansion and interactive lifecycle

Reference: AvaloniaEdit `7277e36366d754219d7ede16b6613786b32caf4c`,
`src/AvaloniaEdit/Snippets`. Attribution is retained in
`src/FerroUI.Edit.Core/NOTICE.md` and `src/FerroUI.Edit/NOTICE.md`.

The core prepares the complete expansion before a control applies its protected
edit transaction. Activation attaches UTF-16 anchors to that accepted expansion.
The native `SnippetController` owns keyboard and editor-event subscriptions;
the core owns document subscriptions and grouped mirror edits.

## Source mapping

| Upstream source | Rust implementation | Contract |
| --- | --- | --- |
| `Snippet.cs`, `SnippetElement.cs`, `SnippetContainerElement.cs`, `SnippetTextElement.cs` | `snippets/snippet.rs` | Structured preparation and insertion; containers; additional numeric tab-stop syntax |
| `SnippetReplaceableTextElement.cs`, `SnippetBoundElement.cs` | `snippets/snippet_session.rs` | UTF-16 anchored fields, linked changes, atomic mirror validation and grouped undo |
| `SnippetCaretElement.cs`, `SnippetSelectionElement.cs` | `snippets/snippet.rs`, `snippet_controller.rs` | Selected-text expansion and retained final caret |
| `InsertionContext.cs` lifecycle | `PreparedSnippet`, `SnippetSession` | Before-activation completion/deactivation callbacks, whole-snippet deletion tracking and explicit teardown |
| `SnippetEventArgs.cs` | `snippets/snippet_event_args.rs` | All six upstream reasons, plus explicit `Completed` and `Error` outcomes |
| `SnippetInputHandler.cs` | `snippet_controller.rs` | Native key routing, completion-popup precedence, field traversal, Return acceptance and Escape cancellation |

## Lifecycle and ownership

`PreparedSnippet::on_insertion_completed` and `on_deactivated` register observers
before activation, including for a plain expansion with no active elements.
Completion observers run before immediate `NoActiveElements` deactivation.
After activation, `SnippetSession::deactivated` returns the same explicit
disposable subscription type as the document event system.

Deactivation is terminal and idempotent. It clears the active field, records the
reason and removes document subscriptions before invoking observers. Observers
can inspect the session or request teardown again without nested notifications
or a borrowed listener collection. Document subscriptions and undo ownership
records retain weak session references.

The whole expansion uses tracked document anchors. Deleting an initially
nonempty expansion, including undoing its insertion, deactivates the session
before the linked-field handler can recreate deleted mirrors. An initially
empty expansion remains interactive. `set_deactivate_when_empty(false)` is an
explicit extension for hosts that intentionally retain all-empty linked fields.

Return accepts the snippet, consumes the key and moves to the retained final
caret. Escape cancels interaction while preserving the current selection. Host
replacement or disposal reports `InputHandlerDetached`. Numeric traversal after
the last tab stop reports `Completed` and moves to the final caret. Automatic
mirror validation failures report `Error` and retain their diagnostic through
`SnippetSession::error()`. Terminal sessions release native keyboard handlers
immediately, even while the application retains the controller.

## Explicit adaptations and remaining work

Numeric tab stops traverse by ID and leave interactive mode after the final
stop. This existing Rust extension differs from upstream's positional cyclic
Tab traversal. The current custom active-element, transformed binding, named
anchor and insertion-context formatting surfaces are tracked separately from
the lifecycle support; their presence is not implied by the source mapping.
Placeholder background/outline rendering needs the editor's background-layer
integration. Native keyboard tests do not establish operating-system IME or
screen-reader behavior.

## Validation

The first lifecycle checkpoint passes all **14** focused core snippet tests,
including existing coincident empty-field and undo/redo ownership cases, ordered
completion/deactivation, reentrant teardown, subscription disposal and deletion
without mirror resurrection. Added native control cases cover Return versus
Escape caret behavior and handler cleanup after undoing insertion; they belong
to the shared `ferroui-edit` integration gate.

```sh
cargo test -p ferroui-edit-core snippets --lib
cargo test -p ferroui-edit snippet_controller --lib
```
