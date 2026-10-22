# Editor document model

This report covers the reusable document model in `ferroui-edit-core`. The source baseline is [AvaloniaEdit revision `7277e36366d754219d7ede16b6613786b32caf4c`](https://github.com/AvaloniaUI/AvaloniaEdit/tree/7277e36366d754219d7ede16b6613786b32caf4c/src/AvaloniaEdit). The [source inventory](editor-source-map.md) records every upstream source file, including files that do not yet have a Rust implementation. This report describes implemented behavior and the remaining differences; a source mapping is not a claim of complete API or behavioral parity.

## Architecture and framework boundary

`TextDocument` is a cheap-clone, identity-comparing handle backed by `Rc` and interior mutability. Documents, anchors, subscriptions, and editing services stay on the UI thread, matching FerroUI's object model. `TextSnapshot`, `Utf16Rope`, and `DocumentVersion` use immutable `Arc` storage and are `Send + Sync`, so callers can transfer snapshots to a worker without transferring the document. The core does not depend on a renderer, filesystem, thread scheduler, or clock. FerroUI controls consume this core through `ferroui-edit`.

All offsets and lengths are **UTF-16 code-unit counts**. Lines and columns are one-based; an empty document has one empty line. `from_utf16`, `replace_utf16`, and `get_text_utf16` preserve isolated surrogate units as well as valid pairs. Rust `String` cannot represent isolated surrogates, so the string-returning APIs replace those units with U+FFFD. This is an explicit presentation conversion: underlying text, snapshots, undo fragments, and offsets remain lossless.

The text store is a persistent AVL rope with shared leaves of at most 512 UTF-16 units. Branches cache lengths and line-break metrics. Edits copy the paths that change; snapshots clone a root pointer. CR, LF, and CRLF are recognized even when the pair crosses a leaf boundary or an edit creates or separates the pair. Line lookup traverses cached metrics and scans a bounded leaf. Holding a `DocumentLine` retains a value from the queried revision, while holding a `TextSnapshot` retains that entire immutable revision.

`LiveDocumentLine` provides the additional mutable line identity contract. The document creates its separate line index lazily when a caller asks for a live line or registers a line tracker. This index uses an implicit-order arena treap with cached subtree counts and lengths; line number and offset lookups have expected logarithmic cost, and edits do not individually shift all suffix objects. A line handle follows splits, merges, and suffix shifts, records deletion, and exposes previous/next identities. The first line survives whole-document rebuilds. An independently retained handle keeps the line index alive after the document is released. Existing snapshot-based consumers incur no live-index maintenance until it is enabled.

Anchors use an arena-backed treap with stable node identifiers, parent links, and lazy offset shifts. Edits split the affected interval from the unchanged suffix; the suffix receives one lazy shift. Only anchors in the changed interval require individual updates. Insert, remove, and offset lookup have expected logarithmic cost. This is an intentional data-structure adaptation from the reference red-black tree and has expected rather than worst-case logarithmic bounds. No unsafe code is used.

## Source mapping

All Rust paths in the table are relative to `src/FerroUI.Edit.Core/`. Consolidated files are identified explicitly.

| Upstream source | Rust source and symbols | Status and scope |
|---|---|---|
| `Document/TextDocument.cs` | `document/text_document.rs`: `TextDocument`, `DocumentEdit`, `DocumentError`, `UpdateGuard` | Adapted shared handle, UTF-16 edits, line queries, notifications, update groups, snapshots, versions, undo; remaining public APIs below. |
| `Document/DocumentLine.cs` | `document/document_line.rs`: `DocumentLine`; `document/live_document_line.rs`: `LiveDocumentLine` | Immutable query values and a separate stable live identity with deletion and sibling access. |
| `Document/DocumentLine.LineNode.cs`, `Document/DocumentLineTree.cs`, `Document/LineManager.cs` | `document/line_manager.rs`: `LineNode`, `DocumentLineTree`, `LineManager`; `utils/rope.rs`: `Utf16Rope` | Lazy live line treap ports split/merge/rebuild rules; the persistent rope retains immutable line metrics. |
| `Document/ILineTracker.cs`, `Document/WeakLineTracker.cs` | `document/i_line_tracker.rs`: `ILineTracker`; `document/weak_line_tracker.rs`: `WeakLineTracker` | Before-remove, before-length-change, after-insert, rebuild, and completed-change callbacks; explicit or weak registration. |
| `Document/TextLocation.cs` | `document/text_location.rs`: `TextLocation` | One-based values, ordering, formatting, and parsing. Negative coordinates are not representable by `usize`. |
| `Document/TextLocation.cs`, `Document/SimpleSegment.cs` | `document/text_location.rs`: `ISegment`, `SimpleSegment`; `document/text_anchor.rs`: `AnchorSegment` | Segment trait, immutable value, and live anchored segment; checked document ranges. |
| `Document/ITextSource.cs`, `Document/IDocument.cs` | `document/text_source.rs`: `ITextSource`, `TextSnapshot`, `StringTextSource`; `document/document_change_event_args.rs`: `TextChangeEventArgs` | Immutable snapshot/source contract and generic change offset semantics; remaining interfaces, IO and comparison overload gaps below. |
| `Document/RopeTextSource.cs` | `document/text_source.rs`: `TextSnapshot` | Immutable, shared rope snapshot replaces the wrapper class. |
| `Document/DocumentChangeEventArgs.cs` | `document/document_change_event_args.rs`: `DocumentChangeEventArgs` | Removed/inserted snapshots, validated custom maps, inversion, offset mapping. |
| `Document/OffsetChangeMap.cs` | `document/offset_change_map.rs`: `OffsetChangeMap`, `OffsetChangeMapEntry`, `OffsetChangeMappingType` | Immutable maps replace mutable/freezable lists; the four mapping modes retain their anchor behavior. |
| `Document/ITextAnchor.cs`, `Document/TextAnchor.cs` | `document/text_anchor.rs`: `TextAnchor`; `document/offset_change_map.rs`: `AnchorMovementType` | Shared anchor handles, before/after insertion affinity, deletion survival, location, explicit subscriptions; no separate `ITextAnchor` trait. |
| `Document/TextAnchorNode.cs`, `Document/TextAnchorTree.cs` | `document/text_anchor_tree.rs`: `TextAnchorTree` | Private lazy-offset treap and weak anchor references; deleted nodes are reclaimed. |
| `Document/TextSourceVersionProvider.cs` | `document/text_source_version_provider.rs`: `TextSourceVersionProvider`, `DocumentVersion` | Thread-safe checkpoints, forward and backward changes, offset translation, document identity checks. |
| `Document/TextSegment.cs`, `Document/TextSegmentCollection.cs` | `document/text_segment.rs`: `TextSegment`, `ITextSegment`, private `ISegmentTree`; `document/text_segment_collection.rs`: `TextSegmentCollection<T>` | Shared mutable intervals, generic payload composition, indexed inclusive-overlap queries, automatic document tracking and manual offset-map updates. |
| `Document/UndoStack.cs`, `Document/UndoOperationGroup.cs`, `Document/DocumentChangeOperation.cs` | `document/undo_stack.rs`: `UndoStack`, private `UndoHistory`/`UndoGroup`; `document/text_document.rs`: replay | Document/custom undo and redo, nested/continued groups, optional operations, size limits, original-file marker. |
| `Document/IUndoableOperation.cs` | `document/i_undoable_operation.rs`: `IUndoableOperation`, `IUndoableOperationWithContext` | Plain and context-aware custom actions; explicit push variants replace runtime interface probing. |
| `Document/DocumentTextWriter.cs` | `document/document_text_writer.rs`: `DocumentTextWriter` | Insertion-position writer, newline inference, UTF-16 writing, grouped line writes, `std::fmt::Write`. |
| `Document/TextUtilities.cs`, `Document/NewLineFinder.cs` | `document/text_utilities.rs`: `TextUtilities`, `LogicalDirection`, `CaretPositioningMode`, `CharacterClass` | Whitespace and indentation segments, newline discovery/normalization, reference caret stops and Unicode category classification. |
| `Utils/Rope.cs`, `Utils/RopeNode.cs`, `Utils/CharRope.cs` | `utils/rope.rs`: `Utf16Rope` | Specialized persistent UTF-16 rope; not a generic collection or complete overload-for-overload `Rope<T>` API. |
| `Utils/DelayedEvents.cs`, weak event plumbing | `utils/events.rs`: private `Event`, `DocumentSubscription`; document edit notification queue | Callback snapshots and explicit subscription disposal replace framework weak-event managers. Weak subscription ownership is not automatically reproduced. |

## Mutation, anchors, and event semantics

`insert`, `remove`, and `replace` validate offsets, range overflow, read-only state, and update state before writing. `apply_edits` validates the entire list of nonoverlapping original-coordinate edits before applying them in reverse offset order under one update group. Errors are `DocumentError` values. No partial mutation occurs for an invalid range or overlapping batch. An update group coalesces history and completion notifications; it is not a rollback transaction for arbitrary caller code.

The outer update raises `UpdateStarted`. Each nonempty edit raises `Changing`, replaces the rope, updates versions and any enabled live line index, updates anchors, raises the line tracker's `change_complete`, raises deleted-anchor callbacks after **all** deletion flags have been set, and raises `Changed`. At the outer update boundary, `TextChanged`, length/line-count/property notifications, and `UpdateFinished` are coalesced. Handlers execute after internal `RefCell` borrows have been released. Reads from notifications are valid. Reentrant document writes from `Changing`, line tracker callbacks, `Changed`, or an anchor-deleted callback return `ReentrantChange`; writes from `TextChanged` can join the pending undo group and trigger another completion pass. Subscription removal during dispatch applies to future dispatches; the current handler snapshot remains stable.

`ILineTracker` receives low-level notifications while the live line index is being updated. Before-remove and before-length-change callbacks observe the old line state; after-insert callbacks observe the new identity. As in the reference implementation, a line's transient length and delimiter may not yet describe the final text, and consumers must not publish UI events until `change_complete`. Rust's unsigned content length clamps a transient negative difference to zero. The reference `LineManager` implementation emits length notifications only when total length changes, despite broader wording in the interface comments; the Rust port follows the implementation. Whole-document replacement preserves the first identity and sends `rebuild_document`, then `change_complete`. `WeakLineTracker` upgrades a weak target for each notification and removes its subscription after the target expires. Strong tracker subscriptions require explicit disposal.

`DocumentSubscription::dispose()` unsubscribes explicitly. Dropping an ignored subscription handle keeps the subscription active, following FerroUI's event convention. A subscriber that captures its document strongly should dispose its subscription when the owning service ends; otherwise that ownership cycle can keep the document alive.

Anchor movement is tested separately for normal replacement, remove-then-insert replacement, character replacement, and keeping default anchors before insertion. Normal deletion removes anchors strictly inside the removed interval unless `survive_deletion` is set. Boundary anchors survive. Character replacement retains interior relative positions and clamps anchors when the replacement shrinks; a character replacement with an empty insertion or removal uses normal mapping. Generic `TextChangeEventArgs` and `DocumentChangeEventArgs` intentionally have different endpoint mapping rules, as in the reference source.

Undo entries retain shared removed/inserted rope fragments or `Rc` custom operations. Nested updates form one entry; continued updates can extend the previous entry with the reference empty-group rules. Custom actions are already performed when pushed; undo runs them in reverse order and redo in forward order, interleaved with document changes. Context-aware actions receive the stack so they need not capture and retain its document. Optional actions are discarded when a new group contains no required operation; optional continuation can attach to the previous group without creating a new text state. New actions after undo discard redo history. The original-file marker uses history state identity, so undo/redo can return to the marked state and branching can invalidate it. Read-only documents reject undo and redo before history changes. The size limit counts top-level groups, including custom actions, and discarded custom payloads are dropped outside document borrows. Arbitrary document writes during custom playback are rejected, matching the reference playback restriction.

`TextSegmentCollection<T>` is a shared generic interval index. Extension handles implement `ITextSegment` by returning their embedded shared `TextSegment`. The arena treap caches maximum interval ends and lazily shifts suffix offsets, giving expected logarithmic insertion/removal and affected-interval updates. Queries include touching endpoints and return owned snapshots; custom payload cloning runs outside tree borrows. Intervals wholly removed by a document edit remain in the collection with zero length. Pure insertion at a segment start moves the segment forward; insertion at its end does not expand it. Collection drop or removal detaches surviving segment handles at their final offsets. Connected intervals are validated against the document extent so all automatic shifts remain representable; disconnected collections can track any nonoverflowing offset. `disconnect()` permits subsequent manual updates.

Versions retain forward change links from a held checkpoint. The current document does not retain an unbounded backward chain. A caller retaining an old checkpoint intentionally retains the changes needed for translation to newer revisions. Dropping a long retained chain releases it iteratively. Snapshot fragments inside changes have their version removed to prevent reference cycles.

## Explicit differences and remaining APIs

These gaps are not implemented as placeholder methods and are not implied complete by the source map:

| Reference surface | Current difference or missing API |
|---|---|
| Live `DocumentLine`, `PreviousLine`, `NextLine`, `IsDeleted` | `get_line_by_number`/`get_line_by_offset` return immutable `DocumentLine` values. Use `get_live_line_by_number`/`get_live_line_by_offset` for `LiveDocumentLine` identities. Coordinate access on deleted identities returns `DeletedLine`; length metadata remains readable. The live index has expected logarithmic treap bounds rather than red-black worst-case bounds. |
| `ILineTracker`, `WeakLineTracker` registration and intermediate text | `add_line_tracker` returns an explicit disposable subscription instead of exposing a mutable tracker collection. Low-level callbacks see the final committed document text while the live index is transitional; the reference removal phase exposes intermediate text before insertion. `change_complete` sees a consistent final state in both. Weak registration does not keep a released document alive. |
| `IDocument`, `IDocumentLine`, `ITextAnchor`, `ITextSourceVersion` interface families | Concrete document, line, anchor, and version types are exposed; only `ITextSource` and `ISegment` have dedicated traits. Host code requiring the full interfaces needs adapters. |
| `TextDocument` ownership transfer and synchronization | `Rc` makes the mutable document UI-thread-only at compile time. `SetOwnerThread`, `VerifyAccess`, `TryGetLock`, and lock-based cross-thread mutation are not provided; transfer immutable snapshots instead. |
| Document service container, `FileName`, general document-changed event wrapper | No `IServiceProvider`/`IServiceContainer`, filename metadata, or `DocumentChangedEventArgs` adapter in the core. Hosts own those services and metadata. |
| `UndoStack` descriptors and sharing | Custom and optional actions are implemented. `LastGroupDescriptor`, descriptor-bearing overloads, and assigning one undo stack to multiple documents remain absent. The independent context trait is selected through `push_with_context` rather than runtime interface discovery. |
| `TextSegmentCollection` inheritance and bounds | `ITextSegment` composition replaces subclassing. Connected intervals must lie within the document, while the managed API permits arbitrary offsets. The treap has expected rather than red-black worst-case bounds. Explicit property changes raise `segment_changed`; automatic document changes are silent. Disconnected collections remain UI-thread handles, not concurrently readable objects. |
| Mutable `OffsetChangeMap` and freeze semantics | Maps are immutable owned Rust values. Build entries, construct a map, and validate it when attaching to a change. |
| `Rope<T>` generic collection, lazy `FunctionNode`, `RopeTextReader` | Storage is specialized for UTF-16. There is no deferred arbitrary rope function node, generic collection facade, or `TextReader` compatibility wrapper. Chunk traversal and snapshots provide the implemented streaming boundary. |
| `ITextSource` readers, writers, and culture-sensitive search overloads | UTF-16 range access, snapshots, ordinal forward/backward search are present. Managed IO classes and the entire `StringComparison` overload family are absent. |
| Control caret navigation | `TextUtilities` implements the reference UTF-16/codepoint and combining-mark stop rules. Editing services may use Unicode grapheme navigation; that is a separate host behavior and not an identity claim about every reference caret mode. |
| Invalid/negative values | Rust offsets use `usize`, errors use `Result`, and empty locations use `(0, 0)`. Negative sentinel offsets and culture-dependent formatting are not reproduced. |
| Newline fallback | Existing document delimiters are inferred; a document without a delimiter defaults to LF consistently across targets. There is no host-dependent `Environment.NewLine`. |
| Exceptions from client event handlers | Rust panics are programmer failures; recovery from a panicking handler is not a transactional guarantee. Normal invalid input and reentrant mutation use `Result`. |

Selection, folding, search, completion, and snippets consume the document and interval primitives. Their additional contracts are recorded in the [editor services report](editor-services.md).

## Verification

The document and rope tests combine translated edge cases with deterministic model-based checks:

- Mixed CR, LF, CRLF and empty last lines; CRLF assembled or split at rope boundaries; UTF-16 supplementary characters and isolated surrogates.
- Stable live line identities across splits, merges, CRLF joining, deletion, whole-document replacement, and document release; low-level tracker ordering, weak expiry, registration during `Changing`, and 2,500 randomized edits with undo/redo and treap/rope consistency checks.
- All anchor affinities and deletion modes, boundary survival, deleted-event ordering, arena reclamation, randomized edits, and thousands of lazily shifted suffix anchors.
- Nested and continued undo groups, empty continuation groups, limits, redo branching, original-file markers, read-only rejection, overflow, invalid maps, and batch prevalidation.
- Custom action ordering mixed with document changes, context callbacks, optional-only and continued groups, playback rejection, panic guard cleanup, and payload destructors reentering document reads after limit/clear operations.
- Inclusive interval queries, automatic and manual tracking, zero-length and replacement boundaries, typed payload reentrancy, stable detachment, 2,500 model-checked tree edits and 8,000 lazily shifted suffix intervals.
- Callback ordering, explicit subscription disposal, reads inside notifications, rejected reentrant writes, and edits from `TextChanged`.
- Forward/backward version translation, cross-document rejection, snapshots crossing threads, and iterative release of a 40,000-change chain.
- AVL balance and cached metrics checked against a flat UTF-16 model for 1,600 randomized edits and 5,000 repeated appends.
- Newline/whitespace utilities, Unicode character categories, surrogate/combining-mark caret positions, and text-writer cursor/undo behavior.

Reproduce with:

```sh
cargo test -p ferroui-edit-core document --lib
cargo test -p ferroui-edit-core utils::rope --lib
python3 scripts/edit-port/editor_port.py --check
```

Verified checkpoint: the combined Cargo run reports `83 passed; 0 failed` for `ferroui-edit-core`, `28 passed; 0 failed` for the control, and `2 passed; 0 failed` for `EditorDemo`. This includes all live line additions. A standalone harness compiling the same `document/` and `utils/` modules also reports `58 passed; 0 failed` (56 document tests and 2 rope tests). The upstream-name convention scan over `document/` and `utils/` finds no matches. These tests exercise the listed contracts; they do not establish complete upstream test-suite parity or the absent interfaces above. Attribution and the upstream MIT license accompany the crate in `src/FerroUI.Edit.Core/NOTICE.md`.
