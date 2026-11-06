# Native resource text storage

Resource authors, extension dependencies and popup-author metadata now use ResourceTexts,
a shared Arc<RwLock<Vec<Option<Utf16Text>>>>. Cloning retains actual allocation identity;
from_shared/shared_values expose the Rust storage explicitly. Defaults are fresh mutable
buffers, not process-wide empty-array singletons.

Indices are usize. push, clear, remove_value and position use Rust collection conventions;
bounded get/set/insert/remove_at return explicit errors. Signed XAML indices remain checked
at the metadata boundary. Existing Authors and Count markup names remain available.

Iteration snapshots nullable UTF-16 handles and returns a standard Vec::IntoIter. No lock
spans callbacks; reentrant or cross-thread mutation is visible to aliases and subsequent
iterations, while an existing iterator keeps its original snapshot. There are no managed
Current/boxed Current/MoveNext/Reset/Dispose APIs, version checks or fixed-array flags.
Snapshot creation costs O(n) handle clones; these short resource metadata lists are not
the pointer-move or dictionary hot path. No additional asynchronous scheduling is introduced.

The unchanged 372-row C# resource-text fixture remains in place. Native tests retain
69 data/mutation/JSON row projections and two metadata alias/cache projections. The 280
cursor rows and 21 default-array mutation rows are retired runtime conformance, not
keyboard algorithm coverage. Mixed identity/shape/metadata rows assert only the useful
fields; singleton/fixed-array/cursor fields are explicitly excluded. Separate native tests
cover fresh-default isolation, shared ownership, snapshot stability, reentrant callback
mutation and usize boundaries. Four other default-model fixtures omit only the author
singleton/read-only fields; their other behavior assertions remain.

ResourceTextListError remains shared legacy error vocabulary for other resource paths;
its removal/renaming is separate work. Generic object lists and dictionary cursors have
not been migrated in this stage. No SDK, C# reference, fixture data or embedded resource
change is required. Native device/platform verification is still incomplete.
