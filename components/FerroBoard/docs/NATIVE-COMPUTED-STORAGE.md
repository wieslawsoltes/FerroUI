# Native computed row and key storage

ComputedList now owns Rc<RefCell<State<T>>> with Vec<Option<T>> storage. Clones retain
the actual buffer. The computed rows are local UI/engine state; this does not change
the separate threading model for shared resource caches.

The private revision guards the keyboard's reentrant computation contract: a key callback
can alter its row or the outer row set, and traversal rejects that edit when the affected
row traversal resumes. It is a native layout revision, not an exposed CLR collection
version. It uses u64 and has no exact runtime integer-wrap compatibility requirement.
No borrow spans key computation, user callbacks or dropping removed values.

ComputedItems and the non-null iterator implement Rust Iterator/FusedIterator. They have
no Current, boxed Current, MoveNext, Reset, Dispose, array-kind flags, empty-enumerator
singleton or interface/weak projection APIs. Exhausted iterators remain exhausted.
A fresh traversal sees current storage. The existing deferred computed-key query now
uses these iterators directly, with no ResourceObjectEnumerator in its row/key path.

Rust API migrations:

- Indices are usize. Signed XAML indexers convert and validate at the metadata boundary.
- push/push_nullable and clear return unit; remove returns the actual nullable value.
- get/get_nullable, set, insert and remove return explicit bounds/missing-value errors.
- snapshot returns Result<Vec<T>, _>; null cells no longer trigger typed panics.
  snapshot_nullable preserves the complete nullable data.
- The old nullable_storage resource-list bridge is removed. Clone the ComputedList handle
  to retain the actual buffer; mutation stays through the tracked native operations.
- Computed row/key markup views own the actual ComputedRows/ComputedRow. source returns
  that handle. The old from_original/original generic resource-interface bridges are
  removed. from_rows/from_row and existing markup names, Count and indexers remain.
- CharacterKeys and query Snapshot produce native computed buffers directly.

Existing row-mutation, nullable-cell, layout/geometry, engine and consumer fixtures
remain unchanged. No additional source cases are retired by this stage. The prior
query migration's explicit runtime-facet retirements remain in force. Negative source
fixture indices are translated by the test driver to native Result errors; native
buffer boundary coverage uses usize::MAX.

New checks cover actual native metadata buffer identity, live nullable/count changes,
snapshot errors and recovery, checked markup indices, and value destruction after
releasing the row borrow. The latter permits reentrant Rust Drop callbacks to observe
the committed state.

The common ResourceTextListError enum remains existing error vocabulary. Other generic
resource object/text-interface/dictionary code and its runtime compatibility facets
remain cleanup work. There is no new SDK, dependency, C# reference or resource change.
Prior native/relocated evidence retains its exact older source/binary scope; full
platform, touch, privacy, IME, accessibility and performance verification is incomplete.
