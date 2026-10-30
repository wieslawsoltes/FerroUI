# Current delivery: standard ReactiveUI core catalog

A usable production catalog now ships in FerroDock.Serializer.SystemTextJson.ReactiveUI.
It covers the14 standard layout models and DockWindow through the existing DockSerializer,
with no caller-assembled test metadata, UI dependency, reflection or replacement model.
See [usage and limitations](#standard-reactiveui-core-catalog). The general compiler/default
cross-backend catalog remains incomplete and is outside the reduced delivery priority.

# Serializer port status and requirements

The SystemTextJson resolver-based overload now has an actual Rust implementation in
src/Dock/FerroDock.Serializer.SystemTextJson. The library is partial. Supplied compiled
metadata now supports reference casts, registered polymorphism, base fallback and
configured list creation. The emitted object-payload helper runtime is also ported with
property-local compiled converters. The default constructors, complete original model
catalog and automatic Rust compiler adaptation remain required before ordinary sample
persistence. Semantic selection and executable Rust emission now exist as separate stages.

## Implemented pipeline

Serialize/Deserialize/Load/Save retain the original declared generic type, source null
versus failure distinction and supplied metadata resolver. The Core serializer interface
and workspace manager use Result, following the FerroUI guide's parse-error convention.
DockSerializationError retains its original shared cause through Capture/Restore/TryRestore;
failed operations do not commit payload, active workspace or clean state.

Compiled typed descriptor tables call original getters/setters instead of reflecting
fields. Metadata is cached per options instance; object reference IDs and read tables are
independent per call. Classes register their original instance before property setters,
allowing self-reference cycles. Readonly properties serialize, nullable properties are
omitted before conversion, and null decoded values remain distinct from malformed input.
The generic JSON DOM retains property order, duplicate names and lexical number tokens.
String escaping and payload indentation match the recorded original outputs.

Generated object-property payloads now use compiled type/name registrations and the same
original shared class handles. The helper deliberately starts a fresh reference scope for
each typed property, as the emitted source SerializeToElement/Deserialize calls do: separate
object properties do not share instances after load, while each typed payload retains its
internal self-reference. Its tag precedes the inner $id, distinct from interface polymorphism.
Unknown tags, multiple string tags and untagged values remain raw JSON, including duplicate
properties and lexical numbers. A known late tag is removed before typed decoding. Null
properties are omitted, while a boxed raw JSON null is written as null. Unregistered runtime
types and registered converters returning non-object JSON fail.

Explicit property converters take precedence over resolver codecs; nullable wrappers retain
the underlying supplied codec without installing it globally. Get-only property converters
write values and ignore read input. Preserved class reads reject unexpected dollar-prefixed
metadata, including mapped $type properties. The source helper also supports JsonDocument;
the Rust document ownership/disposal branch and complete HandleNull/callback/depth/error/
performance behavior remain unimplemented or unverified. Raw elements use the existing DOM.

Builtin scalar descriptors now include all eight signed/unsigned 8/16/32/64-bit integers
and Single/Double alongside String/Boolean. Floating-point values use the existing locked
Ryū dependency for shortest roundtrip digits and source general-format notation. Named
NaN/infinities serialize as strings; finite quoted numbers are rejected under the source
options. Negative zero and the recorded overflow/underflow behavior are preserved.
Integer codecs reject fractional/exponent tokens, quoted numbers, null and range overflow.
NaN reads use the source canonical value; arbitrary NaN payloads do not survive JSON in
the original either. Exhaustive bit-pattern and performance parity are not established.

Compiled reference conversions resolve the same instances across concrete and interface
declarations. Registered polymorphic codecs keep $id/$type/$ref order; unknown interface
types without a base constructor fail rather than fabricating models. An explicitly
supplied base descriptor supports source fallback and ignored unknown discriminators.
Fallback writing uses the same object-depth guard as normal class writing, stopping
before the 65th object's getter. Policy-aware metadata now distinguishes exact runtime
classes from assignable views, supports base fallback and selects the unique most-specific
compiled ancestor. Ambiguous matches fail before a derived getter runs. Abstract/interface
schemas write declared fields and resolve existing references without inventing constructors.
Automatic production of runtime/cast/ancestry facts for all backends remains open.

The list-type/resolver constructor retains the configured observable, plain or custom
compiled IList creator. Lists keep their own IDs, aliases and null entries, register
before child decoding and use the actual shared model collections. Complete generic
creator validation and malformed-metadata callback ordering remain unverified.

Actual ReactiveUI RootDock, DocumentDock, Document and DockWindow fixtures roundtrip
against source wire goldens with an explicit limited property projection. These cover
owner references, active/default aliases, root/window cycles, repeated windows, shared
lists and original factory initialization after load. Pinned bounds, six dockable capability
flags, nullable overrides and shared root/dock policies now retain their original fields
and property order. Loaded policy precedence and shared policy mutation run through the
original DockCapabilityResolver in both direct and emitted-context tests. DockingState and
concrete allowed-source/target operation masks now retain signed bits and the original
operation rules. Optional restriction fields remain on backend class schemas; root-interface
fallback metadata does not invent those members. These checks do not establish full model
catalog or sample layout serialization. Owner, RootDock.Window and DockWindow.ParentWindow
now retain their original objects, matching source reachability. Rust Rc does not collect
cycles: CLR-compatible cyclic lifetime remains an open requirement. No serializer sidecar
cache or second model graph is used to keep them alive.

Load and Save use the original optional shared stream. Constructor validation happens
before a close scope exists. Save serializes before validating its output stream. Successful
and failed operations close the stream; a close failure replaces an earlier body failure.
UTF-8 preambles, UTF-16/32 BOM detection and output preamble suppression at nonzero position
are implemented for the tested cases. Exact framework buffering/read/write callback order,
all malformed encoding cases and the complete stream framework surface remain unverified.

## Source evidence

The serializer reference compiles four unchanged original serializer/options/resolver
files and references original Dock.Model and Dock.Model.ReactiveUI. Thirteen formerly source-only JSON checks now
have Rust counterparts using supplied typed metadata. Twelve additional cases verify
stream read/write/flush errors, close precedence, constructor failures and serialization
before null-stream validation. Six string outputs and a full payload golden verify wire
format. Seven original workspace assertions verify original error identity and unchanged
state on failure. The existing37 stream/interface checks remain matched.

JsonModelReference.cs records 44 additional model/list/reachability assertions and two
wire goldens. Its resolver deliberately removes properties outside the recorded projection.
The projection now includes proportions, collapsed proportions, size constraints and window
coordinates/sizes, with NaN, infinity, signed zero and finite values. A coordinate initialized
to +0 remains +0 when its original setter receives -0; the fixture preserves that behavior.
CoreModelReference.cs adds ten assertions, 49 input/replay cases and six wire goldens over
actual original Core models. Rectangles retain mutable value semantics, signed zero and
named nonfinite components; they never acquire reference IDs. A leading string $id is
accepted and ignored for value objects, while other reserved metadata fails. Policy and
override codecs retain the six nullable flags, omit nulls and exclude HasAnyOverride.
Compiled reference casts share one original instance through base/interface properties;
a base-first read cannot later fabricate the derived override type. The policy interface
has no invented constructor. Tests also reject unexpected reserved properties even when
a property explicitly maps that dollar-prefixed name.

core_model_type_metadata() explicitly supplies 28 descriptors, including nullable
dependencies. Supplied contexts do not implicitly gain a model catalog. The emitter can
compose these compiled dependency collections; the actual-model consumer registers Core
bounds, policies, get-only points, shared docking options and three signed flags types
this way. Automatic discovery and full default/application catalogs remain
unfinished.

CoreValueReference.cs adds ten source assertions, 490 read/replay rows and four goldens.
DockPoint writes its actual get-only components but uses the original default value when
reading; its parameterized constructor is not inferred. Incoming getter values are skipped,
even when they are not numeric. Shared DockManagerOptions retain the original default true,
explicit false values and alias mutation. DockingWindowState, DockOperationMask and DragAction
use default numeric enum JSON with all signed Int32 bits retained. Token/range boundaries and
128 seeded values per flags type match the source. Nullable forms retain null. An explicit
Int32 converter changes integer properties without changing enum conversion, as in the
original. The numeric codec does not delegate to a scalar metadata resolver.

DockWindowOwnerMode and DockWindowState now use safe transparent Int32 storage, retaining
all signed values, original names, defaults, equality/hash and four-byte layout. Raw numeric
casts use from_raw/to_raw; the existing TryFrom remains a checked declared-member helper.
Their native enum descriptors still export the original enum kind and named constants, and
runtime numeric/nullable conversion now preserves unnamed values. Shared compiled XAML now emits unnamed values through explicit Core SDK Int32 factories
in both model and runtime hosts. The semantic source reader now records literal transparent
single-field storage. With an explicit shared Int32 factory, the binder recognizes those
actual source structs as logical enums without marking them as flags. Complete inherited
System.Enum APIs and automatic default/native enum catalogs remain open.
DockMode, PinnedDockDisplayMode and DockFloatingWindowHostMode now use the same original
Int32 representation and explicit factories. The Core collection contains 34 descriptors,
including all three codecs and nullable forms. The original source supplies 423 seeded
value/name/wire rows and 102 nullable/nonnullable read cases. Runtime and shared compiled
XAML preserve unknown values on real RootDock properties and settings options. Original
root/global host precedence remains unchanged, and DockModeConverter returns unset for
Center or unnamed model values. The native framework Dock enum remains closed, so its
unnamed reverse-converter case remains explicitly deferred. Other plain Rust enums still
need equivalent numeric representation and codecs.
No flags JSON result is used to claim full enum, model catalog or sample persistence parity.

WindowEnumReference.cs adds seven source assertions, 282 value/name/wire rows, 50 read/replay
cases and two options/shared-parent goldens. All four DockWindowOptions properties use actual
source accessors and constructors; ApplyTo preserves undefined owner modes, null/false
presentation values and the original parent object. The actual ReactiveUI window projection
now covers all 13 persisted properties, including WindowState, Topmost, OwnerMode, IsModal
and ShowInTaskbar; Owner/Factory/Host remain excluded by the original ignore contracts.
Numeric XAML consumers now match seven additional original compiler assertions, including
nullable owner values and signed endpoints. Oversized compiled XAML literals narrow into
Int32 exactly as in the original frontend; JSON continues to reject out-of-range numbers.
The emitter uses only declared typed factory metadata and retains named constants without
inventing constructors or marking nonflags enums as flags. Root/list goldens retain parent aliases and unknown state/owner values in direct and emitted
contexts. This remains an explicit projection for the other three model classes, not a full
default/application/backend catalog or runnable sample persistence.

JsonFallbackReference.cs records five depth/getter cases and two fallback identity cases,
plus the self-reference wire golden. Rust tests consume those files. The initial fallback
test demonstrated an extra getter at depth 65; the shared depth guard fixes that divergence.

JsonNumberReference.cs records 1,362 exact numeric wire/read cases, including 512 seeded
bit patterns for each floating-point type, selected boundaries/named literals, canonical
decoded bits and all eight integer ranges. Rust tests consume the corpus through the actual
generic serializer. Original C# serializer/options/model sources remain unchanged.

GeneratedPayloadReference.cs builds and runs the unchanged original incremental generator,
using its emitted resolver rather than a manually copied C# converter. Twenty assertions,
25 input/replay rows and eight wire goldens cover the helper runtime, independent reference
scopes, closed generics, explicit/get-only/set-only converters, duplicates/raw null and failures.
Runtime and compiled generated-consumer tests match that evidence. JsonPolymorphismReference
adds 15 write/16 read policy cases, five graph checks and a shared-holder golden for exact,
base/nearest fallback, interface ambiguity and empty configuration. Error categories are
bridged; full CLR exception types/messages and metadata configuration timing remain open.

The new compile-time library FerroDock.Serializer.SystemTextJson.Generators consumes
explicit type/property/assembly facts rather than inspecting runtime objects. It ports the
semantic GenerationModelBuilder: early registration guards, activation/missing-contract
ordering, local type discovery, accessibility/friend/nested/generic checks, deduplication,
inherited/overridden/hidden property ignores and interface unions, context closure and
unique names, seven sorted polymorphism plans and collision diagnostics DSTJ001-DSTJ004.
Names use source ordinal UTF-16 ordering; full-name construction retains nested generic
arguments, arrays, tuple underlying types and qualified assembly identities.

scripts/dock/json-generator-reference runs the unchanged original generator against 19 C#
compilations. Public Roslyn facts are exported separately from the expected emitted source
plans and diagnostics. Rust compares those plans, messages and source spans; 152 additional
checks compare compiler-formatted names with actual runtime FullName values. Malformed
consumed symbol links/cycles return adapter errors. The compiler library has no production
dependencies and is not installed in the runtime serializer. These are compiler symbol
records, not docking models or a replacement runtime engine.

The Rust emitter consumes resolved type/property/family bindings and writes executable
context/resolver/factory code. It calls original constructors/accessors, applies source
ignored names and custom converter precedence, emits nullable/object and get/set-only
properties, preserves reference casts and produces policy-aware family ancestry tables.
Type-level/value/custom codecs can be supplied as compiled factory expressions. Missing
bindings fail explicitly; inactive generation emits no code.

tests/Dock/FerroDock.Serializer.Generated.Tests builds and executes two generated contexts.
The actual ReactiveUI RootDock/DocumentDock/Document/DockWindow projection matches the source
golden and runs original Factory.InitLayout over the same decoded instances. Source payload
fixtures also compile and match generics, independent scopes, explicit/read-only/set-only
converters and duplicate tags. These are explicit LIMITED field bindings. They do not
constitute the default catalog or automatic complete model metadata discovery.

The separate build-time library `ferrodock-serializer-system-text-json-generators-rust`
now binds a shared scanner `TypeModel` to `RustTypeBinding` with `bind_declared_type`.
It consumes source declaration order, resolved types and call forms; public/crate-private
paths use typed calls, while invokers require the declaration macro's public typed function.
It retains JsonPropertyName, unconditional JsonIgnore, separate property attributes,
get-only/set-only members and explicitly resolved JsonConverter factories. Type converters
bypass class members. It introduces no runtime metadata lookup or UI dependency.

Seven payload fixture classes now use this bridge. Their build-only declaration projections
in `tests/Dock/FerroDock.Serializer.Generated.Tests/payload_metadata.rs` are scanned against actual
Rust application fixture accessors, then compiled and executed against existing C# wire goldens.
The two closed generics and actual docking-model projection remain explicitly bound. This
is declaration binding, not automatic semantic discovery or a generated default catalog.
Identity, System.Object value-type mapping, converter factories and pre-resolved inherited
properties are still explicit compiler inputs. Source ignored-member rules remain applied
by the semantic plan/emitter using effective JSON names, matching the original resolver.

The adapter reports unresolved types, unselected cfg, absent inheritance decisions,
unsupported call forms, fallible accessors without explicit error transport, fallible
constructors and unsupported JSON attribute contracts. It requires the target declaration macro's typed functions where an accessible direct path is
unavailable. It does not synthesize missing class constructors or discover hidden fields.
The shared scanner still lacks some original accessibility/abstract/generic facts. Complete
semantic adaptation, incremental host/cancellation, complete registration/attribute integration and
exhaustive property/value/constructor/attribute binding remain required. No ordinary sample save/load
path is claimed. Exhaustive compiler diagnostics, malformed-source, parameterized constructors,
all JSON attributes and performance parity remain unproven beyond the recorded cases.

Regenerate the semantic compiler corpus with the installed .NET10 SDK:

```sh
cd scripts/dock/json-generator-reference
TMPDIR=/Volumes/SSD/temp/ferrodock-port NUGET_PACKAGES=/Volumes/SSD/NuGetCaches/packages NUGET_HTTP_CACHE_PATH=/Volumes/SSD/temp/ferrodock-port/nuget-http dotnet run -- ../../../../tests/Dock/data/json-generator-semantics.json
```

The actual source includes readonly payload properties and throws on empty Load, while
closing the stream. Two older source unit-test expectations contradict those outcomes;
port the measured current implementation, not those stale expectations.

## Actual Rust declaration facts

`read_source_declarations` follows the shared scanner's real module/file graph. It reads
native struct/enum/trait/union kind, declared visibility, type/lifetime/const parameters,
bounds/defaults, where clauses, explicit supertraits and trait impls, file/line/column and
inherited cfg/cfg_attr. Inline modules, literal #[path] modules and raw identifiers retain
their scopes. The shared production test-only rule excludes cfg(test)/all(test,...) items;
other target conditions remain unevaluated. Registration discovery shares that rule.

`transparent_field` records the normalized storage type only for a literal
`repr(transparent)` struct with exactly one unconditional field. Named and tuple fields
are supported. Other layouts, multiple fields and conditional field selection remain
unclassified. This fact alone declares neither an enum nor a codec. Semantic binding of
nonflags Int32 enums additionally requires Enum metadata and its explicit typed factory;
unsigned storage, missing or malformed factory names and unresolved target conditions
remain errors. Actual window owner/state declarations exercise this path. Full automatic
catalog discovery, alias expansion and non-Int32 enum adaptation remain required.

Known public reexports canonicalize through the shared ModelSet. The overload
`read_source_declarations_with_models` accepts the compiler's source/dependency model set,
including the scanned crate. It does not expand type aliases or synthesize missing dependency
facts/macro-generated declarations. Definition lookup requires the same canonical identity
and returns no match for ambiguous conditional declarations or unsupported wrapper forms.
Tests read actual original-engine IRootDock/IDock/IDockable declarations and verify that
reexports retain those same type identities. A referenced Rust source fixture verifies the
dependency export path too.

The compiled payload consumer now uses actual Rust visibility, generic declaration arity
and locations. Logical class abstraction, serialization names, property/contract graphs and
closed-generic symbol substitution still require explicit compiler facts, currently including
C# snapshots. Rust struct kind does not decide whether its source counterpart is a reference
class, value type or abstract composed base. Absence of a default constructor does not decide
abstractness. Export/emission-scope accessibility also remains separate from declared visibility.
Complete symbol construction must join these real declarations with the full shared class/
property metadata and dependency identities before the production catalog can omit snapshots.

## Native declared semantic symbols

`bind_declared_semantic_type` constructs non-generic logical class/value/enum/interface
headers and their declared public instance/static property symbols. It reads logical kind,
property names/types and direct/separate IgnoreDataMember tags from shared metadata; source
syntax validates representation/provenance. Logical serialization name/namespace, assembly,
context accessibility, abstraction, containing/base type, complete ordered interface closure
and actual property override links remain explicit resolved compiler inputs. A markup alias
is not implicitly used as serialization identity. JSON renaming does not change the source
property names used by the original ignore algorithm.

Type/property references resolve through the compiler table before mutation. Appending a new
type supports self references. Existing type slots can be rebound, with newly appended property
symbols; the future incremental host must rebind dependent declarations/override links too.
The function does not provide whole-graph incremental invalidation. It validates declaration
paths/kinds, class/value/interface abstraction, base/containing cycles, direct/transitive
interface closure and override ancestry/static shape. Missing types, unsupported generic
substitution, unselected cfg, registered properties or indexers fail rather than fabricate facts.

The compiled payload consumer replaces the seven ordinary classes' old C# headers/property
schemas using these native symbols. Its build-only declarations explicitly choose the original
wire names, and a native assembly identity is registered. The source compiler's dependency/type
slot seed and two closed generic examples are still required; this is not yet snapshot-free
production compilation or a full default catalog. Actual wire/identity/engine tests pass with
native class schemas. Nine additional semantic tests exercise fresh/self symbols, original
ignore/static/command/override/hiding behavior, atomic errors, explicit abstraction, native
logical kinds and serialization identity independent of markup aliases.

## Closed generic metadata and symbols

`instantiate_generic_metadata` specializes a shared template against an actual constructed
Rust type and its real generic declaration. Type parameters are replaced through parsed type
syntax across handles, properties, base/interfaces, member signatures and type-valued attributes.
Member names and literal attributes remain unchanged. Typed callable generic arguments and
call forms are specialized too. Parameterized public/stated paths retain their exported spelling.
A normalizer using the actual template module/shared models must resolve the resulting types.
The original template stays unchanged on success or failure.

`bind_closed_generic_semantic_type` validates the constructed path/definition, source parameter
order/arity, metadata arguments, logical metadata arity and recursively closed compiler type
arguments before constructing headers and substituted property symbols. Logical serialization
identity, source abstraction/access/inheritance and override facts remain explicit. The original
full-name algorithm still formats generic arguments with their registered assembly identities.
It shares the ordinary semantic member/ignore/static/override checks and atomic commit.

Both original integer/string generic payloads now instantiate one actual Rust metadata template,
construct native semantic symbols and bind their original generic constructors/getters/setters.
The hand-written per-type property/getter branches are removed. All nine payload schemas now
use native construction. The docking-model projection remains explicit and limited.

These APIs support closed, non-nested type parameters. Const/lifetime parameters, nested or
captured generic arguments, ambiguous parameter-associated projections, macros, inferred types
and unselected cfg require complete compiler resolution and return errors where unsupported.
Generic bounds/constraints remain on actual Rust definitions and are checked when emitted calls
compile; this is not a separate complete constraint solver. Full native dependency construction and discovery for all source forms,
alias/projection/access/cfg/macro/incremental integration and
production catalogs/sample persistence remain required. Codecs for all generic value domains
and CLR/system identity/lifetime/error/performance contracts remain partial.

## Native application symbol allocation

`bind_semantic_declarations` allocates contiguous IDs from actual native declarations and
binds their headers/members in a private compilation. The resolver receives that allocation
in declaration order, retaining explicit dependency and nullable/shared-handle mappings.
Properties can refer forward or mutually; closed generic arguments can refer to later native
headers and are validated after all declarations bind. Base/containing/interface headers must
precede dependents, and override IDs must name real property rows. No placeholder type symbols
or runtime instances are created. Errors, duplicate identities and missing/out-of-batch
references leave the entire input unchanged. Activation, registrations and discovery remain
explicit compiler-host decisions, separate from the transaction.

The generated payload consumer now discards every C# application type/property/assembly slot,
registration and discovery/name entry before building native classes. Ordinary metadata-declared
classes follow actual Rust source traversal; closed constructions follow actual registrations.
All nine application IDs come from the batch binder. Dependency imports retain only the
transitive required-contract/builtin closure, with remapped type/property/assembly IDs.
Those dependency facts are still explicit C# test-reference inputs, including the actual
IgnoreDataMember contract; they are not fabricated native descriptors. Logical wire identities,
base/access/abstraction and handle mappings remain explicit. This is not a complete native
compiler, exhaustive source discovery or the full docking-model catalog.

Eight batch regressions cover forward/mutual properties, late rollback, duplicate identities,
out-of-batch targets, forward closed arguments, late open-argument failure, dependency order,
actual override rows and independence from deliberately corrupted discarded application facts.
Existing consumer wire goldens and original-engine post-load assertions remain unchanged.

## Docking-mode properties in provided model contexts

The existing direct and emitted four-model contexts include original Dock and nullable
PinnedDockDisplayModeOverride, plus root PinnedDockDisplayMode and FloatingWindowHostMode.
The descriptors call actual original model accessors. Root and document docks retain unnamed
signed modes, document overrides retain Int32.MinValue, and absent/null root overrides remain
null. The original factory initializes the restored graph without changing those values.

Property order follows compiled original serializer output: root pinned mode precedes window
properties, floating host mode precedes root policy, Dock follows Proportion and pinned
override follows CanPin. Updated root/list goldens match exact bytes in both Rust contexts.
Six new source assertions expand44 to50 while retaining every previous case. Removing only
these new scalar property families leaves all prior property values/order/reference IDs;
43 other reference outputs remain byte-identical. Complete model schemas, automatic compiler
bindings, default/backend catalogs and actual sample persistence remain required.

## Native receiver projection

RustReceiverProjection and project_type_binding adapt a bound class view to its original
model handle. The compiler supplies an explicit live receiver conversion and, when the
source has a constructor, an explicit conversion of that constructor's owned result.
The source constructor runs exactly once; the conversion extracts its same model. Missing
constructor conversions fail rather than replacing construction, and absent source
constructors stay absent. Custom type codecs need explicit target metadata and are rejected.

Properties retain their names/order/value types, converters and individual fallibility.
Identity and declared reference casts use the same receiver conversion. Callback expressions
are captured once at metadata construction; each invocation converts its current model.
No model/view is cached or copied and no class/cast/constructor name is inferred. Base-member
hiding/overrides must already be resolved before projection; this API does not claim full
native inheritance discovery or complete model catalogs.

Independent generated consumers use source-scanned class-view metadata over the original
fixture model. They match existing C# wire output/aliases, serialize distinct live models,
verify a constructor returns the same original instance exactly once, and retain the same
accessor error across operations. The original source reference adds two cases (15 to17)
for configured constructor identity/call count and repeated error identity; all15 old cases
and46 other output files remain unchanged. The Rust serializer still has no markup runtime
or reflection dependency. Actual SDK receiver/schema discovery, complete constructor/base
contracts, model/default/backend catalogs and sample persistence remain required.

## Typed accessor failures

JsonPropertyInfo provides try_new, try_readonly and try_writeonly, plus matching explicit-
converter variants. Their callbacks return the original SerializationResult. A getter error
prevents conversion and later properties; reading converts a value before invoking its
setter. A setter may commit changes before returning its original error, which is propagated
unchanged. Read-only input values skip the getter/converter and write-only values emit no
getter output. Existing infallible APIs delegate to the same implementation without catching
panics or replacing errors.

Native bindings retain individual getter_fallible/setter_fallible facts. A fallible declaration
requires explicit RustAccessorErrorMapping::Preserve for an existing DockSerializationError,
or Map with a resolved typed foreign-error adapter. No error conversion is guessed; ignored
members still win before validation. The emitter selects the matching try API and wraps only
an infallible partner with Ok. Original callback expressions are evaluated and captured once
when metadata is constructed; their state persists across operations as in the source.

The unchanged original serializer supplies seventeen ordering/partial-commit/error-identity
cases, including persistent getters, configured constructors and repeated failures. All46 previous source outputs remain byte-identical.
Independent generated consumers compile all six try forms, mixed callback signatures and
explicit foreign-error mapping; readonly and stateful probes execute against real fixture
models. The source metadata callback protocol is adapted without linking a markup runtime
into the serializer or generated consumer. Existing wire goldens remain unchanged.

This removes the fallible-property binding gap. Full native actual-model receiver/base casts,
complete inheritance/constructor contracts, all conditional attributes, default/application/
backend catalogs and sample persistence remain required. CLR exception types/wrapping and
all parser/buffering/lifecycle failure orders remain outside these recorded cases.

## Native enum headers in the generated consumer

The payload compiler host additionally reconstructs DockMode, PinnedDockDisplayMode,
DockFloatingWindowHostMode, DockWindowState and DockWindowOwnerMode from the actual original
Rust model declarations. These are explicit SDK dependency roots, not an automatic full
model catalog. Their schema headers no longer come from the C# reference. The shared source
reader supplies canonical paths, kind, visibility and locations; the five transparent values
also retain their checked signed storage. Their configured `from_raw` factory is verified
against the scanner's actual public static function signature. Original named constants and
value operations remain in the actual compiled model; the original semantic generator does
not inspect enum fields or methods.

The importer constructs twelve native entries in one transaction: six SDK interfaces,
ICommand and these five enums. Explicit resolved source IDs unify incoming references;
no discarded enum header or property row is imported. Original System.Enum inheritance and
its complete interface closure remain an explicit foreign dependency. Nullable pinned-display
arguments and actual dock properties remap to the fresh enum IDs. A test corrupts all twelve
discarded headers/property rows and still obtains identical native symbols and generation
plans. A further test compares all five enum kinds, names, inherited interfaces, base types,
source provenance and property/nullable links with the unchanged source reference.

Macro-generated flags, other ancillary types, native system symbols, automatic dependency
root discovery and default/application/backend catalogs remain incomplete. This change is
the generated consumer's native compiler path; it does not add runtime reflection or another
model/engine, nor claim complete inherited enum APIs or unknown-value support for closed
Rust enums.

## Native required Dock contract schemas

`RustSourceDeclarations::metadata_definition` resolves a metadata declaration's single
`dyn Trait` carrier to its actual declared Rust trait. It does not unwrap receiver, shared,
nullable or arbitrary trait-object bounds, nor infer source properties from method names.
The ordinary semantic binder validates that exact definition and treats interface `base:`
metadata as a required interface edge in the resolved closure. It never records a class base
for an interface; missing or inconsistent closures still fail before any commit.

The payload compiler host now rebuilds the six required Dock interfaces from the existing
Core SDK declarations: IDockable, IDock, IRootDock, IDockWindow, IDocumentTemplate and
IToolTemplate. All 81 original property names, order, resolved types, flags and interface
edges match the independent C# reference. Command ignores and original selected payload
plans match too. Root metadata now places EnableAdaptiveGlobalDockTargets after ExitWindows,
matching original declaration order in both scanned and compiled runtime metadata.
The shared scanner canonicalizes both explicit binding keys and property/interface uses.
The native SDK assembly and source locations identify these headers; declared receivers
still refer to the original domain handles, without constructing replacement models.

Five regressions cover the full contract comparison, independence from corrupted old
headers/property rows, honest missing-system-attribute diagnostics, precise dyn-trait
provenance and interface/class inheritance separation. Existing generated consumer wire
and original factory-after-load tests still pass. The adapter and consumer gain no normal
runtime/UI dependencies. The compiler host now allocates fresh dependency slots and constructs ICommand from its actual
locked native source, as described below. Ancillary value/list/enum and system symbols still
come from explicit reference facts. Native interface/property schemas alone do not complete
native dependencies or the full default/application catalog. FerroUI.System currently has no IgnoreDataMemberAttribute
declaration; the host leaves a missing symbol absent and preserves DSTJ004 rather than
fabricating a descriptor. Full system metadata and discovery remain required.

## Atomic dependency import and native command identity

`import_semantic_dependencies` imports only selected foreign roots and their structural closure,
then allocates native IDs in actual declaration order. Explicit external symbol identities unify
references to those declarations; their old headers, members and slots are never copied.
Foreign generic/array/tuple/containing/base/interface/property/override/assembly links are
remapped. Required symbols remain optional, preserving missing-symbol diagnostics. Imports start
without activation, registration, local discovery or occupied-name state; context root expressions
remain explicit compiler input. Overrides into excluded declarations need complete native member
rebinding and return an error rather than retaining old property schemas.

The host binds native declarations in a private working graph. Foreign list types can refer forward
to reserved native IDs while no placeholder headers exist. A result is returned only after all
native headers are constructed and the final table size and every symbol link validate. Source
input is immutable; malformed dependencies, binding errors and late invalid links publish no graph.
This is structural import validation, not a complete source/cfg/macro/type-constraint compiler.

The payload consumer now imports ancillary facts, builds six Core SDK interfaces and the actual
ReactiveUI ICommand in seven fresh slots, then builds its nine application schemas/registrations.
All old application and seven replaced dependency schemas are discarded. Foreign IList type
arguments and command property types refer to the new IDs. ICommand's method/event-only interface
header comes from the real published source; no command object, duplicate trait or runtime metadata
stub is introduced. Its logical wire namespace remains explicit, with actual source location and
native package assembly/version. The native SDK property callbacks still retain original models.

The test compiler host locates one crates.io ReactiveUI package by the actual Cargo.lock version
and normalized cached package name/version, reads its complete module graph, and canonicalizes the
public ICommand reexport through the same shared ModelSet as declaration reading. It supports the
current single published registry source; vendored/path/multiple-version source discovery still
needs explicit compiler integration. It reads the cache without editing it or linking UI/runtime
code into the adapter. No package version, feature or normal dependency changes are required.

Seven additional regressions cover dependency graph identity/remapping, alias validation, absence
of placeholder headers, immutable/failed transactions, malformed and final links, excluded override
rebinding, native slot order, actual command provenance and real list/command links. Existing
native-contract poison tests now corrupt all seven excluded headers, including names/kinds and
assemblies; no corrupted schema reaches a returned table. Full system/ancillary symbol construction,
exhaustive export/cfg/macro/incremental discovery, codecs/catalogs and sample persistence remain open.

## Rust activation and registration declarations

The runtime library now exports `DockJsonSourceGenerationAttribute` and
`DockJsonSerializableAttribute`, retaining distinct shared instances and the supplied
logical type identity, including null. The Type property uses native `std::any::TypeId`;
complete shared TypeInfo/Attribute inheritance and runtime assembly metadata remain open.
Seven original C# constructor/type/identity assertions have matching Rust tests.

Applications can declare the source-generation request at crate scope:

```rust,ignore
ferrodock_serializer_system_text_json::dock_json_source_generation!();
ferrodock_serializer_system_text_json::dock_json_serializable!(MyDocument);
ferrodock_serializer_system_text_json::dock_json_serializable!(MyPayload<i32>);
```

These macros retain typed attribute constructor references without constructing registered
types or creating a runtime registry. Activation is unique per crate scope; registrations
may repeat. `parse_generation_attributes` in the build-time adapter reads the actual source
file or literal included fragment using its shared scan's module/import resolution. It
recognizes qualified paths and renamed imports, preserves order/duplicates and source
locations, and rejects nested markers, duplicate activation, malformed arguments and
unselected cfg. Test-only cfg(test)/all(test,...) items follow the shared production scanner.
The source's imports must belong to the scanned module; arbitrary macro bodies are not
expanded and dependency aliases beyond the shared scanner's resolution are not inferred.

`apply_generation_attributes` replaces the compilation's attribute inputs only after all
logical types resolve to valid compiler table entries. Original semantic validation and
registration deduplication still run afterward. Missing target symbol facts are adapter
errors; they are not reported as invented invalid source registrations. The compiled payload
consumer discards the C# fixture's activation/registration list and reads all nine requests
from the actual Rust `json_registration.rs` compiled into its crate. Its semantic class facts
and logical type-to-symbol mapping are still explicit. Full automatic Rust compiler symbol
construction, incremental integration, cfg/macro expansion and complete catalogs remain
required before ordinary application save/load can use this path alone.

## Required remaining work

Port default model-catalog constructors; complete original IList creation modifier and
generic creator semantics; static/default and generated model catalogs; complete registration
attribute/syntax adaptation and exhaustive automatic executable bindings; remaining interface
casts and polymorphic dock/window/template families; automatic runtime class/cast discovery;
remaining primitive and enum
codecs; original ignored interface/command members; automatic custom/closed generic payload
registration and complete converter behavior. Duplicate discriminator metadata is rejected
for polymorphic interface reads; the generated payload helper deliberately keeps multiple
string tags as raw JSON instead.
Complete metadata validation, CLR/UTF16/exception/lifetime behavior and performance also
remain required. Decimal/128-bit/character/date-time/identifier codecs and full enum numeric
domains, including unnamed underlying values, remain outside the current scalar bridge.

Validate actual original layouts through the existing engine after load: visible, hidden
and pinned tools, floating windows, active/default documents, shared lists and owner cycles.
Extend the limited projected layouts to all original fields and model families without
a layout-only snapshot, copied objects or runtime reflection. Other Newtonsoft, YAML,
XML and Protobuf serializers and alternate model backends remain part of the full goal.

Original tests are under tests/Dock.Serializer.UnitTests, source-generation tests under
tests/Dock.Serializer.SystemTextJson.SourceGenTests, and generator tests under
tests/Dock.Serializer.SystemTextJson.Generators.UnitTests in the parent repository.

## Property binding for native inherited views

`bind_declared_properties` accepts class and interface metadata with only borrowed
`RustPropertyMetadataOptions`. It binds the declaration's own instance properties without
asking abstract layers for constructors, reference identity or a decision about their base
properties. Existing class binding uses the same property path. Source property order,
JSON rename/ignore attributes, explicit converters and independent Result contracts remain.
Registered properties and type converters still require a complete typed binding; no partial
property table silently substitutes for a class codec.

`project_property_bindings` adapts only those callbacks through an explicit typed receiver
conversion, preserving missing accessors, fallibility, converters and object-payload flags.
Callback and conversion expressions are captured once; each invocation derives its live
receiver. Constructors and identity are deliberately outside this operation. The semantic
compiler must still resolve source inheritance, hiding and overriding before selecting the
complete serialized property table.

`metadata_instance_type` preserves the declared handle shape. When a view's instance handle
is the declaring type itself, the declaration's explicit public export is used for emitted
annotations. This fixes downstream compilation through private implementation modules.
Other handle shapes still need full compiler export resolution. Absolute value types have
space-separated parameter annotations in both native accessors and mixed Result adapters.

The independent markup consumer now scans the actual ReactiveUI SDK. Its test compiler walks
RootDock -> DockBase -> DockableBase -> ReactiveBase -> ReactiveObject using declared base
edges, checks the real registered casts, and finds each public base-view callable by owner
and normalized return type rather than assuming its name. It binds all declared properties
and selects fourteen scalar properties in original serializer order for a compiled callback
probe over the original `Rc<RootDock>`.

This probe reads the existing original C# root golden, retains null-override omission,
round-trips its selected scalar property order/values, preserves unnamed enum values, and
checks distinct live objects plus repeated setters on an explicitly configured same original
model. Core value metadata is explicitly supplied. It does not provide a default catalog,
replace the full semantic generator, or complete sample persistence. Existing complete
source-model fixture consumers and generated factory contexts remain separate checks of the
production emitter. All new dependencies are confined to the integration-test package and
reuse existing workspace package versions; production manifests and feature sets are unchanged.

## Actual SDK constructors and emitted four-model context

The downstream SDK consumer now feeds production `emit_rust` with native class bindings for
Document, DocumentDock, RootDock and DockWindow. Each uses its real metadata-declared
parameterless constructor, its explicit existing public model-view API, and its source-declared
concrete handle plus registered original-model-to-view cast. The compiler checks those facts
rather than deriving a concrete type from a class-name convention. Explicit model accessor
selection is checked against public function signatures and declared/inherited interfaces;
Document returns its IDockable base, while DocumentDock also exposes a distinct factory
interface, so an arbitrary unique-interface rule is insufficient.

`project_type_binding` captures each original constructor, invokes it once and extracts its
same concrete model through the original interface's boxed reference. Downcasts here validate
the configured original constructor contract; no input JSON chooses or guesses a model type.
Identity is the original enclosing Rc pointer obtained through that same checked accessor.
All 92 selected property bindings (Document22, DocumentDock26, RootDock31, DockWindow13) now
come from actual SDK declarations and inherited receiver projections. No selected-property
fallback remains. The production emitter supplies the normal resolver/factory overloads and
existing polymorphic reference scopes.

DockableBase SDK metadata now includes AllowedDockOperations, AllowedDropOperations,
ShowInSelector and SelectorTitle. Its 37 declared property names/order exactly match the
unchanged original compiled schema. The new typed Result accessors query the same original
model's optional restrictions/selector contracts; incompatible views return typed metadata
errors. Signed unnamed mask bits, selector edits and nullable title clearing are covered by
SDK tests. DockWindow's existing concrete handle registration is written with the explicit
qualified MarkupTyped form required by the shared scanner; runtime identity is unchanged.

The context retains the original four-model projection and runs the existing full graph
assertions against its unchanged wire fixture: original classes, cycles, aliases, lists,
window/root backreferences, shared policies, numeric modes, masks, bounds and actual factory
InitLayout. Distinct constructor-created instances, live edits and the configured list
factory overload also pass. The prior UI-independent generated consumer shares its unchanged
fixture configuration through test support; its emitted context remains byte-identical.

This does not complete native semantic discovery, source attributes/hiding/override rules,
all original model properties, dependency/system/ancillary schemas, native family fallbacks,
default/application/backend catalogs or sample persistence. The semantic input, selected
projection and family fallback bindings still use explicit original reference/typed facts.
The SDK consumer is an integration proof of the actual native constructor/accessor pipeline,
not an implicit catalog installed in production serializers. Production manifests, feature
sets, dependency versions and framework packages remain unchanged. The new serde_json edge
is confined to the integration-test build script and reuses the existing workspace version.

## Native declared order and IgnoreDataMember annotations

DockableBase, DockBase, RootDock, DocumentDock, Document and DockWindow native declarations
now retain the exact compiled original property order. Their 101 declared properties include
24 original IgnoreDataMember annotations, represented directly in the shared metadata DSL.
The downstream compiler compares every ordered property name and ignore flag against the
unchanged original schema before binding actual constructors/accessors. The annotations do
not remove properties from markup or rewrite their original callbacks; the semantic generator
uses them to compute its exclusion table, as required by the original policy.

This closes missing native annotation/order facts, including the command/factory/content and
transient owner/lifecycle properties. The six classes still require complete native logical
header/type/dependency resolution and full source attribute coverage. ReactiveObject's external
schema, other classes/backends, inheritance/hiding/overrides, family/default/application catalogs
and sample persistence remain open. The current four-model projection and its 92 accessor
bindings retain their original wire order and all previous engine graph checks. Its semantic
compilation input still includes explicit original reference facts; declaration conformance
alone does not replace that compiler stage or establish full serialization parity.


## Native SDK class schemas and accepted delivery priority

The downstream SDK consumer now uses seven newly allocated native class schemas, including
ReactiveBase and the six classes above. Source metadata supplies all101 property types,
order and ignore flags; actual Rust definitions supply provenance and native base edges
supply dependency ordering. Logical names, abstraction and direct source interfaces remain
explicit host configuration. Complete ordered interface closures match the original compiler.
The production dependency importer and semantic binder construct the schemas without reading
or copying old class headers/members. Family/typed accessor symbols remap to the fresh IDs.
Actual transitive dependency exports canonicalize foreign public/native carrier paths.

Three regressions compare all original schema facts, replace every old class header/member
with invalid data while requiring identical compilation/output, and mutate native property
facts to prove they affect the result. Existing original wire/graph/list/factory checks run
against the rebuilt schemas. Final98284 passes all21 stages and1,932 Rust tests/55 suites.
No reference assertions, production dependencies/features or framework sources change.

The user has narrowed delivery to core docking, ordinary JSON serialization and working
Fluent/Browser themes: see [delivery scope](DELIVERY-SCOPE.md). A complete general compiler,
all alternative backends and additional serializer formats are no longer prerequisites.
Next priority is a usable core docking model catalog, including Tool/ToolDock and required
layout containers, and normal save/load followed by engine/control use of the restored graph.
The current production serializer still needs a supplied resolver, and the four-model fixture
is not that catalog. Foreign System/ReactiveObject/ancillary schemas remain explicit inputs;
replace/extend those only where needed for the accepted persistence path.


## Standard ReactiveUI core catalog

Add the local `ferrodock-serializer-system-text-json-reactiveui` package. Its public
`create_serializer()` returns the existing `Rc<DockSerializer>` configured with the standard
ReactiveUI model catalog and observable lists. `create_serializer_with_list_type` accepts
an explicit `JsonListType`; `model_type_metadata` exposes descriptors for deliberate
application-level resolver composition. Serialize/Deserialize/Save/Load retain the existing
`IDockSerializer` API, declared generic type, null/error distinction and stream behavior.

```rust
use ferrodock_model::{controls::IRootDock, core::IDockSerializer};
use std::rc::Rc;

let serializer = ferrodock_serializer_system_text_json_reactiveui::create_serializer()?;
let json = serializer.serialize(layout.clone())?; // layout: Rc<dyn IRootDock>
let restored = serializer.deserialize::<Rc<dyn IRootDock>>(Some(&json))?;
if let Some(restored) = restored {
    factory.init_layout(restored.clone());
    // Bind restored to DockControl.Layout through the application's view model.
}
```

The catalog covers DockDock, Document, DocumentDock, GridDock, GridDockSplitter,
ProportionalDock, ProportionalDockSplitter, RootDock, SplitViewDock, StackDock, Tool,
ToolDock, UniformGridDock, WrapDock and DockWindow. Metadata invokes original constructors,
getters and setters; reference conversions retain their actual Rc identities. Core values,
policies, nullable fields, shared lists, numeric flags and window enums use the existing
runtime codecs. Four original core polymorphic families register the standard concrete
classes. Transient context/factory/host/commands and ignored lifecycle fields are omitted;
original property order and wire discriminators are retained.

The unchanged C# default serializer produces15 default-model fixtures, a complete graph
and configured-container wire, with27 source assertions.46 earlier outputs stay identical.
Seven runtime tests and a compiled usage example cover wire/readback, factory initialization
and AddDocument, owner/window/selection aliases and cycles, MDI/grid/selector values,
configured containers, observable/plain lists and null slots, and stream save/load with
original UTF-8 BOM/close behavior. Both Fluent and Browser tests load fresh standard models,
bind the actual DockControl and use native tab pointer input and original pin/unpin.
Final72420 passes all22 stages:1,942 tests/57 suites, including markup198.

Supported scope remains explicit. The catalog does not discover application subclasses or
persist arbitrary application content automatically. Such types require concrete descriptors
and corresponding polymorphic registrations; adding a concrete descriptor alone does not
extend an existing family table. The current sample's custom classes/actions still need that
integration. Ten closed Rust layout enums accept their declared variants only, returning typed
errors for unsupported numeric values. Complete template/custom/unknown-interface fallback
coverage and exhaustive original serializer semantics remain partial. The accepted delivery
scope remains open until the runnable application workflow and final docking/visual checks
are complete.


## Explicit application model registration and sample services

The production backend adapter now exposes `DockJsonModelRegistration` with typed dock,
document, tool, document-dock and root constructors. Each accepts a logical wire name,
an actual model constructor and typed own-property descriptors. Inherited class fields and
reference casts are supplied by the role helper. `model_type_metadata_with_models` composes
these registrations with the standard catalog; `create_serializer_with_models` returns the
existing serializer with observable lists. Empty/duplicate names and duplicate/standard model
types fail before any model constructor runs. Callers still resolve own ignored/hidden/override
members and additional value codecs explicitly; automatic discovery remains deferred.

The sample uses the helpers for its12 original classes and original C# discriminator names,
independently of renamed markup namespaces. No core classes replace application subclasses.
Its application composition root injects `ILayoutSerialization` and `ILayoutInitialization`
into MainWindowViewModel. Public save_layout_json/load_layout_json use the original graph.
Invalid JSON returns before closing/replacing it; successful load reconnects original factory
fields/live locators and fresh contexts before publishing the new layout. JSON null clears
layout locators/status. The legacy constructor still creates the original sample without
persistence services; callers use with_layout_services for those additional operations.

The primary Documents locator is not derived from a required model Id: the source sample's
document dock Id is empty. The initializer selects the first document dock in source traversal
order and reconnects the original private field. Nonstandard applications with multiple primary
dock conventions should supply their own initializer policy through the abstraction.

The reference links12 unchanged C# sample classes and passes18 checks, producing12 default
wires and one class/alias/command graph.64 previous outputs stay identical. Rust backend9 tests
verify actual own properties/identity/constructor timing/registration guards and all old cases;
sample34 tests include exact source wire replay, fresh restored classes/contexts, live Root/
Documents locator identity, malformed/null/unconfigured behavior and original CreateDocument.
Actual App Headless restore under Fluent and Browser realizes original Document/Tool views,
creates/activates Document4 and serializes again. Final57455 passes all 23 validation stages,
1,951 tests/57 suites. User-facing File menu/picker/storage commands and desktop checks remain
unfinished. Closed enum, template/unknown-interface and ownership/cycle limitations above still
apply; this is not exhaustive source serializer/compiler parity.


## Sample file commands and native storage

The sample now composes ILayoutFileStorage with a separate LayoutFileViewModel. Compiled
File menu bindings execute Open Layout and Save Layout As asynchronously; the status bar
reports results. Native selection resolves the current TopLevel from a weak parent, uses
JSON filters/default extension and overwrite confirmation, reads/writes platform storage
streams and disposes selected file handles. Domain/presentation code has no UI controls.

LayoutJsonSerialization.save_bytes/load_bytes use the existing DockSerializer Stream
Save/Load over MemoryStream, preserving its UTF-8 preamble and encoding detection. Main
layout replacement happens after successful decoding, then the original factory reconnects
contexts/locators. Cancelling a picker, failing to read or malformed JSON preserves the
current graph. JSON null remains a successful clear operation. Both file commands are
unavailable while either operation is pending. Save failures report their cause; platform
writes are ordinary truncating writes, without a claimed transactional file guarantee.

Tests cover native SSD byte IO/truncation/errors/options, weak and unavailable hosts,
command outcomes/gating/null/UTF-16, and real compiled menus/status restoring native views
under both themes. Workspace 1,959 tests/57 suites and all 23 checks pass. Actual native
picker presentation and the refreshed desktop interaction audit still require verification.
Existing serializer limits above remain unchanged.


The native file workflow now also has end-to-end routed MenuItem click tests with the
actual NativeLayoutFileStorage, a scoped controlled IStorageProviderFactory and real
BclStorageFile streams under both themes. They prove late nested-path binding, owning
TopLevel resolution, pending picker gating, source file options, byte JSON roundtrip,
actual custom models/views/locators and constructor commands, picker cancellation/errors,
native read/write and parse failure preservation, and UTF-16 null clearing. All temporary
files are owned fixture directories under TMPDIR (SSD in validation). Production code
and the review desktop bundle are unchanged. Latest workspace: 1,967 tests/57 suites,
all 23 stages pass. Platform picker panel presentation still requires desktop verification.


Composed native pointer tests now start from a standard JSON-restored graph under each
theme, move a tab between panes, float it through the original factory into an actual
HostWindow and redock its last tab across windows. The live floating graph roundtrips
with identical JSON; original identities/factory, selected native floating tab panel,
closed empty host and registry cleanup are asserted. These tests use the existing core
catalog and original engine; no new serialization format or production behavior was added.
Latest checkpoint: 1,967 tests/57 suites and all 23 validation stages pass. Selector splitting/
resizing, other combined tool flows and native desktop appearance remain acceptance work.


The scoped composed acceptance suite now includes rendered selector split, actual native
splitter resize and model proportion propagation, document/tool reorder, pin/preview/unpin,
explicit factory hide/restore, both tool close modes and native document close under both
themes after standard JSON reload. Exact split/pinned/hidden JSON replay is also checked.

The original ReactiveUI DockableBase declares OriginalOwner with IgnoreDataMember, and
DockModelPolymorphicTypeResolver.RemoveIgnoredMembers honors it. Accordingly, the standard
catalog omits this transient value; a deserialized hidden tool has no OriginalOwner. The
same-session hide/restore tests preserve the original live owner through the factory API;
they do not claim that ignored original-owner state survives JSON load. Application policy
must reconnect transient state where its workflow needs it, as with contexts/factories.
This preserves the upstream wire contract rather than adding a different persistent field.

Final acceptance: 1,967 tests/57 suites and all 23 stages pass. Native desktop dialog and
appearance acceptance remains pending Mac unlock; the goal is not complete.

## macOS desktop verification finding (2026-10-24)

The first actual Save As attempt exposed a double release in FerroUI's native security
scope input handling, before the JSON stream could be written. The isolated native
package now borrows caller-owned URI strings for scope, bookmark and file-reference
operations. Its ownership regression rejects the original backend; all 127 native
library tests pass with the fix. Native accessibility callbacks also reject released
controls, avoiding a separate shutdown after menu/layout replacement. Neither fix changes
the serialization engine or JSON wire format.

The clean rebuilt macOS sample now passes actual native Save/Open. The saved49,403-byte
UTF-8 BOM file uses the original ReactiveUI RootDock discriminator. Adding Document4,
reopening the saved file and then creating Document4 again verifies restored graph,
views, factory and commands; F11 confirms the resulting live graph. Save/Open cancellation
and an owned malformed JSON file preserve the current layout, selection and Browser theme.
The parse error is reported in the status bar. The artifact and its hash are recorded in
CONTINUATION.md and the native desktop proof. Earlier unlock/picker-pending entries above
are historical; the remaining acceptance question concerns physical Browser docking/flyouts.
The204-test markup suite also verifies restored flyout bindings and actual pointer opening
before and after pin/unpin under both themes. No serializer or production code changed in
this final desktop verification turn.
