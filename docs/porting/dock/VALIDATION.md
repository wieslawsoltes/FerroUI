# Native desktop stability checkpoint (2026-10-24)

Both isolated native dependency fixes pass **127 native library tests**. The storage
ownership regression fails against the original Objective-C++ code and passes with the
borrowed-input fix. The released-control case verifies callback rejection without keeping
a removed control alive. Full workspace tests pass **1,967 tests/57 suites**, zero failed
or ignored. Strict workspace/all-target Clippy, formatting,24Python tests and source/theme/
inspector/seven-package provenance guards pass. The clean desktop sample builds and is
packaged as `/Volumes/SSD/temp/ferrodock-port/FerroDock Core Review.app`.

Proof: `/Volumes/SSD/temp/ferrodock-port/ferrodock-native-desktop-checkpoint.json`.
The earlier 23-stage proof below remains historical; release-library and all standalone
checks were not repeated for this native-only revision. No Dock engine/model/app behavior
or original C# source was changed. Temporary menu tracing has been removed from source.

The clean rebuilt sample now passes actual native Save/Open: a49,403-byte UTF-8 BOM
layout was saved, a fourth document added, and reopening restored the saved three-document
layout and actual views. CreateDocument and F11 work after reload. Save/Open cancellation
and malformed JSON preserve the current graph; parse errors appear in the status bar.
The user confirmed physical document reorder and tool floating/docking. Browser light/dark,
pin/preview/unpin, document close, F9, pane resize and full-width grip were checked on the
clean build. All204 markup integration tests pass after adding compiled menu-binding retention and
actual pointer opening after JSON restoration and after pin/unpin under both themes.
Targeted strict all-target Clippy also passes. Physical Browser docking/flyout
confirmation remains pending. See CONTINUATION.md.

## Previous checkpoint

# Scoped core acceptance checkpoint

Exact driver81513 is confirmed terminal0. **1,967 Rust tests in 57 suites** pass, zero
failed/ignored; markup 204, sample 44, native UI 373 and diagnostics 49. All 23 stages pass:
strict workspace/all-target and 16 release-library Clippy, ten standalone suites, eleven
normal boundaries, format, 24 Python tests, source/theme/inspector/six-vendor guards and
both generators. Full source completion remains correctly rejected. Final proof/logs use
SSD prefix ferrodock-core-acceptance-exact-.

Six composed Headless cases cover both themes after JSON reload: native document/tool
reorder, rendered Right selector split, actual proportional splitter geometry/model
updates, cross-pane move/HostWindow float/cross-window redock/empty-host cleanup, native
pin/preview/unpin, factory hide/restore, tool close modes and native document close.
Split/floating/pinned/hidden JSON replay is exact. Source IgnoreDataMember behavior on
OriginalOwner is checked; its transient value is not claimed to survive reload.

Production code/dependencies are unchanged. Initial final13377 passed before a literal
key assertion was tightened; exact81513 repeats the full validation over final frozen
inputs. All 168 source hashes, 64 prior output wires and previous emitted context match.
Original parent source, empty staged index, protected old executable and distinct review
bundle/source executable are verified. No desktop rebuild was needed.

Current-state completion audit: automated evidence covers the scoped core flows and
source sample integration. Native refreshed application/dialog presentation, both-theme
appearance, F11 inspector and header grip rendering remain unverified. CUA still reports
Mac locked, recurring across more than three goal turns; the existing unlock request is
pending. The goal cannot be marked complete on automated evidence alone.

## Previous checkpoint

# Composed pointer docking validation checkpoint

Driver70568 is confirmed terminal0: all 23 stages pass, **1,963 Rust tests in 57 suites**,
zero failed/ignored. Markup 200, sample 44. Strict workspace/all-target and 16 release-library
Clippy, ten standalone suites, eleven normal boundaries, format, 24 Python tests, all source/
theme/inspector/six-vendor guards and both generators pass. Full-repository completion
remains correctly rejected, independently of scoped delivery. SSD prefix:
ferrodock-composed-drag-final-. Proof records 168 frozen inputs and 64 prior source outputs,
unchanged generated context, original parent sources, empty staged index and both preserved
desktop artifacts.

Two added tests drive native pointer events through actual DockControlState/DockManager
under Fluent and Browser after standard JSON restoration. They cover cross-pane document
move, actual HostWindow floating with the selected native tab panel, exact floating-graph
JSON replay, cross-window redocking of its last tab, model/owner/factory/selection identities,
drag reset, closed empty host and original DockControls registry 1->2->1. Host lookup and
styles use original APIs/classes. Production code/dependency inputs did not change.

The first fixture's zero-width third pane was corrected with explicit proportions. Early
private projection and dictionary-import test compilation errors are retained in logs.
No production workaround or weakened old assertion. The final tests retain the full float/
redock path. They do not establish selector-driven split, splitter resize or all tool flows;
those remain next, alongside desktop appearance/panels while the Mac is locked.

## Previous checkpoint

# Native picker-to-file validation checkpoint

Driver64612 is confirmed terminal0. All 23 stages pass: **1,961 Rust tests in 57 suites**,
zero failed/ignored; sample 44. Strict workspace/all-target and 16 release-library Clippy,
ten standalone suites, eleven normal dependency boundaries, format, 24 Python tests,
source/theme/inspector/six-vendor guards and both generators pass. SSD log/proof prefix:
ferrodock-native-files-final-. All 167 frozen input hashes and 64 previous source outputs
match; original generated context, original parent source and empty staged index match.

Two added tests route compiled MenuItem click events through the actual native storage
adapter, a scoped controlled platform picker provider and real temporary files under
TMPDIR on SSD. Both themes verify supported-provider options, late compiled binding,
nested window resolution, delayed native picker gating, BOM JSON, restored actual views/
locators/CreateDocument and cancellation/picker/file/parse failures plus UTF-16 null.
The original sample has contexts only for Document1..3; new Document4 retains null context
and the actual factory after the source command. An incorrect test expectation was
corrected against unchanged original source; no production behavior or old test weakened.
Initial generic trait-object lookup compilation errors were corrected to public locator
callbacks. All intermediate logs are retained.

Production code/dependencies are unchanged. Both old protected and distinct SSD review
executables/bundle hashes are unchanged and verified. Native desktop panels/appearance
remain unverified because CUA still reports Mac locked. Core composed drag/drop/float/
redock acceptance remains open; narrow helper or persistence tests do not close it.

## Previous checkpoint

# File workflow validation checkpoint

The workspace passes **1,959 Rust tests across 57 suites**, zero failed/ignored;
the sample has 42 tests. All 23 stages are green. Strict workspace/all-target and 16
release-library Clippy, ten standalone suites, eleven normal dependency boundaries,
format, 24 Python tests, source/theme/inspector/six-vendor guards and both generators pass.
Full-repository completion remains correctly rejected, independently of delivery scope.

Runtime checks initially passed before inventory audit31549 rejected five unmapped new
support files. Added their explicit source/infrastructure mappings and updated stale App
F11 notes in docs/porting/dock/ports.json without changing any Rust input or coverage status. Finisher
48756 terminal0 reran the remaining structural checks. Intermediate failure logs remain.
Final stage results and proof use SSD prefix ferrodock-file-workflow-final-.

The new tests exercise standard byte serialization, original custom sample restoration,
file cancellations/read/parse/write failures, pending command gating, null/encoding behavior,
platform JSON options, actual SSD storage truncation/roundtrip and original IO errors,
weak/missing/unavailable hosts and compiled menu/status behavior under both themes.
A submenu lookup failure was corrected to use the actual logical tree before final tests.

All 166 frozen source hashes, 64 earlier source outputs and previous emitted context match.
The original parent source, staged index and protected desktop executable are unchanged.
The separate desktop review build55508 passes (dev debug0, incremental false, -j1),
and its distinct SSD review bundle matches the built executable hash. It has not been
launched: CUA still reports the Mac locked. Native picker/visual acceptance remains pending;
passing Headless/stream tests does not establish those platform results.

## Previous checkpoint

# Current validation: application model JSON and full sample restore

Final57455 is confirmed terminal0. All23 sequential stages pass: **1,951 Rust tests across57
suites**, zero failed/ignored; strict workspace/all-target and16 selected release-library
Clippy; ten standalone suites; eleven normal boundaries; format;24 Python tests; source/theme/
inspector checks; six framework provenance guards; both generators; correct rejection of broad
source completion. Scoped delivery is still incomplete.

Backend9 tests include own application properties, original instance/reference identity,
constructor deferral/count and registration validation, retaining every standard wire/graph
case. Sample34 includes12 source default model wires, original custom graph/tag/alias/constructor
command replay, full restored contexts/live locator checks, malformed/null/unconfigured behavior
and one real App Headless test for restored views/CreateDocument under both Fluent/Browser.
The linked12 original C# classes pass18 source checks.64 previous output files are unchanged.
Initial test setup missed ReactiveObject; a test-only existing dependency supplies it. The first
full-sample test incorrectly assumed Documents was an Id; source inspection shows an empty Id,
so restoration identifies the primary document dock by its contract/source traversal order.
A needless-borrow test lint is corrected without allowances. Prior assertions remain intact.

SSD prefix ferrodock-app-json-final-. All81 pre-validation input hashes match afterward;
64 prior source outputs and previous UI-independent emitted context remain byte-identical.
Protected desktop size/mtime/hash, original parent source and staged index match. All owned
Cargo/reference handles terminal. All scratch/artifacts on SSD; only docs follow green.
Sample adds existing local JSON package edges; adapter adds test-only ReactiveUI; all framework
versions/features/vendor code remain unchanged. Inventory227/465/1236 unchanged.

Actual file-menu/picker/storage/cancellation/error behavior and refreshed desktop appearance/
docking acceptance remain unverified and required next. Programmatic JSON restore is not a
claim that the user-facing file workflow is finished. See DELIVERY-SCOPE.md.

## Previous checkpoint

# Current validation: sample live Fluent/Browser switching

Final21542 is confirmed terminal0. All23 sequential stages pass: **1,944 Rust tests across57
suites**, zero failed/ignored; strict workspace/all-target and16 selected release-library
Clippy; ten standalone suites including sample --lib; eleven normal dependency boundaries;
format;24 Python tests; source/theme/inspector checks; six framework provenance guards; both
generators; correct rejection of broad source completion. Scoped delivery remains incomplete.

Sample29 includes two new real App Headless cases. They validate compiled selector/command
wiring, actual Browser delayed panel versus Fluent, repeated switching, original layout Rc
identities, native pointer document selection, presets/light-dark, shared application service,
no duplicate overlays and initial choice after real view initialization. The existing pure
ReactiveUI command test checks valid/invalid tab indices. Initial24874:28/29 passed; the shared
service test inspected an uninitialized view. Attaching/showing the window before assertions
matches the actual behavior lifecycle; focused36739 then passes. No source assertions are
removed and no production behavior is weakened.

SSD prefix ferrodock-sample-browser-final-. All56 pre-validation input hashes match after
validation.46 prior source outputs and the prior UI-independent emitted context remain byte-
identical. Protected desktop size/mtime/hash, original parent source and staged index match.
Only the sample's local Browser dependency/Cargo.lock edge is added; existing framework
versions/features/vendor code are unchanged. All owned jobs/handles terminal; all scratch on
SSD; only docs follow green. The running desktop has not been rebuilt/refreshed.

Next acceptance evidence must cover actual sample save/load and application subtype/context/
command preservation, then refreshed desktop/themed docking checks. The broad file inventory
227/465/1236 is not the scoped delivery percentage or completion gate.

## Previous checkpoint

# Current validation: standard ReactiveUI JSON catalog

Final72420 is confirmed terminal0. All22 stages pass: **1,942 Rust tests across57 suites**,
zero failed/ignored; strict workspace/all-target and14 selected release-library Clippy;
nine standalone suites; eleven normal dependency boundaries; format;24 Python tests;
source/theme/inspector checks; six framework provenance guards; both generators; correct
rejection of the broad full-source completion audit. That audit is separate from the user's
[functional delivery scope](DELIVERY-SCOPE.md), which remains incomplete.

The new adapter has7 runtime tests plus a compiled usage doctest. All15 original default
model wires and complete graph/configured-container fixtures match the unchanged C# default
serializer.27 source assertions pass;46 previous outputs remain byte-identical. Coverage
includes readback wire order, original classes/constructors/accessors, shared owner/selection/
window identities and cycles, MDI/grid/selector fields, policies/nullable dependencies,
original factory initialization/AddDocument, configured observable/plain lists and null
entries, stream BOM/close, and invalid/unsupported-value errors. Two new Headless tests
reload standard JSON under Fluent and Browser and verify original factory binding, native
pointer tab selection and pin/unpin; markup198. Focused strict Clippy20384 passes.

SSD logs/summary/proof prefix: ferrodock-catalog-final-. All45 pre-validation source hashes
match; prior UI-independent emitted context and46 earlier source outputs are unchanged.
The protected desktop size/mtime/hash, original parent sources and staged index match.
All owned Cargo/reference handles are terminal. All artifacts/scratch on SSD. Source changes
freeze at green; only docs follow. Intermediate test setup failures are retained and corrected
without changing original engine behavior or weakening assertions. Existing dependency
versions/features and six framework sources are unchanged; the new backend adapter and its
workspace/test wiring add only local dependency edges.

This does not prove sample persistence or full delivery completion. Existing application
subclasses need explicit compiled descriptors/registrations; the sample has no save/load or
Browser selection yet. Unnamed values of ten closed Rust layout enums, template/unknown
interface cases and exhaustive serializer behavior remain partial. Complete the scoped
application workflow and final cross-feature/desktop checks next.

## Previous checkpoint

# Current validation: native SDK class schema construction

Final98284 is confirmed terminal0. All21 sequential stages pass: **1,932 Rust tests across55
suites**, zero failed/ignored, strict workspace/all-target and selected release-library Clippy,
eight standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/
inspector checks, six framework provenance checks and both generators. The broad source
completion audit is correctly rejected. The user subsequently narrowed functional delivery;
[DELIVERY-SCOPE.md](DELIVERY-SCOPE.md) now defines acceptance instead of full file coverage.
This green baseline does not prove ordinary sample persistence or all delivery scenarios.

Adapter70 includes three new actual-SDK class-schema tests: all original seven header/base/
ordered-interface/property/type/ignore facts; identical fresh compilation after poisoning
all seven old class headers/members; and sensitivity to native property type/order/ignore
changes. Markup196 retains original four-model wire/graph/factory/list checks and constructor/
identity behavior while its emitter consumes the rebuilt schemas. Initial focused failures
(missing public BoxedValue mapping; test mutation's incorrect PropertyModel field name) are
retained in SSD logs and corrected without weakening checks. Strict focused Clippy passes.

SSD logs/summary/manifest prefix: ferrodock-native-class-final-. All14 pre-validation source
hashes match afterward; prior UI-independent emitted context remains byte-identical. Protected
desktop executable size/mtime/hash, original parent sources and staged index are unchanged.
No production dependency/version/feature/framework/reference changes. All owned handles are
terminal; all artifacts/scratch on SSD. Only documentation follows green. Inventory remains
227/463/1238 of1928. Ordinary core JSON catalog/persistence and final Fluent/Browser functional
checks remain required by the accepted delivery scope.

## Previous checkpoint (historical broader scope)

# Current validation: native declared property order/ignore attributes

Focused89762 passes SDK8/markup196/consumer9. The actual SDK compiler audits101 ordered
property names and ignore flags over six classes, retaining24 original IgnoreDataMember
annotations. Existing92 native selected accessor bindings, source wire/engine graph assertions,
constructor identity, list factories and prior UI-independent emission remain intact.

Final85300 is confirmed terminal0: all21 stages pass, **1,929 tests across55 suites**, zero
failed/ignored, strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six provenance checks and both generators. Full completion is correctly rejected.
SSD prefix ferrodock-native-attributes-final-. Selected pre-validation source hashes match
afterward; only docs follow green. All owned Cargo/reference handles terminal. Protected
actual desktop and original parent/sibling/cache sources are unchanged; all scratch on SSD.

No reference data, existing assertion, dependency/version/feature or framework changes. Native
class semantic/type/dependency header construction, remaining attributes, full catalogs and
sample persistence remain incomplete. Inventory227/463/1238 of1928 stays unchanged; the entire
original goal stays active/incomplete/unblocked.

## Previous validated checkpoint

# Current validation: actual SDK constructor/model contexts

Focused39536 passes SDK8, markup196 and emitted consumer9. Actual SDK constructor and inherited
accessor bindings supply all92 selected properties of the original four-model projection.
Runtime checks repeat the existing full engine graph assertions against unchanged wire fixtures,
including alias/cycle/policy/bounds/numeric values and real factory InitLayout, and verify fresh
original constructor instances plus live edits. The original list factory overload remains.
An SDK metadata test validates four added base properties, unknown signed masks and nullable
selector values. Native/source DockableBase property order matches all37 declarations.

The shared fixture extraction leaves the prior UI-independent emitted context byte-identical.
Source-declared concrete handles/casts and actual public function/interface signatures are
required. Initial20009 passes all1929 tests but Clippy rejects duplicate fixture module loading;
the shared module now has one owner. All intermediate failure logs are retained; source behavior,
reference data and earlier assertions are unchanged.

Final85675 is confirmed terminal0. All21 sequential stages pass: **1,929 tests across55 suites**,
zero failed/ignored, strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six framework provenance checks and both generators. Full completion is correctly
rejected. SSD prefix ferrodock-native-model-checked-. Selected source hashes before validation
match afterward; only docs follow green. Original parent/sibling/cache and protected actual
desktop artifacts are unchanged. All owned Cargo/reference handles terminal; all scratch on SSD.

Only integration-test build wiring adds the existing workspace serde_json dependency. Production
manifests, dependency versions/features and six framework packages are unchanged. Inventory
227/463/1238 of1928 stays unchanged. Complete native semantic schemas/inheritance/attributes,
all constructors/catalogs/family fallbacks and sample persistence remain open. The full goal
remains active/incomplete/unblocked.

## Previous validated checkpoint

# Current validation: actual SDK inherited scalar callbacks

Focused67656 passes adapter67, generated consumer9 and markup193. Four added adapter tests
cover property-only class/interface binding, rejected incomplete facts, public direct-instance
exports and property receiver projection without constructor/identity. Two SDK consumer tests
compile actual five-level inheritance callbacks and match the existing original scalar wire
projection, enum values, null omission, live model distinction and repeated original-instance
setters. Existing goldens and assertions stay intact. Intermediate failure logs are retained;
private path/token spacing errors are fixed, and probe setup explicitly supplies Core metadata.

Final73699 is confirmed terminal0. All21 sequential stages pass: **1,925 Rust tests across55
suites**, zero failed/ignored, strict workspace/all-target and selected release-library Clippy,
eight standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/
inspector checks, six provenance checks and both generators. Full completion remains correctly
rejected. SSD prefix ferrodock-native-base-final-. Source hashes are recorded before validation
and verified after it; only documentation follows green. No owned Cargo/reference handle is
left running. Protected actual desktop artifact hashes and original parent/sibling/cache
sources are unchanged. All scratch/artifacts remain on SSD.

Test-only dependencies reuse existing workspace packages without production manifest/version/
feature changes. Inventory227/463/1238 of1928 stays unchanged. Actual complete SDK constructor/
identity/inheritance/hiding resolution, all native schemas/catalogs and sample persistence
remain open. The entire original goal remains active/incomplete/unblocked.

## Previous validated checkpoint

# Current validation: original model receiver projection

Original source30015 exits0 with17 configured-callback cases; all15 earlier cases and46
other outputs remain identical. Focused15654 passes adapter63/consumer9/runtime36. Independent
compiled consumers match existing original source wire/aliases, retain distinct live models,
extract the exact constructor-created model once and preserve original errors across calls.
Unit checks preserve callback contracts/casts/input data and reject missing conversions,
custom type codecs and unresolved/nonconcrete types. An initial generated annotation spacing
failure (49913) is corrected; no runtime/reflection fallback or weakened behavior is added.

Final49691 is confirmed terminal0. All21 sequential stages pass:1,919 tests across55 suites,
zero failed/ignored, strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six provenance checks and both generators. Full completion is correctly rejected.
SSD prefix ferrodock-receiver-final-. Selected source hashes are recorded before this driver
and verified afterward; only docs follow green. All owned Cargo/source-reference handles are
terminal. Full native inherited/model schemas, default/backend catalogs and sample persistence
remain open; inventory227/463/1238 of1928 and the entire goal remain incomplete.

Original parent/sibling/cache sources, dependencies/versions/features/framework packages and
protected actual desktop artifacts are preserved. All scratch/artifacts stay on SSD.

## Previous validated checkpoint

# Current validation: typed native accessor failures

Original source42531 exits0 with15 callback cases; first14 probe rows and all46 previous
source output files remain byte-identical. The runtime matches every trace, partial commit,
skip/omit result, original error identity and output wire. Independent consumers compile
all six try API forms, mixed accessor signatures and explicit foreign-error mapping.
Readonly and persistent-callback probes execute with actual fixture models. Full original
model/wire/post-load tests remain. Error callbacks return Results rather than unwrap/panic.

Final59967 is confirmed terminal0. All21 sequential stages pass:1,914 Rust tests across55
suites, zero failed/ignored; strict workspace/all-target and selected release-library Clippy,
eight standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/
inspector checks, six provenance checks and both generators. Full completion remains
correctly rejected. Logs/summary prefix: ferrodock-fallible-accessor-checked-. Source hashes
are recorded before that driver and verified after it; only documentation follows green.

Initial47563 had1913 passing tests but failed generated-code Clippy; callback expressions
now capture once and the added stateful regression passes (80986).16523 had1914 passing
tests but failed broader Clippy on test-module placement; moved to file end. The new readonly
probe's original reserved-metadata assumption was corrected without relaxing reader behavior.
Failure/intermediate logs are retained. All owned Cargo/source-reference handles terminal.

All scratch/artifacts on SSD. Original parent/sibling/cache and protected actual desktop
are unchanged. No dependency/version/feature/framework modifications. Inventory227/463/1238
of1928; full native schemas/catalogs/sample persistence and the full goal remain incomplete.

## Previous validated checkpoint

# Current validation: docking and pinned/root layout properties

Original source82557 exits0 with50 restored-model checks (44 previous plus6 new). All old
checks remain. Root/list goldens gain only Dock, pinned override and root pinned/host modes;
prior property values/order/reference IDs remain unchanged when those additions are removed.
All43 other source outputs compare byte-for-byte. Focused53414 exits0: serializer35 and
emitted consumer4. Both contexts match exact original wire output and preserve actual model
values through the original Factory.InitLayout, retaining all earlier alias/list/capability
checks. These are supplied four-model contexts; full catalogs/sample persistence stay open.

Final driver83466 is confirmed terminal0. All21 sequential stages pass:1,909 Rust tests across
55 suites, zero failed/ignored; strict workspace/all-target and selected release-library
Clippy, eight standalone suites, ten dependency boundaries, format,24 Python tests,
source/theme/inspector checks, six provenance checks and both generators. Full completion
is correctly rejected. SSD logs/summary use ferrodock-layout-mode-final-. All owned handles
are terminal; only documentation follows green. Source hashes preserve exact validated code.

No dependency/version/feature/framework edits, original parent/sibling/cache source changes
or desktop executable/app/window changes. All scratch/artifacts on SSD. Inventory227/463/1238
of1928; the full original goal stays active/incomplete/unblocked.

## Previous validated checkpoint

# Current validation: unnamed docking-mode values

Final focused57047 exits0: native UI373, markup191, model235, JSON35, native adapter60 and
settings34. Original serializer source62077 exits0:423 seeded value/name/wire rows and102 read
cases, all44 prior source output files byte-identical. Compiled source35148 exits0:914 checks,
retaining all908 previous checks. Domain/storage/display, explicit numeric/nullable JSON,
compiled real models/settings, host precedence and source converter fallback are covered.

Full sequential driver86671 is confirmed terminal0: all21 stages pass. The workspace passes
1,909 tests across55 suites, none failed/ignored. Strict workspace/all-target and selected
release-library Clippy, eight standalone suites, ten dependency boundaries, format,24 Python
tests, source/theme/inspector checks, six provenance checks and both generators pass. Full
completion remains correctly rejected. Logs/summary use SSD prefix ferrodock-dock-enum-final-.
All owned Cargo/source-reference handles are terminal. Only documentation follows full green;
source/test behavior remains exactly as verified. Initial failure logs remain preserved;
the source converter's wildcard/default is restored and missing test import corrected.

All scratch on SSD, -j1/one Cargo job. No original parent/sibling/cache source or dependency/
version/feature/framework edits. Actual desktop executable/app/window protected. Inventory
227/463/1238 of1928; structural audit passes and full completion remains unproven/incomplete.

## Previous validated checkpoint

# Current validation: native enum catalog entries

Final owned driver56061 is confirmed terminal0. All 21 sequential stages pass, with **1,900
Rust tests across 55 suites**, zero failed/ignored. Strict workspace all-target and selected
release-library Clippy, eight standalone suites, ten normal dependency boundaries, format,
24 Python tests, source/theme/inspector checks, six framework provenance checks and both
generators pass. Full completion remains correctly rejected. SSD logs/summary use
ferrodock-native-enum-catalog-final-; all owned Cargo handles are terminal.

Targeted adapter23021 passes60 tests and consumer50969 passes4. The compiler constructs
five model enum headers from actual source alongside the seven existing native contracts.
Tests compare original enum kind/name/inheritance/interface closure, source locations and
property/nullable links. Corrupting all twelve discarded schemas still gives identical
native symbols and generation plans. Initial29445 fails only the obsolete seven-slot range;
the updated assertion verifies seven interfaces followed by five actual enum slots.
Original C# reference data/wire goldens and all original-engine consumer assertions remain
unchanged; no source model/runtime behavior is weakened to obtain these results.

Only docs/porting/dock/inventory change after full green. The inventory corrects three closed enums to
partial pending unnamed numeric representation and inherited APIs:227/463/1238 of1928.
Structural audit passes after that correction. Entire goal remains active/incomplete.
No original source or protected actual desktop artifact is changed. All scratch on SSD.

## Previous validated checkpoint

# Current validation: transparent numeric enum provenance

The targeted native adapter suite passes 59 tests, including three new source-backed tests:
transparent storage facts, atomic unsupported-representation rejection and actual window enum
declarations. Final sequential driver71621 is confirmed terminal0: all 21 stages pass.
The workspace passes 1,899 tests across 55 suites, none failed or ignored. Strict workspace
all-target and selected release-library Clippy, eight standalone suites, ten normal dependency
boundaries, format, 24 Python tests, source/theme/inspector checks, six framework provenance
checks and both generators pass. Full completion remains correctly rejected. Logs/summary
use SSD prefix ferrodock-transparent-enum-final-. Only documentation follows full green;
all owned Cargo handles are terminal. Source/test behavior remains exactly as validated.

The previous numeric-enum driver24490 is confirmed terminal0. All 21 stages pass: 1,896
workspace tests with no failures/ignored tests, strict workspace all-target and selected
release-library Clippy, eight standalone package suites, ten normal dependency boundaries,
format, 24 Python tests, source/theme/inspector checks, six framework provenance checks and
both generators. The completion gate correctly rejects the unfinished full port.

Original compiled C# reference passes 908 checks, including all 900 existing checks, seven
numeric-enum checks and boxed-scalar namescope identity. Final source run15969 exits0;
ferrodock-numeric-enum-verified-reference.log and numeric-enum-verified-reference.tsv retain
evidence. Full isolated framework run27451 exits0: Base4381/Build25/Scan46/Loader402 passed,
with one pre-existing ignored Loader diagnostic. Test-only missing published font/theme
assets are copied from read-only sibling sources to owned SSD scratch and hash-recorded.
All four changed full patches replay byte-for-byte against exact published archives.

Actual desktop executable size/mtime/SHA256 match the protected baseline. Parent/sibling/cache
sources remain unchanged. Inventory230/460/1238 of1928; full goal remains active/incomplete.

## Previous validated checkpoint

# Current validation: open window enums and complete window JSON projection

Window source references21107/40183/79251 terminal0 pass. Final79251 supplies7 assertions,
282 value/name/wire rows,50 read/replay cases and two options/shared-parent goldens; model
assertions expand to44. Runtime/generated contexts retain original windows, source casts,
parent aliases, all13 persisted window properties and original ApplyTo behavior.
Domain storage/name/number tests and native runtime/nullable conversion match source values.

69359 initial Model227/CoreSDK62 passes;77930 expanded Model229/CoreSDK63/consumer4/JSON33
passes. All24 Python tests pass. Initial full98588 reveals an old settings expectation that
all unnamed enums fail. Source-correct settings assertions now require raw signed roundtrips
for the two window types and reject out-of-range values;42789 passes33.37663 started while
42789 still awaited terminal confirmation; Cargo's build-directory lock serialized them.
42789 then confirmed terminal0; no duplicate validation was restarted on an observation timeout.

FINAL37663 terminal0 passes **1,894 Rust tests/55suites**,zero failed/ignored; all21 stages
pass in owned SSD ferrodock-window-enum-final-validation-summary.json. Strict workspace/
debug/all-target and selected release-library Clippy,8 standalone packages,10 normal
boundaries,format,24Python/source/theme/inspector/two generators/six provenance guards pass.
Matching driver/per-stage logs share that prefix; initial failed full-run logs are preserved
under ferrodock-window-enum-initial-. Only docs and the bridge's explanatory comment change
after full green; behavior/test code remains as verified.

The verifier now checks every pure enum source declaration even with multiple mapped targets;
strict transparent storage/member checks and a metadata-target bypass regression pass.
Window owner/state are correctly partial pending compiled undefined numeric emission, full
inherited enum APIs and automatic compiler provenance. Inventory230/460/1238 of1928; the full
completion gate still rejects unfinished work. No dependencies/versions/features/framework
patches, original parent/sibling/cache source or actual desktop are changed. Full goal active.

## Previous validated checkpoint

# Current validation: get-only values, shared options and signed flags

Reference 21111 terminal 0 passes original value/flags behavior; source 63423 reveals initialized
documents have Document state (4), not Docked (1). Corrected exact source assertion passes
69869. Final source 12567 terminal 0 adds custom Int32 converter independence and passes
all ten value assertions, 490 read/replay rows, four goldens and 41 model assertions. Retained
Core/model corpora compare byte-for-byte with that final unchanged-original run.

Rust 93023 passes JSON29. Expanded generated consumer 41699 exposes a test-host lexical
spacing error for absolute type annotations; fixed by separating colon and path.3014 passes
consumer4/JSON29;64253 final consumer4/JSON30 passes with integer-codec independence.
No source golden or original engine assertion is weakened. The get-only point input and
roundtrip goldens intentionally differ because the source skips incoming component values.

FINAL18971 terminal 0 passes **1,886 Rust tests / 55 suites**, zero failed/ignored; all 21
stages pass in owned SSD ferrodock-core-value-final-validation-summary.json. Matching driver
and per-stage logs share that prefix. Strict workspace/all-target/debug and selected release-
library Clippy, eight standalone packages, ten normal dependency boundaries, format, 21
Python tests, source/theme/inspector checks, two generators and six framework provenance
guards pass. The full-completion gate still correctly rejects remaining partial/pending work.
No production/test edits follow full green. Inventory 232/458/1238 of1928 unchanged.

Default/application catalogs, plain-enum undefined values, sample persistence, remaining
compiler/serializer/backend contracts and broad API/platform/lifetime/performance parity
remain open. No new dependency/version/feature/framework changes. Original parent/sibling/
cache sources and the actual desktop are preserved. Entire original goal remains active.

## Previous validated checkpoint

# Current validation: Core JSON metadata and original capability behavior

Reference session 18873 terminal 0 produces 10 Core assertions, 49 read/replay rows and six
wire goldens; the expanded original-model probe passes 38 assertions. The reference uses
unchanged original models, serializer/options/resolver and incremental generator. Only its
explicit test projection adds capability fields and pinned bounds. Runtime and emitted-model
consumers preserve source wire order, reference aliases, value copies, initialization and
loaded capability precedence/mutation. New tests reject reserved metadata and verify that
explicit contexts do not silently gain a model catalog.

Initial expanded Rust run exposes the six source base capability flags in the new reference;
both runtime and emitted projections now retain those real fields in original declaration
order. Session 34599 passes consumer 4 / JSON 27 / compiler 4. Session 49860 strict changed-
package all-target Clippy passes. FINAL 37200 terminal 0 passes **1,883 Rust tests / 55 suites**,
zero failed/ignored; all 21 stages pass in owned SSD
ferrodock-core-model-final-validation-summary.json. No production/test edits follow full green.
The full driver is ferrodock-core-model-final-validate.py with the matching driver.log and
per-stage logs. Strict workspace/all-target/debug and selected release-library Clippy,
eight standalone packages, ten normal dependency boundaries, format, 21 Python tests,
source/theme/inspector, two generators and six framework provenance guards all pass.

Inventory remains 232/458/1238 of 1928; full completion is correctly rejected. Complete
catalogs, sample persistence and the original broad compiler/serializer/backend/platform/
lifetime/API scope remain open. Original parent/sibling/cache sources and actual desktop
are preserved; no dependencies, versions/features or framework patches change.

## Previous validated checkpoint

# Current validation: atomic native dependency import and command identity

68893 passes consumer4/adapter49 for new import allocation.50148 passes4/49 after actual
published-command registry lookup and ModelSet canonicalization.21554 and80022 pass4/56
with seven new import/command tests.99509 strict changed-package/all-target Clippy passes
after removing needless closure borrows;20008 final4/56 passes with activated-plan poisoning.
All81 original ordered Dock property facts remain matched under remapped logical identities;
actual native command source/assembly, list arguments and command property IDs are verified.
Full selected generic/array/tuple/nested/property identity, errors/rollback, alias validation,
excluded overrides and final links are covered. Original wire/factory/engine assertions unchanged.

FINAL18632 terminal0 passes **1879 Rust tests/55suites**,zero failed/ignored; all21 stages pass
in owned SSD ferrodock-native-import-final-validation-summary.json. Strict workspace/debug/
all-target and selected release-library Clippy;8 standalone packages;10 normal boundaries;
format/21Python/source/theme/inspector/two generators/six provenance guards and expected full-
completion rejection pass. Adapter56/consumer4/compiler4/JSON23/CoreSDK62 and existing engine/
UI/sample suites pass. No production/test edit follows full green. Inventory232/458/1238
of1928 unchanged; no source unit promoted complete. Remaining system/ancillary facts still explicit C# test
inputs; this is not complete native dependencies, general Cargo/source/cfg/macro/incremental
compilation, default catalogs or sample persistence. Full goal active; actual desktop protected.

## Previous validated checkpoint

# Current validation: native required Dock interfaces

28735 passes CoreSDK62/consumer4/adapter49.92501 strict changed-package/all-target Clippy
passes.81417 repeats62/4/49 after adding the compiled runtime root declaration-order assertion.
Five new adapter regressions compare all81 original ordered property facts, interface closures,
command ignores and selected payload plans; poison old schemas; preserve DSTJ004 for the
missing system attribute; constrain dyn provenance; and reject incorrect interface/class
inheritance atomically. Consumer golden/factory-after-load tests remain unchanged.

Initial61004 identifies canonical key/use spelling differences, corrected through the shared
scanner;99884 passes4/44.88960 identifies only shared self-import wiring; parent alias fixes
it without adding runtime/build dependencies.

FINAL21504 terminal0 passes **1872 Rust tests/55suites**,zero failed/ignored, and all21 stages
in owned SSD ferrodock-native-contracts-final-validation-summary.json. Strict workspace/debug/
all-target and selected release-library Clippy;8 standalone packages;10 normal boundaries;
format/21Python/source/theme/inspector/two generators/six provenance guards and expected full-
completion rejection pass. CoreSDK62/consumer4/adapter49/compiler4/JSON23 and all existing
engine/UI/sample suites pass. No production/test edit follows final green. Inventory
232/458/1238 of1928 unchanged, no source unit promoted complete. Remaining
dependency IDs/ancillary/system/command facts are explicit C# test inputs, not complete native
dependency compilation/catalogs or sample persistence. Full goal active; actual desktop protected.

## Previous validated checkpoint

# Current validation: native batch allocation and dependency isolation

Targeted10338 passes consumer4/adapter42. Added poisoned-discarded-source independence and
actual ordered override tests;68044 passes consumer4/adapter44. Strict changed-package/all-
target Clippy91265 passes. Eight meaningful batch regressions cover forward/mutual properties,
late rollback, duplicate identities, out-of-range references, forward generic arguments and
late closure failure, required inheritance order, original override rows and exclusion of
corrupted old application facts. Existing source golden/engine assertions are unchanged.
Initial1051 identifies reference metadata's literal location marker;26836 identifies generic
template unresolved parameters in definition lookup. Both fixed through actual reference
facts/validated template construction, without invented schema or weakened source assertions.

FINAL15708 terminal0 passes **1867 Rust tests/55suites**,zero failed/ignored, and all21 stages
in owned SSD ferrodock-native-batch-final-validation-summary.json. Workspace/debug/all-target
and selected release-library strict Clippy;8 standalone packages;10 normal dependency
boundaries;format/21Python/source/theme/inspector/two generators/six provenance guards and
expected full-completion rejection pass. Adapter44/consumer4/compiler4/JSON23 and all original
engine/native UI/sample suites pass. No production/test edit follows full green. Inventory
232/458/1238 of1928 unchanged; no source unit promoted complete. Dependency facts still
come from an explicit C# test reference; exhaustive native discovery/dependencies/catalogs
and sample save/load remain incomplete. Entire goal stays active. Desktop remains protected.

## Previous validated checkpoint

# Current validation: generic template specialization and closed semantic symbols

FINAL71032 terminal0 passes **1859 Rust tests/55suites**,zero failed/ignored. All21 stages pass
in owned SSD ferrodock-generics-final-validation-summary.json: workspace/debug/all-target and
selected release Clippy;8 standalone packages;10 normal boundaries;format/21Python/source/theme/
inspector/two generators/six retained source guards and expected full-completion rejection.
Adapter36,consumer4,compiler4,JSON23 and existing original engine/native UI/sample suites pass.
No production/test edit follows final green.4970 changed-package strict Clippy also passes.

Six additional generic tests cover actual template/handle/member specialization without
mutation, nested types/type-valued attributes and unchanged literals, typed callable and
parameterized export paths, associated/inferred/const/generic/cfg errors, recursive logical
argument closure/atomicity and original generic full-name output. Consumer now compiles both
closed examples using the same actual Generic constructors/accessors as ordinary metadata;
manual per-type branches removed. Initial47381 exposes only missing closure annotation; fixed.
99400,65977 and final14056 pass. Existing original C# source corpora remain unchanged.

All9 payload schemas now use native symbols/bindings; snapshot dependency/type-slot/discovery
seeds remain. Tests do not establish complete constraints, nested/captured generic mapping,
all projections/aliases/visibility/cfg/macros, production catalogs or sample save/load. Full
original engine/API/lifetime/backend/platform/accessibility requirements remain. Inventory
232/458/1238 of1928 unchanged, goal active. Actual desktop protected; final preservation and
source/staging/format checks follow below.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-generics-desktop-preserved.json; final-checks.json records lightweight
evidence. All owned jobs terminal; entire goal remains active and incomplete.

## Previous validated checkpoint

# Current validation: native declared semantic type/property construction

FINAL75564 terminal0 passes **1853 Rust tests/55suites**,zero failed/ignored and all21 stages
in owned SSD ferrodock-semantic-symbols-identity-final-validation-summary.json: workspace/debug/
all-target/selected release Clippy,8 standalone packages,10 normal boundaries,fmt/21Python/
source/theme/inspector/two generators/six retained-source guards and expected full-completion
rejection. Adapter30,consumer4,compiler4,JSON23 and existing engine/native UI/sample suites pass.
No production/test edit follows final green. First full89540 passed1852/all21 before the added
explicit serialization-identity contract/test; final full covers that change.

Seven ordinary classes' native schemas replace C# header/property rows in the compiled consumer.
Native aliases explicitly preserve source wire names; general API keeps logical identity
separate from markup names. Dependency/type-slot seeds and two closed generic examples still
use snapshots. Nine semantic tests cover fresh/self headers, source ignore/command/static
names, override/hiding, atomic errors, abstraction, unsupported shapes/kinds, cycles and separate
identity. Initial51381/29459 pass; Clippy26558 detects only duplicate test module loading, fixed
with a single shared compiler fact reader.3003 and45489 pass29/30 adapter tests respectively.

These checks do not prove a fully automatic/native compilation, production catalogs, whole-
graph incremental rebinding, all generic/access/override/cast/constructor/value contracts or
sample save/load. All previously required source/lifetime/platform/backend scope remains.
Original source corpora unchanged. Inventory232/458/1238 of1928 unchanged; full goal active.
Actual native desktop protected; final source/staging/format/preservation checks follow.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-semantic-symbols-desktop-preserved.json; final-checks.json records
lightweight evidence. All owned jobs terminal; entire goal remains active and incomplete.

## Previous validated checkpoint

# Current validation: actual Rust declarations and shared export identity

Full43887 terminal0 passes **1844 Rust tests/55suites**,zero failed/ignored. All21 stages pass
in owned SSD ferrodock-rust-declarations-final-validation-summary.json: strict workspace debug/
all-target/selected release Clippy;8 standalone packages;10 normal boundaries;fmt/21Python/
source/theme/inspector/two generators/six retained source integrity guards and expected
full-completion rejection. Adapter21,consumer4,pure compiler4,JSON23 and existing engine/native
UI/markup/diagnostics/sample suites pass. No production/test edit follows full green.

Seven new declaration-reader tests cover actual kinds/visibility/locations, generic lifetime/
type/const bounds/defaults, trait impls/supertraits, cfg/path/inline/raw modules, ambiguity,
missing graphs and source/dependency reexports. Actual original Core trait declarations and
compiled original application fixtures are read; payload contexts compile with native Rust
visibility/arity/locations. Alias mismatch from initial47243/focus45019 is fixed through
published shared ModelSet identity, preserving the original graph assertion. Initial70208
and canonical81046 pass; final11464 passes consumer4/adapter21. Strict97333 also passed before
the final test-only cfg unification. Conditional selection/abstractness/macro expansion and
full logical class/property/identity construction remain unproven, not silently inferred.

C# snapshots still supply some logical facts; full production symbol construction/catalogs/
sample save/load remain incomplete. Original source corpora are unchanged. Inventory232/
458/1238 of1928 remains unchanged, and the entire goal stays active. Native desktop not
rebuilt/restarted; final size/mtime/SHA256 and source/staging/format checks follow below.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-rust-declarations-desktop-preserved.json; final-checks.json records
lightweight evidence. All owned jobs terminal; entire goal remains active and incomplete.

## Previous validated checkpoint

# Current validation: native source-generation markers and Rust registration discovery

Full44357 terminal0 passes **1837 Rust tests/55suites**,zero failed/ignored. All21 stages pass
in /Volumes/SSD/temp/ferrodock-port/ferrodock-json-registration-final-validation-summary.json:
strict workspace/debug/all-target and selected release-library Clippy;8 standalone targets;
10 normal dependency boundaries;format/21Python/source/theme/inspector/two generators/six
retained package integrity guards and expected completion rejection. Adapter14,compiled
consumer4,semantic compiler4,JSON23 and duplicate activation compile-fail doctest1 pass.

Actual Rust source registration fragment compiles into the payload consumer and is parsed
by the build adapter. Its9 requests replace the C# fixture's activation/registration input
before the original semantic pass; source wire/identity/engine assertions still pass.
Tests cover resolved aliases/qualified paths, duplicate registration order, source locations,
assembly scope, cfg rejection, malformed/unresolved types, missing-fact atomicity and source
missing-activation DSTJ002/deduplication. Full semantic Rust class/dependency discovery, all
macro/cfg modes and source compiler diagnostic parity are still unproven.

Source20536 terminal0 records7 unchanged original attribute constructor/type assertions,
consumed by Rust tests. Activation/registration objects retain distinct/shared references,
closed logical types and null, without constructing registered types. Full shared TypeInfo/
CLR Attribute/runtime assembly metadata/AttributeUsage surface remains open. Existing source
serializer/model/number/payload/policy corpora also pass. First targeted36290 finds included
inner-doc syntax; fixed.65310 exposes only incorrect expected source column1:43 vs actual1:42;
fixed and expected source diagnostic ID read/corrected.50197 and strict91712 pass thereafter.

No semantic/production/test change follows full green. All original source/dependency integrity
checks pass; actual native desktop was not rebuilt/restarted. Inventory232 complete/458partial/
1238pending of1928, reflecting2 newly partial attribute units. Full goal remains active.
Complete compiler inputs/catalogs/sample persistence and owning Rc cycle collection remain open.
Final source/staging/format and desktop size/mtime/SHA256 checks follow this checkpoint.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-json-registration-desktop-preserved.json; final-checks.json records all
lightweight results. All owned jobs terminal; full goal stays active and incomplete.

## Previous validated checkpoint

# Current validation: shared declaration-to-Rust JSON bindings

Final7639 terminal0 passes **1828 Rust tests across55 suites**, zero failed/ignored. All21
stages pass in /Volumes/SSD/temp/ferrodock-port/ferrodock-shared-bindings-final-validation-summary.json:
strict debug workspace/all-target and selected release-library Clippy;8 standalone packages
(adapter8,consumer4,semantic compiler4,JSON21,CoreSDK62,recyclingSDK2,ReactiveSDK7,runtime3);
10 normal dependency boundaries;format,21 Python tests,source/theme/inspector/two generators,
six retained-package integrity guards and expected full-completion rejection. Full workspace
includes original engine/native UI/markup/diagnostic/sample scenarios. No production/test source
change follows green validation. Targeted41370 and Clippy54510 also pass before final adapter test8.

The compiled payload consumer now binds7 classes from shared scanner declaration metadata and
executes unchanged source wire/identity/converter assertions. Eight adapter tests cover property
ordering/renames/read-only/set-only/ignores, converter precedence, path/typed function scope,
explicit inherited fields, unresolved/cfg/fallible/unsupported attributes and constructors.
These prove the recorded declared-member binding contract, not automatic semantic discovery,
all native typed-function/configuration combinations or full default model catalogs. Identity,
object-type and converter-factory facts remain explicit. Two closed generics and actual model
field projection stay explicit; sample save/load and Rc cycle collection remain unfinished.

Original source corpora are unchanged. Original parent/sibling sources and all6 retained package
sources are preserved. Actual standalone desktop size/mtime/SHA256 match prior baseline, recorded
in ferrodock-shared-bindings-desktop-preserved.json. All temp work stays on SSD. Source inventory
remains232 complete/456 partial/1240 pending of1928; goal remains active. Final lightweight
source/format/inventory/staging checks are recorded after this checkpoint.

Final lightweight checks pass: workspace/include formatting, source inventory and factory
generator, expected full-completion rejection, new-code naming/unsafe/stub scan, original
source diff empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256
remain unchanged. Evidence: owned SSD ferrodock-shared-bindings-final-checks.json.
All owned jobs terminal; full goal remains active and incomplete.

## Previous validated checkpoint

# Current validation: executable Rust metadata emission and runtime fallback policies

Targeted2897 terminal0 passes consumer4 +runtime21 +compiler3. Compiler emission activation/
missing-binding test then adds1 (compiler4). First41336 fails a fixture upcast triple-colon
syntax; corrected target expression. Clippy81870 finds fixture clone-on-Copy; corrected getter.
22847 strict all-target Clippy on all3new/changed packages passes. Generated code compiles and
executes actual ReactiveUI model/engine projection and original application payload fixtures.

Source23466/19261/98298 terminal0 regenerate source data:19 compiler fixtures/152 name checks,
20payload assertions/25read cases/8goldens,15policy writes/16reads/5graph checks/holder golden;
prior1362numeric/33model/7fallback/37stream/13JSON/12failure and900factory cases stay covered.
Empty polymorphic configuration fails on use as source; full CLR failure types/timing remain
unverified. Policy dispatch supports exact/base/nearest/ambiguity with compiled class views,
abstract schemas and original reference identity. Full automatic runtime cast discovery remains open.

First full82278 fails on actual Skia archive ENOSPC before tests. After owned jobs terminal,
retired16 exact previously logged test harnesses=4593194272bytes with manifest under owned SSD;
actual desktop binary excluded. Other projects untouched. Retry exec53537/Python17653 terminal0 passes **1820 Rust tests/53suites**, zero failed/ignored.
All20stage driver checks pass, including strict workspace debug/all-target and selected
release Clippy,7 standalone packages,9 normal boundaries,format/21Python/source/theme/
inspector/two generator/six provenance guards and expected full-completion rejection.
Summary: /Volumes/SSD/temp/ferrodock-port/ferrodock-json-emission-final-validation-summary.json.
No stale run is restarted on an observation timeout. All temp/TMPDIR/NuGet caches remain SSD.

Schema bindings in generated consumer are EXPLICIT LIMITED projections, not default model
catalogs or an automatic syntax adapter. Full field/value/constructor/attribute/cancellation/
metadata/error/performance parity remains incomplete; Source owning Rc cycles still need collection.
Actual desktop size/mtime/SHA256 preservation has already passed once this turn; final checks
will repeat/checkpoint it after the driver. Inventory232/456/1240 of1928; goal remains active.

## Previous fully validated checkpoint

# Current validation: generator semantic compiler pass

Final40695 terminal0 passes **1812 Rust tests/51 suites**, zero failed/ignored; compiler3,
JSON18,ReactiveUI568 and existing engine/UI/sample suites. All19 values zero in owned SSD
ferrodock-json-generator-final-validation-summary.json: debug/all-target/release-library
Clippy, standalone3+18+62+2+7+3,8 normal dependency boundaries, format/21Python, source/theme/
inspector/two generators/six package provenance guards and expected full-completion rejection.
No code/test change follows the final green run. First full28702 also passes before extra
name-case records; final rerun includes those records. Compiler has no production dependencies.

Original generator reference17 compilations export compiler facts, expected emitted plans and
DSTJ001-004 messages/spans. Rust compares local discovery/accessibility/generic registration,
context closure/naming, inherited/overridden/hidden ignores and interface unions,7 polymorphism
plans and collision filtering. Runtime name comparisons cover8 shapes across17 snapshots=136
checks: nested generic/array/tuple and CoreLib/unsigned project assembly identities.56791 first
passes11 fixtures;77946 expands17;89067 final name probe passes. Initial1345 only fails missing
variant docs/porting/dock/unused import;82147,62121 and14807 pass3 tests; strict62616 passes. Malformed consumed
symbol links/cycles return errors. No source algorithm or assertion requirement was weakened.

This does not validate a Rust syntax adapter, incremental host, macros, source emission or
full catalog/sample persistence: these remain unimplemented. Exhaustive compiler errors,
locations, cancellation and performance remain unverified. Existing runtime serializer,
full enums/metadata/model coverage and CLR cycle collection limitations stay open. Prior
factory900, serializer1362numeric/33model/17payload+25read+7goldens/7fallback/37stream/13JSON/
12failure and controls4428 results remain applicable to unchanged source/dependency features.
Inventory232/456/1240 of1928; generator remains partial. Final desktop/source/staging checks
are recorded after this checkpoint. All temporary work uses owned SSD storage.

Final lightweight checks pass: format/includes, inventory, factory generator, expected incomplete
gate, source naming/unsafe/stub rules, compiler normal tree containing only itself, original
source diff empty and no staged contents after intent-to-add. Actual desktop size/mtime/SHA256
match the preserved baseline; ferrodock-json-generator-desktop-preserved.json and -final-checks.json
in owned SSD temp record that evidence. All owned jobs terminal; the full goal remains active.

## Previous fully validated checkpoint

# Current validation: source-generated payload helper runtime

Full95023 terminal0 passes **1809 Rust tests/49 suites**, zero failed/ignored; JSON18 and
ReactiveUI568. All18 stages pass in owned SSD ferrodock-json-payload-validation-summary.json,
including strict debug/all-target/release-library Clippy, standalone18+62+2+7+3, seven normal
boundaries, fmt/21Python, source/theme/inspector/two generators/six provenance guards and
expected full-completion rejection. No production/test change follows the green run.

The C# reference now executes the unchanged ORIGINAL incremental generator and its emitted
resolver, rather than reimplementing the helper in the probe. Final84796 terminal0 records
17 assertions,25 read/replay cases and7 goldens. Four Rust tests match payload class handles,
separate inner/outer scopes, self refs, closed generics, explicit/get-only converters, duplicates,
raw null, unknown tags, metadata and failure paths. Source1362numeric/33model/7fallback/
37stream/13JSON/12failure and factory900 remain covered. Prior4428 controls tests apply to
unchanged retained dependencies/features. No Cargo dependency change this turn.

First2954 passes2/fails1 on residual non-string$type accepted by class metadata. Source expanded
probes establish typed dollar-metadata failures, including a mapped$type property; raw values
must retain that same data. The actual class reader now rejects reserved unknown metadata;
89981 and final85153 pass all18 JSON tests. No assertion was weakened.

Generator frontend/attributes/discovery/diagnostics/emission/default catalogs are NOT implemented;
only emitted runtime helper behavior is newly partial. Inventory232/456/1240 of1928. JsonDocument
ownership/disposal, root object handling, exact constructor/HandleNull/depth/error/performance and
CLR cycle collection remain open. Final lightweight checks and actual desktop preservation
are recorded after this checkpoint. All temporary work stays on owned SSD storage.

Final lightweight checks pass: format/include formatting, source inventory, factory generator,
expected incomplete gate, source naming/unsafe/stub rules, unchanged original source files and
empty staging after intent-to-add. Actual desktop size/mtime/SHA256 match the preserved
baseline; ferrodock-json-payload-desktop-preserved.json and -final-checks.json in owned SSD
temp record these checks. All owned build/validation jobs terminal; goal remains active.

## Previous fully validated checkpoint

# Current validation: numeric JSON codecs and expanded model projection

Full96955 terminal0 passes **1805 Rust tests /49 suites**, zero failed/ignored. JSON14,
ReactiveUI568,Domain225,UI373,markup189,diagnostics49,settings33,sample27,CoreSDK62.
All18 stages pass in /Volumes/SSD/temp/ferrodock-port/ferrodock-json-numeric-validation-summary.json:
strict debug/all-target and release-library Clippy, standalone14+62+2+7+3, seven normal
dependency boundaries, format/21Python, source/theme/inspector/two generators/six provenance
guards and expected full-completion rejection. No code/test change follows the green run.

Source16729 terminal0 regenerates1362 numeric cases,33 actual-model assertions,7 fallback
rows and the prior37stream/13JSON/12failure corpora plus goldens. Numeric records include
512 seeded bit patterns for each float type and selected format/range/literal boundaries.
Rust matches exact source wire/read results and finite roundtrip bits. Actual model projection
now includes NaN proportions/constraints, infinity, signed zero and floating-window bounds;
it remains explicitly limited and does not prove the default catalog or full sample layout.
Existing900 source factory assertions and prior4428 controls tests remain applicable.

Source59773 first failed a probe expectation that X=-0 would replace default+0; source
property equality keeps+0. Interrupted corpora caused4 failures in21405. Corrected the probe
to measured behavior;16729 regenerates full files and87466 passes all14 JSON tests. No Rust
production change was needed for that setter observation. Earlier69332 passes numeric2.
Ryū1.0.23 was already locked and in the model dependency graph; no new package version,
feature or retained source change. Full enum/scalar/catalog/cycle/performance parity remains
unproven. Final lightweight checks and desktop hash evidence follow this checkpoint.

Final lightweight checks pass: format (including include files), inventory, factory generator,
expected incomplete gate, source naming/unsafe/stub rules, original source diff empty and no
staged contents after intent-to-add. Actual desktop size/mtime/SHA256 match the preserved
baseline; numeric-desktop-preserved.json and numeric-final-checks.json under owned SSD temp
record the evidence. All owned build/validation jobs are terminal; the goal remains active.

## Previous fully validated checkpoint

# Current validation: actual model reference projection and base fallback

The latest locked workspace passes **1,803 tests /49 suites**, zero failed/ignored, including
12 JSON and568 ReactiveUI tests. Strict debug/all-target and selected release-library Clippy
pass. All18 driver stages pass, including standalone12+62+2+7+3, seven dependency boundaries,
format/21Python, source/theme/inspector/two generators/six provenance guards and expected
full-completion rejection. No production/test change follows the full green run. Results:
/Volumes/SSD/temp/ferrodock-port/ferrodock-json-fallback-validation-summary.json.

Source44813 terminal0 records28 actual-model checks, two model wire goldens, five fallback
depth/getter cases and two fallback cases plus a wire golden. The model projection is
explicitly limited; it does not prove full original model catalog/sample serialization.
Original37 stream,13 JSON,12 failure cases and prior900 factory assertions remain covered.

The source model tests exposed lost owner/root-window references after decoding: the actual
serialized fields now own references, matching source reachability. Existing lifetime tests
clear cycles where needed to isolate snapshot/window-release assertions. CLR cycle collection
remains unimplemented and is a material limitation. Initial poly full suite passed1801 before
Clippy found an unused import in the new test; removed. Fallback36054 then demonstrated an extra
getter at depth65. Production fallback dispatch now shares normal object-depth checks;52521
passes all12 serializer tests, including the source-matched depth/getter corpus.

No retained package versions/features/source changed since the prior4428 controls regression.
No actual desktop rebuild/restart; final binary preservation check is recorded below. Inventory remains
232/455/1241 of1928, with expected full-completion rejection. All owned temporary files use SSD.

Final lightweight checks confirm explicit include-file rustfmt, original source diff empty,
intent-to-add with no staged contents, and unchanged actual desktop size/mtime/SHA256.
Preservation manifest: /Volumes/SSD/temp/ferrodock-port/ferrodock-json-fallback-desktop-preserved.json.
No owned Cargo/validation process remains running. Source and generator checks are repeated
following this documentation update; the whole goal remains active and incomplete.

## Previous fully validated checkpoint

# Current validation: resolver-based JSON serializer

Locked workspace **1,797 tests / 49 suites**, zero failed/ignored: new serializer7,
ReactiveUI567, Domain225, UI373, markup189, diagnostics49, settings33, sample27, CoreSDK62.
Targeted71588 passes6JSON+16workspace+5stream before the final IO/finally test.58283 passes
final7JSON tests; the earlier locked invocation only requested a lock refresh after adding
a test dependency. Final full50330 terminal0: all **18 values zero** in
/Volumes/SSD/temp/ferrodock-port/ferrodock-json-validation-summary.json. Strict debug and
selected release-library Clippy, standalone7+62+2+7+3 tests, seven normal boundaries,
fmt/21Python, source/theme/inspector, both generators and all six package guards pass.
Complete-port rejection remains expected at232/455/1241. No production/test change after
that full green run.

Source factory81977 passes900 assertions (7new error/state cases). Serializer40486 passes
37stream+13JSON+12IO/finally assertions; source18272 establishes six string outputs and
payload wire data. Formerly source-only13 JSON cases now have typed-resolver Rust coverage.
The source JSON failure suite confirms close errors mask earlier failures, constructors
leave invalid streams undisposed, and serialization precedes output stream validation.
Default model catalog/polymorphism and the sample layout roundtrip are not covered.

Early all-target15976 exposes two old caller Result/Option unwraps, corrected;49434 passes.
Target55817 catches an accidental options type/default expression replacement, corrected;
71733 passes five initial JSON tests. Preflight62126 flags chunks_exact constant-size style;
equivalent as_chunks calls precede full validation. No assertion requirement was weakened.
New direct serde/serde_json dependencies use existing locked versions; raw_value is enabled
for lexical/duplicate-preserving parsing. Six retained package sources/hashes remain intact.
The controls regression is rerun separately with the new feature configuration.

Full28322 and49930 terminate on real linker ENOSPC, not test failures or observation timeout.
Owned logged stale harnesses were retired with manifests and no owned Cargo live. Test
profile then uses line-tables-only (incremental still false); dev/actual desktop unchanged.
Manifest ferrodock-json-debug-harness-retired.json records12 library test harnesses,
4,009,754,096bytes. The rebuilt main UI harness is365.3MiB versus405MiB before. Full50330
rebuilds and passes; cache/old attempts remain documented under ferrodock-json- on SSD.
Actual desktop executable is checked against its preserved hash before final checkpoint.
All owned temp and NuGet HTTP cache use the SSD. Other projects and original sources untouched.

The additional guarded controls run92422 also passes all4,428 tests with the new
serde_json/raw_value dependency feature and line-tables-only profile. Retained sources
match their exact provenance hashes; no dependency implementation patch was changed.

## Previous fully validated checkpoint

# Current validation: shared serializer streams

Locked workspace **1,789 tests / 47 suites**, zero failed/ignored. Domain model225,
ReactiveUI566, UI373, markup189, diagnostics49, settings33, sample27 and Core SDK62.
Targeted29516 completes five new model I/O tests and the existing15 workspace tests.
Shared stream tests cover native adapter/source identity, byte-array aliases, logical
position/length, closed/read-only/fixed behavior, failure order, custom nonclosing
Dispose, close reentry and actual nullable generic Load/Save dispatch.

New C# reference2179 passes37 matched stream/interface assertions and13 explicitly
SOURCE-ONLY JSON audits. Earlier67875 fails a probe expecting ObjectDisposedException
for closed MemoryStream.SetLength; actual source throws NotSupportedException first.
The probe and Rust check retain that actual order. Intermediate55803 passes the first33
stream cases; final2179 adds four original generic interface cases. The JSON audit compiles
four unchanged original serializer/options/resolver files and uses the original Core
project. It proves behavior for the next serializer implementation, not Rust JSON parity.
The earlier893 factory reference assertions remain unchanged.

First full driver62682 terminates on an actual linker ENOSPC while building the Fluent
library test harness. Six stale owned harnesses had already been retired (2,452,749,928
bytes); three obsolete variants were then retired (510,264,880bytes), all with manifests
and no owned Cargo live. The other observed Cargo was FerroBoard and was untouched.
Retry81373 completes all **17 checks**, every result zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-serializer-stream-validation-summary.json.
No production or test changes follow that green full run. Strict debug/release-library
Clippy, standalone62+2+7+3, six dependency boundaries, fmt/21Python, source/theme/inspector,
both generators and six retained-package guards pass. Full completion correctly rejects
the unchanged232/454/1242 inventory. Prior4,428 controls tests cover unchanged dependencies.

After all Cargo/rustc processes ended, five inactive incremental cache directories were
retired:3,974,305,547 logical bytes, newest file2026-10-22 22:30:40UTC. Workspace dev/test
profiles disable incremental and no environment override is set. Manifest:
ferrodock-serializer-stream-incremental-retired.json. Actual desktop and other projects
are preserved; all owned temp/TMPDIR use the SSD. Last observed SSD free2.9GiB, volatile.

## Previous fully validated checkpoint

# Current validation: source query cursor disposal

Locked workspace **1,784 tests / 47 suites**, zero failed or ignored: UI373, markup189,
diagnostics49, settings33, sample27, ReactiveUI566 and Core62. Targeted model session87906
passes3 tests; native78815 passes1 test covering 32 traces through each native factory view.
The shared corpus records direct/all-controls queries, root/child/nested predicate disposal,
true/false matches, recursive MoveNext, repeated disposal, actual null yields, later source
access failures and retained Current. Additional model checks cover before-start disposal,
unsupported reset, original model identity and independent repeat enumeration.

Original C# probes4032 and91173 establish the cleanup traces before production changes.
Final source reference55488 passes **893 assertions**,34new, all rows1. The source fixture
uses unchanged original Dock methods; tests/Dock/data/query-cursor-reference.tsv retains the
32 complete traces. Rust recognizes the corresponding source-access panics; the tests
do not assert CLR exception-object or custom enumerator cleanup equivalence.

Full driver52232 passes all1,784 workspace tests. Strict Clippy first rejects a redundant
borrow in test failure formatting, then resume95088 rejects test Boolean comparisons.
Both corrections affect tests only; production remains identical to the green full run.
Final resume75818 completes successfully with all **17 values zero** in
/Volumes/SSD/temp/ferrodock-port/ferrodock-query-dispose-validation-summary.json.
Strict debug all-target and selected release-library Clippy, standalone62+2+7+3 tests,
six normal dependency boundaries, fmt/21Python, source/theme/inspector, both generators
and all six package guards pass. Full completion still correctly rejects232/454/1242.

Six prior logged owned library test harnesses (2,452,252,344 bytes) were retired before
native/full validation, with no owned Cargo job live. The SSD manifest is
ferrodock-query-dispose-test-cache-retirement.json. No sample executable/bundle or other
chat artifact was retired. Dependencies remain unchanged; prior4,428 controls tests
apply. All owned temp/TMPDIR use the SSD. No native desktop rebuild or restart occurred.

## Previous fully validated checkpoint

# Current validation: nullable search and native cursor advancement

Locked workspace **1,780 tests / 47 suites**, zero failed or ignored: UI 373, markup 188,
diagnostics 49, settings 33, sample 27, ReactiveUI 563 and Core 62. Nine new model tests
cover nullable roots/callables, immediate versus deferred failure order, null yields,
fixed/virtual dispatch, live window Layout reads and original callback/model identity.
Four native tests cover both factory views, canonical optional declarations, actual
nullable overrides, cursor Current/fault/disposal scenarios and recursive MoveNext
from root and child predicates.

Original C# reference session 91934 completed successfully: **859 assertions**, 34 new,
all rows passing. Targeted model session 40694 passes nine tests. Native session 91316
first exposed fixture type errors, corrected without changing behavioral assertions.
Session 63643 then exposed a real recursive MoveNext borrow failure; the nocapture log
ferrodock-null-search-reentry-failure.log records it. After the per-cursor guard, native
session 3663 passes all four tests. Initial workspace check 34442 is superseded by the
successful Some-only caller migration/check 58987. Fixed Find dispatch is unchanged.

Full sequential driver session 28615 completed successfully. All **17 values** in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-search-validation-summary.json are zero:
workspace tests, strict debug all-target and selected release-library Clippy,
standalone 62+2+7+3 tests, six normal dependency boundaries, fmt, 21 Python tests,
source/theme/inspector checks, both generators and all six package integrity guards.
The complete-port gate correctly rejects the incomplete 232/454/1242 inventory of 1928.
No production change followed the full green run. Prior 4,428 controls tests cover the
unchanged retained dependency packages. Actual desktop executable/bundle/window and
original parent/sibling source are preserved. All owned temporary files use the SSD.
Broader cursor Dispose/reentry and full API/platform/lifetime parity remain outstanding.

## Previous fully validated checkpoint

# Current validation: shared nullable windows and lifecycle

Locked workspace **1,767 tests/47 suites**, zero failed/ignored: UI373, markup184,
diagnostics49, settings33, sample27, ReactiveUI554 and Core62. Eleven new model tests
cover default guards/failures, real null list/event payloads, initialization arguments,
committed effects before failure, live list replacement, optional host dispatch, root
initialization order, Show/Exit side effects, search/state traversal, live Layout argument
evaluation and nullable collection-root results reaching actual Add overrides.

Five native tests pass: compiled and runtime Windows Count/indexer/ItemsSource bindings,
IList mutations retaining actual null/model references, both Core factory views dispatching
all six window lifecycle slots and native host-factory casting/method/result identity.
No OS window is presented by these probes. Existing raw nonnull arguments remain tested
against canonical optional declarations; original handle comparisons remain intact.

Original C#1923 terminal0: **825 assertions**,40 new, all rows1/zero source-only. Earlier
86493 passes the first38 cases; final two cover actual nullable root-result dispatch.
Model30007 passes11; native20995 passes5. Earlier targeted6536 correctly exposed setup
Id logging in the custom getter fixture; its setup log is cleared before ordered link
assertions. No production requirement or behavioral assertion removed.

Full first82106 exposes two old Core metadata expectations: missing Option-window argument
comparison and old CloseWindow declaration lookup. Both now retain exact raw input/model
identity while checking the canonical optional type. Full55335 then passes all1,767 tests;
strict Clippy requests aliases for nested window-list storage types. Added only equivalent
private aliases in the reactive root and shared test fixtures. Resume47994 terminal0:
all17 values zero in /Volumes/SSD/temp/ferrodock-port/ferrodock-null-windows-validation-summary.json.
No runtime behavior change follows the passing workspace run.

Strict all-target/selected release-library Clippy, standalone62+2+7+3 tests, six normal
dependency boundaries, fmt/21Python, source/theme/inspector, two generators and six package
integrity guards pass. Complete-port gate still correctly rejects232/454/1242 of1928.
Previous all4,428 controls tests cover unchanged dependencies. Original/sibling sources,
staging and actual desktop executable/bundle/window remain preserved; all temp/TMPDIR
use the SSD. Full original goal remains active and incomplete.

## Previous fully validated checkpoint

# Current validation: nullable window options

Locked workspace **1,751 tests/47 suites**, zero failed/ignored: UI373, markup179,
diagnostics49, settings33, sample27, ReactiveUI543 and Core62. Four new model tests
cover direct null argument rejection, protected null-options short circuit, unchanged
window fields/notifications, actual hook handling/delegation, shared option/window
identity and setter callbacks that change subsequent parent/modal/taskbar reads.
Two native tests cover original null argument error, canonical optional parameter,
raw/optional window conversion and live callback behavior on the same source models.

C#60128 terminal0: **785 assertions**,14 new, all rows1 and zero source-only rows.
Targeted64716 terminal0/passed4. Initial check49630 found two old factory-window test
call sites; they now wrap the original target in Some without changing assertions.
Workspace92613 passes all1,751 tests, then strict Clippy rejects err().expect() in
one new test. Replaced with equivalent expect_err(); production did not change.
Resumed62657 terminal0 runs strict Clippy and all remaining checks. All **17 values**
are zero in /Volumes/SSD/temp/ferrodock-port/ferrodock-null-window-options-validation-summary.json.
The resume preserves the successful workspace result and records the test-only style
correction rather than rerunning unchanged production tests.

Strict all-target/selected release-library Clippy, standalone62+2+7+3 tests, six normal
dependency boundaries, fmt,21 Python tests, source/theme/inspector checks, both generators
and all six retained package guards pass. The full completion check correctly rejects
232 ported/454 partial/1242 pending of1928. The two option units regained ported status
only after the missing contracts were implemented and checked. Prior all4,428 controls
tests cover unchanged dependencies. Actual desktop executable/bundle/window, original
parent/sibling source and staging are preserved; all temp/TMPDIR use the SSD.

## Previous fully validated checkpoint

# Current validation: nullable focus and activation

Locked workspace **1,745 tests in47 suites**, zero failed/ignored: UI373, markup177,
diagnostics49, settings33, sample27, ReactiveUI539 and Core62. Six model tests cover
source null defaults, optional custom root lookup, committed focus/init before final
null-owner selection failure, actual OnSelected policy changes/abort and real host
activation clearing or replacing root.Window. Two native consumers cover both Core
factory views, canonical optional declarations, actual null override dispatch and
fixed ActivateWindow calling virtual FindRoot. Existing raw nonnull argument tests
still check the same original objects, with normalized optional expected values.

Original C#4540 terminal0: **771 assertions**,21 new, zero source-only rows. The first
new-reference run98652 and targeted run41215 exposed additional original state-sync
root lookups missing from the test expectations. Both source and Rust expectations now
retain these calls in order; no production callback or assertion requirement removed.
Targeted67081 passes6; final17983 passes6 with the original owner explicitly retained
before initialization, then all-target strict Clippy passes. That last change strengthens
a test only; production is identical to the full green workspace run.

Full validation71072 terminal0: all **17 checks** in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-focus-validation-summary.json are zero.
Workspace/all-target and selected release-library Clippy; standalone62+2+7+3 tests;
six normal dependency boundaries; fmt;21 Python; source/theme/inspector; both generators
and all six retained package integrity guards pass. Source-only Some adaptation71425
passes after8 rounds; initial74077 is superseded. No actual desktop binary/bundle/window
was rebuilt or restarted. Prior all4,428 controls tests cover unchanged dependencies.

The inventory audit downgrades two existing window-option units to partial for missing
null-window input behavior. Final source/completion checks run again after that audit:
230 full/456 partial/1242 pending of1928; complete-port rejection remains required.
Original/sibling source and staged contents are preserved. Full original goal ACTIVE,
incomplete/no blocker. All new temporary files and child TMPDIR use the SSD.

## Previous fully validated checkpoint

# Current validation: nullable floating/window overloads

Locked workspace **1,737 tests in47 suites**, zero failed/ignored. Included UI373,
markup175, diagnostics49, settings33, sample27, ReactiveUI533 and Core62. Eight new
model tests check no-op null/unsupported creation, ownership short circuits, actual
virtual wrapper dispatch, default operation failure, parent capture before a null
result, prepare abort/mutation, apply order, nullable split lifecycle and ordinary
coordinate fallback with original handles. Three native consumers cover both factory
views, all eight overloads, object-null versus typed-null behavior, returned window/
options/parent identity and actual ownership/modality/taskbar application.

Original C#27452 terminal0: **750 assertions**,27 new, zero source-only rows. Targeted
model61806 passed8. Original code itself independently verifies the same null paths,
callback order, applied flags and identity. No window is presented or opened by probes.
Existing raw nonnull argument and canonical optional metadata tests continue to pass.

Initial44542 diagnostics included required Option trait upcast and an accidental expect
in unrelated nonnull DockAsDocument; removed before successful checks. Source-only Some
migration then found sample override signature; propagated the same Option to its base
call without changing conditional title or rebuilding actual desktop. Fresh adaptation
55312 terminal0 after4 rounds validates old test call sites; targeted43322's earlier
compile diagnostics are superseded. No behavioral assertion was removed or relaxed.

Full validation17308 terminal0: all **17 gates** pass in one sequential driver. Strict
workspace all-target/selected release-library Clippy; standalone62+2+7+3 tests; six normal
dependency boundaries; fmt;21 Python; source/theme/inspector; both generators and six
retained dependency integrity guards pass. Full completion correctly rejected:
232 full/454 partial/1242 pending of1928. No source status upgrade.

All17 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-floating-validation-summary.json.
Driver ferrodock-null-floating-validate.py and all logs share that SSD prefix. All owned
jobs terminal. Prior all4,428 controls tests cover unchanged retained dependency. Source
original/sibling files, staged contents and actual desktop executable/bundle/window
preserved. New files intent-to-add; full original goal remains active and incomplete.

## Previous fully validated checkpoint

# Current validation: nullable hidden-dockable hide/restore

Locked workspace **1,726 tests in47 suites**, zero failed/ignored. Included UI373,
markup172, diagnostics49, settings33, sample27, ReactiveUI525 and Core62. Six new
actual-factory model cases cover root no-ops/default failures, pin-hook rerooting and
hidden-list creation before Owner failure, nullable membership/collection removal/event
before OriginalOwner failure, null payload/index, observer list replacement, live hidden
list reentry and redirected original owner with exact original model/list identities.
Three native consumers check both factory views, actual nullable overrides, committed
removal/event and ID predicate failure/root ordering on actual uninitialized DockControls.
Canonical optional declarations and existing raw nonnull handles remain tested.

Original C#29810 terminal0: **723 assertions**,27 new and zero source-only rows.
Targeted model38852 passed6. Initial check13096 diagnostics superseded by Some-only
adaptation33990 terminal0 after4 rounds. All same original Rc handles; no behavioral
assertion, source callback or failure requirement relaxed. ID helper not-null result
guards match the source while first-null Id predicate failures remain explicit.

Full validation16333 terminal0: all **17 gates** pass in one sequential driver. Strict
workspace all-target and selected release-library Clippy; standalone62+2+7+3 tests;
six normal dependency boundaries; fmt;21 Python; source/theme/inspector; both generators
and all six retained dependency guards pass. Full completion correctly rejected:
232 full/454 partial/1242 pending of1928. No source status upgrade.

All17 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-hidden-validation-summary.json.
Driver ferrodock-null-hidden-validate.py and all stage logs use the same SSD prefix.
All owned jobs terminal. Prior all4,428 native controls tests cover unchanged patch.
Original C#/sibling source, staged contents and actual desktop executable/bundle/window
preserved. New files intent-to-add; full original goal remains active and incomplete.

## Previous fully validated checkpoint

# Current validation: nullable pinned preview/hide lifecycle

Locked workspace **1,717 tests in47 suites**, zero failed/ignored. Included: UI373,
markup169, diagnostics49, settings33, sample27, ReactiveUI519 and Core62. Eight new
model tests cover root lookup/no-op, cleanup before incoming null failure, fixed toggle
relookup, null snapshot short circuit and partial mutations, live preview removal versus
cached final state, nullable Init recovery/default failure after list commit, post-Init
root clearing, and first-nonsplitter kept selection. Two native consumers verify typed
versus object null calls and actual nullable Init recovery through both factory views.
Original list, preview and model Rc identities are asserted throughout.

Original C#43236 terminal0: **696 assertions**,27 new, zero source-only rows. Targeted
model10156 passed8. Initial model3705 only lacked the INPC trait import in the new test
module; importing the existing trait fixed compilation. Initial64667 diagnostics were
superseded by Some-only adaptation77315 terminal0 (four rounds). Existing nonnull callers
retain original handles; canonical optional declaration lookups retain raw-handle and
behavior assertions. No source engine expectation or callback was removed or relaxed.

Full validation94406 terminal0: all **17 gates** pass in one run. Strict workspace
all-target and selected release-library Clippy; standalone62+2+7+3 tests; six normal
dependency boundaries; fmt;21 Python; source/theme/inspector; both generators and all
six dependency integrity guards pass. Full completion correctly rejected:
232 full/454 partial/1242 pending of1928. No source status upgrade.

All17 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-preview-validation-summary.json.
Driver ferrodock-null-preview-validate.py and all logs use that SSD prefix. All owned
jobs terminal. Prior all4,428 native controls tests cover unchanged retained patch.
All new files intent-to-add and staged contents empty; initial subdirectory pathspec
error was corrected with the parent Git path. Original C#/sibling source and actual
desktop executable/bundle/window preserved. The full original goal remains active.

## Previous fully validated checkpoint

# Current validation: nullable moves, swaps and connected pin/capability APIs

Locked workspace **1,707 tests in47 suites**, zero failed/ignored. Included: UI373,
markup167, diagnostics49, settings33, sample27, ReactiveUI511 and Core62. Eight new
model tests and three native consumers exercise original shared lists, nullable slots,
exact mutation/events/state callbacks, reentrant swaps, default Init failure after list
commits, unpin-before-target order, target creation before source access, first-strip
membership and actual enclosing overrides. Default capability entry accesses are
covered through both direct and native calls. Canonical optional metadata and original
raw nonnull model handles both pass existing operation/pinned-order assertions.

Original C#42630 terminal0: **669 assertions**,38 new, zero source-only rows. Targeted
model30668 passed8. Initial43422/C#24309 exposed expected traces omitting original
state synchronization after nonnull added/removed events. Both references now explicitly
include the exact five FindRoot calls for each recorded payload; no callback filtered.
C# source itself independently verifies those complete traces.

Validation4163 stopped at one old Core test's MovePinnedDockable declaration lookup.
Only its Rc parameter type lookup became canonical Option; raw handles, pinned order
and behavior assertions unchanged. Run68875 then passed all1,707 tests, stopping at
strict lint on3 redundant cast-parenthesis pairs (6 messages). Removed exactly those
parentheses per lint; no expression behavior changed. Resume28166 terminal0 confirms
strict workspace all-target and selected release-library Clippy plus all remaining
checks. Already-passing workspace behavior was not rerun for those syntax-only edits.

All **17 gate values** are zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-move-validation-summary.json.
Drivers ferrodock-null-move-validate.py and -resume-validation.py; logs same SSD prefix.
Failed run logs preserved as attempt1/attempt2. Standalone62+2+7+3 tests; six normal
dependency boundaries; fmt;21 Python; source/theme/inspector; both generators and all
six retained dependency integrity guards pass. Full completion correctly rejected:
232 full/454 partial/1242 pending of1928. No source status upgrade. All owned jobs
terminal. Prior all4,428 native controls tests cover unchanged retained dependency.
Original C#/sibling sources, staged contents and actual desktop executable/bundle/window
preserved. Full original API/backend/serialization/CLR/platform/lifetime goal remains active.

## Previous fully validated checkpoint

# Current validation: nullable factory targets, pin root and protected helpers

Locked workspace **1,696 tests in47 suites**, zero failed/ignored. UI373, markup164,
diagnostics49, settings33, sample27, ReactiveUI503 and Core62 included. Seven new
model cases exercise actual composed factories, original shared lists, nullable Init
side effects, all six protected list helpers, unused removed-item selection, pin-root
no-op and original failure paths. Both bounds sources and both owner branches pass.
The new native consumer checks default Init side effects before null-target failure
through both actual factory views. Extended native dispatch accepts null target/item
in actual Add overrides. Existing raw nonnull handle/API tests continue to pass.

Original C# reference3171 terminal0: **631 assertions**,27 new, zero source-only rows.
Targeted model46595 passed7 cases before the full suite. Its initial15599 compile
failure was a test getter typo, corrected to original GetPinnedBounds/NaN checks.
All-target adaptation31000 terminal0 used compiler-suggested Some(same Rc) only,
after initial92897 diagnostics. No assertion or source failure order was relaxed.

Full validation31139 terminal0: all **17 gates** pass in a single driver run. Strict
workspace all-target and selected release-library Clippy; standalone62+2+7+3 tests;
six normal dependency boundaries; fmt;21 Python tests; source/theme/inspector checks;
both generators and six retained dependency integrity guards pass. Full completion is
correctly rejected:232 full/454 partial/1242 pending of1928. No source status upgrade.

All17 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-target-validation-summary.json.
Driver ferrodock-null-target-validate.py and all logs use that SSD prefix. All owned
jobs terminal. Previous all4,428 native controls tests still cover the unchanged
retained patch. Original C#/sibling sources, staged contents and actual desktop
executable/bundle/window preserved. Remaining broader API/backend/serialization/
CLR/platform/lifetime scope keeps the whole original goal active.

## Previous fully validated checkpoint

# Current validation: nullable insertion/removal and pin cleanup

Locked workspace1,688 tests in47 suites pass, zero failed/ignored. Native UI373,
markup163, diagnostics49, settings33, sample27, ReactiveUI496 and Core62 are included.
Six new actual-factory model tests cover negative-null insertion, nullable Init/list
identity/events, invalid-index and null-target failure order, removal root/pin cleanup,
protected pin overload dispatch and whole-pane nullable snapshot mutation. Two new
native consumers cover actual null values through both original factory metadata views,
the same shared list and enclosing overrides. Existing canonical optional declaration
and raw nonnull model-handle tests also pass.

Original C# reference46544 is terminal (handle missing; successful log). Its corpus
has604 passing assertions,20 new and zero source-only rows. All17 gates pass: strict
all-target and selected release-library Clippy; standalone62+2+7+3 tests; six normal
dependency boundaries; formatting;21 Python tests; source/theme/inspector checks;
both generators and all six retained dependency integrity guards. Full completion
is correctly rejected.

Main52674 stopped at factory-generator after all code/behavior gates passed. Exact
diff showed only whitespace in the generated InsertDockable macro signature after
trait formatting; a.split()==b.split() proved token identity. Regenerated the file,
then resume63749 terminal0 passed formatting and remaining structural/generator gates.
No behavioral assertion or completion requirement was removed or relaxed. All17 values
are zero in /Volumes/SSD/temp/ferrodock-port/ferrodock-null-mutation-validation-summary.json.
Drivers ferrodock-null-mutation-validate.py and -resume-validation.py and all stage
logs use that SSD prefix. All owned jobs are terminal.

No dependency or desktop executable/bundle/window changes. The previous all4,428
native controls tests cover the unchanged retained patch. Full source inventory stays
232 full/454 partial/1242 pending; broader original API/backend/serialization/platform/
lifetime scope remains required and goal completion remains unproven.

## Previous fully validated checkpoint

# Current validation: nullable state hooks and root recursion

The locked workspace passes **1,680 tests in 47 suites**, zero failed or ignored.
Native UI373, markup161, diagnostics49, settings33, sample27, ReactiveUI490 and Core62
are included. Four new model cases use actual composed factories to check null child
and split-extra order, resolver-before-assignment/scope unwind, owner mutation before
recursive virtual FindRoot, null pin membership and first dereference order. One
independent consumer checks null FindRoot/pin calls through both native factory views.
The original-engine C# reference passes **584 assertions**, including thirteen new
matching state/root/pin cases. No source-only rows remain.

All **17 current gates pass**: strict all-target workspace and selected release-library
Clippy; standalone Core/recycling/backend/runtime tests62+2+7+3; six normal dependency
boundaries; formatting; 21 Python tests; source/theme/inspector checks; both generators;
and all six retained dependency integrity guards. No vendor/dependency change was made,
so the previous all4,428 controls-test result still covers that unchanged patch.
Full completion is correctly rejected:232 full/454 partial/1242 pending of1928.

Driver30816 completed code gates then required the new test file's original-source
mapping. That mapping was added; resume46890 is terminal0 and completed the remaining
documentation/generator gates. No failed behavioral assertion was removed or relaxed.
All17 entries are zero in
`/Volumes/SSD/temp/ferrodock-port/ferrodock-null-state-validation-summary.json`.
Drivers are `ferrodock-null-state-validate.py` and
`ferrodock-null-state-resume-validation.py`; logs share that prefix. Reference78108
and targeted-model79957 are terminal0. All temp artifacts/TMPDIR use SSD.

The nearly-full SSD required retiring36 rebuildable library test harnesses (7.2GiB)
with no owned build live. Sample-prefixed files, actual desktop/bundle/window, source,
native/dynamic libraries and other chats' builds were preserved. Manifest:
`ferrodock-null-state-test-cache-cleanup.json` in the same SSD directory.
Original C#/sibling sources and staged contents remain unchanged. New files are
intent-to-add. Remaining nullable factory/service arguments and the broader original
API/serialization/backend/CLR/platform/lifetime scope keep the full goal active.

## Previous validated checkpoint

# Current validation: nullable collection elements and native object values

The locked workspace passes **1,675 Rust tests in 47 suites**, with zero failures or
ignored tests. This includes 373 native UI, 160 downstream markup, 49 diagnostics,
33 settings, 27 sample, 486 ReactiveUI model and 62 Core SDK tests. Nine new cases
cover source null-list behavior, compiled/runtime consumers, native change payloads
and query/delegate boundaries. Selection and replacement assertions retain exact
original Rc identity through the public native conversion.

The original-engine C# reference passes **571 assertions**, with no source-only rows.
It verifies base Add through a null-aware Init override, list normalization, null
query values/predicates, cleanup and original compiled-indexer staleness until rebind.
Count and ItemsSource updates remain live. The retained controls package additionally
passes **all 4,428 unit tests**, including its two new nullable boxing regressions.
Its standalone SSD copy was checked against every retained source hash. Earlier
filtered item/selection test groups are subsets, not additional test counts.

All **17 sequential validation gates pass**. These cover strict workspace all-target
and selected release-library Clippy; standalone Core/recycling/backend/runtime tests
(62 + 2 + 7 + 3); six normal dependency boundaries; formatting; 21 Python tests;
source/theme/inspector checks; both generators; and all six retained package provenance
guards. Full completion is correctly rejected: 232 full, 454 partial and 1,242 pending
files out of 1,928. Protected nullable state hooks, remaining factory/service arguments,
generic/CLR/lifetime behavior, serializers and the remaining original scope stay open.

Final driver session18467 is terminal0. All 17 values in
`/Volumes/SSD/temp/ferrodock-port/ferrodock-null-list-validation-summary.json` are zero.
The driver is `ferrodock-null-list-validate.py`; per-stage logs share that prefix.
Additional evidence: `ferrodock-null-list-controls-all-tests.log`,
`ferrodock-null-list-reference-final.log` and `ferrodock-null-list-test-counts.json`.
All paths are in the same SSD task directory, and every subprocess inherits TMPDIR
there. The first link attempt hit SSD errno28; superseded build outputs were removed
and the unchanged gates retried. Earlier fixture/transport assertions were corrected
to retain source identity and behavior, and strict Clippy issues were resolved with
type aliases rather than suppressed lints. Failed/intermediate logs remain available.

Original C# and sibling sources, actual desktop executable/bundle/window and staged
contents are preserved. New files are intent-to-add. See CONTINUATION.md for exact
remaining work and the owned cache-cleanup manifests.

## Previous validated checkpoint

# Current validation: nullable factory overrides and original argument dispatch

The full locked workspace passes **1,666 Rust tests in 47 suites**, zero failed or
ignored. UI372, markup157, diagnostics49, settings33, sample27, source models482,
Core61, recyclingSDK2, backendSDK7 and runtime3.
Targeted source tests pass **482**; independent consumers pass **157**. Four new
source cases verify original document/tool three-call factory order with null, owner
identity, factory replacement/removal between calls, base Add dispatching null to an
initialization override and original default first-dereference failures. The existing
two compiled/runtime null-command consumers now also exercise null-aware factory
subclasses, and one new direct native consumer checks all three slots through both
original interface and abstract base metadata.

The unchanged C# reference passes **544 assertions** (545-line TSV). Thirteen new
matched-source assertions cover the same factory/owner/reread/default/command cases.
One additional explicitly SOURCE-ONLY assertion proves original base AddDockable
appends a null IList element after a custom initialization override accepts it. That
list element is unsupported by the current Rust list representation; it is not a
matched Rust case. Full nullable list/argument/CLR behavior remains incomplete.

All-target workspace cargo check passed during migration. Strict workspace all-target
and release-library Clippy pass. Standalone Core/recyclingSDK/backendSDK/runtime tests
pass61+2+7+3. All six normal dependency boundaries, formatting, 21 Python tests,
source/theme/inspector checks, both generators and all five vendor integrity guards
pass. The owned sequential validation session10004 closed0, with all17 stages zero in
`/Volumes/SSD/temp/ferrodock-port/ferrodock-null-factory-validation-summary.json`.
The final gate correctly rejects full completion (232 ported / 454 partial / 1242 pending).

Command: `/Volumes/SSD/temp/ferrodock-port/ferrodock-null-factory-validate.py`; all
stage logs use that prefix. Every subprocess inherited TMPDIR on that SSD directory,
per user request. Original C# reference command is recorded in its harness README.
No dependency/vendor changes or actual desktop sample refresh was made. All new files
are intent-to-add, authored whitespace is clean, staged contents are empty, and original
parent README/src/tests/samples are unchanged.

## Previous validated checkpoint

# Current validation: source nullable callbacks and original virtual arguments

The locked workspace passes **1,661 Rust tests in 47 suites**, zero failed/ignored:
UI 372, markup 156, diagnostics 49, settings 33, sample 27, ReactiveUI model 478,
Core SDK 61, recycling SDK 2, backend SDK 7 and shared runtime 3. Four new model tests
cover null callback/no-Factory no-op, original nullable document/tool override slots,
and default-factory failure before graph mutation. Two new independent consumers
exercise compiled and interpreted three-command null flows. The six temporary runtime
adapter tests were removed with that now-unnecessary module; direct callback identity
and native cross-assignment remain covered by existing Core and consumer cases.

The unchanged original C# reference passes **530 assertions** (531-line TSV, every
assertion passing). The formerly source-only no-Factory null/virtual cases now have
Rust parity coverage. Five new source assertions cover the compiled three-command
flow, document/tool overrides and detached ToolDock.AddTool(null).

Workspace validation log: `/Volumes/SSD/temp/ferrodock-port/ferrodock-null-virtual-workspace-tests.log`.
Reference log: `/Volumes/SSD/temp/ferrodock-port/ferrodock-null-virtual-reference.log`.
Strict all-target workspace and release-library Clippy pass. Standalone Core,
recycling SDK, backend SDK and runtime tests pass 61+2+7+3. All six normal dependency
boundaries, formatting, 21 Python tests, source/theme/inspector checks, both generators
and all five retained dependency integrity guards pass. The owned sequential pipeline
is terminal with all 17 stages recorded as successful in
`/Volumes/SSD/temp/ferrodock-port/ferrodock-null-virtual-validation-summary.json`; its final stage confirms that
full completion remains rejected. No vendor/dependency changes were made.
Authored whitespace is clean, all new files are intent-to-add, staged contents are
empty and original parent README/src/tests/samples are unchanged. The actual desktop
sample executable/bundle/window was preserved; these checks did not refresh its UI.
The full objective remains incomplete, including present custom-factory null arguments
and the broader original source/API/platform requirements. The inventory remains
232 ported / 454 partial / 1242 pending of 1928.

## Previous validated checkpoint

# Current validation: shared callback cross-assignment and native identity

The locked workspace passes **1,661 Rust tests in 47 suites**, zero failed/ignored:
UI 372, markup 154, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 61, recycling SDK 2, backend SDK 7 and shared runtime 9.
Six new runtime tests cover deferred/reused adapters, both exact typed Rc roundtrips,
native null results versus required Rust contract failure, distinct identity, captured
state release/cache cleanup, callback-body reentry and capture-drop reentry.
Two Core cases exercise original factory properties and locator maps in both directions,
including native Func null result/identity without eager execution. Two independent
consumers cover compiled/runtime TwoWay object assignment, source identity after rebind,
original callback recovery and exactly one original CreateDocument engine invocation.

The unchanged original C# reference passes **525 assertions** (526-line TSV). New
source cases verify matching TwoWay object binding, locator-map roundtrip, deferred
execution, native null result and identity. Two additional SOURCE-ONLY assertions
prove remaining engine behavior: AddDocument(null) without Factory is a no-op and a
subclass override receives null. These do not have Rust parity and are not presented
as matched cases. The required Rust callable cannot represent that result. Full
null-argument/override semantics and general CLR delegate scope remain unfinished.

The first validation passed workspace tests but strict Clippy requested aliases for
two nested weak-callback types. Private aliases replaced the field types without
behavior changes. The final workspace rerun includes the aliases and all tests above.
Strict all-target workspace and release-library Clippy pass. Standalone Core,
recycling SDK, backend SDK and runtime pass 61+2+7+9. All six normal dependency boundaries,
formatting, 21 Python tests, both generators, source/theme/inspector checks and all five
retained dependency integrity guards pass. A final Core-test needless-borrow lint fix
is covered by subsequent strict Clippy and standalone Core tests; production behavior
is unchanged. Final job 62205 closed 0 and every stage in
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-validation-summary.json is 0. Commands and logs are listed in
CONTINUATION.md. No vendor/dependency patches or engine algorithms changed.
The completion gate correctly rejects the unfinished original scope.

Inventory remains 232 ported / 454 partial / 1242 pending of 1928. Native callback
cross-assignment is implemented, but the document-factory record stays partial for
broader delegate metadata/Equals/multicast/CLR semantics; the null-engine edge is
tracked on the source DocumentDock class. Full completion remains correctly unproven.

## Previous validated checkpoint

# Current validation: native document factories and original template attributes

The locked workspace passes **1,651 Rust tests in 47 suites**, zero failed/ignored.
UI 372, markup 152, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 59, recycling SDK 2, backend SDK 7 and runtime 3. The unchanged C# reference
passes **514 assertions** (515-line TSV), including native method assignment,
Core-qualified Func binding/invocation, class-property callback identity, original
CreateDocument graph ownership/activation and CLR cross-nullability delegate identity.
The three new assets run through both Rust compiler hosts and runtime interpretation.

Six new Core cases verify original factory contract/checked membership, callback
identity/deferred execution/null, one canonical Func descriptor for both source forms,
original CreateDocument engine dispatch, capture release, exact fourteen source
RequiresDataTemplate attributes in runtime/compiler exports and invalid known method
signature rejection. Canonical Invoke rejects invalid receivers with structured
errors. One backend case verifies original class/interface/callback identity without
general UI controls. Three independent consumer cases exercise native method assignment,
Core Func invocation, concrete class creation and source docking graph behavior.

The runtime consumer initially omitted application fixture registration, then assumed
a single Rust boxing form for the returned root. It now uses the fixture's composition
root and standard registered source casts. The isolated runtime case passes; final
workspace validation includes all three cases. No vendor changes or application
workaround to the docking engine was introduced.

Strict all-target workspace and release-library Clippy pass. Standalone Core,
recycling SDK, backend SDK and runtime pass 59+2+7+3 tests. All six normal dependency
boundaries pass. Formatting, 21 Python tests, both generators, source/theme/inspector
checks and all five retained dependency integrity guards pass. Job 65941 closed 0;
every stage in /Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-validation-summary.json is 0. Exact commands
are in /Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-validate.py; logs are listed in CONTINUATION.md.
The completion gate correctly rejects the unfinished original scope.

The inventory records 232 ported / 454 partial / 1242 pending of 1928. Eleven
interface files completed their sole remaining attribute-metadata gap. The public
RequiresDataTemplateAttribute class/System.Attribute/AttributeUsage API remains pending.
IDocumentDockFactory remains partial until native cross-assignment between the two Rust
callback representations preserves CLR identity; shared metadata and correct individual
source views do not prove that requirement. General delegate/CLR, collection, serializer,
platform and wider source parity remain incomplete. The full goal stays active.

## Previous validated checkpoint

# Current validation: original Model.Controls Core ownership

The locked workspace passes **1,641 Rust tests in 47 suites**, zero failed/ignored.
UI 372, markup 149, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 53, recycling SDK 2, backend SDK 6 and runtime 3. The unchanged C# reference
passes **502 assertions** (503-line TSV). Six new source assets explicitly enable
compiled bindings; the same scenarios run as emitted and interpreted Rust consumers.

Six new standalone Core cases cover 22 control-contract owners and original handles,
MDI inheritance/local-target identity/source notifier and setters, tool-template
arbitrary content/null/events, optional tool-content checked casts and source method
dispatch, shared root-window list mutations/Count events, and complete scanner facts.
Seven downstream cases cover root windows/pins, document/tool Add method commands
and original graph ownership/activation, inherited MDI title/state, nullable grid
TwoWay definitions, stack spacing/orientation and both runtime compiler paths.

The first consumer build correctly rejected unread casts introduced by wildcard
imports in new registration helpers. Explicit original trait imports resolve those
facts. Core has no unread native registrations; UI has no unread casts. No compiler
completeness checks or retained dependency patches were changed. A compiler-export
regression test initially needed an Option field type correction; final workspace
validation includes the corrected test.

Strict all-target workspace and release-library Clippy pass. Standalone Core,
recycling SDK, backend SDK and runtime pass 53+2+6+3 tests. All six normal dependency
boundaries pass. Formatting, 21 Python tests, both generators, source/theme/inspector
checks and all five retained dependency integrity guards pass. Job 26479 closed 0;
every stage in /Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-validation-summary.json is 0. Exact commands
are in /Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-validate.py; logs are listed in CONTINUATION.md.
The completion gate correctly rejects the unfinished original scope.

Inventory: 221 ported / 465 partial / 1242 pending of 1928. IDocumentDockFactory is correctly
partial because native delegate metadata remains unfinished. RequiresDataTemplate,
full generic collection metadata and broader source/API/platform/lifetime parity
remain open. The full goal is active and incomplete.

## Previous validated checkpoint

# Latest validation: Core host contract and shared binding fixes

The final locked workspace passes **1,628 Rust tests in 47 suites**, zero failed or
ignored: UI 372, markup 142, diagnostics 49, settings 33, sample 27, ReactiveUI model
474, Core SDK 47, recycling SDK 2, backend SDK 6 and runtime 3.

Seven targeted independent consumers pass after the Core target/container ownership
and host metadata additions. They exercise real native host geometry/layout/state/
presentation/exit, compiled Core bindings and source command invocation, raw Control
identity, TwoWay string/null assignment and both runtime paths. Weak interface tests
restore host/nullable and inherited brush views; the brush is released when its owners
drop. Original HostWindowState retains its source strong readonly host reference;
this does not establish general CLR cycle-collection parity.

The unchanged C# reference was rerun after the tool interruption and passes **470
assertions** (471-line TSV), including 28 host/container/marker cases. Source mappings,
20 Python tests, both generators, inspector/theme comparisons and all five vendor
integrity guards pass. The base patch restores native weak interface handles through
existing class interface facts. Markup and runtime-loader patches normalize object
setter transport boxes through a shared helper. Exact published-source hashes and
patch dry-runs verify the retained dependency changes.

Strict all-target workspace and release-library Clippy pass. Standalone Core,
recycling Model, backend SDK and runtime pass 47+2+6+3 tests. Normal dependency trees
confirm Core/recycling SDK/runtime exclude general UI controls, general Dock controls
exclude the concrete backend/SDK, backend SDK excludes general Dock UI/runtime compiler,
and Domain excludes native base/controls. Backend SDK retains native controls through
recycling. Formatting passes. Commands and logs are recorded in CONTINUATION.md.
Completion still rejects the unfinished original scope.

## Previous validated checkpoint

# Latest validation: Core dock contracts and recycling Model SDK

The final locked workspace passes **1,616 Rust tests in 47 suites**, zero failed or
ignored. Native UI 372, downstream markup 135, diagnostics 49, settings 33, sample 27,
ReactiveUI model 474, Core SDK 42, recycling Model SDK 2, backend SDK 6 and runtime 3.
Strict all-target workspace Clippy passes. Standalone Core/recycling Model/backend/runtime tests pass (42+2+6+3). Strict release
library Clippy passes for runtime, recycling SDK/UI, Core/backend SDK, general UI,
diagnostics, markup extension and sample. Normal dependency trees verify Core and
recycling SDK exclude UI controls, runtime excludes UI, general Dock controls exclude
the concrete backend/SDK, and Domain excludes native base/controls. The backend SDK
retains native controls through recycling. Logs are listed in CONTINUATION.md.

Three original dockable metadata tests moved from UI to Core without algorithm changes.
Three new Core cases verify original logical ownership, inherited recycling contract,
checked source downcasts, shared observable-list mutations/Count notifications and
separately boxed reference identity, source command identity and actual navigation.
Two recycling SDK cases cover null/empty/stable IDs, original property/arguments/results,
shared identity and original interface method dispatch using an application implementation.
Four downstream cases validate compiled Core-qualified interfaces, live inherited/list
bindings, TwoWay changes, native Button execution of original Navigate, exact raw Control
Context projection, existing native cache calls and both compiler hosts. Runtime views
are loaded with compiled bindings enabled and verified against live original models.

The original unchanged C# assemblies pass **442 assertions** (443-line TSV), with
18 added Core-contract/cache/reference cases and equivalent compiled source assets.

Core owns IDockable, IDock and typed-list descriptors through original FerroDock.Model.
The new recycling Model SDK owns its original logical assembly. UI aliases preserve the
same declaration identity. Core uses the existing shared runtime bridge; recycling UI
registers raw Ref<Control> type knowledge to retain the previous object projection.
No original Domain/ReactiveUI engine algorithm, sibling source or vendor patch changed.
The main inspector and three property views retain source layout/behavior, with Core
assembly qualification guarded by exact source comparison and 20 Python regressions.

Inventory remains 222 ported / 464 partial / 1242 pending of 1928; the completion gate correctly
rejects it. Native TryGetValue bool/out metadata remains absent rather than redefined.
Complete collections, generic/default/out metadata, remaining Core ownership, CLR
identity/lifetimes/serialization and native platform parity remain unfinished.
The full goal remains active; the running desktop sample was preserved.

## Previous capability checkpoint

# Latest validation: Core capability/window ownership and original POCO APIs

The locked workspace passes **1,607 Rust tests in 45 suites**, zero failed or ignored.
Native UI375, downstream markup131, diagnostics49, settings33, sample27, ReactiveUI
model474, Core SDK36, backend SDK6 and runtime3. One unchanged window metadata test
moved from UI to Core. Six new Core cases cover policy/interface identity, nullable
properties/inheritance, custom virtual Get dispatch, options/defaults/application/null
failure, immutable evaluation constructor tuples and original resolver context/precedence.
Four downstream cases compile six Core-qualified assets, retain original model objects,
update parent-window title bindings, edit options TwoWay, invoke the actual factory,
bind immutable results and compile through both hosts using public Core paths.

The unchanged C# reference passes **424 assertions**, including equivalent source assets,
all six nullable capability properties, virtual policy overrides, taskbar/parent option
application and null-window rejection, 27 evaluation tuples and resolver context cases.
DockCapabilityValueSource is the 23rd Core enum descriptor, generated from its original
Domain declaration with a hidden declaration hook. Domain algorithms are unchanged.

IDockCapabilityPolicy, its two concrete policies and IDockWindow now have their source
Core logical owner; general controls retain aliases to the same declarations. Only
inspector window references move to Core. Exact source-markup comparison and two new
Python regressions reject unrelated changes or old ownership. All 20 Python verifier
tests, both generators, inventory/theme/inspector checks and all five vendor guards
pass. No new dependency/vendor changes or original C#/sibling FerroUI edits are involved.
The running desktop sample/bundle/window/layout were preserved.

Strict all-target workspace Clippy and release runtime/recycling/Core/backend/UI/
diagnostics/markup/sample library Clippy pass. Full Core ownership, generic/optional/
collection metadata, serialization, arbitrary CLR enum/string/reference/GC and native
platform parity remain incomplete. The completion gate still rejects 222 ported,
464 partial and 1242 pending /1928. Standalone Core/backend/runtime suites pass
(36+6+3), independently of general Dock controls/runtime compiler metadata. Normal
dependency trees confirm the declared Core/runtime/UI/SDK/Domain boundaries. The
backend SDK retains native controls through recycling. Formatting and authored
whitespace checks pass. New files are intent-to-add with no staged contents or commits.
Current logs are recorded in CONTINUATION.md.

## Previous factory API checkpoint

# Latest validation: original factory operations and lazy searches

The locked workspace passes **1,597 Rust tests in 45 suites**, zero failed or ignored.
Native UI376, downstream markup127, diagnostics49, settings33, sample27, ReactiveUI
model474, Core SDK29, backend SDK6 and runtime3. Eight added Core tests exercise actual
collection/window/pin/activation behavior, original command overloads through FactoryBase,
20 virtual overloads through both enclosing factory views, predicate signatures and
variance, and query/cursor metadata. Two independent compiled consumer cases add
source-style predicates and a Core IFactory ActivateWindow command binding; runtime
compiler tests include both assets. Query descriptors retain their source interface
kind, and cursor conversion reaches the existing native IDisposable contract.

Find queries remain deferred and repeatable, with independent cursor state and current
tree reads. Source duplicate nested results are retained. Tests preserve Current after
end/fault/disposal and the original unsupported Reset failure. These checks do not
establish arbitrary CLR iterator identity, general interface variance, cycle collection
or platform parity. Optional source arguments have arity adapters; general optional-
parameter metadata and generic CreateList/GetDockable remain incomplete.

The unchanged C# reference passes **275 assertions**, including method predicates,
compiled command binding, graph mutations, pin order, window initialization/lists/
activation and lazy query/cursor behavior. Window fixtures keep the owner and floating
root separate. An accidental self-containing layout failed recursively in both source
and port and was corrected without modifying the engine. The original C# and sibling
FerroUI checkout remain unchanged. The desktop sample/bundle/window/layout were preserved.

All 18 Python verifier tests, both generators, source/theme/inspector guards and all
five exact vendor integrity guards pass. No new dependencies or vendor edits are part
of this checkpoint. The completion gate still rejects the unfinished source inventory:
222 ported, 464 partial and 1242 pending of 1928 files. Remaining full collection/Core
ownership, CLR/GC/platform and wider source scope stay in the active goal.

Strict all-target workspace Clippy and release runtime/recycling/Core/backend/UI/
markup/sample library Clippy pass. Standalone Core/backend/runtime suites pass (29+6+3)
without general Dock controls or the runtime compiler. Normal dependency trees confirm
Core/runtime exclude native controls, general Dock controls exclude the concrete
backend/SDK, the backend SDK excludes general Dock controls/runtime compiler, and
Domain excludes native base/controls. The backend SDK retains native controls through
recycling. Formatting and authored whitespace checks pass. A structural comparison
finds all 77 nongeneric method name/arity pairs from original IFactory.cs in both
factory descriptors; full parameter/default/generic/type-system parity remains open.
No staged contents, commits or original tracked source changes are present. Current
logs are recorded in CONTINUATION.md.

## Previous locator checkpoint

# Latest validation: Core factory ownership and locators

The locked workspace passes **1,587 Rust tests in 45 suites**, zero failed or ignored.
Native UI376, downstream markup125, diagnostics49, settings33, sample27, ReactiveUI
model474, Core SDK21, backend SDK6 and runtime3. Five existing factory command/helper
tests moved into Core with their owning descriptors. Six new Core cases cover source
delegate/container identity, deferred invocation, all three dictionary APIs/failures,
null versus missing fallbacks, transient virtual getters, invalid delegate signatures
and releasing captured values outside mutable borrows. Three independent compiled
consumer cases cover original method declarations, Core-qualified tracking/TwoWay
bindings and both compiler hosts. A resource-owned source graph retains weakly tracked
models; explicit callback teardown does not establish CLR cycle collection.

The unchanged C# reference passes **236 assertions**, including equivalent locator
assets, original object identity, dictionary failures and the default-host getter
becoming null between its two InitLayout reads. Native nullable locator entries now
preserve that source result. No original C# or sibling FerroUI source changes,
desktop rebuild/restart or live layout changes are part of this checkpoint.

The Core logical assembly owns IFactory and FactoryExtensions; legacy UI aliases
point to the same descriptors. The fifth isolated native dependency patch records
existing checked boxed-cast facts in the shared scanner; runtime conversion predicates
remain in use. Two scanner regressions pass. Full collection/delegate/generic method
metadata, remaining Core ownership, broader CLR/GC/platform behavior and the wider
source inventory remain incomplete. Completion correctly rejects 222 ported,
464 partial and 1242 pending of 1928 files.

Strict all-target workspace Clippy and release runtime/recycling/Core/backend/UI/
markup/sample library Clippy pass. Standalone Core/backend/runtime tests pass (21+6+3)
without general Dock controls or the runtime compiler. Normal dependency trees confirm
Core/runtime exclude native controls, general Dock controls exclude the concrete
backend/SDK, the backend SDK excludes general Dock controls/runtime compiler, and
Domain excludes native base/controls. The backend SDK retains native controls through
its existing recycling integration. Formatting, 18 Python verifier tests, both
generators, source/theme/inspector checks and all five exact vendor guards pass.
New files are intent-to-add for review with no staged contents or commits. Original
source trees are unchanged. Current logs are recorded in CONTINUATION.md.

## Previous event checkpoint

# Latest validation: original factory event metadata

The locked workspace passes **1,578 Rust tests in 45 suites**, zero failed or
ignored. Native UI381, downstream markup122, diagnostics49, settings33, sample27,
ReactiveUI model474, Core SDK10, backend SDK6 and shared runtime3. All 28 source
factory events are exercised through native metadata and the actual engine. Checks
cover original sender/payload identity, explicit disposal, ignored handles, publisher
release, cancellation and context replacement. Constructors/properties and shared
identity are checked for all 29 argument classes and the weak tracking snapshot.
Compiled application consumers construct original roots, cancel an actual close
through an event method and bind all ten original tracking reasons. Both compiler
hosts use the public original value paths.

The unchanged C# reference passes **214 assertions**, including all 28 event sender/
payload/unsubscribe cases, every event-argument constructor, mutable cancellation/
context and equivalent compiled roots/handler/reason binding assets. No original
C# or sibling FerroUI source changes, desktop app rebuild/restart or layout changes
are part of this work. Rust tests read the committed reference corpus without .NET.

The shared object bridge is extracted into ferrodock-runtime, with one representation
and type registry reused by recycling and Core metadata. Existing recycling/control
checks remain green; Core retains no controls dependency. No vendor source changes
were needed. CLR EventArgs base casts, general metadata remove accessors, arbitrary
CLR delegate identity/GC/lifetimes and the wider original inventory remain incomplete.
Factory locators, remaining FactoryBase/collection/generic metadata, other backends,
services, serializers, full Core logical ownership and platform behavior remain in
the active goal. Completion must still reject 222 ported, 464 partial and 1242 pending
of 1928 files.

Strict all-target workspace Clippy, release runtime/recycling/Core/backend/UI/markup/
sample library Clippy, formatting, 18 Python verifier tests, both generators, source/
theme/inspector checks and four exact vendor guards pass. Standalone Core/backend/
runtime tests pass (10+6+3) without general controls or compiler metadata. Normal
dependency trees confirm all declared UI/Core/SDK/runtime/Domain boundaries. The
compiled application probe explicitly clears its retained factory/sender references
and verifies owner/publisher release; it does not claim general CLR cycle collection.
New files are intent-to-add for review; there are no staged contents or commits.
Final logs are recorded in CONTINUATION.md.

## Previous constructor checkpoint

# Latest validation: Factory and DockWindow metadata

The full locked workspace passes **1,567 Rust tests in 43 suites**, zero failed or
ignored. Native UI381, downstream markup118, diagnostics49, settings33, sample27,
ReactiveUI model474, Core SDK6 and backend SDK6. Seven added consumer cases cover
original Factory/DockWindow constructors, source inheritance and class/interface
identity, declared window properties/method metadata, all concrete model creators,
shared mutable registries, layout initialization/close and live compiled TwoWay
bindings. HostWindows Count observes additions/removals on the original list.
Two SDK cases independently cover constructor defaults, erased identity projection,
composed source window overrides and empty instance release.

The unchanged C# reference passes **111 assertions**, including equivalent compiled
factory/window assets and source graph initialization. InitDockWindow does not set
Owner.Window in either implementation. Resource-owned fixtures preserve composition
ownership and do not prove general weak-backlink/CLR GC lifetime equivalence.
Original C# and sibling FerroUI source files remain untouched; the desktop sample,
bundle, window and live layout were preserved.

Strict all-target workspace Clippy and release Core/backend/control/markup/sample
library Clippy pass. All 18 Python verifier tests, both generators, source/theme/
inspector links and four exact vendor guards pass. No vendor code changes were
needed in this checkpoint. Existing native dependency/linker warnings remain.
Formatting and authored non-vendor whitespace checks pass; no staged contents or
original-source changes are present. Both SDKs additionally pass standalone (6+6),
without general controls or compiler metadata. Normal dependency trees confirm
UI excludes concrete backend/SDK, Core excludes controls, backend SDK excludes
general controls/runtime compiler, and Domain excludes native base/controls.

FactoryBase metadata and collection metadata remain partial: full generic methods,
locators, events, dictionary/indexer operations and remaining inherited methods are
still required. Other Core logical descriptor ownership, backends, services,
serializers and broader API/platform/CLR lifetime contracts remain in the full goal.
The structural completion gate must reject 222 ported, 464 partial and 1,242 pending
of 1,928 files. See CONTINUATION.md for current validation logs and next scope.

## Previous Controls checkpoint

# Latest validation: layout/splitter models and shared model values

The final workspace passes **1,558 Rust tests**, zero failed/ignored, **43 suites**.
Native UI381, downstream markup111, diagnostics49, settings33, sample27, ReactiveUI
model unit474, model metadata6 and backend SDK4. Ten new original layout/splitter
constructors bring the backend Controls directory to all 14 classes. New checks cover
actual concrete models, all declared properties/defaults, source inheritance/interface
identity, original ToolDock factory ownership/activation/focus, SplitView methods,
actual pane/content identity/null, live enum/bool/numeric compiled bindings and both
compiler hosts. Empty-instance retain/release checks cover all ten new class views.
The standalone SDK6+4 checks pass without general controls or compiler metadata.

The original unchanged C# assemblies independently pass **93 assertions** using the
equivalent root assets, source defaults, real factory methods, compiled live/TwoWay
bindings, pane identity/null, original Core enum assembly and required/nullable original
policy interface assignments. Rust tests do not need .NET. No original or sibling
source is modified. The running desktop/bundle/layout state is preserved.

The new shared model SDK owns 21 original enum/flags descriptors under logical
FerroDock.Model; settings retains helper aliases and its own source types. Assembly-
qualified static values and original typed enum conversions pass. Normal dependency
checks confirm Core metadata excludes controls, general controls exclude the concrete
backend/SDK, backend SDK excludes general controls/runtime compiler, and domain code
excludes native base/controls. The model SDK adds no UI dependency to Domain.

Nested source model assignment exposed an invalid native coercion to a descriptor
trait. Shared compiler metadata now distinguishes the actual owning trait and actual
shared source handle/constructor value. Class views/foreign contracts use registered
casts. Executable/model/runtime checks include shared original policy overrides and
a valid native multi-handle declaration, preserving ordinary Rust unsizing. The final
workspace, strict all-target Clippy, release Core/backend/control/markup/sample library
Clippy and formatting pass after strengthening that case.
The four retained dependency copies remain base12/markup3/loader5/build1 changed files
with exact provenance, licenses, notices and VCS metadata. All 18 Python tests, both
generators, source/theme/inspector checks and dependency guards pass. Existing native
dependency/linker warnings remain; retained vendor whitespace is unchanged.

Full constructor, metadata, serializer/backend, generic/collection/x:Class/root, CLR
custom identity/string/boxing/GC/lifetime and native platform parity remain incomplete.
Completion correctly rejects **222 ported, 464 partial, 1,242 pending /1,928**.
The full goal remains active. See CONTINUATION.md for final logs and next scope.

## Previous constructor SDK validation

# Latest validation: separate ReactiveUI model metadata and constructors

The refreshed workspace passes **1,547 Rust tests**, zero failed/ignored, **41 suites**.
Native UI 381, downstream markup 103, diagnostics 49, settings 38, sample 27, ReactiveUI
model unit 474 and standalone metadata SDK 2. Eight added tests cover four original
model constructors/properties and logical assembly exports, abstract/inherited classes,
real engine instances and factory ownership/close, fresh Load/name scope completion,
shared model/class identity, source command handles, live compiled notifications and
TwoWay bindings, and model/runtime compiler public return paths. The standalone SDK
checks run without loading or depending on the general control library.

The unchanged original C# assemblies independently pass **69** assertions, including
four equivalent assembly-qualified assets, original source defaults/commands, base
classes, fresh instances, compiled live Title/TwoWay Title/IsOpen and factory owner/close.
No original source or sibling FerroUI checkout is edited. Rust tests require no .NET.

The new native integration library owns the Dock model descriptors once. Applications
register it separately from controls; source sample subclasses inherit its class views.
The general controls and domain retain the intended dependency boundary. A reproduced
build-compiler private-module signature failure is fixed by rendering complete actual
type components through existing public export paths while retaining canonical type
keys. The existing external carrier regression fixtures continue using the same compiler.
Four published dependency copies retain exact provenance: base12, markup3, loader5, and
build1 modified source files, with licenses/notices/VCS data.

Strict all-target workspace Clippy, release control/SDK/markup/sample library Clippy,
formatting and normal dependency boundaries pass. Controls exclude SDK/concrete
backend; SDK excludes general controls/runtime compiler; model code excludes native
base/controls. SDK2 additionally pass alone without compiler metadata. Source/theme/
inspector inventory checks, both generators, 18 Python verifier tests and all four
exact dependency guards pass. Wrong-assembly model lookup is rejected. Authored
non-vendor whitespace checks pass; retained vendor source/patch context is unchanged.
Desktop window/layout state is preserved; dependency/linker warnings remain.

Other concrete constructors, external descriptor logical assembly ownership, complete
x:Class/name/generic/cross-package/CLR/custom identity/string/GC/API/platform/lifetime
contracts remain incomplete. Goal status is ACTIVE. Completion still correctly rejects
**222 ported, 464 partial, 1,242 pending /1,928**. Final logs are in CONTINUATION.md.

## Prior external carrier validation

# Latest validation: external constructor carriers and original Core roots

**1,539 Rust tests pass**, zero failed/ignored, **39 suites**. Native UI 381, downstream
markup 97, diagnostics 49, settings 38, sample 27, ReactiveUI model unit 474. Four new
integration tests cover actual policy/override roots, scope completion, inherited/default
properties, Load/fresh objects, original engine resolver/override precedence/live mutation,
interface identity, both compiler hosts and rejection/explicit registration of a prefix-
named external constructor result. The baseline reproduced invalid invented type names.

The unchanged original C# assemblies independently pass **56** assertions. The new Core
POCO assets and real initialized layout establish the same policy/override/fresh/mutation
outcomes; no source assembly is replaced or modified. Normal Rust tests require no .NET.

Strict all-target workspace Clippy, release UI/markup library Clippy, formatting, 18 Python
verifier tests, both generators, source/theme/inspector links and exact vendor provenance
pass. The shared compiler uses the declared model return type and existing type-path
registry; compiler-only owner metadata prevents carriers being mistaken for models.
Base 12 modified files, markup 3, compiler 5 have exact original/modified/patch hashes and
retain licenses/notices/VCS metadata. The original and sibling sources are untouched.
Desktop sample window/layout state is preserved. Native dependency/linker warnings remain.

All original concrete model constructor exports, logical assembly metadata, x:Class/names,
every generic/cross-package root, arbitrary CLR/identity/string/GC and wider API/platform/
lifetime scope remain incomplete. Final goal metadata unexpectedly reports BLOCKED
without a reason, despite verified progress; no implementation blocker was found and
no blocking update was issued. The exposed tools have no resume status. Completion must reject **222 ported,
464 partial, 1,242 pending / 1,928**. Final logs are listed in CONTINUATION.md.

## Historical plain-root validation

# Latest validation: plain model roots and shared reference identity

**1,535 Rust tests pass**, zero failed/ignored, **39 suites**. Native UI 381,
downstream markup 93, diagnostics 49, settings 38, sample 27 and ReactiveUI model unit 474.
Eight new tests cover a plain declared model root, ordinary Load/fresh engine objects,
model/runtime compiler hosts, interface/inherited-class identity matrices, registry/scope
callback reentry, native projection priority/counts, and ReferenceEqualsMultiConverter.
The root context, name registration, property setters and returned value use ONE instance.

The original unchanged C# assemblies independently pass **50** assertions. A compiled
PlainDocument.axaml asset constructs the original ReactiveUI Document as a non-control
root. Interface/base views preserve identity in scopes and the original converter;
distinct models and equal boxed scalars remain different. Rust tests require no .NET.

Strict all-target workspace Clippy, strict release UI/markup library Clippy, formatting,
18 Python verifier tests, both generators, source/theme/inspector links and exact provenance
checks pass. Enabling the real runtime compiler in the test consumer also validates the
compiler-metadata configuration and exposed/fixed three qualified theme-class paths.
Production dependencies gain no runtime compiler. Native dependency/linker warnings remain.

All three published dependency copies retain exact original hashes/licenses/notices/VCS
metadata and declared diffs: base 10 files, markup 3, compiler 5. Original and sibling source
are untouched. Desktop window/layout state was preserved; this work is verified through
native Headless, the compiled consumer and runtime-host emission.

The full goal stays ACTIVE. Constructor descriptor/public-path metadata, all concrete model
constructor exports, plain-model x:Class/names, every generic/cross-package root variant,
arbitrary identity/string boxing/CLR type/GC and wider source/API/platform/lifetime scope
remain partial. Completion must reject **222 ported, 464 partial, 1,242 pending / 1,928**.
Only the final root-identity logs listed in CONTINUATION.md are authoritative for this turn.

## Historical object namescope validation

# Latest validation: shared object namescopes and compiled model references

**1,527 Rust tests pass**, zero failed/ignored, **39 suites**. Native UI 380,
downstream markup 86, diagnostics 49, settings 38, sample 27, ReactiveUI model unit 474.
Twelve new cases exercise the actual shared namescope, metadata, Reference/ResolveByName,
child fallback/shadowing, null/completion/error ordering, callback reentry and native owner/
plain-object lifetime. A real compiled consumer names a plain application object backed by
the source Document model; Reference retains its identity and a compiled #Document.Title
binding follows mutations of the original engine Document. The fixture adds no native
control base or alternate docking model.

The C# harness uses unchanged original project assemblies and independently asserts
**42** supported source outcomes, including plain Document naming/resolution and scope
identity, duplicate, mixed-type collision, pending synchronous callback and completion.
Normal Rust tests require no .NET. The recorded corpus is not a substitute for native tests.

The shared native namescope/compiler changes have exact provenance for three isolated
published packages (base 10 modified files, markup 3, compiler 2). Their original licenses,
notices and VCS metadata remain. The source inventory, theme/inspector links, both generators,
formatting, 18 Python verifier tests and exact dependency integrity guards pass. Strict
all-target workspace Clippy and release UI/markup library Clippy also pass. Original and sibling sources are untouched.
Desktop sample layout/window state was preserved; this turn's new behavior is checked
through the native Headless and compiled-package boundary.

Standalone plain model XAML roots, complete generated fields/custom/interface/CLR identity,
arbitrary managed GC cycles and wider source/API/lifetime/platform parity remain open.
The full goal remains ACTIVE. Completion rejects **222 ported, 464 partial, 1,242 pending
/ 1,928**. The clean final test log supersedes the artifact conflict caused by overlapping
initial validation; only /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-final-workspace-tests.log is authoritative.

## Historical factory and markup-extension validation

# Latest validation: factory focus API and markup extensions

**1,515 Rust tests pass**, zero failed/ignored, **39 suites**. Native UI 380,
downstream consumer 74, ReactiveUI model unit suite 474, diagnostics 49, sample 27.
All five FactoryExtensions methods use the original engine/model interfaces. Seven
model cases test null receivers, tracked inactive roots, lazy search order, document/
focus casting including Tool, close dispatch and real engine mutation, and live host
lookup. A native static metadata case verifies all exported original method names,
nullable arguments and actual returned object identity.

The new ordinary ferrodock-markup-extension package exports both original classes,
constructors/properties/ProvideValue through shared scanner metadata. Thirteen native
Headless cases include all eight original source tests plus live URI/name/provider,
new loaded object identity, missing loader asset, callback and metadata cases. A separate
compiled consumer checks default/explicit namespaces, positional/property constructors,
zero-argument forms and actual reference identity. Loader Source is read again after
the external BaseUri getter, preserving source replacement and null-after-guard failure.

The unchanged C# project reference asserts 33 supported source outcomes and one explicit
source-only gap: arbitrary plain models can be named in the original INameScope. The
native INameScope accepts native object references only, so plain-model lookup is still
unimplemented and no alternate registry/model is substituted. Complete CLR/inheritance/
custom metadata, additional loaders, callback/GC/thread/platform scenarios remain partial.

Strict all-target workspace Clippy and release model/UI/markup library Clippy, formatting,
18 Python verifier tests, both generators, source markup/theme links, structural inventory
and exact vendor provenance pass. Existing dependency/linker warnings remain. Both native
vendor patches and original/sibling sources are unchanged. The sample's running window
was not rebuilt/restarted; previous native inspector/grip evidence remains authoritative.

Goal metadata is ACTIVE again. Full completion correctly rejects **222 ported, 464 partial,
1,242 pending / 1,928**. No blocking condition prevents further progress. Detailed scope,
logs and the shared namescope/compiler next step are in CONTINUATION.md.

## Historical main inspector/F11 and grip validation

# Latest validation: complete main inspector, F11 and grip correction

**1,494 Rust tests pass**, zero failed/ignored, **37 suites**, after the full inspector,
conditional native template-matching fix and requested grip correction. Diagnostics 49,
downstream markup 61, native UI 379, settings 38 and sample 27. Strict all-target workspace
Clippy and release diagnostics/sample library Clippy pass. Existing dependency/linker
warnings remain. Formatting, 18 Python verifier tests, both generators, original markup/theme
links and exact provenance for both isolated dependencies pass.

Eleven main-inspector cases cover all 26 factory events/source strings, lazy ordinal
filtering/query identity/notifications, factory replacement, full compiled five-tab/
18-template tree and Windows bindings, actual filtered ListBox clear through a native
command button, default/custom F11 lifecycle, canceled close/repeated disposal, native
selection map/adorner/picking and exported overloads/null-factory fallback. The actual
sample startup test verifies DEBUG F9 and F11 through the real composition root.
The unchanged C# harness independently confirms the source event and lifecycle behavior.

Native desktop checks confirm the complete live inspector, correct Tool1 Selected
view, selection highlight, F11 close/reopen, filter input and native close. Populated
Events/filtered-clear evidence is C#/native Headless rather than a desktop event log.
The physical bundle SHA256 is
7262f885d1469c30ebacfafe765648cf0902266602f84a5241b8080e62d7d465.
Screenshots and live state are recorded in CONTINUATION.md.

The original C# Fluent header reproduces a 12px grip in a 600px header. Both linked Rust
themes now deliberately use Auto,*,Auto columns in response to the user's correction.
A native regression asserts the grip receives spare width and grows by 200px as the
window grows by 200px; desktop Tool1/Tool3 render the expanded pattern. This is a
recorded source-layout correction, independent of the FerroUI checked-cast defect.
That separate one-file native markup patch prevents a Tool matching IDock merely
because a conditional interface cast exists. The selected-view regression and native
UI check confirm the actual property view. Original source/sibling files remain intact.

Inventory **222 ported, 455 partial, 1,251 pending / 1,928** remains incomplete. Full
platform/API/lifetime/CLR parity, other model frameworks and serializers remain open.
Goal metadata still reports blocked without a reason; only user `/goal resume` can
restore automatic continuation through the exposed controls. Work itself progressed.

## Historical inspector property-view validation

# Latest validation: original inspector property views

**1,483 Rust tests pass**, zero failed/ignored, **37 suites**. Native UI remains 379;
diagnostics is 38, settings 38, downstream markup 61, sample 27. All 18 original
passive property views are normal library classes with native compiled bindings,
public metadata and an independent consumer. Native inherited construction, actual
model edits/notifications, nested owner/active/layout changes, pinned/hidden list
mutation/replacement, read-only fields, original content identity/null/factory dispatch
and splitter properties are checked. The new fidelity guard checks every original
XAML byte after only namespace/assembly migration; its four verifier tests pass.

The independent scripts/dock/inspector-view-reference compiles all 18 unchanged original
XAML/code-behind pairs with actual original projects and Headless 12.1.3 / .NET
10.0.13. Native tests match every original row/column definition, field type/position,
enabled/read-only state and **24** actual source enum TextBox edits. Names, lowercase,
whitespace, known numeric members, comma combinations, invalid/empty/overflow text
agree. The reference also asserts original title/permission edits and nested-owner
notifications. Three missing domain enum descriptors and standard type-converter
metadata resolve the discovered native text-binding gaps without another SDK patch.
Five missing interface descriptors forward actual original contracts and retain model
identity. The existing unknown numeric plain-enum representation gap stays deferred.

Full strict all-target workspace Clippy, formatting, **16 Python tests**, both
factory/coordinate generators and original-source/theme/provenance/markup guards pass.
After removing redundant test conversions for Clippy, all 38 diagnostics cases pass
again. The completion gate correctly rejects **222 ported, 451 partial, 1,255 pending
/ 1,928**. Current authoritative logs and next steps are in CONTINUATION.md.
No original C#, sibling framework or vendor support patch changed. Existing base,
SourceLink package and compact-unwind warnings remain. The full RootDockDebug/F11
inspector, generic/custom metadata, culture, callback/GC and platform scope remain
unfinished. No new desktop inspector result is claimed; the running sample is still
the previous F9 build, visually checked after unlock for tab/pin/open/unpin and F9.

## Historical F9 diagnostics overlay validation

# Latest validation: original F9 diagnostics overlay

**1,468 Rust tests pass**, zero failed/ignored, **37 suites**. Diagnostics grows from
12 to 25 and the sample from 26 to 27; native UI remains 379 and downstream markup 60.
The new actual-source reference links the unchanged original adorner/helper/manager/
extensions with original native Dock controls. Under Avalonia 12.1.3 / .NET 10.0.13,
it captures **16** render cases. Native tests compare every source rectangle's order,
stroke color/thickness, optional fill and bounds. They also inspect actual glyph output
for white 12-point text and empty/missing context behavior through the shared SDK.

Native Headless checks cover real pointer movement, first marked hit/topmost order,
root exclusion, hidden descendants, strong ownership/removal, repeat-add behavior,
default/custom/handled key events, dynamic DockControl changes, explicit cleanup,
current-only desktop windows and top-level-only single views. Static metadata retains
both public original extension overloads and namespace; the normal shared scanner
exports the library. A dedicated real App startup test proves DEBUG F9 is attached
through the source composition root and actual compiled docking graph.

Independent C# and native tests both exposed the source detached-dock cleanup quirk:
lookup occurs after detach, leaving its old-layer adorner. Source snapshots and manager
cleanup retain that behavior. Routed native tokens remove all matching helper handlers;
CLR layout unsubscribe removes its latest match. Undo publisher handles are weak, but
actual callbacks retain source strong owners. There is no once-only/Drop substitution
for the original ActionDisposable, nor a replacement drawing or docking algorithm.

Strict locked all-target workspace Clippy, strict release diagnostics/sample library
Clippy, formatting, 12 Python tests, both generators and source/theme/provenance guards
pass. A Copy-only Clippy correction is followed by another passing 25-case diagnostics
run. Original C# and sibling FerroUI sources remain untouched. Existing base dependency,
SourceLink package and compact-unwind warnings persist. Full completion correctly
rejects **222 ported, 415 partial, 1,291 pending / 1,928**.

The desktop sample rebuilt and was physically repackaged, with matching SHA256
**e1cf99e392fd3a6112166c59afcb94698c524f23a8fa109f414d02146efaa94a**.
The refreshed native macOS app has been visually verified: F9 enables the outlines,
Document2 can be selected through the overlay, and F9 removes it. It is left running
with Document2 selected and overlay off. Actual screenshots and accessibility state are
listed with absolute paths in CONTINUATION.md. Desktop hover/footer filling is not
separately claimed; those branches have native Headless evidence. Remaining F11 views,
full custom/generic type/culture/metadata, callback/GC and platform scope stay partial.

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-overlay-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-overlay-reference.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-app-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-debug-desktop-build.log

## Historical diagnostics selection and native visual validation

# Latest validation: diagnostics selection and native visual check

**1,454 Rust tests pass**, zero failed/ignored, **37 suites**. The new normal workspace
diagnostics library adds 12 cases; native UI remains 379, downstream markup 60 and
sample 26. The actual unchanged C# selection helper/adorner link with the actual
original factory/control projects in an independent Avalonia 12.1.3 Headless harness.
All **216** visible/pinned/tab map combinations match exact source root/control outputs,
including short-circuit success with a null control. Source attached geometry, live
layout, disconnected target, removal and nested-removal failure assertions also pass.

Native Headless tests exercise actual AdornerLayer ownership/logical parent, same-root
identity, root switches, absent layers, pre-attach highlight retention, native layout
updates, subscription disposal and source strong capture. DrawingGroup output contains
the exact yellow two-pixel pen, no fill and source rectangle geometry. Plain property
metadata and custom/native base/root class projection retain actual object identity.
The hidden marker projection adds no UI dependency to the model. Removal retains the
source attached value and orphan-in-old-layer behavior; callback reentry raises the
source cleared-field failure instead of silently continuing with a stale snapshot.

The user unlocked the Mac; the refreshed Rust sample is now visually verified and left
running. The physical bundle matches the rebuilt desktop executable (SHA256
f5e4460b009711b1ba36b0bcffde2e32e0bf162c4bda46aa74cedcf82f19a6af).
Native UI actions verified corrected tab headers, Document2 selection, Tool2 selection,
left pinning, opening the pinned host, unpinning and selecting Document1 again. Final
layout retains all three documents/eight tools, no pinned items and Tool2 selected in
its original tool group. Actual screenshots/accessibility state live under the current
system-volume artifact directory, named ferrodock-rust-running.png,
ferrodock-rust-pinned.png and ferrodock-rust-running.ax.txt. CONTINUATION.md records
absolute paths and the observed app process. The prior visual-refresh blocker is clear.

This is recorded native-flow evidence on this macOS machine. Diagnostics inspector
views, debug overlays/hotkey integration and broader input/platform/metadata/callback/
collection/exception/GC/thread/process parity remain partial. The inventory is **222
ported, 411 partial, 1,295 pending / 1,928**; full completion remains rejected.
The original SourceLink harness build needed its existing NuGet cache path supplied
with a trailing slash through a global build property. Original source/build files were
not edited. Existing source dependency warnings, 23 native base warnings and the
compact-unwind linker warning remain.

Strict locked all-target workspace Clippy and strict release library checks for
diagnostics, UI, model and the sample pass. Formatting, 12 Python verifier tests,
both generators and native/source/theme integrity guards pass. Workspace whitespace
checks include new files and exclude the exact vendor copy/literal patch. A final
needless-borrow correction is covered by another passing 12-case diagnostics run;
the assembly test also checks both exact XML namespace mappings and registration
identity. No original source file outside FerroDock changed. The completion gate
correctly exits 1 for remaining partial/pending source scope.

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-overlay-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-overlay-reference.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-desktop-build.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-selection-final-clippy.log

## Historical Browser overlay and delayed panel validation

# Latest validation: Browser theme and delayed tab panel

**1,442 Rust tests pass**, zero failed/ignored, in **35 workspace suites**. Native
UI contributes 379, downstream markup 60, Browser 5, Fluent 4, Simple 7 and the
original sample 26. The full locked workspace run includes the actual original
engine/model/factory graph, native pointer tab selection, Browser selector states
and multi-tool floating-host composition. All five Browser source documents compile;
the compact dictionary remains empty and the original Fluent dependency is retained.
Live resource bindings validate the actual light/dark palettes and native typed
platform options. No replacement docking/layout/timer engine is introduced.

The original DelayedUniformTabPanel's six properties, measure/arrange calculations,
viewport precedence, delayed expansion, repeated-close restart, transitions and
observer teardown run through shared native APIs. A new source-only .NET harness
links the unchanged C# control and compatibility extension under Avalonia 12.1.3.
Its 24 captured layouts compare desired extent and every child's position/width.
The committed fixture uses '-' for empty arranged-child lists. Native tests also
exercise signed/nonpositive delay, NaN/signed-zero/rounding boundaries, hidden and
minimum-width children, real callback collection mutation, and inherited timer
construction. All three theme classes now populate their resources through the
native inherited constructed virtual; dedicated derived-class tests pass.

Strict locked all-target workspace Clippy and strict release library Clippy for UI,
Fluent, Simple, Browser and the sample pass. The structural inventory, source and
theme-link integrity guards, both generators and 12 Python verifier tests pass.
The original C# and sibling FerroUI sources remain unchanged. Existing 23 native
base warnings and compact-unwind linker warnings remain. Debug/test incremental
caching is disabled after its generated caches exhausted the SSD. A cleanup/rebuild
overlap failed once on a missing object; after cleanup completed, the fresh full
workspace run above passed.

Inventory: **221 ported, 408 partial, 1,299 pending / 1,928**. The completion gate
correctly rejects the unfinished port. Actual animation-frame cadence, attached DPI,
full generic metadata, native callback/collection/exception/CLR GC parity and broader
input/platform scenarios remain partial. Source strong timer callbacks stay strong;
native stopped-timer Rc cycles are recorded rather than silently weakened. The final
sample GUI refresh still awaits the previously requested manual Mac unlock.

After the serialization-only empty-list fixture adjustment, the actual .NET generator
and targeted native 24-layout corpus test pass again. Formatting and workspace
whitespace checks pass, excluding the exact vendor copy/literal patch. The desktop
sample rebuilt and its system-volume app bundle was physically updated using the
normal packaging helper. Its executable matches the build byte-for-byte (SHA256
9b1467f87ed04b76324dec90bdecd08ccf6d8d7bd1b6f6925743aa3ab0b9cc95).
The old app process has not been restarted; no refreshed visual verification is claimed.

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-browser-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-browser-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-browser-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-delayed-reference.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-delayed-final-corpus.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-browser-desktop-build.log

## Historical Simple theme validation

# Latest validation: original Simple theme library

**1,421 Rust tests pass**, zero failed/ignored, 33 workspace suites. The new Simple
library contributes six cases and its independent UI consumer three (downstream UI
markup 57 total). Native UI 368, sample 26, settings 37 and settings markup 2 remain
green. The later targeted color matrix also passes: exact sidebar/editor colors for
all five presets in both native Simple light/dark variants, including Default aliases.

The original Simple root preserves all 42 merged dictionaries in order; all 56
class/resource documents compile, including the original linked Controls group.
Tests cover the nine original normal/compact scalar expectations, resource-interface
lookup/invalidation and equal-value no-ops, source cached style identity/lifecycle,
manager shared lists/guards/default-owner includes, real surface updates and nested
VisualBrush grip clearing for both dark presets. Independent compiled markup creates
the public class/properties and actual title-bar/engine root/split/document/tool graph,
retains cached document containers through selection and updates inline pinned
alignment. All five original images open from the Simple assembly. Its normal Cargo
dependency graph includes neither Fluent theme library.

Strict all-target Clippy, formatting, structural inventory, native patch guard and
TWELVE Python verifier tests pass. The linked-resource guard checks all original
shared Control/Asset bytes, permitting only the owning URI migration; its tests reject
missing/extra/changed files and wrong owners. Existing dependency and compact-unwind
warnings persist. Full source API, manager/base class metadata, callback/enumeration/
GC/thread/process and broader native input/platform parity remain partial.
Inventory: **221 ported, 397 partial, 1,310 pending / 1,928**. Full completion must reject.
The sample visual refresh still awaits a manual Mac unlock; it is not claimed verified.

Strict release checking of the new Simple library passes. Both source generators,
unchanged-original-source and workspace whitespace checks pass too (the latter excludes
the exact vendor copy/literal patch). Full completion was rerun and correctly rejects
remaining source scope. Release log: /Volumes/SSD/temp/ferrodock-port/ferrodock-simple-release-clippy.log.

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-simple-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-simple-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-simple-color-matrix.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-simple-clippy.log

## Historical docking-state metadata validation

# Latest validation: original docking-state metadata

**1,412 Rust tests pass**, zero failed/ignored, 31 suites. Settings adds one case
(37 total), and its independent compiled consumer adds one (2 total); native UI 368,
downstream UI markup 54, sample 26 and Fluent manager 3 remain green. The new checks
compare generated build/runtime metadata, six original names/values/flags marker and
actual domain handles. Runtime conversion and nullable assignment round-trip all 98
independent source .NET cases, including signed unknown bits; out-of-i32 integers and
unrelated values are rejected. Independently compiled resources exercise x:Static,
named and composite literals, zero, all known bits and known numeric combinations.
There is no UI dependency in the model, duplicate enum or custom text parser.

The native typed flags emitter still requires values made from declared members;
compiled unknown bits and complete inherited System.Enum APIs remain pending.
Inventory stays **221 ported, 383 partial, 1,324 pending / 1,928**; the full completion
gate must reject. Existing native dependency/compiler and compact-unwind warnings
remain as recorded below. NINE Python verifier tests and formatting pass.

A new source-only reference compiles the actual original ProportionalStackPanelView,
its code-behind and controls under Avalonia 12.1.3 Headless. Construction throws the
original NullReferenceException (initial stack: markup line 50, null ItemsSource).
The reference asserts this contract. Native compilation of the unchanged structure
fails earlier at undeclared IList.Add. That view remains pending; no altered collection
or Rust view was included. Mac app control still reports a locked screen, so the final
visual refresh remains pending a manual unlock.

Strict all-target workspace Clippy and strict release library checks for settings,
native UI, Fluent and the sample pass. Formatting, both source generators, NINE Python
checks, unchanged-original-source and provenance checks pass; workspace whitespace
checks exclude the exact vendor copy/literal patch. Existing native warnings persist.
Strict log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-clippy.log.
Release log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-release-clippy.log.

The actual desktop executable was rebuilt and physically repackaged; the bundle
matches the new executable byte-for-byte. Visual refresh remains pending the manual
unlock. Desktop build log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-desktop-build.log.

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-metadata-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-original-proportional-view.log

## Historical original application validation

# Latest validation: original Rust application and native sample

The locked workspace passes **1,410 Rust tests**, zero failed/ignored, in 31 unit/doc/
binary-test suites: sample 26, native UI 368, downstream markup 54 and Fluent manager 3.
The earlier 1,399-case checkpoint is historical. New checks validate the real App's
compiled resources and full graph, all five preset choices, actual ReactiveCommands,
desktop and single-view startup, window Closing/Exit layout cleanup, same-list identity
and observable updates, source static-manager capture and nullable template conversions.
Both document/tool icon presenters retain explicit empty templates and have no full
body views inside tab strips. Original metadata/engine/model/view checks still pass.

Shared preset cases cover validation order, no-app and invalid/no-op cases, casing,
unknown-source fallback and first match, recursive custom/default resource owners and
Styles-only traversal, include identity and unrelated resource retention. String helper
cases reuse 3,005 .NET ordinal inputs. Flags Display matches 98 independent outputs from
the actual unchanged source enum. The dedicated harness records .NET 10.0.13; ordinary
Rust tests consume committed fixtures. Full flags/System.Enum APIs remain incomplete.

Strict workspace all-target Clippy, formatting, structural/source inventory, native
patch provenance and NINE Python verifier tests pass. Inventory: **221 ported, 383
partial, 1,324 pending / 1,928**. Full completion correctly rejects. The local native
base dependency emits 23 preexisting warnings; native compact-unwind warnings persist.

A native macOS FerroDock Demo window was launched with a physical executable in a
system-volume app bundle, showing the original full layout. Direct terminal launch
failed in the native render timer (-6661); a symlink bundle stalled before main.
Neither failed mode is treated as evidence of a known root cause. Native tab header
fallback discovered in that launch was fixed and covered in Headless before packaging
the refreshed binary. This does not establish broad input/platform/lifetime parity.
The original DEBUG diagnostics and other full-port gaps remain open.

Strict release checks for native UI/Fluent/sample libraries and both source generators
also pass. Workspace whitespace checks pass excluding the exact vendor copy/literal
patch, whose original whitespace is intentionally preserved and provenance-checked.
The physical bundled executable matches the desktop build byte-for-byte. The Mac locked
before the refreshed window could be inspected; native app control requires a manual
unlock. The initial window was observed, while the corrected headers currently have
Headless regression evidence and a packaged build, pending the final visual check.


Authoritative current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-app-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-app-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-app-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-app-desktop-build.log

## Historical compiled sample view validation

# Latest validation: compiled original sample views and native model metadata

The full locked workspace passes **1,399 Rust tests**, zero failed/ignored, in 30
unit/doc-test suites (sample 20; native UI 368; downstream markup 54). Strict all-target
Clippy and strict release checks for sample/native UI/Fluent libraries, fmt, structural
inventory, factory/coordinate generators, SEVEN Python checks, source-name/placeholder,
whitespace and unchanged-original-source checks pass. Inventory: **222 ported, 376
partial, 1,330 pending / 1,928**. Completion remains rejected; the existing macOS
compact-unwind warning persists.

Ten new sample cases validate original model-class metadata/inheritance and actual
owner identity, live/two-way source bindings, original Dashboard/Home views and real
navigation, raw context identity, actual main-model commands, source static option-array
semantics, all document/eight-tool views and their docking permission/nullable enum/flags
selectors, exact ViewLocator factories and original missing/error behavior, plus the
ACTUAL MainWindow, original full docking graph/document views and root reset/rebinding.
The recycling bridge also adds a non-Clone mutable-value identity/lifetime case.

Fourteen original view classes/resources compile. This is a library with Headless
composition evidence; **there is no desktop executable or launched Rust sample yet**.
MainView theme behavior, actual App resources/lifetimes/Program startup and faithful
preset managers/shared native dictionary API support remain required. Full generic
ReactiveUI metadata and broader lifecycle/platform/GC parity remain open. See the
latest continuation record for exact constraints, source-generator evidence and logs.

## Historical sample model/factory validation

# Latest validation: original ReactiveUI sample model/factory library

The full locked workspace passes **1,388 Rust tests**, zero failed/ignored, in 30
unit/doc-test suites. The new normal-workspace original sample library contributes ten
cases: full three-document/eight-tool layout and source permissions, context/locator
identity and freshness, real create/activate/focus and nullable/no-factory edges, late
native/managed host choice, custom floating docks/window title, original startup/reset/
close behavior, global tracking status, reactive sender/interface identity and actual
derived native host class names.

Strict all-target Clippy and strict release checks for sample/native UI/Fluent theme,
fmt, structural inventory, source-name/whitespace/original-source checks, factory and
coordinate generators pass. The verifier now covers Rust sample units too; all SEVEN
Python tests pass, including missing mapping/name/placeholder rejection in a sample file.
Inventory: **222 ported, 345 partial, 1,361 pending / 1,928**. Completion stays rejected.
The existing macOS compact-unwind linker warning persists.

The original sample library now has its actual model classes, factory, main view model
and commands, using the already-ported engine. It has no executable or compiled sample
views/startup yet. Exact preset-manager porting also needs shared safe plain-provider
identity/type recovery and mutable merged-dictionary interface support. The actual native
APIs were inspected; no unsafe or reduced preset algorithm was introduced. See the
current continuation checkpoint for evidence and remaining work.

## Historical complete Fluent theme validation

# Latest validation: complete Fluent theme and original window resources

The full locked workspace passes **1,378 Rust tests**, zero failed/ignored, across all
28 test/doc-test suites (UI 368; downstream markup 54). Nine new cases cover native
window metadata, complete theme/density/resource notifications/cache style identity,
full titlebar/automation, integer conversion, original MDI button transitions, runtime
root/preset resource loading, live HostWindow.Topmost binding and actual cached document
container identity/visibility while the original engine switches tabs.

All four remaining source window dictionaries, actual DockFluentTheme class/root and
density enum now compile. The full root retains all 50 original merged dictionaries,
CompactStyles and managed chrome override. Separate preset compilation and the composite
loader preserve all public resources; all bindings use compiled metadata. The shared
compiler still embeds the original assets and exact =0.3.0 dependencies remain unchanged.

Strict all-target Clippy, strict release checks for both native UI and Fluent theme
libraries, formatting, structural inventory, factory/coordinate generators, six Python
verifier tests and source-name/whitespace/original-source checks pass. The existing
macOS compact-unwind linker warning persists.

Inventory: **212 ported, 329 partial, 1,387 pending / 1,928**. Completion remains rejected.
The original application/bootstrap is still absent; no runnable Rust sample is claimed.
Full source/platform/input and identity/lifetime parity remain unverified beyond the
recorded evidence. See CONTINUATION.md for the current source correspondence and next work.

## Historical pinned/layout validation

# Latest validation: actual pinned host and original dock-layout graph

The full locked workspace passes **1,369 Rust tests**, zero failed/ignored, across all
28 test/doc-test suites (UI 367; downstream markup 46). Twenty-five additions cover
actual source pinned control restore/drag/override/inline/minima/custom-thumb/provider/
window rules, all four original fixed-size panel assertion sets, converter roundtrips/
null/passthrough, native contract forwarding/content identity and three real engine/UI
resource graph scenarios. Original pinned replacement/alignment rebuild also passes.

The actual native PinnedDockControl, PinnedDockHostPanel and peer compile. Twelve complete
source dictionaries compile with all nine layout/content interface descriptors and the
original model projections/notifiers. The downstream consumer loads 46 available source
resources in original order, creates real DockControl/default templates and original
factory/root/proportional/document/tool models, and verifies source container realization,
engine-driven live selection and original per-tool pinned sizing/replacement. No replacement
docking algorithm or production theme/control fixture was introduced.

Strict all-target and release UI-library Clippy, formatting, structural inventory,
factory/captured-coordinate generators, six Python verifier tests and source-name/
whitespace/original-source checks pass. Published dependencies remain =0.3.0, and the
original C# is unchanged. The existing macOS compact-unwind warning remains.

Inventory: **210 ported, 320 partial, 1,398 pending / 1,928**. The completion gate rejects
unfinished scope. Four window dictionaries, DockFluentTheme/root/density/cache API and
original application/bootstrap remain absent; no runnable Rust sample is claimed.
General original lifecycle/input/desktop/alternate-model scenarios still need validation.
The borrowed-slice converter API cannot preserve EitherNotNull's original all-null input
IList identity; its list-valued fallback currently copies values, explicitly partial.
Recorded box/native/model reference tests do not prove arbitrary string/interface boxing,
CLR GC/delegate/CWT or all generic/runtime API behavior.

## Historical Fluent and native checkpoints

# Latest validation: original Fluent scroll, document and tool/chrome resources

**1,344 Rust tests pass**, zero failed/ignored, in the full locked workspace (UI 345;
downstream markup 43). Fifteen additions validate three source scroll visibility/number/
direction matrices, native line command directions/receiver/event/no-op behavior, five
converter mappings/identity cases, four original document empty-content scenarios,
original tool chrome/deferred content realization and actual compiled scroll-button
command/viewer/offset bindings. Existing tab/pinned templates now use the original custom
ScrollViewer dictionary in the expanded source resource graph.

Six complete source dictionaries and eight source code units (command/enum plus seven
converters) compile with original models/native controls and the shared compiler. Data
and control-theme binding scopes are explicit; original trees/styles/commands remain.
Implicit document/tool theme lookup and actual source empty content templates are tested.
The Fluent crate directly references the already-ported deferred-content crate used by
its original templates. Published dependencies remain =0.3.0; no sibling checkout changed.

Strict all-target and release UI-library Clippy, formatting, structural inventory,
factory/captured-coordinate generators, six Python verifier tests, source-name and
whitespace checks pass. Generated Fluent markup alone additionally allows the shared
0.3.0 emitter's repeated nullable command setter branches; handwritten lints stay strict.
The existing macOS compact-unwind linker warning remains.

Inventory: **210 ported, 295 partial, 1,423 pending / 1,928**. Full completion remains
rejected. The complete DockFluentTheme/root/resource graph, actual PinnedDockControl,
original Rust application startup and broader source input/lifetime/platform scenarios
are still pending. The source alternate framework-property model fixture is not supplied
by using ReactiveUI models. ReferenceEquals tests prove recorded box/native/model identity;
arbitrary reboxed strings/custom Rc/interface identities still need shared representation
work, and no content-equality fallback is claimed. No runnable Rust sample exists yet.

The historical checkpoints below describe earlier scopes/counts and are superseded here.

## Earlier native stack validation

# Latest validation: compiled native host, tab, pinned and tracking stack

The locked full workspace passes **1,329 Rust tests**, zero failed/ignored, including
336 UI tests and 37 downstream markup tests. Twenty-one new tests cover native host
methods/geometry/reopen/move cancellation/release, interface/context/factory identity,
factory ownership transfer, tab state/providers, original pinned bounds cases, all three
tracking maps/context/detach, document list replacement/detach, document/tool host peer
activation, converter boundaries and compiled markup/property/template construction.

Actual source controls/hosts/state/adorner/preview/factory helpers are now ordinary Cargo
modules. All 26 staged duplicates were removed. Native document/tool tabs, DockableControl,
DocumentControl/ToolControl, their peers and three source converters compile. Seven
complete source tab/pinned/chrome-resource dictionaries compile with the shared model
XAML compiler; a downstream Headless consumer realizes real source tab and pinned item
containers and tracking panels, then checks original factory registration cleanup.
These are original resources and engine calls, not replacement theme/control fixtures.

Full source Fluent theme and original application remain absent. There is no runnable
Rust sample yet. The custom scroll/chrome/content/window/dock theme graph, broader
original input/capture/drop/adorner/close/reentry/accessibility scenarios, alternate
models/serializers and desktop visual/platform behavior are not validated. Callback/
delegate/native-interface weak lifetime and CLR GC/ephemeron differences remain open.
Generic tab peers use closed native macro specializations with typed strong owners;
CLR generic runtime naming and full ownership/provider parity are not established.

Structural inventory: **210 ported, 280 partial, 1,438 pending / 1,928**. Compilation and
subset tests do not promote these source units to complete. The completion gate remains
red as required. Original C# and published =0.3.0 dependencies remain unchanged. The
existing macOS compact-unwind linker warning persists.

Final checks pass: locked workspace tests; strict all-target Clippy; strict release
UI-library Clippy; cargo formatting; structural inventory; factory and captured-coordinate
generator checks; six Python verifier tests; source-name scan and whitespace checks.
The completion check fails solely because partial/pending sources remain, as expected.
See CONTINUATION.md for the current next steps. Everything under the historical heading
below describes earlier checkpoints and is superseded where it mentions staging or counts.

## Historical validation checkpoints

# Latest validation: native host, chrome and pinned source dependency group

The locked workspace passes **1,308 Rust tests**, zero failed/ignored, including 317 in the
UI crate. Strict all-target and release UI-library Clippy, cargo formatting, syntax-only
formatting of all 26 staged Rust files, inventory, factory/coordinate generators and six
Python verifier tests pass. Inventory: **210 ported, 253 partial, 1,465 pending / 1,928**.
The full completion gate correctly rejects the unfinished port. Rule-2 source search has
no matches. Original C# and published =0.3.0 dependencies remain unchanged; the existing
macOS target unwind warning remains. Only FerroDock files changed.

The one new compiled test exercises the domain IHostWindowCloseState native-receiver bridge:
plain receivers have no native cast by default, interface views retain the original object,
read its current closed flag, and compare reference identity rather than closed values.
This supplies the property-value equality/receiver hook required by native class interface
registration. It does not instantiate the staged HostWindow or establish its interface
adapter/canonical/weak lifetime behavior.

The following are full initial source translations **outside Cargo**. They are only syntax-
formatted, uncompiled, and behaviorally unvalidated; no new runtime member is counted complete:

- HostWindow: actual native Window class, three source properties, shared-options/state
  construction, strong constructor handlers and host/state edges, source grip/close-button
  subscriptions, pointer/native move/release/cancel, original geometry/magnetism and model
  notifications, cancelable close/preserve-layout cleanup, taskbar/modal/owner presentation,
  normal bounds, title/layout/activation and native host/close-state interface factories.
- HostWindowState/WindowDragContext: every field/default, source validation/adorners/drop,
  all Process phases and diagnostics. Focused dockable validates; active dockable executes.
  Native context initially has NONE action and Start/End set MOVE; managed context has its
  original MOVE initializer. WindowChrome searches only external surfaces. Screen sums,
  threshold input, float fallback and repeated field reads retain original source logic.
- ToolChromeControl: original properties (including TitleProprty spelling and null title
  default), part/event/template/lifecycle ordering, focus routing, full source window-drag
  helper use, menu lifetime, maximize and pseudo classes. Ordinary-window attach remains
  disabled in source; no attach deduplication or detach floating/part reset is invented.
- ToolPinItemControl and ToolPinnedControl: original helper callbacks/start/move engine
  path, orientation/menu/style/peer, actual native container create/prepare/recycle checks.
  Pin source does not capture the pointer; no new live orientation propagation is added.
- Host/chrome/pin-item/pinned-container peers: exact original role/name/ID/help and invoke,
  flyout and selection operations, live item-index loops and readonly strong source owners.
- Remaining DockHelpers and ManagedWindowLayer.TryGetLayer continuations: source visual
  mode/owner resolution, external drop/target hit priority, eager native-window snapshot
  plus deferred/repeatable stable control ordering and exact dock-first/root layer lookup.

The original current-runtime host branch is translated; version-11-only titlebar code is
not included here. Public Rust overload suffixes distinguish host SetPosition/SetWindowState/
SetTitle from inherited UI setters, while metadata uses original method names. Native out-
parameter metadata and all generated class/property/constructor invokers still need tests.
Button subscription metadata keeps button keys weak and handlers keep the source host
strong, avoiding a new host-to-button ownership edge. Real duplicate/remove-last/reentry
and disposal behavior, native casts/weak lifetime, source CLR GC/cycles and shared-runtime
owner/parent/process-scope behavior remain unverified.

Source Z-order returns concrete DockControl instances, so the staged main capture-transfer
caller now consumes native Ref<DockControl> directly. The initial helper snapshots windows
when called, retains the original list, recomputes/sorts control keys per enumeration, and
maps sorted entries lazily without a second output collection. No model tree is mirrored.
TopLevel::get_top_level spellings now use the actual optional native visual argument in
staged consumers. DragAction flags use uppercase native names throughout staging. These
syntax/API repairs do not establish type-checking or behavior.

Next integrate real document/tool tab strips/items and their peers, register/compile the
whole native dependency group, repair API/ownership/reentry errors, and port the original
full UI/host/state/factory/drag/accessibility scenarios. Full source themes and a runnable
original Rust sample remain required. Goal stays active.

The preceding pass remains relevant below.

# Latest validation: main controller source and native receiver integration

The locked workspace run passes **1,307 Rust tests**, zero failed/ignored, including 317 in
the native UI crate (`test result: ok. 317 passed; 0 failed; 0 ignored; 0 measured; 0 filtered
out`). All-target and release UI-library strict Clippy, formatting, inventory, factory and
coordinate-body generators and six Python verifier tests pass. Inventory: **210 ported,
244 partial, 1,474 pending / 1,928**. The complete-port gate correctly rejects unfinished
work. Rule-2 search in all src Rust files returns no matches. Original C# and published
=0.3.0 dependencies remain unchanged; existing target unwind warning remains.

Nine new tests:

- Five test extracted original DockControl static methods: gesture modifiers (including
  unknown bits and already-required Shift), Alt/Shift/Control drag precedence, selector
  hidden/pinned/windows/visible/split traversal order, same-title distinct model identities,
  duplicate/cyclic graph suppression, ignored PinnedDock content and strong output/visited
  retention. These are original model objects; no substitute tree or fake controller.
- The comparison test matches 21 .NET 10 OrdinalIgnoreCase pairs. Supplementary scalars sort
  after BMP scalars in this source comparer; a UTF-16 ordinal sort would be wrong. The native
  continuation uses the existing captured simple-uppercase map, without full-case expansion
  or comparer allocations. This is expression evidence only; full selector/older runtime/
  unpaired-surrogate behavior remains unverified. Rust strings cannot represent unpaired
  UTF-16 surrogates. Normal tests use the committed fixture without .NET.
- One Headless test verifies independent native selector interface handles compare the
  actual receiver, round-trip through shared ValueTypes/Control values, share original state
  and retain the native control until the final interface is released. Plain service
  metadata dispatch still passes. This does not instantiate the staged main DockControl.
- Three diagnostic-event tests cover original optional model identity, duplicate ordered
  subscriptions, shared event handle, snapshot mutation/reentry, exception short-circuit
  and strong callback captures until explicit removal. Existing shared System Event handles
  multicast behavior. Native event scope is UI-thread-local, unlike CLR process-wide state;
  cross-thread/event/delegate identity, metadata and GC parity remain open.

The full initial DockControl class, WindowDragHelper instance body, external-surface contract
and source automation peer
are **staged outside Cargo, syntax-formatted only, uncompiled and behaviorally unvalidated**.
The staged peer retains its original readonly owner strongly; native base-owner weakness
does not replace that source field. No class/peer/surface member is counted complete.
Initial main class includes the original
engine/options/state constructor, all ten styled properties, template/lifecycle/commandbar/
factory cleanup/default callbacks, managed registration, activation/selector/key handling,
external surfaces, window events and all pointer/capture branches. Original source-bound
callbacks retain their receiver strongly; weak metadata identifies ownership without
invoking callbacks. Constructor/event/state ownership and native Rc/CLR GC cycles remain
explicit. HostWindowFactory's callback property wrapper/markup conversion and arbitrary
managed delegate target/method equality require further integration work.

Staged native external weak references resolve the native receiver before querying its
real registered interface, so a transient adapter is not the tracked object. Plain
contract weak references retain their source Rc semantics. Enumeration pruning is lazy
on first MoveNext and tracks source list version changes; original callback/getter reentry,
class/interface registration, lifetime and cancellation behavior still need real-control
integration tests. Source native/managed host/state/adorner/preview and tab/pinned classes,
complete themes and the Rust sample remain required. There is still no runnable sample.

The preceding pass remains relevant below.

# Latest validation: drag-context transfer and controller/state source group

The locked workspace run passes **1,298 Rust tests**, zero failed/ignored (308 in the UI
crate). Strict locked all-target and release UI-library Clippy, formatting, inventory,
factory generation and six Python verifier tests pass. Inventory: 210 ported, 240 partial,
1,478 pending of 1,928 source inputs. The full completion gate remains red. Original C# and
published =0.3.0 dependencies remain unchanged; the existing macOS unwind warning remains.

Four new tests cover the actual compiled DockDragContext and extracted numeric expressions.
The context retains exact native control/visual references, every copied field, safe self-
copy and source cached DragOffset across Start/End. Detached controls remain alive through
context transfer until explicit End. Numeric continuations match 17 .NET 10 input cases for
Math.Round-to-int32 and Math.Max-zero, including ties, overflow saturation, NaNs/infinities
and signed-zero bits. The probe checks the original expressions, not the full state class
or older CLR behavior. Normal Rust tests read the committed fixture without .NET.

The initial FULL DockControlState body is now staged, including preview placement/activation,
ownership/capture transfer, operation resolution/cache, local/global validation/drop/float,
Execute and all Process/debug branches. The offset calculator interface/default and complete
factory lifecycle service/interface source bodies are staged alongside it. They are formatted
only, UNCOMPILED and UNVALIDATED; passing totals do not establish their behavior, and no full
state/service member is marked complete. Source drag-context/numeric boundaries are the only
new compiled state members.

Factory service staging retains shared recycling, exact duplicate-key rejection, same-layout
protection, all eight cache-prune calls, source graph traversal and receiver-owned default
callback transfer/removal. Add-only visited references use shared System Dictionary insertion
slots. Callback owner identity must be supplied by actual DockControl without invoking
callbacks or comparing their results; its representation is still pending. Weak-key/strong-
value recycling cannot reproduce CLR ephemeron cycle collection. Strong source callback and
host/state references are retained.

Actual DockControl, full native HostWindow/State/window-drag, tab/pinned controls and required
lookups still precede compilation of this group. Original routed state/capture/owner-handoff,
public preview/adorner/host/factory cleanup scenarios, full source themes and runnable Rust
sample remain pending. The prior root probe remains valid: both pinned C# and native Headless
roots are TopLevelHost, with the same original direct-cast fallback. Keep that behavior.
Previously recorded interface/delegate/collection/binding/runtime gaps remain open.

The older entries below describe their respective intermediate validation points.

# Latest validation: native drag-preview mechanics and original coordinate reference

The locked workspace run passes **1,294 Rust tests**, zero failed/ignored (304 in the UI
crate). Strict locked all-target and release UI-library Clippy, formatting, inventory,
factory generation and six Python verifier tests pass. Inventory: 210 ported, 235 partial,
1,483 pending of 1,928 inputs. The complete-port gate remains red. Original C# and published
=0.3.0 dependencies are unchanged; the existing nonfatal macOS unwind warning remains.

Seventeen new tests validate the compiled original DragPreviewHelper mechanics. Both source
live-content visual-brush cases and the abandoned-freeze assertion set run on actual native
controls and production ReactiveUI counterparts. Additional cases cover original size/owner/
managed-bounds fallbacks, opacity (including signed zero), original template order and partial
add failure/retry, floating continuation/model identity/reuse, custom-theme premeasure and
one-axis preferred-size behavior, native overlays/chrome overage/private managed build,
macOS coalescing/duplicate/session/sequence behavior, reentrant pending overwrite, real zero-
size four-retry abandonment, actual native DPI changes and managed-first Hide retention of
an existing floating window. Test-only custom templates are not production theme substitutes.

A separate .NET probe compiles the EXACT original GetManagedPosition body plus the original
VisualRootCompatExtensions against pinned Avalonia 12.1.3 Headless. The method's layer
parameter uses a Visual alias because it reads only base Visual APIs; an actual Border
provides the coordinate surface. Detached, attached render scales 1/2 and hidden cases
produce the same root names/scales/(0,0) results in the native four-case regression. Both
original and Rust Headless roots are TopLevelHost. The direct TopLevel cast fallback is
observed SOURCE behavior, not an established Rust-specific difference. Preserve it. The
probe generator check passes; it does not compile the full helper or prove desktop/public
managed host lookup/positive TopLevel-root behavior. No new reflection is used to invoke
private methods. See scripts/dock/preview-root-reference for exact reproduction and limitations.

Original Show's floating continuation is extracted into compiled show_floating so its body
can be tested. Public Show and TryGetManagedLayer are staged outside Cargo until actual
DockControl/layer host lookup exists; no fabricated fallback or marker control replaces
them. All other source helper methods and native static defaults are compiled. UI-thread
Rc/TLS and reentry-safe Cells/RefCells retain source strong state/explicit cleanup but do not
establish process-wide Monitor/cross-thread/CLR GC parity. Debugger output uses native debug
stderr; ambient-culture transport/unknown runtime cases remain open. The original preview
entry point, host/state/adorner/drag integration, full themes/tabs/pinned controls and runnable
Rust sample remain pending. Previously recorded binding/collection/lifetime gaps remain open.

The older entries below describe their respective intermediate validation points.

# Latest validation: target queries and host-state source group

The locked workspace run passes **1,277 Rust tests**, zero failed/ignored (287 in the UI
crate). Strict locked all-target Clippy, release UI-library Clippy, formatting, inventory,
factory generation and six Python verifier tests pass. Inventory: 210 ported, 234 partial
and 1,484 pending of 1,928 source inputs. Complete-port validation still fails. Original C#,
published =0.3.0 dependencies and the existing nonfatal macOS unwind warning are unchanged.

Eight new tests verify the native casts used by the original state engine: local, global,
both and neither markers; source concrete models; composed document/tool/dock/proportional
models; a tool-only interface; routable models; and ManagedDockWindowDocument. Every positive
cast retains the original model address. A marker-only custom model remains alive through
its query results and releases after the last result is dropped. Erased runtime diagnostic
names retain the concrete native model name. Queries use actual inherited interfaces and
Rc trait upcasts; they add no replacement model, stored role flag or UI dependency to Domain.
Arbitrary marker-only application models explicitly supply the native optional query beside
the marker trait, as with existing native interface hooks. CLR generic/alias naming remains
unverified; custom models can provide their explicit runtime name without reflection.

The original IGlobalDockingService interface now compiles. Its concrete service and the
initial DockManagerState, ManagedHostWindow and ManagedHostWindowState/context source
translations are STAGED OUTSIDE CARGO with the prior adorner. These files are formatted
only, not type-checked or behaviorally validated; no source member is marked complete.
They preserve the source branch structure for local/global adorners and validation, managed
coordinates, preset resolution, presentation/close cancellation/preserve-layout lifecycle,
managed drag phases, preview/float/drop and layer lookup. None of those staged behaviors is
established by the passing test totals. Native host/state mutual strong edges remain strong;
Rc cycle collection versus CLR GC remains an explicit runtime gap.

Actual DockControl/DockControlState, DragPreviewHelper, full HostWindow/State/window-drag,
external/Z-order/visual host lookup, full tab/pinned integration and complete source themes
remain pending. Source global-target/adorner/managed-host lifecycle and drag tests must run
when this group enters Cargo. There is still no runnable Rust sample. Previous collection,
binding, weak-table lifetime and mixed-DPI/runtime gaps remain open.

The older entries below describe their respective intermediate validation points.

# Latest validation: hit testing and global docking helpers

The locked workspace run passes **1,269 Rust tests**, zero failed/ignored (286 in the UI
crate). Strict locked all-target Clippy and release UI-library Clippy, formatting, structural
inventory, factory generation and six Python verifier tests pass. Inventory: 209 ported,
230 partial and 1,489 pending of 1,928 inputs. The complete-port gate remains red. Original
C# and published =0.3.0 dependencies are unchanged; the existing nonfatal macOS debug
compact-unwind warning remains.

Fourteen new Rust tests validate the original DockHelpers subset and complete
GlobalDockingHelper algorithm. The three original DockHelpersTests assertion sets run on
native Headless controls and production ReactiveUI counterparts; the original alternate
framework-property fixtures remain pending. Additional cases cover window visibility,
raw/filtered hit fallback, topmost order, explicit drop flags, native screen scaling,
model/root/factory precedence, source getter counts, diagnostics and original nested-root
ShowWindows command/HostAdapter dispatch.

Global docking tests preserve the original directional tuple, splitter/single-child-wrapper
exclusions, direct proportional-child recursion, live list reads, threshold exit and custom
enumerator disposal. A separate .NET compiler probe checks the repeated Owner pattern's
single getter read; it copies the pattern shape only, not the original helper class. The
existing logger uses a match to preserve callback preference in both build configurations.
Its existing settings tests pass after the release lint adjustment.

The initial AdornerHelper translation is staged outside Cargo. It is formatted but not
type-checked or behaviorally validated, and no source member is marked complete. Actual
DockControl visual host lookup and ManagedWindowLayer.TryGetLayer remain absent. The
concrete hosts/states, source adorner lifecycle/geometry tests, tabs/pinned integration,
complete source themes and runnable Rust sample remain pending. DockHelpers also lacks
concrete-host/external-surface/Z-order members and CLR exception stack/inner graph transport.
Mixed-DPI desktop and previously recorded native lifetime/binding gaps remain unverified.

The older entries below describe their respective intermediate validation points.

# Latest validation: managed-window layer and source theme

The locked workspace run passes **1,255 Rust tests**, zero failed/ignored. Strict locked
all-target Clippy, formatting, structural inventory, factory generation, six Python tests
and diff checks pass. Inventory: 208 ported, 227 partial and 1,493 pending of 1,928 inputs.
The complete-port gate still fails as intended. The known macOS debug compact-unwind
warning remains unchanged; original C# and published dependencies are unchanged.

This pass adds 18 tests: sixteen native layer/registry/conversion/callback cases and two
separate compiled source-theme consumers. Five more original ManagedWindowParity scenario
assertion sets run with actual production models/native controls. Tests retain registry
same-key/replacement/unregister/default identity, exact list event actions and original
window subscription removal, keyed overlays/NaNs/template replacement, first-visible-frame
content offsets, hidden-probe cleanup/retry/cache, custom manager list identity/mutation/
post-callback Count and reentrant snapshot order. Native detach releases the actual layer
with dependencies still alive. The original strong factory backreference/cycle retention
and noncyclic weak-key expiry/pruning are explicitly characterized.

The complete source ManagedWindowLayer dictionary creates real MDI frames, original list/
model/state/topmost bindings and original ZIndex=10000 overlay canvas. Raw/handle/nullable
managed dock references convert to nullable styled IDock through shared ValueTypes and retain
identity; no replacement model or binding engine is used. The complete MdiDocumentWindow
content/header/button dictionary is still absent; unit content-part templates test offset
measurement only. Static layer TryGetLayer is absent until actual DockControl search exists.

ConditionalWeakTable/ephemeron GC, native process/thread identity and value-to-key cycle
collection remain unresolved. Concrete managed host/controller, tracking/adorner/preview,
full tab/pinned integration, global DockFluentTheme and runnable Rust sample remain pending.
Previously recorded compiled IList Item versus Item[] and generic collection/runtime gaps
remain open. This validates the implemented layer subset, not full engine/API/UI parity.

The older entries below describe their respective intermediate validation points.

# Latest validation: root policies and factory command bindings

The locked workspace run passes **1,237 Rust tests**, zero failed/ignored. Strict locked
all-target Clippy, formatting, structural inventory, factory generation, six Python tests
and diff checks pass. Inventory: 208 ported, 224 partial and 1,496 pending of 1,928 inputs.
The complete-port gate still fails as intended. The known native macOS compact-unwind
warning remains; original C# and published dependencies are unchanged.

This pass adds 18 tests: ten original converter/boundary cases, one complete root metadata
case, four factory argument/engine/override cases and three downstream compiled Headless
cases. All six capability singleton pairs retain identity and use the original resolver.
Removal tests distinguish original opened count from visible count; close visibility covers
mode/active defaults, typed versus numeric objects, signed text and comma-name parsing.

All 15 root own metadata members preserve original lists/windows/policies/commands and
notifier identity. Every typed/object command pair is compared with direct calls into the
original production engine, including actual split/pin/preview/float/close/layout results.
Original options and enclosing factory overrides retain identity and typed/object-null
semantics. Compiled controls respond to original property/reference/Count changes and invoke
nested factory overloads to change layout/tab modes and close the same document.

IFactory metadata covers 47 command overloads and two flags; non-command properties/methods/
events and generic metadata remain pending. Complete source window/global themes, actual
DockControl/managed host/adorner/tracking, tab/pinned integration, alternate framework-property
models and a runnable Rust sample remain pending. Existing compiled IList Item versus Item[]
and generic collection/runtime lifetime gaps remain explicit, as do native singleton scope,
CLR exception/UTF-16 and arbitrary raw type discovery. This is verified port progress, not
full engine/API/UI parity.

The older entries below describe their respective intermediate validation points.

# Latest validation

The locked workspace run passes **1,219 Rust tests**, with zero failed/ignored. After
aligning EmptyContentTemplate setup with source attachment order, all four affected native
MDI theme tests also pass. Strict locked all-target Clippy, formatting, structural inventory,
factory generation, six Python tests and diff checks pass. Complete-port validation still
fails as expected: 208 ported, 218 partial and 1,502 pending of 1,928 source inputs.

This pass adds 16 tests: six dock/list/policy metadata, four original list projection,
four complete MDI host-theme scenarios, one downstream compiled dock command/binding
consumer and one six-enum metadata test. Live original lists retain exact typed source
change payloads/indices/identity and explicit unsubscribe. Mutable native reordering uses
the original list. The complete source host dictionary renders empty content/templates,
real document frames and live state/ZIndex. A named native button executes the original
CascadeDocuments ReactiveCommand, and two-way fields/object/null roundtrips are verified.

The compiled IList indexer notification mismatch in published FerroUI 0.3.0 is explicitly
characterized, not fixed: Item vs Item[] leaves the old indexed item subscribed until path
replacement. Counts-only custom notifying list adapters require their original typed event
hooks. Full window/context-menu/global themes, framework-property models, host/controller,
tab/pinned integration, desktop rendering and the Rust sample remain pending. The native
MDI frame class is present but its full document window/content theme is absent. The known
macOS debug compact-unwind warning remains unchanged. No dependencies or original C# changed.

The older entries below describe their respective intermediate validation points.

# FerroDock

FerroDock is the Rust port of Dock for FerroUI. It preserves the docking engine,
contracts, layout behavior, and markup structure, using FerroUI's corresponding
framework and MVVM libraries.

**Work in progress.** The shared operation types, capability policies, manager
options, MDI defaults, geometry/tracking adapter, optional model contracts,
recycling model contracts, core interface/event graph, group and capability
validation helpers, navigation adapter, concrete reactive documents/tools/splitters,
all ten reactive dock containers, MDI layout algorithms and global busy service
are implemented. The reactive DockWindow, host lifecycle adapter and production
reactive layout factory, docking service/manager, outer-region splitting and
content/template state cache, workspace snapshots/dirty tracking and fluent
creation/configuration helpers are present. UI controls, themes, production
serializers, diagnostics and samples still need porting. There is no usable
docking control yet.

The C# source in the parent repository remains the reference implementation.
The source baseline and current file/member status are recorded in
[docs/porting/dock/PORTING.md](docs/porting/dock/PORTING.md) and [docs/porting/dock/ports.json](docs/porting/dock/ports.json).

From this directory:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/dock/check_port.py
python3 -m unittest discover -s scripts -p 'test_*.py'
```

FerroUI dependencies are pinned together to `0.3.0`. Builds use the published
crates, so no sibling checkout or local changes to FerroUI are needed. The domain model, reactive models
and reactive services do not depend on the UI framework's controls or platform.

See [the dependency map](docs/porting/dock/DEPENDENCIES.md) for the corresponding System,
Rx, ReactiveUI, and UI contracts, and [the remaining work](docs/porting/dock/CONTINUATION.md)
for the next porting steps. These checks cover the implemented subset; they do
not establish full API or behavioral parity.

The geometry tests include 7,895 outputs captured from the actual C# geometry
sources, protecting invariant formatting and shortest-digit rounding. See the
[reference generator](scripts/dock/geometry-reference/README.md) for reproduction;
the normal Rust test run reads the committed fixture without requiring .NET.

The interface graph includes the factory, docks, tools/documents, windows/hosts,
policies and lifecycle events. These are contracts and helper implementations;
the reactive dock tree, navigation commands and MDI layout now have concrete
implementations, including floating window models and host lifecycle handling.
The production factory now composes the source engine; partial generic/runtime members and
runtime parity gaps remain explicit in the inventory.

DockBase uses the published WhenAnyValue and ReactiveCommand APIs for activation,
focus and navigation. DocumentDock creation, cascade/tile/restore commands and
virtual add methods follow the source algorithm and callback order. Native
composition constructors preserve the outer model identity through inherited
properties and callbacks. The latest validation comprises 1,009 Rust tests;
[docs/porting/dock/VALIDATION.md](docs/porting/dock/VALIDATION.md) records the scope and remaining gaps.

The factory base now includes locator/search algorithms, repeatable lazy Find
queries, visible-list normalization, both window-initialization overloads,
recursive layout and selection initialization, focus transfer, global tracking,
all factory events, lifecycle notifications, window registration/removal and
docking-state synchronization. Collection updates, move/swap overloads, pinned
reordering, close-range requests and document layout commands are also present.
Pin/preview, removal/collapse, hide/restore, split layouts, floating-window
construction, document-pane creation, close handling and ItemsSource tracking
now have native source implementations. Weak-table lifetime and custom boxed-value
metadata remain explicit parity gaps.
FactoryBase remains an abstract component requiring the full IFactory contract.
The concrete ReactiveUI Factory now supplies registries, creators, virtual dispatch
and source root pin lists. Generic equality/erased dispatch, composed base-class
casts and existing lifetime/metadata gaps remain incomplete.

Model IList properties now use `ferrodock_model::collections::ListHandle`, which
can wrap a plain/custom list or the published FerroUI observable collection.
Clones preserve collection identity and live mutation. Use `from_plain`,
`from_list` or `from_observable` for the desired backend; the default `new` and
`from_items` still use FerroUI's observable implementation. This corrects the
initial contract that admitted only observable collections.


The production model factory can construct and initialize a layout graph:

```rust
use ferrodock_model::core::IFactory;
use ferrodock_model_reactiveui::Factory;

let factory = Factory::new();
let root = factory.create_layout().unwrap();
let documents = factory.create_document_dock();
factory.add_dockable(root.clone(), documents.clone());
factory.add_dockable(documents, factory.create_document());
factory.init_layout(root.clone());
```

This creates models; the docking UI is still unported. Application factory
subclasses compose `Factory`, implement `FactoryBaseImpl`/`FactoryImpl`, and use
`impl_factory!`. See the downstream-crate examples in
[src/Dock/FerroDock.Model.ReactiveUI/tests/external_factory.rs](src/Dock/FerroDock.Model.ReactiveUI/tests/external_factory.rs).


The fluent API is available through `ferrodock_model::fluent_extensions::*`.
Factory helpers configure the actual virtual creator result; `_into` overloads
write an output handle and retain the factory for chaining. Property helpers
return the same shared model handle. Collection helpers append to existing lists
and use plain lists for missing properties, following the source; they do not
initialize the docking tree. Framework-property and UI integration remain pending.


## Overlay requests and busy/global service pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 724 passed; 0 failed/ignored (209 model, 460 reactive model, 12 downstream consumer, 36 service, 7 recycling tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 135 ported, 86 partial, 1,707 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

Twenty-eight new tests validate native request/service algorithms. Eight model
cases cover request metadata, command/callback/content/task identities, nullable
results and titles, first-completion wins, reentrant completion, global handle
replacement without implicit disposal, delegate predicate/execute/event behavior,
reentrant subscription removal and actual dialog-content/reload contracts.

Eight global-scope cases validate dialog/confirmation defaults, nested/out-of-order
scope disposal, latest/empty messages, queued notification/property order,
reentrant reopening followed by the source outer message clear and native release.
Twelve busy/reload cases validate defaults, eager/pending scoped work, error object
identity and cancellation, begin failures, cleanup-failure precedence, same/foreign
context and queued dispatch paths, handler identity/replacement, reactive CanReload
and execution gates, reload errors, reentrant setters, explicit scope end and release.
Actual shared System tasks and ReactiveUI commands are used, with no local task or
reactive engine substitute. A synchronous local cleanup notification failure skips
later global disposal exactly as the source code does; no extra cleanup is invented.

The canonical BoxedValue ICommand result boundary, rich exception/reference/value/
delegate metadata, retained-command/CLR GC lifetime equivalence, cross-thread
Interlocked/ConfigureAwait behavior and original multiwindow/Headless integration
remain partial. Remaining per-host dialog/confirmation collections/services need a
faithful ReadOnlyObservableCollection and asynchronous dispatcher continuation
representation. Hosting/lifecycle/navigation, framework-property controls/themes/
markup, production serializers and all earlier engine/runtime/API gaps remain in
the full goal. Passing these tests does not prove a usable overlay/docking UI or
complete parity.


## Per-host dialog/confirmation stack pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 756 passed; 0 failed/ignored (214 model, 461 reactive model, 12 downstream consumer, 62 service, 7 recycling tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 137 ported, 90 partial, 1,701 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

Thirty-two new tests cover the original stack algorithms and their native runtime
boundaries. Four read-only adapter tests validate live view/identity, all IList
mutation failures, notification payload and property order, typed/untyped listener
registration order, source subscription/reentrancy counts and native detachment.
One model result test checks value/default/nullable/reference rules; one actual
reactive model test proves shared factory reference queries retain model identity.

Twenty-four service regressions cover source stack defaults, insertion/active-last
selection, read-only view identity, remove/dispose/complete/active/Has ordering,
foreign and already-closed requests, snapshot cancellation with reentrant additions,
source stale active state after reentrant close during Add, listener/disposal/posting
failures without invented rollback, eager/pending lifetime, queued insertion/close,
early content close before Add, completed-request guard, typed result defaults,
original content/result identity and all six actual services through the production
host adapter. Same-context and queued dispatcher paths are exercised.

Two dispatcher tests establish that a pending await's wake runs off the completing
thread, repeated polls retain a single System wake registration, already-completed
awaits remain inline and action failures become task errors. Queued service tests
resume through the actual System owner executor/queue. Task completion/status and
collection storage remain in shared System primitives; no duplicate task or mutable
collection engine is provided.

The native internal await adapter does not implement the entire shared
TaskCreationOptions API. Its worker-per-pending-wake transport still needs performance
and shared-runtime integration; full ConfigureAwait/cross-thread and exception
metadata remain partial. Shared observable mutation versions, automatic unregistered
content/interface/base-class metadata, CLR retained-callback/event/GC lifetime,
compiled binding metadata and original multiwindow/Headless/UI integration remain
unverified. Hosting providers, lifecycle/navigation, framework controls/themes/
markup, serializers and all prior engine/runtime/API gaps remain within the active
full-port goal. These checks do not establish a usable docking UI or complete parity.


## Host resolution, fallback provider and lifecycle pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 771 passed; 0 failed/ignored (214 model, 461 reactive model, 12 downstream consumer, 77 service, 7 recycling tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 138 ported, 95 partial, 1,695 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

Fifteen new tests execute real lookup/provider/lifecycle algorithms with production
factory searches, shared ReactiveUI routers, composed reactive root models and
actual overlay services. Both original lifecycle cases and production factory
closed/removed event integration are covered. Additional cases verify native
concrete/interface identity and rejection of false projection, last-owner and
reverse-router lookup, host cycles, host-window/factory/floating root precedence,
root normalization, per-screen and reentrant cache behavior, correct-key eviction,
nonfocusable root/window context, expired noncyclic release, no fallback creation
in cache-only cleanup, callback layout replacement, failure marking and reentry.

The native registry holds weak same-object projections. It does not prove automatic
unregistered CLR class/interface/base discovery or custom Equals/GetHashCode parity.
Provider weak-key/strong-value caches still lack ephemeron cycle collection, exact
GC eviction timing and pruning performance verification. Full original framework/
multiwindow/Headless integration remains pending. The new dev-only dependency on
production reactive models supports tests; production service dependencies and
published FerroUI pins remain unchanged. No UI control, theme, serializer or full
runtime/API completion is claimed.


## Navigation, routable models and busy root pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 794 passed; 0 failed/ignored (214 model, 461 reactive model, 12 downstream consumer, 100 service, 7 recycling tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 140 ported, 105 partial, 1,683 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

Twenty-three new tests execute the source navigation and routable/overlay model
algorithms. All seven original cases in both classes of DockNavigationHelpersTests
are represented, using actual shared ReactiveUI Navigate/NavigateBack commands,
production reactive models and the production factory's base operations, including
floating and removal/readdition. Additional cases cover independent nearest
factory/document-dock resolution, attached host/factory fallback, unlisted explicit
default selection before direct visible children, no recursive fallback, foreign
visible owners, no-context no-op, cancelled close and all eight overlay helper
overloads with unchanged arguments.

Model tests verify independent routers, explicit empty routes, enclosing-model
class-name defaults and boxed identity, inherited INPC and dock commands, both
abstract document/tool helpers using self as both screen and dockable, missing
provider messages, and own-router back navigation. DockViewModel tests verify
factory CreateLayout/InitLayout order and counts, null/nonoverlay service failures,
current-layout service identity, source notification equality and replacements
without implicit initialization. BusyRootDock tests cover one services-factory
invocation, active-child precedence without parent fallback, top-router-only reload
before self, pending/error task identity, actual busy command execution, reload
removal before cancellation, pending request completion, reentry and failure
marking without retries. No substitute task, router or docking engine is introduced.

The services crate now depends on the production reactive model crate as the
source assembly does; this promotes the prior test-only dependency. All published
FerroUI pins remain exactly 0.3.0. Native registries explicitly project erased
screen/routable objects and use dockable hooks. Automatic unregistered class,
interface and base metadata, generic class names, serialization IgnoreDataMember,
weak-host versus CLR GC/retained-callback lifetime, observable mutation versions,
custom equality and full cross-thread ConfigureAwait remain partial. Activation,
DI/dispatcher integration, framework-property controls/themes/compiled markup,
serializers, alternative models and samples remain in the full goal. Passing this
subset does not establish a usable docking UI or complete API/runtime parity.


## Activation, main-thread dispatch and Splat registration pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 811 passed; 0 failed/ignored (214 model, 461 reactive model, 12 downstream consumer, 117 service, 7 recycling tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 142 ported, 108 partial, 1,678 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

Seventeen new service tests cover the activation, dispatch and resolver algorithms.
Three activation cases use the actual ReactiveUI activator and resources to check
nested activation counts, setup/event/cleanup order, explicit and ignored handles,
view noninspection/nonretention, failures and independent models. The bridge adds
no activation state. Shared activator scopes hold weak native state; equivalence
to retained CLR scopes after model release remains unproved and partial.

Four dispatcher cases exercise actual Rx schedulers and System tasks. Every call
reads the current main-thread scheduler and schedules through it, even with a
current synchronization context. Immediate/queued failures fault the task;
scheduler failure propagates synchronously. Scheduler changes between calls and
during reentrant dispatch, pending completion and reusable action handles are
covered. Raw System Task continuations remain synchronous because the pinned
runtime exposes no RunContinuationsAsynchronously option; this source requirement
is still deferred. The private service await adapter does not complete this public
raw-task requirement.

Ten Splat cases exercise actual published resolvers and all nine registrations:
source order, lazy/idempotent registration, preservation of existing services,
named/default/null contracts, singleton identities, fresh local overlay services
sharing global services, global busy/confirmation state, per-screen provider
caches and six independent global lookups in source constructor argument order.
Required-service lookup uses the local read-only resolver when available; the
mutable-only branch reads Locator.Current at each lookup. Explicit erased native
projections must reference the same allocation. Missing local registrations do
not fall back to the locator. Copied registrations and resolved Func closures
retain their original resolver, preserving resolution context.

An integration case resolves the actual registered provider/lifecycle service,
constructs BusyRootDock from the registered Func, and delivers window events via
the production factory. Closed event delivery precedes cancellation; pending
dialog/confirmation tasks complete and root disposal disables reload once.

Splat is now an explicit normal dependency at exactly 0.3.0, already the shared
ReactiveUI dependency version. No new container, locator, router, activation or
task engine is introduced. Automatic IReadonly membership on arbitrary erased
mutable resolvers requires shared metadata; the native API carries that projection
explicitly. Registration/Func closures intentionally retain the original resolver.
Explicit resolver disposal breaks registrations; collection of unreachable native
resolver/closure cycles without Dispose remains a CLR GC parity gap. Rich exception/
FullName metadata, cross-thread semantics and original UI/Headless/leak integration
remain open. Microsoft IServiceCollection registration is still pending: the
inspected published System/Splat APIs provide no matching service-collection
implementation. Framework-property models/controls/themes/compiled markup,
serializers, alternative model stacks, samples and all earlier gaps remain in
the full goal. The completeness gate must continue rejecting this port.


## Reactive framework control and initial Headless integration pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 815 passed; 0 failed/ignored (214 model, 461 reactive model, 12 downstream consumer, 117 service, 7 recycling, 4 framework Headless tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 142 ported, 110 partial, 1,676 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

The new separate framework integration crate uses published Base, Controls and
ReactiveUI.Controls 0.3.0 with the actual Headless platform in tests. Domain,
reactive model and framework-independent service dependencies remain free of UI
controls/platforms. All framework versions, including build-scan and transitive
Headless dependencies, use the same release; no sibling checkout is a dependency.

Four tests execute actual headless windows, loading/unloading, registered property
changes and dispatcher jobs. They verify one bridge lookup per activation, use of
the captured bridge after locator replacement until reactivation, original control
identity, obtaining a new handle before disposing the previous serial value,
nonactivatable/null values, subscription cleanup on detach, direct shared-activator
fallback and release of a cleared model while the view survives. A derived control
retains native class identity, inherited typed ViewModel/DataContext synchronization
and activation. Model resources use the actual shared ReactiveUI activator.

The generic source control follows FerroUI's existing non-generic class plus typed
ReactiveView handle convention. It derives from ReactiveUserControlBase and calls
the parent's constructor, preserving inherited property and activation behavior.
The source activation scope adds SerialDisposable before its ViewModel subscription
and resolves IActivationBridge once per activation. An explicit same-object native
activatable projection is currently required BEFORE binding/activation because the
shared automatic activatable lookup is not exposed across crates. The existing
HostTypeRegistry supplies this projection, with no second activation/type engine
or global mutable registry. Unregistered CLR membership remains partial.

The library uses the shared build-scan exporter and Cargo links metadata. The
active export contains its public class path, source library identity and three
framework metadata dependencies. Runtime class/namespace/assembly registration
and public Rust paths are supplied; abstract class metadata has no default
constructor. A dependent compiled-XAML application has not yet validated the export.

The macOS debug linker emits a nonfatal warning that its compact unwind offsets
cannot encode the large __eh_frame section; all tests pass. Debug exception-path
performance is not verified. No linker warning is suppressed and no broad profile
change is introduced.

This is initial framework integration, not a usable docking control. The visual
host resolver still requires real DockControl, OverlayHost, HostWindow and managed
window layer controls. No smaller surrogate controls or visual-tree lookup engine
are substituted. Full compiled consumer markup/bindings, generic/subtype metadata,
original application/desktop/leak tests, all docking behaviors and themes, serializers,
alternative model stacks and earlier runtime/ownership/DI gaps remain in the full
goal. The original DockControl activation-order leak test is still pending and is
not claimed by these narrower reactive-view lifetime checks.


## Proportional panel, splitter and compiled theme pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 841 passed; 0 failed/ignored (214 model, 461 reactive model, 12 downstream consumer, 117 service, 7 recycling, 4 reactive framework and 26 proportional Headless tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 149 ported, 121 partial, 1,658 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejected the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

The new proportional-control library uses the actual FerroUI Panel/Thumb/Control
class hierarchy, attached/styled properties, native virtual dispatch, layout,
pointer routing, cursors, adorners and rendering. It ports all eight source sizing,
constraint and navigation helpers, the main panel/splitter bodies and preview
adorner. Pure helper records are ported where complete; controls and helpers with
recorded integration/runtime differences remain partial. No replacement layout or
docking algorithm is introduced.

Twenty-six new tests run on actual FerroUI Headless. They include exact horizontal,
vertical and nested child rectangles; all eight original border allocation rows;
collapsed storage/restoration; original minimum/maximum assignment cases; all six
source private splitter constraint cases without reflection; both sizing methods
and their three rows; all four source policy methods and their ten rows. Native
derived panels retain the five source sizing/resize hooks, intrinsic/per-pane
measurement, infinite-space rejection, explicit extent across window resizing,
default normalization and pointer/preview resize arguments.

Additional cases cover source numeric NaN/signed-zero/ties-to-even handling,
collection-content mutation detection, presenter-wrapped splitter discovery and
collapsed/splitter sibling skipping. Real pointer input with layout jobs between
events commits preview resizing and removes the adorner. A recording drawing core
forwards to the real Headless backend and checks actual preview rectangle commands
and default Gray, host accent and explicit brush precedence. Live opacity, raw
capture-loss cleanup, repeated cleanup and a reentrant opacity callback during
preview object initialization are covered. The new preview field is assigned only
after object initialization; later field references are reread at source callback
boundaries. These are supported rendering-command/lifecycle checks, not desktop
pixel or performance comparisons.

The splitter ResourceDictionary and Border template are compiled by shared
ferroui-build Model support with explicit typed and compiled-binding scopes. A
Headless test loads the real compiled resource, applies its theme while attached,
and verifies TemplateBinding follows the control. The library exports class,
namespace, assembly, public Rust paths and compiled document metadata. Compiling
this theme exposed and corrected a namespace-table issue: use the standard named
NAMESPACES constant for both source scanning and runtime registration. The prior
reactive framework crate now follows that pattern too; its consumer compiled-XAML
integration is still pending.

The shared UI FerroList exposes copy-on-write storage instead of the original
mutation version. A private iterator detects storage replacement without adding
notification subscribers or another collection engine. Content invalidation has a
regression, but exact version semantics for capacity-only/storage changes remain
partial. Original Debug.Assert diagnostics, the original ItemsControl layout case,
remaining full min/max/constraint/runtime-binding suites, complete model/template
integration, desktop/pixel/leak validation and target builds remain unverified.
The macOS debug linker still emits the previously recorded nonfatal compact-unwind
size warning. Full DockControl, OverlayHost, managed/native hosts, the visual-tree
resolver, other themes, serializers, model alternatives, samples and earlier
runtime/metadata/ownership/DI gaps remain within the active full-port goal.


## Proportional integration and external markup pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 862 passed; 0 failed/ignored (214 model, 461 reactive model, 12 existing downstream model consumers, 117 service, 7 recycling, 4 reactive framework, 46 proportional Headless and 1 external compiled-XAML Headless test) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 152 ported, 120 partial, 1,656 pending |
| `python3 scripts/dock/generate_factory.py --check` | Passed |
| `python3 -m unittest discover -s scripts -p 'test_*.py'` | 6 passed |
| `python3 scripts/dock/check_port.py --require-complete` | Correctly rejects the incomplete port |
| `git diff --check` and original-source diff check | Passed; original C# source unchanged |

All eleven main ProportionalStackPanelTests methods now have native counterparts.
The original ItemsControl method deliberately installs no control template and
expects all source item bounds to remain zero. That expectation is retained. An
additional case uses actual ItemsPresenter templates in a Headless window and
checks exact rectangles for generated nested horizontal/vertical proportional
panels and splitters.

All seventeen MinMax source scenarios are covered, including the original seven
layout/dimension rows, repeated proportion changes and simulated drags, changes
to panel width, mixed constraints across three panes, NaN/zero proportions,
overflow and three private splitter operations. All fourteen SplitterHeadless
source scenarios are covered: target/neighbor min/max in both orientations,
movement direction, simultaneous minima/maxima, sequential mixed-limit drags,
conflicting limits, two consecutive splitters and exact zero-delta bounds. Private
child-module access replaces reflection. Native test functions consolidate source
rows; test-function counts are not xUnit theory row counts.

Two additional regressions use production ReactiveUI Document models and actual
compiled typed-accessor binding paths. Both orientations at local and style
priority exercise default TwoWay metadata, normalization and resize writes to
the models, later model changes and preserved binding. DataContext replacement
removes the old source subscription; disposal removes the current subscription,
and the old acyclic model is released with the control alive. These are supported
binding/lifetime checks, not CLR GC or complete heterogeneous model metadata.

A separate unpublished workspace package compiles ProportionalConsumer.xaml
against the proportional library's exported dependency metadata. It resolves the
custom types/namespace, attached property literals and compiled paths, and a
ResourceInclude of the source splitter dictionary. Loading in actual Headless
checks exact layout and implicit theme Border creation, writes through a default
TwoWay binding after resizing, then observes later source changes. The compiler's
own assembly defaults to ferrodock_markup_tests. Resource values use shared native
object projections; implicit themes are verified through their created visuals.

The previous unverified original ItemsControl/MinMax/SplitterHeadless entries are
now resolved. Complete original docking themes/generated model containers,
automatic erased interface/property-model metadata, arbitrary equality/class/
reference behavior, native collection versions, Debug.Assert diagnostics,
desktop/pixel/leak/target checks and all other full-port work remain. The existing
macOS debug compact-unwind linker warning remains nonfatal and unchanged.


## Native recycling controls and external consumer pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 882 passed; 0 failed/ignored (214 model, 461 reactive model, 12 existing model consumers, 117 service, 7 recycling-model, 4 reactive framework, 46 proportional, 19 native recycling and 2 external XAML tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; two shared generated-emitter lints allowed only inside the test package's generated module |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 156 ported, 123 partial, 1,649 pending |
| Factory generator check and six verifier tests | Passed |
| Completion gate | Correctly rejects the unfinished full port |
| Original C# diff and `git diff --check` | Clean |

Native Headless counterparts cover all fifteen ControlRecyclingTests methods and
all five ControlRecyclingDataTemplateTests methods. Related cases are consolidated
into native functions. The actual ManualPresenterHost native subclass retains its
source default template and manually assigned presenter, with different logical
host content. Real native templates and controls cover cached updates, differing
ID keys, blank/disabled identifiers, panels and presenters, unparented reuse and
moving cached content while retaining the original presenter's compiled binding.

Additional cases cover plain/recycling template rebuild order on cache hit/miss,
null fallback retaining cache, replacement controls and null cached updates,
existing/null callback arguments, unsupported parents, direct data-template and
self-reference guards, inherited cache values, direct-property notifications,
canonical original-contract handles, key equality (custom values, NaNs, signed
zero and base/derived native aliases), Remove IDs/null, explicit release and raw
derived native object projection through the domain trait. The transient native
class probe calls shared ValueTypes; no probe becomes a stored model/property.

The separate downstream XAML package now also compiles and loads the recycling
classes, their direct bool and inherited attached cache properties, original
contract projections and compiled #Presenter Parent binding. An actual Headless
window initializes the same-control forward StaticResource; only then does the
test inspect the cache. Changing content away/back restores the cached native
control. Both external consumers pass through shared metadata, compiler and loader.

The source model traits stay UI-independent. UI-owned empty metadata carriers
host descriptors of their ORIGINAL Rc handles because foreign trait/MarkupTyped
impls violate Rust's orphan rule. They are not behavioral replacements. Full
interface member/notification metadata and automatic arbitrary raw domain value
metadata/equality remain incomplete. Shared AnyValue has no arbitrary custom hash
API; equality-preserving per-type buckets do not prove equivalent hashing cost or
custom equality reentrancy. Native/CLR ownership, cycle collection, complete
original docking templates, desktop/pixel/leak/platform checks and all broader
full-port requirements remain open.

The published compiler emits duplicate nullable setter branches and Clone::clone
for a direct bool literal. clippy::if_same_then_else and clippy::clone_on_copy
are allowed ONLY around the generated module of the external test package;
handwritten sources remain checked. The existing nonfatal macOS debug linker
compact-unwind size warning is unchanged.


## Native overlay host, layers and downstream markup pass

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 901 passed; 0 failed/ignored (214 model, 461 reactive model, 12 existing model consumers, 117 services, 7 recycling-model, 4 reactive framework, 46 proportional, 19 native recycling, 18 native overlay and 3 external XAML tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; only the previously documented scoped shared generated-emitter allowances |
| `cargo fmt --all -- --check` | Passed |
| `python3 scripts/dock/check_port.py` | Passed; 1,928 source inputs: 160 ported, 130 partial, 1,638 pending |
| Factory generator check and six verifier tests | Passed |
| Completion gate | Correctly rejects the unfinished full port |
| Original C# diff and `git diff --check` | Clean |

The new native UI crate executes the actual source overlay pipeline using
Decorator/FerroObject, shared native properties and collections, ContentPresenter,
ControlTheme, visual/logical parents and the actual dispatcher. Eighteen Headless
cases include the four original pipeline regression method bodies: content moving
from a visual presenter, the detach-time layer mutation with maximum callback
depth one, equivalent replacement retaining original attached objects/private
list, and a genuinely different replacement rebuilding.

Further cases cover native defaults/properties and canonical original-contract
identity; lazy repeatable stable ordering that buffers before getters; repeated
entry getter order; factory/null/result priority and per-host fresh instances;
provider collection identity, empty fallback and invisible-layer suppression;
shared service-provider queries and provider closure release; native, ordinary
and complete custom read-only list backends with exactly one direct notification
subscription; internal content-host precedence; theme/resource type keys, input
and nonzero-z behavior; detach/reattach/latest-value behavior and supported
acyclic host release with layers still alive; full list operations and source
factory programmer-error/style-key values.

The separate consumer compiles native host/layer/collection Add APIs, original
interfaces, content metadata, nullable collection properties and original
PascalCase static style fields. An actual Headless window confirms the resulting
pipeline, explicit-layer precedence, fallback control left unparented, input
blocking and z-index. Shared nullable registration fixed a model/compiler query
failure; no custom type-model/compiler path is used.

This does not port the original App.axaml FluentTheme + DockFluentTheme fixture.
The source OverlayHostRebuildPipelineTests record remains PARTIAL until that
harness and its implicit busy/dialog/confirmation layers are present. Actual
service controls/templates, DockControl, native/managed hosts/resolver and all
other full-port work remain. Native registry thread affinity is not global
cross-thread/repost parity, native Rc identity is not full CLR delegate equality,
and direct source/collection/native release checks are not full CLR GC or original
leak/theme/desktop/input/pixel/target proofs. Collection versions, arbitrary raw
object resource keys, full inherited/erased metadata and exception/iterator
failure behavior remain explicitly partial. The nonfatal macOS debug linker
compact-unwind warning is unchanged.

## Service overlay controls and original compiled theme resources

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 913 passed, 0 failed/ignored; adds 11 service-control Headless tests and one downstream source-theme consumer to the previous 901 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; generated theme module alone additionally allows the shared emitter's `clone_on_copy` |
| `cargo fmt --all -- --check` | Passed |
| Structural inventory, factory generator, six verifier tests | Passed; 1,928 inputs: 161 ported, 137 partial, 1,630 pending |
| Completion gate | Correctly rejects incomplete full port |
| Original C# diff and whitespace check | Clean |

All three actual service overlay controls use the native class/property/visual
systems and original domain service contracts. Tests cover source field defaults,
read-only direct properties, pre-attachment subscriptions, same-instance assignment,
replacement, manual and actual tree detach/reattach, retained state versus cleared
request collections, local/global precedence, input blocking, live read-only
request identity and command identity/enablement/visibility. The original content
host pipeline now composes these controls. A trace probe checks repeated command
getters, exact name filters and short-circuits; a reentrant direct-property callback
removes the service and proves subsequent reads use current fields.

A separate consumer compiles ResourceIncludes from the new native Fluent theme
assembly. Its source dictionaries retain the original visual tree, template and
relative-parent bindings, resource keys and z-order, with explicit compiled typed
scopes. A Headless window verifies production services drive actual template
content enabled/hit-test state, busy text/progress and reload button state/execution.
Dialog/confirmation ItemsControls observe the original read-only collection
identity, live adds/removes and item handles. Clearing services clears their
commands/items. The UI adapter forwards borrowed collection event views to shared
IItemsList without copying the collection. Command adaptation uses the published
ReactiveCommandAdapter; the source direct property retains the original command.

The full DockFluentTheme, original palette/geometry aggregation, request dialog
controls/templates, default service layers and original themed application harness
remain incomplete. This is not a rendered-dialog interaction/pixel test. Empty
UI-owned carriers describe original foreign handles; full member/notification and
generic/erased metadata remain open. Native weak subscriptions and explicit cleanup
are not proof of CLR event retention/cycle collection or arbitrary custom event
accessors. All existing full-port/native ownership/desktop/platform limitations
still apply. The known nonfatal macOS debug compact-unwind size warning is unchanged.

## Rendered request shells and compiled default layers

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 918 passed, 0 failed/ignored: previous 913 plus two native shell property tests and three downstream rendered/default-layer tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, retaining the existing scoped shared-emitter allowances only |
| `cargo fmt --all -- --check` | Passed |
| Structural inventory | 1,928 inputs: 162 ported, 144 partial, 1,622 pending |
| Factory generator and six verifier tests | Passed |
| Completion gate | Correctly rejects incomplete full port |
| Original C# diff and whitespace check | Clean |

Both source dialog shells use native TemplatedControl and all original styled
properties/defaults, nullable boundaries, native command identity and dialog
content/template metadata. UI-owned descriptors expose the original immutable
request objects to the compiled source DataTemplates. Their native content bridge
reuses the existing recycling conversion; it does not replace the request models.

The downstream Headless application now loads published FerroUI FluentTheme
0.3.0 so source ItemsControls generate actual request presenters. Tests display
original request content, titles/messages and labels, update title/close visibility,
verify compact dimensions from the full original Compact dictionary and use
keyboard Space press/release to execute native buttons. Close completes the
production dialog task with null; confirmation and cancel complete true/false.
All command dispatch uses the existing framework and original request/service
commands. This is stronger evidence than the earlier list-count-only check.

Original OverlayLayerStyles and OverlayLayerDefaults compile with typed
IHostOverlayServices scopes. A fifth XAML fixture creates two actual OverlayHosts.
The test verifies fresh nonshared collections/controls, six exact original service
identities, source alias themes on displayed pipeline objects, local/global busy
priority, rendered confirmation keyboard completion and DataContext rebinding.
Public equivalent-layer replacements can retain a different old private pipeline;
the test respects this source behavior instead of changing it to simplify lookup.

The full DockFluentTheme/App.axaml, remaining accent/geometry/theme dictionaries,
all docking controls/resolvers/hosts and complete runtime metadata/GC/desktop/
pixel/target requirements remain unverified. Request content's arbitrary raw
object metadata and nonnull raw native command parameter promotion are still
partial; the tested original shell Close button supplies null. UI descriptors do
not establish all request constructors/GlobalHandle/Task/Complete metadata. The
known nonfatal macOS debug compact-unwind linker warning is unchanged.

## Complete palette/presets and original foreign enum resources

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 922 passed; 0 failed/ignored (adds two runtime/build enum metadata tests and two downstream palette/preset tests) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed; redundant generator closure corrected, no new Clippy suppression |
| `cargo fmt --all -- --check` | Passed |
| Structural audit, factory generator, six verifier tests | Passed; 1,928 inputs: 169 ported, 144 partial, 1,615 pending |
| Completion gate | Correctly rejects incomplete full port |
| Source C# and whitespace checks | Clean |

The entire source Accents/Fluent dictionary, five original IDE presets and
ControlStrings now use the shared Model compiler. Their source keys, types,
resource dependencies and literal values are retained. Rendered dialog/default
host fixtures include the real palette before compact tokens. No palette subset,
substituted enum integer, new UI engine or partial DockFluentTheme was added.

A generated UI-owned foreign enum descriptor bridges the original domain enum to
standard runtime MarkupType and exported TypeModel metadata. The shared scanner
cannot read handwritten descriptors and Rust's orphan rule prevents its enum
macro from being implemented on a foreign type. The original model package exposes
only its immutable native declaration text; it retains UI independence. The build
reads that source with syn, generates source PascalCase constants returning the
actual enum, checks every numeric value with const assertions and reuses original
TryFrom. Normal source-scanned exports, standard schema/compiler/loader and all
other metadata remain in place. This bridge adds descriptor data, not a compiler.

Unit tests compare both metadata representations and actual original handles,
values and unknown-number boundaries. A separate compiled consumer instantiates
all eleven variants and checks every payload is the original DockOperation. Other
checks cover the original enum resource, source sizing, static brush alias box
identity, live dynamic-color updates, all eleven source icon geometry objects,
control strings, each preset's resources and distinct editor colors. Presets load
through compiled includes after the original accent dictionary, retaining lookup
context. Constant brushes are consumed through the shared IBrush contract because
the shared compiler may make them immutable; the dynamic brush remains mutable.

This does not complete full DockFluentTheme, its manager, remaining controls and
accessibility/host/resolver dependencies, the original App.axaml or full desktop/
pixel/target/leak checks. Other foreign enum/flags shapes, undefined native enum
values, full raw/erased/generic metadata and all earlier scope gaps remain open.
The existing nonfatal macOS debug compact-unwind warning is unchanged.

Incremental generation was also verified after an actual forced build-script input
rerun: the new shared scan report proves the rerun occurred; content hashes AND
mtimes of all 21 existing runtime/model artifacts remained unchanged. Generated
metadata was not temporarily overwritten with a different unaugmented export.

## Native lifecycle attached behavior and downstream markup

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 933 passed; 0 failed/ignored (adds ten native lifecycle tests and one downstream consumer) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, with existing generated-emitter allowances only |
| `cargo fmt --all -- --check` | Passed |
| Source inventory | 1,928 inputs: 169 ported, 146 partial, 1,613 pending |
| Factory generator and six verifier tests | Passed |
| Completion gate | Correctly rejects the unfinished full port |
| Source C# and whitespace checks | Clean |

The behavior uses the original model lifecycle contract, actual native visual
and property events, a native attached-property owner and standard metadata export.
Ten Headless tests include the original control/model reattachment assertions.
Further cases cover immediate attached enable/disable, repeated values, callback
order, live context reads, immutable event values across nested changes, removal
before callback-driven reparenting, source continuation after reentrant disable,
self-context identity for three native encodings, disabled/non-lifecycle contexts
and actual acyclic control/model release. A separate Cargo consumer compiles and
loads the attached API, attaches an original erased model and verifies lifecycle
counts across detach, reattach and disable.

A detach hook's context change still observes an attached PresentationSource.
This matches the source dependency's [versioned Visual implementation](https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/Visual.cs#L527):
PresentationSource is cleared after the detach event. The regression retains the
source nested old-/old-/new+/control- sequence, rather than suppressing it.

Private weak native token storage preserves source static-delegate removal order
without observable storage properties or strong control/model roots. This does
not establish full CLR GC/delegate/threading or pruning cost equivalence. Unknown
raw model membership and noncanonical custom adapter identity remain open; native
class projections, canonical original interface handles and explicitly registered
typed models are covered. The original test App.axaml's full DockFluentTheme and
all broader docking UI/app/desktop/pixel/target requirements remain incomplete.
The existing nonfatal macOS debug compact-unwind warning is unchanged.

## Native root control, accessibility and compiled consumer

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 946 passed; 0 failed/ignored (twelve new native root/helper cases and one external consumer) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| Source inventory | 1,928 inputs: 169 ported, 150 partial, 1,609 pending |
| Factory generator and six verifier tests | Passed |
| Completion gate | Correctly rejects the unfinished full port |
| Source C# and whitespace checks | Clean |

The control uses the actual native TemplatedControl, optional ContentControl part,
Tunnel input, parent-first template/detach hooks and shared automation peer system.
Original ReactiveUI models and Factory perform preview dismissal. Headless tests
cover KeepVisible, current context/factory replacement, skipped handled input,
renamed/missing template parts, old-handler removal, source detach/reattach behavior
and derived controls. The original root peer test method body runs with original
models and finally-style window cleanup. Additional cases verify name/ID/label
precedence, complete help-state text and current model counts/context encodings.

Helper tests cover actual engine activation plus a composed original Factory
subclass that replaces the owner factory after each callback. Each next operation
uses the current factory. Native Fluent ItemsControl containers and a custom
index-missing source verify direct and fallback lookup, live indexed reads,
first-unrealized-match stopping and selection peer identity. Registered enum names
precede registered general object display. Arbitrary invariant formatting, full
flags/undefined enum text and complete source object equality remain partial.

A separate Cargo consumer compiles exported RootDockControl metadata and a
clearly test-only template, boxes an original root interface, preserves its model
identity, verifies the peer and dismisses a preview through actual pointer input.
Explicit interface boxing needs no runtime registration. Raw concrete contexts
are covered with shared ValueTypes composition registrations; arbitrary automatic
raw-model membership and full generated root binding metadata remain pending.

Full original RootDockControl/DockFluentTheme/App.axaml, other source automation
cases, desktop/pixel/platform and CLR GC/retention/performance checks remain open.
Shared native peers keep weak owners, unlike the source peer's strong readonly
owner; no duplicate native strong cycle is introduced and retention parity is not
claimed. The existing macOS debug compact-unwind linker warning remains unchanged.

## Native deferred content, scoped timelines and reveal animation

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 975 passed; zero failed/ignored |
| Deferred library | 28 native Headless tests |
| External compiled markup package | 12 Headless tests, including the new deferred scope and updated real root main part |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| Source inventory | 1,928 inputs: 170 ported, 153 partial, 1,605 pending |
| Factory generator and six verifier tests | Passed |
| Completion gate | Correctly rejects the unfinished full port |
| Source C# and whitespace checks | Clean |

The new separate library uses actual shared ContentControl/ContentPresenter,
TimeSpan, attached properties, plain/static type metadata, dispatcher/timers and
CubicEaseOut/DoubleTransition animation. Tests preserve queue due-entry identity,
order/sequence, successful-item and elapsed-time budgets, not-ready retention/drop,
re-enqueue updates, callback replacement and version-invalidated scheduling.
Native control cases cover deferred/coalesced materialization, repeat/pre-template
ClearPresentation and all four source standard/deferred pending/realized cases,
reattachment/retemplating, stale child/measure retention, eight default template
bindings, scoped timeline/order/delay/null semantics, independent timelines and
content opt-out. Actual Fluent ItemsControl containers enforce scoped time budgets.
Reveal tests preserve user transitions, update the owned subtype and complete on
native render ticks with replacement content visible throughout.

An external consumer resolves an unprefixed deferred control through the assembly
XML namespace, constructs a timeline from compiled markup, reads its shared default
via x:Static and inherits scheduling into real hosts. Actual asynchronous dispatcher
pumping verifies positive-delay native materialization and replacement. The root
consumer also uses the real DeferredContentControl as its named main-content part;
the outer root fixture remains test-only and does not complete the original theme.

Cross-thread repost/default settings sharing, CLR GC/retention, native timer vs
source Task.Delay transport and wall-clock/exception edges remain unverified or
partial. Arbitrary unregistered raw opt-out discovery and full runtime metadata
remain open. Other original themed-app/alternate-model integration cases, internal
default-queue facade, v11 packaging, desktop/pixel/platform and performance checks
remain in scope. The known nonfatal macOS debug compact-unwind warning is unchanged.

## Native settings, attached properties and independent compiled consumer

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 1,009 passed; zero failed/ignored |
| Settings library | 34 tests, including source property/group/settings/builder cases and metadata |
| External compiled markup | Twelve existing docking UI cases plus one independent settings-only case |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Formatting, inventory, factory generator and six verifier tests | Passed |
| Source inventory | 1,928 inputs: 179 ported, 157 partial, 1,592 pending |
| Completion gate | Correctly rejects the unfinished full port |
| Source C# and whitespace checks | Clean |

Settings tests use the source empty App subclass and Headless factory. They cover
all original property/group/builder assertions, all four host/owner resolution
bodies through original ReactiveUI RootDock, exact defaults, mode precedence,
process-wide visibility, callbacks and destructor reentrancy. Additional cases
verify inheritance/reparent/clear/null behavior, noninherited target flags, exact
TwoWay defaults, native owner construction/derivation, all 21 option branches,
unvalidated scalar values and strict drag/pixel-overflow/OR-short-circuit behavior.
The alternate original Model.Avalonia root fixture remains pending.

The independent settings consumer has no docking UI or theme dependency. Shared
compiled markup constructs original nullable core/settings enum options and a
KeyGesture, reads a static field, resolves an unprefixed property owner and proves
actual default TwoWay updates. All twelve prior compiled cases also pass after
moving model enum descriptors into Settings and retaining the hidden UI alias.
The bridge uses original enum declarations and the shared TypeModel; its original
Rust declaration path, distinct from descriptor path, preserves nullable parsing.

A forced incremental input rerun executed the real build script and refreshed its
scan report. The active runtime descriptor and scanned/final model hashes and
mtimes stayed unchanged; all 18 recorded files across build variants also stayed
unchanged. Cross-platform Debug.WriteLine transport,
callback/writable-static untyped metadata, broad CLR/runtime reference semantics,
package distribution and the remaining full docking/application integration remain
open. The known nonfatal macOS debug compact-unwind linker warning is unchanged.

## Native item dragging, factory policies and auto-scroll

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 1,027 passed; zero failed/ignored |
| Native item-drag helper | 18 Headless tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Formatting, inventory, factory generator and six verifier tests | Passed |
| Source inventory | 1,928 inputs: 179 ported, 159 partial, 1,590 pending |
| Completion gate | Correctly rejects the unfinished full port |
| Source C# and whitespace checks | Clean |

Native input cases cover both axes, strict threshold comparisons, both threshold
delegates in source order, movement-only visual displacement and release-time
reordering, default/explicit policy matrices and policies revoked during movement
or before release. Noninteractive targets, outside selection/reset/callback order,
shared original press state, nonleft releases and source ZIndex overflow/clear and
transform replacement are exercised. All eighteen original tool/document policy
cases have helper-boundary counterparts using actual native TabStrip/TabStripItem
and original ReactiveUI models/factories. A real original pinned layout is mutated
through MovePinnedDockable; visible dockables remain empty.

Mutable native source tests prove actual backend identity and live mutation through
the original IList bridge, native control boxing/nulls, invalid element rejection,
unregistered-source behavior and a caller-registered generic item type. This is not automatic
arbitrary generic mutable discovery or complete adapter IList metadata/notification/
enumerator parity. Bounds cases include native descendant scrolling, explicit
container transforms and the complete scrollable extent. Auto-scroll cases cover
20px edges, 200px/s intensity, 16ms/fixed delta, clamp/reuse/replacement, NaN equality,
signed-zero math, cancellation, actual asynchronous native timer ticks and inactive
helper release after detach. Native Direct capture-loss events do not invoke the
source Tunnel subscription; registration behavior is preserved and the source
callback body is separately invoked with native arguments.

The complete original DockControl/DockFluentTheme/Model.Avalonia application, actual
tool/document tab and pinned subclasses, active ownership cycles and original
forced-GC leak tests remain pending. The internal helper has a temporary scoped
production dead-code allowance until those source consumers exist. No full docking
UI, desktop/exception performance or cross-platform parity is established. The
known nonfatal macOS debug compact-unwind linker warning remains unchanged.


## Native dock targets and floating adorner-window resources

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 1,044 passed; zero failed/ignored |
| Native target/adorner-window library | 16 Headless tests |
| Separate compiled consumer | Actual three source themes, five original images, operation selection and conditional/window styles |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Formatting, inventory, factory generator and six verifier tests | Passed |
| Source inventory | 1,928 inputs: 186 ported, 167 partial, 1,575 pending |
| Completion gate | Correctly rejects unfinished source inputs |
| Source C# and whitespace checks | Clean |

Tests cover every original headless DockTarget case, including all five same-
template operations and both local/global retemplate cases. Additional tests
exercise original virtual defaults/protected dictionary replacement, all inner/
outer direction gates, registration/last-valid precedence, visibility before
validation, missing/noncontrol names, indicator-only validation/hit bypass, native
reset interface, provider/theme inheritance/nulls, read-only margin notification,
accessibility/live state/Invoke and detached native release with a live window.
Native nested transforms and separate-window screen/client coordinates resolve
the original selector. The floating host uses actual native Window construction,
factory inheritance and decorative peer behavior.

The independent consumer loads the real source Fluent target dictionaries. All
five local selector images load with the original 40px dimensions and resolve
their operations. Directional/global and indicator-only visibility updates and
original floating-window defaults are verified. Asset SHA-256/byte comparison
confirms all five PNGs are identical to the frozen source assets.

Source forced CLR GC and full DockFluentTheme/leak application fixtures remain
pending. Exact reentrant dictionary enumeration version/removal-slot behavior,
managed/regular/floating adorner lifecycle through the real AdornerHelper,
DockControlState/controls, native strong owner/CLR metadata semantics and desktop/
mixed-DPI/pixel/performance checks remain partial. The known nonfatal macOS debug
compact-unwind linker warning remains unchanged.

## Native MDI layout and compiled application boundary

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 1,061 passed; zero failed/ignored |
| Native MDI layout/control API | 15 Headless tests and one flags metadata test |
| Separate compiled consumer | Default/new/shared/null managers, combined flags and original ReactiveUI document arrangement |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Formatting, inventory, factory generator and six verifier tests | Passed |
| Source inventory | 1,928 inputs: 186 ported, 172 partial, 1,570 pending |
| Completion gate | Correctly rejects unfinished source inputs |
| Source C# and whitespace checks | Clean |

All original ClassicMdiLayoutManager method bodies and MdiLayoutPanel bodies are
present. Tests distinguish stored model bounds from visual bounds, reserve the
minimized row, compress and stably order minimized windows, preserve valid model
bounds while clamping visuals, initialize invalid bounds with source cascades,
and maximize after the source bounds-write/state-reread sequence. All sixteen
resize edge combinations and unknown Rust flag bits, min/max anchoring, clamp-last
ordering, contradictory constraints and NaN/infinite/negative geometry are checked.
Z-order uses the source snapshot, moves only the first active member and retains
duplicates. Tests verify original model/control identity, custom managers, manager
property arrange invalidation, measure arguments and non-document filtering.

Observable getter tests verify two source state partitions, cached stable key
reads and effects of key callbacks on the second partition. A real .NET 10
OrderBy.ToList probe confirmed 0/0/2/3 key calls for 0/1/2/3 items; the native code
preserves the singleton optimization. A model getter mutating the original plain
input list produces the original fail-fast enumeration outcome. Entry constructor/
read-only property metadata, manager Instance field/constructor and all four
contract/class method invokers run through actual shared native descriptors.

The separate consumer compiles with the standard Build/Model scanner and loads
the original native panel through exported library metadata. It verifies distinct
new vs shared managers and null fallback, the combined Left/Top flag literal,
actual arrangement of canonical original dockable/MDI ReactiveUI document handles,
minimized model bounds remaining untouched and maximized visual dimensions.
Concrete reference nullable registrations are explicitly exported, allowing the
shared compiler to resolve null and interface conversion without a private parser.

Remaining API/runtime differences are tracked as partial: ListHandle is the
existing live original model abstraction but broader than IReadOnlyList; native
children do not expose source foreach invalidation; arbitrary raw interface/model
casts need application registration; the source invalid default entry/null argument
semantics and custom Equals behavior remain open; Classic.Instance is native
UI-thread-local; native flags metadata rejects unnamed numeric bits despite Rust
flag retention. The published flags descriptor macro emits an assign-operation
Clippy pattern; its allowance is confined to the enum/descriptor module. Full
managed hosts, MdiDocumentWindow, AdornerHelper, original DockControl/tab/pinned
controls/themes, ownership/GC and desktop/pixel/performance paths remain pending.
There is still no Rust docking sample executable. Existing macOS debug compact-
unwind linker warning remains unchanged.

## Original managed dockable state and compiled bindings

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 1,074 passed; zero failed/ignored |
| Managed base state/API | 12 native tests |
| Separate compiled consumer | Source property reads/notifications, two-way editing, rebind/old-model detachment and native Context identity |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Formatting, inventory, factory generator and six verifier tests | Passed |
| Source inventory | 1,928 inputs: 186 ported, 173 partial, 1,569 pending |
| Completion gate | Correctly rejects unfinished source inputs |
| Source C# and whitespace checks | Clean |

ManagedDockableBase preserves plain source notifications, source defaults and
unvalidated values for every dockable/selector/restriction property, strong owner/
original-owner/factory/context retention, recycling ID, lazy original tracking,
optional pinned bounds and every original virtual hook. Tests exercise equality
suppression (including NaN payloads and signed zero), exact notification names,
assign-before-event and store-before-callback order, equal repeated tracking writes,
finite-dimension-only pinned validation, nested updates, handler mutation snapshot
order and source failure propagation after assignment. A reentrant generic comparer
reads its field without a native storage borrow being held. Strong acyclic links
release after explicit clearing; no CLR cycle-collection claim is made.

The abstract native class has no default constructor and exposes all 37 properties
and 18 public methods. Actual descriptor getters/setters/method invokers and the
same original Event<str> are checked. Shared method invokers unwrap nullable return
values into their optional boxed result; the recycling ID test follows that native
convention. Context equality covers original scalar/geometry/canonical references,
native bridged strings/NaNs and the same native control boxed through different
base/derived handles. No alternate model state or independent type engine is used.

A separate compiled XAML consumer uses the original abstract base with a test
subclass. It proves string/boolean bindings, original property notifications,
TwoWay title changes, a new model context, old-model detachment and macro use
across a crate boundary. ManagedDockableBaseReference is a local reference adapter
holding the SAME Rc model and event; its own storage contains no model properties.
The local adapter is required by Rust's orphan rules for external System traits
on Rc. Canonical raw trait handles can use the exported concrete ValueTypes cast;
the shared metadata notification hook itself recognizes the adapter. Direct raw
trait-handle discovery remains partial and its conversion boundary is tested.

Still partial: generic Clone/PartialEq and cloned comparison-value/performance
semantics, automatic CallerMemberName, arbitrary raw custom Equals/interface
subtypes, undefined numeric enum bindings, invalid null strings, complete protected/
ref/event sender/args metadata and CLR cycle collection/threading. The consumer
covers selected source property types, not all typed bindings or original complete
managed-host application scenarios. ManagedWindowDock/ManagedDockWindowDocument,
managed hosting, adorner/control/theme integration and the runnable Rust sample
remain unfinished. Existing macOS compact-unwind linker warning remains unchanged.

## Original host cleanup, preview, drag and tool-menu helpers

| Check | Result |
|---|---|
| `cargo test --workspace --locked --quiet` | 1,089 passed; zero failed/ignored |
| New helper behavior | 15 tests, including native Headless controls and compiled property bindings |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| Formatting, inventory, factory generator and six verifier tests | Passed |
| Source inventory | 1,928 inputs: 189 ported, 177 partial, 1,562 pending |
| Completion gate | Correctly rejects unfinished source inputs |
| Source C# and whitespace checks | Clean |

All source bodies of DockWindowStateHelper, DragPreviewContext,
DragDockableResolver, HostWindowContentCleanup, ToolMenuLifetime and
VisualRootCompatExtensions are present in matching internal files. Actual callers
are still being ported; scoped production dead-code allowances remain on those
modules until the original consumers exist. There is no simplified host, document,
chrome or alternative docking engine.

Window state tests cover every accepted native/Dock/MDI variant and source
FullScreen-to-Normal MDI conversion. Preview state tests cover activation gating,
reference identity, strong retention and ordered clearing. All three original
resolver scenarios run through the production ReactiveUI factory/model engine;
additional checks span all eleven operations and absent/inactive/mismatched pinned
previews. Regular tool dock drags remain whole-dock operations. These are source
resolver counterparts, not the complete original DockControlState input harness.

Cleanup snapshots all visual model ContentControls before changes, discovers
original logical model presenters, walks visible/hidden/four pinned/pinned-dock/
split lists in source order, deduplicates model reference visits and clears in
reverse. Tests remove all visual descendants from a clearing callback and still
verify later snapshot entries; check nested/deferred ordering; traverse all nine
logical root/split locations and intentionally cyclic source layouts; and preserve
nonmodels, the host and presenters in another window. The native logical-model
fixture registers a real native class/core-interface conversion while retaining
the same original model. It is a test adapter, not the unported production property-
framework models.

A real compiled binding materializes DeferredContentControl before cleanup, which
clears content and explicitly releases its presenter child. Source local SetValue
preserves the binding; a later source update refills the still-attached presenter.
The regression verifies that refill and a repeated cleanup. Original framework
12.1.3 ValueStore.SetLocalValue and EffectiveValue.SetLocalValueAndRaise confirm the
behavior; no unbinding step is added:

- https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/PropertyStore/ValueStore.cs
- https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/PropertyStore/EffectiveValue%601.cs

Tool-menu tests cover original instance retention, null/repeated attach/detach,
replacement policy, a property callback replacing its button during detach, and a
compiled native-property binding surviving detach and reattachment through actual
UpdateTarget. Nullable root tests return the actual presentation-source RootVisual,
which is TopLevelHost in this published backend, and verify its loss after detach.

Still partial: source process-wide preview state versus UI-thread TLS; undefined
CLR numeric enum interop; native visual foreach mutation/version behavior;
arbitrary unregistered raw logical/interface/model identity; complete original
framework-property model/host-close/chrome/control fixtures and host/root/desktop/
mixed-DPI/lifetime/performance paths. All managed document/host/layer, adorner-state,
remaining docking controls/themes and runnable Rust sample work stays in scope.
The existing macOS debug compact-unwind linker warning remains unchanged.


## Native dock container/content control validation

The locked workspace run passes 1,106 Rust tests with zero failures or ignored tests
(1,089 previous tests plus sixteen native tests and one separate compiled consumer).
Locked workspace/all-target Clippy with warnings denied and formatting pass. Inventory,
factory generator and six Python inventory tests pass. The full-port gate deliberately
still fails: 1,928 source inputs, 197 ported, 181 partial and 1,550 pending. The unchanged
macOS compact-unwind debug linker warning is nonfatal; no suppression was added.

Native Headless tests check all eleven source classes, inheritance/markup/required
part metadata and ToolDockControl parent lookup. They exercise both metadata properties
through the standard interface descriptor, direct/resource/missing/non-theme cases,
string/type/theme-variant/scalar/explicit custom native resource keys, initial repeated
theme application, original factory/control identity, duplicate/context-switch registry
behavior, metadata getter reentrancy and detach cleanup. They do not claim the original
four complete generated ItemsSource/template scenarios, whose production models and
container generator are still pending.

SplitView tests cover all sixteen close-policy combinations, open synchronization,
source constructor/factory defaults, template replacement/missing parts, null contexts,
callback failure/guard reset and source post-detach handlers. Grid tests realize actual
ItemsPresenter/Grid, raise Loaded and verify native parsed definitions, null preservation,
empty clearing and columns-before-rows partial updates on parsing failure.

The separate compiled ContainerControlsConsumer.xaml resolves all eleven public native
classes and connects original reactive grid/split/document models and Factory. Its
minimal templates are explicitly test-only. Source full Fluent dictionaries, native
property models, managed host/adorner/control wiring, tabs/pinned items and an executable
Rust sample remain unverified and pending. Weak callbacks/CLR delegate lifetime, arbitrary
raw interface/custom resource keys and CLR exception interop remain explicit partials.


## Default dock data templates

Locked workspace tests: 1,112 passed, zero failed/ignored. All-target locked Clippy
with warnings denied, format, structural inventory, factory generator and six Python
inventory tests pass. Inventory is 197 ported, 182 partial and 1,549 pending; the
complete-port gate still fails. Native debug compact-unwind warning remains unchanged.

Six new Headless tests cover all fourteen templates, source order, repeatable lazy
construction, null acceptance, incorrect nonmatching builds, no recycling or manual
DataContext assignment, content-interface overlap/precedence and exclusion of ordinary
reactive Document/Tool models from content interfaces they do not implement. Tests check
all fourteen canonical template interfaces plus IDock/ISplitter aliases, original Rc
identity and notification-event identity. Managed model tests also verify that the
optional notification query returns the original managed event.

Live proportional bindings follow both original properties, preserve one-way behavior,
replace source subscriptions on DataContext changes and remove subscriptions on clear.
A real native ContentPresenter selects these templates, constructs the actual splitter,
sets the original model context and follows live changes; later content/grid replacements
select the correct native classes. Grid binding keeps its live subscription and original
shared cross-enum conversion failure/native Auto fallback.

The console-only [reference probe](../../../scripts/dock/template-reference/README.md) uses the
repository's original GridResizeDirection and original framework 12.1.3. It confirmed
that both Columns and Rows produce BindingNotification errors and generic reference
FuncDataTemplate null matching returns true. This agrees with original framework
[TypeUtilities](https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/Utilities/TypeUtilities.cs),
[DefaultValueConverter](https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/Data/Converters/DefaultValueConverter.cs)
and [generic FuncDataTemplate](https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Controls/Templates/FuncDataTemplate%601.cs).
The port deliberately adds no cross-enum converter. Exact error text/CLR exception
transport and arbitrary raw/nullable model discovery remain partial, as do full source
DockControl/default-theme consumers, platform/lifetime/performance and the Rust sample.


## Native drag-preview validation

Locked workspace run: 1,121 Rust tests passed, zero failed or ignored. Strict locked
all-target Clippy, formatting, inventory/generator and six Python tests pass. Source
inventory is 199 ported, 185 partial, 1,544 pending. The complete-port gate still fails;
macOS compact-unwind debug linker warning is unchanged.

Eight native tests validate original preview defaults/properties and real per-instance
recycling caches, derived construction/style/peer dispatch, nullable dimensions/content,
existing-peer-only notification, ordinal null/empty/case/whitespace comparisons,
reentrant callbacks and status/provider identity. Help/name priority and read-only
errors are checked even when disabled. Transient windows remain excluded from control
and content accessibility roles while shown. The two original AutomationPeersTests
preview cases are translated with source window sizes/content and provider/event checks.

A separate compiled consumer resolves the new public native types, creates properties
and ContentTemplate/PreviewContent, and checks live status/visibility/width/height/
content template bindings. The complete source DragPreviewWindow dictionary is loaded
through the theme assembly and every original setter is checked. The preview-control
template in this consumer is explicitly a test fixture; the full DragPreviewControl
source dictionary, native IDockable Title metadata, DragPreviewHelper and complete
managed/floating docking application are still pending. No sample executable is present.

The window name test originally expected the fallback before clearing the native
nonblank default title. The source explicitly preserves base names first; the test now
checks both that precedence and the empty-title fallback, with no production workaround.
Exact CLR exception types/string/lifetime and full desktop accessibility remain partial.

## Native dockable metadata and complete preview theme

Locked workspace run: 1,128 Rust tests passed, zero failed or ignored. Strict locked
all-target Clippy, formatting, inventory/generator checks and six Python tests pass.
The existing nonfatal macOS debug compact-unwind linker warning remains unchanged.
Inventory is 200 ported, 186 partial and 1,542 pending of 1,928 source inputs. The
complete-port gate intentionally fails; no runnable Rust docking sample exists yet.

UI-owned IDockable metadata exposes every original property and method, including
inherited recycling identity, through the published declarative metadata. Native tests
verify forwarding into the original model, reference/context identity, nullable reset,
geometry methods and borrowed original notification contracts. They detected missing
nullable IFactory conversion; concrete registrations now cover optional factory,
capability overrides, pinned display mode and bounds. The inherited interface cast
retains the same Rc. Explicit imports resolve the compiler's cast scan; the emitted
metadata has no unresolved cast. No replacement model or binding/runtime registry
was introduced.

A separate compiled consumer verifies live Title, two-way text edits, nested Owner.Id,
owner/model replacement and detachment from stale sources. The original notification
hook handles both canonical and nullable Rc references without recursive discovery.

The complete original DragPreviewControl dictionary is tested through a second real
compiled consumer with original accent/string resources. It verifies live model titles,
Dock/Float/None/custom status selectors, translated dynamic resource updates, both
preview-content branches, visibility, sizing and reuse of the same cached body across
content switches. The original NotNullConverter is consumed through x:Static Instance;
unit tests cover null, false, zero, empty strings, bool inversion, ordinal-case Not,
whitespace, unrelated parameters, ignored target/culture, ConvertBack failure and native
singleton/interface identity. UI-thread singleton, CLR exception/string/lifetime gaps
remain explicit partials. Managed preview/helper/host/adorner integration is still pending.

## Original selector dependency for DockControl

Locked workspace run: 1,135 Rust tests passed, zero failed or ignored. Strict locked
all-target Clippy, formatting, inventory/generator checks and six Python tests pass.
The known nonfatal macOS compact-unwind linker warning remains unchanged. Inventory
is 203 ported, 190 partial and 1,535 pending of 1,928 source inputs. The full-port gate
continues to fail as expected; no runnable Rust docking sample is claimed.

The original DockSelectorItem/Mode/Service contracts, native DockSelectorOverlay,
complete automation peer and complete Fluent dictionary are now present. Four native
tests cover captured selector-title fallback/flags/model identity, original defaults,
provider contracts and missing-part fallbacks, notification ordering/name suppression,
and native service metadata dispatch. Two domain tests cover array/custom/observable
IReadOnlyList reference identity, delegated custom indexer/enumeration, direct typed
notification forwarding/disposal and lifetime. The shared collection contract lives
beside the existing IList adapter because System 0.3.0 lacks IReadOnlyList. Observable
mutation-version/fail-fast enumeration and CLR runtime details remain explicit gaps.

A separate compiled consumer uses the full original theme and actual ListBox. It checks
all five badges, title rendering, original selection and container peers, live source
add/remove/replace, nullable/replaced Items and stale-source detachment. The test caught
a real adapter error: native bindings can expose a model itself while the typed list
boxes its Rc handle. SelectorItems now uses shared from_markup_value plus original
reference identity for typed index lookup, matching native collection adapters. No
replacement selection, command or binding engine was introduced.

The Dock peer delegates the source selection/scroll providers. Published FerroUI's
ItemsControlAutomationPeer exposes scroller discovery separately from get_provider;
without that query its scroll provider retains fallback state. The consumer compares
those initial delegated values, explicitly initializes the shared scroller hook, then
verifies actual scroll behavior. Ordinary desktop automation-client discovery/parity
remains partial; the test does not establish screen-reader compatibility. The original
DockControl keyboard navigation, managed hosting and full application remain pending.


## Original command-bar models, merge manager and native host

Locked workspace run: **1,150 Rust tests passed**, zero failed or ignored. Strict
locked all-target Clippy, formatting, structural inventory, factory generator and six
Python tests pass. The existing nonfatal macOS compact-unwind linker warning persists.
Inventory is **205 ported, 198 partial, 1,525 pending** of 1,928 source inputs.
The full completion gate still fails as expected; no runnable Rust docking sample
or complete engine/API parity is claimed.

Fifteen new tests verify original model defaults, all mutable/nullable fields and
reference identity; native foreign-enum descriptors and exports; full merge precedence,
stable ordering, append fallback, last-group insertion and shallow copies; one-level
active scope, recursive document and split-pane resolution; direct provider priority
and canonical/native context discovery; original factory/provider events and provider
switching, disable/reenable semantics, same-layout factory subscription capture,
explicit Detach release while dependencies remain alive and reentrant adapter detach.
Native menu recursion, separators, source ordering, toolbar content composition,
content-control priority and ribbon header-only behavior use actual controls.
Commands preserve arbitrary model parameter identity through the published ReactiveUI
adapter, including CanExecute changes observed by the original command.

The downstream compiled consumer loads the complete original Fluent host dictionary,
renders all three source ItemsControls using original control handles, invokes the
actual toolbar button through its native automation provider, observes command/parameter
execution and verifies nullable list clearing updates the template. Selector regression
tests pass with the shared typed IReadOnlyList projection. Native host peer tests cover
original role, names and live state counts; model/provider metadata tests cover original
properties/method and event dispatch. Direct helper disposal and System token removal
are distinct from the add-only native MarkupEvent surface, which discards subscriber
return values. Arbitrary original EventHandler sender/EventArgs identity, markup -=,
raw domain Any metadata, CLR garbage collection, full desktop input/automation/lifetime
behavior and integration into the full DockControl remain explicit partial work.


## Pinned overlay window and original tab-strip wheel dependencies

Locked workspace: **1,156 Rust tests passed**, zero failed or ignored. Strict locked
all-target Clippy, formatting, structural inventory, factory generator and six Python
tests pass. Inventory: **208 ported, 201 partial, 1,519 pending** of 1,928 inputs.
The known nonfatal macOS compact-unwind linker warning remains unchanged. Full-port
completion still fails; no runnable Rust docking sample is claimed.

A separate compiled root-window consumer loads the complete source PinnedDockWindow
Fluent theme and checks every original setter and explicit dimensions. Its native
WindowAutomationPeer retains original role, title/fallback/explicit naming, live state
text and IInvokeProvider Show-if-hidden/Activate/Focus sequence. Tests cover visible
and hidden-again invocation, activation, focus with Focusable enabled and a derived
window's inherited style key/peer. The source calls Focus() without checking its result;
a default nonfocusable Window need not become focused. Desktop focus/automation parity
and the full PinnedDockControl consumer remain pending.

Five native ScrollViewer Headless tests cover both original scroll helpers: no viewer,
zero/signed-zero/smallest-subnormal rejection, just-above-threshold movement, no overflow,
clamped edges, both axes/directions, fractional ceiling steps, ignored Delta.X,
already-handled events, movement-only handling, orientation replacement via explicit
disposal, detach/reattach and release after removing the strong event capture. Real
child routing verifies presenter consumption precedes Dock's bubbling hook: vertical
movement is handled natively when possible; horizontal-only overflow reaches Dock.
No tunnel interception, iteration cap or replacement scroll engine was introduced.
A local .NET 10/Avalonia 12.1.3 runtime probe additionally verified NaN-to-zero and
positive overflow/infinity-to-Int32.MaxValue step casts and Vector NaN inequality.
Rust's saturating casts/native Vector equality preserve those observed semantics.
The seven full original tab-strip/template tests remain explicitly pending; these
are helper-level counterparts. CLR collection of unreachable strong cycles and full
application/desktop input behavior are unverified.


## Managed floating-window document and dock models

Locked workspace: **1,172 Rust tests passed**, zero failed or ignored. Strict locked
all-target Clippy, formatting, structural inventory, factory generator and six Python
tests pass. Inventory: **208 ported, 204 partial, 1,516 pending** of 1,928 source inputs.
A terminal disk-full build was followed by cleaning only FerroDock cargo artifacts and
rerunning the full suite. Published dependencies and original C# files remain unchanged;
ferroui-markup-xaml 0.3.0 is now a direct dependency of the UI crate. The existing nonfatal
macOS compact-unwind warning persists. The full completion gate still fails and there
is no runnable Rust docking sample.

Fifteen new native tests exercise all original managed document/dock branches: defaults,
identity/null values, focused title updates/replacement and stale-source detach, layout
replacement, full-screen mapping, geometry, ordinary and managed-specific close paths,
explicit Dispose releasing the document while real window/root/focus/factory remain
alive, supported and unsupported parent detach/fallback order, template results,
recursive delegates and self-return guard, preview visual brushes, stable no-op commands,
add/remove and last-item selection, owner/factory retention, mutable field/focus events,
and actual Factory window/dockable activation order. Existing reactive window tests also
verify the optional notifier query for real and composed models and default host query.
Managed-host-specific dispatch uses a test-only contract probe; the concrete managed host
is still pending. This is not an implemented managed presentation/window-drag engine.

The downstream compiled consumer reads/writes inherited Title, updates own MdiZIndex,
uses the original MdiDocumentWindow deferred/recycling content fragment and shared
TemplateContent loader, and rebinds between canonical IDockable and IMdiDocument handles.
It verifies live updates after both bindings and compiled deferred body materialization.
Native checked casts preserve original Rc identities, reject unrelated models and allow
application-owned recycling templates through the existing interface-query extension.
UI-owned MDI interface metadata exposes all three original properties and borrows the
original System notifier. No additional model, event system or binding engine was added.

Published 0.3.0 optimized typed metadata does not notify these Rc model fields. Documented
fallible-setter metadata selects the native compiled INPC accessor while original setters
remain intact. Shared static is_assignable treats conditional opaque-interface cast
registrations as type-wide membership; the managed document Match tests actual values
for these targets. General native template matching on opaque aliases, raw CLR object/
Type/UTF-16/GC and cross-thread behavior, nested window metadata and remaining enum/struct
markup, original framework-property model variants, full MDI/host/layer/control integration
and application/desktop parity remain explicit partial work.


## MDI document window, peer and activation helpers

Locked full workspace: **1,188 Rust tests passed**, zero failed or ignored. Strict locked
all-target Clippy, formatting, structural inventory, factory generator and six Python
tests pass. Inventory: **209 ported, 211 partial, 1,508 pending** of 1,928 inputs.
Original C# and published dependency files remain unchanged. The known nonfatal macOS
compact-unwind linker warning remains. Full completion still fails; no runnable Rust
sample or complete MDI theme integration is claimed.

Sixteen new native Headless tests validate the actual MdiDocumentWindow class, its
properties/defaults, native namespace and pseudo-states, document/dock subscriptions,
live layout invalidation, context replacement/stale detach, explicit subscription
removal and native object release while model/dock/factory/window remain alive.
They exercise original minimize/restore/double-tap, content/header input, original
production Factory activation/focus/Z order, all eight resize directions, drag guards,
pointer capture/release and state changes, custom layout-manager dispatch, adjacent
window magnetism, invalid bounds, reserved minimized strip and clamp, retemplating and
content offset, automation role/title/ID/live help/invoke/focus fallback. Native private
helper calls are permitted in Rust child tests; no reflection is introduced.

Managed cancellation and begin/move/end/screen-coordinate dispatch use the production
Factory and an explicitly test-only ManagedHostWindowAccess probe. Replacement factory
and missing-factory branches are verified. WindowActivationHelper is exercised with
real visible/hidden native Windows and stable managed Z-order probes. The complete
original HostWindow/DockControl/framework-model fixture and desktop order are pending.
WindowDragHelper has only its original child-type walk; remaining methods are pending.

The original CaptureLost registration requests Tunnel; FerroUI raises that event as
Direct, so actual native loss does not invoke this handler. Tests preserve that routing
and separately call the original handler body. Normal released-event cleanup is verified.
DoubleTapped itself bubbles and invokes the source's handler once. No route correction,
replacement pointer state machine or silent behavior change was introduced. Original
runtime comparisons and full desktop input remain to be completed. Strong template
handler cycles are retained as in the source; the untemplated native release test does
not establish CLR collection of unreachable full-template cycles.

The downstream compiled managed consumer now additionally constructs the actual MDI
window, resolves shared template properties, binds original MdiState in both directions,
and observes pseudo-states. Existing deferred/recycling tests remain intact. Original
MdiWindowState metadata is generated by the existing Settings bridge; runtime enum names
and native model identity are retained. Full source MDI dictionaries need DockableControl,
MdiDocumentControl, converters and remaining nested dock/factory metadata. The consumer
is not the complete original MDI theme or a runnable docking sample.


## MDI host control and managed-tool converters

Locked workspace: **1,203 Rust tests passed**, zero failed or ignored. Strict locked
all-target Clippy, formatting, structural inventory, factory generator and six Python
tests pass. Inventory: **209 ported, 215 partial, 1,504 pending** of 1,928 inputs.
Original C# and published dependencies remain unchanged. The known nonfatal macOS
compact-unwind warning remains. Full completion still fails; there is no runnable Rust
sample or complete MDI dictionary integration.

Ten new native Headless tests cover every host property/default, read-only direct state,
pseudo-state, notifying/plain/custom list behavior, membership including Tool, visibility
notification suppression, stale-source detach, collection/context replacement, custom
manager order/active member and exact update triggers, null lists, callback reentry and
unsubscribing/release while dock/collection/factory/window remain alive. Native enumerators
prove Any stops and disposes before visibility callbacks and Z-order materializes before
manager dispatch, without an extra observable Count read. Original automation Pane role,
name/ID/help fields, real Factory/provider invoke, first-any-dockable fallback and focus
fallback are verified. The full source-theme/framework-property lifecycle fixture and
CLR unreachable-template-cycle collection remain pending.

Three converter tests cover both source classes using actual managed documents, window,
root and tool/document docks. They verify bool/ordinal Not inversion, ignored culture and
target, null/unrelated inputs, source ConvertBack failure via BindingError, same tool
identity, null layout/disposed document, constructors/metadata/interfaces and shared
UI-thread singleton. Canonical IDockable, IMdiDocument and managed-base references resolve
the original model using native casts and existing boxed-reference projection.

Two downstream compiled tests verify all MDI host template/manager properties, native
readonly-state/empty-visibility binding, original dock notifications and rebinding to
canonical IDock; managed converter x:Static fields, inversion and original tool identity
work through the shared compiler/runtime. These untemplated API consumers do not establish
complete original MDI dictionaries, ItemsControl layout, EmptyContent rendering, all four
MdiEmptyContentTests or an interactive docking sample.

The host peer's original runtime manager name uses existing native descriptors plus a
compiler-supplied concrete-name fallback. Ordinary registered/custom manager names are
verified; exact CLR names for unregistered generic/nested types remain partial. No runtime
reflection, copied docking state, replacement manager or private binding runtime is added.
UI-thread static/cross-thread, exact CLR exception transport and UTF-16 behavior remain
shared gaps. Full IDock/IDocumentDock/IToolDock/factory/capability/list metadata and
DockableControl/tracking/managed-layer dependencies still precede full MDI theme rendering.
