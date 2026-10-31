# FerroUI designer collaboration engine

`ferroui-designer-collaboration` replicates operations over the same source-preserving XAML engine
used by visual tools. It has no sockets, server dependency or rendering dependency. A host supplies
authentication, permission checks, durable storage and delivery over its preferred transport.

## Session and synchronization

```rust
use ferroui_designer_collaboration::DocumentReplica;

let mut alice = DocumentReplica::new("alice-installation-1", "design-42", "<Grid/>")?;
let snapshot = alice.snapshot();
let mut bob = DocumentReplica::from_snapshot("bob-installation-1", snapshot)?;
let operation = alice.set_attribute(alice.root(), "Width", "800")?;
bob.merge(alice.message(vec![operation]))?;
assert_eq!(alice.source(), bob.source());
# Ok::<(), ferroui_designer_collaboration::CollaborationError>(())
```

Create peers from the same snapshot. Actor identifiers identify durable writers; persist a writer's
snapshot/counter or create a new unique actor after restart. SHA-256 of the exact initial source
is included in synchronization messages. Snapshot merge also compares the baseline bytes.

`changes_since(version)` sends operations above each peer's highest *contiguous* actor sequence.
A gap does not advance the summary. Resending later operations is safe because identity and bytes
are checked, and duplicates are idempotent. Reusing an actor sequence or operation identity with
different content rejects the entire message before mutation. Timestamps must leave room for each
missing sequence, and a resumed writer with holes must synchronize before authoring new operations.
Contiguous vectors are maintained incrementally; reading them does not rescan the operation log.

Network hosts use `changes_since_bounded(version, actor, max_bytes)` to send one page whose actual
JSON representation, including escapes and envelope fields, fits the requested wire budget. Set
`actor` when uploading one writer's outbox. Apply the page and request the next with the updated
receiving vector. Pages expose complete sender prefixes so gaps cannot repeatedly fill a page
without advancing that vector. `pending_operation_count` counts an actor's outstanding writes
without cloning their source payloads.

Documents are bounded at 16 MiB, wire messages at 32 MiB, and complete retained snapshots at 64 MiB.
Checkpoint replay supports multiple wire batches; a large acknowledged log remains restorable.
The retained operation count is bounded at 100,000; checkpoint into a new session with a
new identity when a host needs compaction. Compaction requires coordinating all participating
peers and is deliberately not inferred from local receipt. Hosts must reject a vector-only
checkpoint while `has_sequence_gaps()` is true: the vector cannot identify unseen operations beyond
the contiguous prefixes.

## Conflict behavior

This is **ordered operation-set replication**, not a character-level text CRDT. Every replica
replays the same operation set in `(Lamport, actor, sequence)` order. Property changes to different
properties merge. Concurrent writes to one property choose the last operation in that order.
Insert/remove/reparent operations use shared identities, and cyclic moves or missing/deleted
parents become explicit conflicts. Transactions either apply completely or change nothing.
Operations delivered before their insertion dependencies remain in the set and are re-evaluated
when the missing earlier operation arrives. Sorting and replay make both the source and conflict
list independent of delivery order. Ordered delivery applies only the appended suffix; earlier
arrival triggers a replay from the common baseline.

Arbitrary text and whole-source edits use exact-source compare-and-set. A concurrent change
causes a visible conflict, preserving the winning authored source. This prevents a full-buffer
replacement from silently erasing another editor's property/layout edit.

`undo_property` submits a conditional `UndoAttribute` operation carrying the original writer's
identity. Its precondition is checked during every replay, so it refuses to overwrite a later
writer even when that writer supplied the same value or had not yet been delivered when undo
was authored. Changes to unrelated properties still merge. Failed operations never acquire
property ownership. Structural undo uses an explicit compensating
transaction. The replicated log is never mutated to pretend an already transmitted edit did not
happen.

## Designer integration

`submit_document_edits(&document, label, edits)` converts the existing designer `Edit` commands
into shared commands. A host document may have different process-local node IDs; exact source
spans map it to the replica's identities. A differing source buffer is rejected until it is
synchronized. After a successful submit or merge, project `replica.source()` into the host with
one document transaction and refresh the preview. Source trivia, namespace declarations and
unrelated markup stay under the lossless engine's ownership.

`Presence` transports selection and finite canvas coordinates with an actor sequence number.
Expiry uses the receiving host's monotonic time, avoiding assumptions about peer clocks. Presence
is ephemeral and is not retained in the durable operation log. Node identities are validated and
at most 1,024 actors are retained; prune expired entries before admitting new peers at that limit.

## Performance checks

```sh
cargo bench -p ferroui-designer-collaboration --bench replication
```

The dependency-free harness measures a 4,000-writer batch, repeated contiguous-vector reads, and
local property edits against 100 controls. It prints CSV timings and checks the resulting operation
counts. Compare release runs on the same machine; timings are observations, not flaky test gates.
