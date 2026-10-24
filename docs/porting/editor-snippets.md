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
| `SnippetReplaceableTextElement.cs`, `SnippetBoundElement.cs` | `snippets/snippet_session.rs`, `snippet_element.rs` | UTF-16 anchored fields, replaceable text-change observers, transformed linked changes, atomic mirror validation and grouped undo |
| `SnippetCaretElement.cs`, `SnippetSelectionElement.cs` | `snippets/insertion_context.rs`, `snippet_controller.rs` | Selected-text indentation, conditional final caret and accepted/cancelled caret policy |
| `InsertionContext.cs` | `snippets/insertion_context.rs`, `PreparedSnippet`, `SnippetSession` | Context formatting, active registration/lookup, before-activation callbacks, existing-range linking, whole-snippet deletion tracking and explicit teardown |
| `IActiveElement.cs`, `IReplaceableActiveElement` | `snippets/i_active_element.rs` | Object-safe custom lifecycle, editability, tracked segment and text-change contracts |
| `SnippetAnchorElement.cs` | `snippets/snippet_anchor_element.rs` | Named tracked text anchors and protected replacements |
| Custom `SnippetElement` implementations | `ISnippetElement`, `SnippetElementRef` | Checked expansion callbacks and deferred active-element factories with shared identity |
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

Activation inside the host insertion transaction records an optional weak undo
operation. Undoing an atomic selection replacement therefore deactivates the
insertion even when restoring the old selection leaves nonempty anchor ranges.
Undo of later field edits keeps the session active; redo does not reopen a
terminated session. Activation outside a document update has no insertion undo
record, so custom hosts should activate within their insertion transaction.

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

## Context formatting and structured extensions

`SnippetInsertionOptions::from_document` captures leading indentation and the
local line terminator before selection replacement. Native insertion also uses
`EditorOptions::indentation_string()`, preserves selected leading whitespace,
and rejects document or selection changes made by preparation/update-start
callbacks before applying the original edit. Whitespace-only selections are
clamped to their actual extent.

`insert_text` expands tabs first, then normalizes CR/LF/CRLF and adds captured
indentation to each subsequent line. This order preserves CRLF pairs assembled
across a tab-expansion boundary. The common no-transform path borrows the input
and appends directly to the prepared output. Selected-text elements trim initial
spaces/tabs and apply their explicit additional indentation using the captured
line terminator, following the separate upstream selection path. All positions
advance by UTF-16 units. The preparation API also exposes `insert_verbatim` for
already formatted content.

An application implements `ISnippetElement::insert` to append text, standard
fields, named anchors, nested nodes or a final caret. It registers an active
factory through the scoped `SnippetInsertionContext`; each `SnippetElementRef`
identity may register once. Preparation cannot itself acquire a document through
that context. Factory callbacks run after the host accepts the complete text,
and all factories finish before active-element completion callbacks run. An
active factory creates its tracked ranges without changing document text.
Factory document mutation and invalid ranges are reported as errors.

Custom `IActiveElement` instances participate in traversal, expose their current
segments, receive completion and exactly one terminal deactivation callback, and
are available through `get_active_element`. `WeakSnippetSession` supports
callbacks without ownership cycles. A dropped live session invokes custom
teardown with `InputHandlerDetached`. Completion failures deactivate all created
active elements, including those whose completion callback has not run yet.
Preparation errors leave the document unchanged; errors after accepted insertion
retain that text and release the failed interactive state. Arbitrary extension
side effects are not rolled back.

`SnippetElement::Anchor` creates a named `AnchorElement`, whose `text`, `segment`
and `set_text` follow UTF-16 edits. Empty anchors enlarge correctly when filled.
`activate_with_validator` installs the host policy before custom completion
callbacks can write anchors or replace fields. Named anchor writes and active
field replacement recheck document versions after application policy callbacks.

## Binding, events and navigation

`SnippetTextTransform` provides the `SnippetBoundElement.ConvertText` extension
point as an infallible or fallible Rust callback. Every changed primary field is
observed, including fields edited through the document while another field is
selected. The session computes all changed mirror replacements, checks overlaps,
and validates the complete write set before applying it in reverse offset order.
The continuation uses the initiating edit's undo group. Unchanged transformed
mirrors produce no edit or protection check. Replacement text-change observers
run after synchronization and can inspect the fully updated document.

Converter and validator callbacks run without retained collection borrows.
Synchronization is guarded before invoking these callbacks, and a version change
rejects the stale plan. A converter error never partially writes other mirrors.
Undo/redo replay their recorded mirrors and update observers without generating
additional changes. `SnippetSession::link` and `link_with_transforms` attach the
same machinery to existing nonoverlapping document ranges without an initial
text write.

`next_element` retains numeric tab ordering and completion after the final field.
`next_element_from_offset` supplies upstream positional cyclic navigation,
including backward traversal and wrapping. Custom editable elements participate
in both APIs. The native controller retains the numeric traversal convention and
defers keyboard ownership while a completion popup is open.

## Explicit adaptations and remaining work

The context separates checked preparation from live activation instead of
exposing arbitrary mutation of an insertion-position property. Custom nodes use
traits and identity handles rather than managed inheritance or serialized XAML
objects. Standard primary fields use numeric IDs; custom nodes use `Rc` identity.
Named anchors require unique names. Linking rejects initially overlapping ranges
rather than creating an overlap that later suppresses bound writes. An explicit
256-level expansion limit returns a diagnostic for recursive custom/container
expansion; the forward-default scan uses an iterative stack.

Placeholder background/outline rendering still needs the editor's background
layer integration. Native default traversal uses numeric stop order; callers
request positional cycling through the separate session API. Virtual-space
padding for snippet insertion, custom input-handler stacking beyond completion
precedence, and rendering equivalence of placeholder adornments remain host
integration work. Numeric notation deliberately rejects variable/choice/regex
syntax; structured callback nodes and transforms are the supported extension
mechanism. These parser additions are not upstream snippet language features.

## Validation

The extension checkpoint passes all **28** focused core snippet tests, including
existing coincident empty-field and undo/redo ownership cases; ordered
completion/deactivation; reentrant teardown; deletion without mirror
resurrection; custom registration/factory/completion/disposal; named-anchor
writes and protection; transformed multi-source edits, no-op mirrors and
reentrant callback rejection; existing-range linking; local formatting; selected
text indentation; conditional carets; stale preparation and pre-activation host
policy. Added native control cases cover Return versus Escape, input cleanup on
undo and preservation of selected indentation; they belong to the shared
`ferroui-edit` integration gate.

```sh
cargo test -p ferroui-edit-core snippets --lib
cargo test -p ferroui-edit snippet_controller --lib
```
