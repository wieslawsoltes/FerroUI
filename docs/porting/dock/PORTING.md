# Native desktop dependency fixes (2026-10-24)

The scoped desktop pass exposed two FerroUI 0.3.0 failures. A seventh isolated published
package, `vendor/ferroui-native`, preserves caller ownership for four borrowed URI inputs
in the Objective-C++ storage backend and rejects accessibility queries/actions whose
control has been released. Live controls are held only for the duration of a callback;
removed visual trees are not retained. The original Dock engine, sample ViewModels,
JSON format and sibling FerroUI checkout are unchanged.

`vendor/ferroui-native-provenance.json` records every published file, three modified files
and exact patch hashes. The package's MIT license and notices are retained. The ownership
regression fails against the original backend and passes with the fix. All 127 native
library tests and 1,967 workspace tests pass. Actual native Save/Open now pass on the clean SSD review bundle, including restored
views/commands, cancellation and malformed JSON. Physical reorder and tool floating/docking
were confirmed by the user. See CONTINUATION.md for the
current artifacts and validation state. These dependency fixes do not promote broad
source-file port coverage or imply completion of deferred framework APIs.

# Scoped core automated acceptance checkpoint

Both themes now have composed native pointer coverage for rendered selector splitting,
actual splitter resizing, document/tool tab reordering, move/float/redock, pin/preview/
unpin and close. Original factory hide/restore and both close configurations are tested.
Standard JSON reload and exact split/floating/pinned/hidden replay retain original model
contracts, including source OriginalOwner ignore behavior. No production/dependency code
changed; no alternate engine or test-only production entry point was added.

All 23 stages pass: 1,967 tests/57 suites, markup 204, sample 44, zero failed/ignored. Source
coverage statuses/counts remain unchanged. Both desktop artifacts and original parent
source/index remain preserved. The distinct SSD review build is ready, but native desktop
acceptance remains unavailable while the Mac is locked. See DELIVERY-SCOPE.md and
CONTINUATION.md for the concrete remaining gate; automated tests do not close that gate.

## Previous checkpoint

# Composed docking acceptance checkpoint

After ordinary JSON reload, real pointer tests now move a document between panes, float
it through the original factory into an actual HostWindow and drag its last tab back
across windows under both themes. Exact floating-layout JSON, original model/factory/owner/
selection identity, selected floating tab panel, drag reset, closed host and registry
cleanup are checked. Production code and dependencies are unchanged. Tests use original
public APIs and compiled/native controls, without alternate state or engine entry points.

All 23 stages pass: 1,963 tests/57 suites, markup 200, sample 44, zero failed/ignored.
New test target is mapped to original DockControlState/DockManager; inventory status counts
are unchanged. Original parent source and both desktop artifacts remain preserved. Next:
selector-driven split/resize and composed tool operations, plus native desktop verification
when Mac is unlocked. See DELIVERY-SCOPE.md, VALIDATION.md and CONTINUATION.md.

## Previous checkpoint

# Native file workflow acceptance checkpoint

Added end-to-end native storage evidence without production changes. Compiled menu click
events run the unchanged native adapter with controlled platform picker results and
actual temporary-file streams under both Fluent and Browser. Late binding, owning-window
lookup, async gating, JSON encoding, original custom graph/views/locators/commands and
cancellation/picker/file/parse/null outcomes pass. Source inspection preserves the sample's
lack of context for newly-created Document4; existing restored documents retain contexts.

All 23 checks pass: 1,961 tests/57 suites, sample 44, zero failed/ignored. New test file is
mapped explicitly; source statuses/counts stay 227/465/1236 of 1928. Production/dependency
inputs, old desktop artifact, distinct SSD review bundle and original parent source are
unchanged. The Mac remains locked. Next: composed core docking/drag/drop/float/redock
acceptance through actual state/engine/host paths and the pending desktop visual pass.
See DELIVERY-SCOPE.md, VALIDATION.md and CONTINUATION.md.

## Previous checkpoint

# File menu persistence checkpoint

The accepted core delivery sample now composes native Open/Save file services with a
separate ReactiveUI file-action model. Compiled menus/status bind to its asynchronous
commands; view code remains passive. Original serializer stream Save/Load owns byte
encoding, and the original factory initializes loaded custom model graphs. Cancellation,
errors, busy gating and both-theme restored views have tests. App wires both lifetimes.

All 23 checks pass: 1,959 Rust tests/57 suites, sample 42, zero failed/ignored. Five new support
files now have explicit inventory mappings; source coverage statuses/counts are unchanged.
The original App F11 attachment note was corrected to reflect already-tested source.
Framework versions/features/vendor code, original parent sources and the protected old
executable/window remain unchanged. The separate SSD desktop review build passes; its distinct bundle is ready. Native
picker/visual acceptance remains open while the Mac is locked.
See DELIVERY-SCOPE.md, VALIDATION.md and CONTINUATION.md for evidence and next actions.

## Previous checkpoint

# Current delivery progress: actual sample JSON restore

The backend catalog now accepts explicit typed application subclasses and own properties,
retaining their real constructors, Rc identities and original polymorphic families. The sample
registers all12 persisted custom classes with C# wire names. App composes serialization and
initialization services; MainWindowViewModel can save/load ordinary JSON programmatically.
Restoration updates original live factory locators and contexts; malformed input preserves
current layout, while JSON null clears it. The compatibility constructor keeps source behavior.

18 unchanged-source checks and13 JSON fixtures match;64 previous outputs remain identical.
Native tests retain custom classes and the original CreateDocument command. Full sample restore
under Fluent/Browser realizes original views and creates Document4 through that command.
Final57455 passes all23 stages:1,951 tests/57 suites (sample34, backend9 plus usage doctest),
strict Clippy and all normal boundaries. Protected desktop/original source/index remain intact;
framework versions/features/vendor sources are unchanged. Only local JSON and test dependency
edges are added. Inventory227/465/1236 unchanged; full file coverage is not the delivery metric.

Next: File menu commands, native picker/storage integration, cancellation/error feedback and
refreshed desktop verification. [DELIVERY-SCOPE.md](DELIVERY-SCOPE.md) remains the acceptance
scope. The source sample's primary document dock has no required Id; restore chooses the first
document dock in source traversal order. General application conventions, metadata discovery
and exhaustive serializer parity remain partial; see SERIALIZATION.md.

## Previous checkpoint

# Current delivery progress: runnable sample tab style selection

The source sample now includes a compiled Fluent/Browser Tabs selector. ReactiveUI commands
route through the existing passive behavior and framework-independent service contract;
App owns one actual Browser overlay added/removed via native Styles. Existing preset/light-
dark behavior remains. Live App tests switch both ways without replacing original layouts,
use native tab pointer input and change presets/variants. Newly initialized views inherit the
choice; repeated commands remain idempotent. The new UI hook uses a weak owner reference.

Final21542 passes all23 stages:1,944 tests/57 suites, sample29, strict workspace and16 selected
release-library Clippy, ten standalone suites and eleven normal boundaries. Existing framework
versions/features, six vendor packages, original source and protected desktop artifact are
unchanged. Only a local sample-to-Browser dependency edge is added. Inventory227/465/1236
unchanged. Read [DELIVERY-SCOPE.md](DELIVERY-SCOPE.md) for the user's accepted scope.

Next: explicit sample model JSON registrations and save/load actions, then a refreshed desktop
run and final cross-feature docking/appearance checks. The existing desktop window has not
been refreshed yet. See CONTINUATION.md for concrete integration targets.

## Previous checkpoint

# Current delivery progress: standard core JSON catalog

The accepted core docking/JSON/Fluent/Browser scope is defined in
[DELIVERY-SCOPE.md](DELIVERY-SCOPE.md). New backend adapter
FerroDock.Serializer.SystemTextJson.ReactiveUI supplies a usable compiled catalog for14
standard layout classes and DockWindow, using the existing serializer and original models.
Its public factories need no caller-assembled test metadata. Source default-model wires,
complete graph and configured-container fixtures match;27 C# checks pass and46 prior
outputs stay unchanged. Original shared identities, factory initialization and post-load
AddDocument, stream contracts and requested list creation are covered. Both Fluent and
Browser restore fresh models and retain native pointer tab selection and original pin/unpin.

Final72420 passes all22 stages:1,942 tests/57 suites, strict Clippy and dependency boundaries.
Existing framework versions/features, vendor sources, original source and protected desktop
artifact are unchanged. The new adapter/workspace and test-only integration edges are the
only manifest additions. Inventory227/465/1236 of1928, with two newly started serializer
units and no complete promotions; full source coverage is not the delivery metric.

Next: runnable sample save/load, application-specific model registrations and Browser
selection, then final cross-feature docking/visual checks. The standard catalog does not
automatically discover application subclasses, and closed Rust layout enums currently reject
unnamed numeric values. Template/unknown-interface fallback coverage and exhaustive serializer
parity remain partial. See SERIALIZATION.md for usage and exact scope.

## Previous checkpoint

# Accepted functional delivery target

The user narrowed delivery on 2026-10-23 to core docking, standard JSON serialization,
Fluent (the current normal sample theme) and Browser tabs, with their supporting features
working. [DELIVERY-SCOPE.md](DELIVERY-SCOPE.md) defines acceptance and supersedes historical
full-repository completion requirements below. Preserve original engine/API behavior within
that scope. Additional backends/serializers/samples and exhaustive framework/compiler APIs
are deferred unless required. The broad source inventory remains accurate, but its11.8%
complete figure does not measure this functional delivery.

Seven actual SDK class schemas now build from native declarations and replace the old C#
class headers/property rows in the downstream consumer. All101 property types/order/ignore
facts, actual bases/locations and original interface precedence are verified. Poisoning all
seven old schemas leaves compilation/output identical; native fact mutations affect the
schema. Existing92 selected native accessors and source wire/engine graph assertions remain.
Logical class identity/abstraction/direct interfaces and foreign dependencies stay explicit.
Final98284 passes all21 stages and1,932 tests/55 suites; protected desktop remains unchanged.
Next: ordinary core docking JSON catalog/save/load, then the two-theme functional acceptance
checks. See SERIALIZATION.md and CONTINUATION.md for implementation boundaries.

## Previous checkpoint (historical broader scope)

# Current: native class property order and IgnoreDataMember

DockableBase/DockBase/RootDock/DocumentDock/Document/DockWindow native metadata now preserves
all101 original declared property names/order and24 ignored-member annotations. Original
callbacks and markup accessibility remain. The actual SDK consumer checks each native row
against the unchanged compiled original schema before emitting its existing92 selected
accessors. This supplies missing native facts for the semantic stage; full native logical
header/type/dependency schemas, remaining attributes and catalogs are still incomplete.

Focused89762 passes SDK8/markup196/consumer9. Final85300 passes all21 stages with1,929 tests/
55 suites; full completion stays correctly rejected. No reference/assertion/dependency/version/
feature/framework edits or source-unit promotions. Inventory227/463/1238 of1928 is unchanged:
11.8% complete,24.0% partial,64.2% pending;35.8% started. These measure file coverage, not effort.
Protected desktop and original/sibling/cache sources remain unchanged. Full goal stays active/
incomplete/unblocked. See SERIALIZATION.md and CONTINUATION.md for exact scope and next work.

## Previous validated checkpoint

# Current: native SDK constructors and four missing base metadata properties

The downstream consumer now uses production Rust emission with actual SDK metadata constructors,
source-checked public new/model APIs, original enclosing Rc identity, declared concrete handles
and casts. All92 properties in the existing four-model projection bind through actual native
getters/setters and inheritance projections; no selected-property fallback remains. Original
wire graph/aliases/cycles, factory initialization, numeric values, policies, bounds, list
factory and distinct live constructor-created models pass. Logical compiler input, projection
selection and family fallback bindings remain explicit source reference/typed facts.

DockableBase metadata adds AllowedDockOperations, AllowedDropOperations, ShowInSelector and
SelectorTitle. Its37 ordered property declarations match the unchanged source schema. Accessors
use original optional contracts and typed errors; SDK tests retain unknown signed flags and
selector edits/null clearing. DockWindow's existing concrete registration uses explicit qualified
MarkupTyped syntax so the scanner exports the real handle. The prior UI-independent context
stays byte-identical after shared test-fixture extraction. Only test-package serde_json build
wiring is added; production manifests/versions/features/framework sources remain unchanged.

Final85675 passes all21 stages with1,929 tests/55 suites. Initial20009 all tests pass but Clippy
finds duplicate fixture module loading; consolidated imports resolve it without lint allowances.
Inventory227/463/1238 of1928 stays unchanged. Native class semantic attributes/hiding/schema and
full catalog coverage, further constructors, remaining enum/flag metadata and sample save/load
remain open. Full original goal active/incomplete/unblocked. See SERIALIZATION/CONTINUATION.

## Previous validated checkpoint

# Current: native inherited property binding without abstract construction

Borrowed property-only compiler options and bind_declared_properties bind actual class or
interface properties independently of constructors/identity/base decisions. Whole-class
binding reuses this path. Explicit project_property_bindings preserves source callbacks,
order/converters/fallibility and live receiver conversion. metadata_instance_type uses a
simple view's declared public export, avoiding private implementation paths. Absolute native
value types now compile in accessor and mixed Result wrapper annotations.

The independent SDK consumer scans RootDock/DockBase/DockableBase/ReactiveBase/ReactiveObject,
checks declared casts and resolves public base conversion functions from actual owner and
return-type facts. Fourteen scalar callbacks compile against original Rc models; source
scalar wire values/order, null omission, unnamed enum values, distinct live models and repeated
setters on the same configured original instance pass. Existing emitted consumers and source
fixtures remain. Four new adapter and two native SDK tests bring the workspace to1,925/55;
final73699 passes all21 validation stages. Only test-package dependencies on existing workspace
packages are added; production manifests, versions/features and framework sources stay fixed.

This is a property callback milestone. Complete native inheritance/hiding/constructor/identity
binding, general exported handle types, complete model/system/ancillary schemas, catalogs and
sample persistence remain open. Inventory227/463/1238 of1928; full goal active/incomplete.
See SERIALIZATION.md and CONTINUATION.md for boundaries and next work.

## Previous validated checkpoint

# Native receiver projection

Native JSON class bindings can now project callbacks onto the same original model handles.
The compiler supplies typed live-receiver and constructor-result conversions; no names or
replacement constructors are guessed. The original constructor executes once and its same
model is extracted. Missing required conversions fail and absent constructors stay absent.
Identity, properties/order, converter/fallibility facts and declared cast targets remain.
Callback expressions capture once at metadata construction; current models are converted
per operation without caching a model/view or copying its state. Custom type codecs need
explicit target metadata, and base hiding/override decisions must already be resolved.

Independent compiled consumers scan class-view declarations and retain original source
wire/aliases, distinct live instances, constructor identity/count and error identity. The
source probe has17 cases, retaining15 old cases and46 prior outputs. Focused suites pass
adapter63/consumer9/runtime36. Final49691 passes all21 stages:1,919 Rust tests across55 suites,
no failed/ignored, strict workspace/release-library Clippy and completion rejection.
SSD logs use ferrodock-receiver-final-; only docs follow green. Original sources and protected
desktop artifacts remain preserved; no dependencies/features/framework changes.

This is receiver adaptation, not complete native actual-model/inheritance discovery or a
full default/backend catalog. Conditional/fallible constructor contracts, remaining schemas,
sample persistence and all original API/lifetime/platform scope remain required. Inventory
227/463/1238 of1928; the full original goal stays active/incomplete.

## Previous validated checkpoint

# Fallible native accessors in JSON bindings

Native Result getters/setters now bind through typed try property APIs, including get-only,
set-only and explicit-converter forms. Existing infallible APIs retain their behavior through
the same implementation. Individual fallibility facts survive compilation; error transport
is explicit Preserve for existing DockSerializationError or Map for a typed foreign adapter.
No callback Result is unwrapped, no error conversion is guessed and no Rust reflection or
alternate engine/model is added. A mixed callback expression is evaluated once when metadata
is built and retains its captured state across operations.

Fifteen unchanged-original serializer cases verify identity/order, partial setter commits,
read-only skipping, write-only omission, converter precedence and persistent callback state.
All46 earlier source outputs remain byte-identical. Independent consumers compile all six
fallible forms, mixed callbacks and foreign mapping, and execute readonly/stateful probes
alongside all existing original-model/golden cases. Ignored members retain priority over
invalid callback contracts. Existing reserved-metadata rejection remains unchanged.

Final59967 passes all21 stages, **1,914 Rust tests across55 suites**, none failed/ignored,
strict workspace/release-library Clippy and the expected rejection of full completion.
Logs use SSD prefix ferrodock-fallible-accessor-checked-. Only documentation follows green.
This removes a native property binding prerequisite; full receiver/base/constructor binding,
complete model/default/backend catalogs and sample persistence remain unfinished. Inventory
227/463/1238 of1928; the entire original goal stays active/incomplete, with source and protected
desktop artifacts unchanged.

## Previous validated checkpoint

# Docking-mode properties in supplied layout contexts

The direct actual-model and emitted JSON contexts now retain original Dock,
PinnedDockDisplayModeOverride, root PinnedDockDisplayMode and FloatingWindowHostMode.
Nullable overrides retain null, named and unnamed values; all enum values use actual source
model accessors. Compiled C# wire order is the authority: root pinned mode precedes window
properties, floating host mode precedes root policy, Dock follows Proportion, and pinned
override follows CanPin. Both contexts match the original expanded root/list bytes and keep
all earlier reference identities, numerical fields, lists and post-load engine behavior.

The source probe adds six checks (44->50), retaining every earlier check. The two expanded
goldens retain all prior properties/order/reference IDs;43 other source outputs are identical.
All21 validation stages pass, with1,909 Rust tests across55 suites, no failed/ignored tests,
strict workspace/release-library Clippy and completion rejection. SSD log prefix is
ferrodock-layout-mode-final-. No behavioral changes follow full green.

This is the existing four-model supplied projection, not full default/backend catalogs or
sample save/load. Complete native actual-model schemas, compiler metadata/accessor discovery,
constructors/casts, other backends and inherited APIs remain open. Inventory227/463/1238 of
1928 and the entire goal remain incomplete. Original engine/models and protected desktop
artifacts are preserved; all scratch stays on SSD.

## Previous validated checkpoint

# Original unnamed docking-mode values

DockMode, PinnedDockDisplayMode and DockFloatingWindowHostMode now retain every signed Int32
value through safe transparent original domain types. Original public names/defaults,
value/hash/storage and checked named-member conversions remain. Raw numeric operations use
from_raw/to_raw; Display keeps original named/numeric formatting. As for window owner/state,
this adaptation of the guide's default closed-enum spelling is needed for faithful source
numeric casts. The same native descriptor generator discovers the actual transparent source
constants and exports explicit typed factories; no guessed constructor or false flags.

Explicit Core JSON metadata adds all three codecs and nullable forms (34 descriptors total).
Original source provides423 seeded value/name/wire rows and102 read cases, with44 earlier
source outputs unchanged. Compiled original XAML/real native consumers preserve unknown root
properties and nullable settings; host precedence and converter fallback match original code.
Source reference914 assertions retains all908 previous checks. DockModeConverter now uses the
original wildcard unset fallback for Center/unnamed values. Native Dock's closed representation
still leaves its unnamed reverse-conversion branch unported and explicitly recorded.

Final86671 exits0: all21 stages pass, with1,909 Rust tests across55 suites, none failed/ignored.
Strict workspace/all-target and selected release-library Clippy, eight standalone suites,
ten normal boundaries, format,24 Python tests, source/theme/inspector checks, six provenance
checks and both generators pass. Full completion remains correctly rejected. Logs use SSD
prefix ferrodock-dock-enum-final-. Only documentation follows green. These files remain partial until
inherited APIs and automatic catalogs are complete. Inventory227/463/1238 of1928; the entire
original port stays active/incomplete. Original engine/models are used throughout; full
model projections, other enums, catalogs, lifetime, platform and accessibility remain open.

## Previous validated checkpoint

# Native enum dependencies in the generated consumer

The payload compiler host now reconstructs twelve native headers: six SDK interfaces,
ICommand and five original enum declarations. The enums are DockMode, PinnedDockDisplayMode,
DockFloatingWindowHostMode, DockWindowState and DockWindowOwnerMode. Canonical paths, source
kind, visibility and locations come from the actual model source. Transparent Int32 values
also use verified actual public static factory signatures. Explicit source IDs unify incoming
references, while old type/property schemas and allocated slots are discarded. System.Enum
and its interface closure remain a foreign dependency; property and nullable links remap.
The original semantic generator does not inspect enum fields/methods, which remain on the
real compiled models. No runtime reflection or replacement engine/model is introduced.

The twelve-schema corruption regression and five-enum inheritance/provenance/link comparison
pass. Final validation passes **1,900 tests across 55 suites**, none failed/ignored, and all
21 sequential stages including strict workspace/release-library Clippy and completion rejection.
Logs use SSD prefix ferrodock-native-enum-catalog-final-. No behavior changes follow full green.

These are explicit native dependency roots in the generated consumer, not full automatic
catalog discovery. System/macro/ancillary types, inherited APIs and complete backend catalogs
remain required. Three closed enum units are corrected from complete to partial because
unnamed numeric-value support is still missing. Inventory227/463/1238 of1928. The entire
original goal remains active/incomplete; source and protected desktop artifacts are unchanged.

## Previous validated checkpoint

# Transparent numeric enum source provenance

The Rust semantic adapter now records normalized single-field transparent storage from the
actual declaration. A logical nonflags Int32 enum represented by a struct requires both
signed i32 storage and an explicit shared typed factory. This preserves the original window
enum types and source identities without false flags or inferred constructors. Named and tuple
fields work. Conditional fields/layouts, unsupported layouts/storage and missing or malformed
factories are not guessed. Failed bindings preserve the original semantic tables.

Three tests cover source layout facts, atomic rejection and both actual open window enum
declarations. The native adapter passes 59 tests. Final driver71621 exits0: all 21 sequential
validation stages pass, including 1,899 workspace tests across 55 suites, no failed/ignored
tests, strict workspace and selected release-library Clippy, eight standalone suites,
ten dependency boundaries, format, 24 Python tests, source/theme/inspector checks,
six provenance checks and both generators. Logs use ferrodock-transparent-enum-final-.
Automatic native/default enum catalogs,
inherited APIs and remaining compiler/system facts are still required; no source is promoted
complete. Inventory remains 230 ported / 460 partial / 1,238 pending of 1,928.

The preceding shared numeric XAML change passes all 21 validation stages, including 1,896
workspace tests and strict workspace/release-library Clippy. Original reference checks expand
900 to 908; isolated framework tests pass 4,854 with one existing ignored diagnostic. Both
compiler hosts use explicit factories through the shared emitter; source XAML overflow
narrowing and separate JSON range behavior are preserved. Patches replay against published
archives, with versions, dependencies and features unchanged. Full goal remains incomplete.

## Previous validated checkpoint

# Open window enum values and complete window JSON fields

DockWindowOwnerMode and DockWindowState now retain all signed Int32 values, preserving
source names/defaults, value equality/hash and four-byte storage. Raw casts use from_raw/
to_raw; existing TryFrom remains a checked named-member helper. Display matches original
named/numeric output. No unsafe Rust or alternate docking model is introduced.

The guide's default closed Rust enum spelling cannot safely retain undefined values.
These two original types therefore use transparent Int32 value structs with the same named
constants and source semantics. Native descriptors retain enum kind and exact member tables;
the declaration bridge reads actual source constants, validates signed storage and emits
compiled casts over those original types. Runtime/nullable conversions preserve all values.
Shared compiled XAML emission of undefined numeric literals, automatic compiler provenance
for this representation and complete inherited System.Enum APIs remain unfinished. The two
source units are corrected from ported to partial to retain those explicit requirements.

Core JSON metadata expands to28 descriptors, adding owner/state codecs, original window
options and nullable dependencies. All four option properties use original constructors/
accessors and ApplyTo. The real-model projection now includes all13 persisted DockWindow
properties; direct and emitted contexts retain unknown state/owner, presentation flags and
shared parent identity. Other model projections/default catalogs remain incomplete.
The unchanged-original reference adds7 assertions,282 value/name/wire rows,50 read cases and
two goldens. Model assertions expand41->44. A settings test now requires numeric roundtrips
for the two corrected types while retaining explicit gaps for other plain enums.

The structural verifier now audits pure enum definitions despite multiple metadata/test
targets, including strict transparent Int32 storage, exact names/values and duplicate/member
checks. Three new Python regressions pass. MDI resize member records now list actual source
names for this stronger audit; its existing broader flags gap remains partial.

Full validation: **1,894 Rust tests /55suites**,zero failed/ignored; all21 stages pass, including
strict workspace/debug/all-target and selected release-library Clippy, eight standalone
packages, ten normal boundaries, format,24 Python tests, source/theme/inspector checks, two
generators and six framework provenance guards. Inventory230/460/1238 of1928; full completion
is correctly rejected. Only documentation and a bridge-comment clarification follow green.
Entire original goal stays active/incomplete/unblocked. Full catalogs/sample persistence,
other enums, compiler/serializer/backend and broad lifetime/API/platform/accessibility/
performance contracts remain required. Actual desktop and original sources preserved.

## Previous validated checkpoint

# Get-only values, shared options and signed flags JSON

The explicit Core metadata collection now contains 21 descriptors, adding DockPoint,
DockManagerOptions, DockingWindowState, DockOperationMask, DragAction and nullable forms.
Get-only value properties serialize original getters and skip read input; DockPoint retains
its source default-value construction instead of inferring its parameterized constructor.
Shared manager options use the actual original default true, setter and object identity.
The numeric enum codec retains every signed Int32 bit, checks source tokens/ranges and
remains independent of custom Int32 codecs. No runtime reflection or alternate model is used.

CoreValueReference runs unchanged original models/serializer and adds ten assertions,
490 read/replay cases and four wire goldens. Direct tests match all cases, including 128
seeded values per flags type. A separate roundtrip golden preserves original get-only point
semantics. The real-model probe now has 41 assertions; runtime and generated contexts also
persist DockingState and concrete source/target masks, then use original engine mask rules.
Optional restrictions stay on concrete class descriptors and are excluded from interface
fallback schemas. Source state classifies initialized documents as Document (4).

Full validation passes **1,886 Rust tests across 55 suites**, none failed or ignored, and
all 21 sequential stages. Workspace/debug/all-target and selected release-library strict
Clippy, eight standalone packages, ten dependency boundaries, format, 21 Python tests,
source/theme/inspector checks, two generators and six framework provenance guards pass.
Inventory remains 232 ported / 458 partial / 1,238 pending of 1,928; full completion remains
correctly rejected. Entire original goal stays active, incomplete and unblocked.

Plain enums still need undefined numeric-value support; automatic codecs, all default/
application/backend catalogs and real sample save/load remain required. Full original API,
lifetime/cycle collection, other serializers, platform/accessibility and performance scope
remain unchanged. No production/test edit follows full green. Original parent/sibling/cache
sources and actual desktop executable/app/window remain preserved; scratch/artifacts on SSD.

## Previous validated checkpoint

# Core bounds and capability-policy serialization

The explicit Core JSON metadata collection now retains actual DockRect value copies and
original DockCapabilityPolicy/DockCapabilityOverrides constructors, six nullable flags and
base/interface reference casts. HasAnyOverride is excluded. Mutable value descriptors use
original fields without shared wrappers or reference IDs. Class reads now reject reserved
dollar-prefixed metadata even when a property declares that name. Supplied contexts gain
no implicit model catalog; generated contexts can explicitly compose dependency metadata.

The actual ReactiveUI projection now includes pinned bounds, six base capability flags,
overrides and shared root/dock policies. Both direct and compiled generated-context tests
match original wire order and reference IDs. After InitLayout, the original capability
resolver verifies override precedence and mutation of the shared policy instance.
CoreModelReference supplies 10 original assertions, 49 read/replay cases and six wire goldens;
JsonModelReference expands from 33 to 38 assertions. Only the reference fixtures change;
original C# models, serializer/options/resolver and generator remain unchanged.

Full validation passes **1,883 Rust tests across 55 suites**, zero failed or ignored, and
all 21 sequential stages. Strict workspace/all-target/debug and selected release-library
Clippy, eight standalone packages, ten normal dependency boundaries, format, 21 Python tests,
source/theme/inspector checks, two generators and six framework provenance guards pass.
The completion gate correctly continues to reject the unfinished full port. Inventory
remains 232 ported / 458 partial / 1,238 pending of 1,928; no source unit promoted complete.

Default/application catalogs, sample save/load, complete compiler adaptation, other codecs/
backends, cyclic lifetime, full API/platform/accessibility and performance parity remain
required. Entire original goal remains active, incomplete and unblocked. Scratch/artifacts
stay on /Volumes/SSD/temp/ferrodock-port; actual desktop executable/app/window preserved.

## Previous validated checkpoint

# Atomic native dependency allocation and actual command source

A production atomic importer now remaps the selected foreign dependency graph and allocates
native IDs independently of old slots. Replaced headers/properties are excluded. Native binding
uses real declarations in private working state, with forward foreign links and final graph
validation. Incomplete/invalid bindings and cross-boundary overrides return errors without
publishing placeholder headers or mutating source facts.

The payload host now constructs six SDK interfaces and the actual locked published ReactiveUI
ICommand in seven fresh slots, then its nine native application schemas. System/ancillary facts
remain explicit C# reference inputs. Published source lookup supports the current single crates.io
package; general Cargo/cfg/macro/export/incremental integration and full catalogs/persistence
remain required. Full workspace1879tests/55suites and all21 validation stages pass, including
strict workspace/all-target/debug and selected release-library Clippy. Consumer4/adapter56
pass; inventory232/458/1238 of1928 unchanged. Entire original goal active/incomplete.
Desktop executable preserved.

## Previous validated checkpoint

# Native required Dock interface schemas

The semantic adapter now resolves actual single dyn Trait carriers and validates interface
base handles as interface-closure edges. The payload host rebuilds the six required Dock
contract headers and81 ordered property schemas from existing SDK declarations. Five tests
compare original member/edge/ignore/selection facts, corrupt discarded reference schemas,
retain missing-system-attribute DSTJ004 and reject unsupported dyn/base forms. SDK root
property order matches the original in both scanned and compiled metadata.

Full workspace1872tests/55suites and all21 validation stages pass, including strict workspace
all-target/debug and selected release-library Clippy. CoreSDK62/consumer4/adapter49 pass. Residual dependency slots, ancillary types and ICommand/IgnoreDataMemberAttribute
facts remain explicit C# reference inputs. Full native dependencies, exhaustive discovery,
codecs/catalogs and sample save/load remain unfinished. Entire original goal active/incomplete;
no source unit promoted complete. Desktop sample protected.

## Previous validated checkpoint

# Native application allocation and metadata-declared discovery

The atomic batch binder now allocates fresh application IDs, supports forward/mutual property
references and forward closed generic arguments, and commits only after every declaration and
argument closure validates. Dependencies must precede inherited/containing/interface headers;
override rows remain explicit. No placeholder schemas or runtime instances are introduced.

The payload consumer discards all C# application schemas/slots/registrations/discovery inputs.
Actual metadata-declared Rust classes and actual registered closed constructions supply all
nine application IDs and source/name discovery. Only the transitive required-contract/builtin
dependency closure remains from the explicit C# test reference, with all IDs remapped.
Complete native dependency construction, exhaustive discovery, codecs/catalogs and sample
persistence remain required. Full workspace1867tests/55suites, consumer4/adapter44 and all21 validation stages pass,
including strict debug/all-target/selected release Clippy. Inventory232/458/1238 of1928
unchanged. Entire original goal remains active/incomplete; desktop executable preserved.

## Previous validated checkpoint

# Native closed generic templates and semantic symbols

One actual Rust generic metadata template now instantiates the original integer/string payload
examples through parsed type substitution and shared normalization. Constructors/accessors are
the original typed generic functions; per-type getter/schema branches are removed. Closed
semantic headers validate actual declaration/argument arity, recursive closure and logical
identity before commit. Parameterized public/stated paths retain export spelling. Unknown
cfg/const/lifetime/inferred/macro/ambiguous associated decisions remain explicit errors.

All9 payload schemas now native; dependency/type-slot/discovery seed remains snapshot-based.
Complete native compilation, nested/captured generics and constraints/projections/aliases/
access/cfg/macro/incremental integration, original codecs/catalogs and sample persistence stay
required. Full workspace1859tests/55suites and all21 stages pass; inventory232/458/1238 of1928
unchanged. Entire original goal active/incomplete. See SERIALIZATION.md and VALIDATION.md.

## Previous validated checkpoint

# Native ordinary type/property semantic symbols

The adapter now constructs non-generic semantic headers and declared property symbols from
actual shared metadata/Rust provenance, retaining original ignored-member/static/command/
override/hiding inputs. Logical serialization identity remains explicit, independent of markup
aliases. Resolved source abstraction/access/base/containing/interface/override facts stay explicit;
unresolved/unsupported inputs fail atomically. Existing type rebinding requires future dependent
recomputation by the incremental host. No alternate engine or copied model state is introduced.

Seven ordinary application classes now replace old C# class/header/property schema with native
symbols and preserve original wire/identity/engine checks. Snapshot seeds still provide type
slots/dependencies and two closed generic examples; complete native compilation/default catalogs
and sample persistence remain required. Full workspace1853tests/55suites and all21 stages pass.
Inventory232/458/1238 of1928 unchanged; entire original goal remains active/incomplete. See
SERIALIZATION.md/VALIDATION.md for exact contracts and remaining source/lifetime/platform scope.

## Previous validated checkpoint

# Actual Rust declaration facts and canonical dependency identities

The build adapter now reads actual Rust type/trait declarations and implementations through
the shared scan's file/module graph, retaining declared visibility, generic parameters,
constraints/defaults, where clauses, conditional context and source locations. Known public
reexports resolve through the shared source/dependency ModelSet. Actual docking-contract
supertraits and application definitions have tests; the compiled consumer now uses native
visibility, generic arity and source locations. Production test-only cfg rules are shared
with activation discovery, while unknown target conditions remain unevaluated.

Full workspace1844tests/55suites and all21 validation stages pass. Inventory232/458/1238 of1928
stays unchanged; generator remains partial. Complete source abstraction/identity/property/
inheritance/closed-generic symbol construction, export/emission accessibility, cfg/macro/
incremental integration, catalogs and sample persistence remain required. C# snapshots still
supply logical facts; this step does not make them production metadata. See SERIALIZATION.md
and VALIDATION.md. Original engine/backends/serializers/lifetime/platform scope stays intact.

## Previous validated checkpoint

# Native activation and registration source for generated JSON contexts

Original attribute constructors/type getter now have shared Rust reference classes and
seven C#-backed tests. Type identity is native TypeId, including null; complete shared
TypeInfo/CLR Attribute/runtime assembly metadata remains deferred. Crate-scope marker
macros retain typed constructors without runtime registration or class construction.
The build adapter resolves actual qualified/aliased macro imports, preserves order/spans,
rejects invalid scopes/duplicates/unselected cfg and feeds original semantic validation.

The compiled consumer reads9 registrations from its actual Rust source fragment, replacing
the C# fixture attribute inputs. Semantic class/accessibility/generic/cast facts remain
explicit; this is not complete automatic compiler discovery/default catalogs or sample
save/load. Full workspace1837tests/55suites and all21 validation stages pass. Source7
attribute assertions and all existing serializer corpora pass. Inventory232/458/1238 of1928:
2 additional source units are partial. Full original goal stays active/incomplete. See
SERIALIZATION.md for usage, current boundaries and remaining exact-port requirements.

## Previous validated checkpoint

# Shared declaration bindings for generated JSON metadata

A separate build-time adapter now converts shared FerroUI TypeModel declarations into the
existing emitter's typed constructor/property/converter bindings. It preserves declaration
order, effective JSON names, unconditional ignores, get-only/set-only members, explicit
converter precedence and public/crate-private/typed-function call forms. It links no runtime
or UI. Seven source application payload classes compile and execute through this path against
unchanged C# wire goldens. Full workspace1828tests/55suites and all21 validation stages pass.

This is partial declared-member adaptation. Complete compiler semantic facts/discovery,
activation macros, target cfg, inherited binding/casts, closed generic construction, full
value/constructor/attribute contracts, default/application catalogs and sample save/load
remain required. Compiler-provided identity/object/converter facts stay explicit. Original
model reachability is preserved; cycle collection remains open. Inventory232/456/1240 of1928
and the entire original goal remains active/incomplete. See SERIALIZATION.md and VALIDATION.md.

## Previous validated checkpoint

# Executable Rust context/resolver emission over original model accessors

The generator now emits compilable Rust from its semantic plan and resolved compiler bindings:
class/custom codecs, source property order/ignore rules, nullable/object/get-only/set-only/
explicit converters, reference casts, ancestry/policy tables, context resolver and factory
list overloads. A dedicated consumer builds and executes two generated contexts. It matches
actual ReactiveUI RootDock/DocumentDock/Document/DockWindow golden output and runs the original
Factory.InitLayout after load. Application payload fixtures compile and preserve generic,
reference-scope, converter and duplicate-tag behavior. No alternate engine or copied model
state is introduced. Bindings remain EXPLICIT LIMITED projections; automatic discovery/full
catalogs and ordinary sample persistence are not complete.

Runtime metadata now supports exact runtime selection, base fallback, unique nearest compiled
ancestor and ambiguity failure, with abstract/interface schemas lacking fabricated constructors.
Source15write/16read cases,5 graph checks and a holder golden back that behavior. Source payload
coverage grows20assertions/25reads/8goldens; compiler coverage19cases/152name checks. Complete
runtime/cast metadata production, validation/constructor/CLR errors/timing and cycle collection
remain deferred. Source schema field/value/attribute coverage is still partial.

Targeted consumer4/runtime21/compiler4 and strict changed-package Clippy pass. Full workspace
retry passes1820tests/53suites; all20stage checks pass. First attempt hit actual
Skia archive ENOSPC, resolved by retiring16 logged owned test harnesses with manifest. Original
sources/other projects and the actual desktop executable/app/window remain preserved. Final
validation is recorded in VALIDATION.md/CONTINUATION.md; full goal stays active/incomplete.

## Previous fully validated checkpoint

# Compiler semantic selection for generated JSON metadata

The new compile-time library ports GenerationModelBuilder over explicit immutable compiler
facts, without production dependencies or runtime model inspection. It preserves registration/
activation/missing-contract order, local discovery and nested/friend/generic accessibility,
inherited and hidden/overridden property ignores, interface unions, context closure/naming,
seven polymorphism tables, collision filtering and all four source diagnostic messages/spans.
Full-name construction retains qualified nested generic/array/tuple arguments and ordinal order.

The unchanged original C# generator supplies17 compiler fixtures; public Roslyn inputs are
exported independently from its emitted output plans. Rust matches those outputs and136 runtime
name checks (8shapes across17 snapshots), including unsigned project-assembly arguments. Three
compiler tests and malformed consumed symbol-link/cycle errors cover this stage. This is not
an executable Rust generator: the syntax/shared-metadata adapter, incremental host/macros,
codec/context/resolver emission and full default/model catalogs remain required.

Final locked workspace **1812 tests/51 suites** and all19 validation stages pass. Inventory
remains232/456/1240 of1928; the already-partial generator now includes this semantic stage.
Full completion correctly rejects; the entire original goal remains active. No production/test
change follows final green validation. No retained package/version/feature change or actual
desktop rebuild/restart. All temp files stay on SSD; final preservation checks accompany this
record. Runtime cyclic lifetime and all previously deferred engine/API/platform scope remain.

## Previous fully validated checkpoint

# Source-generated object-payload helper and property-local codecs

The emitted helper's runtime now uses compiled payload TypeId/name registrations, original
shared handles and source fresh reference scopes. Each object property decodes independently;
internal self refs and the outer typed-property scope retain their recorded identities. Raw
JSON preserves lexical numbers/duplicate tags; late known tags decode after removal, while
unknown/multiple string tags remain raw. Explicit/read-only property codecs and nullable
wrappers retain original accessors and do not replace global object metadata. Non-object
converter results, unregistered values and reserved typed metadata fail as in the source.

The reference runs the unchanged original1662-line incremental generator as an analyzer.
Seventeen assertions,25 read/replay records and7 wire goldens come from its emitted resolver.
Four new Rust tests match those cases. This is a partial runtime translation, not a Rust
compiler frontend: activation/registration, discovery, diagnostics, code emission and complete
default/model catalogs remain required. JsonDocument ownership and complete converter/lifetime/
performance semantics are also open. No alternate docking engine or model cache is introduced.

Locked workspace **1809 tests/49 suites** and all18 validation stages pass. Inventory is now
232ported/456partial/1240pending of1928, with the generator source newly partial for emitted
runtime behavior only. Full completion correctly rejects; the entire goal stays active.
No retained source/Cargo dependency or actual desktop rebuild/restart occurred. Final desktop
preservation checks and source/staging checks accompany this checkpoint; all temp work on SSD.

## Previous fully validated checkpoint

# Numeric JSON metadata and model size/proportion roundtrips

The generic JSON serializer now has builtin descriptors for Single/Double and all eight
signed/unsigned8/16/32/64-bit integers. The existing locked Ryū dependency supplies shortest
digits; source general-format spelling, named nonfinite strings, signed zero and measured
read/range failures match1362 original C# wire/read records. Two Rust tests consume every
record and verify exact finite roundtrip bits. No numeric algorithm was copied.

The actual ReactiveUI model projection now includes proportion/collapsed proportion,
constraints and floating-window bounds, with33 source model checks and two wire goldens.
Original engine InitLayout still uses the same decoded instances. Source setters keep+0
when an incoming-0 compares equal, while NaN-initialized dimensions retain-0. This remains
an explicit limited projection; generated/default catalogs, full field/family coverage,
remaining scalar/enum domains and original cycle collection are incomplete.

Locked workspace **1805 tests/49 suites** and all18 validation stages pass. No production/
test change follows that green run. Inventory remains232/455/1241 of1928, full completion
correctly rejects, and the entire goal stays active. Original source and retained dependency
implementations are unchanged. Actual desktop binary preservation is checked at checkpoint;
all temporary work remains in owned SSD storage.

## Previous fully validated checkpoint

# Model reference casts, configured lists and base fallback

The supplied-metadata JSON pipeline now retains actual model references across declared
concrete/interface types, registered polymorphic discriminators and shared IList identities.
The list-type/resolver constructor supports compiled observable, plain and custom creators.
Actual ReactiveUI RootDock/DocumentDock/Document/DockWindow projections match two original
wire goldens and initialize through the original factory after load. The explicit source
projection has limited fields: default/generated catalogs and full sample persistence remain
unimplemented. Twenty-eight source cases cover aliases, creators and retained model links.

Owner, RootDock.Window and DockWindow.ParentWindow retain source references after decode;
weaker links had lost reachable objects when the read context was dropped. This restores
source reachability directly in the model rather than adding serializer caches. Rust Rc
cycle collection remains absent, so cyclic lifetime parity is incomplete. Existing snapshot
and release tests explicitly clear graph cycles to isolate their own contracts.

Base fallback serialization now enters the same object-depth guard as other class codecs.
A source probe measured five depth/getter cases; the failing Rust regression originally called
getter65 before DOM rejection. The fix stops before that getter. Two source fallback cases
and a wire golden cover self-reference and an ignored unknown discriminator with an actual
base constructor. Automatic subtype discovery, nearest-ancestor ambiguity and full metadata
validation remain deferred.

The locked workspace passes **1,803 tests /49 suites** with all18 validation gates, including
strict debug/all-target and selected release-library Clippy. Twelve serializer tests pass.
The inventory remains232 ported/455 partial/1241 pending of1928; full completion correctly
rejects. No dependency versions/features or retained framework sources changed. The actual
desktop executable's size, mtime and SHA256 remain identical; no rebuild/restart occurred.
All temporary work stays in owned SSD storage. The full original goal remains active.

## Previous fully validated checkpoint

# Actual resolver-based JSON serializer and shared failures

FerroDock.Serializer.SystemTextJson now implements the source resolver-based constructor
and all four generic methods. Supplied compiled metadata invokes actual getters/setters;
no reflection, copied layout tree or second docking engine is used. Reference IDs are
per call, metadata caches belong to the original options instance, and classes register
before recursive setters so cycles keep original instance identity. Readonly properties
serialize; null values are distinct from failures and skipped before property conversion.
Ordered DOM values retain duplicate names and lexical numbers. Default escaping and the
recorded payload indentation match the unchanged source wire output.

Load/Save retain original streams, UTF preambles and recorded close/failure order. A close
failure overrides an earlier body failure; invalid constructor streams are not closed;
Save serializes before validating the target stream. Core IDockSerializer and workspace
operations now return SerializationResult. DockSerializationError retains the same cause;
Capture/Restore/TryRestore propagate it before committing workspace state. All existing
consumers retain their original generic type, handles and behavior.

Seven serializer tests and one new workspace error test pass. Source references establish
900 factory/workspace assertions,37 stream checks,13 JSON scenarios,12 failure cases,
six string goldens and a full payload golden. Workspace **1,797 tests / 49 suites** and
all **18 validation gates** pass, including the standalone JSON boundary. Inventory is
232 ported / 455 partial / 1241 pending of1928: the actual serializer source is newly
partial. Full goal remains active/incomplete. No production change followed the green run.

Default/list constructors and factories, original model catalogs and generator/attributes,
polymorphic interfaces/casts/fallback, complete primitive codecs and model roundtrips,
full metadata validation, stream buffering and CLR/string/lifetime/performance remain
required. The original sample cannot yet use this partial library for layout roundtrips.
Other serializers/backends/platform scope remain intact. Test profile retains line tables
to bound debug-symbol size; actual desktop dev profile/executable/window are preserved.

The additional guarded controls run92422 also passes all4,428 tests with the new
serde_json/raw_value dependency feature and line-tables-only profile. Retained sources
match their exact provenance hashes; no dependency implementation patch was changed.

## Previous fully validated checkpoint

# Shared serializer stream contracts and source JSON audit

The serializer audit exposed a missing prerequisite: borrowed native Read/Write/Seek
could not express source stream identity or closure. Load/Save now receive the original
optional Rc stream handle. The shared Stream contract includes capabilities, length,
position, read/write/seek/flush, Close and IDisposable. Concrete implementations retain
their own disposal policy. The memory implementation keeps the original fixed buffer or
expandable store, logical length/position, closed aliases and post-close ToArray. StreamIo
forwards native I/O to that same object and does not close on drop. No docking engine,
layout snapshot or mirrored stream state replaces the source objects.

Five new Rust tests match37 C# memory/interface assertions and exercise original nullable
stream dispatch, generic declared types, native adapter identity and custom close/reentry.
The existing15 workspace tests also pass with the shared stream contract. Full workspace
**1,789 tests / 47 suites** and all **17 validation checks** pass. Inventory remains
232 ported / 454 partial / 1242 pending of1928; the full goal is active and incomplete.
No dependency or actual desktop executable/bundle/window change occurred.

The separate13 JSON audit cases remain SOURCE-ONLY. Compiling the unchanged original
serializer proves readonly properties are included, null properties omitted, self-reference
IDs retained, UTF-8 BOM emitted on Save, and original streams closed on success/failure.
Empty Load throws, contrary to an older source test expectation. Production serializer
implementations, generated/default metadata and model graph roundtrips remain absent.
See SERIALIZATION.md for requirements; do not replace them with a layout-only format or
runtime reflection. Full framework stream metadata/surface, non-memory resources, text
reader/writer semantics, generic error propagation and other backends remain required.

## Previous fully validated checkpoint

# Query cursor disposal in the original model engine

The UI-independent query engine now owns its cursor's Current, MoveNext, Dispose and
Reset contracts. Rust iteration and native IEnumerable/IEnumerator views use that same
cursor state. Native metadata keeps its original names, handles, identity and IDisposable
cast; the separate native RefCell iterator/advancement state has been removed.

The original Find state machine preserves its active cleanup scopes. Dispose during
the first direct root predicate has no effect before the first foreach scope. Child
cleanup can clear the pending field so a matching predicate yields actual null; a later
advance fails at the source enumerator access. Nested and all-controls cleanup can clear
the live inner cursor before its Current is read, preserving the previous outer Current
on failure. Recursive MoveNext returns false while a cursor is running. Disposal before
first advancement prevents enumeration; Reset remains unsupported. No RefCell borrow
crosses predicate callbacks, collection advancement or custom cleanup.

Thirty-two recorded original C# traces cover direct/all-controls traversal, root/child/
nested predicates, true/false matches, recursive advancement and repeated disposal. Model
and both native factory views match that corpus. Three model tests and one native test
accompany 34 new source assertions. Workspace **1,784 tests / 47 suites**, original C#
**893 assertions** and all **17 validation checks** pass. Only test formatting changed
after the full workspace run. Inventory remains 232/454/1242 of1928; the whole goal is
active, incomplete and unblocked. Dependencies and the actual desktop sample are preserved.

Custom collection enumerator cleanup/disposal reentry, CLR exceptions and lifetime,
complete generic/collection/delegate metadata and allocation/performance parity remain
unverified beyond these scenarios. Serializers, other model backends, remaining API and
service contracts, plus the full original desktop/platform/input/accessibility scope
remain required. The trace corpus does not prove complete cursor or full port parity.

## Previous fully validated checkpoint

# Nullable search/query contracts and cursor advancement

FindDockable and both fixed Find overloads now retain actual nullable roots and
predicate callables through the original factory, query engine and both native views.
Immediate search invokes the predicate before accessing the incoming dock; lazy search
performs no work until advancement. A matching null root yields a real null once before
the later source list-access failure. Empty all-controls searches never invoke an absent
callable. Existing callers retain their original root and callable identities in Some.

The window search loop preserves separate live Layout reads for its guard, predicate,
return and recursive virtual call. A callback-cleared layout reaches the actual override;
a null window element still fails at the original Layout access. Find remains fixed,
while FindDockable remains virtual, matching the C# engine.

Source probes exposed a native cursor RefCell borrow failure during recursive MoveNext
from a predicate. A per-cursor advancement guard now returns false and retains previous
Current during that callback, matching root and child source scenarios. The original
iterator remains in use. Dispose inside callbacks and broader reentry/lifetime behavior
remain unverified; this does not establish complete cursor parity.

Nine model tests and four native tests accompany 34 new original C# assertions. The
locked workspace passes 1,780 tests in 47 suites; the C# reference passes 859 assertions.
All 17 validation checks pass. Inventory remains 232 ported / 454 partial / 1,242 pending
of 1,928 files. Dependencies and the actual desktop executable/bundle/window are
preserved. The full goal remains active, incomplete and unblocked.

Remaining service/manager/state contracts, the public attribute class, complete generic,
collection, delegate and constructor metadata, serializers, other model backends and
original platform/input/accessibility/lifetime/performance scope remain required.

## Previous fully validated checkpoint

# Shared nullable window collections and lifecycle contracts

IRootDock.Windows now carries actual optional window references through the original
shared ListHandle, concrete reactive root, generic factory list creation, native Core/
backend descriptors, IList operations and native UI ItemsSource conversions. Null entries
remain null; no dummy window, filtered collection, mirror state or alternate engine is
introduced. Existing nonnull callers retain the same model/list references in Some.

Add/Insert/Remove/CloseWindow and both InitDockWindow slots accept nullable incoming
root/window values through the original factory and both native views. Negative insertion
returns before root access; positive bounds failure keeps an allocated list. Null addition
commits its collection payload and added event before virtual initialization. Callbacks
may replace the live list while initialization keeps its original arguments. Optional
host creation sees the actual null window before the explicit-host overload; the default
path fails at the source window.Id or Host access. Protected collection-root resolution
also retains nullable inputs/results and can forward null to an actual Add override.

Init preserves separate live Layout argument evaluations: the first argument may be null
while the subsequent Layout.Owner comes from another layout. The later null owner access
still fails before virtual Init. Traversal consumers retain original first-null failures
and earlier side effects; Show iterates live windows, Exit snapshots before Save/Exit.
The Core SDK now exports IDockWindowHostFactory with its actual method and factory cast.

Eleven model tests and five native consumers match40 new unchanged C# assertions. Native
coverage includes both factory views, host identity and compiled/runtime Count, indexer
and ItemsSource bindings. Original compiled IList indexers remain stale until rebind.
Workspace1,767 tests/47 suites, C#825 assertions and all17 validation checks pass. A final
private type-alias cleanup only follows the full workspace run; production behavior and
representation are unchanged. Dependencies and actual desktop executable/bundle/window
are preserved. Inventory stays232/454/1242 of1928; the full goal remains ACTIVE/incomplete.

Nullable search/query roots, remaining live layout forwarding, service/manager arguments,
public attribute class, complete metadata/generics/delegates, serializers/other backends,
CLR/lifetime and the full original platform/input/rendering/accessibility/performance
scope remain required. This checkpoint does not complete the window stack or full port.

## Previous fully validated checkpoint

# Nullable window-option guards and live application

DockWindowOptions.ApplyTo now accepts a nullable borrowed window and rejects null
before any property write, retaining the original window argument message. Protected
ApplyWindowOptions accepts nullable window handles through the base and actual hook.
Null options return before inspecting the window; nonnull options delegate to ApplyTo.
Overrides can accept null or call the original base body with the same option object.

Original OwnerMode, ParentWindow, IsModal and optional ShowInTaskbar setter order is
preserved. Real reactive setters may change the same options during callbacks; later
reads use those updated values. Tests verify parent replacement, modality changes,
cleared taskbar override, notifications and source option/window identity. Ordinary
callers wrap their original borrowed/shared handles. Native ApplyTo retains its existing
argument-error bridge and accepts both raw and canonical optional window arguments.
No dependency change, copied model, option snapshot or replacement engine is introduced.

Four model tests and two native consumers accompany14 new unchanged C# assertions.
Workspace1,751 tests/47 suites and C#785 assertions pass. All17 validation checks pass,
including strict debug/release Clippy, standalone boundaries, format and generators.
A test-only err().expect() style fix followed the passing workspace run; production
remained unchanged and the remaining validation checks were resumed successfully.

The two audited option units return to ported status after their missing contracts
were implemented and verified:232 ported/454 partial/1242 pending of1928. Full original
goal remains ACTIVE, incomplete/unblocked. The next coherent work is nullable shared
window collections and Add/Insert/Remove/Close/Init window lifecycle contracts. Other
backends/serializers, complete metadata/generic/delegate support, CLR/lifetime and full
original platform/input/rendering/accessibility/performance scope remain required.

## Previous fully validated checkpoint

# Nullable focus, selection and window activation

SetFocusedDockable's dock, InitActiveDockable's owner and fixed ActivateWindow's
item now accept runtime null through the original factory, generated dispatch and
both native factory views. Protected ShouldFocusActiveDockable accepts both nullable
inputs; its source default still returns true. Nonnull callers retain the same model
handles. The private SetIsActive helper preserves the source null type-pattern no-op;
focused-item dereferences that genuinely fail in C# remain in place.

InitActive retains active event, initialization, OnSelected, protected policy, focus
and final sibling selection order. A null owner fails at final VisibleDockables access
when earlier overrides return; original initialization and selection effects survive.
OnSelected can change policy or abort before the later calls. Null focus/activation
inputs reach virtual FindRoot, which can return no root without invoking defaults.
Activation rereads root.Window after the actual host SetActive callback and raises the
original event with the cleared or replacement window. Split/whole-pane floating pass
the live optional window layout to focus rather than rejecting it before dispatch.

Six model tests and two native consumers accompany21 unchanged C# assertions. Workspace
1,745 tests/47 suites and C#771 assertions pass; the sequential17-check driver passes.
A final stronger test retains the original owner before initialization, and all six
focused tests plus strict all-target Clippy pass again. No production change followed
the full workspace run. All existing desktop executable/bundle/window and dependencies
are preserved; temp artifacts use the SSD.

An audit corrects WindowOptions and DockWindowOptions from ported to partial: protected
ApplyWindowOptions still cannot receive null window with null options, and ApplyTo
cannot represent the source null-window ArgumentNullException path. Inventory is now
230 ported/456 partial/1242 pending of1928. This records existing missing contracts,
not a runtime regression. The full goal remains ACTIVE, incomplete and unblocked.
Window collection/null-element and initialization contracts, serializers/other backends,
complete metadata/generics/delegates, CLR/lifetime and all original platform/input/
rendering/accessibility/performance scope remain required.

## Previous fully validated checkpoint

# Nullable floating/window factory overloads

FloatDockable, FloatAllDockables, CreateWindowFrom and SplitToWindow overloads now
accept nullable items/docks through IFactory, generated FactoryImpl and both native
factory views. Their original wrapper calls retain virtual overload dispatch, options
and coordinates. Default floating still resolves operation context before item access;
a custom options override can handle actual null before that default path runs.

Plain CreateWindowFrom(null) returns no window before root lookup/creation, as does an
unsupported source classification. Its options overload first invokes actual nullable
PrepareWindowOptionsForDockable, then virtual plain creation, and applies options only
for an actual result. Preparation short circuits null/noncapturing/already-parented
options; otherwise it passes null to virtual FindRoot and captures the original parent.
That options mutation survives a null creation result. Prepare can abort or mutate the
same options before creation; null-options calls still reach the actual Apply hook for
an actual window. Source parent/window/options identity and applied flags are retained.

Split keeps root lookup before preparation/removal, forwards nullable item through
actual Remove/undocked/CreateWindowFrom overloads, and retains source collapse policy.
The ordinary nonnull float coordinate/size fallback and exact dock/item/options handles
are also checked. No fake null model, alternate engine, mirror list or dependency change.
The sample override forwards the same Option to base and keeps conditional demo title;
its actual desktop executable/bundle/window are preserved.

Eight model tests and three native consumers accompany27 new original C# assertions.
Workspace1,737 tests/47 suites, C#750 assertions and all17 validation gates pass. Native
coverage includes all eight overload slots, default null behavior and real ownership
capture/application with the same original objects. Inventory stays232 full/454 partial/
1242 pending of1928. Full original goal ACTIVE/incomplete: focus/activation and other
factory/service/manager slots, remaining protected/window lifecycle contracts, other
backends/serializers, complete metadata/generics/delegates, CLR/lifetime and full original
platform/input/rendering/accessibility/performance scope remain required.

## Previous validated checkpoint

# Nullable hidden-dockable hide/restore lifecycle

HideDockable and RestoreDockable now accept actual nullable dockable references
through IFactory, generated FactoryImpl and both native metadata views. Hide retains
source membership/pin cleanup, a second virtual root lookup and hidden-list creation
before incoming Owner access. A nullable pin hook can change the root between queries;
the original pin list remains unchanged when that custom hook does not remove it.

Restore returns before item access when no root/list/null membership exists. When
null is present it removes the actual null slot and raises the original restored event
before OriginalOwner access fails. Collection payload/index and earlier list mutations
remain committed. Observer replacement of the hidden list is retained. Nonnull hide
uses the live hidden list after removed-event reentry; restore rereads OriginalOwner
after its event, allowing the original observer-selected destination with the same model.

ID helper result guards now mirror the original not-null checks; their lazy predicates
still fail at the first null.Id. Native tests use actual DockControls with initialization
disabled, matching the unchanged C# setup, and keep later matches untouched. No null
placeholder, copied model, mirrored list, callback filtering or alternate engine added.

Six actual-factory model tests and three native consumers accompany27 new C# assertions.
Workspace1,726 tests/47 suites, C#723 assertions and all17 validation gates pass. Native
canonical optional declarations and original raw nonnull argument handles both pass.
Inventory remains232 full/454 partial/1242 pending of1928; full goal ACTIVE/incomplete.
Floating/window and other factory/service/manager inputs, serializers and other model
backends, complete metadata/generics/delegates, CLR/lifetime and full original platform/
input/rendering/accessibility/performance scope remain required. Original/sibling source,
retained dependencies and actual desktop executable/bundle/window are preserved.

## Previous validated checkpoint

# Nullable pinned preview/hide lifecycle and original reentry

PreviewPinnedDockable, TogglePreviewPinnedDockable and HidePreviewingDockables now
accept nullable item/root references through their original fixed methods, generated
FactoryImpl and both native factory views. Typed null preview/toggle arguments reach
the actual virtual FindRoot; interface object-null commands retain the source no-op.
Toggle keeps its fixed Preview call and second virtual lookup. Existing preview cleanup
runs before incoming item Owner access; explicit cleanup ignores KeepVisible.

Hide preserves the source snapshot, KeepVisible short circuit and distinct OriginalOwner
access when the flag is not consulted. Earlier item mutations remain committed when a
later null snapshot item fails. Removal rereads the current root.PinnedDock while final
count/selection use the original cached preview/list, even after a property callback
replaces or clears the live preview. Kept entries retain the first nonsplitter selection.

A cleared live preview now passes actual null to the original InitDockable override,
with the original root as owner, instead of failing before virtual dispatch. Custom
Init can restore a live preview and receive final selection; default Init fails at
Context after the original preview list has committed. Clearing a preview during
nonnull Init keeps the later ActiveDockable access failure and saved preview identity.
No mirror state, null placeholder, alternative engine or new dependency is introduced.

Eight actual-factory model tests and two independent native consumers accompany27
original C# assertions. Workspace1,717 tests/47 suites, C#696 assertions and all17
validation gates pass. Full inventory remains232 full/454 partial/1242 pending of1928.
Full original goal ACTIVE/incomplete; hide/restore and other factory/service/manager
inputs, serializers and other backends, complete metadata/generics/delegates, CLR/
lifetime and platform/input/rendering/accessibility/performance scope remain required.
Actual desktop executable/bundle/window and original/sibling sources are preserved.

## Previous validated checkpoint

# Nullable movement, swapping and connected pin/capability APIs

All move/swap dock and item arguments now carry actual nullable references through
IFactory, generated FactoryImpl and both native metadata views. The original shared
list is queried with IndexOf/Contains on the nullable value; null is retained as an
entry and event payload. Within-dock ordering, selection and reentrant swap snapshots
match the source. Added/removed nonnull payloads still perform all original state
synchronization callbacks, including five root lookups in the recorded case.

Between-dock movement performs fixed Unpin before target access, creates the target
list before source access and retains the same-owner insertion-before-removal order.
Both swap lists commit before the actual virtual Init calls; default null Init failure
leaves those commits in place. The between-docks swap overload still initializes twice
for the same owner/null slots, unlike the within-dock overload's equal-index no-op.
Pinned movement only uses the first matching strip. Unpin preserves fixed membership
lookup then virtual Pin dispatch, including null-aware enclosing overrides.

Public capability resolver item inputs and their native declarations are nullable.
Default operation/drop resolvers fail at original Owner access for null; explicit
capability evaluation fails at the original flag access. Factory pinning still resolves
the operation dock before evaluation. Ordinary callers wrap the same original handles.
No alternate engine, copied model/list storage, null placeholder or dependency change.

Eight actual-factory model tests and three independent native consumers accompany38
new original C# assertions. Exact source traces, original shared list/Rc identities,
custom nullable Init/Remove/Pin overrides and default failure paths are checked.
Workspace1,707 tests/47 suites, C#669 assertions and all17 validation gates pass.
Source inventory stays232 full/454 partial/1242 pending of1928. Full goal ACTIVE and
incomplete; remaining operation/service inputs, preview lifecycle, other backends and
serializers, complete public/generic/delegate metadata, CLR/lifetime and full original
platform/input/rendering/accessibility/performance scope remain required.

## Previous validated checkpoint

# Nullable targets, explicit pin root and protected selection/helpers

AddDockable now carries a nullable target through the original virtual slot, generated
FactoryImpl and both native factory metadata views. Actual Init receives the item and
nullable owner before the first target VisibleDockables access. Default Init retains
its Owner/Factory side effects even when that later access fails. A null item still
fails at the original Context access unless the enclosing Init override handles it.

Six protected visible-list helper targets are nullable and access the original list
before index/item processing. The protected active-selection target and removed-item
arguments are nullable too; its default body ignores removed-item and keeps original
empty/null-child/splitter/identity/index behavior. Enclosing overrides receive those
actual arguments without a placeholder model or duplicate state.

The explicit protected pin root remains nullable throughout membership lookup. A null
root can return with no pin action; the visible path updates pinned bounds before its
original root-collection access fails. A fallback membership lookup may find another
root, but does not replace the explicit caller argument: unpin then fails at its
original root.PinnedDock access. Both tool-owner and non-tool-owner branches, and
item/owner bounds fallback, are checked against unchanged C# bodies.

Seven new actual-factory model tests, one independent native consumer and an extended
native override test accompany27 original-engine assertions. Workspace1,696 tests in
47 suites, C#631 assertions and all17 validation gates pass. Same existing engine,
list and Rc identities; no dependency or actual desktop executable/bundle/window edits.
Inventory remains232 full/454 partial/1242 pending of1928; full goal ACTIVE/incomplete.
Remaining operations/services, other backends, serializers, full public metadata,
CLR/lifetime and platform/input/rendering/accessibility scope remain required.

## Previous validated checkpoint

# Nullable insertion, removal and protected pin item arguments

InsertDockable now accepts nullable dock/item references through the original virtual
slot and both native Core factory views. Negative indices return before initialization
or either dereference. For nonnegative indices the actual Init override receives the
nullable item/owner first. The original shared list then receives null, retaining its
identity, notification payload/index and empty/opened state. Invalid indices preserve
source initialization/list-creation order before failure.

RemoveDockable forwards null through original root resolution and virtual pin cleanup
before its first Owner access. Both protected PinDockableCore item arguments are
nullable; default root lookup and explicit-root dispatch keep source failure order.
FloatAll no longer dereferences null snapshot entries before the actual Remove
override; original Remove/Add/moved/active handling sees those entries unchanged.

Six actual-factory model tests and two independent native consumers accompany twenty
new original-engine assertions. Locked workspace1,688 tests/47 suites and C#604
assertions pass. All seventeen validation gates pass; see current continuation.
Canonical optional metadata declarations and existing nonnull raw-handle calls are
both tested. The original snapshot is retained; no alternate engine, placeholder
object or mirror collection is introduced.

The explicit pin root, other factory/service arguments, serializers, other model
backends, public attribute/generic/delegate metadata, CLR/lifetime and full desktop
input/rendering/accessibility scope remain open. Full goal ACTIVE/incomplete;
inventory232 full/454 partial/1242 pending of1928. Original and sibling sources,
retained dependencies and actual desktop executable/bundle/window are preserved.

## Previous validated checkpoint

# Nullable protected state hooks and root lookup

The original protected update/recursive/resolve slots now accept nullable dockable
references. Visible null children reach the actual enclosing factory overrides before
the source type checks. Split-view extras retain their original ordering and duplicate
exclusion. Default update resolves first, enters synchronization suppression, then
fails at its original DockingState assignment for null; scope disposal restores the
observable synchronization behavior during unwinding.

FindRoot accepts null in its original virtual slot, including a null owner reread
after a predicate mutates the graph. Custom null-aware overrides receive that argument.
The default body fails at its original Owner access. Location/floating helpers preserve
OriginalOwner dereference order. IsDockablePinned remains the source nonvirtual body,
with nullable membership checks over the same four original strips. Both native Core
factory views use those arguments and actual original root/list identities.

Four actual-factory model tests and one independent native consumer accompany thirteen
new original-engine assertions. All 1,680 workspace tests, 584 C# assertions and 17
validation gates pass. Existing nonnull callers wrap the same handles in Some; no
alternate engine, null placeholder, copied model or new dependency is introduced.
The six retained dependency copies and actual desktop window/binary are preserved.

Remaining factory/service slots, including RemoveDockable and InsertDockable, still
need nullable override/failure-order coverage. Serializers, other backends/services,
public attribute/generic/delegate/CLR/lifetime and platform/input/rendering/performance
scope remain open. The full original goal stays active; inventory remains 232 full,
454 partial and 1,242 pending of 1,928.

## Previous validated checkpoint

# Nullable collection elements and native object values

Visible, hidden and pinned IList<IDockable> handles now use the original shared
ListHandle engine with Option<Rc<dyn IDockable>> elements. Original base AddDockable
passes null through the Init override, then appends it and updates empty/opened state
and the original added event. Observable normalization, splitter cleanup and first-null
fallback preserve source order, list identity and actual original model references.
Queries and predicates accept null, and Some(None) means a yielded null rather than
end-of-enumeration. Native list methods, indexers and query cursors expose that same
engine and retain null slots, indices and payloads.

The native boundary uses Base's existing null/value/object conversion. An isolated
one-file controls patch corrects generic item boxing/unboxing/equality; registered model
handles denote their actual native object and restore the same Rc through public
conversion. It serves all collections without Dock-specific branches or duplicate state.
The Core predicate adapter uses the same normalization. Private storage/iterator aliases
and the canonical Core predicate alias retain the same underlying types and dispatch.

Four new model tests, three independent consumers, a native projection regression and
a delegate-boundary regression accompany original C# cases. Compiled IList indexers
retain the original stale value until rebind; Count/ItemsSource remain live. Selector
checks preserve exact original selection and replacement identities. Full locked
workspace passes 1,675 tests in 47 suites; all 4,428 native controls tests and 571
original-engine assertions pass, alongside all 17 sequential validation gates. See VALIDATION.md.

This does not complete nullable virtual dispatch: protected state recursion/resolution,
FindRoot, RemoveDockable and other factory/service inputs still require nullable slot
propagation and original first-dereference/override/event ordering. Arbitrary generic
collections/delegates, serializers, remaining model/service backends, CLR/lifetime and
platform/input/rendering/performance scope remain required. The full goal remains active;
232 files full, 454 partial and 1,242 pending out of 1,928. No original/sibling source, staged contents
or actual desktop executable/bundle/window was changed. All temp/TMPDIR use SSD.

## Previous validated checkpoint

# Nullable factory argument slots and original override dispatch

The original IFactory.InitDockable, AddDockable and SetActiveDockable virtual slots
now carry Option<Rc<dyn IDockable>>. FactoryImpl dispatch is regenerated from those
contracts, and both original Core interface and abstract FactoryBase metadata accept
the same runtime null reference. Document/tool base methods forward None directly to
the factory and preserve the three separately reread Factory-conditional calls.
Default InitDockable and SetActiveDockable report failure at their original first
Context/Owner dereference. Null-aware custom factory overrides receive the argument.
Non-null engine, service, UI and sample callers wrap their same original handles in
Some; this does not copy, replace or adapt the original models or engine.

Four new model tests cover all document/tool call paths, parameter/owner identity,
factory replacement and removal after Add or activation, default null failures and
base Add dispatching null to an initialization override before graph mutation.
Independent compiled/runtime null-command consumers now execute null-aware factory
subclasses. A separate native consumer invokes all three nullable slots through both
interface and abstract base metadata. Original C# reference cases cover the same
call order, rereads, owner identity, overridden initialization and bound commands.

A source-only C# assertion proves the remaining nullable collection boundary: when a
custom initialization override accepts null and returns, original base AddDockable
appends null to IList<IDockable>. Current ListHandle<Rc<dyn IDockable>> cannot store
that element. That path, other factory argument nullability, collection/event/CLR
behavior and remaining original API/platform scope stay incomplete. The assertion is
not presented as Rust parity. Do not replace the list engine or use a UI-specific
workaround. Other public factory slots and original serialization remain required.

Temporary artifacts and subprocess TMPDIR now use
`/Volumes/SSD/temp/ferrodock-port`, per the user's requirement. The 138 previous
owned-prefix files were moved from /tmp; historical documentation links were updated.
No dependency/vendor changes or desktop executable/window refresh was performed.

## Previous validated checkpoint

# Source callback identity and nullable document/tool virtual arguments

DocumentFactory now aliases the original DockableFactory callable:
`Rc<dyn Fn() -> Option<Rc<dyn IDockable>>>`. CLR reference-nullability annotations
do not define distinct Func types, so both native views retain the same actual Rc.
Conversions, identity checks and original locator/property roundtrips need no callback
adapter or origin cache. The temporary runtime callback bridge has been removed;
Domain remains independent of UI, Base and Runtime.

CreateDocument invokes the source callback then the original AddDocument virtual slot.
IDocumentDock.AddDocument/AddTool and IToolDock.AddTool now carry nullable reference
arguments through one original slot. Concrete nonnull conveniences and fluent helpers
forward through that slot, while derived base calls use explicit base-body helpers.
The base document/tool implementations preserve all three independently reread
Factory-conditional calls. Without a factory a null argument is a no-op; a null-aware
enclosing override receives None. Core/backend metadata preserves the same arguments.
Independent compiled/runtime three-command consumers exercise CreateDocument,
AddDocument(null) and AddTool(null) against actual source models and custom overrides.
Equivalent unchanged C# reference scenarios pass.

A remaining boundary is explicit: IFactory arguments are still nonnullable. A present
default factory reports its original first null dereference before graph mutation,
but a custom null-aware factory override cannot yet receive that argument. Faithful
nullable factory propagation, list-null behavior and wider CLR exception/override
semantics remain required. General Func/Delegate metadata, Equals/multicast, serializer,
platform/input/rendering/performance and the remaining original source scope are also
incomplete. Do not replace these contracts with a UI-specific engine branch.

## Previous validated checkpoint

# Shared native callback identity and cross-assignment

The UI-independent runtime now bridges required/nullable Rust callback forms when
Option represents a CLR reference-nullability annotation. This must not alias nullable
value types, whose CLR types differ. Core registers native casts in both directions
between its document-factory and locator views of the one System.Func<IDockable>.
The docking engine and source model callable types are unchanged.

A typed weak adapter cache reuses live adapters and restores the actual original Rc
when converted back. Weak origin maps preserve the original native reference identity
and native Func.Invoke null results through required Rust views. No cache retains
application callbacks strongly. Adapter-held cleanup guards retain only a weak registry
and remove their entries when the last adapter dies, checking the exact adapter pointer
before removing a source entry. Callback bodies and captured-state destruction may
reenter the bridge; no registry borrow spans a callback. The code uses no unsafe or
additional dependency, and the same shared runtime serves library consumers.

Standalone tests cover both roundtrip directions, deferred/reused adapters, native
null results, required-contract violation, distinct callback identity, capture release,
cache cleanup and reentry from callback bodies and captured-state destruction. Core
checks original contract setters/getters and locator-map roundtrips; independent emitted
and interpreted TwoWay object bindings execute original document creation exactly once.
Equivalent original C# source cases check the same identity/map/binding/engine flow.

Required raw Rust invocation cannot return null and reports a programmer contract
error; native Func.Invoke preserves null by invoking the original nullable callable.
The original C# AddDocument(null) behavior, including a no-op without Factory and
nullable override dispatch, is not represented by the current required source Rust
signature. That engine edge and general Delegate.Equals/multicast/type metadata,
CLR cycles/exceptions, serializer and remaining original scope remain unfinished.
Do not add an application-specific engine branch or claim full delegate parity.

## Previous validated checkpoint

# Original document factory and template-required metadata

Core now declares the original IDocumentDockFactory contract and its nullable
DocumentFactory property. The source callback retains its existing nonnullable-result
Rc callable; a shared DocumentFactory alias is re-exported by the ReactiveUI model.
No original engine algorithm or callback invocation is replaced.

DocumentFactoryReference retains that same callback instance. It and the existing
nullable-result locator view are handles of a single native System.Func<IDockable>
descriptor. Canonical Invoke metadata dispatches the actual source representation;
invalid receivers return a structured invocation error. Getters, setters, clearing,
reference identity, deferred execution, native method signature validation and capture
release are covered independently. The ReactiveUI native DocumentDock class exposes
its original factory interface/property and keeps the same enclosing source instance.

All fourteen original RequiresDataTemplate interface applications now use FerroUI's
existing custom-attribute data. Runtime and compiler exports are checked exactly;
eleven interface files whose sole remaining gap was this metadata are now complete.
The public RequiresDataTemplateAttribute class, System.Attribute base and AttributeUsage
API remain unported and are tracked separately.

Independent consumers cover native method assignment, class construction, Core-qualified
Func invocation and original CreateDocument graph ownership/activation through compiled
and runtime paths. Runtime test fixtures register their own application assembly and
recover source roots through registered casts rather than assuming one Rust boxing
form. Equivalent C# assets explicitly enable compiled bindings and execute unchanged
original source. No reflection or vendor/dependency changes are introduced.

IDocumentDockFactory remains partial: CLR nullable annotations do not change delegate
type or reference identity. The C# reference explicitly checks assigning a nullable-
result locator callback into DocumentFactory with the same identity/result. A shared
identity-preserving bridge between the two Rust callable representations is still
required. The shared native descriptor and correct own-representation views do not
prove that cross-assignment or general delegate/CLR parity.

## Previous validated checkpoint

# Original Model.Controls ownership in the Core SDK

The UI-independent `ferrodock-model-ferro` SDK now owns 22 original Model.Controls
contracts under the logical assembly `FerroDock.Model`. Twenty descriptors moved
from the UI SDK; their previous public paths alias the same declarations. UI files
retain only conversions needed for native or managed model projections. New native
IToolTemplate and IToolDockContent descriptors dispatch their original source
contracts, including arbitrary object/null content. A default optional INPC discovery
hook on IToolTemplate mirrors the existing IDocumentTemplate integration contract.

IMdiDocument now declares its original IDocument base. IDocumentDock and IToolDock
also declare original ILocalTarget membership. Core retains source interface casts,
reference identities and the actual notifier; it adds no model wrappers or engine.
Root Windows metadata exposes the original IList<IDockWindow> handle, Count, indexer
and supported mutators, with the same original notifications and shared identity.
Complete generic IList identity, CopyTo and enumerator metadata remain unfinished.

All 18 inspector property views and the main inspector now resolve Model.Controls
from the original Model assembly. Source guards preserve the original layout and
bindings. Independent compiled and runtime consumers verify root window/pinned lists,
document/tool Add commands and original graph ownership/activation, inherited MDI
state/title, nullable grid definitions and stack spacing/orientation. Equivalent C#
assets explicitly enable compiled bindings against unchanged original source.

The scanner cannot export native cast facts from unresolved wildcard trait imports.
New registration helpers use explicit original trait imports; Core export has no
unread registrations and UI export has no unread casts. A standalone regression
checks all 22 control exports, original assembly ownership and MDI/marker inheritance.
No compiler completeness check or dependency patch was weakened or changed.

IDocumentDockFactory remains partial: original source API and engine behavior exist,
but native DocumentFactory delegate metadata and canonical generic delegate identity
are not exposed. Its nonoptional-result Rust callable must not be replaced by the
optional-result locator delegate. The inventory corrects the previous ported label
and explicitly records this gap. RequiresDataTemplate attributes and the wider
original platform/input/rendering/lifetime/serialization scope remain incomplete.

## Previous validated checkpoint

# Core target/container ownership and original host-window interface

Core now owns ILocalTarget, IGlobalTarget and IDockItemContainerMetadata descriptors
under their original Model logical assembly. UI retains aliases to the same descriptors
and managed/native projection casts; source-only checked casts and reference facts are
Core-owned. Container properties use the existing shared runtime conversion, preserving
original arbitrary objects and nulls without importing controls. Domain and original
engine algorithms are unchanged.

IHostWindow native metadata exposes HostWindowState, IsTracked and Window plus the
original Present/Exit/SetPosition/SetSize/SetWindowState/GetWindowState/SetTitle/SetLayout/
SetActive methods. Calls go to the actual source host. IHostWindowState and inherited
IDockManagerState descriptors retain original identity and inheritance. GetPosition and
GetSize still have original Rust trait adaptations; original native out-parameter
metadata remains explicitly unexposed until shared metadata supports it.

Core host bindings exposed a shared framework bug: WeakValue retains native sources by
weak object plus original handle TypeId, but ValueTypes.from_object restored only class
and element handles. The isolated published0.3.0 base patch now reuses existing class-
declared interface conversion facts when restoring interface handles, including nullable
views and inherited interface declarations. It adds no object retention or cast table.
The patch and exact hashes are regenerated against verified untouched published sources;
a dry-run confirms applicability. Original/source sibling checkouts remain unmodified.

TwoWay container binding also exposed a shared object-setter mismatch: typed
Option<BoxedValue> transport was passed intact to untyped metadata setters. Shared
markup runtime object_assignment_value unwraps registered nullable/object boxes and
retains native Control identity. Both emitted and runtime metadata accessor paths use
it for System.Object setters; runtime accessors cache that declared type fact. Existing
markup/runtime-loader isolated patches and exact provenance now include the fix. A
consumer checks null, nested boxes, nullable strings, scalars, controls and original
container TwoWay behavior against the unchanged C# source.

Native-host bindings and direct lifecycle/geometry calls, container raw Control object
identity and TwoWay selectors, standalone custom host interface dispatch and source
marker membership are checked in independent consumers and an unchanged C# reference.
Binding normalization may use another Rust box for the same native Control; direct Core
getters retain raw values, while content bindings preserve the actual enclosing object.
A generic inherited mutable-brush interface case checks weak source restoration and
collection independently of the existing original strong readonly HostWindowState/host
cycle. That source lifetime requirement remains visible; it is not weakened to make a
host collection assertion pass. General CLR cycle collection and broader ownership,
serialization, generic/default/out/collection and platform parity remain unfinished.
See VALIDATION.md and CONTINUATION.md for final results and remaining work.

## Previous Core dock/recycling checkpoint

# Core dock contracts and recycling Model SDK

IDockable, IDock and the typed dockable-list metadata now live in FerroDock.Model.Ferro
under the original logical FerroDock.Model assembly. Their original getters, setters,
methods and notification hooks moved intact; public controls exports remain aliases
to the same descriptors. Core owns the source IDockable-to-IDock checked cast through
the original as_dock hook. UI retains managed/control projection and ItemsSource
integration. No Domain or ReactiveUI engine algorithm changed.

The new FerroDock.Controls.Recycling.Model.Ferro package owns IControlRecycling and
IControlRecyclingIdProvider metadata under the original recycling Model assembly.
Its normal dependencies are the existing source contract crate, shared runtime and
native base. Core now uses this SDK without loading general controls. GetControlRecyclingId,
TryToUseIdAsKey and Add/Build/Clear call original interface implementations. The shared
runtime bridge preserves nullable arguments/results and object identity. TryGetValue
with its original bool/out signature remains absent from native metadata: substituting
a return-only signature would not establish that API's parity.

Core Context metadata uses the shared runtime object bridge. Normal recycling UI
registration adds the existing raw Ref<Control> value fact, retaining the prior raw
control projection without introducing a UI dependency in Core or an object table.
Standalone metadata tests and compiled downstream consumers cover source lists,
Count notifications and separately boxed list reference identity, actual command identity/navigation, Core-qualified inherited
bindings, TwoWay changes and native control Context/cache identity. Equivalent new
C# cases run through unchanged original source assemblies. Both compiler hosts accept
the new interface consumers.

The main inspector and three property views now reference all migrated Core contracts
through the original Core assembly; source layout, bindings and commands are unchanged.
Exact source comparison and Python regressions guard the namespace migration. Generic
collection metadata, out/default/generic parameters, remaining Core contracts, complete
CLR identity/lifetimes/serialization and platform parity remain unfinished. See
VALIDATION.md and CONTINUATION.md for actual checks and counts.

## Previous capability checkpoint

# Core capability classes, window contract and original resolver

IDockCapabilityPolicy, DockCapabilityPolicy, DockCapabilityOverrides and IDockWindow
metadata now belong to the Core SDK and original logical FerroDock.Model assembly.
Controls retain Rust aliases to the same declarations and stop exporting duplicate
paths/registrations. Source properties, virtual Get dispatch, shared identity and
window notifications stay on the original model objects. The existing window metadata
test moves with its owning descriptor and runs without general controls.

Core adds the original DockWindowOptions constructor/properties/ApplyTo, including
nullable taskbar behavior and null-window rejection; immutable DockCapabilityEvaluation
constructor/read-only fields; the original resolver's static operations and context
overloads; and DockCapabilityValueSource as the 23rd enum descriptor. Its fields are
generated from the original Domain declaration, with only a hidden source-string hook
added to Domain. No algorithms, dependencies or vendor patches change. Three small
options/result/resolver metadata declarations share core_options_metadata.rs and map
to their separate source files in ports.json.

Six Core cases and four independent consumers cover original policy constructors,
nullable values, inheritance, custom Get overrides, identity, option application,
resolver precedence/context, immutable result fields, Core-qualified markup and both
compiler hosts. Parent-window notifications and TwoWay edits reach actual engine
objects. Runtime constructor compilation records the actual public Rc<model> types,
using the same shared metadata path as existing Core policy constructors.

Only the inspector's window-contract references move to Core; all source layout and
behavior remains unchanged. The exact migration guard has two additional regressions
rejecting old ownership or unrelated changes. Full Core ownership, serialization,
generic/optional metadata, arbitrary CLR enum/string/identity/GC and native platform
parity remain open. The desktop sample was preserved. See VALIDATION.md for evidence.

## Previous factory API checkpoint

# Factory operations and lazy search metadata

Core IFactory and FactoryBase metadata now expose the original container lookup,
three-argument window initialization, active initialization, activation, collection
insertion/removal/move/swap, pin lookup/reordering, window list operations, collapse,
split creation/docking and window creation/splitting overloads. FactoryBase additionally
publishes the original command overloads already exercised through IFactory. Calls
retain the actual engine, source objects and enclosing virtual dispatch. Twenty newly
exported virtual overloads have argument/order/result identity checks through both
factory views; production graph cases cover mutations, pin order and window tracking.

FindRoot/FindDockable/Find publish the original predicate types and deferred queries.
Checked native method conversion validates input assignability and Boolean results
without invoking callbacks. Query views hold the source DockableQuery; each cursor
uses its original iterator, preserving independent traversal, source duplicate nested
results and rereading current trees on later enumerations. Native interface metadata
exposes Current, MoveNext, Reset failure and explicit IDisposable. Fault/end/disposal
preserve the source Current behavior. This does not establish general CLR iterator
object identity, GC or interface variance parity.

Original optional arguments have explicit arity adapters; general optional-parameter
metadata is incomplete. Generic CreateList/GetDockable metadata, complete collections,
non-generic enumeration contracts, remaining Core ownership and wider CLR/platform
scope remain open. Independent original-style predicate declarations and Core command
bindings compile in a downstream consumer, with matching unchanged C# reference cases.
No original source or sibling FerroUI changes, vendor edits, or desktop rebuilds are
part of this checkpoint. See VALIDATION.md and CONTINUATION.md for validation scope.

## Previous locator checkpoint

# Core factory ownership and locator metadata

IFactory and FactoryExtensions now belong to the Core SDK and logical FerroDock.Model
assembly; general controls retain aliases to the same descriptors. FactoryBase and
IFactory expose tracking, all five locator properties, source creators/initialization,
GetContext/GetHostWindow, and the existing commands/events. Delegate and dictionary
views retain the actual source functions and containers. They preserve null entries,
absent-key fallbacks and original duplicate/missing-key failures. Captured values are
released outside mutable dictionary borrows.

Domain locator entries now represent nullable delegates. A source virtual default-host
getter that becomes null between its two InitLayout reads stores that null, matching
the unchanged C# implementation. Source-style compiled method declarations remain
deferred and are checked before assignment. Independent Core/consumer tests and the
original C# reference cover actual engine identity and behavior.

The shared scanner omitted checked boxed-cast registrations already known at runtime.
An isolated exact published ferroui-build-scan 0.3.0 copy now records those cast pairs;
emitted assignments still use the existing runtime conversion. Two scanner regressions
and exact provenance guard this fifth dependency patch. Remaining generic methods,
collection/delegate metadata, full Core ownership and broader CLR/platform parity are
still incomplete. See VALIDATION.md for final checkpoint results.

## Previous event checkpoint

# Original factory events and shared object interop

FactoryBase and the current IFactory descriptor expose all 28 original events and
public On callback overloads through native metadata. The Core adapters subscribe
to the existing engine, forwarding its enclosing sender and the original shared
arguments. Cancel/Context mutations reach the engine; subscriptions dispose
explicitly and do not root their publishers. Metadata add accessors retain the CLR
void return contract. General metadata removal and CLR delegate/GC parity remain open.

The Core SDK owns all 29 event-argument descriptors, their actual constructors and
declared properties, plus GlobalDockTrackingState and the original tracking reason
enum. Inner identity hooks compare source instances across boxes and clones. No
replacement argument model is introduced. Full System.EventArgs base casts remain
unimplemented. The original object bridge moves to the native-base-only runtime
support crate and is shared by Core metadata/recycling, with the existing raw-control
projection retained in recycling. Source factory/docking algorithms stay unchanged.

Independent compiled consumers and unchanged C# assets verify event wiring, source
argument roots and tracking reason bindings. See VALIDATION.md for verified scope.

## Previous constructor checkpoint

# Factory and DockWindow constructor metadata

The backend integration SDK now constructs the original Factory and DockWindow
classes, alongside all 14 Controls classes. Factory inherits the abstract original
FactoryBase view; DockWindow inherits the original ReactiveBase view. Class, base,
interface and original concrete handles retain the same enclosing objects. A hidden
static enclosing-reference hook also supports composed window overrides without
reflection, duplicated state or a per-object identity table.

The Core SDK owns FactoryBase metadata and identity-preserving registry views. Its
creation, initialization and close operations call the existing factory contract.
Registry dictionaries expose Count and the actual mutable Rust model; the original
observable lists forward Count notifications. Full dictionary/indexer/generic,
factory locator/event and remaining inherited method metadata remain partial.

Compiled consumer assets cover constructors, scalar/enum/reference properties,
live TwoWay updates and source factory graph initialization. A runtime-emitter
failure exposed missing public paths on notification-bearing copies of interface
descriptors. Those actual registered descriptors now declare their existing public
paths through native compiler metadata; no compiler or vendor change was needed.
Resource-owned graph fixtures preserve composition ownership and do not establish
general CLR/GC lifetime equivalence. See VALIDATION.md for verified outcomes.

## Previous Controls checkpoint

# All ReactiveUI Controls constructors and shared model values

All 14 concrete classes in the original ReactiveUI Controls directory now have
constructor/class metadata in the separate integration SDK. The ten new views retain
the existing concrete engine objects, their source base classes, property changes,
original scalar/enum/reference fields, methods and identity. They neither copy model
state nor implement another docking engine. ToolDock.AddTool uses the real factory;
SplitView actions and live pane/numeric/bool bindings keep the source behavior.
Native class-view lifetime checks cover all ten new models.

The source model enum/flags bridge is now owned by `ferrodock-model-ferro` and the
logical `FerroDock.Model` assembly. The same declaration-driven generator and converter
remain, with two additional original SplitView enums; settings helper imports remain
aliases. Core metadata can register without loading UI controls. SDK consumers declare
the model SDK directly when their generated Rust names its helpers. Domain stays UI
independent. Other source interfaces/POCO descriptors still need complete logical
ownership, so the full Core metadata port remains partial.

Actual SplitView assignment exposed an invalid native compiler coercion to a foreign
metadata helper trait. Both compiler hosts now distinguish a real trait's own Rc
handle from separate carriers and use the existing registered conversion otherwise.
Original pane/content models and shared original capability overrides now assign to
required/nullable source interfaces through the same path as runtime loading. Native
SetterBase remains recognized as its own trait. No replacement cast engine or state
registry is introduced. The existing five loader/one build changed files retain exact
provenance against the pinned published packages.

Remaining FactoryBase metadata, other backends/services/serializers, complete Core
contracts and logical ownership, all x:Class/generic/collection/enum root cases, CLR
custom identity/boxing/string/GC/lifetimes and native platform behavior remain in the
unchanged full goal. See CONTINUATION.md and VALIDATION.md for evidence.

## Prior constructor checkpoint

# ReactiveUI integration SDK checkpoint

The model class metadata now lives in the separate
`ferrodock-model-reactiveui-ferro` library. General controls retain no normal
dependency on that library or the concrete ReactiveUI backend, and model/domain
code retains its existing UI boundary. Applications register both libraries in
their composition root. Rust helper imports move from the general control crate
to `ferrodock_model_reactiveui_ferro::reactive_metadata`; original engine APIs and
class inheritance remain unchanged. The sample and downstream consumers are migrated.

The original Document, Tool, DocumentDock and RootDock classes now have ordinary
zero-argument markup constructors in the original logical model assembly. They
construct the real existing engine objects and return their identity-preserving
class handles. Abstract source bases stay constructor-free. Property state,
original commands, notifications, factory ownership/close, source subclass metadata
and compiled TwoWay bindings remain on those same objects. Other concrete class
constructors and full logical ownership of external descriptors remain partial.

A consumer reproduced a native build-compiler public-signature bug: canonical type
keys include private implementation modules even when the crate exports a public
alias. The isolated published build package now renders complete declared type
components through their existing public export paths. Canonical comparison keys
are unchanged. Separate metadata carriers retain their actual model return types,
including generic arguments, and the earlier carrier regression remains covered.
Exact provenance for all four retained native packages is guarded. See
CONTINUATION.md and VALIDATION.md for current evidence and the remaining full scope.

## Prior checkpoints

# FerroDock porting contract

Dock source baseline: `0be0ab3118c6d24be5c4d78befda881d0e539409`.
FerroUI reference docs: `98e091f119e203169f64ae4331fe66bbdccdcbac`.
FerroUI build dependencies: version `=0.3.0`, with a self-contained, provenance-checked
four-file native base support patch and one-file native markup instance-check patch.
Both isolated packages have exact provenance; the sibling checkout is read only.

The original ReactiveUI sample now has actual App resources, startup/lifetimes and a
desktop executable. Its main view theme actions use commands and attached behaviors;
views remain passive, and framework-independent presentation depends on a focused
service interface implemented in the composition root. The original Fluent manager
uses the shared preset algorithm, including custom resource dictionaries, source
matching order and no-op rules. Fifteen original class resources compile. A native
macOS app has launched with the original engine/layout. Its refreshed tab headers,
document/tool selection and pin/open/unpin flow are now visually verified; broader
platform/input and lifetime validation is still required.

Current workspace validation: **1,515 Rust tests**, zero failed/ignored; inventory
**222 ported, 464 partial, 1,242 pending / 1,928**. Full completion remains rejected.
DockingWindowState display is checked against 98 outputs from the original .NET enum,
and now has generated original flags/nullable/static-member metadata and downstream
compiled named/composite/known-numeric literal checks. Compiled unknown bits and
inherited System.Enum APIs remain partial. The original proportional sample view
throws during construction; its native view remains absent pending equivalent
IList.Add emitter/failure behavior. See the committed source-only reference. F11 inspector composition is now implemented. Complete
static-field metadata, generic/runtime contracts and callback/collection/GC differences
remain open. See CONTINUATION.md and VALIDATION.md for current evidence and limits.

The original Simple theme now compiles as a normal workspace library with its own
source class, manager, eight own documents and 48 shared control dictionaries, plus
five linked images under its own assembly URI. The root keeps its original 42-entry
merge list. Native Simple =0.3.0 Headless checks validate density, caching, all presets
and light/dark surface colors, real grip updates and an independent actual docking
graph. No Fluent theme is a normal dependency. The original shared inputs are guarded
by scripts/dock/check_theme_links.py. Full source metadata/lifetime/platform scope remains
partial, as recorded in the source inventory and latest validation.

The Browser theme now compiles its five original documents and source Styles class
as a normal workspace library. Its original Fluent dependency, empty compact resource
dictionary, platform options and floating-host overrides remain intact. Closed typed
OnPlatform metadata forwards to the published native implementations. The original
DelayedUniformTabPanel uses the actual layout, debounce and native timer/transition
APIs; 24 independent .NET layouts and native dispatcher scenarios validate its current
scope. All three themes and the panel perform source constructor work in FerroUI's
inherited constructed hook, including derived instances. Actual animation cadence,
attached DPI and broader metadata/input/lifetime parity remain partial.

The original diagnostics selection helper/adorner now compile as a normal workspace
library. They use native adorner layers, logical parenting, coordinate transforms,
drawing and strong layout subscriptions. The helper keeps the source factory-map
priority and success-with-null-control result. An unchanged-source .NET reference
validates 216 map combinations and actual attached/live/detached/removal/reentry
behavior. The original debug overlay adorner/helper/manager and both extension bodies
now compile too, preserving native drawing, pointer input and strong subscriptions.
The DEBUG source App attaches F9; native macOS verifies enable, tab input through the
overlay and removal. The unchanged source reference confirms 16 render cases and the
source detached-dock orphan behavior. The 18 passive inspector property views now compile with their unchanged source
markup apart from required namespace/assembly ownership. Their five missing model
interface descriptors forward to original handles. Three missing domain enum
descriptors and standard parse/type-converter metadata enable source TextBox edits;
normal native compiled bindings perform the updates. Original .NET reference output
covers every grid and 24 enum edits. The complete RootDockDebug/F11 composition now uses these actual views and native
selection/logging/window APIs. Broad native/CLR lifetime/platform parity remains partial. Undeclared numeric plain-enum
values remain the existing native/domain representation gap.

The following checkpoints are historical.


The original ReactiveUI sample's model/factory/main-view-model units now compile as a
normal Cargo workspace library, including all source three-document/eight-tool layout
classes, locators, commands and tracking. Ten cases pass with the original engine.
Current validation: 1,388 Rust tests, zero failed/ignored; inventory 222 ported, 345
partial, 1,361 pending / 1,928. The actual sample views/bootstrap remain unported; there
is still no executable. Preset manager work exposed missing safe ResourceInclude type
recovery and mutable merged-dictionary interface APIs in shared FerroUI. No unsafe or
reduced replacement was introduced. See CONTINUATION.md for the current path.

The following Fluent checkpoint is historical.


The complete original ScrollViewer/document/tool/chrome dictionaries now compile with
native source command and seven additional converters. Downstream tests exercise the
original document empty-content flows, deferred tool content and real scroll button/
viewer bindings. No replacement theme or docking algorithm was introduced. Current
validation: 1,344 tests, zero failed/ignored; inventory 210 ported, 295 partial, 1,423
pending / 1,928. Source units remain partial until full original API/scenario/identity
scope is established. The complete Fluent root/theme, actual pinned host and Rust
sample startup remain pending. CONTINUATION.md records the current implementation path.

The checkpoint below predates this Fluent integration and is retained as history.


The actual native host/controller/state/adorner/preview/factory group is now integrated
into normal Cargo modules. All formerly staged Rust duplicates have been removed.
Document/tool tabs, their closed generic native peers, tracking panel, document/tool
hosts and peers and three source converters compile too. Seven complete original
Fluent dictionaries compile, and downstream tests realize actual source containers and
tracking panels. All native classes use the shared class/property/metadata model.

Latest validation: 1,329 Rust tests, zero failed/ignored; UI 336; downstream markup 37.
Inventory: 210 ported, 280 partial, 1,438 pending / 1,928. New units remain partial until
remaining original scenarios, APIs and lifetime/platform differences are addressed.
The complete source Fluent theme and original Rust sample application remain pending;
there is no runnable Rust sample yet. See CONTINUATION.md and VALIDATION.md for current
integration details and next work. Earlier staging/count statements below are historical.

## Historical implementation checkpoints

The native host/chrome/pinned source dependency group now has complete initial translations
staged alongside actual controller/state bodies. Host/HostWindowState, tool chrome, pinned
item/container and all four peers remain uncompiled and unvalidated. Remaining concrete
DockHelpers and layer discovery continuations are staged for merge into existing compiled
modules. Source strong edges, close/preserve ordering, geometry/drag contexts and actual
model/engine calls remain explicit. One tested native receiver/equality bridge on the domain
close-state contract brings runtime validation to 1,308 tests. Inventory: 210 ported, 253
partial, 1,465 pending / 1,928. No staged member is counted complete. Document/tool tabs,
actual full-group compilation/registration/testing and source themes still precede a sample.

The main control now has a full initial native class translation staged with its actual
host/state/item dependencies. Independent original selector traversal/gesture/drag-action
methods compile; five tests use the original reactive models and a 21-pair .NET comparison
fixture. Native selector receiver identity/value conversion and diagnostic multicast/event
capture behavior add four tests, for 1,307 total. Staged class, surface contract and peer are
syntax-formatted only: no runtime members are claimed complete. Source-bound callbacks
remain strong; native weak surface references track actual native receivers. Delegate
property/metadata/weak lifetime/GC/process-wide parity and all real host/input integration
remain explicit. Inventory is 210 ported, 244 partial, 1,474 pending of 1,928. No runnable
Rust sample yet; actual native hosts and tab/pinned dependencies are next.

The latest state pass compiles original drag-context transfer/reset and extracted numeric
continuations, adding four tests (1,298 total). A .NET 10 primitive reference verifies ties,
saturation, nonfinite and signed-zero behavior; it does not validate the full controller.
The complete initial DockControlState, offset interface/default and factory lifecycle service/
interface are staged, uncompiled and unvalidated until actual DockControl/host/input classes
and callback receiver identity are available. No state/service runtime member is claimed
complete. Source strong edges and original cached-offset/capture/operation behavior remain
explicit. There is still no runnable Rust sample. See current validation and continuation.

The preview pass compiles original movement/sizing/templates/content/overlay/cleanup methods
and Show's extracted floating continuation. Public Show/managed host discovery remain
staged with actual DockControl. Seventeen tests bring validation to 1,294 tests. A pinned
C# 12.1.3 Headless probe compiles the exact original coordinate method plus its actual root
compatibility extension. Both original and native roots are TopLevelHost; their (0,0) direct-
cast fallback matches in four reference cases. Preserve that source behavior; no ancestor
lookup replaces it. Full desktop/public host behavior and runtime lifetime gaps remain open.
See the latest validation/continuation and scripts/dock/preview-root-reference for exact scope.

The host-state pass adds same-reference local/global marker queries using actual inherited
source interfaces and Rc upcasts. Erased model diagnostic names use compiler type names or
an explicit custom override without reflection; arbitrary CLR naming parity remains open.
Eight native tests bring validation to 1,277 tests. The original global-service interface
compiles; its implementation, DockManagerState and managed host/state translations are
staged outside Cargo, uncompiled/unvalidated with no completed-member claims. Source mutual
strong ownership remains intact; native Rc/CLR GC cycles are not hidden by weak edges.
Actual DockControl, preview and native host/state integration still precede a runnable
Rust sample. See the latest validation/continuation and staging records.

The latest helper pass validates native hit testing/model resolution and the complete
global-docking count algorithm with 14 new tests (1,269 total). Concrete host/external/Z-order
members remain pending. A C# compiler probe records repeated Owner-pattern getter semantics.
The initial AdornerHelper translation is staged outside Cargo, uncompiled and unvalidated;
real DockControl host lookup is required before source lifecycle/geometry integration.
See VALIDATION.md and CONTINUATION.md for current scope. There is no runnable Rust sample yet.

Follow the [FerroUI porting guide](https://github.com/wieslawsoltes/FerroUI/blob/98e091f119e203169f64ae4331fe66bbdccdcbac/docs/porting/PORTING-GUIDE.md)
and the engineering rules in the parent AGENTS.md. Rust tests are the translated
counterparts of the C# xUnit tests. UI tests must use FerroUI Headless and port
the same input scenarios. Initial reactive view activation now has real Headless
integration; the docking control and its full UI tests remain pending.

Preserve file/member correspondence, algorithm branches, default values, event
ordering, extension points, reference identity, and collection handle semantics.
Methods/properties use snake_case, interfaces keep the `I` prefix, enums retain
their variants and numeric values, and flags use SCREAMING_CASE. UI classes use
FerroUI's class macros; plain model objects use Rc, interior mutability, and
composition. No reflection is introduced. Markup retains public PascalCase names
and must use compiled bindings. Do not create substitute docking algorithms or
replace the UI with a simpler demonstration.

Source inventories include all tracked `.cs`, `.axaml`, and `.csproj` files under
`src`, `tests`, and `samples`. `upstream-files.json` freezes paths and SHA-256
digests (CRLF normalized to LF for Git checkouts on different platforms).
`ports.json` records target paths, ported members, deferred members and
test correspondence. An absent source in ports.json is **pending**, never waived.
Shared-source projects and compatibility layers remain pending until explicitly
mapped; duplicate source inclusion does not establish separate implementations.
No project is silently dropped from scope.

The inventory checker verifies the source baseline, target existence, explicit
enum values/variants, and source naming conventions. It does not prove behavioral
parity or automatically infer that non-enum members were implemented correctly.
The corresponding tests and later integration/runtime verification must do that.
`--require-complete` rejects pending/partial files; it is a necessary completion
gate, not sufficient evidence that the full goal is achieved.

Current adaptations and open differences:

| Item | Treatment |
|---|---|
| C# invalid enum casts | Rust enums use checked TryFrom<i32>. Policy `get_raw` returns None for unknown capabilities, preserving the tested boundary result without constructing invalid Rust values. |
| Virtual DockCapabilityPolicy.Get | `DockCapabilityPolicyImpl` provides the virtual query contract. Custom policies compose the base, delegate property access, and implement Get. Consumers must keep the trait handle to preserve dispatch. |
| DockCapabilityOverrides inheritance | Embedded base plus Deref and delegated interface implementations. All inherited property behavior and highest-precedence override state are retained. |
| DataContract attributes on policies | Serialization metadata is deferred until serializers are ported. Policy files are marked partial for this reason. HasAnyOverride must remain excluded from persisted data. |
| Global busy service threading | Follows FerroUI's Rc/thread-affine reactive model; explicit IDisposable handles and notification order are preserved. Native cross-thread entry points from the C# service remain an open difference, not a verified equivalent. |
| ServiceDispatcher.InvokeAsync | Native internal await adapter uses actual System task completion and forwards pending wakes asynchronously. The shared TaskCreationOptions surface, transport performance and full cross-thread parity remain open. |
| DockPoint/DockRect equality | Value-type PartialEq follows Double.Equals, including equality of NaNs and signed zero. Derived Rust float equality would change the NaN behavior. |
| DockPoint/DockRect formatting | Display uses Ryū shortest digits and invariant general-format rules, including uppercase signed exponents, scientific thresholds, Infinity and negative zero. Checked against 7,895 reference values captured from the actual C# sources. |
| DockRect serialization | DataContract and ordered DataMember metadata are explicitly deferred; constructor, mutable fields, value equality and Display are implemented. |
| ItemsSource weak tables | Source tracking algorithms are implemented with weak identity keys and retained owner/item values. Native pruning releases expired noncyclic keys on access; CLR ephemeron cycle collection and native pruning performance remain incomplete. Explicit Untrack follows the source control lifecycle. |
| Boxed source-item keys | Existing Context scalar/domain/canonical-reference equality is shared with native key hashing. Custom boxed Equals/GetHashCode and concrete/subtype value metadata remain deferred; arbitrary object-key parity is not claimed. |
| GetContainerFromItem null | Rust accepts Option<BoxedValue> to preserve the source method's explicit null guard despite its nonnullable C# annotation. |

The operation mask treats OuterLeft/Top/Right/Bottom as their corresponding
direction's permission bits. ToAlignment deliberately returns Unset for outer
operations, matching the source. Alignment's default is Left (numeric zero), not
Unset (numeric minus one).

The first validation pass covers 64 Rust unit tests: 48 model tests and 16 service
tests. It ports all 14 xUnit cases in DockCapabilityPolicyTests, adds operation
mask coverage over all 64 permission masks, checks reference sharing/custom
policy dispatch, and exercises busy scope nesting, notification order, explicit
disposal, retention/release and dispatch selection. Update this report as work
continues; final command output is the authoritative test result.

The second pass adds the complete TrackingAdapter, geometry behavior, immutable
DockCapabilityEvaluation, four optional model contracts and both recycling-model
contracts. The five source Recycling.Model xUnit cases run through the ported
trait with a test-only dummy cache. At that point adapter tests reproduced the coordinate
assertions of DockableTrackingTests without its concrete Document integration.
That integration is now implemented in the later reactive model pass. Validation
for the second pass comprised 88 Rust tests (65 model, 16 services, 7 recycling-model), with no ignored
tests. See VALIDATION.md for commands and the inventory snapshot.

The third pass declares 49 mutually dependent interface files and ports all 29
core lifecycle argument classes. Event arguments are cheap shared handles:
clones share cancellation/context changes and retain the original targets.
GlobalDockTrackingState holds weak references, while focus/activation arguments
hold strong context snapshots and resolve the current window host on demand.
DockWindowOptions retains its parent reference and applies fields in source order.

All 44 DockGroupValidationTests cases now run through the full IDockable/IDock
contracts without reflection. Added tests cover the distinct local/global group
rules, disabled global-docking ancestors, event mutations/lifetimes, dictionary
identity, option application and all 18 default factory command adapters.
Capability resolution covers all six defined capabilities and 324 combinations of
base/root/dock/override values, including factory/context/owner root precedence.

The following are explicitly partial: generic factory dispatch through erased
handles, adaptation of arbitrarily typed boxed model parameters, undefined numeric
capability evaluation, group validation's enumerator invalidation under reentrant
collection mutation, and RequiresDataTemplate metadata. Find's repeatable query
behavior also needs preservation when the factory is implemented. See
CONTINUATION.md and each file's deferred_members. These tests establish the
contract/helper subset; the later reactive model pass adds the first concrete
models. The production factory and docking UI remain pending.

The reactive model pass composes ReactiveBase and DockableBase into concrete
Document, Tool, ProportionalDockSplitter and GridDockSplitter models. All generated
properties, lifecycle synchronization, tracking callbacks and source defaults
are present; the base invokes notifications and virtual callbacks on the actual
outer model. An external consumer test covers custom model extension through the
exported forwarding macro. Tool retains its source document/MDI memberships.
Owner/original-owner/factory links are weak, following the native ownership guide;
contexts and navigation history retain their referenced objects. DataMember
metadata and equality of arbitrary custom boxed context values remain partial.

NavigateAdapter ports the source history/lookup/window algorithm. String lookup
checks direct visible entries first, then lazily searches snapshotted sibling
sequences in the source Flatten order, including split-view pane/content. Window
exit snapshots the list before invoking Save/Exit. Standalone adapter construction
retains its source dock, while the containing model uses the explicit weak path
to avoid a cycle. Navigation accepts boxed string
IDs and canonical erased dockable handles; concrete boxed model adaptation and
live-enumerator invalidation remain partial. Invalid argument kinds use a Rust
panic, following the shared System port convention. Fixture tests establish the
adapter behavior; the original four production-Factory integration tests remain
explicitly pending.

The container pass adds DockBase and all ten source ReactiveUI dock containers.
DockBase's active and focused observations use WhenAnyValue, including initial
values, distinct filtering and suppression behavior. InitActiveDockable runs before
CanGoBack/CanGoForward notifications. Standard construction initializes those
observations after Rc identity exists and before returning the model. Root commands
use NavigateAdapter; DocumentDock's creation and MDI arrangement commands retain
the source gates and factory call order. Command/factory auto-properties remain
nonreactive where the source does not raise notifications.

MdiLayoutHelper follows the original cascade/tile/restore algorithms, including
minimum clamp order, minimized-strip reservation, active-last Z ordering, active
documents absent from visible lists, and source snapshot timing. Positive infinity
remains accepted by the source bounds predicate. The current minimized count uses
a live index loop; fail-fast invalidation during custom getter mutations is still
partial. UI framework-model/headless integration remains pending.

Public composition constructors and forwarding macros now support deeper model
extensions. An object-safe bridge invokes typed DockableBaseImpl overrides on the
enclosing model, including calls through embedded concrete leaf components. Tests
cover virtual AddDocument/AddTool and split-view actions, external crate usage,
concrete sender identity, lifecycle synchronization and model release. RootDock's
Window backlink is weak under native ownership guidance. Serialization metadata,
arbitrary custom boxed-value equality and concrete boxed command-parameter
adaptation remain explicit partial items in the inventory.

The window pass adds the reactive DockWindow and source HostAdapter. All sixteen
properties retain defaults, equality and notification identity. The four virtual
close/drag callbacks use the same typed/object-safe composition pattern as the
other models. HostAdapter re-reads the current host between Save/Present
operations and preserves the source's captured original host during Exit. Optional
host factories receive the window model and never fall back to an ID locator when
they return null. Cancellation restores the prior tracked flag, completed closes
clear only the same host, and replacements survive. Rust unwind failures restore
tracking and resume the original panic payload; RAII resets the exit flag even
when the source catch-block restoration itself fails.

A public standalone adapter retains its model, matching the original constructor.
Model-owned adapters use from_weak to avoid native ownership cycles. This pass
corrected the earlier public NavigateAdapter constructor, which had required the
caller to retain its dock separately. Both source class APIs now expose inherent
methods in addition to their complete interface implementations, verified in an
external consumer without adapter-interface imports.

The late-null Layout case in Present is explicitly partial: if a callback clears
Layout during host configuration, C# forwards null to SetLayout; the current Rust
non-null parameter rejects it. Do not hide the difference by capturing an earlier
layout. Window serialization metadata, framework-property model variants,
headless host tests and production factory initialization are also pending.

The initial IList mapping was too narrow: source properties accept ordinary and
custom lists, while the early port admitted only ObservableCollection. The
factory pass corrects the public graph to ListHandle/IList, with plain, custom,
read-only and observable backends. The observable backend delegates to the
published FerroUI implementation and preserves its identity and reentrancy rules.
Plain/custom versioned enumeration now reaches the group, navigation, MDI and
factory algorithms; observable version invalidation remains explicitly partial.

FactoryBase is now a partial native composition component requiring the full
IFactory owner implementing the protected FactoryBaseImpl hooks. Locator state, keyed/default resolution, virtual recursive root
and dockable lookup, both nonvirtual Find overloads and both InitDockWindow
overloads are implemented. Normalization preserves notifying custom lists and
the actual concrete generic CreateList override through a construction-time weak
thunk. InitActiveDockable, ActivateWindow and SetActiveDockable also preserve
source dispatch and ordering. Recursive InitDockable, InitLayout, full focus
handling and global tracking now have source implementations. All event bodies, lifecycle notifications and window/options operations are now
implemented. Docking mutations and the concrete production Factory are pending.

Find returns a lazy repeatable DockableQuery. Each enumeration has independent
state and reads current layouts at MoveNext. It deliberately retains the source
Find overload's duplicate container yields and visible-only traversal; the
separate FindDockable method includes split-view extras and floating layouts.
Exceptions finish and clean up the active iterator; a new enumeration may be
started independently. These algorithms run through full-interface test fixtures
and concrete reactive window integration without substituting a production
factory implementation.

The state pass ports ownership/original-owner location resolution, hidden/floating
modifiers, recursive updates, state flag reconciliation and model-driven requests.
FactoryBaseImpl preserves protected virtual extension points on the enclosing
factory. The explicit state-sync interface routes through those overrides; native
scope guards restore suppression/coalescing during unwinding. Fallback selection
uses a lazy, deduplicating owned-item iterator that retains source model identities
and preserves live enumeration. State requests retain the source conversion
priority and re-resolve after each docking operation before deciding to float.

The 25 new regressions use concrete reactive models and a complete recording
factory. They observe or simulate pending operations; they do not establish
production hide/restore, movement, focus or floating parity. Observable collection
mutation-version invalidation remains deferred alongside production integration.

The initialization/focus/tracking pass connects visible normalization to recursive
context/owner/factory initialization, preserves hidden and four-strip pinned
ownership without assigning OriginalOwner, initializes split-view extras once,
and wires floating layouts with their existing owner. InitLayout retains default
selection, root fallback selection, source tracking resolution and ShowWindows.
Empty-state propagation and opened-count updates use the source container rules.

Focus transfer deactivates other registered focusable roots, preserves their
focused references, reactivates an existing focus when necessary and reconciles
selection flags. Focus does not implicitly change pane selection. Tracking keeps
its root/general resolver distinction, identity comparison and stale-root policy.
Observers run after state replacement and may reenter updates without a held
mutable borrow. Events use the published FerroUI System event implementation.

Thirty additional tests exercise these concrete algorithms with a full-contract
recording composition owner configured to delegate the implemented bodies. This
is not the pending production Factory; unused fixture methods still fail rather
than substitute docking operations. Lifecycle/close/drag/mutation events and
production factory integration remain explicitly deferred.

The lifecycle/window pass implements every FactoryBase event body and the full
IFactoryEvents contract. Closing starts from the model veto (or the optional
window preservation policy) and returns the event's final mutable cancellation
value. Closed/removed callbacks clear tracking, raise the event, then invoke the
protected lifecycle hook. The service list retains source order, optional entries,
live mutation-version checks and exception behavior. Repeated calls remain
repeated notifications; idempotent overlay cleanup belongs to its separate service.

Window operations retain added-before-initialized ordering, concrete generic
window-list creation, exit cancellation and ownership-change guards, captured
layout handling, callback-sensitive link cleanup and reentrant removal. Closure
snapshots visible children and requests descendants before containers. It still
depends on the pending production CloseDockable algorithm. Options capture the
source parent window before movement through native protected virtual hooks.

Twenty-five new tests cover these paths with composed reactive probes, real
HostAdapter behavior and a recording host/factory. The concrete host UI, overlay
lifecycle service, movement/close engine and full production Factory remain
unverified. Observable-list version access is still an explicit dependency gap.

The collection/mutation pass ports AddDockable/InsertDockable, all five protected
visible-list helpers, replacement selection, orphaned splitter cleanup, both
move/swap overloads, pinned reordering, reverse live close ranges and document
layout setters/wrappers. The helpers update empty state/counts without inventing
initialization or events. Add/insert initialize before mutating the current list.

The source overload differences remain explicit: the three-argument move removes
then inserts and selects the moved item; the four-argument same-owner move inserts
before removal and preserves selection. Swap overloads likewise keep their own
notification and selection order. Cross-owner transfer retains the source target
index rules and defers floating-source root collapse until the target commit has
succeeded. Splitter cleanup keeps source snapshot/candidate order, including
duplicate candidates and live membership rechecks during callbacks.

Close ranges use reverse live indices, unlike CloseWindow's per-dock snapshots.
Nineteen new tests exercise these contracts and partial failures with concrete
reactive models. The initial cross-owner removal/collapse tests use explicit callbacks; newer
pin/removal tests now execute those actual bodies. Full CloseDockable remains
pending and its dependency callbacks stay explicit. These tests do not establish complete movement/close/headless
engine parity.

The pin/removal/collapse pass connects collection movement to actual native
removal, selection/focus reconciliation and hierarchy collapse. Public pinning
evaluates capability, while protected pin core and removal/hide cleanup preserve
the source eligibility bypass. Pin alignment, whole-pair bounds fallback,
original-owner history, preview keep-visible/switch/toggle semantics and duplicate
strip cleanup follow the source order and counts. Explicit unpin destinations
retain precedence over automatic alignment/attachment selection.

Collapse distinguishes proportional splitters from other splitter types, honors
aligned nonempty pin strips, checks unlisted split-view content and preserves
proportion when flattening a single-child proportional dock. It removes floating
windows through the real cancellation-aware window implementation. Hide/restore
retain original owners, recursive state propagation and notification-before-owner
update order. ID hiding disposes the search enumeration before invoking mutation.

Twenty-six new tests execute these paths on concrete reactive models, including
real cross-owner removal/collapse and floating host closure after target commit.
The shared fixture supplies real ToolDock creation; it is not the pending
production Factory. Creating an unpin destination beside an existing anchor still
requires the unported SplitToDock, while empty-root creation and aligned-dock reuse
are tested directly. Full close/ItemsSource, floating/split construction,
framework/headless integration and existing dependency parity gaps remain open.


The split/floating pass implements both source split paths, share inference,
window creation/splitting, single/all floating, DockAsDocument and new document
panes. Direct insertion into a compatible proportional parent preserves parent
selection and initializes only the incoming dock. Fallback wrapping updates the
parent's active/default references and initializes the complete wrapper. Tests
preserve source proportion, group, splitter and event-payload differences.

Window creation retains ITool-before-IDocument classification, supported-container
reuse, nullable root selection and uninitialized ownership. Floating registration
uses the outer workspace root, captures parent options before movement, presents
through HostAdapter and retains source event/focus order. Position fallback is
per-axis on NaN; size fallback requires finite values strictly above sixteen.
Whole-pane transfer uses a sibling snapshot with a live per-item collapse policy.

Five complete ItemsSource/generator/selector contracts and native optional query
hooks enable template metadata copying without reflection. Composed test docks
exercise shared templates/themes/selectors/delegates, null overwrite and the
source distinction between metadata and ItemsSource/generator membership. The
non-generic IEnumerable/IEnumerator bridge is a declaration for a missing System
0.3.0 contract, not a duplicate production collection implementation.

Thirty-six new tests bring the workspace to 572 passing tests, including actual
anchor-based unpin splitting, preview-owner floating geometry and descendant
floating/hidden state propagation. The concrete production Factory and full
close/ItemsSource tracking are next. The original framework/headless control tests
and previously documented dependency/runtime parity gaps remain pending.


The close/ItemsSource pass ports the remaining source close bodies and source-item
tracking algorithms. Public close retains null, last-child and capability policy;
protected core bypasses eligibility while preserving cancellation, source removal,
hide/remove and final Closed notification. Document resolution precedes tool
resolution and uses current owner, tracked owner and visible-root search in order.
Recognition bypasses hiding even when Context is null or source removal fails.
Closing removes the current Context; reverse lookup retains the explicit tracked
item until its source control untracks it.

Native source tracking preserves duplicate container order, retracking, reference
removal and retained owner/container lifetimes. It reuses System Dictionary and
shared scalar/domain/canonical-handle equality/hashing. Expired noncyclic identity
keys are pruned with retained-value drops outside borrows. CLR ephemeron cycles,
pruning/performance integration and arbitrary boxed custom equality/hash metadata
remain explicit partial behavior, alongside the prior dependency/API gaps.

Thirty-four new tests bring the workspace to 606 passing tests. Actual native close
now runs through ranges, windows, source removal and real model selection/focus.
Synchronous source observers exercise untracking/removal before core removal and
partial failure after source mutation. Older request-only tests use an explicit
simulation flag. The production Factory and original framework/headless source
controls/generators, UI, serializers and platform integration remain unfinished.


The production factory/locator pass adds the concrete ReactiveUI Factory, full
factory/event/state-sync dispatch and typed ID locator casts. Source registry
identity, observable control/host lists, concrete creators, independent pinned
lists and virtual root/list creation are preserved. FactoryImpl/impl_factory!
provide composed application overrides without test-only bodies. The generator
validates dispatch against both native contracts and frozen C# virtual/override
classification; it emits forwarding declarations, not engine algorithms.

Eighteen new tests bring the workspace to 624 passing tests. They run source
state transitions, splits, modal hosts and close/focus through production Factory.
Downstream crates exercise generic/public/protected overrides, exact custom and
interface locator references, optional host membership and weak creation thunks.

Factory remains partial for general CreateList element/default equality and fully
erased generic dispatch. Exact concrete/interface locator casts are present;
concrete base-class casts of composed subclasses still need class/reference
metadata. Prior observable-version, weak-table, boxed-value, serialization/UI and
platform gaps remain open. The remaining docking service/manager/state and UI
engines are still unported; this does not establish full engine/API parity.


The service/manager/outer/state pass ports source docking decisions and execution
through the production factory. Local/global group rules, nested leaf selection,
metadata/delegate copying, source/target factory choice, capability provenance,
restriction/policy order, pane snapshots/tab order, pinned strip changes and
reverse live group validation retain their source distinctions. Observers preflight
before Begin, revalidate during execution, complete only on success and dispose
even on failure; native unwind handling preserves cleanup-failure precedence.

Outer splitting retains the source container/group and target region across all
four directions, checks enclosing/remaining content, preserves focused selection
and performs remove-without-collapse, split, then old-owner collapse. DockState
preserves content/templates in four ID namespaces across hidden/pinned/preview/
window/split-view/visible traversal, including duplicate overwrite, saved-null
behavior, current-owner fallback and reset. It does not replace serialization.

Forty-one new tests bring the workspace to 665 passing tests. Interface policy and
FactoryBase hooks replace source runtime interface/class casts. An object-safe
CreateList<IDockable> specialization dispatches the actual generic factory override
from the erased engine, verified in a downstream crate. General erased generic
APIs remain incomplete. The original framework-property/Headless harness, native
observable version/default equality/custom snapshot semantics, class/value/lifetime
metadata, UI, workspaces and serializers remain explicit work in the full goal.


Workspace snapshot and dirty-tracking algorithms now follow the source manager,
using DockState and actual factory events. Workspaces/options/event arguments are
native shared objects, with in-place recapture and source metadata/ID rules.
The generic IDockSerializer contract is present without a layout-only replacement;
DockWorkspaceManager currently carries the concrete serializer type because
arbitrary erased generic dispatch remains unresolved. Native borrowed stream
read/write/seek is present; complete System.IO.Stream support and real serializers
remain pending. The test identity serializer is not a production implementation.

Workspace IDs use a generated simple-case equivalence table, validated against
3,005 actual .NET OrdinalIgnoreCase comparisons. Reproduction and runtime/version
limits are documented in scripts/dock/workspace-reference/README.md. The source exact
active-ID removal check is kept separately. The Workspaces collection remains a
live read-only System Dictionary view, with a native captured-key enumerator
that preserves mutation results but adds allocation; shared runtime enumeration
and performance verification remain open. Fifteen integration tests cover all
12 original workspace test scenarios, plus metadata, nullable/failure, content,
collection, shared-option, reentrancy and lifetime cases. Framework-property,
Headless/UI, production serialization and broader runtime gaps remain explicit.


The complete FluentExtensions source surface now has native helper bodies: all
129 public overloads, with traits grouped by receiver contract. Methods consume
and return the same shared handle and keep its native type; `_into` overloads
express source output parameters. Document/Tool output assignment follows their
configuration callback, while other creator outputs precede initial setters and
configuration. The source defaults and virtual creator/add dispatch are retained.

Collection helpers use plain List fallbacks, append in live enumeration order,
reassign existing properties and reread them for every addition. They do not call
the factory list creator, initialize layouts, deduplicate or invent snapshots.
Native IntoIterator represents both source params and IEnumerable forms. Tests
exercise reentrant list changes, setter/callback failures, type/identity retention
and actual downstream overrides. All 14 original split-view/document fluent test
scenarios have counterparts. Shared runtime collection/class/generic metadata and
original framework-property/Headless integration remain explicit partial status.


Overlay request models, source DelegateCommand, per-host busy/reload and the two
remaining global scope services now have native source bodies. Shared System
Task/TaskCompletionSource, INPC/events and explicit disposables are reused; reload
is actual ReactiveUI WhenAnyValue plus ReactiveCommand.CreateFromTask. Requests
retain metadata/content/result/handle identity and complete once without invented
cleanup. The busy service preserves eager RunAsync scope lifetime, source callback/
notification order, cancellation/error identity and cleanup-error precedence.

The native borrowed ICommand value boundary currently requires canonical
BoxedValue handles for Dialog.Close; raw/unregistered object retention and complete
retained-command/CLR GC lifetime metadata remain unresolved. Native synchronous
async-boundary panics use System TaskWorkError diagnostics, not rich CLR exception
metadata. Cross-thread Interlocked/ConfigureAwait and original multiwindow/Headless
integration remain partial. ReadOnlyObservableCollection and the dispatcher's
asynchronous-continuation option still need faithful representation before the
remaining dialog/confirmation/hosting service graph can be claimed complete.
Twenty-eight new tests validate the implemented algorithms, not a usable overlay UI.


Per-host dialog/confirmation services now use the original creation, dispatch,
close and snapshot cancellation algorithms. A live read-only adapter wraps shared
System ObservableCollection storage: one collection/property forwarding pair,
original notification payloads and read-only IList failures. It follows the
[.NET read-only wrapper](https://github.com/dotnet/runtime/blob/main/src/libraries/System.ObjectModel/src/System/Collections/ObjectModel/ReadOnlyObservableCollection.cs),
without a second mutable storage engine. Shared observable mutation versions,
CLR event/GC lifetime and compiled binding metadata remain partial.

ServiceDispatcher.InvokeAsync now captures action errors and stores completion in
System TaskCompletionSource. Its internal Rust Future adapter forwards a pending
await's Send Waker through a worker to the System owner's queue/poster; all Rc/value
state stays on its owner thread. Already-completed awaits continue inline, matching
[the awaiter's completion check](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Private.CoreLib/src/System/Runtime/CompilerServices/TaskAwaiter.cs).
Tests prove pending wake deferral and deduplicated registrations. This does not add
TaskCreationOptions to the shared task API. The worker-per-pending-wake transport
needs performance/shared-runtime integration; cross-thread ConfigureAwait parity
and richer exception metadata remain open.

Generic dialog results apply the source exact-type/default rule after request
completion. Values retain zero/default and nullable-value distinctions, while
reference outputs use Option<Rc<T>> without cloning models. Known dockable casts
reuse the unchanged factory reference-query helper. DialogContent carries an
original object plus optional explicit IDockDialogContent projection, retaining
request Content identity and supporting early close before insertion. Automatic
unregistered class/interface metadata remains deferred. All new services stay
partial for these broader runtime/metadata/ownership and original UI gaps.


Host service resolution, fallback providers and window lifecycle cleanup now use
source algorithms. An instance-scoped weak native projection registry supplies
explicit interface membership that shared IScreen cannot expose. It preserves
actual object identity and rejects foreign projections; automatic unregistered
CLR metadata and custom screen equality are still partial. Lookup preserves host
window/last owner/factory/router order and root normalization. The provider retains
per-screen and reentrant fallback behavior, correct-key eviction and nearest-root
lookup; native weak-key caches still lack CLR ephemeron cycle collection.

Lifecycle marks once before lookup/callbacks, uses direct/resolver/cache-only-or-
custom-provider precedence, cancels dialogs then confirmations and rereads current
Layout for Dispose. Missing services or thrown cancellation do not undo marking.
Fifteen native regressions include both original lifecycle scenarios and production
factory event dispatch with actual services. Full UI/Headless, class/reference/
collection/runtime and GC/performance parity remain explicitly unverified.


Navigation/routable continuation: the actual DockNavigationService, static helpers,
three routable concrete models, three abstract base components, DockViewModel and
BusyRootDock now have native source bodies. The interface and static helpers are
ported; model/service records stay partial for explicit metadata/lifetime and
original application integration gaps. All seven original navigation cases and
sixteen additional regressions bring validation to 794 Rust tests (100 services).
The source inventory is 140 ported, 105 partial and 1,683 pending out of 1,928.

Navigation retains independent factory/dock lookup, attached fallback, root default
then direct visible child, owner membership before readdition, no ID deduplication
and the source activation/focus/float branches. BusyRootDock descends a nonnull
active child without parent fallback; otherwise it checks only the screen's top
routed item, then itself, for IReloadable. Dispose marks first, removes reload,
cancels dialogs then confirmations, and does not retry after failure. Actual shared
ReactiveUI commands/routers, System tasks and the production docking engine execute
these paths. Abstract document/tool helpers pass the enclosing object as both
screen and dockable. DockViewModel initializes only its constructor-created layout
and resolves all overlay getters against its current layout.

Composition must preserve the enclosing runtime identity in typed constructors and
initialize inherited DockBase observations after Rc construction. Model forwarders
must not create extra routers or model state. Native projection registration is
required where shared erased screen/routable contracts cannot expose class/interface
membership. It does not resolve automatic CLR metadata, custom class names,
serialization/compiled binding metadata, or shared weak-host versus CLR GC lifetime.
Activation, DI/dispatcher and framework controls/themes/markup, serializers,
alternative model stacks, samples and earlier runtime gaps remain in scope.


Activation/dispatch/Splat continuation: IActivationBridge and IDockDispatcher
contracts are mapped, with native DockActivationBridge, MainThreadDispatcher and
DependencyResolverExtensions implementations. Seventeen new tests bring the
workspace to 811 passing Rust cases (117 services). Inventory totals are 142
ported, 108 partial and 1,678 pending of 1,928 source files.

The bridge calls the shared activator directly and does not inspect the required
view. The dispatcher always reads/schedules on the current Rx main-thread scheduler;
action errors fault the task while scheduling errors propagate. Source
RunContinuationsAsynchronously remains incomplete on raw System Task. Do not hide
this behind the private overlay await adapter or another task implementation.

Splat registration preserves nine lazy singleton defaults, order, custom/null/named
registrations and original required-service branches. A resolved Func creates
fresh local services around the registered globals and repeats all six global
lookups. Native erased mutable/read-only interface projections must belong to the
same allocation. Copied registration closures retain the original resolver; do
not switch them to the copied resolver or use weak captures that lose their source
resolution context. Native cycle collection and rich CLR metadata remain partial.
The registered provider/lifecycle/BusyRootDock/factory path has an integration test.

Microsoft IServiceCollection support is still pending for its missing shared
dependency, not waived. Next proceed to framework-property models/control adapters,
styles/themes and compiled markup while resolving shared runtime/metadata/ownership
requirements. Original UI/Headless/leak and sample integration remain necessary.


Initial reactive framework control continuation: a separate services framework
crate now ports DockReactiveUserControl using actual FerroUI class macros,
ReactiveUserControlBase and typed ReactiveView. Four Headless cases bring the
workspace to 815 Rust tests. Inventory is 142 ported, 110 partial and 1,676 pending
out of 1,928 source inputs. Source control and project records stay partial.

Keep the parent constructor/property/activation behavior. Resolve the bridge once
per activation, add SerialDisposable before subscribing to ViewModel, acquire the
next tracked scope before disposing its predecessor, use direct activation when
no bridge exists and assign Empty for null/nonactivatable values. Detach disposes
the subscription and activation resources. Native erased activatable membership
currently requires the existing same-object registry before binding/activation;
do not introduce a parallel activation engine or claim automatic CLR discovery.

The native abstract base has no markup default constructor. Type/namespace/assembly
and public Rust paths are registered; shared build-scan exports the library with
its metadata dependency chain. Consumer compiled-XAML verification, full generic/
binding/class metadata and original desktop/leak tests remain pending. The four
Headless tests cover a reactive view's supported lifecycle and model release, not
the original DockControl activation-order leak scenario. The visual host resolver
requires still-pending real Dock controls and must not use production placeholders.


Proportional control continuation: the separate panel/splitter library now has the
source native class/property/layout/input/preview bodies, eight internal sizing
helpers and a genuinely compiled splitter XAML theme. Twenty-six Headless cases
bring workspace validation to 841 Rust tests. Inventory totals are 149 ported,
121 partial and 1,658 pending out of 1,928 source inputs.

Retain Vertical default, TwoWay attached metadata, SetCurrentValue proportion
updates, parent invalidation and the assigning flag reset on failures. All five
policy hooks must dispatch to the enclosing class. Preserve source repeated
fractional calculations, normalization/collapse snapshots, constraint order and
live indexed loops. Splitter parent calls come first; orientation pseudo-classes
accumulate as source. Presenter wrappers and skipped siblings use the actual
source navigation algorithm.

Preview object initialization completes before assigning the field. Mutable preview
fields are reread after callbacks at the same points as source. Capture loss and
release retain source guards/order, even when resizing was disabled during a drag.
Painting calls parent first, then explicit/host-accent/Gray brush precedence and
orientation-specific rectangle geometry. Tests observe actual Headless drawing
commands while forwarding to its backend; no production renderer is substituted.

The theme compiles through Build/Model with typed scopes and its TemplateBinding
is tested after application to an attached native control. Namespace metadata must
use the named NAMESPACES table so scanner and runtime agree. Original ItemsControl,
remaining min/max/runtime-binding cases, full consumers, desktop/pixel/leak/target
coverage and source Debug.Assert diagnostics remain deferred. The native iterator's
copy-on-write storage check covers content mutation but cannot claim exact original
version semantics for capacity-only changes; utils/manager stay partial. All main
Dock UI, resolver, serialization, model alternatives, samples and earlier gaps
remain in the full goal.


Proportional integration continuation: all main, MinMax and SplitterHeadless
source cases now have native counterparts; the three test files are recorded as
ported. Inventory is 152 ported, 120 partial and 1,656 pending out of 1,928 inputs.
Workspace validation is 862 Rust tests, including 46 proportional Headless cases
and a separate dependent compiled-XAML case. Source rows can be grouped into a
single native function; the correspondence lists retain every original method.

Retain the source no-template ItemsControl zero-bounds assertions. A separate
ItemsPresenter case exercises actual nested item-panel generation. Compiled paths
against production reactive Document models use shared INPC accessors and default
TwoWay metadata at both local and style priority. Normalization/resizing preserve
live bindings and model replacement removes old subscriptions. No substitute
view-model synchronization layer or reflection is added.

The unpublished tests/Dock/FerroDock.Markup.Tests workspace member validates the real
library export/consumer boundary. Its XAML resolves the custom namespace, attached
property metadata/compiled paths and source splitter dictionary ResourceInclude;
Headless verifies implicit theme visuals, layout and two-way updates. The test
crate uses the compiler-default ferrodock_markup_tests assembly URI. Complete
original docking themes, automatic heterogeneous interface/model metadata,
property-model variants and all earlier runtime/lifetime/platform requirements
remain open. A supported consumer fixture does not establish a usable DockControl.


Native recycling continuation: source ControlRecycling and ControlRecyclingDataTemplate
now run on actual native FerroObject classes, native direct/styled/attached
properties, System Dictionary, shared data/recycling templates and the real
visual/logical tree. All original cache update/miss, ID lookup, detachment and
fallback bodies are present. Their records remain partial for explicit raw domain
metadata/equality, arbitrary custom hash/reentrancy, class/reference/CLR cycle
lifetime, full consumer and platform gaps. Inventory is now 156 ported, 123 partial
and 1,649 pending; full workspace validation is 882 Rust tests.

Keep cache and template-wrapper fallback semantics distinct, including null return,
existing/null argument order, replacement-cache-before-detach and unchanged cache
on failed fallback. Visual presenter detachment wins over a different logical host
content. Current-value clearing and actual presenter UpdateChild preserve bindings.
Source Add/TryGetValue do not translate IDs; Build/Remove do. No cache/member is
silently omitted or replaced by an identity-only production fixture.

The native AnyValue/domain Any conversion belongs to the UI library. Native class
handles use shared ValueTypes projections; a temporary probe is not a stored
property/model wrapper. UI-owned metadata carriers name the original foreign
trait handles through shared macros/export, avoiding orphan impls and a domain UI
dependency. Remaining interface member/notification and arbitrary raw domain
metadata must still be completed. Strong ownership follows source; tests prove
supported release/disposal paths, not CLR cycle collection.


Native overlay host continuation: FerroDock.Ferro begins the original UI assembly
with OverlayHost, OverlayLayer, layer collection/query, source content/layer/factory
contracts, provider registry and original style keys. Actual native property,
visual, template, dispatcher and resource engines execute the source pipeline.
Structural equivalence, service/factory precedence and collection identity,
repeatable stable ordering, callback getter order, all wrapping/detach branches
and dispatcher coalescing/reentrant follow-up are retained.

Native tests include all four original rebuild-regression method bodies; their
source file stays partial because the original themed application harness adds
unported implicit service overlays. A third external compiled-XAML consumer
exercises real host/layer/collection/interface/content/static metadata. Plain
collection nullable forms are explicitly registered through shared ValueTypes.
The full goal retains built-in service controls, original templates, DockControl,
native/managed windows, host resolver and all previous runtime/API requirements.

Current validation is 901 Rust tests. Inventory is 160 ported, 130 partial and
1,638 pending of 1,928 inputs. Registry threading/delegate equality, arbitrary
raw object resource keys, complete inherited collection/interface metadata,
mutation versions, native/CLR lifetime/GC and the full original themed/desktop/
leak/target harness remain open. No test-only control substitutes a missing
production busy/dialog/confirmation control or docking host.

The source Settings assembly now has a separate native counterpart. Its property
owner uses the shared attached-property engine and exact inheritance/TwoWay
metadata. Native plain options use Rc/Cell without notifications; builder extension
methods preserve the framework's shared builder and take Rust boolean parameters
explicitly for source calls whose omitted enabling argument is true. The source
static configuration remains process-wide. Callback Arc/Send/Sync requirements,
underlying KeyGesture reference/value adaptation, writable untyped static-field
metadata and default debugger output remain documented runtime gaps.

Model enum descriptors are owned by the lowest native settings dependency. Shared
TypeModel.rust_path describes the original enum; public_path describes the metadata
carrier. Both are needed for typed nullable-enum conversion without a replacement
enum. Existing hidden docking-UI descriptor access remains available as an alias.

## Native item drag helper and mutable source access

The source internal ItemDragHelper now has all algorithm bodies implemented in
FerroDock.Ferro. It uses shared native routed input, actual items/container and
selection APIs, native transforms and ZIndex, descendant ScrollViewer coordinates
and DispatcherTimer. Source docking operations still go through the original
IFactory.MoveDockable/MovePinnedDockable; native UI code does not duplicate the
model engine. Optional IDockItemDragPolicy discovery is forwarded through the
existing IFactory/FactoryImpl composition machinery to the original policy trait.

The shared native IItemsList only exposes read/notification APIs. Source non-generic
IList mutations are therefore resolved through shared ValueTypes to the existing
model ListHandle<Option<BoxedValue>> contract. mutable_items_list::from_ferro_list
wraps a shared FerroList without copying items; native list mutation still sends
its existing native notifications. Native registration includes concrete string,
boxed and nullable-boxed lists. Other item types and custom original IList
implementations need a concrete composition-root cast. This does not complete
arbitrary generic/runtime interface discovery, adapter notification/version or
untyped metadata semantics. No new mutable collection engine is introduced.

Tests exercise actual Headless pointer sequences, original policy theory-case
behavior through ReactiveUI models, pinned and visible list moves, source visual
reset/selection ordering and native asynchronous auto-scroll. They also preserve
the source Tunnel capture-loss subscription even though the native event is Direct;
the original handler body is separately tested with native event arguments.
The actual tool/document tab and pinned item consumers, original complete themed
application, alternate model and forced-GC leak tests remain pending. Source helper
integration stays partial, with a temporary module-scoped production dead-code
allowance until those consumers are ported.


## Native docking targets and floating adorner host

DockTargetBase, DockTarget, GlobalDockTarget and DockAdornerWindow now use the
shared class/template/property/visual/window system. Original contracts and
virtual/protected extension points remain available. Target mappings use shared
System Dictionary enumeration order with live reads outside callbacks; its
unexposed version/removal-slot protocol remains an explicit runtime gap. Native
weak template/part restoration, selector-root coordinate translation and source
indicator-only bypass/opacity behavior are preserved. Native accessibility peers
and Invoke provider follow the source behavior, with shared weak peer owner
retention remaining distinct from source strong readonly fields.

Complete source local/global/adorner-window Fluent dictionaries are compiled with
explicit types and native namespaces. Original PNG selector assets are embedded
unchanged under the actual theme assembly. The independent compiled consumer
verifies actual visuals, image sources/sizes, operation resolution and original
conditional/window styles. Complete AdornerHelper/managed-window/state/control
integration, original full application/leak harness and desktop/platform/pixel/
performance/runtime metadata requirements remain pending. No smaller DockControl
or alternate docking engine is introduced to hide these dependencies.
