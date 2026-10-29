# Shared typed numeric enum construction

Four existing published 0.3.0 copies now expose an optional Int32 enum factory through
the native compiler metadata. Base carries the associated typed function name only with
compiler metadata; Build.Scan exports it as an optional format-2 field. Build's model host
and the loader's runtime host pass that exact declaration to the same Rust emitter.
Known constants keep their original paths. Unnamed values use a declared `fn(i32) -> value`
on the public metadata carrier, with no inferred constructor, name or false flags marking.
The Core SDK generates this function over the actual open window enum types.

The original XamlX frontend narrows oversized numeric XAML constants into Int32. Original
compiled C# XAML and native compiled consumers agree for positive/negative overflow, signed
endpoints, ordinary unnamed values, nullable settings and original window options/parents.
This differs from the separately tested JSON range behavior. Original factory/XAML reference
checks expand900->908 (seven enum cases plus boxed-scalar namescope identity).

The complete patches for Base, Build.Scan, Build and Loader replay against the exact cached
published archives and reproduce current bytes. Licenses, VCS records and original hashes
stay retained; versions, dependency lists and feature defaults do not change. Existing Base
and Loader namescope tests are corrected to assert the previously ported object registration
contract (including boxed scalars) and original identity/null/duplicate/completed errors.
No new namescope production behavior is introduced by that test correction.

In owned SSD copies, all four full library suites pass4,854 tests, with one pre-existing
ignored well-known-type diagnostic in Loader. Published archives omit font/theme fixtures;
those assets are copied from the read-only sibling into test scratch, recorded by hash and
never added to shipped packages. Registry/sibling source and actual desktop remain unchanged.
The standalone test workspace avoids Cargo's host-only build-dependency package-test panic.
This is explicit SDK construction, not automatic transparent-enum compiler provenance,
remaining enum migration or full default/application model catalogs.

## Previous validated dependency checkpoint

# Native nullable collection items

The sixth isolated package is `ferroui-controls` 0.3.0. Its generic item boxing
previously exposed a Rust `Option<T>` wrapper to native templates and event consumers.
`box_item` and `unbox_item` now use the existing Base metadata conversion, so a nullable
source reference becomes actual native null or its original contents. Typed equality
retains its direct, allocation-free comparison before trying the nullable conversion.
The same shared implementation serves collection reads, change views and native lists;
there is no Dock-specific type branch, alternate storage or filtered null slot.
Registered reference handles also use Base's native object form. Consumers recover their
Rc through `from_markup_value`; the same original allocation/reference is retained.
Selector integration checks exact original Rc identity for selection and replacement,
rather than requiring a particular internal transport box.

The retained published archive, VCS identity, all original file hashes, the one changed
file (`items_source.rs`) and its exact patch are recorded in
`vendor/ferroui-controls-provenance.json`. Two dependency regression tests cover nullable
boxing/unboxing/equality and borrowed change views. FerroDock's native collection and
application consumers additionally exercise original model identity, null event payloads
and indices. All 4,428 retained controls tests and the 1,675-test workspace pass; see VALIDATION.md. The
registry package, sibling checkout and actual desktop executable remain unchanged.

## Previous validated dependency checkpoint

# Core-owned capability and window metadata

Capability policy contracts/classes and IDockWindow descriptors now live in the Core
SDK. UI retains aliases but no longer owns their metadata or public compiler paths.
Core registers original policy nullable/reference handles and identity-preserving
casts independently. Window notification metadata retains the actual optional source
INotifyPropertyChanged contract. Core/runtime retain no native controls dependency.

Original DockWindowOptions, DockCapabilityEvaluation and DockCapabilityResolver now
have native metadata over existing engine types. Runtime constructor compilation
records actual public Rc<DockWindowOptions>/Rc<DockCapabilityEvaluation> value paths;
no dummy carrier object, copied state or replacement resolver is introduced. The
23rd Core enum descriptor reads DockCapabilityValueSource's actual declaration.

The inspector migration qualifies only IDockWindow through FerroDock.Model. Its
source-comparison verifier still rejects unrelated markup changes and now additionally
rejects restoring the old assembly owner. Two verifier regressions accompany the change.
Original source and sibling FerroUI remain unchanged, and no new dependencies/vendor
edits or desktop rebuild/restart are involved. Full serialization, Core ownership,
generic/optional metadata, CLR and platform/lifetime scope remain incomplete.

## Previous factory API checkpoint

# Factory operations and deferred query views

The Core SDK extends its existing source factory descriptors with original mutation,
activation, window, split and search APIs. Predicate views retain actual Rc callbacks;
checked MarkupDelegate adapters use the shared ValueTypes cast path. Query and cursor
views adapt the original DockableQuery and its own iterator without a second traversal
engine or snapshots. IEnumerable/ IEnumerator metadata retains interface kind and the
cursor also exposes the existing native IDisposable contract. General CLR enumeration/
GC/type-system behavior remains incomplete.

The independent consumer compiles source-style Func predicates and a Core IFactory
ActivateWindow command binding. Its actual engine graph verifies tracking and exact
returned model instances. The unchanged C# reference covers equivalent predicates,
query ordering/deferred execution, cursor fault/Current/Reset/disposal, mutations,
pinned ordering and window ownership/tracking. An accidental self-containing window
fixture failed recursively in both engines and was corrected to separate owner and
floating roots; source behavior was preserved.

No new dependencies or vendor changes are introduced. Existing five isolated native
patches retain their exact provenance. Domain algorithms remain unchanged, Core/runtime
retain no native controls normal dependency, and general Dock controls remain separate
from the concrete backend SDK. Remaining generic/optional method metadata, complete
collections/Core logical ownership and native platform parity remain in the full goal.

## Previous locator checkpoint

# Core factory locators and shared scanner cast facts

The Core SDK now owns IFactory and FactoryExtensions metadata under FerroDock.Model.
Controls retain aliases and register Core first. Core views use the actual source
locator functions, mutable dictionaries and model interfaces; no controls or concrete
backend normal dependency is introduced. Nullable locator entries preserve original
null/missing behavior. Tracking/lookup and source-style method declarations use the
existing factory and shared object bridge.

The fifth isolated published dependency is ferroui-build-scan 0.3.0. Runtime already
registered checked boxed-cast pairs, but its source scanner omitted those facts and
rejected original delegate assignments. Two changed files recognize boxed_cast as an
existing cast pair and normalize counts through ordinary, declaration-token and macro
scanning. Runtime predicates remain authoritative. Two scanner regressions pass;
original archive hashes, exact changed-file hashes, patch, license and VCS metadata are
retained and checked by check_framework_patch.py. The sibling FerroUI checkout remains
untouched. The previous four retained dependency copies are unchanged in this pass.

Concrete backend projection registrations now expose closed scanner-readable types;
the generic runtime projection algorithm is unchanged. Core/runtime remain independent
of controls, general controls remain independent of the concrete backend/SDK, and the
Domain model remains independent of native base/controls. Full collection, delegate,
Core contract and CLR/platform scope remain open.

## Previous event checkpoint

# Shared runtime object bridge and factory events

ferrodock-runtime extracts the existing UI-independent object/native conversion
bridge and depends only on native base services. Recycling retains its public helper
aliases and raw Control projection. Both recycling and the Core metadata SDK depend
on this shared support crate; they use one representation and one type registry.
No source model or docking algorithm moves into the runtime library.

Core metadata now owns all event-argument/tracking snapshot descriptors and 22 model
enum/flags descriptors. FactoryBase and the current UI-owned IFactory descriptor
use the same Core adapters for the 28 original events. Core retains no controls or
concrete backend normal dependency. The backend SDK remains separate from general
controls, and Domain remains independent of native base/controls. CLR EventArgs base
casts and full Core interface/POCO logical ownership remain partial.

No vendor or sibling FerroUI changes were needed. The source and live desktop
sample/bundle/layout are preserved. See VALIDATION.md for current checks.

## Previous constructor checkpoint

# Factory/window metadata dependencies

The ReactiveUI integration SDK owns the original Factory and DockWindow class
constructors and their source inheritance. The shared Core SDK owns FactoryBase
and registry-view descriptors. Core now uses native System services to forward
notifications from the existing model lists; it still has no controls dependency.
FactoryBase metadata remains incomplete for generic methods, locators, events and
the remaining inherited operations. Registry wrappers expose the same dictionaries,
not copied data or a replacement collection implementation.

The general control library retains no normal concrete ReactiveUI backend/SDK
dependency. Its notification-bearing interface descriptors now explicitly register
the public Rust paths of the actual runtime descriptor copies. Both compiler hosts
use the same existing path metadata and conversions. Domain only adds hidden static
enclosing-reference projections on factory/window contracts and remains independent
of native UI metadata. No additional vendor source changes were needed.

## Previous Controls checkpoint

# Model value metadata and the remaining ReactiveUI control classes

The new `ferrodock-model-ferro` SDK owns the existing 19 source model enum/flags
descriptors moved from settings, plus SplitViewDisplayMode and SplitViewPanePlacement.
Its logical assembly is `FerroDock.Model`. It depends on native base/System and the unchanged
domain value types; it has no control-library dependency. Declaration-driven build
generation, typed original values and converter behavior are retained. Settings
keeps public helper aliases to the same descriptors, registers the model SDK first
and exports only its own source types. XAML consumers declare the model SDK directly
so emitted public type paths are ordinary visible Rust dependencies. Domain retains
its UI boundary; only two additional build-only enum declaration constants were added.

The ReactiveUI integration SDK now also constructs ToolDock, ProportionalDock, StackDock,
GridDock, DockDock, WrapDock, UniformGridDock, SplitViewDock, ProportionalDockSplitter
and GridDockSplitter. All 14 original classes in the backend Controls directory have
constructor/class metadata. Each new class view retains its original model and source
base, notifications, original properties/methods and interface identity. Generic
contract casts/nullability remain backend-independent facts; no surrogate state,
engine or duplicate class descriptors are introduced. Remaining FactoryBase metadata, full Core
contract ownership and the broader constructor/type-system scope remain incomplete.

Actual nested SplitView pane/content assignment reproduced an invalid generated
`as Rc<dyn IDockableMetadata>` cast from a plain model class view. The shared compiler
now distinguishes actual declaring trait handles from separate foreign descriptor
carriers. It emits a direct Rc unsizing coercion only for the actual shared
source handle or constructor value and a target that really owns that trait handle. Class views/foreign
contracts take the existing registered cast path, matching runtime loading. A second
fixture assigns actual shared capability overrides to required/nullable original
policy interfaces, and confirms native SetterBase still describes its own trait.
Both compiler hosts and executable generated assets are checked. These changes stay
in the existing five loader and one build files with refreshed exact provenance.

No original Dock or sibling FerroUI files are modified. The live sample/bundle is
preserved. After build storage pressure, obsolete test executables and older duplicate
base/control archives were removed only after the active build had terminated; the
record is /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-cache-cleanup.json. No source or app bundle was removed.

## Prior ReactiveUI constructor checkpoint

# ReactiveUI metadata SDK and public constructor return signatures

`ferrodock-model-reactiveui-ferro` now owns the eight model class-reference
descriptors formerly in the general control library. It composes the original
ReactiveUI model backend and native metadata services without a dependency on the
general control library. The general controls also retain no normal dependency on
this SDK/backend; the SDK is only a dev dependency for their identity converter test.
Domain models remain UI independent. Applications explicitly register controls and
SDK; the sample and downstream consumer have been migrated.

The SDK registers one logical `FerroDock.Model.ReactiveUI` assembly and the original
Document, Tool, DocumentDock and RootDock constructors. Each creates the existing
concrete model and returns its identity-preserving class view. Abstract source bases
have no constructors. Existing class inheritance, notifications, mutable engine
state, original factory operations and custom sample subclasses are retained.
No duplicate metadata descriptors or UI replacement models are introduced.

The published `ferroui-build` 0.3.0 package additionally needs one isolated source
change: the model compiler must render root signatures through the declaration's
public export path instead of its private canonical module. Canonical keys used
for equality/conversion remain unchanged. Complete type components, including
generic arguments, resolve to their actual public declarations; metadata carriers
cannot rename a different returned model. This makes the new four consumer assets
compile without exposing their private modules. The older external-carrier regression
fixtures still run against the same compiler. Four retained packages now have exact
source/modified/patch hashes, original licenses, notices and VCS metadata. The shared
guard includes the build package. No sibling checkout or original source is edited.

Remaining scope includes the other model constructors, full logical ownership of
external Core/ReactiveUI descriptors, all x:Class/generic/cross-package metadata,
CLR identity/string/GC/lifetime/platform behavior, and the wider port inventory.

## Prior constructor checkpoint

# External constructor return types and compiler owner metadata

Core policy/override roots reproduced invalid generated types from the previous carrier
name substitution. The model emitter now uses the declared/import-resolved value type.
The runtime emitter first uses the existing public type-path registry; UI-owned Core
registration declares exact return paths. Its fallback uses the actual Rust declaration
owner, recorded by ferro_markup_type only with compiler metadata, with complete-component
substitution. An unknown external return path is refused, never inferred from the carrier.
No side registry, reflection, UI replacement model or production runtime compiler is added.

Published base changes now cover 12 files, markup 3, compiler 5; exact provenance and
licenses/notices/VCS data remain. Both actual roots drive the original resolver through
normal Load and initialized layouts. All concrete model constructor exports, logical
assembly ownership and remaining generic/CLR/GC/platform contracts remain further work.
The original source and sibling checkout remain untouched.

The following sections retain previous dependency checkpoints.

# Plain-root compiler and enclosing-object identity support

The existing ValueTypes registry now supplies explicit typed identity accessors and
reference comparison. Rc<T> reference registration supplies its enclosing pointer;
UI-owned metadata adds interface/class views without changing Domain dependencies.
Namescopes and the reference converter share that metadata; accessors retain no model
instances and run outside borrows. No per-object side registry is used.

The existing published compiler patch now changes five files. Plain declared roots use
the constructor's value type, one materialized instance, the original context and scope
completion, and actual-value group handles. Build-time and runtime emitter hosts are
checked. The native asset loader and Load extension remain in use. Descriptor/public-path
metadata, all x:Class/name/generic/cross-package/CLR/GC variants remain partial.

The downstream test consumer adds the published runtime compiler as a dev dependency,
activating compiler metadata for actual runtime-host emission checks. This revealed three
short theme class paths, now crate-qualified in Fluent, Simple and Browser. Production
crates gain no runtime compiler. Original/sibling source is untouched. Exact provenance
still covers base 10 modified files, markup 3, compiler 5; see vendor manifests/patches.

The following sections retain earlier dependency checkpoints.

# Factory helpers and markup extension library

FactoryExtensions remains in the model crate and uses only the existing original
contracts and query machinery. Its native metadata carrier is UI-owned. The returned
host handle uses the shared nullable registration, without a UI dependency in Domain.
The new ferrodock-markup-extension package depends only on existing base and markup
packages; its standard build scanner export is consumed through Cargo links metadata.
It asks the genuine service provider for URI context/name scope and delegates to the
native loader/value conversion. No runtime parser or alternate name registry is added.
The shared namescope now accepts arbitrary object values and actual plain model
handles. Existing native methods project from the same map and asynchronous searches.
Reference, ResolveByName and compiled #name bindings use that shared scope. Names,
duplicate identity, completion and child-scope shadowing retain the original behavior.
The compiled consumer names an application object backed by the actual Document model,
resolves it through Reference, and follows title changes from the engine. It is not a
native control, and its fixture does not replace a docking model.

The isolated published base package now declares ten modified files (four previous
resource/cast files, six namescope/binding/metadata files); the markup package declares
three (existing template fix plus scope runtime and ResolveByName); a published compiler
package declares two (emitter and runtime AST registration). Exact manifests, patches,
original licenses/VCS metadata and the guard cover all three. Plain values share native
scope storage; there is no alternate registry. Native-only custom scopes retain their
old methods and explicitly reject unsupported plain registration. The original weak
attached-owner rule remains. Standalone plain model roots and arbitrary CLR/GC identity
representations are still unimplemented. The sibling FerroUI checkout is untouched.

The next sections retain the earlier inspector and theme dependency history.

# Main inspector and conditional template matching

RootDockDebug uses the existing native UserControl, compiled templates/bindings,
observable collections, ReactiveUI, routed input, Window and adorner APIs. Presentation
is UI independent and an injected clock is resolved in the library composition root.
The real DEBUG desktop sample attaches both F11 and F9; views remain passive.

Desktop validation exposed native DataTemplate matching accepting a registered
conditional IDockable-to-IDock cast without testing the object. The isolated published
ferroui-markup-xaml 0.3.0 dependency changes only templates/data_template.rs to require
a successful actual cast before accepting assignability. The existing native matching,
building and selection engines remain in use. Exact original/modified file hashes,
registry VCS metadata and the one-file patch are committed under vendor; the framework
integrity guard verifies both packages. Its existing four-file base patch is unchanged.
No sibling source is edited. See vendor/ferroui-markup-xaml-patch.md.

The grip correction changes only the linked Fluent/Simple header column allocation.
The original C# template reproduces the same narrow grip; no renderer patch is involved.
The following dependency notes describe earlier checkpoints.

# Original debug overlay dependencies and metadata

The diagnostics library now also uses the existing settings and System packages for
original area flags and the .NET-order Dictionary. Its normal build script delegates
to the shared metadata scanner, exporting the original static extension class and both
overloads. Native Control/AdornerLayer, event routing, visual hit testing, object display,
FormattedText and drawing APIs carry all implemented behavior. There is no new framework
patch, dispatcher or renderer. The original sample has the diagnostics dependency and
attaches its source F9 hook at DEBUG desktop startup; no view code-behind logic is added.

Native callback undo tokens preserve source subscription semantics without retaining
publishers beyond the source contract. Routed RemoveHandler removes every equal
callback; CLR event -= removes its last match. Source callback owners remain strong.
The source's explicit ActionDisposable repeats its stored action. Arbitrary unregistered
CLR display names, current-culture object display, full delegate/type metadata and
lifetime/platform scope remain partial. F11 inspector consumers remain pending.

# Diagnostics selection dependencies

The new normal workspace crate ferrodock-ferro-diagnostics depends on the existing
UI/model packages and shared native base/controls. Its actual native selection class
and helper use the existing adorner layer, logical parent, coordinate transform,
DrawingContext and IDisposable subscription APIs. No new framework patch or sibling
source edit is required. The source inspector consumer is still pending; internal
modules carry narrowly scoped dead-code allowances while that consumer is ported.

The model-neutral IDockableControl marker now has the same hidden boxed-reference
projection pattern already used by other model contracts. Plain implementations keep
their original Rc; the canonical native adapter exposes its exact native Ref. A
borrowed-type probe at the UI boundary delegates to native runtime object casts,
including base/root handles and custom controls. It holds no replacement model/control
or extra lookup registry and adds no UI dependency to the model.

The independent reference links the unchanged original helper/adorner and actual
original Dock native/model projects. Its local SourceLink build requires the configured
NuGetPackageRoot to have a trailing slash; this is supplied as a global command-line
property, without changing original source or build files. Existing source dependency
and native compiler/linker warnings are recorded in validation.

# Browser overlay dependencies and native construction

The Browser library preserves the original dependency on Dock Fluent and compiles all
five original class/resource documents. Its platform-specific source values use the
EXISTING published OnPlatformExtensionOf<f64>/OnPlatformExtensionOf<Thickness> types.
UI-owned closed metadata declarations expose their original options and delegate to
those actual types; no replacement platform rules/parser/storage or additional native
source patch is used. Complete generic SDK metadata beyond the required source
operations remains pending.

DelayedUniformTabPanel uses the shared signed animation TimeSpan, DispatcherTimer,
DoubleTransition and CubicEaseOut. Constructor work uses the genuine inherited
FerroObjectImpl::constructed hook, as does source resource population for Fluent,
Simple and Browser. This preserves derived native constructor behavior and consumes
the optional service provider after population. Source strong timer captures retain
native Rc cycles; CLR collection equivalence and broader animation/DPI/platform cases
remain unverified. The source-only layout reference links the actual panel/helper.

Debug/test incremental caches are disabled after generated caches exhausted the SSD;
normal assertions, optimization and debug-symbol settings remain unchanged.

# Original Simple theme dependencies and linked resources

The Simple library uses the existing model/native control/shared compiler packages.
Its NORMAL locked graph includes neither FerroDock Fluent nor FerroUI Fluent.
Published ferroui-themes-simple =0.3.0 is pinned alongside all existing 0.3.0 packages
and used for Headless validation. The original sample's Fluent configuration is intact.

The original C# Simple project links the entire Fluent Controls tree and five images.
The Rust library carries self-contained copies of those already-translated shared
inputs so compilation/packaging needs no neighboring theme source checkout. The
assembly-local image URI owner changes to Simple; every other shared byte must match.
Run python3 scripts/dock/check_theme_links.py; its tests reject missing/extra/altered inputs
and the wrong URI owner. Eight own original Simple documents define its root, accent,
compact dictionary and five presets. The root uses exactly the original 42 merged
includes, while all 56 source class/resource documents compile in the assembly.
No additional native framework patch was needed.

# Current enum and collection compiler boundaries

DockingWindowState metadata is generated in the UI settings crate from the model's
own exact bitflags declaration, following the existing foreign-enum TypeModel bridge.
All values retain the original domain type. Shared metadata drives text conversion;
there is no custom enum parser. The native typed flags emitter's flags_compose rule
still limits emitted literals to combinations of declared members, while runtime
integer metadata preserves every original signed i32 bit pattern. Complete System.Enum
and compiled unknown-bit values remain pending.

The source-only proportional-view reference establishes an original null-ItemsSource
construction failure. Native compilation rejects the source IList.Add operation
before runtime (FRN3000); that interface/emitter support is still required. No new
framework patch or modified collection is introduced to bypass either boundary.
The existing isolated base support patch below remains unchanged and verified.

# Current native support and desktop dependencies

The sample now uses published `ferroui-desktop`, `ferroui-fonts-inter`, native Fluent
and ReactiveUI control integration at `=0.3.0`, with the original Inter registration.
Desktop is the sample's default feature; `--no-default-features` keeps its library and
Headless usage independent of desktop bootstrap.

`[patch.crates-io]` selects `vendor/ferroui-base`, an exact published 0.3.0 package
copy except FOUR declared source changes. All original file hashes, package Git
revision and modified hashes are in `vendor/ferroui-base-provenance.json`; the reviewable
diff is `vendor/ferroui-base-resource-api.patch`. Run
`python3 scripts/dock/check_framework_patch.py` to reject undeclared changes, missing files
or extra files. The package is excluded from workspace formatting. Original notices
and MIT licensing are retained. The dirty sibling FerroUI checkout and Cargo registry
sources remain untouched.

The resource changes add safe Any recovery for plain IResourceProvider values,
optional dictionary-interface projection and the existing mutable merged-provider list
through IResourceDictionary. Dock's original recursive preset matching/removal therefore
works with custom dictionaries too, without a side registry or replacement resource
store. The fourth file routes existing assignability through ValueTypes::try_convert
when there is no direct registered converter, including object-form DataTemplate to
nullable IDataTemplate. Registered conversion precedence and one-call failure remain.
This fixes actual tab icon presenters without narrowing Dock's object-valued API.

The copied package currently emits 23 preexisting compiler warnings, now visible
because it is a local dependency. They are not suppressed or fixed outside the declared
patch. Strict checks apply to the FerroDock workspace. Native callback/reentry,
collection enumeration, general runtime typing and CLR lifetime parity remain open.

# Shared dependencies

The compile-time JSON generator semantic library has no production dependencies. Its
tests use the existing locked serde_json to read original compiler snapshots. It does
not pull the runtime serializer, models or FerroUI into its normal dependency graph.
The .NET reference probe uses original compiler/generator projects and cached Roslyn5.0.0;
those are reference-tool dependencies, not Rust runtime dependencies. Rust syntax/shared
metadata adaptation and executable codec emission remain pending.

All FerroUI crates use exactly `0.3.0`; Cargo.lock records transitive versions.
The base package is supplied by the isolated support patch described below.
Use the existing ports instead of recreating .NET/reactive primitives.

The recycling-model contracts have no external dependencies. They use
`Rc<dyn Any>` without depending on the UI base crate. The UI Base property
BoxedValue instead uses `Rc<dyn AnyValue>` with equality; the native recycling
library owns this conversion boundary. The docking model references these contracts for
IDockable's inherited IControlRecyclingIdProvider contract.

Geometry Display uses the safe stack-buffer API of `ryu` (locked at 1.0.23), then
applies .NET's invariant general-format exponent thresholds and spelling.
Rust's standard formatter rounds some shortest-decimal halfway cases differently;
7,895 reference outputs from the actual DockPoint/DockRect sources cover that
boundary. No number-conversion algorithm or third-party source was copied.

The JSON scalar metadata also uses that same locked Ryū version for Single/Double
shortest digits before source general-format spelling. Its explicit dependency edge adds
no new version or feature; the model already depends on Ryū. JsonNumberReference.cs records
1,362 numeric cases for the actual source serializer. No additional framework package
patch is needed. The normal JSON dependency boundary remains independent of UI/ReactiveUI;
the actual-model fixture dependency is dev-only.

| C# dependency | FerroUI counterpart | Use in FerroDock |
|---|---|---|
| `IDisposable` | `ferroui_system::IDisposable` | Busy scope handles; explicit disposal, without implicit disposal on drop |
| `INotifyPropertyChanged` | `ferroui_system::component_model::INotifyPropertyChanged` | Model service contracts, implemented through ReactiveUI |
| `EventHandler`, event lists | `ferroui_system::events::{Event, HandlerList}` | ReactiveUI notifications and factory event handler contracts; FactoryBase lifecycle events reuse the shared publisher |
| `ObservableCollection<T>` | `ferroui_system::collections::object_model::ObservableCollection<T>` | Observable backend of the general ListHandle/IList contract used by concrete models, factory contracts, navigation and group validation |
| Insertion-ordered dictionaries | `ferroui_system::collections::generic::Dictionary` | Shared dictionary/locator handles in IFactory; FactoryBase locators retain shared mutation |
| `System.Windows.Input.ICommand` | `ferroui_reactiveui::ICommand` | Shared command contract in IDock and other model interfaces; bridge to control commands remains in the UI layer |
| `Task<T>`, `TaskCompletionSource<T>` | `ferroui_system::threading::tasks` | Available for dialogs and dispatchers; task continuation policy needs attention below |
| `CancellationToken` | `ferroui_system::threading::CancellationToken` | Available for asynchronous service lifecycles |
| `SynchronizationContext` | `ferroui_system::threading::SynchronizationContext` | Busy service notification dispatch, using `post_local` for thread-affine callbacks |
| Rx `IScheduler`, scheduling | `ferroui_reactive::concurrency::{IScheduler, SchedulerExt}` | ServiceDispatcher fallback scheduling |
| `ReactiveObject`, notifying setters | `ferroui_reactiveui::{ReactiveObject, reactive_object, IReactiveObjectExt}` | ReactiveBase/DockableBase/DockBase, concrete models and DockGlobalBusyService |
| `RxSchedulers` | `ferroui_reactiveui::RxSchedulers` | Main-thread scheduler fallback |
| Reactive commands, interactions, routing | `ferroui_reactiveui` | DockBase navigation/close, RootDock window commands, DocumentDock creation and MDI commands |
| ReactiveUI test schedulers | `ferroui-reactive-testing`, `ferroui-reactiveui-testing` | Available for remaining Rx/ViewModel test ports |
| Framework integration | `ferroui-reactiveui-controls` | Keep in the UI integration layer; domain/model crates remain independent |

`ObservableCollection` is a shared handle with collection identity and observable
mutation. Returning or cloning that handle preserves live collection semantics;
returning a copied `Vec` would not. Never keep a collection/RefCell borrow across
notifications or user callbacks.

FerroUI's reactive objects, collections, tasks and disposables are `Rc`-based and
thread-affine. Its synchronization contexts are `Arc`-based and have both `Send`
and local posting entry points. The global busy service follows that same thread
model; its C# interlocked counter becomes a `Cell<i32>` with the same wrapping
arithmetic, and callbacks are posted locally. Native cross-thread calls to the
C# service are not available through this Rust API. See the recorded adaptation
in PORTING.md; cross-thread entry points remain an open parity question.

`TaskCompletionSource` in FerroUI 0.3.0 runs registered continuations synchronously.
The C# ServiceDispatcher's InvokeAsync explicitly requests asynchronous
continuations. That member is currently **absent**, recorded as deferred, rather
than silently changing its reentrancy contract. Determine the appropriate shared
System task support before porting it and MainThreadDispatcher.

Sources inspected: the published `0.3.0` crate sources and
[FerroUI.System](https://github.com/wieslawsoltes/FerroUI/tree/main/src/FerroUI.System),
[MVVM guide](https://github.com/wieslawsoltes/FerroUI/blob/98e091f119e203169f64ae4331fe66bbdccdcbac/docs/mvvm.md),
[reactive porting decisions](https://github.com/wieslawsoltes/FerroUI/blob/98e091f119e203169f64ae4331fe66bbdccdcbac/docs/porting/reactive.md).

The reactive model base uses the existing `raise_and_set_if_changed`, `changed`
and `when_any_value` APIs. It supplies the enclosing concrete model as the
notification sender, preserving compiled reactive observation through composition.
The source-generated float setters need an explicit NaN/signed-zero equality
check because Rust float PartialEq differs from .NET EqualityComparer<double>.
No replacement reactive implementation or UI framework dependency was added.

DockBase retains the source WhenAnyValue subscriptions for active/focused state.
Weak callback captures and disposal on owner drop avoid native ownership cycles.
Its object-valued Navigate command wraps a typed ReactiveCommand, translating
borrowed String/erased dockable/BoxedValue inputs and preserving ReactiveUI error
routing; it does not reimplement command scheduling or execution state.

Every concrete reactive model forwards AsRef<ReactiveObject> through the same
base instance. Composition constructors plus the object-safe callback bridge keep
the outer subclass as the sender and virtual callback target. Reactive setters in
embedded leaf components use that outer identity as well.

DockWindow reuses ReactiveBase and shared notifying setters. HostAdapter uses
standard Rc/Weak ownership and Rust unwind handling for the source catch/finally
path; no platform or UI control dependency was added. Standalone adapters retain
models, while model-owned adapters use explicit weak construction. Layout/host
handles remain owned by the window; parent/owner/factory backlinks are weak.

IHostWindow exposes an optional borrowed close-state hook, and IFactory exposes
an optional full-window host-factory hook. These are explicit native interface
queries, with no reflection. A reentrant late-null SetLayout call still needs a
nullable boundary that preserves the source's live lookup semantics.

FerroUI System 0.3.0 exposes ObservableCollection and Dictionary but no generic
IList contract. FerroDock's ListHandle/IList boundary therefore accepts plain and
custom backends while adapting the published observable collection directly.
It does not replace that collection's notification or reentrancy implementation.
The observable adapter never subscribes to changes itself: even an internal
version-counter subscription would change its subscriber-count reentrancy rule.
Mutation-version access for this backend remains a dependency parity gap.
Plain lists have their own version counter and custom comparers; custom lists
retain their own read-only, notification and enumeration behavior.

FactoryBase binds typed weak CreateList thunks at construction for the source
dockable-normalization and window-registration use sites. This retains concrete
generic overrides after the main factory handle is erased; general public generic dispatch remains pending.
DockableQuery models repeatable IEnumerable semantics instead of a single consumed
iterator, with fresh cursor state and live layout reads for each enumeration.


The non-generic `System.Collections.IEnumerable`/`IEnumerator` contracts used by
ItemsSource are absent from the pinned System 0.3.0 crate. FerroDock declares the
complete boundary in `collections/i_enumerable.rs` (nullable boxed current,
MoveNext, Reset and optional shared IDisposable). It supplies no alternative
production enumeration engine; source-backed control implementations remain
pending. Custom/native sources provide their own cursors and failure behavior.


The pinned System crate also has no ConditionalWeakTable/ephemeron runtime.
FactoryBase's native identity-keyed caches use weak keys, strong owner/item values
and the published Dictionary for reverse item lookup. Explicit untracking releases
source-owned entries, and access prunes expired noncyclic keys. Collection of
owner-to-key cycles remains a lifetime parity gap; native pruning also needs
performance integration. It is not a completed substitute for the CLR collector.

Item keys share the model's scalar/domain/canonical-reference equality and provide
consistent native hashes, including NaNs and signed zero. Arbitrary custom boxed
Equals/GetHashCode and subtype/concrete reference-box metadata still need shared
value registration. The domain model does not acquire a UI Base dependency merely
to access framework value metadata.


The production ReactiveUI Factory reuses System Dictionary and ObservableCollection
through the general shared handles. Its generic CreateList currently inherits the
native PartialEq element requirement. General BoxedValue/unregistered elements
and .NET default comparer semantics (notably floating NaN equality) remain open;
no alternative collection primitive is introduced to mask that dependency gap.


Overlay requests now reuse System TaskCompletionSource/Task, events and explicit
IDisposable, and the busy service uses actual ReactiveObject, WhenAnyValue and
ReactiveCommand.CreateFromTask. No local task or reactive engine is supplied.
Task-returning callbacks use the framework's Task<()>; synchronous Rust panics at
these async boundaries become its standard TaskWorkError diagnostic, while
returned task errors retain their shared object identity and cancellation state.
Rich CLR exception metadata and cross-thread task/service semantics remain open.

The native ICommand parameter is borrowed Any. DialogRequest.Close currently
requires a borrowed canonical BoxedValue handle to retain arbitrary result object
identity. General raw/unregistered borrowed-object promotion needs shared native
value/reference metadata; it is explicitly unsupported, without substituted
nulls or cloned results. Request/model-owned command callbacks are weak according
to the ownership guide; full CLR retained-command/GC lifetime equivalence remains
open. IDockDialogContent and IReloadable declarations do not resolve arbitrary
interface casts by themselves.

System 0.3.0 exposes ObservableCollection but no ReadOnlyObservableCollection.
A native live read-only forwarding adapter now supplies the service boundary,
retaining actual shared storage and source subscription/reentrancy behavior.
Mutation-version and full binding/class/GC metadata remain open. InvokeAsync now
uses an internal Future adapter over System task completion to defer pending
await wakes; the shared TaskCreationOptions API and wake transport performance
still need runtime integration. Navigation
also needs source IScreen-to-model interface resolution: the pinned IScreen has
only Router, with no Any/type-query hook. Do not substitute another routing engine.


Typed ShowAsync uses an object-safe raw-result core and a generic extension on
both concrete and erased service handles. Value outputs use their source default
on mismatch; nullable values use Option<T>, reference outputs Option<Rc<T>> and
object outputs Option<BoxedValue>. This prevents mapping a canceled/mismatched
ShowAsync<int> to a nullable result instead of zero. Native DialogContent retains
the original object and its explicitly supplied optional dialog-content projection.
Automatic membership for unregistered erased values and arbitrary reference/base
casts still require shared native metadata. No reflection is introduced.


Host resolution now uses an instance-scoped HostTypeRegistry for explicit native
IScreen/routable/dockable/service projections. All registered projections must
refer to the same Rc allocation, are held weakly and return the original typed
reference; existing dockable interface hooks are also used. The original resolver
still walks actual owner/window/factory/router structures. No alternate screen,
routing state or global mutable registry is introduced. Automatic unregistered
CLR type/interface/base discovery and custom object equality remain partial.

The overlay provider's cache uses weak screen identity keys with strong fallback
values. Native pruning releases expired noncyclic keys outside borrows. This has
the same unresolved ephemeron/value-retains-key cycle limitation as source-item
weak caches, and is not a replacement garbage collector. Lifecycle window markers
hold weak windows only. Their cleanup order and cache-only provider branch are
implemented with actual services; exact GC timing/performance and original UI/
Headless integration remain open.


Navigation and routable services now depend on the production reactive model crate
in their normal dependencies, matching the source assembly's model reference.
The previous test-only dependency was promoted; published FerroUI versions remain
exactly 0.3.0. The source router, command and Task implementations are reused.
Routable model components add immutable route/host metadata and independent shared
RoutingState instances; abstract RoutableDockBase adds no IScreen or extra router.

The shared IScreen/IRoutableViewModel traits expose no automatic native interface
query. Navigation uses the existing instance-scoped weak projection registry;
routed dockable projections can now forward through their boxed_interface hooks
for IReloadable. Applications using erased custom screens must register their
same-allocation projections explicitly. This is partial CLR metadata integration,
not an alternative screen/router or a claim of arbitrary unregistered casting.
Default routes use the enclosing native type's simple name. Full generic CLR names,
compiled class/binding/serialization metadata, IgnoreDataMember behavior and strong
CLR HostScreen/GC lifetime equivalence remain open. Model-owned callbacks and hosts
follow the existing weak native ownership convention. BusyRootDock reload tasks
retain live work and propagate shared task errors; cross-thread ConfigureAwait
and rich CLR exception metadata remain unresolved.


Activation now delegates to shared IActivatableViewModel.Activator.Activate and
returns the original shared IDisposable. It does not inspect or mutate IViewFor.
The shared activator's returned scope retains state weakly; CLR retention after
the model is released remains partial and must be checked with the eventual UI/
leak harness. IDockDispatcher belongs to the reactive-model services module, as
in the source assembly, and has no UI control dependency.

MainThreadDispatcher looks up RxSchedulers.MainThreadScheduler for EACH call and
always schedules there, without using ServiceDispatcher's captured context path.
It returns the actual System TaskCompletionSource task and captures action errors.
System 0.3.0 has no TaskCreationOptions.RunContinuationsAsynchronously and raw Task
continuations run synchronously. This public dispatcher requirement is explicitly
incomplete. The existing private DispatcherInvocation Future adapts await wakes
for internal overlay code; it does not alter this raw-task API or close the gap.

The services crate now directly uses ferroui-splat at exactly 0.3.0, the already
shared transitive version. RegisterDockOverlayServices uses its actual lazy
registrations and Locator. Nine defaults preserve registration order and existing
null-contract registrations; every HostOverlayServicesFactory invocation creates
fresh local services and resolves the three globals twice, in original argument
order. Query the Func with `get_service::<HostOverlayServicesFactoryFn>()`, which
returns the existing Rc HostOverlayServicesFactory handle without another wrapper.

Typed dual-interface resolvers use DockOverlayResolverExt. An erased mutable
resolver can supply its SAME-allocation IReadonly projection through
register_dock_overlay_services_with_readonly; mutable-only resolvers use the named
mutable_only overload and the current shared locator. Automatic runtime casts of
arbitrary erased custom resolvers remain a metadata gap. Registration and Func
closures retain the original resolver, including for copied registrations. Native
resolver/closure cycles still require explicit resolver disposal; CLR-equivalent
collection of unreachable cycles and full ownership/performance parity remain open.
No ad hoc container or static registry is added. FullName/exception metadata and
cross-thread parity also remain partial.

The Microsoft IServiceCollection helper remains pending. Neither the inspected
published System 0.3.0 nor Splat 0.3.0 exposes that service collection, and the
FerroUI reactive port guide lists Splat dependency-injection adapters outside its
ported framework scope. This is an unresolved dependency for the full goal, not a
waiver or permission to replace the source helper with a different container.
Framework-property control integration can proceed independently.


Framework integration now lives in a separate
ferrodock-model-reactiveui-services-ferro crate. It uses published ferroui-base,
ferroui-controls and ferroui-reactiveui-controls at exactly 0.3.0; ferroui-headless
is test-only. The new UI dependencies do not leak into Domain/reactive model/core
service crates. The older dirty sibling checkout lacks this release's ReactiveUI
controls integration, so only published source/API and crates are used for this
boundary. No sibling files are modified.

DockReactiveUserControl uses the existing ReactiveUserControlBase class and typed
ReactiveView convention. Native class/namespace/assembly/public Rust-path metadata
is registered and the shared build-scan 0.3.0 exports the library for dependent
markup builds. Metadata retains the abstract base's lack of a default constructor.
Consumer compiled markup and full generic/subtype/compiled binding semantics remain
unverified. VM PartialEq is the shared typed-handle constraint; CLR equality and
arbitrary class/value metadata equivalence are still partial.

The shared activatable-model query is crate-private. FerroDock reuses its existing
instance-scoped HostTypeRegistry for explicit same-object activatable projections,
registered before binding/activation. The helper also invokes shared ReactiveUI's
activatable type registration. The property and scope still use real shared
observables, activation fetchers, SerialDisposable, property storage and activators.
Automatic unregistered interface discovery and full CLR event/GC lifetime remain
open. Four real Headless tests prove supported replacement, fallback, subclass,
activation and release paths; they do not prove the full source docking leak suite.

The source visual-tree host resolver stays pending until DockControl, OverlayHost,
HostWindow and ManagedWindowLayer are implemented faithfully. Their absence does
not justify substituting test fixture controls into production lookup. Continue
framework-property models/controls and then wire that exact resolver algorithm.


The separate proportional-control library now uses published Base, Controls and
Markup.Xaml at exactly 0.3.0, with shared Build 0.3.0 for compiled XAML and Headless
0.3.0 for tests. The splitter ResourceDictionary is built against this library's
source model and the shared dependency metadata. Native namespace/assembly/type/
public-path registration and compiled loader registration are supplied. The standard
NAMESPACES constant is required for the build scanner as well as runtime discovery;
inline runtime-only tables were insufficient to compile a custom class by its
mapped XML namespace. The earlier reactive framework crate now uses that constant
pattern too. No dirty sibling checkout is a build dependency.

Native layout remains the source algorithm and uses real property storage and
class callbacks. The Proportion setter uses SetCurrentValue and the attached
proportion/collapse properties retain TwoWay defaults; original model-binding and
full consumer docking-template scenarios are still pending. All five source panel
policy hooks use native virtual dispatch rather than an external sizing engine.
Source midpoint-to-even fractional carry and NaN/signed-zero comparisons have
explicit native representations and source-case regressions.

UI Controls wraps FerroList, whose enumeration is copy-on-write snapshot based and
whose mutation version is not exposed. The source non-splitter enumerator therefore
uses an internal storage-identity check, preserving detected content invalidation
without a new notification listener or collection engine. This does not establish
exact source version behavior for capacity-only/storage changes. Utils/manager
records remain partial for that boundary. Keep indexed source layout loops live;
do not replace them with copied child lists to hide mutation differences.

Preview painting uses native AdornerLayer, resource lookup, shared value/interface
casts and DrawingContext. A test recorder forwards the full shared drawing-core
contract to Headless while observing rectangle/brush commands. This verifies
geometry, resource precedence and lifecycle, not real backend pixels or full CLR
unregistered brush metadata. Original Debug.Assert diagnostics, generic/runtime/
GC and desktop interaction/leak/performance equivalence remain partial. The debug
macOS compact-unwind-size linker warning remains visible and unsuppressed.


The proportional library's model-binding regressions now consume the production
ferrodock-model/ferrodock-model-reactiveui packages as dev-dependencies only. They
use the actual shared compiled binding path, typed property information and INPC
accessor factory. The production dependency direction remains unchanged; no UI
Base/Controls dependency enters the reactive model layer. Default TwoWay binding
metadata and current-value writes are checked at local and style priorities.

The separate unpublished downstream markup-test package uses the same published
Base/Controls/Markup.Xaml/Build/Headless 0.3.0 dependencies. Its custom type,
namespace, attached property and dependency ResourceInclude compile/load tests
exercise actual exported metadata. Native resource boxes may expose base object
handles, so shared ValueTypes projections recover the underlying class. These
are registered native projections, with no CLR reflection. Full model/interface
markup registration and the reactive framework control's dependent XAML remain
separate pending work.


Native recycling uses existing Base/Controls/System 0.3.0 and the original
UI-independent recycling-model interfaces. The concrete UI objects use native
class macros and weak-cached canonical Rc interface adapters. Native values cross
the model boundary through explicit boxing/unboxing functions; raw scalar and
registered native class handles project through the shared ValueTypes registry.
Unknown raw domain references retain identity; automatic plain-domain equality/
UI metadata is still incomplete. No substitute property/template/reactive engine.

The cache uses System Dictionary with a key adapter over shared AnyValue equality.
Native class identity and supported scalar hashes are preserved. Shared AnyValue
has no custom GetHashCode, so other types share a per-type bucket rather than
violating equal-key hash consistency. Arbitrary-value hash cost, equality callback
reentrancy and broad runtime behavior still need validation.

UI-owned metadata carriers describe the ORIGINAL recycling model trait handles
through the standard shared macros/export. This satisfies the Rust orphan rule
without adding UI dependencies to the model crate or behavioral replacement
interfaces. Full original interface member/notification metadata remains open.


The native docking UI crate adds no replacement dependency engine: OverlayHost
uses published Decorator, ContentPresenter, Grid, native properties, actual
visual/logical parents, ControlTheme/resources and Dispatcher::post_local. Layer
collections directly retain Base FerroList. The UI IList boundary also accepts
the original model ListHandle, preserving custom/plain/read-only backends and
subscribing directly to their System notifications. Exact native collection
mutation versions remain an existing shared dependency gap.

Overlay providers use Base's actual IServiceProvider and System Event. Their
static registry follows native thread affinity; source global cross-thread
provider updates/UI repost and complete CLR delegate equality are not proven.
Native class/interface projections retain the same object. No UI dependency was
introduced in the model crate. Plain nullable collection registrations and owned
metadata declarations are exported by the standard BuildScan to the external
consumer; no alternate compiler, dispatcher or DI container is introduced.

Native service overlay adaptation now reuses the published
`ferroui-reactiveui-controls::ReactiveCommandAdapter` at the binding conversion
boundary. The BusyOverlayControl direct property keeps the original ReactiveUI
command so unchanged service state does not raise a spurious command change.
The shared 0.3.0 split between ReactiveUI and Base ICommand remains an explicit
metadata boundary. Read-only request collections project to the published
IItemsList/ItemsSource contract with live indexing, stable original collection
identity and borrowed typed notifications. No alternative command or collection
engine is introduced. The specialized foreign-handle descriptors do not establish
complete generic/erased service member metadata.

Rendered request integration adds the published `ferroui-themes-fluent =0.3.0`
to the Headless consumer, matching the base FluentTheme of the original test app.
Its locked Dialogs/platform dependencies supply shared framework resources;
FerroDock does not copy the base control themes. The native docking UI also
references its existing recycling library, as the source project does, to reuse
the native/domain content value bridge. Request model and host aggregate binding
descriptors retain the original domain handles in the UI layer. All six service
handles have standard shared nullable registrations for typed default-layer
bindings; no custom binding compiler, service engine or command dispatcher is added.

Foreign enum metadata uses the published `ferroui-build-scan` TypeModel and
AssemblyModel schema in addition to its normal Export. Rust orphan rules prevent
applying the shared enum macro to a type owned by the model; the shared scanner
also marks handwritten MarkupTyped implementations as absent from its model.
A build-only syn-based bridge generates runtime and build descriptors from the
original model declaration, with original typed values and const numeric checks.
It reads a hidden immutable source constant through a model build dependency,
so no UI dependency is added to Domain and no sibling checkout is required.
Unchanged descriptor/model output uses shared write_if_changed. All XAML parsing,
compilation, loading, property, brush, resource and enum conversion behavior
continues through the published framework and original enum. Other enum/flags
forms and broad runtime/generic metadata remain separate unimplemented work.

The visual lifecycle behavior reuses shared Visual events, IDisposable tokens,
attached properties, class handlers and ValueTypes interface casts. A private
thread-affine weak table adapts native subscription tokens to the source's static
C# delegate removal; it adds no visible properties or lifecycle engine and owns
neither controls nor models. Full weak-entry pruning cost and CLR event/GC parity
remain unverified. Source detachment timing was checked against the actual
12.1.3 dependency: PresentationSource remains set while detach callbacks run.
Automatic membership for arbitrary unregistered raw model objects and identity
through noncanonical custom adapters remain separate runtime metadata gaps.

RootDockControl and its accessibility peer reuse the native TemplatedControl,
ContentControl, template-part metadata, routed Tunnel events, AutomationProperties
and ControlAutomationPeer. The native UI and markup test packages add the original
ReactiveUI model crate as a dev dependency only. Native root tests also load the
published base FluentTheme; production Domain dependencies remain UI-independent.
Original root/dockable interface handles cross the DataContext boundary through
model_projection::to_root_value/to_dockable_value, retaining the same model Rc.
Raw concrete model contexts can register concrete casts through shared ValueTypes
at the UI composition root. Uninstantiated generic cast registration functions or
exported macros leave unread casts in the published build scanner and invalidate
unrelated downstream assignment proofs, so the native library exports neither.
The interface boxing helpers do not register casts or introduce another type or
binding engine. Arbitrary raw-model discovery/generated interface metadata remains
open. Native automation peers retain weak owners; the source's additional strong
owner field is not duplicated into an Rc cycle. Complete CLR retention/GC parity
and desktop provider behavior remain explicitly unverified.

The separate ferrodock-controls-deferred-content-control crate depends only on
published FerroUI Base/Controls =0.3.0 and exports metadata with the standard
ferroui-build-scan. Its test-only dependencies are Headless/testing and the base
FluentTheme. The native docking library consumes it as the source project does;
no domain or ReactiveUI model acquires UI dependencies. Deferred hosts/presenters,
inheritable scheduling, signed 100ns TimeSpan, transitions/easing and dispatcher
posts/timer wakes use actual shared framework implementations. Plain timelines and
static settings expose source members via standard MarkupType descriptors; a
concrete Rc<Timeline> nullable registration supplies the attached-property boundary.
There is no private XAML, binding, property, animation or task engine.

The source timeline queue algorithm remains local to this source library. Native
positive-delay dispatch uses DispatcherTimer followed by Background post in place
of Task.Delay/ConfigureAwait(false). Queue entry verifies UI-thread access; source
background repost and process-wide default sharing are still partial. Thread-local
defaults and weak queue-owner/target-timeline links follow native thread-affine Rc
constraints; CLR GC/retention equivalence is not claimed. Native target adapters
are canonical while pending and hold the original native controls. Raw opt-out
content can use the original interface handle or explicit shared ValueTypes casts;
arbitrary unregistered membership remains separate runtime metadata work.

FerroDock.Settings is the original separate native configuration/property library,
with published Base/Controls =0.3.0 and the existing UI-independent model. All
settings property/binding/input/builder behavior uses shared framework contracts.
Process-wide configuration uses an immutable OnceLock holding a standard RwLock;
callbacks and their capture destructors run outside storage locks. Global native
callbacks require Arc<Fn(String)+Send+Sync>, matching process-wide native sink
constraints. Source arbitrary delegate/gesture-reference/untyped field semantics
and the Debug.WriteLine transport remain explicitly partial.

The original model-enum metadata bridge now belongs to Settings, which needs it
before the main docking assembly can load. It reads original model declaration
constants for DockOperation, DockFloatingWindowHostMode and DockWindowOwnerMode.
The generated TypeModel declares the original Rust enum; its public metadata path
is the local descriptor carrier. This distinction preserves nullable original-enum
conversion in the standard Model compiler. Domain still has no UI dependency.
The native UI retains its hidden DockOperationMetadata alias and uses normal
Export. Fluent resources and relevant consumers directly depend on Settings where
emitted Rust uses the descriptor owner. A separate settings-only consumer proves
this dependency graph without the docking UI/theme crates. An actual incremental rerun
refreshed the scan report while preserving active runtime/scanned/final output
hashes and mtimes; all 18 recorded files across build variants were unchanged.

The native ItemDragHelper also uses shared InputElement routed pointer events,
TransformOperationsBuilder, ItemsControl/SelectingItemsControl, ScrollViewer and
DispatcherTimer. Original factory and IDockItemDragPolicy contracts remain in the
framework-independent model layer. Mutable source fallback uses shared ValueTypes
to discover the original model IList handle over the same native FerroList; built-in
concrete string/boxed/nullable-boxed casts are registered in the UI layer. Other
concrete sources require composition-root registration. Native IItemsList itself
is read-only at the interface boundary, so arbitrary generic mutable-interface
discovery and broad notification/version/runtime metadata parity remain partial.


Dock targets and the floating adorner window now use shared native System
Dictionary, weak class/template handles, name scopes, visual hit testing/root
transforms, TopLevel screen/client coordinates, styled/direct/attached properties,
automation peers/providers and PlatformManager window construction. Shared
FerroUI build embeds five original selector images alongside nineteen complete
source Fluent dictionaries. Native theme compilation now directly depends on the
ported original ProportionalStackPanel for global target layout. Managed window
and complete AdornerHelper/drag-state/control dependencies remain unfinished.


Native MDI layout uses published Panel/Control/Size/Rect/Vector and the existing
model IMdiDocument, MdiLayoutDefaults and ListHandle abstractions. The published
System collection surface has no generic IReadOnlyList contract; original input
lists remain live through the existing handle with read-only use, while exact
interface/signature/generic discovery and observable version parity stay partial.
Native struct/class/interface/flags descriptors and concrete ValueTypes nullable/
interface registrations feed the standard compiled consumer. Native Controls
count/get supplies live child access without source foreach version invalidation.
Classic.Instance is UI-thread-local Rc, not verified process-wide identity.


ManagedDockableBase uses the original framework-independent IDockable/selector/
restriction/recycling contracts and TrackingAdapter, with System Event<str> and
INotifyPropertyChanged. Its source strong owner/factory/context links remain strong.
A local native reference adapter forwards the same model/event into shared markup
metadata (external notification traits cannot be implemented on foreign Rc<T>).
Context native values travel through the existing recycling bridge and shared
ValueTypes/object/value equality. Composition needs no production dependency on
FerroDock.Model.ReactiveUI. The macro's dependency reexports are tested downstream.


Host cleanup uses actual native ContentControl/StyledElement/Visual and the existing
DeferredContentControl.ClearPresentation, original ListHandle/IDockable containers,
reference-identity visited set and existing recycling/native class projection.
Visual collection reads remain live; shared native enumeration version gaps and
unregistered interface discovery remain partial. Tool menu reattachment uses native
Button/FlyoutBase, BindingOperations.GetBindingExpressionBase/UpdateTarget and
SetCurrentValue. Window-state helpers use original model enums and native WindowState;
preview context uses native UI-thread TLS Rc, with source cross-thread globals still
partial. Nullable root resolution returns the actual presentation-source RootVisual,
which is TopLevelHost in the published native backend; it does not return a substitute
Window. Full host/root integration must be verified during the actual host ports.


Native dock containers/content hosts use published TemplatedControl, SplitView,
ItemsControl/ItemsPresenter/Grid, ControlTheme/ResourceKey, property observables and
the original Factory/model contracts. FerroDock.Ferro now directly depends on the
ported proportional-stack-panel crate for ToolDockControl's original parent lookup.
IDockItemContainerMetadata remains a core opaque-object contract; its native descriptor
belongs to FerroDock.Ferro and uses the existing recycling bridge. No UI dependency or
metadata state is added to the reactive/domain model layer. Generated native/property
model integration and complete source Fluent dictionaries remain pending.


Default dock templates use the shared FuncDataTemplate, CompiledBinding/ClrPropertyInfo
and InpcPropertyAccessor over original model contracts. Core IDockable's optional
System INotifyPropertyChanged query returns None for plain models; original reactive
and managed composition forwards their existing event. No UI dependency is added to
core or ReactiveUI models. Native canonical interface projections retain the same Rc,
and raw models use shared ValueTypes/native notification metadata. The standard binding
accessor owns event delivery/disposal. The original grid cross-enum binding conversion
failure is preserved, as verified against framework 12.1.3 by the console-only probe.


Drag-preview controls use native TemplatedControl/Window/properties and the existing
ported ControlRecycling implementation. Every preview gets its own original cache;
there is no additional production dependency or replacement state model. Native
ControlAutomationPeer/WindowAutomationPeer and IValueProvider carry source accessibility
semantics; setter failure uses the shared ProviderError contract. DragPreviewWindow's
complete source Fluent dictionary is compiled through the existing theme assembly.
The complete DragPreviewControl resource now consumes UI-owned IDockable metadata
and the original NotNullConverter through the shared compiled-markup pipeline. The
theme assembly directly references the existing recycling crate for generated types.
DragPreviewHelper still requires actual managed document/layer/host/control implementations.

UI-owned IDockable markup uses the published MarkupType descriptor and optional
notification callback. A copy of the declarative descriptor changes only that hook;
it borrows the original System notifier for canonical Rc and nullable Rc handles.
No orphan trait implementation, reflection or private registry is needed. All source
property/method invokers remain generated by ferro_markup_type. Original nullable
factory/policy/bounds values and inherited recycling-interface conversion use concrete
shared ValueTypes registrations. Explicit imports are required for scanner resolution;
an unresolved cast prevents the shared compiler from answering unrelated conversions.
NotNullConverter implements the existing native IValueConverter with the original
algorithm, shared BindingError failure and x:Static Instance metadata. The UI-thread
Rc singleton remains an explicit CLR process-static identity gap.

The selector uses native TemplatedControl, ListBox, compiled templates and standard
expand-collapse/selection/scroll/value automation providers. Its immutable item model
retains the original IDockable Rc and queries the existing IDockSelectorInfo contract.
No new external dependencies or UI references in domain code were introduced.
IReadOnlyList is absent from the published System package; the shared domain adapter
supports original custom contracts, immutable arrays and live ObservableCollection
views with typed event forwarding. Its ItemsSource bridge accepts both shared model
objects and boxed typed handles through native from_markup_value, preserving identity.
The existing observable adapter cannot recover the shared collection's hidden mutation
version; exact CLR fail-fast enumeration remains partial.

The published ItemsControlAutomationPeer's get_provider path does not invoke its separate
scroller discovery hook. The Dock peer keeps source delegation unchanged. Tests verify
initial delegated values and explicit shared scroller discovery before real scrolling;
ordinary automation-client discovery and desktop behavior need further parity work.

Command-bar models keep the domain's ReactiveUI ICommand and IReadOnlyList handles.
The native adapter uses ferroui-reactiveui-controls' existing ReactiveCommandAdapter
and boxed_parameter, preserving arbitrary parameter identity and CanExecute/Execute.
Content/icons retain the existing recycling value bridge. UI-owned model descriptors
and the Settings foreign-enum bridge export original model handles into the shared
compiler model without orphan traits, a duplicate enum or a private event/binding
runtime. The selector and host share one typed IReadOnlyList ItemsSource projection.
System Event tokens support original manager unsubscribe behavior; shared MarkupEvent
metadata is add-only and discards subscriber results, so markup removal is an explicit
gap. Event<()> also cannot transport arbitrary source EventHandler sender/EventArgs.
Strong manager callbacks retain source behavior until Detach; unreachable Rc cycles
and automatic metadata for raw domain Any context objects remain shared gaps.

PinnedDockWindow uses the native Window class/vtable, inherited WindowAutomationPeer
and IInvokeProvider, plus the complete original Fluent dictionary. Tab-strip wheel
helpers use existing ScrollViewer line methods, native bubbling wheel events and
shared Disposable::create for explicit idempotent cleanup. Strong event captures follow
the original lifecycle; no custom scrolling, input or disposal runtime was added.
The source Double.Epsilon threshold maps to f64::from_bits(1), not f64::EPSILON.

Managed window documents compose the existing ManagedDockableBase and the real domain
IDockWindow/IDock/IDockable objects. Optional System notifier and UI-private boxed-interface
queries preserve runtime interface tests without importing controls into domain code.
The direct ferroui-markup-xaml dependency remains exactly 0.3.0 and supplies DataTemplate
matching and TemplateContent loading. Canonical interface conversions use native
register_boxed_cast and original boxed_reference/boxed_interface queries, retaining the
same Rc model and application-owned template extension points. Original strong event
subscriptions require Dispose; no Drop cleanup or replacement notification runtime.
Native try_set metadata selects the existing compiled INPC path around a nonnotifying
typed specialization. Shared static assignability over conditional opaque-interface
casts is incomplete; the document matches these values through actual checked casts.
General template alias/runtime parity remains explicit partial work.

MdiDocumentWindow uses the shared TemplatedControl/property/vtable, routed input,
IPointer capture, templates, System notifications and automation providers. Layout,
capabilities and factory activation continue through the existing ported engine.
The Settings source-driven enum bridge now also exports original MdiWindowState and its
nullable metadata; there is no duplicate UI enum. Native control/host membership is
queried through existing boxed-interface views, including optional IDockControl query.
Concrete host/state classes still need to supply these views. The original Window-root
check remains unchanged; native TopLevelHost roots can prevent child-window discovery.
The original tunnel capture-loss subscription is retained; native Direct capture-loss
routing is tested separately from the handler body and normal release. Original runtime
and desktop parity, full MDI dictionaries and CLR cycle lifetime remain partial.

MdiDocumentControl uses original System model/list notifications through the existing
ListHandle optional notifier, shared native observable DataContext, read-only DirectProperty
and original MDI managers. Strong subscriptions follow source detach/rebind; custom source
enumerators dispose before visibility/manager callbacks. Its peer projects manager names
from shared native metadata or compiler-supplied concrete names, with a hidden override
for source-visible names; exact unregistered generic/nested CLR names remain partial.
Both managed-tool converters use native checked casts and existing boxed-reference model
projection. Returned tool docks retain original Rc identity. Shared converter/markup/error
APIs and UI-thread singleton conventions are reused; no dependencies or frameworks were
substituted. Complete nested model/factory and live IList ItemsSource metadata remain
required for the original MDI dictionaries and application integration.

## Original dock collection and compiled MDI metadata

UI-owned IDock/IDocumentDock/IToolDock/policy descriptors retain original Rc handles and
optional System INPC. Native base and interfaces declarations serve runtime and compiler
inheritance lookup respectively. Class policy descriptors declare their own six properties
and interface separately; using an interface as a class base loses System.Object assignment
in compiled resources. The original virtual Get uses DockCapabilityPolicyImpl, not a new
policy object. Six source enums are exported through the existing Settings generator;
negative integer declarations now preserve Alignment.Unset=-1.

ListHandle now exposes optional INPC, original typed collection event hooks and stable
backend identity. The UI ItemsSource view does not copy state. Source event slices preserve
Add/Replace/Move/Remove/Reset contents and indices, including removed/replaced objects.
Unsubscription removes the source token. The domain remains UI independent. A notifying
custom backend with only untyped counts is rejected by native conversion until its original
typed event hooks are supplied; reconstructing removed values or emitting Reset would alter
the source contract. Plain/custom nonnotifying lists retain their semantics. Mutable native
reordering converts to a view of the original IList and retains its custom comparer,
read-only state and enumerator behavior. Full generic list metadata/CopyTo and arbitrary
erased generic discovery remain partial.

FerroUI 0.3.0 uses INDEXER_NAME="Item" in Base compiled bindings, while its System
ObservableCollection emits "Item[]". A compiled integer index path on an IList descriptor
uses the INPC accessor, so Replace leaves the indexed value/subscription stale until the
list property changes. The downstream test explicitly characterizes this native behavior.
No fake INCC interface, renamed source notification, copied collection or private accessor
has been introduced. The complete source MDI host uses ItemsSource, whose original typed
changes work; the indexed binding parity gap remains open for shared runtime work and
original desktop comparison.

Native two-way object bindings can deliver nested Option<BoxedValue> transport wrappers.
The EmptyContent descriptor normalizes that native representation through shared ValueTypes
before the existing UI/domain recycling bridge, preserving content and null roundtrips.
The full host theme/real ReactiveUI model tests verify this boundary without adding UI to
Domain. MdiDocumentWindow's full dictionary, global ChromeButton/context menus and complete
DockFluentTheme/application integration remain pending.

## Root policies, converter contracts and factory method commands

The native root descriptor covers its 15 own members and borrows original optional System
INPC. Policy objects retain the original nonnotifying behavior: compiled MultiBindings
observe source property/reference paths and then the original resolver re-evaluates only
the first value. Additional binding values are invalidation inputs, preserving original
root/dock/override precedence. All six single/multi converter fields share their original
native UI-thread instance; CLR process-wide static/exception/UTF-16 parity remains partial.
The existing enum metadata generator also supplies PinnedDockDisplayMode. Close-button mode
name lookup uses the shared DocumentCloseButtonShowMode descriptor; local source parsing preserves numeric and comma-name
Enum.TryParse behavior for supported inputs without a replacement enum/type system.

The UI factory descriptor currently declares its 47 command overloads and two close flags.
FerroUI's existing GetBestCommandMethod selects the System.Object overload when present,
matching the source preference. Its invoker calls the original IFactory default parameter
adapter, which dispatches the original typed virtual operation. Shared ValueTypes normalizes
native object transport; canonical dockable projections retain model identity. Ordinary
command-only values upcast directly from AnyValue to Any, preserving their concrete type
and identity; no recycling transport wrapper is needed for an input that does not return.
Typed-null and object-null remain distinct. Nullable DockWindowOptions handles are registered
for the original option overloads. Factory creator/search/registry/event/tracking/generic
metadata, the full window-options descriptor and arbitrary erased raw type discovery remain
pending; do not use this command descriptor as proof of a complete native factory API.

Native compiled casts of a second owner use Owner.((dmc:IRootDock)Owner). The source MDI
close-button active input belongs to MdiDocumentWindow.IsActive; IDockable has no such
property. Downstream tests use that actual native class rather than adding a replacement
model property. Full source MDI window/global themes and managed host/control remain pending.

## Managed layer and original registry lifetime

ManagedWindowLayer is an actual native TemplatedControl using the original System Dictionary,
optional source INPC and original typed collection events. The native counts-only untyped
INCC cannot supply removed/replaced objects, so the existing ListHandle typed hooks deliver
source payloads without a mirror or reset approximation. Counts-only custom notifying adapters
still need their original typed hooks. Source callback strength/explicit detach, keyed overlay
identity, template-replacement behavior, custom-manager input list identity and post-callback
Count are preserved. The hidden offset probe is the original concrete ManagedDockableBase
subclass and real MdiDocumentWindow; no replacement production template is introduced.

The original registry operations use factory identity, weak keys, strong dock/layer values
and access pruning, following the existing native weak-key cache boundary. System 0.3.0 has
no ConditionalWeakTable/ephemeron runtime. In particular, ManagedWindowDock.Factory is an
original STRONG reference: normal cached docks keep their keys alive through value-to-key
cycles, and pruning cannot collect them. Native tests characterize that retention, then
explicitly clear Factory only to test the noncyclic expiry path. Original strong properties/
events are not weakened. Full cycle collection/GC timing, process-wide cross-thread table
identity and forced-GC leak parity remain unresolved. Explicit layer unregister/detach
removes the layer entry but does not invent removal of the original persistent dock entry.

Native styled-property conversion reads direct registered conversions; metadata assignability
can also traverse reference/nullable casts. The managed dock registration therefore declares
original raw/Rc ManagedWindowDock to nullable IDock conversion through shared ValueTypes;
nullable source normalization reuses it. All three representations retain the same model,
and a separate compiled consumer verifies the actual layer Dock binding/rebinding. This adds
no binding engine or alternative model object. The source layer sets its own DataContext;
consumer Dock bindings use an explicit external owner source.

The full ManagedWindowLayer dictionary is compiled and renders original MDI classes/list/
state/ZIndex and the overlay canvas. TryGetLayer(Visual) is absent until actual DockControl
ancestor/root discovery is available. Concrete host/controller/tracking/adorner/preview callers,
the complete document window/global themes and the source interactive application remain
pending. Unit content-part templates test probe measurement only; full content rendering is
not claimed.


## Fluent source command and deferred-content template integration

The original ScrollViewer dictionary uses eight nullable command assignments. The
published 0.3.0 emitter repeats the value-none branch for those assignments, producing
if_same_then_else lints in generated code. The existing generated Fluent module alone
allows that lint alongside clone_on_copy; handwritten source code is unchanged and
strictly checked. No shared compiler or dependency checkout was edited.

The Fluent resource crate now directly depends on the existing FerroDock deferred-content
crate: source ToolChromeControl/DocumentControl/ToolControl templates instantiate its
actual native controls. This adds an internal Cargo.lock edge; published crates remain
pinned to =0.3.0. Native scrolling uses the framework's line methods and invariant float
parser, not a local substitute. Original arbitrary object/string/interface ReferenceEquals
scope is not proved by the implemented box/native/model identity bridge and remains open.


## Pinned layout and borrowed converter input identity

Pinned control/panel/peer and original layout dictionaries now compile against the same
published =0.3.0 crates. Native property/model events, actual grids/thumbs/windows/layers
and the original Rust engine are used. UI-owned layout/content descriptors delegate to
the original Rc objects; they do not store another model graph.

EitherNotNullConverter returns the original first non-null object. Its original all-null
fallback returns the caller's IList itself, but the native IMultiValueConverter signature
receives a borrowed slice and cannot retain that owning list reference. The current
list-valued fallback copies values and is explicitly partial. Input-list identity needs
shared native representation support before full parity can be claimed. No null fallback
or value comparison is substituted, and no native dependency checkout was changed.


## Original sample preset-manager dependency boundary

FerroUI 0.3.0 resource provider handles do not inherit Any and expose only optional
as_object() for native class instances. Plain ResourceInclude implements no object
recovery; once erased to Rc<dyn IResourceProvider> there is no safe original Source/type
query. IResourceDictionary exposes only merged_dictionaries_snapshot(), not the mutable
MergedDictionaries collection required by original recursive preset removal. The current
dirty sibling source was read-only inspected and has the same API shape. No sibling or
published dependency source was changed and no unsafe cast, partial preset recognizer or
replacement resource engine was introduced. Original concrete preset manager units remain
pending; their source interface uses the existing shared domain IReadOnlyList adapter.

Published System has no Debug.WriteLine/listener equivalent. The original sample's event
subscriptions remain active, with original messages routed to debug-only stderr. General
listener identity and current-culture debug number formatting are recorded partial gaps.
Native host runtime class-name queries support the original status text without reflection;
arbitrary generic/custom host naming and broader lifetime parity remain unverified.
