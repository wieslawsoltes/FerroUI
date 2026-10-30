# Serializer stream and JSON reference

Run from this folder with the installed .NET10 SDK:

```sh
TMPDIR=/Volumes/SSD/temp/ferrodock-port DOTNET_CLI_HOME=/Volumes/SSD/temp/ferrodock-port/dotnet-home NUGET_PACKAGES=/Volumes/SSD/NuGetCaches/packages NUGET_HTTP_CACHE_PATH=/Volumes/SSD/temp/ferrodock-port/nuget-http dotnet run -p:UseArtifactsOutput=true -p:ArtifactsPath=/Volumes/SSD/temp/ferrodock-port/core-model-reference -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/ -- ../../../tests/Dock/data/serializer-stream-reference.tsv ../../../tests/Dock/data/serializer-json-audit.tsv
```

The project compiles four unchanged original SystemTextJson serializer files and both
registration/activation attribute files. It references original Dock.Model/ReactiveUI and
the original incremental generator as an analyzer. It does not edit original sources.
The stream corpus contains37 matched source assertions for memory streams, source
close/failure order, original fixed buffers and nullable generic serializer dispatch.
Rust model tests consume these outcomes. Native I/O adaptation also tests shared
identity/position, explicit disposal, custom nonclosing overrides and close reentry.

The JSON corpus now contains13 source outcomes with matching Rust provided-resolver
coverage. Twelve additional failure cases are saved in serializer-json-audit.failures.tsv.
Wire strings and a full payload golden preserve original escaping, property order and
reference IDs. These checks do not establish default model catalog or polymorphic parity.

The actual source includes readonly properties and throws on empty Load. Save emits an
Encoding.UTF8 preamble and closes the supplied stream. Load closes on success and failure.
Close errors override earlier body failures; invalid constructor streams are not closed.
The source preserves self-reference IDs and distinguishes null values from failures.

JsonModelReference.cs uses the original ReactiveUI models and factory. Its explicit
property projection records 44 model/list/reachability checks plus root and shared-list
wire goldens. It covers concrete/interface aliases, owner/root/window cycles, observable,
plain and custom list creators and original InitLayout after decoding. This is a limited
projection, not the default model catalog or complete sample layout. Proportions, constraints
and window bounds now cover NaN, infinity, finite numbers and signed-zero setter behavior.

CoreModelReference.cs records ten source assertions, 49 read/replay cases and six JSON
goldens over original DockRect, DockCapabilityPolicy and DockCapabilityOverrides. Cases
cover copied bounds, nullable flags, original constructors, ignored HasAnyOverride, shared
base/interface references, base-first derived-reference failure and reserved metadata.
The read corpus is ordinary JSON containing input, outcome and serialized result/error type.
The model projection also persists pinned bounds, six base capability flags and shared root/
dock policies, then checks loaded precedence and shared mutation through the original engine.
Rust direct and emitted-context tests consume these outputs. Full catalogs remain unfinished.

CoreValueReference.cs adds ten assertions, 490 input/replay cases and four goldens for
get-only DockPoint, shared DockManagerOptions and three signed flags types. It verifies the
source default point on read, constructor-default enabled options, alias mutation, undefined
signed bits and numeric range/token rules (including 128 seeded values per flags type).
A custom Int32 converter leaves enum conversion independent. Separate input and roundtrip
holder goldens preserve the original get-only point behavior. Real-model goldens now include
DockingState and concrete allowed operation masks; original mask rules remain authoritative.
This does not cover the remaining plain-enum unknown-value representation or full catalogs.

WindowEnumReference.cs records seven assertions, 282 numeric/name/wire rows, 50 read cases
and two goldens for window owner/state enums and original DockWindowOptions. Signed Int32
values include 128 seeded samples per enum and values without names. Shared options/parent
references and original ApplyTo behavior are checked. JsonModelReference now includes all
13 persisted DockWindow properties, unknown state/owner values and parent aliases. Default/
application catalogs and shared compiled numeric-literal emission remain unfinished.

JsonNumberReference.cs records 1,362 exact wire/read cases for Single/Double and all eight
signed/unsigned 8/16/32/64-bit integer types. It includes 512 seeded bit patterns for each
float type, format/exponent boundaries, named literals, canonical decoded bits, overflow,
underflow and invalid integer tokens/ranges. Rust tests compare the source output and also
check exact roundtrip bits for the finite float wire cases.

JsonFallbackReference.cs supplies source polymorphic metadata to the unchanged serializer.
Five depth cases record success through 64 objects and failure before the 65th getter.
Two further cases and a wire golden cover fallback self-reference and ignored unknown
discriminators. Matching Rust tests use supplied compiled base descriptors.

GeneratedPayloadReference.cs activates the actual original generator and exercises its
emitted resolver. Twenty assertions, 25 read/replay cases and eight goldens cover typed
payloads, independent inner/outer references, closed generics, explicit/read-only property
converters, duplicate/unknown tags, raw values/null and failure cases. Runtime and compiled consumer tests match the recorded helper behavior. The Rust semantic
pass, emitter, declared-member binding and basic crate-scope activation/registration discovery
now exist. Complete automatic compiler facts, runtime class/cast discovery, incremental host
and full catalogs remain unported.

Default constructors/full model metadata, automatic subtype discovery/nearest-ancestor
rules, full converter/JsonDocument ownership behavior and text buffering/encoding remain
to be ported. Source model
links retain objects; CLR cycle collection remains absent in Rust. Do not replace these
requirements with a layout-only snapshot, model cache or runtime reflection.

AttributeReference.cs compiles the unchanged original activation/registration attribute
classes. Seven assertions in serializer-json-audit.attributes.tsv cover distinct/shared
instances, exact closed type identity, null retention and no registered-type construction.
Rust attribute tests consume them. The Type property uses native TypeId; complete shared
TypeInfo/CLR Attribute and runtime assembly metadata exposure remain open. The Rust marker
macros and source parser enforce unique activation, repeated registrations and crate scope;
these checks do not establish complete compiler/AttributeUsage reflection parity.

DockEnumReference.cs records 423 value/name/numeric-wire rows (128 seeded values per type)
and 102 nullable/nonnullable read cases for DockMode, PinnedDockDisplayMode and
DockFloatingWindowHostMode. All 44 previous source output files are byte-identical in the
new run. The reference uses unchanged original models and serializer code.

The model projection now includes Dock, nullable PinnedDockDisplayModeOverride and root
PinnedDockDisplayMode/FloatingWindowHostMode. Six additional source checks (44 to50) verify
restored unnamed values and original factory initialization. Root/list goldens gain only
these scalar properties; all prior property values/order/reference IDs are retained and
43 other source outputs remain identical. Compiled source wire order, including root pinned
mode before Window/Windows, is checked in both direct and emitted Rust contexts.

AccessorFailureReference.cs configures actual original serializer property delegates and
records15 cases for error identity, getter/converter/setter order, partial setter commits,
read-only skipping, write-only omission and persistent getter state across serializations.
All46 earlier source output files remain byte-identical. The probe exercises original C#
metadata callbacks; the Rust implementation uses typed Results and no runtime reflection.

The accessor reference now has17 cases. Added cases preserve a configured constructor's
original instance/call count and repeated getter-error identity. All15 earlier accessor
cases and46 other source outputs remain unchanged. These also supply the semantics checked
by independently compiled native receiver-projection consumers.


CoreLayoutCatalogReference.cs exercises the unchanged default model resolver without a
property projection. It records27 assertions and15 standard class/default wire fixtures,
a full document/tool/pinned/hidden/floating-window graph and configured layout containers.
The new `*.layout.*` source outputs are captured as `tests/Dock/data/serializer-json-catalog.*`.
All46 older source outputs remain byte-identical. The production ReactiveUI JSON adapter
consumes these fixtures through its runtime tests; Fluent/Browser downstream tests also
reload its ordinary JSON and exercise native tab selection and original pin/unpin.


SampleLayoutReference.cs links the12 unchanged original persisted sample model classes and
uses the source default resolver.18 checks cover12 class/default wires and a custom graph with
original type tags, aliases and constructor-created CreateDocument behavior. The `*.sample-layout.*`
outputs are captured as `tests/Dock/data/serializer-json-sample.*`.64 prior audit/catalog output files
stay byte-identical. The project uses the original sample's ReactiveUI preprocessor branch for
CustomDocumentDock; original source files remain read-only.
