# Native suggestion result API

Suggestion results now use Rust ownership and iteration directly. The source
ISuggestionProvider and useful SuggestionList/markup names remain; no managed list,
array or cursor implementation is required by this result path.

| Previous port API | Native API |
| --- | --- |
| count returning signed Count | len/is_empty returning Result with usize/bool |
| get(i32) | get(usize) |
| from_array | from_nullable or iterator.collect() |
| original resource-list facade | from_shared/shared_values over Rc<RefCell<Vec<SuggestionValue>>> |
| from_custom(IResourceReadOnlyList) | from_custom(Rc<dyn SuggestionSource>) |
| GetEnumerator/MoveNext/Current/Reset/Dispose | iter(), Iterator::next and Drop |
| visit | a for loop over the fallible iterator |

SuggestionSource supplies len/get and can override iter for lazy fallible results.
Its default iterator captures the native length and calls indexed getters. Neither
shared storage nor dispatch holds a collection borrow across caller code.

Shared vectors retain actual allocation and candidate Rc identity. Indexed reads
see live mutations; vector iteration snapshots handles before callbacks. Custom
sources choose their iteration policy. Ordinary empty vectors have ordinary native
ownership, and do not reproduce global Array.Empty or shared cursor identities.
These lists do not generate collection-change notifications.

The native markup view preserves FerroBoard.Text.SuggestionList, Count:i32 and the
signed indexer. It rejects negative indices and counts larger than i32::MAX through
Result; it does not wrap values or invent fallback counts. Existing compiled/runtime
bindings and ItemsSource adapters still receive the actual candidates.

Provider partial-result failure isolation and reentrant attachment/settings behavior
are covered by projections of unchanged C# fixtures. Runtime collection facets are
accounted separately:120 historical collection rows plus six negative-count rows are
retired, while90 candidate/event/state outcomes remain compared. Six source view rows
remain; its negative-count row is retired. Dispose-error precedence is intentionally
replaced by Result and Rust Drop, with early-return/error/unwind cleanup tests.

The generic prediction/key/resource reader APIs and provider exception-marker bridge
remain separate cleanup work. No SDK, Cargo dependency, resource data or C# source
changes are required. This is a scoped migration, not complete port/platform parity.
[Exact source hashes and validation](native-suggestion-source.json).
