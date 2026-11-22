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

On the native editor, insertion guards cover selection replacement, explicit
completion ranges and the standalone `SnippetController` entry point. A
recursive request on the same editor during insertion or teardown of an outgoing
snippet returns `DocumentError::ReentrantChange`. Replacing a snippet checks the
original document revision and selection after deactivation. The controller
also rechecks the document binding and protection provider after policy callbacks
run, before applying its edit. A later insertion can use the new binding or
selection once the callback has finished.

## Bundled language snippets

`TextMateSnippet` parses the syntax used in the pinned packages and binds the
selected-text variable before preparing the native expansion. A completion
prefix stays distinct from the editor's actual selected text. `TextEditor` also
offers named insertion for the language at the start of the selection.

An outer field can contain independently editable children. Editing a child
updates its mirrors and mirrors of the containing field. Replacing an outer
field removes its covered children from navigation. Undo restores their UTF-16
ranges and Redo removes them again. These changes share the document's
existing undo group.
[Reference package syntax](https://code.visualstudio.com/docs/editing/userdefinedsnippets)

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

During IME preedit the snippet handler leaves traversal, acceptance and
dismissal keys alone. Cancelling preedit keeps the current selection. Committing
text updates the active field and its mirrors in one undo step. Tab resumes
normal field navigation once preedit ends. The native regression checks Tab,
Shift+Tab, Enter and Escape during preedit and commits a string containing a
supplementary character. The linked browser case also replays a trusted Enter
key during composition before committing a field prefix.

## Explicit adaptations and remaining work

The context separates checked preparation from live activation instead of
exposing arbitrary mutation of an insertion-position property. Custom nodes use
traits and identity handles rather than managed inheritance or serialized XAML
objects. Standard primary fields use numeric IDs; custom nodes use `Rc` identity.
Named anchors require unique names. Linking rejects initially overlapping ranges
rather than creating an overlap that later suppresses bound writes. An explicit
256-level expansion limit returns a diagnostic for recursive custom/container
expansion; the forward-default scan uses an iterative stack.

The native controller draws active and linked fields through the editor's
background layers. Native traversal uses numeric stop order. Callers can
request positional cycling through the session API. Additional input-handler
combinations and rendering equivalence remain open. `Snippet::parse` retains
its numeric syntax. `TextMateSnippet` handles the separate bundled dialect.

### Virtual-space insertion

FerroUI adapts the pinned snippet's physical insertion position to the
editor's native virtual columns. Structured snippet insertion and named
package insertion materialize the lower virtual column of a single
selection using the editor's existing tab and space rules. On an
indentation-only line, the snippet uses that indentation for continuation
lines. Placeholder ranges begin after the padding, including empty fields.
The padding and expansion share one protected edit and one undo group.
[Pinned snippet insertion](https://github.com/AvaloniaUI/AvaloniaEdit/blob/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit/Snippets/Snippet.cs#L30)

The regressions cover a backwards virtual selection, a supplementary
placeholder, CRLF and tab indentation, linked edits, named package insertion,
empty fields, protected insertion and excessive padding.

### Package snippet syntax

Choice fields use `${1|first,second|}`. The first value is inserted by default.
Lists preserve their order and support escaped commas, pipes and backslashes.
Entering a choice field opens the editor's native completion list. Accepting an
option updates its linked occurrences and advances to the next stop. Free
typing dismisses the list and keeps the field linked. Shift+Tab revisits a
field, and Escape dismisses the list while leaving the snippet active.

The core exposes `SnippetSession::active_choices` and `choose_active` for
other hosts. The edit policy checks prospective changes to the selected field
and its mirrors. Mirror edits run in the same undo group. A mirror inside a
parent contributes to the parent's own mirrors before either is written.
See the [snippet choice syntax](https://code.visualstudio.com/docs/editing/userdefinedsnippets#_choice).

Both named insertion and completion resolve document variables at expansion
time. Selection uses the actual selected text. The line and word come from
the caret position before insertion, in UTF-16 coordinates. Word lookup uses
the active language's pattern when present and the editor's identifier rules
otherwise. Comment delimiters use the language selected for the expansion,
and filename variables use the document's filename metadata.

`TextMateInstallation::set_snippet_variable_provider` lets an application
supply values such as workspace paths, a clock, random values or clipboard
text it has already read. Host values are literal text and take precedence
over editor metadata. Selection remains editor-owned. An unavailable standard
variable uses its default or empty text. An unknown name without a default
becomes a linked editable field. Expansion checks the document, selection and
language installation before editing, and rejects recursive resolution.
See the [snippet variable syntax](https://code.visualstudio.com/docs/editing/userdefinedsnippets#_variables).

Variable transforms run as part of expansion. An unavailable variable transforms
as empty text. The format can reference captures, insert conditional text, or
apply the documented case operations. Patterns and flags use the Corvus
ECMAScript regex engine, which matches UTF-16 code units with the `u` and `v`
flags controlling Unicode interpretation. The output goes through the snippet's
Rust string interface, which represents isolated surrogate units as U+FFFD.
Case operations use Rust's locale-independent Unicode casing.

Placeholder transforms run when leaving the field through Tab, Shift+Tab, or
Return. Typing keeps all occurrences linked as plain text while the field is
active. A stop declared through transform syntax alone is editable. Each
occurrence can use its own replacement, and any affected enclosing fields
mirror the result. A transform batch uses a single undo group and preflights
all replacements against the edit policy before editing. Escape dismisses
the snippet without applying pending transforms. The fallible session methods
report a failed transform before navigation.

Native and linked browser execution uses a 50 ms regex matching budget.
The bare `wasm32-unknown-unknown` library uses a clock-free match because
that target's standard library lacks a monotonic clock. The browser editor
uses the linked build. Patterns that require the engine's separate parser
thread return a compile diagnostic on targets without thread support.
See the [snippet transform syntax](https://code.visualstudio.com/docs/editing/userdefinedsnippets#_variable-transforms).

## Tests

Native lifecycle regressions cover recursive requests from deactivation and
custom completion, callback-driven selection changes, document replacement with
a live snippet, and a read-only provider rebinding the editor. A successful
replacement retains one active snippet and undo restores the preceding text.

```sh
cargo test -p ferroui-edit-core snippets --lib
cargo test -p ferroui-edit snippet_controller --lib
cargo test -p ferroui-edit package_snippets::tests --lib
```
