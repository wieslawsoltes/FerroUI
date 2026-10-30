# FerroDock

FerroDock is the Rust port of Dock for FerroUI, preserving the original docking
engine, contracts, model identities and public markup names. The original C#
source in the parent repository remains the reference implementation.

The current delivery target is core docking, standard JSON layout persistence, Fluent and
Browser tabs, with their required support working. See [delivery scope](docs/porting/dock/DELIVERY-SCOPE.md)
for acceptance and remaining work. Additional backends, serializers and unrelated samples
are deferred; the full source-file inventory is not a completion percentage for this target.

Standard core layout JSON serialization is now available through
`ferrodock_serializer_system_text_json_reactiveui::create_serializer()`. Its catalog uses
the original14 layout classes and DockWindow, with source-matched wire/graph/stream tests.
Restored layouts retain native tab/pin interaction under Fluent and Browser. See
[serialization usage and limits](docs/porting/dock/SERIALIZATION.md#standard-reactiveui-core-catalog).
The sample source now has a Fluent/Browser Tabs selector, tested with live panel changes,
original model identities, native tab clicks and presets/light-dark. The refreshed macOS
sample has been checked with both tab themes. All12 sample application models now have explicit JSON registrations; injected save/load
methods restore their actual classes, contexts, factory locators and original commands.
The File menu now provides Open Layout and Save Layout As through ReactiveUI commands
and native storage services, with cancellation, busy gating and status feedback. The
standard serializer stream API handles file encoding. Compiled menus restore actual
views under both themes in Headless tests. End-to-end menu click tests also run the
actual native storage adapter with controlled platform picker results and real JSON files.
Native macOS Save/Open also pass with an actual saved layout; cancellation and malformed
JSON leave the current layout intact.
Composed pointer tests also move tabs between panes, float into actual themed HostWindows
and redock across windows after JSON restoration. The floating graph roundtrips exactly
and empty hosts/registry entries close cleanly. Rendered selector splitting, actual splitter resizing, tab reorder, pin/preview/unpin,
hide/restore and close flows now also pass under both themes after JSON reload. The
desktop pass additionally verifies grip stretching, pane resizing, pinned previews,
document close, F11/F9 and light/dark Browser resources. Physical mouse reordering and
tool floating/docking were confirmed by the user; see the acceptance notes for details.

**The original ReactiveUI sample now runs as a native Rust application.** Its
compiled App, window and views use the original factory, three-document/eight-tool
layout, native DockControl, Fluent resources, tabs and pinned controls. Theme actions
run through ReactiveUI commands and behaviors; views remain passive. Fifteen original
class resources compile, including App. Desktop and single-view startup paths are
covered in Headless. The refreshed native macOS window has been visually checked,
including document/tool selection, pinning, opening the pinned host and unpinning.

The UI-independent Core SDK now owns 23 original Model.Controls descriptors, including
root/document/tool docks, MDI, layout docks and document/tool content templates.
Inspectors and downstream XAML resolve them from `FerroDock.Model`. Original lists,
notifications and Add commands retain source model identity and docking behavior.
The original DocumentFactory property and native Func invocation now retain the
actual callback and drive the original CreateDocument engine. All fourteen original
template-required interface attributes are exported. Document and locator callbacks
now share the same source Rc callable, preserving identity directly. Null callback
results reach the original document/tool virtual methods; no-Factory calls and
null-aware document/tool and factory overrides are covered in source and compiled/runtime
consumer tests. Factory initialization and activation now accept runtime null references
through their original virtual slots, with default failures at the source dereference.
The temporary runtime callback adapter/cache has been removed. Nullable source list
elements now travel through the original shared collection and lazy query engine.
Nullable protected-state hooks, root lookup and pin membership now preserve source
override/failure order. Insertion/removal and protected pin item arguments also retain
null through the original virtual methods; whole-pane floating sends nullable snapshot
entries through Remove, Add and moved events in source order. AddDockable's nullable
target preserves initialization side effects before list access. Protected list/selection
helpers and the explicit pin root preserve source no-op, lookup and bounds/failure order.
Move/swap overloads now keep null entries in the original lists and preserve mutation,
event, state synchronization and virtual Init order. Pin/unpin and capability resolver
inputs also retain the original nullable dispatch and default access failures.
Pinned preview/hide now preserves nullable fixed dispatch, partial cleanup and live
preview reentry: a cleared preview reaches the actual nullable Init override before
selection rereads the live root. Original cached preview/list identities are retained.
Hidden-item hide/restore now carries null through original virtual slots, preserving
pin cleanup, root relookup, partial list commits and observer-directed destinations.
ID helpers still fail at the first source null.Id instead of skipping entries.
Floating/window overloads now retain nullable dispatch, ownership preparation before
creation and original options/parent/result identities. Plain null creation returns
no window, while default floating keeps its original operation-context failure.
Focus and active-selection owner inputs now retain nullable virtual dispatch, original
initialization/OnSelected/policy order and final sibling-selection failure timing.
Fixed window activation rereads the live root window after host callbacks; events keep
the actual cleared or replacement window. Floating focus forwards the live optional
layout to the original override. Window-option application now keeps the original
null-options guard and null-window argument rejection. Setter callbacks can change later
option reads, including parent, modality and taskbar visibility, on the same objects.
Windows now retain actual null entries in the original shared collection across models,
factory list creation, native metadata, compiled/runtime bindings and ItemsSource. Window
registration and initialization keep committed entries/events before source failures,
optional host creation and live layout argument order. The host-factory interface is
also exported through the Core SDK.
Search and lazy queries now accept nullable roots and predicate callables through the
original factory and native metadata. Live window layouts preserve callback/read order.
The model query now owns shared advancement, Current and explicit disposal state; native
cursors forward to that same engine. Predicate disposal follows the source active scopes,
including ignored root disposal, cleared child/nested fields, null yields and later access
failures. Thirty-two original traces also cover repeated disposal and recursive MoveNext.
Remaining operation/service arguments, the public attribute class, complete generic
collection/delegate metadata and CLR/source-engine edge semantics remain tracked.

Serializer stream contracts now retain actual nullable shared handles, capabilities and
explicit close/dispose behavior. The memory stream and native I/O adapter preserve the
original backing buffer and position across aliases. Thirty-seven source stream/interface
checks have matching Rust coverage. The resolver-based SystemTextJson serializer now implements all four generic operations
with compiled typed metadata, source class/cycle/property behavior and shared stream
closure/error precedence. Thirteen original JSON cases, twelve failure cases and source
wire goldens have matching Rust coverage. Compiled reference casts, registered polymorphism,
base fallback and configured list creators now roundtrip a limited projection of actual
ReactiveUI models through the original engine. A standard ReactiveUI backend catalog is now available; automatic application class/cast
discovery and the complete general generator remain pending.
Owning model links match source reachability, but Rc cycle collection remains unresolved.
Scalar metadata now includes Single/Double and all eight integer widths, checked against
1,362 original C# numeric records. The model projection also retains proportion, constraint
and floating-window bounds, including source NaN/infinity and signed-zero behavior.
Explicit Core metadata now preserves DockRect values, nullable capability policies and
overrides, including shared base/interface references. Pinned bounds, six base flags and
aliased root/dock policies roundtrip through actual models and generated contexts; loaded
capability precedence uses the original engine. Ten original assertions, 49 read cases and
six Core goldens cover this scope. Default and sample persistence remain unfinished.
Core metadata also covers get-only DockPoint, shared DockManagerOptions and three signed
flags types. Ten further source assertions, 490 read cases and four goldens preserve default
construction, unknown bits, numeric token/range rules and independence from custom integer
codecs. Actual model projections persist DockingState and concrete operation masks through
the original engine. Window owner/state enums now retain all signed values through safe four-byte types,
original named constants and native runtime conversion. Window options and all 13 persisted
window properties are covered by source-matched direct/generated contexts, including shared
parents and unknown values. Seven additional source assertions, 282 value/name/wire rows and
50 read cases validate this scope. Explicit Core SDK numeric literals now compile through the shared model and runtime XAML
emitter, preserving source nullable values, signed endpoints and frontend narrowing. Other
plain enums, complete compiler discovery and full catalogs remain unfinished.
The supplied direct/emitted model contexts now also persist Dock, nullable pinned-display
overrides and root pinned/host modes in original wire order. Six more source assertions
(44 to50) retain undefined values through original factory initialization. These remain
four-model projections; complete catalogs and actual sample persistence remain unfinished.
DockMode, PinnedDockDisplayMode and DockFloatingWindowHostMode now preserve signed
values through the same actual domain types, runtime/compiled metadata and six explicit
JSON descriptors. Original probes supply 423 value/name/wire rows and 102 read cases.
Compiled root/options consumers and source host-mode precedence/converter fallback match.
The generator's object-payload helper runtime now supports compiled payload registrations,
property-local explicit/read-only converters and source independent reference scopes.
Twenty source assertions, 25 read/replay cases and eight wire goldens come from the
unchanged original generator's emitted resolver. Rust compiler discovery/emission and
default catalogs remain incomplete.
The compile-time semantic generator pass now matches 19 original compiler fixtures for
registration/discovery, ignored members, diagnostics, context naming and subtype plans,
plus 152 runtime full-name checks. Its Rust emitter now compiles actual model and payload
consumers, matching the recorded wire output and running the existing factory after load.
Policy metadata supports exact class dispatch, base/nearest fallback and ambiguity failures.
Native view bindings can now project callbacks onto their original model handles with
explicit receiver/result conversions. Constructors and error identities are preserved,
with no copied state or cached model view. Independent compiled consumers verify original
wire/aliases, distinct live instances and constructor call count; full native schema and
inheritance discovery remain unfinished.
Typed JSON property callbacks now propagate accessor failures without replacing error
handles or undoing earlier setter changes. Native metadata and generated consumers support
Result getters/setters, mixed signatures and explicit foreign-error maps. Seventeen original
source cases verify order, skip behavior and persistent callback state; full native model
catalogs and sample persistence remain unfinished.
A separate build-time adapter now binds shared declaration metadata to typed accessors,
constructors, renamed/get-only/set-only properties and explicit converters. Seven application
payload classes compile through that path. Bindings remain an explicit limited projection;
complete syntax/semantic adaptation and value/constructor/attribute coverage are still required
for the default catalog.
Crate-scope activation and registration macros now feed actual Rust declarations into the
semantic generator through the shared scanner, including renamed imports, closed type
expressions and source locations. The payload consumer reads its nine registrations from
compiled Rust source. The adapter now reads actual Rust declaration visibility, generic parameters, trait
relationships, conditions and locations, canonicalizing known public reexports through
shared source/dependency metadata. The consumer takes visibility, arity and locations
from these facts. Seven ordinary payload class headers/property schemas now construct fresh semantic symbols
from native declarations. Logical serialization identity is explicit and separate from markup
aliases. Closed generic payloads now specialize a shared template and build native symbols and typed
accessor bindings too. All nine payload schemas use native construction. Native batch allocation
now supplies application IDs and metadata-declared source discovery;
the C# application schemas/slots/discovery entries are discarded. In the payload consumer,
the six required Dock contract headers and all 81 declared property schemas now build from
native SDK metadata, including interface inheritance and command ignores. Atomic dependency
import now allocates fresh IDs for those six contracts, the actual locked ReactiveUI
ICommand declaration and five actual model enum declarations. Their old schemas and slots
are discarded; incoming properties and nullable dependencies remap to the native entries.
Original enum inheritance retains its explicit foreign System.Enum dependency.
Other system and ancillary type facts remain explicit C# reference inputs. Complete native dependency
compilation, discovery for all source forms and production catalogs remain open.
See [serializer requirements](docs/porting/dock/SERIALIZATION.md).

The full port remains in progress. The locked workspace passes **1,967 Rust tests**,
including 373 native UI, 204 downstream markup, 49 diagnostics and 44 sample tests.
The inventory records **227 ported, 465 partial and 1,236 pending** of 1,928 files.
These counts describe the full upstream inventory; they are not a delivery percentage
for the accepted core docking scope.
Seven actual SDK class schemas now compile from native declarations, retaining101 property
types/order and24 ignore annotations plus actual base order and original interface precedence.
Their old C# class schemas are discarded; foreign dependency facts remain explicit inputs.
Native SDK JSON binding now compiles original constructors, enclosing model identity and
all92 properties in the existing four-model projection through actual SDK accessors. Source
wire graphs, aliases/cycles and factory initialization remain intact. DockableBase metadata
also exposes operation masks and selector properties. The standard core docking JSON catalog and sample File menu integration now work in
automated tests; desktop acceptance remains the delivery priority. See the accepted scope above.
Window owner/state retain numeric runtime/JSON behavior and native source provenance;
full inherited APIs and catalogs remain partial. DockMode, PinnedDockDisplayMode and
DockFloatingWindowHostMode now also retain unnamed numeric values, native compiled metadata
and numeric JSON; their inherited APIs and automatic catalogs remain partial. Pure enum declarations are audited even when metadata/test targets are added.
Docking-state markup now retains the original engine flags type for named values,
combinations and static references. These checks establish the recorded scenarios,
not complete API/platform/lifetime parity. The original-engine C# reference passes
914 assertions, and all 4,428 tests in the retained controls package pass, including
the native nullable-item regressions.

Shared FerroUI dependencies use version `=0.3.0`. Self-contained support changes to
the published base, markup, runtime compiler, build compiler and scanner packages supply resource interfaces, nullable
interface conversion and arbitrary-object namescopes required by the original APIs. Its exact provenance and
integrity guard live under `vendor/` and `scripts/dock/check_framework_patch.py`; no sibling
checkout is required or modified. The markup package also checks actual conditional casts during template matching.
All six retained packages have exact original/modified provenance. The controls patch
uses shared native value conversion for nullable item boxing, unboxing and equality;
list slots and change payloads retain actual null or their original model reference. The base patch
also restores declared interface handles when weak binding sources are read. Shared
markup setters unwrap nullable/object transport boxes before assigning object properties.
Domain/reactive models remain UI independent.

The Simple theme library is now available alongside Fluent. It preserves the original
42-entry root, all 56 class/resource documents, five presets, compact density and
cached-document API. Shared control templates and images are compiled into its own
assembly, and its normal dependencies include neither Fluent theme library. Headless
checks exercise the original engine graph with FerroUI's Simple theme; broad native
input/platform and full type-system parity remain open. The original sample continues
to use its original Fluent configuration.

The Browser theme library now preserves the original Fluent overlay and all five
source documents, including the intentionally empty compact dictionary. Its delayed
tab panel uses the original layout/debounce logic and native timers/transitions;
24 layouts are checked against the unchanged C# control. Native construction also
initializes derived panel and theme classes correctly. Compiled consumer tests cover
actual pointer selection, compact tab states and the Browser floating-host override.

The diagnostics library now contains the original selection adorner and helper. Its
native tests compare 216 factory-map combinations with the unchanged C# helper and
cover live geometry, ownership, cleanup and reentry. The original debug overlay now
preserves its adorner/helper/manager and both extension overloads. Its 16 render cases
match unchanged C# output, and the DEBUG sample attaches the original F9 toggle. Native
macOS checks verify outlines, tab input through the overlay and F9 removal. The 18 passive inspector property views now compile as ordinary library classes,
with native bindings to the original model interfaces. An independent unchanged C#
reference verifies all 18 grids, 24 enum edits and 26 event messages. The main RootDockDebug inspector
now supports F11 in the DEBUG sample, all five tabs, live trees, 26 event types,
filter/clear and selection highlighting. Views remain passive. The user-reported
header grip now spans spare width in Fluent and Simple; the unchanged C# reference
also reproduces the original 12-pixel grip. This correction is recorded explicitly.

The original factory focus helpers and Load/Reference markup extensions are now
available through the existing engine and native markup services. Independent C# and
native tests cover original behavior, URI resolution and reference identity. Plain models now share the native namescope with controls, including compiled named
references and live engine title bindings. Declared plain-model roots now compile/load with one constructor instance. External metadata carriers now preserve actual model return types, and loaded policy/
override roots feed the original resolver. Remaining concrete constructor exports, logical
assembly metadata and the remaining CLR/lifetime scope remain open.

The separate `ferrodock-model-reactiveui-ferro` integration library owns the
ReactiveUI model class metadata. Its Document, Tool, DocumentDock and RootDock
constructors create the original engine objects, using the logical assembly name
`FerroDock.Model.ReactiveUI`. Applications register both the general controls and
this backend SDK in their composition root:

```rust
ferrodock_ferro::register_types();
ferrodock_model_reactiveui_ferro::register_types();
```

ToolDock, all seven layout docks and both splitters now have the same constructor
and class metadata, covering all 14 original backend Controls classes. Their source
properties, inherited state, notifications and methods stay on the original objects.
ToolDock.AddTool runs through the original factory; SplitView pane actions and
reference assignments use the shared metadata/conversion pipeline.

The shared `ferrodock-model-ferro` library now owns 23 original model enum/flags
descriptors under the logical `FerroDock.Model` assembly, including the two SplitView
enums. Settings keeps aliases to those same descriptors. Domain remains UI independent;
the model metadata library uses native base/System services and the domain types.

Factory and DockWindow now also have native constructors in their original logical
model assembly. They retain the original factory, window, registries and inherited
APIs. Compiled bindings update the same source objects, including live HostWindows
counts. FactoryBase remains abstract; its exported creation, initialization and close
operations dispatch through the original engine. Generic collection and remaining FactoryBase metadata are still incomplete.

All 28 factory events now have native metadata and source callback methods. Their
shared argument objects preserve cancellation, context changes and sender identity;
a compiled application handler cancels a real engine close. The Core SDK also owns
all 29 event-argument descriptors and the weak tracking snapshot. Recycling and model
metadata share the existing object bridge through the base-only `ferrodock-runtime`
package. CLR EventArgs base casts and general metadata remove accessors remain open.

The general control library has no normal dependency on the concrete ReactiveUI
backend. Rust class-reference helpers now live under
`ferrodock_model_reactiveui_ferro::reactive_metadata`; engine APIs remain in
`ferrodock_model_reactiveui`. Model/runtime XAML compilation, fresh loads, engine
ownership/close and live TwoWay bindings are covered. Remaining model constructor
exports and logical assembly/CLR contracts are still open.

The Core SDK now owns the original IFactory contract and FactoryExtensions metadata;
controls retain aliases to the same descriptors. Factory locator properties, tracking,
creators and lookup methods operate on the original engine. Delegate/dictionary views
retain original objects, including stored null delegates and absent-key fallback rules.
Compiled source-style delegate declarations work through the shared scanner's checked-
cast records. Generic methods, complete collection/delegate metadata and broader
CLR/platform behavior remain open.

Factory metadata now also exposes original mutation, activation, window and split
operations, plus FindRoot/FindDockable/Find. Source-style method predicates remain
deferred, and query views use the original lazy engine iterators with independent
cursors. Compiled Core command bindings update actual window tracking. Optional source
arguments currently use arity adapters; generic methods, complete collection/CLR
metadata and wider lifetime/platform parity remain incomplete.

Core now also owns capability policy classes/contracts and dock-window metadata,
with legacy Rust aliases retained by controls. Window options, immutable capability
evaluations and the capability resolver expose their original constructors, properties
and methods. Independent Core-qualified XAML consumers verify original engine objects,
policy precedence, live parent-window bindings and TwoWay option edits. Inspector
templates now reference the migrated Core contracts through their original assembly. Serialization and wider
Core/CLR ownership and lifetime/platform scope remain incomplete.

Core also owns IDockable, IDock and the typed dockable-list metadata. The independent
`ferrodock-controls-recycling-model-ferro` SDK publishes the original recycling Model
contracts without general controls. Compiled Core bindings retain live original lists,
source navigation commands and native control Context identity. The shared runtime
object bridge supplies conversions; UI registration retains raw Control projection.
Native out-parameter metadata and complete generic collection contracts remain open.

Core now owns target markers and generated-container metadata, and publishes the
original host-window properties and supported methods. Compiled Core bindings operate
on the existing native host and shared model. Its position/size out-parameter metadata
and general host GC-cycle collection remain incomplete.

In the sample, use **File → Save Layout As…** to save a JSON layout, and
**File → Open Layout…** to restore it. The status bar reports the result; cancelling
a dialog or failing to read/parse a file leaves the current layout intact. The **Tabs**
selector switches Fluent/Browser. Debug builds expose F11 for the docking inspector
and F9 for the docking overlay.

Build and run the desktop sample:

```sh
export TMPDIR=/Volumes/SSD/temp/ferrodock-port
export CARGO_TARGET_DIR=/Volumes/SSD/temp/ferrodock-port/desktop-file-workflow-target
cargo run --locked -j1 -p ferrodock-reactiveui-sample --bin ferrodock-reactiveui-sample
```

On macOS, package and open the native application instead:

```sh
export TMPDIR=/Volumes/SSD/temp/ferrodock-port
export CARGO_TARGET_DIR=/Volumes/SSD/temp/ferrodock-port/desktop-file-workflow-target
cargo build --locked -j1 -p ferrodock-reactiveui-sample --bin ferrodock-reactiveui-sample
python3 scripts/dock/bundle_macos_sample.py \
  --binary "$CARGO_TARGET_DIR/debug/ferrodock-reactiveui-sample" \
  --output "/Volumes/SSD/temp/ferrodock-port/FerroDock.app"
open /Volumes/SSD/temp/ferrodock-port/FerroDock.app
```

The helper physically copies the executable into the bundle. `--output` selects another
location. The current verified native launch uses `FerroDock Core Review.app` on the SSD.
Direct terminal launch previously failed in FerroUI's native render timer on this machine.
Other platforms have not been verified. The library remains available with
`--no-default-features` for consumers that do not need desktop startup.

From this directory:

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --release -p ferrodock-ferro --lib -- -D warnings
cargo fmt --all -- --check
python3 scripts/dock/check_port.py
python3 scripts/dock/check_framework_patch.py
python3 scripts/dock/check_theme_links.py
python3 scripts/dock/generate_factory.py --check
python3 -m unittest discover -s scripts -p 'test_*.py'
```

The structural completion check, `python3 scripts/dock/check_port.py --require-complete`,
correctly rejects this unfinished port. Production serializers, other model
frameworks, remaining theme/application units and broad original input, lifecycle,
close, accessibility and desktop scenarios remain open. Native callback/delegate,
weak interface and CLR GC/ephemeron lifetime differences are tracked explicitly.

See [the porting contract](docs/porting/dock/PORTING.md), [dependency map](docs/porting/dock/DEPENDENCIES.md),
[validation evidence](docs/porting/dock/VALIDATION.md), [source inventory](docs/porting/dock/ports.json) and
[current continuation record](docs/porting/dock/CONTINUATION.md).

Reference fixtures retain 7,895 original .NET geometry outputs and additional
captured helper, selector, root and numeric-boundary cases. Normal Rust tests
read committed fixtures without requiring .NET. Their generators and attribution
are recorded under `scripts/` and in the corresponding source inventory entries.
