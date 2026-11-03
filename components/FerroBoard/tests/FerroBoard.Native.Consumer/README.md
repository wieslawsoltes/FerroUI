Current baseline:85 tests pass on FerroUI main b7537398 plus five focused native
fixes (a3af8388). NativeModels.xaml verifies ordinary model construction, bindings
and shared Rust setter identity in compiled/runtime XAML; model_errors.rs verifies
native diagnostics and custom keys without runtime-name mappings. See
[API migration](../../docs/NATIVE-MODEL-API.md) and
[exact checks](../../docs/native-main-b7537398.json).
Following entries retain their historical revision/test scopes.

Latest native popup/computed migration: 58 tests pass on the current integration SDK. PopupSnapshots.xaml, ComputedRows.xaml and ComputedSnapshot.xaml validate native map entries, shared rows, lazy query handles, explicit snapshots and actual key/popup identity. Complete metadata compiles on stock5cab, whose suite is 28/29 due to the independently reproduced native Option-marker SDK bug. Full stock controls/ItemsSource/native platform delivery remains incomplete. See [receipt](../../docs/native-popup-computed.json).

Latest native collection migration: 56 tests pass. KeyList.xaml, TextKeyList.xaml, PredictionList.xaml and SuggestionList.xaml validate compiled/runtime Count/indexer bindings and shared actual data. Generic System collection/iterator facade APIs are retired. Automatic ItemsSource conversion remains incomplete and must use a Controls-side stock IItemsList adapter; these tests do not claim it. See [receipt](../../docs/native-rust-collections.json).

Latest stock-API adaptation: 55 tests pass on the current integration SDK. `DictionaryModels.xaml` uses compiled/runtime bindings over actual native dictionary stores and checks owner release. A custom evaluator needs no PartialEq or SDK interface wrapper. Full metadata still fails on stock main at legacy collection references; these consumer results are not stock-control proof. See [receipt](../../docs/native-stock-metadata.json).

Latest lookup migration: 53 tests pass. `PopupLookup.xaml` validates native `PopupLabelLookup.Found` through compiled and runtime bindings; values retain original popup Rc identity. `TryGetValue` takes one key and returns typed Found/Value data, with no out-parameter service. These builds still use the current isolated SDK, not complete stock-main control integration. See [migration receipt](../../docs/native-rust-popup-lookup.json).

# Independent native FerroBoard consumer

This separate Cargo workspace depends on the public Core, metadata and controls
crates. It imports no sample/private-test implementation or historical consumer.
Its 123 locked packages retain the selected SDK and shared versions, checksums and
licenses. The runtime XAML loader is enabled without its Rust emitter feature.

From the repository root:

```sh
source scripts/env.sh
cargo test --locked --offline --manifest-path tests/FerroBoard.Native.Consumer/Cargo.toml
cargo clippy --locked --offline --manifest-path tests/FerroBoard.Native.Consumer/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path tests/FerroBoard.Native.Consumer/Cargo.toml -- --check
python3 scripts/check_native_consumer.py
```

Test functions cover compiled/runtime XAML scenarios, runtime error/retry,
native Rust emoji/key/language error APIs and native action metadata. They
exercise controls, bindings, attached input properties, enter handlers, custom
layouts/targets/providers/actions, focus/caret preservation and explicit cleanup.
Headless input is synthetic; these are not native OS/device/accessibility-client
or complete API proofs.

The compiled group contains eighteen XAML documents, including native geometry construction and bindings. Three form a nested relative
resource graph: Views/ResourceKeyboard.xaml includes ../Resources/Keyboard.xaml,
which includes Defaults.xaml. Native theme/settings objects are supplied through
StaticResource; the nearer dictionary shadows the imported theme. Four .fxaml
files under Runtime are embedded as assets and deliberately excluded from compiled
construction. FerroXamlLoader loads their native markup by URI after the runtime
loader is registered. The test asserts that each valid runtime graph document has
no compiled builder, so it cannot accidentally substitute compiled construction.
The invalid asset verifies a numeric-setting error with its literal and location;
a missing include remains an error and can be retried with a valid source.

Register the library/consumer types and generated compiled_markup table at the
composition root. For runtime-only assets, also register FerroRuntimeXamlLoader.
The selected framework's actual URI scheme is ferres://FerroBoard.Native.Consumer/;
relative includes resolve against the owning document, not the working directory.
Build::embed_assets uses package-relative Runtime paths. No custom reflection,
filesystem fallback or resource-loader emulation is added.

OnScreenKeyboard preserves the source settings-assignment contract: it copies the
assigned object's values into its engine-owned settings and restores Settings to
that owned instance. Changing the resource afterward does not change a keyboard
until assigned again. Use keyboard.settings() for live edits. The theme resource
remains shared by Arc identity; explicit apply_theme() observes field changes.
The dictionary's resource instances themselves remain shared and cached.

The native emoji integration test uses public shared vectors, stable owned
iteration, reentrant history callbacks and standard Error propagation. It neither
extends nor imports the legacy .NET collection/runtime APIs. Core stays independent
of controls, and application views contain no keyboard touch/focus/engine logic.

TouchConfiguration.xaml binds through the real OnScreenKeyboard.TouchProcessor
and SmartbarActions properties. Four two-way sliders configure cursor/deletion
step distances and popup geometry. Tests verify the same processor/vector identity,
actual gesture effects and release of the bound processor/engine on view disposal.
The Rust touch_processor() getter still returns Rc<KeyboardTouchProcessor>.
Its markup carrier, KeyboardTouchProcessorRef, is a small native Rc identity handle;
original() returns that same processor. Plain source configuration properties do
not acquire change notifications: two-way UI edits work, while programmatic edits
need an explicit binding/context refresh.

SmartbarActionRef similarly carries an existing Rust action for typed XAML data
contexts. ActionDetails.xaml verifies Unicode/nullable Id and Name bindings and
release on context replacement. The SmartbarActions metadata helper exposes Count
and the native Vec adder over the existing shared vector. The rendered-button test
populates that vector through metadata before mouse/Invoke activation. Callbacks
remain Rust closures supplied at the composition root; metadata does not construct
actions or serialize callbacks. Direct metadata tests check original Rc identity,
null entries, callback results, invalid UTF16 errors and cleanup.

The language-error integration target uses real failing dictionary/glide/factory calls
through ordinary Rust `?`, borrowed typed error chains, cached Arc identity and
original factory panic payloads. It adds no runtime emulation or dependency.

Source exports relocate every native workspace and its unchanged lock. Exact
scope, denied-source execution and fresh-export evidence are recorded in
[the resource-consumer guide](../../docs/NATIVE-CONSUMER-RESOURCES.md). Broad markup,
external image/font/other resource forms, arbitrary extensions, native platforms,
physical devices, assistive clients, performance and release delivery remain open.
Apache-2.0 and the repository LICENSE, NOTICE and THIRD-PARTY-NOTICES.md apply.

## Stateful custom composer

The composer_extension integration test implements IComposer in this downstream
crate using Rc/Cell state. It registers through public KeyboardResources, drives
the real engine with UTF16 context, changes state, replaces the registry instance
and verifies release. The public Arc<dyn IComposer> API does not impose Send/Sync;
the test documents that choice with a scoped constructor lint exception. Run from the repository root:

```sh
source scripts/env.sh
cargo test --locked --offline --manifest-path tests/FerroBoard.Native.Consumer/Cargo.toml --test composer_extension
```

This does not establish native IME or shaping support.

## Layout errors and lazy I/O

The layout_errors integration tests use standard `?` and Error downcasting for
layout parsing/serialization. A private file provider first returns PermissionDenied,
then a valid arrangement; the test checks typed causes, lazy retry and cached
layout identity. No external files or permissions are used. Run from the repository root:

```sh
source scripts/env.sh
cargo test --locked --offline --manifest-path tests/FerroBoard.Native.Consumer/Cargo.toml --test layout_errors
```

The Geometry.xaml scenario constructs KeyRect values and shared KeyboardGeometryOptions through compiled and runtime XAML. It uses native property setters and Rust Clone/Hash/equality; generated record methods, boxing APIs and exact CLR hash values are not part of this boundary.

Suggestion.xaml constructs native candidates with the original six XAML property names. Compiled and runtime tests check UTF16/UTF8 text, scalar bindings, shared Rc identity, independent Clone values, explicit binding refresh and release on context removal. Candidates do not gain automatic notifications. Rust uses set_* and values(); CLR record printing, boxed equality and by-ref outputs are removed. XAML constructors have explicit 0/2/4 argument forms rather than runtime optional-argument emulation.

Request.xaml constructs SuggestionRequest with the six original property names. Word and PrecedingText bridge UTF16 to native String fallibly; Culture is now a locale-name String rather than a virtual CultureInfo object. An invalid locale fails without replacing the existing value. KeyProximity uses KeyProximityRef, an opaque handle to the actual Rc<dyn Fn(u16,u16)->f64>; supply closures in Rust. No System.Func, method-delegate conversion or callback-interning registry is exported. The request Clone shares storage; from_values(request.values()) creates an independent field snapshot. Core now carries immutable TextCulture profile values; its virtual CultureInfo/TextInfo/CompareInfo objects and registries are removed.

Compiled/runtime bindings check request values, locale names, shared identity, callback invocation and release. The request_native target checks callback ownership, errors, capture Drop reentrancy and unchanged panic payloads. No desktop/native input proof is implied by these metadata tests.

LocaleModels.xaml constructs evaluator/context objects with native locale-name properties and compiled bindings. Both compiled and runtime construction preserve evaluator interface dispatch, context copying, Unshifted behavior and release. Invalid locale names fail before mutation. Generated record methods, by-ref outputs, CLR culture-object projections and custom GC/weak projections are no longer part of this boundary. Core uses TextCulture; the existing FerroUI 0.4 CultureInfo API is used only to read the host locale name at the composition boundary. Other inherited compatibility surfaces remain pending migration.


Utf16Buffer.xaml validates native String text properties through both loaders,
including Unicode and compiled bindings. Core owns the exact UTF16 buffer; a
malformed value remains readable through Rust but its UTF8 metadata getter fails.
No System.String alias, interning service or lossy buffer Display is installed.


Callbacks.xaml binds the actual engine Clock and SuggestionProviderFactory
properties. Rust supplies the callbacks; the consumer checks native Rc identity,
provider dispatch, the attached editor's clock snapshot and capture cleanup.
No System.Func markup service or method-to-delegate adapter is required. Clock
values still use the legacy Core timing representation pending its own migration.


Subtypes.xaml constructs native subtype and layout-map objects, uses nested
compiled bindings and shared defaults/constants, and checks Arc ownership and
shallow copies. Native fields refresh when the data context is refreshed; these
configuration models do not promise per-field change notifications. No boxed
record views, global object cache or custom weak/GC projection is installed.


CodePoints.xaml checks native buffer length/indexer bindings and shared mutation.
The native_core_traits test supplies custom providers/lists without PartialEq or
SDK AnyValue, checking actual Rc identity and release. Prediction metadata uses
FerroBoard.Text names; UTF8 display boundaries remain fallible. This workspace
still selects the current isolated SDK. The complete stock Core suite is a
separate command: source scripts/env.sh; python3 scripts/check_stock_core.py.
