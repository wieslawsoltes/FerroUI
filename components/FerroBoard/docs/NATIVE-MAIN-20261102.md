# FerroBoard on current FerroUI main

The selected upstream revision is `8a35e0c5fe2214311e441845fef6e363b049d49e`,
observed on 2026-11-02, FerroUI `0.5.0-preview.1`. The isolated SDK is
`b00c16c91bfda60cb4bbfd8ed99a51fa2019120b` after the same five focused integration
fixes. They merge without runtime conflicts. Changelog conflicts retain upstream's
current text. A final upstream check found #497 (GPU pipeline prewarming),
so the pin includes that change too; all five fixes rebase without conflicts.
The shared FerroUI checkout and other ports are untouched.

See [exact revision, migrations and validation](native-upstream-main-20261102.json)
and [the separate SDK patch](../dist/ferroui-native-integration/README.md).
The broad CLR compatibility patch remains withdrawn.

## Native API migrations

- Popup alternatives use `PopupKeys` over native computed storage. `PopupKeysView`
  exposes checked Count/indexers to compiled and runtime XAML. Changes to an open
  popup's alternative count report a geometry error instead of indexing past bounds.
- Resource values use `Arc<RwLock<Vec<T>>>`. Clones retain actual shared storage;
  `push/clear` return unit, indices are usize, and remove returns the removed value.
  Defaults are fresh mutable buffers, without fixed-array flags, empty interning,
  cursor versions or managed enumeration. Library empty constants use immutable
  native slices rather than exposing a mutable buffer.
- Custom resource readers return `ResourceItems<T>`, a boxed fallible Rust iterator.
  Signed Count/index contracts remain only at the existing layout extension and
  markup boundaries. Null resource values remain distinct from missing resources.
  Iterator factory/item errors propagate. Cleanup runs through Drop on exhaustion,
  error or panic; fallible Dispose and null-cursor objects are intentionally retired.
- Resource maps retain IndexMap insertion order and actual shared map values.
  An iterator snapshots entries/keys/values when requested; a retained view still
  reads the current map when making a new snapshot. Removal preserves survivor order.
  Managed Current/MoveNext/Reset/Dispose and cursor-version APIs are removed.
- Collection metadata reads these native buffers/iterators directly. No generic CLR
  covariance, enumeration or collection metadata runtime is introduced.
- Control, adapter, sample and markup cleanup storage uses FerroUI's Subscription
  guards where appropriate. Source-lifetime timers/class handlers explicitly detach.
  Actual SDK trait/markup APIs that still return Rc<dyn IDisposable> are isolated
  adapters to those guards, rather than a second disposable runtime.
- Unused boxed-object equality helpers and ComponentName.deconstruct are removed.
  Callers use Rust equality, pointer identity and public fields. InputAttributes
  diagnostics use native Debug rather than generated CLR record formatting; its
  Equals/ToString markup conveniences retire. Value, clone, notification and input
  attribute tests retain their functional projections.
- The obsolete EmojiFontFallback.IsEnabled constant-false facade is removed.
  Native font loading/installation uses EmojiFont, Rust I/O and FerroUI font
  collections. Constants and the useful emoji classifier remain. Automatic network
  font downloads are not supplied; hosts inject data/loading.

## Behavioral evidence and intentional changes

The C# reference, bundled resource bytes and saved oracle fixtures are unchanged.
Collection runtime-only traces are not completion requirements. The current source
tests explicitly retain:

- 228 arrangement fixture outcomes for layout, serialization, null resources,
  count/index callbacks and factory failures; managed enumeration trace fields and
  seven cursor-specific mode groups are retired.
- 110 currency custom count/index/compute rows and 18 collection-value/mutation
  projections. Managed cursor states and fixed-array mutation restrictions retire.
- 214 popup API rows and two source declaration facts; 24 compute rows requiring
  null cursors or fallible Dispose retire. Native iterator error/unwind cleanup is
  tested separately.
- Dictionary lookup, null/present/missing values, custom dispatch, popup fallback,
  deduplication and engine precedence fixtures. Cursor/default-Current traces retire.
- Popup reentrant mutations are exercised with native snapshots. Source-compatible
  cases compare source outputs; other cases deliberately compute a stable snapshot
  rather than simulating CLR list invalidation or fixed-array mutation errors.

Native tests additionally cover shared handles across worker mutation, snapshots
surviving mutation, drop callbacks after locks are released, independent defaults,
checked metadata, open-popup growth errors and compiled/runtime popup bindings.

The exact test totals and commands belong to the JSON validation record. Earlier
native GUI checks have their original binary scope and do not validate a new binary.

## Remaining verification boundaries

Device-only touch/pen/multitouch, other operating systems, full accessibility,
large-dictionary/performance measurements and distribution signing/notarization
must be reported with actual evidence. They cannot be inferred from headless tests.
A-001 remains a deferred rendering anomaly. Do not resume its contrast investigation.

The legacy add-only SDK event metadata boundary still requires explicit scope
cleanup through the documented release functions when a host retires a Core
metadata event scope. Core domain events and normal Rust callbacks retain native
token/closure ownership. This narrow actual-framework limitation does not justify
a CLR event/delegate runtime.
