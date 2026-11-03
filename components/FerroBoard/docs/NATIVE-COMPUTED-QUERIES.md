# Native computed-key queries

ComputedKeyboard::keys and visible_keys now return a Rust iterator whose items are
Result<Option<Rc<ComputedKey>>, KeyDataError>. An outer None means exhaustion; an inner
None preserves a nullable key in the unfiltered query. VisibleKeys rejects a null key at
its original predicate evaluation point. try_next is the non-null engine convenience.

ComputedKeys no longer has Current, boxed Current, MoveNext, Reset or Dispose APIs.
Consumers take ownership of each returned item and use normal Drop for the query.
iter starts an independent deferred traversal of the same shared rows. Rows are not
copied, nulls are not removed, and traversal errors are not converted to exhaustion.

Layout, HitTest and FindKey return Result. They keep the original geometry calculations,
early exits and partial-state update order. Engine layout/recompute, pointer hit testing
and control refresh propagate those errors directly. Computed-key metadata calls return
checked errors; the old KeyDataError panic recovery boundary has been removed from that
descriptor. Custom Rust panics are not modeled as CLR exceptions.

Markup keeps Keys, VisibleKeys, Rows, CharacterKeys, Layout, HitTest, FindKey and Snapshot
names and the existing overloads. Snapshot still retains nullable values and actual key
handles through the existing collection metadata boundary. Rust callers use question-mark
propagation or collect::<Result<Vec<_>, _>>() rather than assuming an infallible iterator.

Unchanged source fixtures remain:

- row-reentrancy.jsonl:48 complete compute-state rows and240 query-value/error projections
  replay.208 Reset/Dispose/post-Dispose runtime rows are retired. Cached Current/boxed
  Current fields after errors are excluded; successful returned items remain compared.
- computed-rows-null.jsonl:196 query-value/error projections,48 geometry/compute/find/hit
  operations and28 existing insertion rows replay.56 Reset/Dispose rows are retired.
  Its one mixed identity row is not counted as a replayed fixture row; independent native
  alias/query tests cover the retained handles and independent traversal.
- Existing layout/resource, gesture, engine and geometry corpora are unchanged.

New native tests exercise invalid row errors through queries, layout, engine updates,
hit testing and lookup; check partial width/height updates; repair the row and verify
recovery. The independent consumer invokes actual metadata methods and checks that those
failures return errors without unwinding. Query Drop and independent traversal are tested.

Generic ComputedList/ResourceObjectList backing, versioned resource cursors and dictionary
interfaces remain legacy cleanup. This stage removes the computed query's emulated runtime
surface and panic transport; it does not claim a completed storage migration. No dependency,
SDK, C# reference or embedded resource change is required. New native/device/platform
verification remains separate; older native receipts keep their exact archive/binary scope.

The inventory also drops six obsolete compatibility file targets in favor of inspected
current locale, computed-query, subtype, request and font modules. All member targets
are now checked for existence, including declarations outside the detailed native map.
This repairs stale evidence paths; it does not promote API completion status.
