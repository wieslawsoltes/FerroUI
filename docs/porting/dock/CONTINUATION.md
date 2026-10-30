# Current checkpoint: native JSON workflow and physical docking verified

The clean `/Volumes/SSD/temp/ferrodock-port/FerroDock Core Review.app` was launched on
2026-10-24, after closing only the owned diagnostic Desktop Acceptance app. The Mac is
unlocked. The user confirms the File dropdown appears and works with their mouse.
Actual native Save As and Open both PASS on executable SHA
f2728b3b6a71505bc732e4c18040f9ac67460f27287076020c33b1f2718fd6cd.

Native Save produced `/Volumes/SSD/temp/ferrodock-port/ferrodock-core-desktop-20261024.json`:
49,403 bytes, UTF-8 BOM, original Dock.Model.ReactiveUI.Controls.RootDock discriminator,
SHA 9b1bf3de6a6817d14d7bc2657933f4ede40b425c9e42391b383b5c02719a34cf.
Creating Document4 then opening that saved file restored three documents and their actual
views. Creating Document4 again worked, and F11 showed it in the restored original
CustomDocumentDock. Closing the inspector left the sample running.

Save cancellation preserved the four-document layout. Later Open cancellation preserved
the reordered three-document Browser layout. Opening the owned malformed JSON produced
“Loading layout failed: EOF while parsing a value at line 1 column 9”, preserving the
layout, selection, views and Browser theme. Native file dialogs are now independently
verified, rather than inferred from the controlled-picker Headless tests.

The user physically verified document reorder and tool floating/docking (“Both work”).
AX and screenshots show Document2/Document3/Document1/Document4 and Tool2/Tool1 order.
CUA's tab drag selected tabs but did not move them: this is an automation limitation,
not a reproduced production defect. Browser light/dark rendering retains the restored
layout and selection. Browser Tool1 pin/preview/unpin, document close, F9 overlay,
actual pane resize (about100 screenshot pixels) and grip stretching were verified on the
clean build. Physical Browser docking/tool-flyout verification is currently pending (async question).
A focused regression now verifies actual compiled ToolFlyout/TemplateBinding retention and
pointer opening after JSON restoration AND after pin/unpin under both themes. Initial
binding2/2 and pointer2/2 probes pass; final full markup session43616 is terminal0 with
204 passed, zero failed/ignored. Targeted strict all-target Clippy session47625 is terminal0. Final formatting session17932,
source inventory, seven-vendor integrity and project whitespace checks pass. All owned jobs
are terminal. The only acceptance item remaining is the pending physical Browser check.

Both original native fixes remain in vendor/ferroui-native with exact provenance:
borrowed URI ownership in four Objective-C++ storage functions, and callback-local live
owner checks in native accessibility peers. No Dock engine/model/application production
source changed. Previous full automated revision:1967 workspace tests/57 suites and127
native library tests pass, strict all-target Clippy/fmt/24Python/source/theme/inspector/
seven-vendor guards and clean native build pass. Only the restored-layout integration test gained assertions in this turn; the full
workspace count remains1967, and the204-test markup suite was rerun after that change.
Proof: /Volumes/SSD/temp/ferrodock-port/ferrodock-native-desktop-checkpoint.json.

README now records real native Save/Open and uses SSD TMPDIR/target/bundle commands.
All own scratch remains /Volumes/SSD/temp/ferrodock-port; one owned Cargo job at a time.
Original parent source and staged index remain clean. Protected old executable SHA
7262f885d1469c30ebacfafe765648cf0902266602f84a5241b8080e62d7d465 stays untouched.
Do not use old diagnostic apps or the shared window title to select this sample.
Native UI must use CUA. No subagents, commits, pushes or PRs. Accepted scope is
DELIVERY-SCOPE.md; deferred projects/full1928-file coverage are not delivery gates.

## Previous automated checkpoint

# Current checkpoint: scoped core automated acceptance passes; desktop gate needs unlock

Previous goal turn was PROGRESS (restored move/float/redock). This turn is PROGRESS:
four new composed cases extend the existing pair to six tests in
tests/Dock/FerroDock.Markup.Tests/docking_graph_tests/composed_drag_tests.rs. Both themes now
exercise a rendered Right selector to create a split, remove the adorner on release,
drag the real ProportionalStackPanelSplitter, and verify adjacent model proportions,
measured pane width, unchanged unrelated proportion and exact JSON replay. The tool pair
uses real pointer clicks for pin, pinned preview, unpin, tab selection/reorder and close;
public original factory APIs drive explicit hide/restore. Both hide-on-close and removal
modes are checked. Pinned and hidden graphs roundtrip. The earlier move/float/redock pair
now also reorders native document tabs inside their strip and closes the last document
of a pane via its actual close button. No production code or dependencies changed.

Focused selector2, tool2 and combined6 runs pass. No production bug was found or old
assertion weakened. Temporary resize tracing was removed. Source inspection confirms
OriginalOwner has IgnoreDataMember and the original default resolver removes that member;
JSON deliberately omits this transient state. Tests require omitted key and null restored
OriginalOwner, rather than claiming restoration of this ignored value. One literal-key
assertion was tightened after initial full validation13377; exact81513 then repeated the
complete checks over the final frozen source.

EXACT81513 is confirmed terminal0: all 23 stages pass, **1,967 tests/57 suites**, zero
failed/ignored. Markup 204, sample 44, native UI 373, diagnostics 49. Strict all-target and
16 selected release-library Clippy, ten standalone suites, eleven normal dependency
boundaries, format, 24 Python tests, source/theme/inspector/six-vendor checks and both
generators pass. Broad source completion remains correctly rejected, separately from
accepted delivery. Inventory 227/465/1236 of 1928 unchanged; only existing record notes
changed. No new files this turn, no agents/commits/pushes/PRs; staged index remains empty.

Proof: /Volumes/SSD/temp/ferrodock-port/ferrodock-core-acceptance-exact-manifest.json.
All 168 input hashes, 64 earlier source outputs, prior emitted JSON context, original parent
sources and both desktop artifacts are verified unchanged. Scratch/artifacts stay on SSD.
All owned processes are terminal. Production source did not change, so no redundant
native rebuild was performed. The separate actual review bundle remains ready:
/Volumes/SSD/temp/ferrodock-port/FerroDock File Workflow Review.app
identifier org.ferrodock.reactiveui-sample.file-workflow-review
SHA 442e414f32b98acb4d7c2452530cd06eb9a332eadf0ca82ee6e7923cae467926
It has NOT been launched/visually verified. Preserve the old protected executable/window.

Completion audit against DELIVERY-SCOPE.md: original core engine/ReactiveUI models,
standard JSON/file workflow and both themes have automated evidence; actual refreshed
native example/dialog presentation, theme appearance, F11 inspector and full-width grip
still require desktop verification. Those missing results prevent goal completion. Do
not inflate source coverage or infer desktop success from Headless tests. Known broader
serialization/API/lifetime limitations remain explicit and outside claims made here.

The Mac lock was verified again by CUA inventory. It has recurred across more than three
consecutive goal turns while independent authorized core work continued. That work is
now prepared and verified for the desktop pass; the remaining concrete acceptance actions
cannot be performed while native desktop access is unavailable. An unlock question is
already pending. Goal should be marked blocked on this external gate after saving the
handoff; do not create more substitute tests/status turns merely to avoid the blocker.

NEXT ON USER UNLOCK/RESUME: refresh CUA documentation if context was compacted, inventory
apps, launch only the distinct review bundle using cua_repl, and verify it is the correct
new process/window. Never restart/overwrite the protected old app/window. Exercise File
Save/Open/cancel/error with fresh files under /Volumes/SSD/temp/ferrodock-port; switch
Fluent/Browser and verify tabs, split/resize, drag/drop/float/redock and pin/preview/unpin/
hide/restore/close with restored layouts. Check F11 original inspector, F9 overlay and
the header grip across resize. Fix actual findings, rerun relevant checks and update
acceptance evidence. Desktop acceptance is missing, so do NOT mark complete yet. A user
resume starts a fresh blocked audit. Do not repeat the unlock question while still pending.
No general compiler/backends/serializers/samples/full-inventory expansion is needed for
this delivery without a concrete new dependency. Native UI actions must use cua_repl.

## Previous checkpoint

# Current checkpoint: restored pointer move, float and cross-window redock

Previous goal turn was PROGRESS (native file workflow). This turn is PROGRESS: two new
composed Headless tests in tests/Dock/FerroDock.Markup.Tests/docking_graph_tests/composed_drag_tests.rs
exercise actual native pointer events through DockControlState, the original DockManager
and actual model/factory/host classes under Fluent and Browser. No production code or
dependencies changed. Tests are a child of the existing graph module and reuse Scene.

A third document pane is added through original factory APIs. After standard JSON reload,
new model identities are required and the original factory initializes the graph. A tab
is dragged out of its strip into the other pane. The same document then drags outside the
main window and floats through the original IDockWindow host locator into a real HostWindow
with the selected native theme. Floating layout JSON must roundtrip byte-for-byte. Real
pointer input in that window drags its last tab back across window coordinates. Original
owner/factory/selection identities, both main/floating drag reset, native floating panel,
closed empty host and root window-list cleanup are asserted. Factory DockControls grows
1->2 on float and returns 2->1 after redock. No fake engine, replacement host or production
test entry point was introduced. Host locator uses the same ID as the original sample.

Early fixture errors are retained on SSD. The first compile tried a private projection;
consumer code now uses the existing public typed ValueTypes conversion. A manually-added
third pane initially had no remaining proportion after the original two panes consumed
available width; explicit one-third shares fix the fixture rather than production layout.
A dictionary import was corrected to System's existing generic Dictionary. Final tests
retain all required floating/redocking assertions; the temporary diagnostics-setting guard
was removed. No old test was weakened or production workaround added.

Final70568 is confirmed terminal0: all 23 stages pass, **1,963 tests in 57 suites**, zero
failed/ignored. Markup 200, sample 44, native UI 373 and diagnostics 49. Strict all-target and
16 release-library Clippy, ten standalone suites, eleven normal boundaries, format,
24 Python tests, source/theme/inspector/six-vendor checks and both generators pass.
Full repository completion remains correctly rejected. One new test target maps to the
original DockControlState and DockManager records; statuses/counts 227/465/1236 of 1928
stay unchanged. New file intent-to-add only; no agents/commits/pushes/PRs.

Proof: /Volumes/SSD/temp/ferrodock-port/ferrodock-composed-drag-final-manifest.json.
All 168 input hashes, 64 previous source outputs and the prior generated JSON context match.
Original parent source/index and both desktop artifacts are unchanged. Separate SSD review
bundle remains /Volumes/SSD/temp/ferrodock-port/FerroDock File Workflow Review.app;
its hash matches the recorded source executable (production inputs unchanged). No desktop
rebuild was needed. All owned processes terminal. CUA still reports Mac locked; the prior
unlock request is pending. The distinct review app has not been launched/visually verified.

NEXT: selector-driven splitting/resizing and composed tool pin/hide/show/close acceptance
under both themes, including a restored graph. Current composed drag tests drop on tab
borders (Fill), not an adorner selector; do not claim selector geometry/splitting coverage.
Extend real native input to a document body, realize the actual DockTarget, move to a
rendered selector (for example PART_RightSelector), release and verify the engine-created
split/owner/proportions. Exercise the actual ProportionalStackPanelSplitter with native
pointer drag and verify model proportions and measured panes. Source splitter handlers are
in src/Dock/FerroDock.Controls.ProportionalStackPanel/proportional_stack_panel_splitter.rs.
Then exercise original tool commands/controls for pin/preview/unpin, hide/restore and close.
Use original factories/models/controls; preserve source semantics and existing tests.

When Mac is available, launch only the distinct SSD review app via cua_repl and verify
native file panels, both-theme appearance, F11/inspector, grip and scoped interactions.
Preserve the protected old app/window. Do not repeatedly poll or repeat unlock questions.
Continue the remaining independent core acceptance while locked. Goal active/incomplete/
unblocked. Do not expand generic compiler, other backends/serializers/samples or full source
completion. Known serialization/platform/lifetime limits remain documented.

## Previous checkpoint

# Current checkpoint: native picker-to-file acceptance

Previous goal turn was PROGRESS (production File menu/storage wiring and separate SSD
review build). This turn is PROGRESS: two native storage integration tests close the
missing supported-provider path without changing production code or dependencies.
The new native_file_workflow_tests.rs is a child of app_tests.rs so it reuses the actual
App Headless session. A scoped IStorageProviderFactory supplies controlled platform
picker results; the unchanged NativeLayoutFileStorage performs actual BclStorageFile
read/write against unique temporary directories. TMPDIR is SSD for every run; fixtures
dispose the locator scope and remove only their own directory.

Both Fluent and Browser scenarios raise real MenuItem click events from compiled markup.
They attach the file-action model after initial binding, resolve the owning TopLevel
from nested MainView, gate both menu commands during delayed native picker completion,
write/read standard BOM JSON, restore actual custom views and live Root/Documents
locators, and execute the original CreateDocument command after restore. Provider
options and exact owning-window identity are asserted. The failure scenario covers
native picker cancel/errors, missing input, malformed JSON, native write failure and
UTF-16 null; graph/control identities stay intact until valid load, then null clears both.
The fixture restores the application tab theme between scenarios.

Initial test compilation incorrectly assumed a generic lookup on dyn IFactory; its
existing generic API is concrete-only. The fixture now invokes actual public locator
callbacks, cloning them outside the dictionary borrow. The first runtime assertion
incorrectly expected Document4.Context. Unchanged original CustomDocumentDock.cs and
DockFactory.cs show contexts for Document1..3 only: tests now explicitly preserve null
for new Document4 while requiring restored existing document context and the same actual
factory. Production behavior and old assertions were not changed to satisfy the tests.
Intermediate logs remain under SSD prefix ferrodock-native-file-.

Final driver64612 is confirmed terminal0: all 23 stages pass, **1,961 Rust tests in 57
suites**, zero failed/ignored, sample 44. Strict workspace/all-target and 16 selected
release-library Clippy, ten standalone suites, eleven normal dependency boundaries,
format, 24 Python tests, all source/theme/inspector/six-vendor guards and both generators
pass. Full repository completion remains correctly rejected, separately from delivery.
No source coverage promotions: 227/465/1236 of 1928. One new test target is mapped to the
original App record; new file is intent-to-add only. No agents/commits/pushes/PRs.

Proof: /Volumes/SSD/temp/ferrodock-port/ferrodock-native-files-final-manifest.json.
All 167 input hashes, 64 prior source outputs and the previous UI-independent generated
context match. Original parent sources and staged index remain unchanged. The protected
old executable retains its recorded size/mtime/SHA. The separate review bundle and its
build executable still match SHA 442e414f32b98acb4d7c2452530cd06eb9a332eadf0ca82ee6e7923cae467926:
/Volumes/SSD/temp/ferrodock-port/FerroDock File Workflow Review.app
Production inputs did not change, so no redundant desktop rebuild was made. All owned
processes are terminal. CUA still reports Mac locked; the earlier unlock request remains
pending. The review bundle has NOT been launched or visually verified.

NEXT: accepted-scope composed docking acceptance, with a desktop visual pass when
available. Reviewed graph tests prove live selection/pin behavior and restored layouts
under both themes; ItemDragHelper has real-pointer reorder tests, target/preview helpers
have focused checks and tool grip has a resize regression. These do not yet prove one
complete drag/drop/float/redock path through actual DockControlState, its original
DockManager and host factory. The corresponding inventory records still defer original
routed input/capture, drop/float and native geometry/lifecycle scenarios. Exercise this
real composed path under Fluent and Browser (including a restored graph), then split/
resize, pin/hide/show/close as one working feature set. Use actual controls/models/native
Headless input, original manager/factory and source host composition; do not add a fake
engine or production entry point just for a passing test. Existing Scene in
tests/Dock/FerroDock.Markup.Tests/docking_graph_tests.rs supplies root/split/docs/tools with
actual Factory and compiled Fluent/Browser graph dictionaries. Inspect the original
host locator setup if floating needs a host; do not silently replace host behavior.

Once Mac is unlocked, launch only the distinct SSD review app through cua_repl and
verify native Save/Open/cancel/error panel presentation, F11 inspector, full grip and
both-theme appearance/interactions. Preserve the protected old app/window/artifact.
Do not repeat unlock questions or constantly poll. Continue independent core acceptance
work while locked; goal remains active/incomplete/unblocked. Do not broaden into general
compiler, extra backends/serializers/samples or full source completion.

## Previous checkpoint

# Current checkpoint: File menu layout persistence

Progress toward [DELIVERY-SCOPE.md](DELIVERY-SCOPE.md): the sample now has compiled
Open Layout and Save Layout As menu commands plus status feedback. A separate
LayoutFileViewModel uses asynchronous ReactiveCommands and a shared availability gate;
MainWindowViewModel keeps the actual original layout/factory and exposes standard byte
save/load methods. Views remain passive. App composes the file model/storage service in
both desktop and single-view startup paths. The native service holds its parent weakly,
resolves its current TopLevel and uses the platform JSON pickers and storage streams.
Files are disposed after read/write; writes flush and truncate through native storage.

LayoutJsonSerialization delegates bytes to the existing serializer's Stream Save/Load
and MemoryStream. Its original UTF-8 preamble and encoding detection are preserved.
The new flow reuses all 12 application class registrations and original restoration
initialization. Cancellation and read/parse errors preserve the current layout; JSON
null clears it. Both file commands are unavailable while an operation is pending.
Failure/cancellation/success messages update the compiled status binding.

Eight new tests bring the sample to 42 and the workspace to **1,959 tests/57 suites**,
zero failed/ignored. They cover real stream bytes/models, cancellation/errors, pending
picker gating, null and UTF-16 input, unconfigured services, picker options, actual SSD
storage roundtrip/truncation/IO errors, weak/missing/unavailable native hosts, and real
compiled menus restoring actual views under Fluent and Browser. Existing startup tests
also verify file actions are composed for both lifetimes. The first UI test incorrectly
searched unopened submenu items in the visual tree; it now uses the actual logical tree.
No production workaround or weakened assertion. Initial compile errors required an
IReactiveObject import and explicit native IO result types; diagnostic logs are retained.

All 23 validation stages pass (workspace, strict all-target and 16 release-library Clippy,
ten standalone suites, eleven normal dependency boundaries, format, 24 Python tests,
source/theme/inspector/provenance checks and both generators). Initial driver31549
terminated1 only because five new supporting Rust files lacked inventory mappings.
Only docs/porting/dock/ports.json mappings/notes changed after runtime checks; all Rust inputs stayed
frozen. Finisher48756 is terminal0 and reran all remaining structural checks. Coverage
counts remain 227/465/1236 of 1928; no completion promotions. The stale App inventory note
about unported F11 attachment now agrees with its actual startup implementation/test.

Proof: /Volumes/SSD/temp/ferrodock-port/ferrodock-file-workflow-final-manifest.json.
All 166 input hashes, 64 earlier source outputs, prior UI-independent generated context,
protected desktop executable size/mtime/hash, original parent sources and empty staged
index are verified. Existing framework versions/features/vendor sources and manifests
are unchanged. New files are intent-to-add only; no agents/commits/pushes/PRs. Scratch
and native storage test files use SSD. The full goal remains active/incomplete/unblocked.

Desktop build55508 is terminal0 (3m38s). A distinct bundle is ready:
/Volumes/SSD/temp/ferrodock-port/FerroDock File Workflow Review.app
Its bundle identifier is org.ferrodock.reactiveui-sample.file-workflow-review, so a
later launch can be distinguished from the protected original app. Source/bundle hashes
match; the exact artifact proof is ferrodock-file-workflow-desktop-review.json on SSD.
The build uses a separate desktop-file-workflow-target, dev debug0/incremental false,
-j1; debug assertions/F11 remain enabled. It has NOT been launched or visually verified.
Packaging39400 is terminal0. All owned processes are terminal. CUA inventory still reports
the Mac locked after the build; an asynchronous unlock question is pending. Do not keep
polling or claim desktop acceptance. Native UI actions must use cua_repl. Never overwrite
or restart the protected old executable/window. The standard bundler rejects an existing
bundle with the distinct review identifier: use another fresh output for a future build,
or explicitly preserve/check this known review artifact before replacing its bytes.

NEXT: Once available, verify actual native
Save/Open/cancel/error, both themes, F11/inspector, full header grip and scoped docking
operations: tab selection/reorder, split/resize, drag targets/preview/drop, floating and
redocking, pin/unpin, hide/show and close. Update the delivery acceptance evidence from
actual results. Do not expand general compiler/backends/serializers or source coverage
without a concrete core delivery need. Preserve known serializer limits in SERIALIZATION.md.

## Previous checkpoint

# Current: actual application subclass JSON and injected sample restore

Previous goal turn was PROGRESS (live tab style selection). This turn is PROGRESS toward
[DELIVERY-SCOPE.md](DELIVERY-SCOPE.md). Production backend adapter adds DockJsonModelRegistration
and model_type_metadata_with_models/create_serializer_with_models. Explicit typed dock,
document, tool, document-dock and root registrations preserve actual constructors, inherited
fields, supplied own properties, enclosing Rc identity, reference views and original family
entries. Duplicate/empty logical names and duplicate/standard model types return argument
errors before model construction. Existing standard factory APIs and all baseline wires stay
intact; no reflection, surrogate models, global model registry or generic compiler expansion.

The sample explicitly registers its12 persisted classes with original C# wire names. New
LayoutJsonSerialization implements a framework-independent ILayoutSerialization; a separate
ILayoutInitialization reconnects actual DockFactory fields/locators. MainWindowViewModel has
with_layout_services, can_persist_layout, save_layout_json/load_layout_json. App startup composes
these services; the compatibility new constructor retains source behavior without persistence.
Malformed JSON returns before closing/replacing current layout. Successful load closes the
old layout, initializes the new actual graph and publishes it; JSON null clears live locators
and visible status. No file dialogs/actions are wired yet.

Restore selection follows source traversal to find the primary document dock by its contract:
the source sample's document dock has an empty Id, despite its locator key being Documents.
Existing private root_dock/document_dock fields now point to loaded models, and original InitLayout
rebuilds live contexts/host/dockable locators. These are original factory fields, not retention
caches or snapshots. Multiple nonstandard primary-dock conventions need application policy.

Reference project links all12 unchanged original C# sample classes (using their original source
preprocessor branch).18 source checks and13 JSON wires cover class defaults, aliases and the
actual constructor-created CreateDocument command. All64 earlier source outputs remain byte-
identical. Native tests match12 defaults and the source graph, preserve custom types/aliases,
refresh actual contexts/live locators, create the original custom document after load, and
retain null versus malformed/unconfigured behavior. Backend9 tests also cover own application
properties, original identity, constructor deferral/count and invalid registrations.

One new actual App Headless test saves/restores the full sample under each tab theme, rebinds
DockControl, checks original Document/Tool views and contexts, then creates/activates Document4
through the restored CustomDocumentDock command and serializes again. Sample now34. Initial
fixture compilation lacked ReactiveObject; the test-only ReactiveUI dependency supplies it.
The first complete-sample test assumed Documents was a model Id; the actual source convention
above corrects that implementation/test. Focused Clippy84953 found one needless test borrow;
removed without allowances. Intermediate logs are retained; no old assertion is weakened.

FINAL57455 is confirmed terminal0: all23 stages pass, **1,951 Rust tests across57 suites**,
zero failed/ignored. Strict workspace/all-target and16 selected release-library Clippy, ten
standalone suites, eleven normal boundaries, format,24 Python tests, source/theme/inspector
checks, six framework provenance guards and both generators pass. Broad source completion
remains rejected, separately from scoped delivery. SSD prefix ferrodock-app-json-final-.
All81 pre-validation input hashes match,64 earlier source outputs and prior UI-independent
emitted context remain unchanged. Protected desktop size/mtime/hash, original parent source
and staged index match. New files intent-to-add only; no agents/commits/pushes/PRs. All owned
Cargo/reference handles terminal, all scratch/artifacts on SSD. Only docs follow green.
New dependency edges are existing local JSON packages in the sample and a test-only ReactiveUI
edge in the adapter; existing framework versions/features/vendor sources are unchanged.
Inventory227/465/1236 of1928 unchanged; no completion promotions. Goal active/incomplete/unblocked.

NEXT: wire the actual File menu save/load actions. The programmatic sample flow now works,
including its original custom classes, views and commands. Keep ViewModels on abstractions;
use ReactiveCommand task operations and application-composed native file-selection/storage
services or Interaction/behaviors, never code-behind handlers. Expose commands/capability/status
through the sample's explicit markup metadata and compiled bindings. Cancellation must leave
the layout untouched; read/parse failures must not close/replace it; show useful action status.
Native picker APIs are available in vendor/ferroui-base/platform/storage/i_storage_provider.rs:
open/save_file_picker_async return local futures, IStorageFile.open_read/open_write_async yield
Box<dyn std::io::Read/Write>. TopLevel.storage_provider is public. Own parent controls weakly
and resolve the correct TopLevel; support desktop and single-view lifetime composition.
ReactiveCommand::create_from_task uses the existing local Task executor. Preserve standard
stream/BOM/encoding behavior by using the actual serializer/MemoryStream at the domain boundary
when bridging file bytes, rather than introducing another JSON format. Test the workflow with
controlled storage/picker outcomes plus real compiled menu commands and both tab themes.

Then prepare a separate SSD-target desktop build/run for visual checks, preserving the existing
protected executable/window. Verify actual file save/load, F11/inspector, grip width and the
scoped drag/drop/resize/floating/redocking/pin/hide/close behavior under Fluent and Browser.
Do not broaden into full general compiler, other backends/serializers/samples or unrelated APIs.
Known closed enum/template/unknown-interface/ownership-cycle limits remain documented in
SERIALIZATION.md; no exhaustive serializer/full-source completion is claimed.

## Previous checkpoint

# Current: sample Fluent/Browser selector with live native theme switching

Previous goal turn was PROGRESS (standard core JSON catalog). This turn is PROGRESS toward
[DELIVERY-SCOPE.md](DELIVERY-SCOPE.md). The sample now has a Tabs selector in its status bar,
compiled from MainView.xaml and wired by MainViewThemeBehavior to a ReactiveCommand. The
existing original theme manager's light/dark and preset callbacks retain their captured
manager and behavior. Presentation/service contracts expose only i32 choices and commands;
all native Styles/BrowserTabTheme work stays in infrastructure/composition. Views are passive.

App owns one DockTabThemeSelection instance in a OnceCell; it owns the actual optional Browser
overlay and native Styles handle. Browser is added/removed through native resource/style APIs,
without replacing the factory/layout/models or adding static state. Each view keeps its
original theme manager adapter while sharing App's tab service. Valid indices are0/1; repeated
commands do not duplicate overlays. The new selector captures its owner weakly. The original
sample API remains otherwise intact. Only the existing local Browser package is added to the
sample's dependencies/Cargo.lock; framework versions/features and vendor code are unchanged.

Two new real App Headless tests switch Browser/Fluent repeatedly, verify the actual delayed
panel versus Fluent panel, retain original layout Rc identities and native pointer selection
of Document2, and exercise presets/light-dark during switching. Shared-service tests verify
idempotence, invalid-parameter guards and the initial choice of newly initialized views.
The pure ReactiveUI command test also checks tab actions. Initial24874 passed28/29: the new
shared-service test read its selector before MainView was initialized. It now attaches/shows
a native test window and drains layout before asserting, matching the behavior's real lifecycle;
36739 passes. No production workaround or weakened assertion. Logs are retained on SSD.

FINAL21542 is confirmed terminal0: all23 validation stages pass, **1,944 Rust tests across57
suites**, zero failed/ignored. Sample29, markup198, adapter7 plus usage doctest. Strict workspace/
all-target and16 selected release-library Clippy, ten standalone suites (including the actual
sample --lib), eleven normal dependency boundaries, format,24 Python tests, source/theme/
inspector guards, six framework provenance checks and both generators pass. Broad source
completion remains rejected, separately from scoped delivery. SSD prefix
ferrodock-sample-browser-final-. All56 pre-validation input hashes match;46 earlier source
outputs and prior UI-independent generated context are unchanged. Protected desktop executable
size/mtime/hash, original parent source and staged index are unchanged. New file intent-to-add
only; no agents/commits/pushes/PRs. All owned handles terminal, all scratch/artifacts on SSD.
Only docs follow green. Inventory227/465/1236 of1928 unchanged; no completion promotion.

NEXT: ordinary sample JSON save/load. The Browser choice is now wired/tested in source, but the
protected running desktop window has not been rebuilt/refreshed. Preserve that artifact while
preparing a separate reviewable sample build after persistence integration. Existing sample
persisted custom classes are DocumentViewModel, Tool1..Tool8ViewModel, HomeViewModel (RootDock),
DashboardViewModel (DockBase), CustomDocumentDock (DocumentDock):12 types. Inspect their actual
source before binding; current native classes compose original bases and mostly expose inherited
state. Extend the production backend adapter with explicit typed subclass registrations/common
property schemas and family registrations, preserving enclosing Rc identity and actual source
constructors/commands. Current model_type_metadata family tables are fixed; adding a concrete
codec alone does not register a subtype. Do not substitute core classes for application models
or spend this task on a complete general compiler. Logical wire names should be checked against
the original C# sample; markup namespace renaming is a separate concern.

Then wire save/load actions via framework-independent services/ReactiveUI commands and native
file-selection infrastructure/behaviors, keeping view code passive. MainWindowViewModel owns
its actual factory/layout; DockFactory has live context/host locators that must initialize the
loaded original graph. Do not persist view controls, transient Context, factories or host state
as surrogate layouts. Test actual sample contexts/view lookup and CreateDocument after load,
then live theme selection and scoped docking/desktop appearance checks (drag/drop/resize,
floating/redocking, pin/hide/close). The standard backend catalog, unknown closed enum values,
template/custom/unknown-interface fallbacks and ownership/cycle limits remain as documented in
SERIALIZATION.md; no exhaustive serializer or full-repository completion is claimed. Full goal
stays active/incomplete/unblocked under the user's accepted core-only delivery scope.

## Previous checkpoint

# Current: usable ReactiveUI core JSON catalog and restored-layout theme interaction

Previous goal turn was PROGRESS (native class schemas and accepted scope). This turn is
PROGRESS toward the user's reduced core docking/JSON/Fluent/Browser delivery target. Read
[DELIVERY-SCOPE.md](DELIVERY-SCOPE.md) first; historical full-repository requirements below
remain superseded. The active goal is incomplete/unblocked, with no outstanding owned jobs.

New production package FerroDock.Serializer.SystemTextJson.ReactiveUI supplies compiled
metadata for all14 standard layout models plus DockWindow. Public create_serializer,
create_serializer_with_list_type and model_type_metadata use the existing DockSerializer,
original constructors/accessors/identities, original shared lists, Core values/policies and
four core polymorphic families. Common/dock/root/tool/MDI/window property order and ignores
follow source. The runtime/model packages retain their normal dependency boundaries: the
new adapter depends on the backend/runtime/Model only, with no UI/build/reflection dependency.
Production manifest changes are limited to the new adapter and workspace membership;
Markup.Tests adds a dev edge. Cargo.lock adds local graph entries, with existing framework
versions/features and all six vendor packages unchanged. This is a compiled backend catalog,
not a generic compiler/discovery extension or an alternate docking engine/model.

The new original-source CoreLayoutCatalogReference runs unchanged C# models/default serializer:
27 assertions,15 default-model wires, a complete tools/documents/pinned/hidden/floating graph
and configured layout containers. All46 earlier source outputs remain byte-identical. Native
adapter7 tests match original wire/readback, factory initialization/AddDocument, owner/selection/
window aliases/cycles, MDI/grid/selector values, configured containers, requested observable/plain
lists with null entries and stream Save/Load BOM/close behavior. Invalid JSON/unsupported closed
enum values return typed errors. A compiled usage doctest exercises the public entry point.

Two new downstream Headless tests save/load a standard graph and rebind actual DockControl
under Fluent and Browser. They verify fresh model identity, the original factory, native
pointer tab selection, source pin/unpin and unchanged original graph selection. Both pass;
markup now198. No original assertions/goldens are weakened. Intermediate errors are retained:
63085 missing Option<String> descriptor; initial wire compile used nullable argument on a
nonnullable concrete AddDocument API;21352 consumed the factory test's last strong owner.
Fixes add the actual nullable codec and correct the test call/owner lifetime, with no engine
or lifetime-model changes. Focused Clippy20384 terminal0.

FINAL72420 is confirmed terminal0: all22 stages pass, **1,942 Rust tests across57 suites**,
zero failed/ignored. Strict workspace/all-target and14 selected release-library Clippy,
nine standalone suites, eleven normal dependency boundaries, format,24 Python tests,
source/theme/inspector checks, six framework provenance guards and both generators pass.
Broad source completion audit remains correctly rejected; it is not the scoped delivery gate.
SSD prefix ferrodock-catalog-final-. All45 pre-validation input hashes match afterward;
46 prior source outputs and the prior UI-independent generated context remain byte-identical.
Protected desktop executable size/mtime/hash, original parent sources and staged index are
unchanged. New files intent-to-add only; no agents/commits/pushes/PRs. All owned Cargo/reference
handles terminal. All scratch/artifacts remain on SSD. Only docs follow green.
Inventory227/465/1236 of1928: two newly started serializer source units, no complete promotions.

NEXT: wire a runnable sample's ordinary save/load and Browser theme selection, preserving
MVVM/compiled markup. Existing original sample still uses custom application model classes
and Fluent only, and has no save/load actions. Those custom classes require explicit compiled
metadata/polymorphic registration; the new standard catalog intentionally does not discover
them. Provide that application integration and verify end-to-end before claiming completion.
Ten closed Rust layout enums accept declared values only; unknown numeric values return errors
instead of fabricating a value. Additional template/custom application/interface-unknown
fallback coverage remains partial; no claim of exhaustive original serializer parity. Finish
scoped cross-feature docking/desktop appearance checks, including actual floating/redocking,
resize/drag/drop and pinned hosts. General compiler/backend/API expansion remains deferred
unless a concrete dependency of the accepted path requires it. Preserve the protected existing
desktop artifact while preparing a reviewable next sample build.

## Previous checkpoint

# Current: native SDK class schemas and narrowed delivery scope

The preceding goal turn was NO PROGRESS (a status explanation). This turn is PROGRESS.
The user has now explicitly narrowed the delivery target to core docking, standard JSON
serialization, the normal (assumed Fluent) theme and Browser tabs, with supporting features
working. **Read [DELIVERY-SCOPE.md](DELIVERY-SCOPE.md) first.** This supersedes historical
requirements below to complete all original projects/APIs/backends before delivery. Keep
original engine/API fidelity for implemented features; the broad file inventory remains an
audit, not a delivery percentage or completion gate. The active goal is incomplete/unblocked.

The actual SDK consumer now constructs seven fresh semantic class schemas from Rust
metadata via the production import/bind APIs: ReactiveBase, DockableBase, DockBase, RootDock,
DocumentDock, Document and DockWindow. All101 property types/order/ignore facts come from
native declarations, with actual source locations and base-topological ordering. Logical
wire names, class abstraction and direct original interfaces remain explicit compiler
configuration. Original interface precedence matches all seven source class closures.
Foreign Core/System/ReactiveObject facts remain imported reference inputs. The emitter uses
the rebuilt class schemas and remapped original model/family bindings, preserving the same
four-model projection,92 native accessors, original wire/graph/list/factory assertions.

Three adapter regressions compare every original header/property/ignore fact, poison all
seven old class headers/property rows and require identical output, and mutate native
property types/order/ignore annotations to prove they are consumed. Dependency exports now
canonicalize through actual transitive xamlmeta inputs. No production manifests, dependency
versions/features, framework sources, reference data or existing assertions change.

Focused adapter70/markup196 and strict focused Clippy pass. FINAL98284 is confirmed terminal0:
all21 stages pass, **1,932 Rust tests across55 suites**, zero failed/ignored. SSD logs/proof use
ferrodock-native-class-final-. All14 selected input hashes match after validation; prior
UI-independent emitted context remains byte-identical. Original parent sources/index and
protected desktop size/mtime/hash remain unchanged. New files intent-to-add only; no agents,
commits/pushes/PRs. All owned Cargo handles terminal; all scratch/artifacts remain on SSD.
The broad completion audit is still correctly rejected, but is no longer the narrowed
functional delivery gate. Inventory remains227/463/1238 of1928; no unit promotion.

NEXT: ordinary core docking JSON persistence. Current production DockSerializer still needs
a supplied resolver; the four-model test projection omits Tool/ToolDock and other necessary
layout containers. Implement a usable catalog/integration for the accepted graph, then
exercise restored layouts with Fluent and Browser. Both themes already have native resource
and interaction tests; the current sample uses Fluent and has no save/load actions or Browser
selection. General compiler completion and unrelated backends/serializers/samples are deferred
unless needed by this path. See DELIVERY-SCOPE.md for acceptance evidence and remaining gaps.

## Previous checkpoint (historical broader scope)

# Current: native class declaration order and source ignore annotations

Previous goal turn was PROGRESS (actual SDK constructors/92 properties,1929/all21). This turn
is PROGRESS. Six actual SDK classes now retain compiled original declaration order and24
IgnoreDataMember annotations across101 declared properties: DockableBase37/DockBase15/
RootDock15/DocumentDock12/Document6/DockWindow16. DockBase, RootDock and DocumentDock required
ordering corrections; existing original callbacks and markup access remain intact. The
actual downstream compiler audits every ordered property name/ignore flag before emitting
the existing92 selected original-model bindings. This ports native declaration facts; full
logical class/type/header/dependency compilation and remaining attributes are still open.

Focused89762 passes SDK8/markup196/consumer9. FINAL85300 is confirmed terminal0: all21 stages
pass, **1,929 Rust tests across55 suites**, zero failed/ignored. Strict workspace/all-target
and selected release-library Clippy, eight standalone suites, ten dependency boundaries,
format,24 Python tests, source/theme/inspector checks, six framework provenance checks and
both generators pass. Full completion is correctly rejected. SSD logs/summary prefix
ferrodock-native-attributes-final-. Source hashes recorded before validation match afterward;
only docs follow green. Existing original wire/graph assertions and the prior UI-independent
emitted context stay intact; no reference data/assertions/dependency/version/feature/framework
changes. All owned Cargo/reference handles terminal. Protected actual desktop executable/
app/window and original/sibling/cache sources remain unchanged. All scratch/artifacts on SSD.

Inventory remains227/463/1238 of1928 (11.8% complete,24.0% partial,64.2% pending;35.8% started).
These are source/project file coverage percentages, not an effort estimate. Native declaration
facts fill existing partial files; none is promoted complete. Full goal active/incomplete/
unblocked; index empty, no agents/commits/pushes/PRs. User's percentage question is answered
from the full source audit, which includes pending units absent from the690 explicit records.

NEXT: construct full native logical class/type/header schemas from these declarations and
resolve original inheritance/hiding/override facts without reference application schemas.
Complete external ReactiveObject/system/ancillary/dependency/family/default/application/backend
catalogs, other constructors and remaining enum/flag markup metadata, then real sample save/load.
Other source attributes, conditional/fallible/parameterized construction and the remaining full
engine/API/lifetime/cycle collection/platform/accessibility/performance scope stay required.
Preserve the complete original objective.

## Previous validated checkpoint

# Current: actual SDK constructors and all selected native model accessors

Previous goal turn was PROGRESS (inherited property-only APIs,1925/all21). This turn is
PROGRESS. The actual SDK consumer now binds real metadata constructors and identity to the
original Rc models for Document, DocumentDock, RootDock and DockWindow, through the production
emitter. It requires source-exported concrete handles and casts, explicitly selects/checks
the existing public new/model APIs against actual function/interface facts, and extracts the
same original constructor-created model through its boxed reference. The original constructor
executes once via existing receiver projection; no duplicate model/engine/cache/reflection.

DockWindow's existing concrete handle registration now uses qualified MarkupTyped syntax
understood by the shared scanner. Its runtime metadata identity is unchanged. DockableBase
adds actual AllowedDockOperations, AllowedDropOperations, ShowInSelector and SelectorTitle
metadata. Typed Result accessors query the same original optional contracts; 37 native
property names/order match the unchanged compiled source schema. An SDK test preserves signed
unknown masks and selector visibility/nullable title edits. All92 selected property bindings
(Document22/DocumentDock26/RootDock31/DockWindow13) now come from actual native SDK declarations;
no selected-property fallback remains. New runtime tests apply every existing graph assertion
against the source wire, retain cycles/aliases/real factory initialization/list overloads,
and check distinct actual constructor-created models plus live original-model edits.

The unchanged original four-model fixture is extracted into shared test support so the prior
UI-independent consumer remains byte-identical (SHA256 c16e2653e230312cb8484bf647883c2ea07b38d65845179d03f683565ec59bc3).
Only integration-test build dependencies add existing workspace serde_json; production
manifests/versions/features and all six framework packages remain unchanged. Original C#,
sibling and cache sources are read-only. Semantic seed/projection selection/family fallback
bindings still use explicit original reference/typed facts; full native schemas/catalogs and
sample persistence are unfinished. No source unit is promoted complete.

Intermediate failures are retained:57578 shared fixture identity import;66991 Document's
model accessor returns its inherited IDockable;14017 compares differently formatted dyn
handles;32613 DocumentDock exposes two interface accessors. The host now explicitly checks
the configured SDK accessor and normalizes both property types through the shared scanner.
86116 passes the initial native constructor context;39536 passes SDK8/markup196/consumer9
after all92 accessors and four metadata fields. Initial full20009 passes1929 tests but Clippy
rejects duplicate loading of the fixture module; it now has one shared owner and imports.
No source behavior, wire goldens or previous assertions are weakened.

FINAL85675 is confirmed terminal0: all21 stages pass, **1,929 Rust tests across55 suites**,
zero failed/ignored. Strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six framework provenance checks and both generators pass. Full completion is correctly
rejected. SSD prefix ferrodock-native-model-checked-. Source hashes recorded before validation
are verified afterward; only docs follow green. All owned Cargo/reference handles terminal.
Inventory227/463/1238 of1928, unchanged; full goal active/incomplete/unblocked. New files
intent-to-add only, staged index empty; no agents/commits/pushes/PRs. Protected actual desktop
executable/app/window unchanged. All scratch/artifacts remain on SSD.

NEXT: complete native class semantic attributes/headers, inheritance/hiding/overrides,
constructor/identity coverage beyond these four classes, and general public handle export
resolution. Complete native compiler/system/ancillary dependency schemas and family/default/
application/backend catalogs, including remaining enum/flag markup metadata, then actual
sample save/load. Fallible/parameterized constructors, conditional attributes and the full
remaining engine/API/inherited enum/lifetime/cycle collection/serializer/platform/accessibility/
performance scope stay required. Preserve the full original goal.

## Previous validated checkpoint

# Current: inherited native property callbacks on actual SDK models

This continuation is PROGRESS. The preceding user progress reply was status-only; current
source and validation evidence were rechecked before the next concrete edits. Added borrowed
RustPropertyMetadataOptions/property_options, bind_declared_properties,
metadata_instance_type and project_property_bindings. Constructorless class/interface layers
contribute their own real properties without fabricated constructors, identities or base
member decisions. Existing whole-class binding shares that path while retaining type-codec
precedence and explicit inherited-member decisions. Callback/receiver expressions capture
once and operate on each live model; no replacement model, engine, cache or reflection.

An independent markup consumer scans the actual ReactiveUI SDK's five-level RootDock
hierarchy. Declared base edges, registered casts and real public zero-argument functions
matched by owner/return type supply the conversions, without guessed method names. Fourteen
selected scalar property callbacks compile against original Rc<RootDock> models. Original
source scalar wire order/values, null override omission, unnamed enum values, separate live
instances and repeated same-original-instance setters pass. This is a scalar callback probe,
not a complete inherited/hiding compiler or production schema/catalog. Test-only manifest
edges reuse existing local packages; production manifests, versions and features are unchanged.

The actual downstream compile exposed private declaring-module annotations and absolute
value-type parameter spacing. Native direct-instance views now emit their declared public
export; other handle shapes still need general export resolution. Native setters and mixed
Result emitter adapters use valid spaced annotations. Existing stateful emitted consumers
also compile absolute value types. Four new adapter tests cover the new APIs/public paths;
two real-SDK runtime tests extend markup191->193. Failed/intermediate logs are preserved:
6341 private paths/missing JSON writer method;72101 absolute annotations;2578 probe null/core
metadata setup;98874 wrong existing fixture assumption. Corrections keep original goldens,
source behavior and all earlier assertions intact.67656 passes adapter67/consumer9/markup193.

FINAL73699 is confirmed terminal0: all21 stages pass, **1,925 tests across55 suites**, zero
failed/ignored. Strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six framework provenance checks and both generators pass. Full completion is correctly
rejected. SSD logs/summary prefix ferrodock-native-base-final-. Selected source hashes are
recorded before validation and verified afterward; only docs follow green. All owned Cargo
handles are terminal. Inventory227/463/1238 of1928, unchanged. Full goal active/incomplete/
unblocked; new files intent-to-add only, index empty, no agents/commits/pushes/PRs. Protected
actual desktop executable/app/window and original/sibling/cache sources remain unchanged.
All scratch/artifacts on SSD.

NEXT: complete actual SDK constructor/identity binding and source-backed native inherited
model schemas, including source hiding/overrides and general public handle export resolution.
Resolve complete compiler/system/ancillary schemas and default/application/backend catalogs,
then actual sample save/load. Fallible/parameterized constructors, conditional attributes,
remaining engine/API/inherited enum/lifetime/cycle collection/serializer/platform/accessibility/
performance scope remain required. Preserve the full original goal.

## Previous validated checkpoint

# Current: native receiver projection onto original model handles

Previous goal turn was PROGRESS (fallible accessors,1914/all21). This turn adds public
RustReceiverProjection/project_type_binding. Explicit typed conversions bind native class
views to their same original model handles. Getters/setters, flags, converters, order,
identity and declared reference-cast targets are retained. Each operation converts its live
receiver; no model/view is cached or copied. Original constructor callbacks execute once,
then their owned result is converted to its same model. No constructor/cast/type/identity
name is inferred. Missing constructor conversions fail, absent source constructors remain
absent, and custom type codecs require explicit target metadata rather than guessed wrapping.

Independent consumers scan shared class-view metadata and compile it against the original
fixture models. Tests match source wire/aliases and cycles, distinguish live instances,
verify constructor identity/call count and preserve the same original accessor error across
operations. The class view contains only Rc<Original>, like the actual SDK views; no new
engine/state model exists. Two unit tests cover contracts/casts/nonmutation and rejected
incomplete projections. Existing model/application tests and wire goldens stay intact.
Original source30015 exits0 with17 callback cases (15+2); previous15 rows and46 other outputs
remain byte-identical. Runtime matches configured-constructor identity/count and repeated
error identity. An initial emitted-code `receiver:::absolute_path` tokenization issue in
49913 is fixed with explicit type-annotation spacing.15654 passes adapter63/consumer9/
runtime36; no behavioral expectations or reference data are weakened.

FINAL49691 is confirmed terminal0: all21 stages pass, **1,919 tests across55 suites**, zero
failed/ignored. Strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six framework provenance checks and both generators pass. Full completion is correctly
rejected. SSD logs/summary prefix ferrodock-receiver-final-. Selected source hashes are
recorded before the driver and verified after it. Only docs follow green; all owned Cargo/
reference handles are terminal. Inventory227/463/1238 of1928; full goal active/incomplete/
unblocked. New files intent-to-add only, index empty; no dependencies/versions/features,
framework/original sources, agents, commits, pushes or PRs. Protected actual desktop artifacts
preserved; all scratch/artifacts on SSD.

NEXT: resolve native inheritance/base-member schemas and receiver conversions from actual
SDK declarations; complete native actual-model/compiler/system/ancillary schemas and default/
application/backend catalogs, then real sample save/load. Base-property binding should be
separate from class constructor/identity requirements so abstract native view layers can
contribute their actual accessors without fabricated identities or constructors. The new
projection expects already-resolved hiding/overrides; it does not complete that semantic pass.
Fallible/parameterized constructors, conditional attributes, inherited enum APIs and all
remaining engine/API/lifetime/cycle collection/serializer/platform/accessibility/performance
scope stay required. Preserve the full original goal.

## Previous validated checkpoint

# Current: native fallible property accessors and persistent callbacks

Previous goal turn was PROGRESS (four-model layout properties,1909/all21). Actual native SDK
metadata uses Result accessors which the JSON adapter rejected. This turn adds six typed
try property APIs (read/write/readonly/writeonly and converter forms), preserves existing
infallible APIs, carries independent getter/setter fallibility through RustPropertyBinding,
and requires explicit RustAccessorErrorMapping::Preserve or Map. Existing target error
handles pass unchanged; foreign errors use a supplied typed function. No unwrapping,
runtime Rust reflection, inferred error conversion or replacement engine/model is introduced.

Mixed infallible callbacks wrap only their result. Their original expressions are evaluated
and captured once at metadata construction; state persists across serialization operations.
Original C# source42531 exits0 with15 callback/ordering/error-identity cases, retaining the
first14 probe cases and all46 preceding output files byte-identical. Runtime tests match
all15 traces, partial setter commits, skip/omit behavior, error identities and wires.
Independent consumers compile all six API forms, mixed signatures and an explicitly mapped
foreign error. Readonly and persistent-state probes execute against the actual fixture models;
all existing application/model wire and post-load engine cases remain intact. Missing
fallible callback declarations fail, while original ignored-member priority remains.

Initial11311 existing suites pass.73650 catches an i32->i64 test conversion and unused helper;
70413 passes runtime36/adapter61/consumer4.4329 new readonly probe wrongly feeds reserved $type
metadata to a preserved-class reader; the test now constructs the original named model for
writing and uses ordinary Name input for readonly reads. The existing reserved-metadata
rejection is unchanged.95245 typed mapping and16320 mapped foreign callback pass. Initial
full47563 passes1913 tests but Clippy rejects immediate closure calls; this leads to the
required capture-once correction and new stateful regression.80986 passes runtime36/
generator5/adapter61/consumer6 and targeted strict Clippy.16523 full1914 passes but its broader
Clippy catches the test module at the top of source_emitter; moved to the required file end.
All failed/intermediate logs are retained; no checks are weakened to obtain green.

FINAL59967 is confirmed terminal0: all21 stages pass, **1,914 tests across55 suites**, zero
failed/ignored. Strict workspace/all-target and selected release-library Clippy, eight
standalone suites, ten dependency boundaries, format,24 Python tests, source/theme/inspector
checks, six provenance checks and both generators pass. Full completion is correctly rejected.
SSD prefix: ferrodock-fallible-accessor-checked-. Exact source hashes are recorded before
this final driver and verified afterward; only docs follow green. All owned Cargo/reference
handles are terminal. Inventory227/463/1238 of1928; full goal active/incomplete/unblocked.
No dependency/version/feature/framework/original source edits, agents, commits, pushes or PRs.
New files intent-to-add only; index empty. Protected actual desktop executable/app/window
unchanged; all scratch/artifacts on SSD.

NEXT: actual native model receiver projection and inherited-property/constructor binding,
complete native model/compiler/system/ancillary schemas, full default/application/backend
catalogs and actual sample save/load. Fallible constructors and conditional attributes still
need their full contracts. Complete original engine/API, other enum/inherited APIs,
cycle collection/lifetime, serializers, platform/accessibility and performance remain open.

## Previous validated checkpoint

# Current: docking and pinned/root modes in layout JSON contexts

Previous goal turn was PROGRESS (open numeric docking enums, 1,909 tests/all21). This turn
extends the supplied actual-model and emitted JSON contexts with Dock on dockables, nullable
PinnedDockDisplayModeOverride, root PinnedDockDisplayMode and FloatingWindowHostMode. All use
original constructors/accessors/models and preserve undefined signed values. No replacement
model, engine, reflection path or retention cache is added. This remains the limited four-
model projection; complete default/application/backend catalogs and sample persistence are open.

Unchanged-original serializer run82557 exits0. Six additional original restored-model checks
pass (44->50), retaining all44 old checks. Root/list goldens add only the four scalar property
families; removing those additions preserves every previous property value/order/reference ID.
All43 other source outputs are byte-identical. Original wire order places root pinned mode
before Window/Windows, then floating host mode before root policy; both Rust contexts match
exact bytes. Undefined dock/pinned/host values and null/defined/undefined pinned overrides
survive actual factory InitLayout on the restored graph, with previous identity/cycle/list/
capability checks retained. Source evidence uses SSD dock-layout-source-output/ and
ferrodock-dock-layout-source-preservation.json.

Focused53414 exits0: serializer35 and emitted consumer4. Final83466 is confirmed terminal0:
all21 sequential validation stages pass, **1,909 Rust tests across55 suites**, zero failed/
ignored. Strict workspace/all-target and selected release-library Clippy, eight standalone
suites, ten normal boundaries, format,24 Python tests, source/theme/inspector checks, six
framework provenance checks and both generators pass. Full completion is correctly rejected.
Logs/summary prefix: /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-mode-final-.
All owned Cargo/source-reference handles are terminal; only documentation follows green.
Source/test hashes are recorded at the validated checkpoint. Inventory227/463/1238 of1928;
full original goal stays active/incomplete/unblocked. No dependencies/versions/features,
framework packages, original parent/sibling/cache sources, agents, commits, pushes or PRs.
Actual desktop executable/app/window protected. All scratch/artifacts remain on the SSD.

NEXT: expand actual native model/compiler schemas and full default/application/backend
catalogs, then actual sample save/load. The current four-model provided contexts still rely
on explicit bindings and original-reference semantic facts for their application models;
source metadata and identity-preserving ReactiveUI reference views need complete native
semantic/accessor/constructor binding. Remaining plain enums, inherited enum APIs, native
system/ancillary/dependency facts, full original engine/API/lifetime/cycle collection,
serializers, platform/accessibility and performance scope remain required.

## Previous validated checkpoint

# Current: open docking-mode values, native metadata and numeric JSON

Previous goal turn was PROGRESS (native enum headers, 1,900 tests/all21). This turn changes
actual DockMode, PinnedDockDisplayMode and DockFloatingWindowHostMode to transparent signed
Int32 value types. Original names/defaults, equality/hash/storage and checked named-member
TryFrom remain; from_raw/to_raw retain every signed cast and Display matches source names or
numbers. Existing native enum bridge automatically emits real typed factories for all three.
The native semantic host reads their actual transparent source/storage/factory signatures.
No alternate engine/model or retention cache. The guide's default closed enum spelling is
adapted to preserve original unnamed values safely, as for the two window enums.

Explicit Core JSON metadata now contains34 descriptors with the three codecs and nullable
forms. Original serializer reference62077 exits0; 423 seeded value/name/wire rows and102
read cases are copied. All44 earlier source output files compare byte-identical. Original
compiled XAML reference35148 exits0 with914 assertions; all908 earlier checks are retained.
Real RootDock and settings options carry unknown docking/pinned/host modes and nullable
values. Original host root/global precedence and managed predicate are checked. The forward
DockMode converter now preserves the source wildcard: Center/unnamed values return unset.
The native framework Dock type is still closed, so reverse unnamed-native fallback remains
explicitly deferred. Other enum APIs/catalogs/backends and full model projections remain open.

Source helper40042 initially missed newly added resources after a cwd error; corrected
fixtures/source runs61562 and35148 pass. Initial focused11357 exposes the now-required
converter match/default adaptation. 69857 then exposes a missing test trait import; fixed.
Final focused57047 exits0: model235, native UI373, markup191, JSON35, native adapter60,
settings34; no failed/ignored tests. Domain tests cover source numeric/named display,
equality/hash/storage and original constants/defaults. Settings precedence matrix now also
covers unnamed values and signed endpoints. Source/test code is frozen for final validation.

FINAL driver86671 is confirmed terminal0. All21 sequential validation stages pass:
**1,909 Rust tests across55 suites**, no failed/ignored tests; strict workspace/all-target and
selected release-library Clippy, eight standalone suites, ten normal dependency boundaries,
format,24 Python tests, source/theme/inspector checks, six framework provenance checks and
both generators. Full completion is correctly rejected. SSD logs/summary use prefix
ferrodock-dock-enum-final-. All owned Cargo/source-reference handles are terminal; only
documentation follows full green. Original source and actual desktop artifacts are preserved.
All scratch/artifacts on SSD; no dependency/version/feature/framework edits, agents, commit,
push or PR. New files are intent-to-add only; index empty. Inventory227/463/1238 of1928.
The full original goal remains active, incomplete and unblocked.

NEXT: persist original Dock and pinned/root host properties in
actual-model/runtime and emitted JSON contexts, then full native compiler/system/ancillary
and default/application/backend catalogs and actual sample save/load. Remaining closed enums,
complete inherited enum APIs, all original engine/API, cycle collection/lifetime, serializers,
platform/accessibility and performance scope remain required. Do not narrow the goal.

## Previous validated checkpoint

# Current: native enum dependency headers

Previous goal turn was PROGRESS: transparent source storage and semantic representation
validation passed all 21 stages with 1,899 tests. This turn advances the generated consumer's
native compiler host: six SDK contracts plus ICommand and five actual model enum headers now
occupy twelve fresh native slots. DockMode, PinnedDockDisplayMode,
DockFloatingWindowHostMode, DockWindowState and DockWindowOwnerMode are read from actual Rust
model declarations. Open enum factories are checked against real shared scanner signatures.
Original constants/value operations remain on the compiled models; no alternate engine exists.

Explicit source IDs unify old incoming references, but all twelve old headers/property rows
are discarded. System.Enum and its interface closure remain an explicit foreign dependency.
Properties and nullable pinned-display arguments remap to the native enum IDs. The poison
regression now corrupts all twelve discarded schemas; generation plans and symbols remain
identical. A new comparison checks all five enum identities, kinds, inheritance, source
locations and live property/nullable links. The host roots remain explicit, not an automatic
full catalog. Macro flags/ancillary/system symbols and automatic discovery remain incomplete.

29445 initial focused run exits101 only because an old test expected seven final slots;
it now asserts seven interfaces followed by five native enums. 23021 passes all60 adapter
tests; 50969 passes all4 generated-consumer tests. Final56061 is confirmed terminal0: all
21 sequential stages pass, **1,900 workspace tests across 55 suites**, zero failed/ignored.
Strict workspace/all-target and selected release-library Clippy, eight standalone suites,
ten dependency boundaries, format, 24 Python tests, source/theme/inspector checks,
six provenance checks and both generators pass. Full completion is correctly rejected.
Logs/summary: /Volumes/SSD/temp/ferrodock-port/ferrodock-native-enum-catalog-final-*.
All owned Cargo handles are terminal; only documentation/inventory follows full green.

The audit corrects three closed enum units from ported to partial: their named members and
semantic headers work, but unnamed numeric values and inherited APIs are incomplete.
Inventory is **227 ported / 463 partial / 1,238 pending of 1,928**; structural audit passes.
The full original goal stays active, incomplete and unblocked. New helper and NOTICE are
intent-to-add only; index empty. No source model/runtime/dependency/version/feature/framework
changes, original source edits, agents, commits, pushes or PRs. All scratch on the SSD;
actual desktop executable/app/window preserved.

NEXT: retain unnamed numeric values for DockMode, PinnedDockDisplayMode and
DockFloatingWindowHostMode in actual domain types/native metadata/JSON, with original-engine
switch/default behavior and source-backed tests. Continue native system/ancillary catalogs,
automatic compiler discovery, full default/application/backend catalogs and real sample
save/load. The complete engine/API, inherited enum, lifetime/cycle collection, serializers,
platform/accessibility and performance scope remains required. Do not shrink the goal.

## Previous validated checkpoint

# Current: source provenance for transparent numeric enums

Previous goal turn was a verified wait on driver24490. That same handle is now confirmed
terminal0: all 21 numeric-enum validation stages pass, with 1,896 workspace tests and no
failures/ignored tests. The full original goal remains active, incomplete and unblocked.

This turn also extends native source facts with normalized storage for a literal
repr(transparent) struct containing exactly one unconditional field. The semantic binder
accepts a nonflags logical enum only with actual signed i32 storage and an explicit shared
typed factory declaration. Named/tuple storage works; unsigned/default/C layouts, multiple
fields, conditional selections and absent/malformed factories fail without partial commits.
Tests read actual DockWindowState and DockWindowOwnerMode declarations. All 59 native
adapter tests pass, including three new tests. This validates representation, not automatic
catalog discovery or inherited System.Enum APIs. Inventory remains 230/460/1238 of 1928.

FINAL driver71621 is confirmed terminal0. All 21 sequential stages pass, with 1,899 Rust
tests across 55 suites and no failures/ignored tests. Workspace all-target and selected
release-library Clippy, eight standalone suites, ten dependency boundaries, format, 24 Python
tests, source/theme/inspector checks, six framework provenance checks and both generators pass.
Full completion is correctly rejected. Per-stage logs/summary use SSD prefix
ferrodock-transparent-enum-final-. Only documentation follows this green run; no source/test
behavior changes. All owned Cargo/validation handles are terminal.

Shared numeric XAML compiler work is fully validated: both hosts use the same emitter and
explicit actual-type factories. Original compiled XAML preserves Int32 narrowing on overflow;
JSON retains its range checks. Original reference passes 908 checks, retaining all previous
900. Full isolated framework suites pass 4,854 tests, with one pre-existing ignored diagnostic.
Complete patches replay against exact published bytes. No dependency/version/feature changes,
original source edits, agents, commits, pushes or PRs. Actual desktop size/mtime/SHA256 unchanged;
app/window protected. All scratch is on the SSD. New files are intent-to-add only; index empty.

NEXT: remove residual enum/system catalog seeds
using actual native declarations/dependency metadata and continue other enum representations,
full default/application/backend catalogs and real sample save/load. Complete engine/API,
inherited enum APIs, lifetime/cycle collection, serializers, platform/accessibility and
performance scope remain required. Do not mark the full goal complete.

## Previous validated checkpoint

# Current: open window numeric values, options and complete window JSON projection

Previous turn PROGRESS (1886tests/all21). Current turn changes actual domain value types,
native declaration/export bridge, Core JSON metadata, direct/emitted consumers, original
reference fixtures, settings assertions and structural auditing. Entire original Dock port
goal stays ACTIVE/incomplete/unblocked without reduced scope.

DockWindowOwnerMode/DockWindowState use transparent original Int32 values with original
named constants/defaults, equality/hash and size/alignment. from_raw/to_raw preserve any
signed value; checked TryFrom validates named members as before. Display matches source.
This documented adaptation of the guide's closed-enum spelling is required to preserve
source undefined values safely. The native bridge reads real enum or transparent literal
member declarations and retains original enum kind/handles/members. Runtime/nullable
conversions accept all Int32 values, rejecting outside-range i64. No unsafe, reflection,
substitute model/engine or retention cache. Header comment clarifies actual bridge parsing.

Core JSON metadata expands21->28 descriptors with two window enums, DockWindowOptions and
nullable dependencies. All4 options fields and ApplyTo retain original models/parent. The
actual ReactiveUI window projection covers all13 persisted properties, including unknown
state/owner, modality/taskbar/topmost flags and shared parent aliases; ignored Owner/Factory/
Host stay excluded. Other3 model classes remain limited projections and catalogs incomplete.
WindowEnumReference adds7 assertions,282 original value/name/wire rows (128 seeded samples
per type),50 reads,two goldens; model assertions41->44. Direct/generated tests preserve exact
source wire/IDs, original options ApplyTo and parent identity. All existing earlier cases stay.

Structural auditing now verifies pure enum declaration constants even with extra metadata/
test targets; strict transparent storage, missing/wrong/duplicate constants and prior bypass
have3 new Python regressions (24total). MDI resize record now lists exact source members;
existing signed flags behavior and its broader numeric metadata gap remain unchanged.
The two window enums are corrected to PARTIAL pending shared compiled XAML unknown-literal
emission, complete inherited enum API and automatic native compiler representation facts.
Inventory230/460/1238 of1928; no new source promoted full.

69359 Model227/CoreSDK62 initial passes;21107 initial source,40183 expanded source and79251
final source all terminal0.77930 passes Model229/CoreSDK63/consumer4/JSON33. Initial full98588
fails old settings rejection of all unknown enums; assertions now require original raw
roundtrips for two corrected types and preserve explicit gaps for other plain enums.
42789 settings33 terminal0.37663 driver was started before42789 terminal confirmation;
Cargo serialized via its build-directory lock, then42789 confirmed terminal. Do not repeat
that scheduling mistake: await confirmed terminal status before starting another Cargo job.
FINAL37663 terminal0 passes **1894 Rust tests/55suites**,zero failed/ignored; all21 validation
stages pass. SSD driver/summary/per-stage prefix ferrodock-window-enum-final-; initial failure
logs preserved under ferrodock-window-enum-initial-. Only docs/porting/dock/bridge comment follow green;
no behavioral or test changes. Builds use -j1, all owned command handles now terminal.

Only FerroDock/owned SSD temp edited; no agents/commit/push/PR/dependency-version-feature/
framework edits. Parent/sibling/cache source read-only; actual desktop executable/app/window
protected, no rebuild/restart. TMPDIR/artifacts on /Volumes/SSD/temp/ferrodock-port.

NEXT: implement genuine shared typed compiled numeric-enum literal support (current emitter
only emits named nonflags members; do not falsely mark enums as flags or add guessed calls),
automatic provenance for transparent enums, remaining plain enums/fields/Core options and
full native compiler/system/ancillary facts plus all default/application/backend catalogs,
then actual sample save/load. Complete inherited enum APIs, original engine/API/backends/
serializers, cycle collection, platform/accessibility and performance scope remain required.

## Previous validated checkpoint

# Current: get-only values, shared docking options and signed flags

Previous goal turn was PROGRESS (Core bounds/policies,1883tests/all21). Current turn changes
production code, source references, real-model/emitted consumers and tests; entire original
goal stays ACTIVE/incomplete/unblocked, with no scope reduction.

JsonValuePropertyInfo now supports actual get-only value members. New numeric_enum codec
uses original numeric Int32 JSON directly, independently of custom scalar converters, and
retains all undefined signed bits. Core collection expands11->21 descriptors with DockPoint,
DockManagerOptions, DockingWindowState, DockOperationMask, DragAction and nullable forms.
Point reads retain original default construction and skip get-only component input; it does
not infer the parameterized constructor or force a lossless roundtrip absent in the source.
Manager options retain actual default true, explicit false, original shared Rc and mutation.
No reflection, substitute models, retention cache or alternate engine is introduced.

CoreValueReference adds ten original assertions,490 read/replay rows and four goldens;
128 seeded values per flags type cover full signed bits alongside boundaries/token failures.
Custom Int32 metadata alters integers but leaves enums unchanged. Existing core/model corpus
matches final source bytes. Actual model projections now include DockingState and concrete
operation restrictions. Runtime and generated contexts retain original wire order/IDs and
operation-mask behavior; optional restriction fields are excluded from base-interface
fallback metadata. Model source assertions expand38->41; previous38 retained.

21111 original source passes initial values;63423 exposes wrong fixture assumption that
initialized documents are Docked (source is Document=4);69869 validates corrected assertion.
12567 final source terminal0 passes all checks including integer-codec independence.
93023 JSON29 passes.41699 expanded consumer fails test-host absolute type lexical spacing;
fix keeps source types and uses correct colon/path separation.3014 passes4/29;64253 final
consumer4/JSON30 passes. FINAL18971 terminal0 passes **1886 Rust tests/55suites**,zero failed/
ignored; all21 stages pass. Owned SSD driver/summary/per-stage prefix:
ferrodock-core-value-final-. No production/test changes follow full green. Inventory remains
232/458/1238 of1928, no source promoted complete. Full completion still correctly rejected.

Only FerroDock/owned SSD temp edited. No agents/commit/push/PR/dependency-version-feature/
framework changes. Parent original source, sibling FerroUI and package cache sources read-only.
Actual desktop executable/app/window protected, no rebuild/restart. Scratch/TMPDIR/artifacts
on /Volumes/SSD/temp/ferrodock-port; Cargo sequential -j1. All owned jobs terminal.

NEXT: complete plain-enum numeric representation/codecs, remaining original model fields and
Core/options types, full automatic native compiler bindings/system/ancillary facts and all
default/application/backend catalogs, then actual sample save/load. Original API, all engines/
backends/serializers, cyclic lifetime, platform/accessibility and performance remain required.
Preserve original reference semantics, existing source/tests and the protected actual desktop.

## Previous validated checkpoint

# Current: Core bounds/policies and expanded real-model serialization

Previous goal turn was a status-only update (no progress); current turn makes authoritative
code/test/reference changes and reaches a new fully validated checkpoint. Entire original
Dock-to-FerroUI goal stays ACTIVE/incomplete/unblocked, with no reduced scope.

New core_model_type_metadata() explicitly registers 11 actual Core/nullable descriptors.
DockRect uses original mutable fields and copied values through JsonValuePropertyInfo and
JsonTypeInfo::value_object; no reference IDs or wrapper models. Policies/overrides retain
original constructors/accessors, six nullable flags, ignored HasAnyOverride and compiled
base/interface reference casts. The interface has no fabricated constructor. A base-first
read cannot recover a derived override, matching C#. Class object reads reject unexpected
reserved dollar properties even when declared. The generator composes explicit dependency
metadata collections and emits an immutable local when no extension is needed. Supplied
contexts remain explicit; no default catalog is installed implicitly.

Actual ReactiveUI model projections now include pinned bounds, six base capability flags,
overrides and shared root/dock policies in original property order. Runtime and compiled
emitted-context consumers roundtrip exact source wire/reference IDs, initialize through the
original Factory and evaluate restored policy precedence/shared mutation through the original
DockCapabilityResolver. Core source probe adds 10 assertions, 49 read/replay cases and six
goldens. Existing model probe adds five assertions (38 total); original33 retained. C# source
probe compiles unchanged originals, with only its explicit fixture projection expanded.

Reference82200 terminal0 predates expanded fixture;91837 fails missing resolver namespace;
27531 fails wrong evaluation property spelling;18873 terminal0 validates final expanded source.
Rust98558 initially reveals newly retained six source base flags; original accessors/fields
added to both projections without weakening goldens.34599 passes consumer4/JSON27/compiler4;
49860 strict changed-package all-target Clippy passes. FINAL37200 terminal0 passes
**1883 Rust tests/55suites**,zero failed/ignored; all21 sequential validation stages pass.
Owned SSD driver/summary/log prefix: ferrodock-core-model-final-. No production/test edits
follow green. Inventory232/458/1238 of1928 unchanged; no source unit promoted complete.

Only FerroDock/owned SSD temp edited. No agents/commit/push/PR/dependency-version-feature or
framework changes. Original parent/sibling/cache sources read-only. Actual desktop executable/
app/window protected, no rebuild/restart. TMPDIR/artifacts on /Volumes/SSD/temp/ferrodock-port;
Cargo sequential -j1. All owned jobs terminal. Docs/inventory now record exact supported scope.

NEXT: continue original value/enum/options codecs and full default/application/backend catalogs,
automatic native system/ancillary/compiler facts and exhaustive bindings, then real sample
save/load. Owning Rc cycle collection and broad original API/serializer/platform/accessibility/
performance contracts remain unfinished. Preserve the entire engine and original API, existing
source references, all tests and the actual desktop; no substitute model/snapshot/cache engine.

## Previous validated checkpoint

# Current: atomic dependency import and seven fresh native contract IDs

Goal turn PROGRESS. Full original goal ACTIVE/incomplete/unblocked. Previous native contract
turn changed authoritative code and passed1872/55/all21; current source and terminal records
inspected. New production import_semantic_dependencies imports selected foreign structural
closure, remaps types/properties/overrides/assemblies and reserves native IDs in actual source
order. Old replaced headers/members are never inspected or copied. Binding occurs privately
without dummy headers; foreign lists can refer forward to native IDs. Source input immutable;
malformed/missing links, excluded native override rows, binding errors, missing headers and
invalid final links return errors before publishing a compilation. Registration/activation/
discovery/name state starts empty; roots and external/native identities are explicit.

Payload host now allocates six SDK contracts and actual locked ReactiveUI ICommand in SEVEN
new slots, then all9 application schemas. Both application and replaced dependency schemas/
slots are discarded; native source IDs only unify references. All81 original Dock properties,
interface relationships, command ignores and wire/engine consumer checks remain covered.
ICommand is the actual published method/event-only interface, with native assembly/version/
location and no invented runtime object or duplicate trait. Host reads the single crates.io
package selected by Cargo.lock version and cached normalized manifest name/version, then
canonicalizes the public reexport through the shared ModelSet. Cache and sibling sources
read-only. Generic vendored/path/multiple-version source discovery remains deferred.
System/ancillary facts and IgnoreDataMemberAttribute still explicit C# test-reference inputs.

Seven new tests cover full selected generic/array/tuple/nested/property remapping, unique
native aliases, no placeholder headers, late failure/immutable source, malformed/final links,
excluded override rebinding, new slot ordering and actual command/list identity. Existing
contract poison test now corrupts all7 complete headers/kinds/names/assemblies/properties and
compares activated native plans. Type/property comparisons retain logical identities under
new numeric IDs; real native assembly identities are checked separately. No wire/engine
assertion or original C# source corpus changed. Former duplicate test-only dependency remapper
now calls the production importer. No normal deps, versions or features changed.

68893 initial import consumer4/adapter49 passes. Offline cargo metadata fails only for an
uncached non-Mac platform package; no download attempted.83858 reveals absent legacy
.cargo-checksum.json in this Cargo cache; lookup now uses actual locked registry/name/version
rather than assuming that file exists.64839 exposes public versus canonical command spelling;
shared ModelSet fixes it,50148 consumer4/adapter49 passes.21554 and80022 pass4/56 after import/
command regressions and portable home/path handling. Initial strict Clippy reports only
needless closure borrows; fixed without allowances.99509 strict changed-package/all-target
Clippy passes.20008 final4/56 passes after activated-plan assertion.

FINAL18632 terminal0 passes **1879 Rust tests/55suites**,zero failed/ignored; all21 stages zero
in owned SSD ferrodock-native-import-final-validation-summary.json. Strict workspace/debug/
all-target and selected release-library Clippy,8 standalone packages,10 normal dependency
boundaries,format/21Python/source/theme/inspector/two generators/six provenance guards and
expected full-completion rejection pass. Adapter56/consumer4/compiler4/JSON23/CoreSDK62 and
all existing engine/native UI/sample suites pass. No production/test edits follow full green.
Inventory232/458/1238 of1928 unchanged; generator remains partial.

Only FerroDock/owned SSD temp edited; parent/sibling/cache source read-only. No agents/commit/
push/PR. Actual desktop executable/app/window protected; no rebuild/restart. Scratch/TMPDIR
/Volumes/SSD/temp/ferrodock-port. Cargo sequential -j1. No cleanup/other-project artifacts or
jobs changed. Inventory statuses unchanged; no source unit promoted complete.
Final lightweight format/source/inventory/factory/new-code checks pass. New files intent-to-add;
staging empty. Actual desktop size/mtime/SHA256 preserved after full validation in owned SSD
ferrodock-native-import-desktop-preserved.json; final-checks.json records evidence. All owned
jobs terminal; entire original goal stays active/incomplete.

NEXT: native system/ancillary symbols and full export/source/cfg/macro/incremental/constraint/
projection/alias/override/cast integration, complete original codecs and default/application
catalogs, then actual sample save/load. Keep entire original engine/API/model backend/serializer/
platform/accessibility scope. Owning Rc cycle collection, CLR/UTF16/error/metadata/stream and
performance contracts remain required. Full goal stays active/incomplete.

## Previous validated checkpoint

# Current: six native Dock contract schemas and interface provenance

Goal turn PROGRESS. Entire original goal ACTIVE/incomplete/unblocked. Previous native
application allocation turn changed authoritative code and passed1867/55/all21; source and
terminal records inspected. FerroUI.System has no IgnoreDataMemberAttribute declaration;
that missing dependency remains explicit rather than inventing a native symbol.

metadata_definition resolves actual single dyn Trait metadata carriers to declared traits,
without receiver/nullable/shared-handle erasure or guessed properties. Ordinary semantic
binding validates that exact source and treats an interface base handle as an edge in the
resolved interface closure, never class inheritance. Unsupported/inconsistent forms fail
before commit. The payload compiler host now reconstructs six required Dock interface
headers and all81 declared properties from actual Core SDK metadata. Reference application
schemas remain discarded; residual dependency slots, ancillary types and command/system
symbols remain explicit C# test-reference facts. Required logical identities/native assembly
and native handle-to-wire-type decisions remain explicit compiler policy. No alternate
models/engine or runtime reflection/objects are introduced.

Native schemas match every original property name/order/type/flag and interface closure,
command ignore set and selected payload plan. SDK root metadata order now matches original
EnableAdaptiveGlobalDockTargets after ExitWindows; compiled runtime descriptor order has an
assertion in the existing list/identity test. Five adapter tests also poison old interface
schemas, preserve missing-attribute DSTJ004, reject invalid dyn provenance and distinguish
interface/class bases atomically. Normal dependency boundaries/features/versions unchanged.

Initial61004 identifies shared canonicalization differences between explicit keys and
metadata uses;99884 passes consumer4/adapter44 after canonicalizing both through the scanner.
88960 exposes shared test-host self-import; explicit parent adapter alias fixes it without
new deps.28735 passes CoreSDK62/consumer4/adapter49.92501 changed-package strict all-target
Clippy terminal0.81417 passes the same62/4/49 after compiled root-order assertion. Original
C# compiler/serializer source corpora and all wire/engine assertions remain unchanged.

FINAL21504 terminal0 passes **1872 Rust tests/55suites**,zero failed/ignored, and all21 stages
in owned SSD ferrodock-native-contracts-final-validation-summary.json. Workspace/debug/all-
target strict Clippy, selected release-library strict Clippy,8 standalone packages,10 normal
dependency boundaries,format/21Python/source/theme/inspector/two generators/six provenance
guards and expected full-completion rejection pass. CoreSDK62/consumer4/adapter49/compiler4/
JSON23 and all existing original engine/native UI/sample suites pass. No production/test
edit follows full green. Inventory232/458/1238 of1928 unchanged; generator remains partial.

Only FerroDock/owned SSD temp changed; parent and sibling
sources read-only. No agents/commit/push/PR. Actual desktop executable/app/window protected,
no rebuild/restart. TMPDIR/scratch on /Volumes/SSD/temp/ferrodock-port; Cargo sequential -j1.
SSD availability varied~353MB to8GB; no cleanup or other-project jobs/artifacts changed.
Inventory statuses unchanged; generator and contracts remain partial.
Final lightweight source/inventory/factory/format and new-code scans pass. New files have
intent-to-add entries; staging remains empty. Actual desktop size/mtime/SHA256 preserved after
full validation in owned SSD ferrodock-native-contracts-desktop-preserved.json; final-checks.json
records evidence. All owned jobs terminal; entire original goal active/incomplete.

NEXT: complete native dependency slots/system/command/ancillary type graphs and logical
assembly identities, exhaustive source/cfg/macro/incremental and generic/override/cast
integration; original codecs/catalogs and actual sample save/load. Keep entire original
engine/API/backend/serializer/platform/accessibility scope. Owning Rc cycle collection,
UTF16/error/metadata/stream/performance contracts remain unfinished.

## Previous validated checkpoint

# Current: native application allocation and metadata-declared discovery

Goal turn PROGRESS; entire original goal ACTIVE/incomplete/unblocked. Previous turn was a
status update (no production change); current source and terminal validation records inspected.
Native bind_semantic_declarations now allocates fresh application IDs and binds complete
headers/members in a private working compilation. Forward/mutual properties and forward closed
generic arguments are supported without placeholder symbols. Closure validates after all
headers. Inheritance/containing/interface dependencies must precede dependents; original
property override rows remain explicit. Duplicate identities, invalid members/references,
open arguments and late errors roll back the entire input. Source registration/discovery
is separate from the atomic binder.

Payload consumer now discards ALL C# application types/properties/assemblies/slots/registrations/
discovery/name entries. Actual Rust metadata-declared classes in source traversal order plus
closed constructions from actual registrations allocate all9 IDs. Only an explicit remapped
transitive dependency contract/builtin closure remains from the C# test reference. Logical
wire identities, abstraction/base/accessibility and native handle mappings remain explicit.
This is NOT complete native dependencies, exhaustive discovery or a full model catalog.

Initial1051 fails because reference metadata locations use the literal "metadata", not None;
external-assembly/location guard corrected without allowing application facts.26836 exposes
unresolved generic template parameters in definition lookup; template candidates now use
actual validated instantiation rather than a logical-name match or forged resolved path.
10338 consumer4/adapter42 pass. Added discarded-source poisoning and actual override tests;
68044 consumer4/adapter44 pass.91265 changed-package all-target strict Clippy passes.
Eight batch regressions added; original wire/identity/engine assertions remain unchanged.

FINAL15708 terminal0: **1867 Rust tests/55suites**,zero failed/ignored; all21 validation
stages zero in owned SSD ferrodock-native-batch-final-validation-summary.json. Workspace
strict all-target/debug and selected release-library Clippy,8 standalone packages,10 normal
dependency boundaries,format/21Python/source/theme/inspector/two generators/six provenance
guards and expected full-completion rejection pass. Adapter44/consumer4/compiler4/JSON23
and all existing engine/UI/sample suites pass. No production/test edit follows full green.
Inventory232/458/1238 of1928 unchanged; generator remains partial.

Only FerroDock and owned SSD temp edited; original/sibling source read-only. No agents/commit/push/PR.
Actual desktop executable/app/window preserved, no rebuild/restart. All scratch/TMPDIR on
/Volumes/SSD/temp/ferrodock-port; Cargo sequential -j1. Entire source/API/backend/platform/
serializer/lifetime scope remains required.

Next: construct dependency symbols from actual source/shared metadata (Core/ReactiveUI and
FerroUI system contracts), retaining truthful required-symbol diagnostics and original logical
identity/assembly facts. Complete source discovery/conditional/macro/incremental semantics,
automatic codec/catalog integration, then actual sample persistence. Full original runtime
UTF16/error/metadata/stream/performance, owning Rc cycle collection, other serializers and
model backends, API/platform/accessibility parity remain required.

Final lightweight source/inventory/factory/format and new-code scans pass. New files have
intent-to-add entries; staging remains empty. Actual desktop size/mtime/SHA256 preserved after
the full run in owned SSD ferrodock-native-batch-desktop-preserved.json; final-checks.json
records evidence. All owned jobs terminal; entire original goal remains active/incomplete.

## Previous validated checkpoint

# Current: closed generic schemas and typed bindings construct natively

Goal turn PROGRESS. Entire original goal ACTIVE/incomplete/unblocked. Previous ordinary
semantic-symbol checkpoint, source and all21 terminal0 checks inspected before this work.

New generic_metadata.rs instantiates a shared generic template through parsed Rust type
syntax. Actual source parameter order/type kinds and constructed definition/arity are checked.
Substitution covers handles/properties/base/interfaces/signatures/type attributes and typed
callable arguments/call forms; literal attributes/member names remain unchanged. Public/stated
parameterized paths retain their exported spelling rather than becoming inaccessible canonical
paths. Source template stays unchanged. Unknown cfg, const/lifetime parameters, ambiguous
parameter-associated projections, macros and inferred/opaque types fail without guessed facts.
Actual Rust definition retains bounds/constraints; emitted calls check those, not a new complete
constraint solver. Nested/captured generic mapping and all alias/projection modes remain open.

bind_closed_generic_semantic_type validates actual closed metadata arguments/source arity,
logical metadata arity and recursively closed source type IDs before the shared semantic
constructor commits. Source identity/abstraction/access/base/ordered interfaces/overrides still
explicit. Common implementation preserves ordinary field/ignore/static/override validation.
The integer/string Generic payloads now instantiate ONE actual native metadata template and
use original Generic::new/value/set_value functions. Manual integer/string schema/getter
branches removed. All9 payload class schemas/bindings now native; dependency/type-slot and
source-discovery seed still uses C# snapshots. NOT snapshot-free compilation/default catalogs
or sample persistence. Original engine model/factory projection stays explicit/limited.

Initial47381 fails only build closure type inference E0282 after choosing semantic binder;
explicit RustType argument fixes it.99400 consumer4 terminal0.65977 consumer4/adapter36 pass
(6 new generic tests).4970 strict changed-package Clippy terminal0. Added parameterized
public/stated path specialization + test to preserve export spelling, then14056 consumer4/
adapter36 pass. Source unit/parser/engine behavior and assertions not weakened.

FINAL71032 terminal0: **1859 Rust tests/55suites**,zero failed/ignored. All21 stages zero in
owned SSD ferrodock-generics-final-validation-summary.json: workspace/debug/all-target and
selected release Clippy;8 standalone packages(adapter36/consumer4/compiler4/JSON23/CoreSDK62/
recyclingSDK2/ReactiveSDK7/runtime3);10 normal boundaries;format/21Python/source/theme/
inspector/two generators/six provenance guards and expected full-completion rejection.
No production/test edit follows full green. Source compiler19/152 and serializer numeric/model/
payload/policy/attribute corpora unchanged. Inventory232/458/1238 of1928 unchanged; no source
unit promoted complete. No dependency/version/feature changes; no visit-mut/clone AST feature
added. Native generic code uses existing parser features and actual typed original accessors.

Only FerroDock and owned SSD temp edited. Parent original/sibling FerroUI sources read-only.
No agents/commit/push/PR/staged contents. New files intent-to-add after final checks. Actual
native desktop executable/app/window protected; no rebuild/restart. All temp/TMPDIR on SSD.
Cargo sequential,-j1; no cleanup/other project process/artifact changes. All owned jobs terminal.
Shared free space varied~0.6-4.8GB but every build completed normally. Final source/staging/
format/inventory/desktop size/mtime/SHA256 preservation checks follow below.

NEXT: eliminate snapshot type-slot/dependency/discovery seeds; construct complete native
compilation from real source/shared dependency metadata. Need nested/captured generic arguments,
constraints/projections/alias and emission-scope access, source logical identity/abstraction/
inheritance/overrides/casts, target cfg/macro expansion/incremental dependent rebinding. Do not
infer identity from markup aliases or abstractness from absent ctors. Then exhaustive original
fields/value/constructor/converter codecs, full default/application catalogs and actual sample
save/load. Entire engine/API/backends/serializers/platform/accessibility scope stays intact;
CLR/UTF16/error/metadata/HandleNull/JsonDocument/stream/performance and owning Rc cycle
collection remain unfinished. No reflection/model copies/retention caches as substitutes.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-generics-desktop-preserved.json; final-checks.json records lightweight
evidence. All owned jobs terminal; entire goal remains active and incomplete.

## Previous validated checkpoint

# Current: ordinary class/property semantic symbols construct from native declarations

Goal turn PROGRESS. Whole Dock-to-FerroDock goal ACTIVE/incomplete/unblocked. Previous turn
source declaration extraction and all21 terminal0 checks were inspected before continuing.

New semantic_symbols.rs constructs non-generic class/value/enum/interface TypeSymbol and
public instance/static PropertySymbol records from actual shared metadata + source provenance.
Names/types/IgnoreDataMember source tags stay pre-JSON-renaming. Logical serialization identity
is EXPLICIT, separate from markup alias; native source representation/kind is checked. Resolved
assembly/context accessibility/abstraction/containing/base/complete ordered interfaces/actual
override links remain explicit compiler facts, not inferred from absent ctors or partial graphs.
Missing types/unsupported generic/cfg/registered/indexer shapes, mismatched inheritance/interface
closures/static overrides and cycles fail before type/property commit. Fresh self refs supported.
Existing type rebind appends property symbols; incremental dependent/override rebinding remains
HOST responsibility, not implemented by this local construction API.

Seven ordinary application payload classes now replace their old C# header/name/property schema
with native records. Native declaration aliases explicitly preserve fixture wire names; native
assembly identity comes from scanned name + Cargo version. Consumer still seeds dependency/type
slots and two closed generic examples from source snapshots. NOT snapshot-free production
compilation/default catalogs/sample persistence. Existing actual ReactiveUI/Factory model
roundtrip and all source wire/identity tests remain unchanged and pass. No copied docking state,
replacement engine, runtime reflection or retention cache introduced.

Initial51381 consumer4 passes.29459 passes consumer4/adapter28 (7 new semantic tests). Added
logical-kind/abstraction/closure checks ->adapter29.26558 Clippy fails duplicate_mod because
compiler JSON facts loaded twice; fixed via one test-only compiler_fixture_input module, no
lint allowance.3003 consumer4/adapter29 pass. First full89540 terminal0 passes1852/55/all21.
Review then identifies shared markup name need not equal serialization identity; explicit
metadata_name/namespace options + independent identity/atomicity test fix this BEFORE final
checkpoint.45489 consumer4/adapter30 pass. No assertion/source behavior was weakened.

FINAL75564 terminal0: **1853 Rust tests/55suites**,zero failed/ignored. All21 zero in owned SSD
ferrodock-semantic-symbols-identity-final-validation-summary.json: strict workspace/debug/all-
target/selected release Clippy;8 standalone packages(adapter30/consumer4/compiler4/JSON23/
CoreSDK62/recyclingSDK2/ReactiveSDK7/runtime3);10 normal boundaries;fmt/21Python/source/theme/
inspector/two generators/six provenance guards and expected full-completion rejection.
No production/test edits follow final green. Existing source compiler19/152, generated payload
20assertions/25reads/8goldens, policy15write/16read/5graph, attribute7 and model/number/factory
corpora unchanged. Inventory232/458/1238 of1928 unchanged; generator remains partial.

Only FerroDock/owned SSD temp edited; original parent/sibling FerroUI source read-only. No
agents/commit/push/PR or staged contents. New files intent-to-add after final checks. Actual
native executable/app/window protected, no rebuild/restart. All temp/TMPDIR on SSD. Cargo
sequential,-j1; no cleanup or other project jobs/artifacts changed. Shared SSD free space
varied~0.6-1.6GB but all jobs completed normally. No dependency/version/feature changes this turn.
Final source/staging/format/inventory/actual desktop size/mtime/SHA256 checks follow below.

NEXT: eliminate snapshot type-slot/dependency seeds, construct complete native compilation and
closed-generic symbols/substitution, source logical abstraction/identity/scope/inheritance/
overrides/casts from real declarations + shared metadata. Resolve target cfg/macro expansion
and incremental/dependent graph rebinding. Do not equate markup aliases with serialization
identity or absent ctor with abstractness. Then exhaustive field/value/constructor/converter
bindings, full default/application catalogs and actual sample save/load. Keep entire original
engine/API/backends/serializers/platform/accessibility scope; Runtime UTF16/error/metadata/
HandleNull/JsonDocument/stream/performance and owning Rc cycle collection remain unfinished.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-semantic-symbols-desktop-preserved.json; final-checks.json records
lightweight evidence. All owned jobs terminal; entire goal remains active and incomplete.

## Previous validated checkpoint

# Current: native Rust source declaration facts enter compiler adaptation

Goal turn PROGRESS. Entire Dock-to-FerroDock goal ACTIVE/incomplete/unblocked. Previous turn
implemented/validated activation and registration; its source/checkpoints/all21 terminal0
results were inspected before this step. Original engine/API/full-backend/platform scope intact.

New source_declarations.rs reads the actual shared scan file/module graph: native struct/
enum/trait/union kind, declared visibility, type/lifetime/const parameters and source bounds/
defaults, where clauses, explicit trait implementations/supertraits, file/line/column and
inherited cfg/cfg_attr. Handles inline/out-of-line literal #[path] modules/raw identifiers;
production test-only rules match shared cfg(test)/all(test,...) exclusion. Unknown conditions
are not evaluated; ambiguous conditional definitions are not selected. Wrappers/type aliases,
macro expansions and logical abstract-class/source property semantics are not guessed.

Public reexports canonicalize through shared ModelSet. Default uses scanned crate; overload
accepts real source/dependency metadata and rejects a set omitting the source crate. External
facts not supplied remain as given/unresolved. Actual Core IRootDock->IDock->IDockable source
traits and a referenced Rust source export fixture exercise identity. Payload consumer now
sets visibility, generic declaration arity and source locations from actual Rust definitions.
It STILL uses explicit/C# snapshot logical class abstraction/names/property/contract graphs;
this is not complete Rust symbol construction or default/application catalog discovery.

Initial70208 terminal0: adapter18 source/basic cases pass. Consumer47243 passes4 but adapter
Core trait test fails (19/20) on reexport alias identity. Focus45019 confirms actual supertrait
::ferrodock_model::core::IDock versus declaring ::core::i_dock::IDock, both resolved. Added
shared ModelSet canonicalization;81046 consumer4/adapter20 pass unchanged assertion. Added
dependency model fixture/check and unified production test-only rules across registration/
declaration readers.97333 strict changed-package Clippy passes before that final cfg extension.
Final targeted11464 terminal0: consumer4/adapter21. No source-engine tests/assertions weakened.

Full43887 terminal0: **1844 Rust tests/55suites**,zero failed/ignored. All21 stages zero in
owned SSD ferrodock-rust-declarations-final-validation-summary.json: strict workspace/debug/
all-target and selected release-library Clippy;8 standalone packages(adapter21/consumer4/
compiler4/JSON23/CoreSDK62/recyclingSDK2/ReactiveSDK7/runtime3);10 normal boundaries;fmt/
21Python/source/theme/inspector/two generators/six provenance guards and expected complete
port rejection. Existing original source/compiler/payload/policy/attribute corpora unchanged.
No production/test source edit follows full green. Inventory232/458/1238 of1928 unchanged.

Only FerroDock and owned SSD temp edited; parent originals/sibling FerroUI sources read-only.
No agents,commit,push,PR or staged contents. New files intent-to-add only after final checks.
Actual native desktop executable/app/window protected; no rebuild/restart. All TMPDIR/temp
storage on /Volumes/SSD/temp/ferrodock-port; no cleanup or other project artifacts/processes
changed. Cargo sequential,-j1. Direct proc-macro2/span-locations reuses already-locked scanner
version/features; no native/runtime/backend dependencies added. All owned jobs terminal.
Final source/staging/format/inventory and actual desktop preservation checks follow below.

NEXT: complete semantic symbol builder from actual source + shared/dependency metadata,
retaining logical class/abstract/type identity, source property ignores/overrides/inheritance,
closed generic substitution and real cast facts. Need emission-scope/export accessibility,
target cfg/macro expansion and incremental host. Do not infer abstractness from absent ctor
or publish a C# snapshot as the production catalog. Then complete all model-field/value/
constructor/converter binding, default/application catalogs and real sample save/load.
Runtime metadata/UTF16/error/HandleNull/JsonDocument/stream/performance/other serializers and
backend/platform/accessibility parity remain open; owning Rc cycles still lack source cycle
collection. Do not substitute copied models, reflection or retention caches.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-rust-declarations-desktop-preserved.json; final-checks.json records
lightweight evidence. All owned jobs terminal; entire goal remains active and incomplete.

## Previous validated checkpoint

# Current: Rust activation/registration source feeds generated JSON contexts

Goal turn PROGRESS. Full goal ACTIVE/incomplete/unblocked. Previous goal turn made and
validated the shared declaration adapter; authoritative source and all21-stage terminal0
summary were rechecked before continuing. Native actual executable/app/window protected.

Ported original activation and serializable attribute constructors/type getter as shared
reference classes. Logical Type uses native TypeId, including retained null; no registered
type construction. New macros dock_json_source_generation! and dock_json_serializable!
retain typed constructor references as immutable declarations, with no runtime registry or
model construction. Fixed activation marker rejects duplicates in one Rust scope; repeated
registrations are allowed. Complete shared TypeInfo/CLR Attribute/runtime assembly metadata
and AttributeUsage exposure remain partial; these two source inventory units stay partial.

New generation_attributes.rs parses actual crate-scope source using syn + shared scanner
module/path resolution. Qualified and renamed imports resolve by identity, declaration order
and duplicate registrations persist, locations retain file/line/column. Nested markers,
malformed/duplicate activation/unknown types/unselected cfg fail. Exact cfg(test) modules/
functions are excluded; arbitrary macro bodies are not expanded. apply_generation_attributes
requires real logical type-to-symbol facts and mutates Compilation only after resolution.
The existing semantic generator owns source validation/activation diagnostics/deduplication.

Payload consumer now compiles json_registration.rs into its actual crate. Its build script
reads that SAME fragment and discards the source fixture's old activation/registration list.
Nine parsed actual Rust registrations, including2 closed generics, feed original semantic
selection and emission. Compiler class/accessibility/generic facts and Rust symbol mapping
still come from explicit fixture/adapter inputs; full automatic semantic discovery/default
catalogs/sample persistence are NOT complete. Existing original model/Factory roundtrip tests
remain unchanged; no copied engine/state/retention cache/reflection introduced.

Initial36290 terminal101: included file inner-doc error; corrected comment. Retry65310 passes
consumer4/JSON23/adapter13 but fails one test's hand-counted location (actual1:42,expected1:43).
Corrected that assertion and expected missing-activation ID to source DSTJ002 (read original
builder). No implementation/assertion contract weakened. Final50197 terminal0: consumer4,
JSON23,adapter14, duplicate-activation compile-fail doctest1. Source20536 terminal0 executes
unchanged original attributes and adds7 constructor/type/identity outcomes in attributes.tsv.
Other serializer/model/number/payload/policy corpora regenerate unchanged scenarios. Strict
changed-package Clippy91712 terminal0.

Full44357 terminal0: **1837 Rust tests/55suites**,zero failed/ignored, including duplicate
activation compile-fail doctest. All21 checks pass in owned SSD
ferrodock-json-registration-final-validation-summary.json: debug workspace/all-target and
selected release-library Clippy;8 standalone packages (adapter14/consumer4/compiler4/JSON23/
CoreSDK62/recyclingSDK2/ReactiveSDK7/runtime3);10 normal boundaries;fmt/21Python/source/theme/
inspector/two generators/six provenance guards and expected full-completion rejection.
No semantic/production/test edits follow full green. Inventory now232/458/1238 of1928:
2 previously pending attribute units are partial; no unit promoted complete. All owned jobs
terminal. Compiler only adds direct quote/syn and dev serde_json dependencies already locked
through existing scanner/compiler use. No package versions or native source deliberately changed.

Only FerroDock and owned SSD temp edited. Parent original/sibling FerroUI sources read-only.
No agents/commit/push/PR or staged contents. New files intent-to-add after final checks. All
TMPDIR and NuGet package/HTTP/temp storage remain on SSD. No cleanup or other project jobs/
artifacts changed. Actual desktop size/mtime/SHA256 final preservation follows this record.

NEXT: automatic Rust semantic symbol construction from actual declarations/dependency metadata,
with real accessibility/abstract/closed generic/cast facts; complete target cfg/macro/import
integration and incremental host; inherited/source field/value/constructor/converter binding;
full default/application catalogs and actual sample save/load. Do not use C# snapshots as the
production compiler input or claim the current explicit fixtures are a default catalog.
Keep runtime/model/UI boundaries intact. Full CLR strings/metadata/error/stream/HandleNull/
JsonDocument/lifetime/performance and other engine/backends/serializers/platform/accessibility
scope stays open. Owning Rc cycles still lack source cycle collection; do not revert owning
reachability or hide cycles in caches. Final lightweight checks will append evidence below.

Final lightweight checks pass: workspace/include format, source inventory/factory generator,
expected full-completion rejection, new-code naming/unsafe/stub scan, original source diff
empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256 remain unchanged
in owned SSD ferrodock-json-registration-desktop-preserved.json; final-checks.json records all
lightweight results. All owned jobs terminal; full goal stays active and incomplete.

## Previous validated checkpoint

# Current: shared declaration metadata binds executable JSON accessors

Goal turn PROGRESS. Full Dock-to-FerroDock goal ACTIVE, incomplete and unblocked.
Previous status turn verified that emission-final53537 had reached terminal0/all20 checks;
that evidence enabled the next available safe action. No whole-goal completion is claimed.

New build-time crate src/Dock/FerroDock.Serializer.SystemTextJson.Generators.Rust adapts shared
scanner TypeModel declarations to RustTypeBinding. It links only the semantic generator and
published/preserved shared scanner, with no runtime/model/UI dependencies. It consumes declared
property order/types, typed constructor/accessor call forms, renames, unconditional JsonIgnore,
separate property attributes, get-only/set-only and resolved explicit/type converter factories.
Invokers use the declaration macro's public typed functions, never object-valued runtime lookup.
Crate-private paths are used only in the declaring crate. No invented constructor or inheritance.
Unresolved/cfg/fallible/incomplete base/registered-property/unsupported JSON bindings fail.

Seven original application payload classes now obtain bindings through an actual shared scan
of build-only declaration projections, then compile and execute against unchanged C# wire
corpora. Fixture classes expose their original typed property accessors and constructor paths;
converter marker classes retain their existing typed runtime factories. No UI runtime added
to the consumer. Two closed generic payloads and actual docking-model fields remain explicitly
bound. This is a DECLARED-MEMBER adapter, not complete syntax/semantic discovery, class facts,
activation macros or default/application catalogs. Identity, System.Object target types,
converter factories and already-resolved inherited properties remain compiler inputs.

First targeted41370 terminal0: compiled consumer4,semantic compiler4,adapter7. Initial
changed-package strict Clippy54510 terminal0. Then separate-property/unsupported-attribute
case adds adapter test8. Final7639/Python27696 terminal0: **1828 Rust tests/55suites**,zero
failed/ignored. All21 values zero in owned SSD
ferrodock-shared-bindings-final-validation-summary.json: strict workspace/debug/all-target and
selected release-library Clippy,8 standalone packages,10 normal dependency boundaries,
fmt/21Python/source/theme/inspector/two generators/six provenance guards and expected complete
port rejection. Full green covers final production/test source. No later production/test edits.
Existing source compiler19cases/152name checks,generated payload20assertions/25reads/8goldens,
policy15write/16read/5graph checks and model/number/factory corpora are unchanged. Native/Rust
workspace scenarios still run actual original engine/model handles. Inventory232/456/1240 of1928.

Original parent/sibling sources untouched. Only FerroDock and owned SSD temp edited. No agents,
commit/push/PR or staged contents; new files intent-to-add only after final checks. Actual native
executable/app/window protected; no rebuild/restart. Its size/mtime/SHA256 match prior baseline
in ferrodock-shared-bindings-desktop-preserved.json. All temp/TMPDIR on /Volumes/SSD/temp/ferrodock-port.
Cargo jobs sequential,-j1. No cleanup or other project process/artifact changes this turn.
Python3 here is3.9, so use hashlib.sha256 with streamed reads rather than file_digest/tomllib.

NEXT: complete shared Rust semantic adaptation with real accessibility/abstract/generic/cast
facts, attribute/activation discovery and target cfg selection; bind complete original model
hierarchies/fields and source supported values/constructors; compose full default catalogs and
wire ordinary sample save/load. Do not replace missing semantics with guessed types, runtime
reflection, copied model graphs or caches. JsonDocument, remaining scalar/enum domains, full
metadata/constructor/HandleNull/callback/stream/error/performance parity remain required.
Owning original Rc links still lack CLR cycle collection. All original engine/API/platform/
backend/serializer/accessibility scope remains intact. Final lightweight checks follow below.

Final lightweight checks pass: workspace/include formatting, source inventory and factory
generator, expected full-completion rejection, new-code naming/unsafe/stub scan, original
source diff empty and empty staging after intent-to-add. Actual desktop size/mtime/SHA256
remain unchanged. Evidence: owned SSD ferrodock-shared-bindings-final-checks.json.
All owned jobs terminal; full goal remains active and incomplete.

## Previous validated checkpoint

# Checkpoint: executable Rust emission and policy-aware metadata validated

Goal turn PROGRESS. Entire Dock-to-FerroDock goal ACTIVE/incomplete/unblocked.
Executable Rust emitter now consumes semantic plan + resolved original type/property/family
bindings. It generates typed class/custom codecs, getter/setter-only/custom/null/object payload
properties, reference casts, compiled ancestry/policy tables, context resolver and original
factory overloads. No runtime field reflection, copied layout graph or replacement engine.
Syntax/shared-metadata binding discovery is STILL absent; consumer schemas are EXPLICIT LIMITED
PROJECTIONS, not a full/default catalog. Native application save/load remains unwired.

New tests/Dock/FerroDock.Serializer.Generated.Tests build.rs emits TWO actual Rust contexts and
compiles/executes them. Real ReactiveUI RootDock/DocumentDock/Document/DockWindow roundtrip exact
source model golden and InitLayout uses the original Factory and decoded instances. Payload
fixtures verify closed generics, independent object scopes, explicit/read-only/set-only
converters, duplicate tags and non-object custom codec failures. Bindings call original model
accessors; payload fixtures port the original simple application classes. Additional test-only
compiler snapshot parser is shared at tests/Dock/support/json_compiler_input.rs.

Runtime now exposes policy-aware exact dispatch, source base fallback, unique nearest compiled
ancestor and ambiguity rejection; class-view selections retain original handles. Abstract/interface
schemas have no invented constructor but can resolve existing preserved refs. Empty polymorphic
configuration fails on use, including direct null reads. Source JsonPolymorphismReference records
15write/16read cases,5 graph checks and1holder golden. Runtime implementation still requires proper
compiled runtime/cast/ancestry facts; no full automatic unknown-class or CLR error/timing claim.
Set-only plain/custom properties are now supported. Source generated payload corpus expands to
20assertions/25reads/8goldens. Compiler source fixtures expand to19 (actual model + payload cases),
8name shapes per case=152comparisons. Source runs23466/19261/98298 pass; compiler66829/79940 pass.

First targeted41336 exposes triple-colon lexical error in a fixture upcast expression; whitespace
fix precedes2897 terminal0: consumer4 +runtime21 +compiler3 pass. Compiler emission inactive/
missing binding test added afterward. Clippy81870 catches fixture clone-on-Copy; corrected getter.
22847 strict all-target Clippy for all3packages passes. Compiler now4tests. No Cargo package
versions/features deliberately changed; local consumer entry/build-dependency edges added.

Full82278 TERMINAL1 on actual Skia archive ENOSPC, before tests. No owned Cargo running after
failure. Retired16 exact logged owned test harnesses=4593194272bytes, manifest
ferrodock-json-emission-harness-retired.json; actual standalone desktop excluded. Other projects
and dependency sources untouched. Shared SSD space varies independently. Retry exec53537/Python17653 TERMINAL0. Workspace completed **1820 tests/53suites**,
zero failed/ignored. All20stage driver checks pass in owned SSD
ferrodock-json-emission-final-validation-summary.json, including strict workspace/all-target
and selected release Clippy,7 standalone targets,9 normal boundaries,format/21Python,
source/theme/inspector/two generators/six provenance guards and expected completion rejection.
The subsequent shared-declaration adapter checkpoint below supersedes these counts.

Only FerroDock + owned SSD temp edited; parent originals/sibling FerroUI read-only. No agents,
commit,push,PR or staged contents. Actual desktop executable/app/window protected; no rebuild/
restart. All TMPDIR/temp/NuGet caches on SSD. One owned Cargo job at a time,-j1; other project
Cargo jobs untouched. Final fmt/includes/source/inventory/generator/staging and desktop hash checks
still required. Inventory remains232/456/1240 of1928; no unit/subsystem/goal declared complete.

NEXT after green validation: actual Rust syntax/shared metadata adapter with faithful attributes,
accessibility/abstract/generic/cast facts and full field/value/constructor binding; incremental
host/activation macros; complete default/application catalogs and real sample persistence.
Keep JSON normal boundary backend/UI independent. Full scalar/enum numeric domains, all model
families/platform/backends/serializers, runtime JsonDocument and constructor/HandleNull/metadata/
callback/depth/error semantics, exact stream behavior and profiling remain required. Owning model
Rc cycles still lack CLR cycle collection; do not revert source reachability or add retention caches.

## Previous fully validated checkpoint

# Current: generator semantic selection and name construction validated

Goal turn PROGRESS. Entire original Dock-to-FerroDock goal stays ACTIVE/incomplete/unblocked.
New library src/Dock/FerroDock.Serializer.SystemTextJson.Generators ports the pure semantic
GenerationModelBuilder stage over immutable compiler type/property/assembly facts. It has
NO production dependencies and does not create/replace docking models or runtime registries.
Source registration guards run before activation, supported-payload filtering after required
symbol resolution. Local discovery, abstract/open/nested/friend accessibility, deduplication,
context type closure/unique names, inherited/hiding/override ignore selection and interface
unions,7 sorted polymorphism plans and DSTJ001-004 messages/spans match source. Full-name
construction retains nested generic segments/arguments, arrays/tuple underlying types,
assembly versions/cultures/tokens; ordering uses UTF16 ordinal comparison. Malformed consumed
symbol links/cycles return CompilationError, separate from source registration diagnostics.

This is a SEMANTIC COMPILER PASS, not a completed Rust syntax frontend or executable generator.
Compilation facts are explicit because the shared scanner lacks some abstract/access/generic
facts. Target expressions come from the adapter. Rust syntax/shared-metadata adaptation,
incremental host/cancellation, activation/registration syntax/macros and Rust codec/context/
resolver emission remain absent. Actual default model catalogs/sample persistence still need
that path. Do not claim or simulate a generated catalog through manual snapshots/fake models.

scripts/dock/json-generator-reference builds/runs the unchanged original incremental generator on
17 C# compilations and exports PUBLIC Roslyn facts separately from expected generated plans.
Rust compares context types/names, all7 option tables, ignored member tables and diagnostics/
locations. Cases include inactive/invalid registration ordering, abstract/open/private/nested/
friend access, local-only discovery, value/ordinary/contract payload domains, override vs new
hiding, interface ignore unions, closed generics, context name collisions and duplicate tags.
136 runtime full-name comparisons are8 shapes across17 snapshots (not136 distinct shapes),
including CoreLib and unsigned project-assembly arguments, nested generics/arrays/tuples.
Source56791 passes11 initial cases; expanded77946 passes17; final89067 adds unsigned nested
argument/name probes. Source files/algorithms are unchanged, no private reflection hook or
manually copied C# generator implementation used. The source-names log in owned temp records the final17-case probe run.

Initial Rust1345 stops at missing enum-variant docs/porting/dock/unused import; fixed.82147 passes3 initial
tests (11cases/66names);62121 passes expanded17/102.62616 strict compiler Clippy passes.
14807 passes final malformed assembly-link checks; final corpus increases names to136.
First full28702 passes1812/51 and19stages before those added name records. Final40695 terminal0:
**1812 Rust tests/51 suites**, zero failed/ignored. Generator3, JSON18, ReactiveUI568,Domain225,
UI373,markup189,diagnostics49,settings33,sample27,CoreSDK62. All19 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-json-generator-final-validation-summary.json:
strict debug/all-target and selected release-library Clippy (compiler included), standalone
3+18+62+2+7+3,8 normal dependency boundaries, fmt/21Python, source/theme/inspector/two generators/
six provenance guards and expected full-completion rejection. No code/test change after final
full green. Compiler is a workspace member; lock adds only its local entry/dev serde_json edge.
No dependency package version/feature or retained-source changes. Prior4428 controls tests apply.
Prior source factory900; serializer1362numeric/33model/17payload+25read+7goldens/7fallback/
37stream/13JSON/12failure cases remain unchanged and covered. Inventory232/456/1240 of1928:
existing generator unit stays partial, now including semantic passes; no unit promoted.

Only FerroDock and owned SSD temp edited; parent original/sibling FerroUI read-only. No agents,
commits,pushes,PR or staged contents. Protect actual desktop executable/app/window; no rebuild/
restart. All owned temp/TMPDIR/NuGet caches on SSD. One owned Cargo job at a time,-j1. All owned
jobs terminal; no other project's processes/artifacts touched and no cleanup needed. Final
lightweight source/staging/inventory/format/generator and desktop hash checks follow this record.

NEXT: Rust syntax/shared metadata adapter with faithful compiler facts and source ignore/
activation/registration attributes, incremental host and executable codec/context/resolver
emission; full generated/default model catalogs and ordinary sample save/load. Preserve
backend/UI-independent normal runtime serializer. Need automatic subtype/nearest-ancestor/
ambiguity behavior, all model families/fields/custom payloads/converters, remaining CLR scalars
and full enum underlying numeric domains. Runtime payload JsonDocument ownership, complete
constructor/HandleNull/metadata/callback/depth/error behavior, list creator order and exact
stream buffering/encoding/failure timing still open. CLR lifetime/string/exception/performance
parity remains unproven; owning model Rc cycles can leak. Do not revert source reachability or
add serializer retention caches. Other original engine/API/model backend/serializer/desktop/
input/platform/accessibility scope stays intact. Read docs/porting/dock/SERIALIZATION.md and sibling
FerroUI/docs/porting/PORTING-GUIDE.md. No whole-goal completion is claimed.

Final lightweight checks pass: format/includes, inventory, factory generator, expected incomplete
gate, source naming/unsafe/stub rules, compiler normal tree containing only itself, original
source diff empty and no staged contents after intent-to-add. Actual desktop size/mtime/SHA256
match the preserved baseline; ferrodock-json-generator-desktop-preserved.json and -final-checks.json
in owned SSD temp record that evidence. All owned jobs terminal; the full goal remains active.

## Previous fully validated checkpoint

# Current: emitted object-payload helper runtime and property converters validated

Goal turn PROGRESS. Entire original Dock-to-FerroDock goal remains ACTIVE/incomplete/unblocked.
Read the original1662-line DockJsonSourceGenerator, source-generation tests and porting guide.
The emitted ObjectPayloadConverter runtime is now partially ported in object_payload_converter.rs,
with compiled TypeId/name tables over original BoxedValue values/shared class handles. It is
property-local, not a global object codec. JsonPropertyInfo.with_converter/readonly_with_converter
and JsonTypeInfo.optional_with keep actual accessors, explicit converter precedence and nullable
behavior without mutating resolver-wide metadata. Reader/writer typed entry helpers share the
existing codec dispatch; isolated payload calls use fresh reference/depth scopes and the same
options metadata cache, matching emitted SerializeToElement/Deserialize calls.

Source helper behavior matters: separate object properties deserialize as distinct instances,
each retaining its own self-reference; an ordinary typed property uses the outer scope. Tag is
before inner$id. Unknown/untagged/multiple string tags retain raw JSON, preserving duplicates
and lexical numbers. A single known late tag is stripped before typed decode. Boxed raw null
is written but actual nullable null is omitted. Unregistered runtime types and registered
converters returning arrays fail. Get-only custom properties write and skip read input; explicit
property converters win. JsonDocument ownership/disposal branch remains absent, not substituted.
Root object dispatch and complete constructor/HandleNull/depth/error/performance semantics remain
unverified. Unsupported error text does not yet carry the source runtime full name.

GeneratedPayloadReference.cs builds/runs the UNCHANGED ORIGINAL incremental generator as an
analyzer, with both original activation/registration attributes compiled into the reference.
Seventeen source assertions,25 read/replay records and7 wire goldens cover source emitted
resolver behavior, including closed generics, read-only/explicit converters, raw null and
dollar metadata. Rust uses supplied compiled tables; this does NOT port the compiler frontend,
activation/registration macros, semantic discovery/diagnostics/emission or full default catalogs.
The generator source unit is newly PARTIAL only for its emitted helper runtime, now688 total
records:232ported/456partial/1240pending of1928. Do not promote it to a complete generator.

First targeted2954 passes2/fails1: ordinary class reader accepted residual non-string$type that
source rejects after stripping the known tag. Added source probes for$foo/$values/$Type/$$,
repeated/ref metadata, mapped$type schema and raw fallback: preserved typed classes must reject
unexpected dollar metadata while raw helper values keep it. Production class reader now does so;
89981 and final85153 pass all18 serializer tests. Source99135,92348,68574,67768 and final84796
are terminal0. The final source proof regenerates17helper+25read,33model,1362numeric,7fallback,
37stream,13JSON,12failure corpora and goldens. Original factory900 assertions unchanged.

Full95023 terminal0: **1809 Rust tests/49 suites**, zero failed/ignored (JSON18,ReactiveUI568,
Domain225,UI373,markup189,diagnostics49,settings33,sample27,CoreSDK62). All18 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-json-payload-validation-summary.json: strict debug/
all-target and selected release-library Clippy, standalone18+62+2+7+3, seven normal dependency
boundaries, fmt/21Python, source/theme/inspector/two generators/six provenance guards and expected
full-completion rejection. No production/test change follows full green validation. No Cargo
package/version/feature or retained source change; prior4428 controls tests remain applicable.

Only FerroDock and owned SSD temp edited; parent originals/sibling FerroUI read-only. No agents,
commits,pushes,PR or staged contents. Actual desktop executable/app/window protected; no rebuild/
restart. All owned TMPDIR/temp/NuGet caches on SSD; no /tmp. One owned Cargo job at a time,-j1;
all owned jobs terminal. Other observed Cargo processes belong to FerroBoard/FerroRibbon and
were left alone. Shared SSD free space varies; no cleanup required this turn. Final lightweight
source/staging/format/generator/inventory/desktop hash checks follow this checkpoint.

NEXT: incremental compiler frontend and original model/type discovery, registration/activation,
ignored interface/command members, collision diagnostics, generic full names and codec/context/
resolver emission; default/generated full model catalogs and ordinary sample persistence.
Normal JSON library must stay backend/UI independent; actual-model references remain dev-only.
Need all original model families/fields/custom converters/closed generic payloads, automatic
subtype/nearest-ancestor ambiguity rules, remaining scalar and full enum numeric domains. Exact
metadata/constructor/list callback order, stream buffering/encoding/error timing, CLR strings/
exceptions/lifetime and performance remain open. Source owning links still require cycle
collection; Rc cycles can leak. Do not revert source reachability or add serializer retention
caches. All other original engine/API/backend/serializer/platform/input/accessibility scope stays
intact. Read docs/porting/dock/SERIALIZATION.md and sibling FerroUI/docs/porting/PORTING-GUIDE.md before work.

Final lightweight checks pass: format/include formatting, source inventory, factory generator,
expected incomplete gate, source naming/unsafe/stub rules, unchanged original source files and
empty staging after intent-to-add. Actual desktop size/mtime/SHA256 match the preserved
baseline; ferrodock-json-payload-desktop-preserved.json and -final-checks.json in owned SSD
temp record these checks. All owned build/validation jobs terminal; goal remains active.

## Previous fully validated checkpoint

# Current: numeric codecs and model proportions/window bounds validated

Goal turn PROGRESS. Full original Dock-to-FerroDock goal ACTIVE/incomplete/unblocked.
Builtin JSON metadata now covers signed/unsigned8/16/32/64-bit integers, Single and Double,
alongside String/Boolean. Numeric descriptors call the same generic serializer; no model
snapshot/cache or alternate docking engine was introduced. Existing locked Ryū1.0.23 supplies
shortest digits; explicit JSON dependency adds no version/feature. Source general notation
uses fixed exponents -4 through16(Double)/8(Single), signed/padded scientific exponents,
-0 and named NaN/infinity strings. Quoted finite numbers fail; numeric overflow/underflow
and integer range/token failures match source records. NaN reads retain source canonical
values, not arbitrary input payload bits (which original JSON also discards).

JsonNumberReference.cs compiles unchanged source serializer/options/resolver files and writes
1362 numeric wire/read rows:535writes+24reads per float type, plus244 integer records.512
seeded bit patterns per float type supplement finite/nonfinite/threshold boundaries. Rust
compares every row and checks exact finite roundtrip bits. Numeric targeted69332 passes2;
final87466 passes all14 serializer tests. JsonModelReference now records33 cases; expanded
LIMITED projection adds proportions, collapsed proportions, min/max constraints and X/Y/
Width/Height. Source model fixtures retain NaN, infinity, finite values and signed zero.
Window source setter keeps initial+0 when assigned-0; MinHeight startsNaN and retains-0.
First source59773 failed a mistaken negative-X-zero expectation, leaving incomplete corpora;
21405 then failed4 tests using those incomplete files. Corrected probe expectation to measured
source setter behavior; final16729 terminal0 regenerates every corpus/golden;87466 passes.
No production behavior or assertion was weakened to hide an implementation failure.

Latest locked workspace passes **1805 tests/49 suites**, zero failed/ignored; JSON14,
ReactiveUI568,Domain225,UI373,markup189,diagnostics49,settings33,sample27,CoreSDK62.
Full96955 terminal0: all18 values zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-json-numeric-validation-summary.json.
Strict debug/all-target and selected release-library Clippy, standalone14+62+2+7+3,
seven normal dependency boundaries, fmt/21Python, source/theme/inspector/two generators/
six package provenance guards and expected full-completion rejection pass. No production/
test change follows green validation. Prior4428 controls tests remain applicable: no
retained package source/version/feature changed. Source factory900 assertions unchanged;
serializer37stream+13JSON+12failure+33model+7fallback cases and string/payload goldens persist.
Inventory remains232ported/455partial/1241pending of1928. No unit/subsystem declared complete.

Only FerroDock and owned SSD temp edited. Parent original/sibling FerroUI read-only. No
agents,commits,pushes,PR or staged contents. Protect actual desktop executable/app/window;
no rebuild/restart. All owned TMPDIR/temp/NuGet HTTP cache on SSD. One owned Cargo job at
a time,-j1; all owned jobs now terminal. Before work,SSD free32MiB; retired8 logged,
noncurrent owned test harnesses=472238720bytes with numeric-harness-retired manifest,
excluding actual desktop/current workspace harnesses. Other projects left untouched.
Shared SSD space varies independently; do not infer blockers or blindly clean other work.
Final lightweight source/staging/inventory/format/generator and desktop preservation
checks follow this documentation checkpoint.

NEXT: original generated/default full model catalog and registration attributes/generator,
all model families/fields/custom/closed generic payload converters, automatic subtype and
nearest-ancestor/ambiguity resolution. Remaining CLR scalars and full enum underlying
numeric domains (including unnamed values) require faithful handling. Current Rust closed
enums must not silently narrow the original domain. Preserve the backend-independent normal
serializer boundary; source actual-model references remain dev-only until a proper compiled
catalog composition is implemented. Original ignored interface/command members, malformed
metadata/constructor order, exact stream buffering/encoding/failure timing and lifetime/
performance remain unverified beyond the recorded cases. Model owning links still require
CLR-compatible cycle collection; Rc cycles can leak. Do not revert required source reachability
or hide it behind serializer retention caches. Remaining engine/API/backends/serializers/
platform/input/accessibility scope stays intact. Read docs/porting/dock/SERIALIZATION.md and sibling
FerroUI/docs/porting/PORTING-GUIDE.md before the next implementation.

Final lightweight checks pass: format (including include files), inventory, factory generator,
expected incomplete gate, source naming/unsafe/stub rules, original source diff empty and no
staged contents after intent-to-add. Actual desktop size/mtime/SHA256 match the preserved
baseline; numeric-desktop-preserved.json and numeric-final-checks.json under owned SSD temp
record the evidence. All owned build/validation jobs are terminal; the goal remains active.

## Previous fully validated checkpoint

# Current: model/list reference projection and polymorphic base fallback

Goal turn PROGRESS. Full original Dock-to-FerroDock goal remains ACTIVE/incomplete/unblocked.
Compiled casts now retain actual model references across concrete/interface declarations.
Registered polymorphism keeps source $id/$type/$ref order; configured observable/plain/custom
IList creators keep original collection IDs, aliases and nullable entries. The original
list-type/resolver constructor is exposed. No reflection, copied layout tree, model cache
or replacement engine is used. Source catalogs/generator/default constructors are still absent.

JsonModelReference.cs compiles unchanged source serializer/options and uses actual source
ReactiveUI models/factory. Its 28 cases and two goldens cover RootDock/DocumentDock/Document/
DockWindow, owner/root/window cycles, active/default/list aliases, custom creators and
original InitLayout after load. Rust matches an EXPLICIT LIMITED PROPERTY PROJECTION,
not the full default catalog/sample layout. Owner, RootDock.Window and ParentWindow retain
the original objects as C# does. Previous weak links lost source reachability after the
read context dropped; source probes establish the stronger behavior. Existing weak-snapshot
and window ownership tests explicitly clear model cycles to isolate their own contracts.
CLR-compatible cycle collection is NOT implemented: owning Rc graphs can leak. Do not
revert required source reachability or hide it behind a serializer-side retention cache.

JsonFallbackReference.cs adds five depth/getter records and two fallback cases plus a wire
golden. Rust initially read getter65 before DOM rejection; shared encode_info now applies
the original object-depth guard before entering that fallback object. Fallback self-reference
and ignored unknown discriminators match explicit compiled base metadata. Automatic subtype
registration, nearest-ancestor/ambiguity selection and full metadata validation remain absent.

Targeted52521 terminal0: all12 JSON tests pass. ReactiveUI91811 passes568 tests. Source44813
terminal0 regenerates all37 stream,13 JSON,12 failure cases,28 model cases and7 fallback
rows; original factory900 assertions unchanged. First full poly workspace passes1801 but
strict Clippy rejects an unused test import, now removed. Fallback pre-fix36054 fails the
new getter-count assertion;52521 passes after the production correction. Latest full locked
workspace passes **1,803 tests /49 suites**, zero failed/ignored; strict debug/all-target
and selected release-library Clippy pass. All18 final driver stages pass, including standalone12+62+2+7+3, seven normal dependency
boundaries, fmt/21Python, source/theme/inspector/two generators/six provenance guards and
expected full-completion rejection. Summary:
/Volumes/SSD/temp/ferrodock-port/ferrodock-json-fallback-validation-summary.json.
No production/test change follows the green run. All owned Cargo/driver jobs terminal.

Inventory remains232 ported/455 partial/1241 pending of1928; full completion must reject.
The JSON normal dependency boundary remains independent of ReactiveUI/UI: the new actual-model
dependency is dev-only. No dependency versions/features or retained package sources changed;
prior4428 retained controls tests remain applicable. All temp/TMPDIR/NuGet HTTP cache on owned
SSD. One owned Cargo job at a time,-j1. Actual desktop executable/app/window protected;
do not rebuild/restart it. Parent original and sibling FerroUI remain read-only. No agents,
commits,pushes,PR or staged content. Final intent-to-add and desktop hash verification are recorded below after lightweight checks.

NEXT: default/generated full model metadata catalogs, source attributes/generator and complete
primitive/enum codecs (current builtins String/Int32/Boolean), original ignored command/interface
members, all model families/custom/closed generic payload converters and full sample layouts.
Automatic subtype/ancestor resolution, malformed metadata/list constructor callback order,
exact stream buffering/encoding/error timing and CLR cycle/string/exception/performance behavior
remain required. Preserve all original engine/API/backends/serializers/desktop/input/platform/
accessibility scope. Source guide: docs/porting/dock/SERIALIZATION.md and sibling FerroUI/docs/porting/
PORTING-GUIDE.md. No subsystem or whole goal is declared complete.

Final lightweight checks confirm explicit include-file rustfmt, original source diff empty,
intent-to-add with no staged contents, and unchanged actual desktop size/mtime/SHA256.
Preservation manifest: /Volumes/SSD/temp/ferrodock-port/ferrodock-json-fallback-desktop-preserved.json.
No owned Cargo/validation process remains running. Source and generator checks are repeated
following this documentation update; the whole goal remains active and incomplete.

## Previous fully validated checkpoint

# Current: resolver-based JSON wrapper and controls regression validated

Goal turn PROGRESS. Full original Dock-to-FerroDock goal ACTIVE/incomplete/unblocked.
New crate src/Dock/FerroDock.Serializer.SystemTextJson implements actual generic wrapper with
supplied typed metadata, per-options caches/per-call IDs, class cycles, readonly/null
properties, ordered duplicate-preserving DOM and lexical numbers, source escaping and
UTF/BOM stream closure/error precedence. Metadata is supplied compiled tables: original
generator, default catalogs/list constructors/factories/polymorphic interfaces are NOT
implemented. No reflection, layout snapshot, copied models or replacement docking engine.
Core IDockSerializer returns SerializationResult and accepts nullable text; workspace
Capture/Restore/TryRestore propagate original shared error before commits. Actual source
still dictates null versus exception behavior. Existing tests preserve their original
objects/expectations with explicit Result unwraps. New result-failure model test retains
cause identity and payload/active/dirty state. Native generic dispatch remains partial.

Full50330 terminal0 passes **1,797 tests/49 suites**, zero failed/ignored; JSON7,ReactiveUI567,
Domain225, UI373,markup189,diagnostics49,settings33,sample27,CoreSDK62. All18values zero in
ferrodock-json-validation-summary.json. Strict debug/all-target and release-library Clippy
(including new crate), standalone7+62+2+7+3, seven normal boundaries, fmt/21Python, source/
theme/inspector/two generators/six package guards pass. Expected complete gate rejects
232/455/1241 of1928 (new serializerpartial). No production/test change after full green run.
Source factory81977 passes900 assertions,7new; serializer40486 passes37stream+13JSON+12failure
cases. Six string wire rows and payload golden derive from unchanged original serializer.
13 former SOURCE-ONLY JSON cases now MATCHED via typed descriptors. FinalJSON targeted58283
passes7. All model/stream/workspace targeted tests pass. Earlier15976,55817 and62126 expose
caller unwrap/type-expression/style issues corrected before final full validation. Cargo.lock
adds the new crate/dev dependency with existing versions; serde_json raw_value changes the
workspace feature graph, so old controls results alone are insufficient.

Additional92422 terminal0 passes all4,428 retained controls LIB tests, zero failed/ignored.
The guarded owned temp copy enables serde_json/raw_value and the same test profile;
source hashes/provenance match and retained packages are unchanged. Log
ferrodock-json-controls-all.log, driver ferrodock-json-controls-driver.log. All owned jobs
terminal. Final51424 terminal0 confirms fmt, inventory/generator, expected incomplete gate,
intent-to-add, empty staging/original source diff and actual desktop executable hash.
One owned Cargo job at a time,-j1 remains required.

Disk: full28322 and49930 terminal1 on actual linker ENOSPC. After no owned Cargo live,
retired logged obsolete harnesses with ferrodock-json- manifests, excluding actual desktop.
Test profile now debug="line-tables-only",incremental=false; dev unchanged.12 logged LIB
harnesses (4,009,754,096bytes) retired in debug-harness manifest. Main UI test harness now
365.3MiB vs405MiB. Full50330 rebuilds all native deps and passes. No source/rlib/other-project
blind cleanup. Last free9.4GiB, shared/volatile. Desktop baseline hash manifest remains
ferrodock-query-dispose-desktop-preservation.json. Protect standalone app/executable/window;
no rebuild/restart. All temp/TMPDIR/NuGet HTTP cache owned SSD. No agents, commit,push,PR or
staged contents; final git add -N --FerroDock still needed for new files. Parent original and
sibling FerroUI read-only; only FerroDock + owned SSD temp edited. New crate NOTICE copied.

NEXT major implementation: original default/generated metadata catalogs, reference casts
across declared interface handles and polymorphism, configured IList creators/identity,
then real original model layout roundtrips through the existing engine. Current classes
support same-declared-type references only. Builtin scalar descriptors currently String,
Int32,Boolean; float/enums/full codecs missing. Preserve original ignored members/commands,
subtype fallback, registration attributes/generator, custom/closed generic payloads and
property converters. DOM preserves duplicate $type names, but type resolution still pending.
Do not substitute snapshot data or silently instantiate fallback models. Source paths and
requirements: docs/porting/dock/SERIALIZATION.md and /Volumes/SSD/repos/FerroUI/docs/porting/PORTING-GUIDE.md.
Also outstanding: exact text reader/writer buffering/callback order, malformed UTF/CLR
exception/lifetime semantics, metadata validation/allocation/performance; other serializers/
model backends; remaining engine/API nullable contracts, public attribute class, generic/
delegate/constructor SDK metadata and full desktop/input/platform/accessibility validation.
Whole goal unchanged; no subsystem declared complete.

## Previous fully validated checkpoint

# Current: serializer stream prerequisite validated; actual serializers still pending

Goal turn PROGRESS. The full original Dock-to-FerroDock goal remains ACTIVE, incomplete
and unblocked. Only FerroDock and owned SSD temp edited; parent original source and sibling
FerroUI read-only. No agents/commit/push/PR or staged contents. New files intent-to-add.
Preserve actual desktop executable/app bundle/running window; no rebuild/restart. All
owned temp and child TMPDIR=/Volumes/SSD/temp/ferrodock-port; keep NuGet/package/HTTP
caches on SSD too. One owned Cargo job at a time,-j1. Do not interrupt other chats/builds
or restart live jobs on observation timeout. All owned jobs now terminal.

Source audit changed the next action: original JSON Save/Load close their supplied Stream,
including failure, while the old Rust borrowed std::io-only trait could not express that.
core/i_dock_serializer.rs now uses Option<Rc<dyn DockStream>> for both generic methods;
DockStream aliases the shared io::Stream contract, retaining IDisposable, capabilities,
position/length, read/write/seek/flush/close. MemoryStream uses original fixed byte-array
handles or expandable storage, source zero-fill/truncate/overflow/failure order, shared
closed aliases and post-close ToArray. StreamIo adapts the same Rc to native I/O and does
not close on drop. No std::fs, threads, unsafe, reflection or dummy models introduced.
Existing workspace mock now uses this actual stream instead of borrowed std::io::Cursor.
Generic T and original object/payload/stream identities remain intact. This is a partial
runtime bridge, not a claim of the entire System.IO.MemoryStream/Stream API or metadata.

Five io::tests match37 source stream/interface assertions and cover custom close overrides,
close during read and native adapter identity. Targeted29516 passes5 model +15 workspace
tests. Source2179 passes37 matched assertions and13 SOURCE-ONLY JSON audits. Source67875
first exposes closed SetLength throwing NotSupportedException before ObjectDisposed;
corrected probe/Rust requirement, no behavior weakened. Source55803 passes33 before four
generic nullable-stream cases appended. serializer-reference compiles unchanged original
DockSerializer.cs, OptionsFactory, ListTypeInfoModifier and PolymorphicTypeResolver plus
original Dock.Model. It deliberately does not introduce a Rust serializer or pretend the
JSON audits have Rust coverage. The old893 factory corpus remains unchanged.

First full62682 terminal1: linker ENOSPC on Fluent test harness. Initial retirement of6
stale owned test harnesses=2,452,749,928bytes; then3 obsolete historical variants=
510,264,880bytes, with manifests/no owned Cargo live. Other observed Cargo cwdFerroBoard,
untouched. Retry81373 terminal0: **1,789 tests/47 suites**, zero failed/ignored (Domain225,
ReactiveUI566, UI373, markup189, diagnostics49, settings33, sample27, CoreSDK62). All17
values zero in ferrodock-serializer-stream-validation-summary.json. Strict debug/all-target
and selected release-library Clippy; standalone62+2+7+3; six normal boundaries; fmt/21Python;
source/theme/inspector; two generators/six vendor guards pass. No source/test changes after
full green validation. Prior4,428 controls tests apply to unchanged dependencies. Inventory
unchanged232/454/1242 of1928; complete-port gate correctly rejects full completion.

After all Cargo/rustc processes ended, removed5 inactive owned incremental directories,
3,974,305,547logical bytes; newest2026-10-22 22:30:40UTC, dev/test profiles incremental=false,
no overriding env. Manifest ferrodock-serializer-stream-incremental-retired.json. Other
retirement manifests share ferrodock-serializer-stream- prefix. Actual desktop sample
protected. Last SSD free2.9GiB (shared/volatile); never blindly remove another project's
artifacts. Original standalone desktop hash baseline remains available in
ferrodock-query-dispose-desktop-preservation.json; verify before ending/checkpointing.

NEXT: implement the actual SystemTextJson serializer and its metadata/resolver pipeline,
starting from docs/porting/dock/SERIALIZATION.md and the original source files/tests it names. Preserve
original declared generic T, reference/list identities and owner cycles, active references,
polymorphic dock/window/template families, configurable list types, ignored attributes/
commands, generated metadata, payload converters and duplicate $type members. Follow the
FerroUI guide's parse-error convention and preserve source null versus exception behavior;
audit the current trait/workspace error handling rather than suppressing failures to None.
Use generated typed metadata, not runtime reflection or a layout snapshot serializer.

Thirteen source-only JSON audit outcomes show readonly payload properties ARE included;
null properties omitted; self $id/$ref restored; Save emits UTF-8 BOM and closes; Load
closes even after empty/malformed input; empty Load throws JsonException. Older original
unit assertions for readonly omission and empty-load null contradict measured source:
port the implementation. StreamReader/Writer buffering, encoding/BOM detection and source
close/failure order remain unimplemented, as do actual serializer crates and default/
generated JSON metadata. Source framework guide:
/Volumes/SSD/repos/FerroUI/docs/porting/PORTING-GUIDE.md. No System.IO/JSON layer was found
in the inspected sibling System crate; reuse published dependencies where actually available.

Other whole-goal requirements remain: remaining service/manager/window/state inputs;
public RequiresDataTemplateAttribute class; complete collection/generic/delegate/constructor
metadata; custom query/collection cleanup and CLR/lifetime/exception/allocation semantics;
other serializer/model backends; full desktop input/drag/drop/platform/accessibility and
performance validation. Keep the entire goal intact; no source unit promoted this turn.

## Previous fully validated checkpoint

# Current: original model query cursor disposal validated

Goal turn PROGRESS: original query cursor state now owns cleanup/advancement and native
views forward to the same model engine. Full original Dock-to-FerroDock goal remains
ACTIVE, incomplete and unblocked. Only FerroDock and owned SSD temp edited; parent
original source and sibling FerroUI remain read-only. No agents, commits, push or PR.
New files intent-to-add; no staged contents. Preserve actual desktop executable/app
bundle/running window; no rebuild/restart. All owned temp and child TMPDIR must use
/Volumes/SSD/temp/ferrodock-port. One owned Cargo job at a time,-j1. Never interrupt other
chats/builds or restart live jobs merely because an observation timed out.

All owned jobs terminal. Source probes4032/91173 establish actual .NET cleanup behavior.
Final source55488 passes893 assertions,34new, no source-only rows. Targeted model87906
passes3; native78815 passes1 (32 source traces through each factory view). Full52232 passes
1,784 tests/47 suites, zero failed/ignored: UI373, markup189, diagnostics49, settings33,
sample27, ReactiveUI566, Core62. Its Clippy check rejects test-only redundant formatting
borrow. First resume95088 then finds test Boolean comparisons. Equivalent test-only
corrections applied; production unchanged since full green tests. Final75818 terminal0:
all17 values zero in ferrodock-query-dispose-validation-summary.json. Strict debug/
release-library Clippy, standalone62+2+7+3, six normal boundaries, fmt/21Python, source/
theme/inspector, both generators/six vendor guards pass. Complete-port rejection remains
expected; inventory unchanged232/454/1242 of1928. Prior4,428 controls tests cover unchanged
dependencies. Six logged owned stale test harnesses (2,452,252,344bytes) retired before
builds with no owned Cargo live; see ferrodock-query-dispose-test-cache-retirement.json.
Last observed SSD free3.4GiB (shared/volatile); no blind cleanup. Actual sample protected.

Production factory_query.rs now exposes the original shared DockableQueryEnumerator
through DockableQuery.get_enumerator; .iter is a Rust adapter over that cursor. QueryState
retains original FindInDock/FindAll state, Current, unwind-safe running guard and explicit
Dispose. Root predicate disposal before its foreach is ignored. Child disposal clears
pending child/collection fields: a true match can yield real null before later source
MoveNext-access failure. Nested/all-controls cleanup can clear the live inner cursor
before Current reread and preserve previous outer Current on failure. Recursive MoveNext
returns false. Before-start Dispose ends the cursor; Reset stays unsupported. Native
factory_search.rs forwards to that actual model cursor, preserving metadata and identity;
its former separate RefCell iterator/running state is removed. Collection storage is
taken out before custom iteration/cleanup; no RefCell borrow crosses user callbacks.
Do not replace this with ignoring every running Dispose, a pending-disposal flag, a
snapshot or a second traversal engine. Both Find overloads remain fixed/nonvirtual.

New QueryCursorReference.cs and query-cursor-reference.tsv record32 source traces;
query_cursor_tests.rs has3 model tests and query_cursor_sdk_tests.rs has1 test through
both native views. Model tests use existing TestControl; native tests use actual DockControl
with InitializeLayout=false. All tests retain original source list/model/predicate handles.
Before-start/source traces add34 assertions. Source exception names in the trace are
normalized Rust source-access failures, not CLR exception-type/object parity. C# source
reference README updated to893. Backups query-dispose-backup/ are the prior engine;
never blindly rerun generation/edit scripts over current source.

NEXT: advance remaining public API and serializer work. Actual serializers are absent
apart from core/i_dock_serializer.rs. Audit original src/Dock.Serializer.SystemTextJson/
DockSerializer.cs, DockSerializerOptionsFactory.cs, its converters/resolvers and original
tests/Dock.Serializer.UnitTests plus source-generation tests. Preserve generic declared
T, reference identity/cycles, polymorphism, original list types, generated metadata and
stream behavior; do not substitute a layout-only snapshot serializer or introduce
reflection. Reuse existing FerroUI dependencies where available after inspecting them.
The sibling guide remains /Volumes/SSD/repos/FerroUI/docs/porting/PORTING-GUIDE.md.

Also outstanding: custom collection enumerator Dispose/cleanup callback order/reentry,
CLR/lifetime/exception semantics and allocation/performance parity; remaining service/
manager/window/state nullable inputs; public RequiresDataTemplateAttribute class;
complete generic/collection/delegate/constructor metadata; alternative model backends;
full original desktop/input/drag/drop/platform/accessibility/performance validation.
No source unit or whole subsystem was promoted to complete by this checkpoint.

## Previous fully validated checkpoint

# Current: nullable search/query and native cursor advancement validated

Full original Dock-to-FerroDock goal ACTIVE, incomplete and unblocked. Only FerroDock
and owned SSD temp edited; original parent source and sibling FerroUI remain read-only.
No agents, commit, push, PR or staged contents. New files are intent-to-add. Preserve
the actual desktop executable/app bundle/running window; no rebuild/restart. All owned
temp and child TMPDIR=/Volumes/SSD/temp/ferrodock-port. One owned Cargo job at a time,
-j1. Never interrupt other chats/builds or restart a confirmed live job on observation
timeout. Last observed SSD free 2.5 GiB; no cleanup performed this checkpoint.

All owned jobs terminal. Full driver 28615 passes all 17 checks, recorded in
ferrodock-null-search-validation-summary.json. Workspace 1,780 tests / 47 suites,
zero failed/ignored: UI373, markup188, diagnostics49, settings33, sample27,
ReactiveUI563, Core62. Model40694 passes9; native3663 passes4. C#91934 passes859
assertions,34new. Strict all-target/release-library Clippy, standalone62+2+7+3,
six dependency boundaries, fmt/21Python, source/theme/inspector, both generators and
six vendor guards pass. Complete-port rejection remains expected. Inventory unchanged
232/454/1242 of1928. Dependencies unchanged; prior all4,428 controls tests apply.

Production accepts Option roots AND Option predicate callables in FindDockable, Find
and FindInDock through the original model, lazy query engine and native metadata.
DockablePredicate itself still takes the actual optional model. Missing callable is
distinct from a callable receiving null. Immediate search calls predicate first;
fixed lazy Find defers all work, can yield null once, then fails on original list access.
All-controls search skips absent layouts before invoking any callable. Both Find slots
remain fixed; FindDockable remains virtual. Window search reads live Layout separately
for guard/predicate/return/recursive dispatch. Actual None reaches virtual overrides;
null window elements still fail at first Layout access.

QueryCursor now has per-cursor moving:Cell<bool> and an unwind-safe CursorMoveScope.
Recursive MoveNext during root/child predicate returns false and retains previous
Current, matching unchanged C#. The original RefCell iterator is retained. Dispose
inside a predicate still needs source probing; do not assume ignoring it while moving
is correct. Broader cursor disposal/reentry parity remains explicitly deferred.

New model nullable_search_tests.rs (9), native nullable_search_sdk_tests.rs (4), and
NullableSearchReference.cs (34 assertions) map to original factory contracts/locator.
ProbeWindow visibility alone changed private to internal in the C# fixture for reuse.
Old ordinary callers wrap original Rc handles in Some. Canonical native lookup tests
now expect Option declaration types while continuing to send raw original references.
Initial fixture compile91316 corrected concrete factory bounds and child upcast.
Native63643 exposed real reentry failure; exact nocapture proof is in
ferrodock-null-search-reentry-failure.log. Native3663 passes after the production guard.
Initial check34442 is superseded by Some-only migration58987 passing after7 rounds.
Do not rerun ferrodock-null-search-edit.py, -calls.py or stale migration diagnostics;
first-copy backups are nullable-search-backup/. No code change after full validation.

Next coherent work: probe original cursor Dispose during callbacks and broader reentry,
then implement only source-proven behavior with native/model tests. Continue auditing
remaining service/manager/window/state nullable inputs and callable/argument metadata.
Public RequiresDataTemplateAttribute class, full collection/generic/delegate/constructor
metadata, serializers/other model backends, CLR/lifetime and original desktop input,
platform, accessibility and performance scope remain required. The runnable sample and
current regression coverage do not establish full completion; preserve the whole goal.

## Previous fully validated checkpoint

# Current: shared nullable window collection/lifecycle validated; next search/query roots

Goal turn PROGRESS; full original Dock-to-FerroDock goal ACTIVE/incomplete/unblocked.
Only FerroDock + owned SSD temp edited. Original parent source and sibling FerroUI read-only.
No agents, commit/push/PR/staged contents. New files intent-to-add. Preserve actual desktop
executable/app bundle/running window; no desktop rebuild/restart. All temp/child
TMPDIR=/Volumes/SSD/temp/ferrodock-port. One owned Cargo job at a time,-j1. Never interrupt
other chats/builds or restart confirmed live handles solely because observation timed out.

All owned jobs terminal. Model30007 passes11; native20995 passes5. C#1923 terminal0:
825 assertions,40new, zero source-only rows; first86493 passed38 before two root-result
cases added. Full82106 finds old metadata test expectations, corrected without changing
raw input identity. Full55335 passes1,767 tests/47 suites (UI373, markup184, diagnostics49,
settings33, sample27, ReactiveUI554, Core62), then Clippy asks for type-complexity aliases.
Only equivalent private aliases added in reactive root + shared test fixtures. Resume47994
terminal0/all17zero in ferrodock-null-windows-validation-summary.json. Strict all-target/
selected release-library Clippy; standalone62+2+7+3; six normal boundaries; fmt/21Python/
source/theme/inspector/two generators/six vendor guards pass. No runtime change after full
tests. Prior all4,428 controls tests cover unchanged packages. Last SSD free5.1GiB.

All helpers/logs ferrodock-null-windows- under SSD. Initial edit script and fixture adapter
must NOT be rerun (would double-wrap Some); first-copy backups nullable-windows-backup/.
Some-only migration repeatedly stopped on nonmechanical Option-list/read errors; each
was inspected/fixed with fresh checks. Many old nonnull fixture accesses now explicitly
expect their original windows, preserving all assertions. One intermediate helper Some
around a fixture if/else was removed after both branches regained their original Rc type.
No safe edit applied from stale diagnostics. Initial targeted39021 had old call-site compile
errors; targeted6536 runtime setup-log mismatch was corrected by clearing Id setup calls.
First root-result append used reference cwd and failed without edits; absolute source
append then succeeded. Source/reference825 and model11 verify final result.

Production: Windows is now one ListHandle<Option<Rc<IDockWindow>>> across IRootDock,
reactive root, generic factory creation, native Core/backend references, IList metadata,
UI conversions, source traversals and sample test source. No null filtering/dummies,
mirrored list or replacement engine. Source Add/Insert/Remove/CloseWindow, InitDockWindow
both overloads, optional host creation and protected root resolver accept nullable inputs/
results through original virtual slots. Negative Insert returns before root; list commits/
added payload precede Init; live list replacement keeps original Init owner/window. Host
factory null input precedes explicit overload; default failures remain original Id/Host.
Init's first Layout argument may be null while second Layout.Owner is actual different
root: preserve this and genuine second-owner failure. Shared TestWindow now has optional
layout_getter hook for source getter-order tests, cloned before calling to avoid borrows.
Private WindowList aliases silence complexity lint without changing representation.

New Core dock_window_host_factory_metadata.rs publishes IDockWindowHostFactory descriptor,
nullable window method input, actual host result, canonical factory cast, reference identity
and Rust paths. It stays in UI-independent SDK boundaries. New model/native/C# probes and
CoreNullWindowsBindings.xaml/.axaml mapped to original source units. Original stale compiled
IList indexer behavior kept; Count/ItemsSource live and same source list. Source null window
fails at first Layout/IsModal/Save read in search/state/navigation/control/service walkers;
no skip. DockControl nullable layout re-read now forwards actual None to source collector.
Inventory unchanged232/454/1242 of1928; no source status upgrades/dependency edits/cleanup.

NEXT: nullable dock/root arguments for FindDockable and fixed Find(IDock,predicate), Rust
find_in_dock/DockableQuery::in_dock/FindInDock. Read actual ../../../Dock/src/Dock.Model/FactoryBase.Locator.cs
104-193 and216ff afresh, plus generator fixed dispatch rules. Source first invokes predicate
on incoming dock (including null); true FindDockable(null) returns null immediately, false
fails at VisibleDockables. Fixed lazy Find(null,predicate) invokes nothing until MoveNext;
true predicate yields actual null once before later VisibleDockables failure. Preserve
repeatable/deferred query, Current/disposal/fault behavior and original owner/predicate Rc.
Current FindInDock.dock is Rc and assumes Some(self.dock) at root; change coherently with
IFactory/base/generated/native signatures, canonical declarations and raw argument callers.
Don't make source nonvirtual Find overload virtual. Predicates already take actual optional
IDockable values; don't create new delegate/query/collection engines.

Also remove premature Layout expects in FactoryBase locator's window predicate/recursive
FindDockable calls only when source passes live nullable arguments to actual virtual slots.
Original window loop: guard window.Layout !=null, predicate(window.Layout), return live
window.Layout if true, otherwise virtual FindDockable(window.Layout,predicate). New existing
layout_getter fixture can expose real read sequence/null override dispatch. First null
WINDOW element still genuinely fails at Layout and must not be skipped. Collection-root
resolver and Add null result now verified end-to-end by NullCollectionRootFactory.

Remaining service/manager/window/state inputs, public RequiresDataTemplateAttribute class,
full collection/generic/delegate/constructor metadata, serializers/other model backends,
CLR/lifetime and full original platform/input/rendering/accessibility/performance remain
in scope. Full window stack and entire goal are not complete. Keep whole objective intact.

## Previous fully validated checkpoint

# Current: nullable window-option contracts verified; next shared window lifecycle

Goal turn PROGRESS. Full original Dock-to-FerroDock goal ACTIVE, incomplete/unblocked.
Only FerroDock and owned SSD temp edited. Parent original source and sibling FerroUI
read-only. No agents/commit/push/PR/staged contents; new files intent-to-add. Preserve
actual desktop executable/app bundle/running window; no desktop rebuild/restart.
All temp and child TMPDIR=/Volumes/SSD/temp/ferrodock-port. Owned Cargo jobs sequential,
-j1; don't interrupt other chats or restart confirmed live handles on timeouts.

All owned jobs terminal. Workspace92613 passes1,751 tests/47 suites, zero failed/ignored:
UI373, markup179, diagnostics49, settings33, sample27, ReactiveUI543, Core62. Driver then
fails Clippy for test-only err().expect(); changed to equivalent expect_err(), no production
change. Resume62657 terminal0: strict all-target/selected release-library Clippy, standalone
62+2+7+3, six normal boundaries, fmt/21Python/source/theme/inspector/two generators/six vendor
guards pass. All17zero in ferrodock-null-window-options-validation-summary.json. Resume
retains successful workspace result. Don't rerun tests only for that equivalent style fix.
Original C#60128 terminal0:785 assertions,14new, zero source-only rows. Targeted64716 passes4.
Initial all-target49630 diagnostics only required Some(original target) at two test calls;
fixed before targeted/full checks. Helpers/logs ferrodock-null-window-options- on SSD,
source backups nullable-window-options-backup/ retain first copies. Last SSD free5.9GiB.
No cleanup/source deletion, dependency changes or actual desktop rebuild here.

DockWindowOptions.apply_to now takes Option<&dyn IDockWindow> and rejects None before
any writes with original window argument message. Base/protected apply_window_options
window is Option<Rc<IDockWindow>>; options None returns first even for null window.
Actual override can accept null or delegate to base with the same objects. Calls from
creation and existing tests retain original references in Some. Existing native ApplyTo
nullable descriptor retains its argument-error bridge and passes Some(borrow) to base.
Real property-change callbacks update later option reads: ParentWindow changes during
OwnerMode, modality during ParentWindow and taskbar override during IsModal. Setter and
notification order, option/window/parent identity verified in model/native/C# scenarios.
Four model tests + two native consumers mapped to the two source option units. Both
units now ported again after prior audit gaps were fixed; inventory232/454/1242 of1928.
Full objective and all broader partial/pending scope remain unchanged.

NEXT: make one coherent source window collection/lifecycle port. IRootDock.Windows and
all concrete/backend/native/UI views currently use ListHandle<Rc<dyn IDockWindow>>;
source IList<IDockWindow> retains null elements. Change the actual shared collection to
ListHandle<Option<Rc<dyn IDockWindow>>> (including generic factory list creation, native
IList indexer/method signatures, registration and downstream consumer types). Preserve
list/event/source identities; never filter nulls, add dummy models or mirror collections.
Inspect factory_base.rs WindowListCreator, controls/i_root_dock.rs, ReactiveUI root model,
model_items/registration, core/reactive native root references and dock_window_list_metadata.
Use nullable DockableList metadata as the existing pattern, without adding a new engine.

Window public/virtual contracts AddWindow/InsertWindow/RemoveWindow/CloseWindow and both
InitDockWindow overloads also need optional incoming root/window references. Protected
ResolveWindowCollectionRoot must accept/return source nullable root through actual hooks.
IDockWindowHostFactory.CreateHostWindow must accept actual nullable window before the
source conditional host lookup; native descriptors and enclosing overrides must match.
Source exact behavior: negative Insert returns before any dereference; positive Insert
creates Windows before index failure; Add appends null and raises OnWindowAdded(null)
before custom Init can accept it (default fails at window.Id unless host factory selected).
Init explicit overload fails first at window.Host; normal link/owner/factory/layout order
must be retained. Read ../../../Dock/src/Dock.Model/FactoryBase.Window.cs and Init.cs241-264 afresh.

Source window traversal consumers fail at first null dereference, retaining earlier
side effects and skipping later work only because of that failure. NavigateAdapter.Show
iterates live windows; Exit snapshots, then Save and Exit each item. DockState reads
window.Layout. Factory search and Init must preserve actual null virtual dispatch rather
than silently skip. InitDockWindow's Layout argument evaluation rereads are also subtle:
InitDockable(window.Layout, window.Layout.Owner) evaluates nullable first argument before
the second layout's Owner dereference. Don't insert an earlier expect or suppress null.
Floating layout rereads already forward Option to focus but need a dedicated arbitrary
getter-reentry fixture. Test source null list/event/init/order and native boxing/Count/
ItemsSource identity using existing retained generic nullable conversion support.

Remaining services/manager inputs, public attribute class, full generics/delegates and
metadata, serializers/other model backends, CLR/lifetime and full original platform/input/
rendering/accessibility/performance stay in scope. All previous baseline limitations
remain unless directly superseded by inspected source and passing evidence.

## Previous fully validated checkpoint

# Current: nullable focus/activation; remaining window option and lifecycle contracts

Goal turn PROGRESS. Full original Dock-to-FerroDock objective ACTIVE, incomplete and
unblocked. Previous user-facing status turn was no-progress; this turn takes the next
safe source implementation step. No agents/commit/push/PR/staged contents. Only FerroDock
and owned SSD temp edited. Original parent source and sibling FerroUI read-only. Preserve
actual desktop executable/bundle/window; no desktop rebuild/restart. All temp/child
TMPDIR=/Volumes/SSD/temp/ferrodock-port. One owned Cargo job at a time,-j1, never interrupt
other chats/builds or restart live handles because observation timed out.

All owned jobs terminal. Full validation71072 terminal0/all17zero in
ferrodock-null-focus-validation-summary.json. Workspace1,745 tests/47 suites, no failed/
ignored: UI373, markup177, diagnostics49, settings33, sample27, ReactiveUI539, Core62.
Strict all-target/selected release-library Clippy, standalone62+2+7+3, six normal
boundaries, fmt/21Python/source/theme/inspector/two generators/six vendor guards pass.
Dependencies unchanged; previous all4,428 controls tests remain applicable. After full
validation, only one test was strengthened to retain its original owner before Init;
final17983 targeted6 + strict all-target Clippy pass. No production change afterward.

C#4540 terminal0:771 assertions,21new, zero source-only rows. Targeted67081 passes6.
First98652/41215 expectations omitted original state synchronization root lookups;
actual source/Rust traces established them, expectations retain the complete order.
An accidental initial reference11012 ran old corpus because a root-relative editing
script used reference cwd; no edits occurred, and it was terminal before retry.
Initial74077 diagnostics superseded by fresh Some-only71425 terminal0 after8 rounds.
New helpers/logs ferrodock-null-focus-, backups nullable-focus-backup/ on SSD. Never
rerun initial edits or apply stale diagnostic spans; backup copies were updated while
editing and aren't guaranteed to be the first pre-change file.

IFactory/FactoryBase/generated FactoryImpl/native both views now use Option for focus
dock, active owner and fixed ActivateWindow item. ShouldFocus hook accepts both nullable
inputs and default returns true. Source ordering retained: active-change, Init, selected,
policy, focus, final selection. Null owner may fail only at final VisibleDockables after
custom focus returns. Real host SetActive can clear/replace live root.Window; actual
nullable OnWindowActivated payload reaches the normal event/tracking engine. Private
SetIsActive's null type-pattern no-op preserved; genuine focused-item dereferences remain.
Split/whole-pane floating forwards optional live layout into virtual focus. Ordinary
callers retain same handles in Some; native raw arguments remain accepted. Recording
fixtures unwrap only their explicitly nonnull recorded state, not production inputs.
Six new actual-model tests + two native consumers mapped to Init.cs and IFactory.cs.

Audit corrected two previously overclaimed units to partial: FactoryBase.WindowOptions.cs
and Core/DockWindowOptions.cs. ApplyWindowOptions's nonnullable window prevents the source
null-options/null-window no-op; ApplyTo's borrowed window prevents original null-window
ArgumentNullException. Inventory now230/456/1242 of1928, no implementation removed. Init
mapping also removes stale claims that production Factory/null list elements were absent.

NEXT: port these window-option input contracts coherently before expanding window
collection/initialization. apply_window_options Option<Rc<IDockWindow>> (base + protected
hook) must return immediately when options None, otherwise forward actual nullable window
to DockWindowOptions.ApplyTo. ApplyTo should represent optional borrowed/shared input,
check null at original ArgumentNullException location before setter writes, retain setter
order and parent identity; adapt native ApplyTo descriptor and both raw/canonical callers.
Read exact C# Core/DockWindowOptions.cs36 and FactoryBase.WindowOptions.cs before editing.
Then cover null-window+null-options guard, actual override handling and nonnull flags/order
against C#, Rust model and native metadata. Source statuses should only upgrade after
required parity evidence. Current unit docs record these concrete deferred members.

Next window lifecycle spans Add/Insert/Remove/CloseWindow, ResolveWindowCollectionRoot,
InitDockWindow overloads and IDockWindowHostFactory. Crucially root.Windows is still a
ListHandle<Rc<IDockWindow>>; original IList permits null elements, so preserve one actual
nullable shared window collection, not an adapter/mirror or filtered nulls. InitDockWindow
layout rereads must preserve argument-evaluation order before virtual Init; inspect source
rather than replacing every expect. Arbitrary getter reentry for nullable floating layout
is forwarded in production but has no dedicated getter fixture yet. Remaining service/
manager inputs, public attribute class, generics/delegates, serializers/other backends,
CLR/lifetime and full original platform/input/rendering/accessibility/performance stay in
scope. Last SSD free7.2GiB; monitor without blind cleanup. No cache/source deletion here.

## Previous fully validated checkpoint

# Current: nullable floating/window factory overloads

Previous/current goal turns PROGRESS. Full original Dock-to-FerroDock goal ACTIVE,
incomplete/no blocker. Preserve complete objective. Only FerroDock edits; original
parent C#/README/src/tests/samples and sibling FerroUI read-only. No agents, commits,
push/PR/staged contents. New files intent-to-add. Preserve actual desktop executable/
bundle/window; no desktop rebuild/restart. All temporary artifacts and child TMPDIR
MUST use /Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1; never interrupt
other chat builds or restart a confirmed live process because observation timed out.

All owned jobs terminal. Full validation17308 terminal0: all17 values zero in
ferrodock-null-floating-validation-summary.json. Driver ferrodock-null-floating-
validate.py; logs same SSD prefix. Workspace1,737 tests/47 suites zero failed/ignored:
UI373, markup175, diagnostics49, settings33, sample27, ReactiveUI533, Core62. Strict
all-target/selected release-library Clippy; standalone62+2+7+3; six normal boundaries;
fmt;21 Python; source/theme/inspector; both generators/all six vendor guards pass.
Completion rejects232/454/1242 of1928. No source status upgrade or dependency edits;
prior all4,428 controls tests cover unchanged retained patch. No cleanup/source deletion.

C#27452 terminal0:750 assertions,27 new, zero source-only rows. Targeted61806 passed8.
Initial44542 check terminal101; first migration stopped at Option dock upcast plus
accidental expect in unrelated DockAsDocument, both corrected before any green check.
Fresh12424 terminal101;91006 stopped at sample override signature after3 rounds. Sample
now takes Option and passes same value to base (original conditional title unchanged).
Targeted43322 compile diagnostics from remaining old calls superseded by fresh check/
Some-only55312 terminal0 after4 rounds. Never apply old spans or rerun initial mutation
scripts. Logs/scripts ferrodock-null-floating-, backup nullable-floating-backup/ on SSD.

Float/FloatAll/CreateWindowFrom/SplitToWindow overload items/docks and protected Prepare
item now Option through original slots, generated FactoryImpl and native views. Wrappers
send actual None/options/coordinates to enclosing virtual overloads. Plain creation None
or unsupported class returns None before root/creation. Options overload prepares first,
then virtual plain, then Apply only for actual window. Prepare guards null options,
non-DockableWindow or preexisting parent before any lookup. Otherwise actual None reaches
virtual FindRoot; captured parent mutates same options even if plain creation returns
None. Custom prepare abort/mutation and Apply(null options) order remain original.

Default floating null fails in ResolveOperationDock Owner access before later item.Owner;
ordinary coordinate/size fallback and original handles preserved. Split accepts nullable
dock into virtual FindRoot, then nullable item into actual prepare/remove/undocked/create
steps, retaining collapse flag and options identity. No fake objects, mirrored state,
engine replacement or host window presentation. Existing snapshot/null list behavior
continues through original engine. Actual sample executable/bundle/window preserved;
only sample source override signature and test/library builds updated.

Eight nullable_floating_tests and three nullable_floating_sdk_tests match27 original
NullableFloatingReference.cs cases. Both views cover all eight slots, canonical optional
declarations plus raw nonnull calls, exact returned window/parent/options identity and
applied modality/taskbar fields. New files mapped to FactoryBase.cs, Dockable/WindowOptions
partials and IFactory.cs. No tests asserting source behavior removed or callbacks filtered.

NEXT: remaining focus/activation input contracts (factory_base_focus.rs, factory_base_init
and hooks) versus original FactoryBase.Init.cs290/324 and surrounding InitActive body.
SetFocusedDockable still has nonnull dock input before custom FindRoot can handle None;
ShouldFocusActiveDockable default returns true without reading either input. Expand
InitActive owner/ShouldFocus inputs coherently, preserving override/activation/event order.
ActivateWindow still takes nonnull item and rereads live root.Window around SetActive/
OnWindowActivated; nullable event input must survive source reentry. Private SetIsActive
uses source type-pattern no-op for null and must not pre-reject an owner reread. In newly
floating Split, existing window.layout expect before SetFocused is still a temporary
boundary when layout changes between reads: pass actual None to nullable focus override
as source does. Do not cement such expects into source default behavior. Remaining
window collection/option lifecycle hooks, factory/service/manager inputs, serializers,
other backends/services, public attributes/generic collections/delegates, CLR/lifetime
and full original platform/input/rendering/accessibility/performance remain required.
Never narrow goal around nullable work/sample. C# writes BETWEEN owned Cargo jobs, in
reference project cwd with SSD TMPDIR and -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/.
Last SSD free5.1GiB; monitor without blind cleanup. Other chat builds untouched.

## Previous fully validated checkpoint

# Current: nullable hidden-dockable hide/restore lifecycle

Previous/current goal turns PROGRESS. Full original Dock-to-FerroDock goal ACTIVE,
incomplete/no blocker. Preserve complete scope. Only FerroDock edits; original parent
C#/README/src/tests/samples and sibling FerroUI read-only. No agents/commit/push/PR/
staged contents. New files intent-to-add. Preserve actual desktop executable/bundle/
window; no desktop rebuild/restart. All temp and child TMPDIR MUST use
/Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1; never interrupt other chat
builds or restart a confirmed live job solely because an observation timed out.

All owned jobs terminal. Full validation16333 terminal0: all17 values zero in
ferrodock-null-hidden-validation-summary.json. Driver ferrodock-null-hidden-validate.py,
all logs same SSD prefix. Workspace1,726 tests/47 suites zero failed/ignored: UI373,
markup172, diagnostics49, settings33, sample27, ReactiveUI525, Core62. Strict all-target
and selected release-library Clippy; standalone62+2+7+3; six normal boundaries; fmt;
21 Python; source/theme/inspector; both generators/all six vendor guards pass. Completion
correctly rejects232/454/1242 of1928. No vendor/dependency changes; prior all4,428 native
controls tests cover same retained patch. No cache cleanup/source deletion this turn.

C#29810 terminal0:723 assertions,27 new, zero source-only rows. Targeted model38852
terminal0:6 new cases pass, then full suite validates all. Initial all-target13096
terminal101 superseded by Some-only adaptation33990 terminal0 after4 rounds. Source
backups nullable-hidden-backup/, migration/check/reference/test logs ferrodock-null-hidden-
on SSD. Never apply stale diagnostics or rerun initial source mutation scripts.

HideDockable/RestoreDockable item slots now Option through original virtual methods,
generated FactoryImpl and both native factory declarations. Hide passes actual None
through fixed pin membership, nullable protected pin cleanup and second virtual root
query before hidden-list allocation and first incoming Owner access. A pin hook can
redirect the second lookup to another actual root; earlier pins and original root lists
remain as source custom override dictates. Restore short circuits absent membership;
if original list contains None, removes actual first null entry, raises restored event
then fails at OriginalOwner. Original null payload/index, committed removal and observer
hidden-list replacement remain. Nonnull Hide rereads live hidden list after removed
event; Restore rereads OriginalOwner after restored event, using observer-selected actual
destination/model without copied state. ID result guards mirror source not-null guard;
first null.Id predicate failure and original lazy root/query order are preserved.

Six nullable_hidden_tests and three nullable_hidden_sdk_tests match27 C# cases in
NullableHiddenReference.cs. Native ID consumers use real DockControls with InitializeLayout/
InitializeFactory disabled, registered with the real factory, as original C# does. Both
native views accept actual null overrides/removal events, canonical optional declaration
lookups retain raw nonnull consumers. New files source mapped; no source status upgraded.
No fake null objects, mirror collections, alternate engine or UI-specific workaround.

NEXT: expand remaining floating/window factory item slots together: float_dockable /
float_all_dockables and with_options in factory_base_float.rs, create_window_from and
with_options in factory_base_create_window.rs, plus their protected option-preparation
hook where needed. Source FactoryBase.Dockable.cs1069/1075/1120/1126 and FactoryBase.cs
628/784 are reference. Plain CreateWindowFrom(null) matches no switch arm and returns
null; do NOT insert an unconditional dereference. Options overload prepares options
before virtual plain CreateWindowFrom, then applies options only for actual window.
Float wrappers dispatch actual nullable item/options to enclosing virtual overloads;
default operation resolver failure order must remain original. Include supported custom
null-aware overrides/default paths, original snapshot/window/model identity and native
metadata/corpus. Then remaining factory/service/manager inputs, serializers, other model
backends/services, attributes/generic collections/delegates, CLR/lifetime and full original
platform/input/rendering/accessibility/performance scope. Never narrow whole goal around
sample or nullable work. C# corpus writes BETWEEN owned Cargo jobs; reference project cwd,
SSD TMPDIR and -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/. Last free6.5GiB;
monitor without blind cleanup. Actual sample and other chat builds remain untouched.

## Previous fully validated checkpoint

# Current: nullable pinned preview/hide lifecycle and original reentry

Previous/current goal turns PROGRESS. Full original Dock-to-FerroDock goal ACTIVE,
incomplete/no blocker; preserve complete objective. Only FerroDock edits; original
parent C#/README/src/tests/samples and sibling FerroUI read-only. No agents/commit/
push/PR/staged contents. New files intent-to-add. Preserve actual desktop executable/
bundle/window; no rebuild/restart. All temp and child TMPDIR MUST use
/Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1; never interrupt other
chat builds or restart a confirmed live process on an observation timeout.

All owned jobs terminal. Full validation94406 terminal0: all17 values zero in
ferrodock-null-preview-validation-summary.json. Driver ferrodock-null-preview-
validate.py and all stage logs share SSD prefix. Workspace1,717 tests/47 suites,
zero failed/ignored: UI373, markup169, diagnostics49, settings33, sample27,
ReactiveUI519, Core62. Strict all-target/selected release-library Clippy; standalone
62+2+7+3; six normal boundaries; fmt;21 Python; source/theme/inspector; both generators
and all six vendor integrity guards pass. Completion rejects232/454/1242 of1928.
No dependency edits; prior all4,428 native controls tests cover unchanged patch.

C#43236 terminal0:696 assertions,27 new, zero source-only rows. Model10156 passed8.
Initial model3705 terminal101 lacked INPC trait import in new test module; imported
existing trait, no behavior/assertion changed. Initial64667 terminal101 superseded by
Some-only adaptation77315 terminal0 after4 rounds. Logs/migration helpers with prefix
ferrodock-null-preview-, backups nullable-preview-backup/ in SSD task directory.
Do not apply stale compiler spans or rerun initial edit scripts. Root Git intent-to-add
path corrected after subdirectory pathspec error; source mapping and staged state pass.

Preview/Toggle item and Hide root now Option through source fixed (nonvirtual) methods,
generated FactoryImpl and both native declarations. Object-null commands remain no-op;
typed null reaches actual FindRoot. Fixed Toggle relooks up through fixed Preview path.
Old preview cleanup precedes incoming Owner failure. KeepVisible vs OriginalOwner null
snapshot failure order is preserved, including earlier committed removals/stale active
state when later null aborts the loop. Removal rereads live root.PinnedDock; final count/
selection keeps cached original preview/list. Root replacement/clearing during Owner
notification retains those original identities and first failure location.

Reentry when preview VisibleDockables is created may clear root.PinnedDock. Actual
None now reaches virtual InitDockable(None,Some(root)); previous pre-dispatch expect
was removed. Source custom Init can restore a live replacement and final selection;
default Init fails at Context after original old-preview list commit. Nonnull Init can
clear the live preview, then original ActiveDockable access fails while saved list
remains committed. No fake object, alternate engine, mirrored storage or event workaround.
Kept preview entries retain cached list and first-nonsplitter selection identity.

Eight nullable_preview_tests and two nullable_preview_sdk_tests match27 original
NullablePreviewReference.cs assertions. All new files mapped to actual original
FactoryBase.Dockable.cs/IFactory.cs. No source status upgrade. Canonical optional
metadata, raw original nonnull handles and all previous native/UI scenarios pass.

NEXT: remaining HideDockable/RestoreDockable slots in factory_base_hide.rs versus
FactoryBase.Dockable.cs1623/1663 are still nonnull. Preserve source IsPinned/PinCore
cleanup before repeated virtual FindRoot, hidden-list creation before incoming Owner
access, and Restore Contains/Remove(null)/restored event before OriginalOwner failure.
Their ID helpers keep source lazy predicate/first-null Id dereference; do not make
null skipping or UI-specific repairs. Then remaining factory/service/manager inputs,
serializers, other model backends/services, public attribute/generic collections/
delegates, CLR/lifetime and full platform/input/rendering/accessibility/performance.
Do not narrow original goal around current nullable work or sample. Future C# writes
must be BETWEEN Cargo jobs to protect include_str! readers; run in reference project
with SSD TMPDIR and -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/. Last free
7.4GiB; monitor without blind cleanup. Actual sample/other chats remain untouched.

## Previous fully validated checkpoint

# Current: nullable moves, swaps and connected pin/capability APIs

Previous/current goal turns PROGRESS. Full original Dock-to-FerroDock goal remains
ACTIVE/incomplete/no blocker. Preserve complete scope. Only FerroDock edits; original
parent C#/README/src/tests/samples and sibling FerroUI read-only. No agents, commits,
push/PR or staged contents. All new files intent-to-add. Preserve actual desktop
executable/bundle/window; no desktop rebuild/restart. All temp and child TMPDIR MUST
use /Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1; never interrupt other
chat builds or restart confirmed live jobs merely on an observation timeout.

All owned jobs terminal. Full68875 passed workspace1,707 tests/47 suites (zero failed/
ignored), then Clippy stopped at3 redundant cast-parenthesis pairs in Some(IndexOf)
adaptation (6 messages). Removed exactly redundant parentheses; no expression behavior
change. Resume28166 terminal0 completed strict workspace all-target and selected
release-library Clippy and remaining gates. All17 values zero in ferrodock-null-move-
validation-summary.json. Drivers ferrodock-null-move-validate.py / -resume-validation.py;
all logs same SSD prefix. Already-passing behavioral suite retained for syntax-only
lint corrections. UI373, markup167, diagnostics49, settings33, sample27, ReactiveUI511,
Core62. Standalone62+2+7+3; six normal boundaries; fmt;21 Python; source/theme/inspector;
both generators and all six vendor guards pass. Completion rejects232/454/1242 of1928.
No dependency edits; prior all4,428 native controls tests cover unchanged retained patch.

Original C#42630 terminal0:669 assertions,38 new, zero source-only rows. Targeted model
30668 passed8. Initial43422 and C#24309 failed incomplete expected event traces: original
added/removed nonnull payloads trigger5 FindRoot calls through state synchronization.
Both references now explicitly include those calls; C# validates exact same trace. No
callback filtered or synchronization suppressed. Initial full4163 failed old Core
MovePinned declaration lookup; only canonical Option lookup fixed, original raw Rc and
pinned-order assertions preserved. Attempt1/attempt2 validation logs retained on SSD.

All move/swap docks/items, Pin/Unpin items and public capability resolver item slots now
Option. Same original shared collection IndexOf/Contains/events and actual model Rc,
no replacement objects/engine/mirrored state. Within move/swap retains mutation/event/
selection order and original swap snapshots through reentry. Between Move preserves
fixed Unpin before target access, target list creation before source access, and original
same-owner insert-before-remove. Between Swap preserves source-list short circuit and
both list commits before actual nullable Init. Same-owner between Swap still inits twice
for null/equal slots. Default Init null failure leaves original committed lists intact.
Pinned movement uses first matching strip. Unpin fixed membership reaches enclosing
virtual Pin with actual None. Source default Pin resolves operation dock before flags;
static capability inputs preserve original Owner/flag failures. Metadata regenerated
and canonical Option declarations plus raw nonnull argument consumers pass.

Eight nullable_move_tests and three nullable_move_sdk_tests match38 original C# cases
in NullableMoveReference.cs. All new files source-mapped to FactoryBase.Dockable.cs,
IFactory.cs and DockCapabilityResolver.cs. No source status upgraded. Some-only migration
17630 terminal0 after7 rounds; initial73824/reference-argument diagnostics superseded.
Backups nullable-move-backup/, scripts/dock/logs ferrodock-null-move-. Never apply stale byte
spans or rerun initial mutation scripts on current sources. Recording fixtures unwrap
only their explicitly nonnull recording payloads; default delegates actual Option.

NEXT: source factory preview/hide lifecycle inputs remain nonnull: factory_base_preview.rs
PreviewPinnedDockable, TogglePreviewPinnedDockable, HidePreviewingDockables and helper
paths versus original FactoryBase.Dockable.cs lines507/561/605. Preserve source fixed
(nonvirtual) dispatch, custom nullable FindRoot/Init/activation, cached preview/model/list
identity, cleanup and first dereferences. Then remaining factory/service/manager slots,
serializers, other model backends/services, public attributes/generic collections/
delegates, CLR/lifetime and full platform/input/rendering/accessibility/performance scope.
Do not narrow the full goal around current nullable work or sample. C# corpus writes
must be BETWEEN owned Cargo jobs (include_str! safety), from its project cwd with SSD
TMPDIR and -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/. Last SSD free10GiB;
monitor without blind cleanup. Actual sample and other chats remain untouched.

## Previous fully validated checkpoint

# Current: nullable factory targets, pin root and protected helpers

Previous goal turn PROGRESS:1,688 tests/604 C# assertions/all17 gates passed. Current
goal turn PROGRESS: next exact factory argument contracts implemented and validated.
Full original goal ACTIVE/incomplete/no blocker; preserve complete scope. Only FerroDock
edits, original C#/parent README/src/tests/samples and sibling FerroUI read-only.
No agents/commit/push/PR/staged contents. New files intent-to-add. Preserve actual
desktop executable/bundle/window. All temp/TMPDIR /Volumes/SSD/temp/ferrodock-port.
Owned Cargo sequential,-j1; never interrupt other chats or restart a live process
because an observation times out. Previous full scope/instructions below still apply.

All owned jobs terminal. Full validation31139 terminal0, all17 summary values zero
in ferrodock-null-target-validation-summary.json. Driver ferrodock-null-target-
validate.py, all logs same SSD prefix. Locked workspace1,696 tests/47 suites, zero
failed/ignored: UI373, markup164, diagnostics49, settings33, sample27, ReactiveUI503,
Core62. Strict all-target and selected release-library Clippy; standalone62+2+7+3;
six normal dependency boundaries; fmt;21 Python; source/theme/inspector; both generators
and all six vendor guards pass. Completion correctly rejects232/454/1242 of1928.
No dependency edits; prior all4,428 native controls tests cover same retained patch.

C# reference3171 terminal0:631 assertions,27 new, zero source-only rows. Model46595
terminal0:7 new cases pass, then full suite validates all. Initial15599 terminal101
was one nonexistent test getter; fixed to original GetPinnedBounds/NaN checks. Initial
all-target92897 terminal101 superseded by compiler Some-only adaptation31000 terminal0
(ten rounds). Production backup nullable-target-backup/. Migration script and logs in
SSD. Do not apply stale diagnostic spans; compiler wrapping only preserves same Rc.

AddDockable target nullable through original Init before target VisibleDockables access;
default Init Owner/Factory side effects and original null-item Context failure retained.
Six protected visible-list helper targets and active-selection dock/removed arguments
nullable. Default removed argument unused; original empty/null child/splitter/index and
selection identity retained. Both protected pin root and item now nullable. Root-null
membership can return no-op, update pinned bounds before collection failure, or find
another membership root then still fail at explicit null root.PinnedDock during unpin.
No early root validation, fake objects, mirrored lists or engine replacement. Metadata
and FactoryImpl regenerated after trait formatting, generators pass with no drift.
Existing recording fixtures unwrap only their explicit nonnull callback inputs.

Seven nullable_target_tests, one new native consumer and existing override consumer
extension match27 original C# assertions in NullableTargetReference.cs. All new tests
mapped to actual FactoryBase.Dockable.cs and IFactory.cs; no source status upgraded.
Run future C# corpus writes BETWEEN Cargo jobs to protect include_str! corpus readers.
Use its project cwd, SSD TMPDIR, -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/.

NEXT: inspect remaining MoveDockable/MovePinnedDockable/SwapDockable slots and their
original null entries, source IndexOf/Contains, snapshot/events/virtual Init and same/
between-dock behavior. UnpinDockable still takes nonnull item before movement; preserve
its source fixed dispatch, nullable IsPinned, then virtual Pin eligibility path when
expanding those contracts. Do not interpret current expects as intended default order.
Other factory/service inputs still remain, followed by serializers, other model backends,
public attributes/generic collections/delegates, CLR/lifetime and full original desktop
input/rendering/accessibility/performance scope. Never narrow full goal around sample
or current nullable work. SSD last observed11GiB; no cache cleanup/source deletions or
actual sample rebuild/restart this turn. All temp stays on SSD.

## Previous fully validated checkpoint

# Current: nullable insertion, removal and protected pin item arguments

Previous status turn: evidence gathered, no implementation progress. Current goal
turn: PROGRESS. Full original Dock-to-FerroDock goal ACTIVE/incomplete, no blocker.
Only FerroDock edits; original C#/parent README/src/tests/samples and sibling FerroUI
read-only. No agents/commit/push/PR/staged contents. All new files intent-to-add.
Preserve actual desktop executable/bundle/window. All temporary artifacts and TMPDIR
use /Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1; never interrupt
other chats or restart a live job solely because an observation times out.

All owned jobs terminal. Main52674 completed workspace/lint/standalone/dependency/
format/Python/source/theme/inspector/vendor gates, stopped at factory-generator.
The only generated difference was whitespace in InsertDockable macro signature after
trait formatting. Compared a.split()==b.split(), regenerated the exact file and resumed.
Resume63749 terminal0 passes remaining gates; all17 summary values are zero in
ferrodock-null-mutation-validation-summary.json. Drivers ferrodock-null-mutation-
validate.py and -resume-validation.py, all logs same prefix in SSD task directory.
Pre-regeneration backup ferrodock-null-mutation-generated-before.rs proves token identity.
Full workspace1,688 tests/47 suites, zero failed/ignored: UI373, markup163,
diagnostics49, settings33, sample27, ReactiveUI496, Core62. Strict all-target and
selected release-library Clippy; standalone62+2+7+3; six normal boundaries; fmt;
21 Python; source/theme/inspector; both generators and all six vendor guards pass.
Completion still correctly rejects232 full/454 partial/1242 pending of1928.
No vendor/dependency changes; prior all4,428 native controls tests cover same patch.

Original C# reference46544 terminal (handle missing; log successful):604 assertions,
20 new, zero source-only rows. Log ferrodock-null-mutation-reference.log; corpus
is tests/Dock/data/factory-markup-reference.tsv. Run future reference BETWEEN owned Cargo
jobs to avoid include_str! reading a partial corpus. Use TMPDIR on SSD and
-p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/ from its project directory.

InsertDockable nullable dock/item and RemoveDockable nullable item now pass through
actual original virtual slots and both Core metadata views. Negative insert index
returns before Init or argument access. For nonnegative indices actual Init override
runs before original VisibleDockables access. Same original shared list, null payload,
index, empty/opened state and added event order. Invalid indices preserve Init/list
creation before failure. Remove passes null through virtual root and pin cleanup
before original Owner access. Both protected pin item overloads are nullable; default
first dereference remains original Owner. FloatAll passes actual null snapshot entries
to original Remove/Add/moved/active slots without an early expect. Original snapshot
retained, no fake objects or mirror state. FactoryImpl regenerated from source trait.
Existing nonnull call sites wrap same Rc; recording fixtures retain nonnull contract.

Six new nullable_mutation_tests and two native consumers pass in full workspace.
Both API views accept actual null and dispatch to enclosing overrides. Original raw
nonnull argument handles and canonical optional metadata declarations also pass.
New source/reference tests mapped to original FactoryBase.Dockable.cs and IFactory.cs.
No source status upgraded. Core/ReactiveUI model/native metadata and C# corpus consistent.

NEXT: inspect and port remaining nullable factory/service arguments, including explicit
protected pin root, AddDockable target, unused removed argument in protected active
selection and broader operation inputs. Preserve exact source failure/override/event/
reentry order; do not mistake temporary expects for source contracts. Then broad
serializers, other model backends/services, public attribute/generic/delegate/CLR/
lifetime and platform/input/rendering/performance scope. Preserve complete objective.
Initial check42395 terminal101 superseded by migration57134 terminal0; targeted model
97896 passed6 cases, then all1,688 validated. SSD last observed11GiB; monitor without
blind cleanup. No actual sample rebuild/restart, source deletions or other chat changes.

## Previous fully validated checkpoint

# Current: nullable protected state hooks and root lookup

Previous/current goal turns: PROGRESS. Full original Dock-to-FerroDock goal ACTIVE,
incomplete, no blocker. Preserve scope. Only FerroDock edits; original C#/parent
README/src/tests/samples and sibling FerroUI read-only. No agents/commit/push/PR/staged
contents. All new files intent-to-add. Preserve actual desktop binary/bundle/window.
All temp/TMPDIR use /Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1;
never interrupt other chat builds or restart a confirmed live process on timeout.

All owned jobs terminal. Main validation30816 completed code gates, stopped only for
missing source mapping for new nullable_state_tests.rs. Added actual original-source
mapping; resume46890 terminal0 completed remaining gates. All17 values zero in
ferrodock-null-state-validation-summary.json under SSD task directory. Drivers
ferrodock-null-state-validate.py and -resume-validation.py; logs same prefix.
Locked workspace1680 tests/47 suites, zero failed/ignored: UI373, markup161, diagnostics49,
settings33, sample27, ReactiveUI490, Core62. Strict all-target/selected release-library
Clippy, standalone62+2+7+3, six normal boundaries, fmt,21 Python tests, source/theme/
inspector, both generators and six vendor guards pass. Completion rejects232/454/1242.
No vendor/dependency change this turn; previous all4428 controls tests cover same patch.

Original-engine reference78108 terminal0:584 assertions, zero source-only rows.
C# NullableStateReference.cs is called by existing harness and covers13 new assertions.
Log ferrodock-null-state-reference.log; corpus tests/Dock/data/factory-markup-reference.tsv.
Run future reference BETWEEN Cargo jobs from scripts/dock/factory-markup-reference, TMPDIR
on SSD, -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/ (trailing slash). Do not
let include_str! read a partially written corpus. Original projects remain read-only.

Five argument contracts now nullable: protected update/recursive/resolve hooks,
IFactory.FindRoot and source nonvirtual IsDockablePinned. FactoryImpl regenerated.
Default FindRoot fails at original Owner access, but recursive owner reread passes
actual None into the virtual override. State recursion sends null child through
original enclosing overrides before type checks; split extras keep order/dedup.
Update calls resolver before entering suppression/assigning state, with scope restored
on failure. Resolve helpers preserve original null type patterns and OriginalOwner
failure order; pin membership accepts null in original strips. Both Core interface/base
metadata match. Existing nonnull callers use Some(same Rc); no fake objects or engine.
Recording fixture callbacks explicitly expect their existing nonnull test inputs;
new null-aware cases use actual composed Factory subclasses and original shared lists.

New nullable_state_tests.rs4 cases and native consumer in null_list_sdk_tests.rs1 case
are in full passing suite. New native test invokes FindRoot(null)/IsDockablePinned(null)
through both original views, checking returned root identity. Existing Core declaration
lookup fixture expects canonical nullable parameter type. Source confirms same paths.

NEXT: faithful nullable RemoveDockable/InsertDockable and other factory/service slots,
including factory_base_float remaining RemoveDockable expect before null-aware overrides.
Inspect original bodies first; preserve override/event/first-dereference/reentry order.
Then broad remaining serializers/model backends/services, public attribute class,
generic collections/delegates/CLR/lifetime and platform/input/rendering/performance.
Do not redefine completion around sample or nullable work. Inventory232/454/1242.

Recovery: initial edit script stopped partway before body edits, fixed before all-target
check passed. Some-only helper ferrodock-null-state-migrate.py reached green after six
rounds (83231 terminal0); never apply stale byte spans. Backups in nullable-state-backup/.
Model79957 passed4 targeted cases then full1680 validates all current code.
Before model tests, SSD nearly full from concurrent work; with owned jobs terminal
retired36 rebuildable library test harnesses/7.2GiB, sample prefix excluded. Manifest
ferrodock-null-state-test-cache-cleanup.json. Actual sample/source/native/dynamic libraries
untouched, no other chat interrupted. Last observed free2GiB; monitor, avoid blind cache
cleanup. Initial all-target sessions8190/71000 terminal101 were superseded. No live jobs.

## Previous validated checkpoint

# Current: nullable collection elements and native object values

Previous/current goal turns: PROGRESS. The full Dock-to-FerroDock goal remains ACTIVE
and incomplete. No blocker. Do not mark complete or narrow the original objective.
Only FerroDock edits; original parent C#/README/src/tests/samples and sibling FerroUI
are read-only. No agents, commits/push/PR or staged contents. All new files intent-to-add.
Preserve actual desktop executable/bundle/window; library/test builds are allowed.
All temp/TMPDIR MUST use /Volumes/SSD/temp/ferrodock-port. Owned Cargo sequential,-j1;
never interrupt other chat builds or restart a live job after observation timeout.

All owned jobs are terminal. Final validation18467 closed0: all17 stages zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-list-validation-summary.json.
Driver ferrodock-null-list-validate.py and stage logs share that prefix. Current code
passes locked workspace1675 tests/47 suites, zero failed/ignored (UI373, markup160,
diagnostics49, settings33, sample27, ReactiveUI486, Core62). Strict all-target and
selected release-library Clippy, standalone62+2+7+3, six normal boundaries, formatting,
21 Python tests, source/theme/inspector checks, both generators and six retained
provenance guards pass. Completion correctly rejects232 full/454 partial/1242 pending.

Native controls: verified SSD standalone copy, every source hash matches retained
provenance; only harness manifest/lock differ. Driver run-native-controls-tests.py
used offline/locked Cargo. Native unit69218 passed2; full actual test binary73118
passed ALL4428 tests, zero failures/ignored. Log ferrodock-null-list-controls-all-tests.log.
Filtered items_source58 and selection437 pass but overlap; do not add their counts.
Original C# reference23720 closed0:571 assertions, zero source-only labels. Log
ferrodock-null-list-reference-final.log; corpus tests/Dock/data/factory-markup-reference.tsv.
Run future reference from scripts/dock/factory-markup-reference, with TMPDIR on SSD and
-p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/ (trailing slash), between Cargo
jobs so include_str! cannot read a partially written corpus. Preserve original sources.

ListHandle/ReadOnlyListHandle/ReadOnlyObservableCollection dockable elements now carry
Option<Rc<dyn IDockable>> across Domain, backends, native descriptors, UI and sample.
One actual shared collection engine; no fake null object, mirror state or filtering.
Generic CreateList specialization/creator thunks and fluent/root lists match; FactoryImpl
regenerated. Base Add passes actual null through the Init override, appends it, updates
empty/opened state and raises original added payload. Normalization keeps notifying
identity and null indices. Splitter cleanup retains null and original first-null fallback.
CloseRange, navigation, DockState, outer-content/group checks and ownership traversal
preserve source null type patterns, dereference order and yielded entries.

Queries/predicates carry nullable references. Some(None) is a yielded null, not end.
Cursors retain Current/null/last value and MoveNext order over the original iterator.
Core list indexer/Add/Insert/Remove/Contains/IndexOf use the same nullable elements.
The Core DockablePredicate alias is reexported by SDK; private storage/iterator aliases
keep identical underlying types while satisfying strict Clippy. Four new model cases,
three consumers, one UI projection and one Core delegate regression pass.

Sixth isolated dependency: vendor/ferroui-controls0.3.0,855 published files, ONLY
items_source.rs changed. Exact archive/VCS/all-file hashes and patch are retained.
Generic box_item/unbox_item/equality use existing Base native conversion, with the
exact-value equality fast path retained. Core predicate adapter normalizes the same
way. Registered references expose their actual native model object, restoring the
same Rc via from_markup_value. Selector tests keep exact selection identity and add
replacement identity. Native events retain null payloads, counts and indices without
copied storage. Compiled indexers intentionally preserve C# stale-until-rebind behavior.
Registry and sibling sources remain untouched. All4428 dependency tests pass.

IMPORTANT remaining null boundaries: protected update/state-recursion/resolution hooks,
FindRoot, RemoveDockable/InsertDockable and service/manager inputs still include nonnull
Rust arguments. factory_base_state child.expect and factory_base_float RemoveDockable
expect remain temporary boundaries before null-aware custom overrides. Do NOT cement
those failures as intended source behavior. Propagate Option through the original
slots, preserve source first dereferences, override/event/reentry order and regenerate
FactoryImpl/metadata as required. No fake objects, alternate engine or UI workaround.

NEXT: inspect original FactoryBase.DockingState.cs and FactoryBase.Locator.cs; port
nullable protected recursive/update/resolve slots and FindRoot. Test custom null-aware
update/resolve/FindRoot overrides and default failure order against unchanged C#.
Then continue remaining factory/service nullable slots and broad original source
families (serializers, other model backends/services, generic collections/delegates,
public attribute class, CLR/lifetime and platform/input/rendering/performance scope).
Inventory stays232/454/1242 until concrete source requirements are met.

Recovery evidence: initial all-target fixture migration logs and helpers are on SSD
(ferrodock-null-list-fix.py and -migrate.py). Do NOT apply old compiler byte spans to
current files. Read fresh diagnostics first. Backups of initial199 declarations/48 files
are in nullable-list-migration/. All current migrations are tested; no helper is live.
Earlier failures: first full link hit errno28 (attempt1 logs); second consumer159/160
exposed a transport-box assertion (attempt2 logs); pre-alias1675 test pass then Clippy
failed complexity/closure checks. Corrected native identity assertions and aliases are
fully validated by final18467. No need to rerun completed jobs without new changes.

Disk cleanup manifests (owned target only): ferrodock-retired-static-archives.json
(39/6.18GiB), -v2.json(152/7.59GiB), -v3.json(258/12.53GiB),
ferrodock-link-space-cleanup.json(54 superseded test/static outputs/5.98GiB; newest test
per crate/newest2 archives retained, sample prefix excluded). Also one obsolete0.67GiB
registry controls archive removed after proving its registry source absent from locked
graph; local-source artifacts untouched (ferrodock-obsolete-registry-controls-archive.json).
No source/native libraries/dynamic libraries/actual desktop or bundle files were removed.
Other chat builds were not interrupted. Do not repeat cleanup blindly; last free15GiB.

## Previous validated checkpoint

# Current: nullable factory override arguments and original first dereferences

Previous goal turn: PROGRESS (full validation and source callback parity records).
Current: PROGRESS (original factory nullable slots, source/consumer/reference tests).
Full original Dock-to-FerroDock goal remains active/incomplete. No blocker.
Only FerroDock edits; preserve original C#, sibling FerroUI and actual desktop sample
executable/bundle/window. No agents, commits/push/PR or staged contents.

USER REQUIREMENT: all temporary artifacts and TMPDIR must use
/Volumes/SSD/temp/ferrodock-port, never /tmp or the main disk. Moved138 owned-prefix
artifacts; historical docs and temporary script prefixes now point at the SSD.
Do not move/delete unrelated chat files. Owned Cargo one at a time, -j1. Do not
interrupt other chat builds or restart a confirmed live job after observation timeout.

All owned jobs are terminal. Final validation session10004 closed0; Python71002 is
absent and every one of17 stages is zero in
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-factory-validation-summary.json.
Command /Volumes/SSD/temp/ferrodock-port/ferrodock-null-factory-validate.py; all stage
logs use the same prefix. No need to resume/restart those completed jobs.

Full locked workspace **1666 Rust tests / 47 suites**, zero failed/ignored: UI372,
markup157, diagnostics49, settings33, sample27, source models482, Core61, recyclingSDK2,
backendSDK7, runtime3. Strict workspace all-target and release-library Clippy pass,
alongside standalone SDK/runtime tests61+2+7+3, all six normal dependency boundaries,
formatting, 21 Python tests, source/theme/inspector checks, both generators and five
retained vendor integrity guards. No vendor/dependency change this checkpoint.
New files are intent-to-add, authored whitespace clean, staged contents empty;
original parent README/src/tests/samples unchanged. Actual desktop sample preserved.

IFactory.init_dockable/add_dockable/set_active_dockable now take nullable reference
arguments in their original single virtual slots. FactoryImpl was regenerated from
those contracts; Core interface and abstract base metadata match. Default Init/active
fail at their original first Context/Owner dereference, rather than in the document/tool
front end. Original document/tool bodies pass None directly and reread Factory before
each of the three calls. Nonnull engine/service/UI/sample/test callers retain the same
original handles inside Some. Existing nonnull recording fixtures assert their own
inputs explicitly; separate null-aware actual Factory subclasses exercise new behavior.

Four new source tests cover document/tool/command null factory order, owner identity,
replacement/removal after Add or activation, default null failures, and original base
Add dispatching to Init override before mutation. Compiled/runtime null-command consumers
now use null-aware factory overrides. One new direct native consumer checks all three
slots through both original interface and abstract FactoryBase metadata.
C# reference47456 closed0: **544 assertions / 545-line TSV**, all rows pass. Thirteen
new matched-source assertions cover those same paths. One explicitly SOURCE-ONLY
assertion proves the still unsupported nullable IList element path. Source log:
/Volumes/SSD/temp/ferrodock-port/ferrodock-null-factory-reference.log.
Successful reference command runs from scripts/dock/factory-markup-reference with TMPDIR
on SSD and -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/ (trailing slash).
Two earlier setup failures changed no original project files and were superseded.
Targeted model44239 and consumer59182 also closed0 (482 and157 tests).

IMPORTANT remaining gap: source IList<IDockable> permits null elements. If custom
InitDockable accepts null and returns, original base AddDockable appends null; current
ListHandle<Rc<dyn IDockable>> cannot represent it. Source-only assertion
SOURCE-ONLY-nullable-base-add-list-element records this exactly. Do not cement the
current rejection as desired behavior, substitute a fake null dockable/list engine,
or claim full nullable/collection/CLR parity. Other factory dock-argument nullability,
general delegates, public attribute class, serializers, services, model backends,
platform/input/rendering/performance and remaining original scope stay required.

Next: port nullable source list element support through original shared collections,
model properties, factory algorithms and native metadata/consumer paths. Preserve
source null acceptance/failure/event/override order and shared collection identity.
Then continue major pending source families, including serialization and other model /
service implementations, rather than treating the working sample as completion.
Inventory unchanged232/454/1242 of1928; full completion remains correctly rejected.
User asked how close: explained these file-coverage figures (12% full,24% partial,64%
pending) are not time estimates; main Dock.Model134/175 full, working sample but broad
parity backlog. Full scope has not been reduced and no finish date was invented.

## Previous validated checkpoint

# Current: source nullable callbacks and original virtual arguments

Previous goal turn: PROGRESS (source callback/virtual migration and targeted tests).
Current: PROGRESS (full workspace evidence and corrected parity documentation).
The full original goal remains active/incomplete. Only FerroDock contents may change;
preserve original C#/sibling FerroUI/desktop sample. No agents, commits, push or PR.
Use one owned Cargo at a time, -j 1, and do not interrupt other chats' builds.

Owned Python validation process63077 and its sequential Cargo children are terminal;
process63077 is absent and all 17 stages in
`/Volumes/SSD/temp/ferrodock-port/ferrodock-null-virtual-validation-summary.json` are zero. Do not restart/resume
that completed process. The command was `/Volumes/SSD/temp/ferrodock-port/ferrodock-null-virtual-validate.py`;
all stage logs use that prefix. Workspace tests, strict workspace all-target and
release-library Clippy, standalone Core/recyclingSDK/backendSDK/runtime tests,
six normal dependency boundaries, formatting, 21 Python tests, source/theme/inspector
checks, both generators and five vendor integrity guards pass. The full-completion
gate correctly rejects the unfinished original scope.

Full workspace: **1,661 tests / 47 suites**, zero failed/ignored. Source models478,
Core61, consumers156, runtime3; see VALIDATION.md for scope and other counts.
C# reference: **530 assertions / 531-line TSV**, all pass; original source unchanged.
Targeted consumer5982 and source/Core/reference jobs are terminal0. The original
source-only no-Factory null/virtual cases now have corresponding Rust coverage.

DocumentFactory and locator callbacks share one exact Rc callable with Option result.
Native views retain that same Rc without adapters. The temporary runtime callback
bridge/cache is deleted. Domain remains independent of UI/Base/Runtime. CreateDocument
passes the nullable result through the original single AddDocument virtual slot;
document/tool Add methods and metadata preserve nullable runtime reference arguments.
Source base methods reread Factory for all three conditional calls. Null/no-Factory
is a no-op; enclosing null-aware overrides receive None. Nonnull conveniences/fluent
helpers dispatch the same slot, and derived base calls forward to base-body helpers.

IMPORTANT remaining gap: IFactory argument contracts are nonnullable. With a present
default factory the base body reports the source first null dereference before graph
mutation, but a custom factory override accepting null cannot receive it. Full nullable
factory/collection/CLR semantics remain partial. Do not claim all null/exception/override
parity or add UI-specific shortcuts. Other original source/API/platform scope remains
required; inventory still232/454/1242 of1928 and completion must remain rejected.

Disk cleanup before validation removed65 old unreferenced owned test executables,
17.19GiB reclaimed; sample binaries/bundles/window and logged artifacts were protected.
Evidence: /Volumes/SSD/temp/ferrodock-port/ferrodock-stale-test-candidates.json and
/Volumes/SSD/temp/ferrodock-port/ferrodock-stale-test-prune.json. No vendor/dependency changes this checkpoint.

Final bookkeeping: new files are intent-to-add, authored whitespace clean, staged
contents empty; original parent README/src/tests/samples unchanged. Actual desktop
sample executable/bundle/window preserved. No full platform/UI refresh claimed.

Next: continue faithful nullable IFactory contracts and original base implementation /
custom override dispatch, followed by nullable list and CLR behavior. Source contracts
currently reject null at the factory boundary before an override can see it; do not
mask that gap with a UI branch or claim the whole port complete. General delegate,
public attribute class, serializer, services and remaining original scope stay required.

## Previous validated checkpoint

# Current: shared callback cross-assignment and native identity

Previous goal turn: PROGRESS (native factories/attributes, 1,651 tests).
This checkpoint: PROGRESS. Full original goal remains active/incomplete; no blocker.
All owned validation processes are closed. Only FerroDock contents changed; original
C# and sibling FerroUI sources remain unchanged. A documentation command used the
parent directory by mistake; parent README content was verified unchanged and paths
were corrected to absolute paths. Preserve desktop sample/window: no desktop binary
rebuild/restart, agents, commits/push/PR or staged contents. Owned Cargo builds use
-j 1 and do not overlap; do not interrupt other chats' builds or restart on timeouts.

Shared ferrodock-runtime callback_bridge now adapts nullable/required Rust Fn forms
when Option denotes reference-nullability annotations. Do not use it to alias nullable
value types, whose CLR types differ. Typed weak adapter caches reuse live adapters;
reverse conversion returns the exact original typed Rc. Weak origin maps retain native
reference identity and native Func.Invoke null results through required Rust views.
No registry retains application callbacks strongly. Adapter-held guards retain only a
Weak registry, remove reverse entries on final drop, and check the exact adapter pointer
before removing source-key entries. No borrow spans callback invocation/destruction.
No unsafe, added dependencies, source-model changes or engine algorithm changes.

Core registers casts both ways between DocumentFactoryReference and the existing
DockableFactoryReference. Native identity/equality hooks use the original callback.
DocumentFactoryReference.from_delegate uses the validated nullable native view and
shared bridge. Canonical Func.Invoke preserves original null results; raw required
Rust invocation still reports a programmer contract violation for null results.
Original source callbacks roundtrip exactly through original model properties and
locator dictionaries. Native TwoWay object assignment retains callback identity after
rebind, invokes the original callback once, and updates the original docking graph.

Final locked workspace passes **1,661 Rust tests in 47 suites**, zero failed/ignored:
UI 372, markup 154, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 61, recycling SDK 2, backend SDK 7 and runtime 9. Six new runtime cases cover
both deferred/reused/exact typed Rc roundtrips, native null/required contract failure,
distinct identity, captured state release/cache cleanup, callback-body and capture-drop
reentry. Two new Core cases cover original property/map flow in both directions and
native null invocation/identity. Two downstream cases test emitted and interpreted
TwoWay object binding, source rebind identity and original engine invocation.

Original unchanged C# reference passes **525 assertions** (526-line TSV). Nine new
source assertions cover matching TwoWay/map/engine/null/identity behavior. Two more
SOURCE-ONLY assertions prove remaining null-engine behavior: without Factory,
AddDocument(null) is a no-op, and a subclass override receives null. These cases have
no Rust parity and must not be presented as matched. Reference job 86795 closed 0;
log /Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-null-engine-reference.log. Earlier reference job 99982
closed 0 (523 assertions); first script path error made no source change.

Strict all-target workspace and release-library Clippy pass. The first validation
requested private weak-callback aliases; the full workspace reran after that cleanup.
A subsequent lint removed a needless borrow in one Core test. Strict all-target Clippy
and all standalone SDK tests then pass after that test-only cleanup. Standalone Core /
recycling SDK / backend SDK / runtime pass 61+2+7+9. All six normal dependency boundaries,
formatting, 21 Python tests, both generators, source/theme/inspector checks and five
retained dependency integrity guards pass. No vendor/dependency patches changed.
Original source and staged contents are clean; new files are intent-to-add and authored
whitespace passes. Final documentation/source bookkeeping runs after edits.

Inventory remains **232 ported / 454 partial / 1242 pending** of 1928. Full completion
is correctly rejected. Cross-form callback assignment/roundtrip/native identity is now
implemented; document-factory/general delegate records stay partial for broader
System.Func/Delegate type metadata, Equals/multicast, CLR exception/cycle semantics.
The required Rust source signature cannot represent the original null-result engine
path, including no-Factory no-op and nullable override dispatch. Do not add a local
engine special case or substitute another engine. Original HostWindowState keeps its
strong readonly host reference. Public attribute class, full generic collections,
host out-parameters, services/command bars, serialization and remaining original
source/platform/input/rendering/performance parity also remain required.

Next: continue source nullable callback/argument/override semantics through faithful
shared system/model support, using the explicit C# null-engine assertions as reference.
Investigate broader generic delegate/type/Equals/multicast APIs and remaining model /
service / serializer source scope. The full goal is unchanged and not completed.

Owned final validation job 62205 closed 0; every stage in the merged summary is 0.
Workspace job 14389 completed its tests before the final test-only borrow cleanup.
Commands: /Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-validate.py and
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-finish-validation.py. Logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-standalone-ferrodock-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-standalone-ferrodock-controls-recycling-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-standalone-ferrodock-model-reactiveui-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-standalone-ferrodock-runtime.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-format.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-python-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-check_port.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-check_theme_links.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-check_inspector_views.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-check_framework_patch.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-factory-generator.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-preview-generator.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-completion.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-validation-summary.json.
Targeted consumer job 52349 closed 0; log /Volumes/SSD/temp/ferrodock-port/ferrodock-delegate-consumers.log.
Earlier failed validation handles are terminal/superseded; do not resume old handles.

## Previous validated checkpoint

# Current: native document factories and original template-required attributes

Previous goal turn: PROGRESS (Core Model.Controls ownership, 1,641 tests).
This checkpoint: PROGRESS. Full original goal remains active/incomplete; no blocker.
All owned validation processes are closed. Only FerroDock files are edited; original
C# and sibling FerroUI sources remain unchanged. Preserve desktop sample/window:
no desktop binary rebuild/restart, agents, commits/push/PR or staged contents.
Use -j 1 for owned Cargo operations and do not overlap owned builds or interrupt
other chats' builds. Observation timeouts are not terminal process states.

Core now owns 23 original Model.Controls descriptors, including IDocumentDockFactory.
The source DocumentFactory alias keeps the existing nonnullable-result Rc callable;
ReactiveUI re-exports it without changing source engine calls. New Core
DocumentFactoryReference retains the actual callback, identity, deferred invocation
and original result. It is another handle of the existing single native
System.Func<IDockable> descriptor, alongside the nullable-result locator view.
Canonical Invoke dispatches the actual source representation and returns a structured
error for invalid receivers. The original nullable property/checked contract casts
are Core-owned. The ReactiveUI class view exposes the original interface/property
and keeps the same enclosing source object. No engine or model mirror is introduced.

All fourteen original RequiresDataTemplate interface markers are existing native
custom-attribute data, with exact runtime/compiler-export tests. Eleven interface
files whose sole remaining gap was this metadata now have completed records.
The public marker class, System.Attribute base and AttributeUsage API remain pending.

Final locked workspace passes **1,651 Rust tests in 47 suites**, zero failed/ignored:
UI 372, markup 152, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 59, recycling SDK 2, backend SDK 7 and runtime 3. Six new Core cases cover
original callback property/checked membership/identity/null/deferred invocation,
canonical shared Func dispatch and invalid receivers, original CreateDocument graph,
capture release, all 14 original template markers and known invalid method signatures.
One backend test checks original class/interface/callback identity without general UI.
Three downstream cases cover native method assignment, class construction, Core Func
invocation, getter/setter sharing and original docking graph through compiled/runtime
paths. Runtime fixtures register their application assembly and use standard source
casts for root values, rather than assuming one Rust boxing form.

Unchanged original C# reference passes **514 assertions** (515-line TSV), including
native method assignment, compiled Core Func binding, original graph identity and
explicit CLR cross-nullability delegate assignment/identity/invocation. C# job 34171
closed 0; log /Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-reference.log. Earlier failed Rust attempts
are terminal and superseded: missing fixture registration / direct root downcast were
fixed in the consumer. Isolated runtime job 45499 closed 0; final workspace covers all
three consumers. Core's new generic metadata test originally named the wrong field;
final tests use the actual definition field.

Strict all-target workspace and release-library Clippy pass. Standalone Core,
recycling SDK, backend SDK and runtime pass 59+2+7+3. All six normal dependency boundaries,
formatting, 21 Python tests, both generators, source/theme/inspector checks and five
retained dependency integrity guards pass. No vendor/dependency patches changed.
Original source and staged contents are clean; new files are intent-to-add and authored
whitespace passes. Final documentation/source bookkeeping is checked after edits.

Inventory reports **232 ported / 454 partial / 1242 pending** of 1928. Full completion
is correctly rejected. IDocumentDockFactory remains partial: CLR nullable annotations
do not change delegate type or reference identity. A shared identity-preserving bridge
between the two Rust callable representations is still required. Own-representation
views and one native descriptor do not establish cross-assignment or general delegate
parity. Do not substitute wrapped callbacks and claim original Rc identity. The C#
cross-nullability assertions are authoritative expected behavior for the next work.

Next: inspect shared system/runtime delegate support and implement native cross-form
assignment/roundtrip/identity/null-return/lifetime semantics while keeping source
engine calls intact. Both source callable types are Rc<dyn Fn()> with different Rust
return types, so no raw Rust coercion can equate them. Native metadata now recognizes
both views; conversion/identity must be solved in shared interop, not by another logical
Func type or an application-specific mirror. Any adapter/weak-origin mechanism must
avoid strong retention, pointer-reuse bugs and reentrant-drop borrows, and preserve
original typed callbacks on roundtrip. Also continue public attribute/model/service/
command-bar/serialization source scope, full generic collections, host out-parameters,
CLR cycles/lifetimes and original platform/input/rendering/performance parity.
Do not weaken the original HostWindowState strong readonly host reference.

Owned validation job 65941 closed 0; every stage in its summary is 0. Commands live in
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-validate.py. Logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-standalone-ferrodock-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-standalone-ferrodock-controls-recycling-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-standalone-ferrodock-model-reactiveui-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-standalone-ferrodock-runtime.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-format.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-python-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-check_port.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-check_theme_links.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-check_inspector_views.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-check_framework_patch.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-factory-generator.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-preview-generator.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-completion.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-native-factory-validation-summary.json.

## Previous validated checkpoint

# Current: original Model.Controls ownership in the Core SDK

Previous goal turn: PROGRESS (Core target/container/host metadata, 1,628 tests).
This checkpoint: PROGRESS. Full faithful-port goal remains active/incomplete; no blocker.
All owned validation processes are closed. Only FerroDock files are edited; original
C# and sibling FerroUI sources remain unchanged. Preserve the desktop sample/window:
no desktop binary rebuild/restart, agents, commits/push/PR or staged contents.
Use -j 1 for owned Cargo operations and do not overlap owned builds or interrupt
other chats' builds. An observation timeout is not a terminal process state.

Core owns 22 original Model.Controls contracts under logical FerroDock.Model.
Twenty descriptors moved from UI, which retains public aliases to the same
original declarations and UI-only projection registrations. New IToolTemplate and
IToolDockContent descriptors dispatch original source contracts and arbitrary-object
content through the existing shared runtime. IToolTemplate has the default optional
source-INPC discovery hook already used by IDocumentTemplate. No docking engine or
model mirror is introduced. Root Windows retains the original IList<IDockWindow>
handle, supported Count/indexer/mutators, source notifications and reference identity.

Corrected IMdiDocument metadata to inherit original IDocument; IDocumentDock/IToolDock
now declare original ILocalTarget membership. All 18 inspector property views and the
main inspector resolve Model.Controls from original FerroDock.Model, preserving
original layout/binding contracts through the source migration guard.

The first consumer build rejected unresolved native cast facts introduced by wildcard
imports in the new registration helpers. Explicit source-trait imports fix the facts:
Core has no unread registrations and UI has no unread casts. No compiler completeness
checks or vendor patches were changed. The new compiler-export regression initially
needed an Option-field type correction; final validation includes the corrected test.

Final locked workspace passes **1,641 Rust tests in 47 suites**, zero failed/ignored:
UI 372, markup 149, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 53, recycling SDK 2, backend SDK 6 and runtime 3. Six new standalone cases
cover original contract ownership/handles/notifiers, MDI inheritance/marker identity,
tool-template arbitrary values/null/events, optional tool-content checked dispatch,
shared root-window list operations/events and complete native scanner exports.
Seven independent cases cover root windows/pins, document/tool Add command bindings
and original graph ownership/activation, inherited MDI title/state, nullable grid
TwoWay definitions, stack values and both runtime compiler paths. The unchanged C#
reference passes **502 assertions** (503-line TSV); six new source assets explicitly
enable compiled bindings. Standalone Core/recycling/backend/runtime pass 53+2+6+3.

Strict all-target workspace and release-library Clippy pass. Six normal dependency
boundaries pass: Core/recycling SDK/runtime exclude UI controls; general Dock UI
excludes the concrete backend/SDK; backend SDK excludes general UI/runtime compiler;
Domain excludes native base/controls. Backend SDK retains controls through recycling.
Formatting, 21 Python tests, both generators, source/theme/inspector guards and all
five retained dependency integrity guards pass. No dependency patches changed.

Inventory reports **221 ported / 465 partial / 1242 pending** of 1928. The reduction
in ported count corrects IDocumentDockFactory: source getter/setter and engine exist,
but native DocumentFactory delegate metadata and canonical generic-delegate identity
remain unexposed. Do not substitute its nonoptional-result Rust callable with the
optional-result locator factory delegate. RequiresDataTemplate attributes, full
IList generic/CopyTo/enumerator metadata, original host out-parameter metadata,
serialization, general CLR identity/cycles/lifetimes and remaining source/platform/
input/rendering/performance scope remain required. The completion gate correctly
rejects the unfinished original scope; do not mark the goal complete.
Original HostWindowState retains its strong readonly host reference; do not weaken it.

Owned validation job 26479 closed 0; all stages in the machine-readable summary are 0.
Logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-standalone-ferrodock-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-standalone-ferrodock-controls-recycling-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-standalone-ferrodock-model-reactiveui-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-standalone-ferrodock-runtime.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-format.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-python-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-check_port.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-check_theme_links.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-check_inspector_views.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-check_framework_patch.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-factory-generator.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-preview-generator.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-completion.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-controls-validation-summary.json.
C# reference job 52841 closed 0; log
/Volumes/SSD/temp/ferrodock-port/ferrodock-model-controls-reference-compiled.log. Independent consumer job 28073
closed 0; log /Volumes/SSD/temp/ferrodock-port/ferrodock-model-controls-consumers-v2.log. Earlier failed attempts
are terminal and superseded by final validation; do not restart live processes.

Next: continue original source/API scope with native DocumentFactory/delegate and
RequiresDataTemplate metadata, remaining model/service/command-bar contracts and
serialization, preserving original engine and logical assembly ownership. Prior
validated checkpoint details below remain useful context, not full completion proof.

## Previous validated checkpoint

# Current: Core target/container ownership and original host-window metadata

Previous goal turn: PROGRESS (Core dock contracts/recycling SDK, 1,616 tests).
This checkpoint: PROGRESS. Full faithful-port goal remains active/incomplete; no blocker.
All owned validation processes are closed. No original C# or sibling-source edits,
desktop binary rebuild/restart, commits/push/PR were performed. Other chats may have
independent builds; do not interrupt them. Use-j1 for subsequent owned Cargo operations.

Core owns ILocalTarget, IGlobalTarget and IDockItemContainerMetadata descriptors under
original FerroDock.Model. UI keeps aliases to the same declarations and managed/native
projection augmentations. Core owns source checked marker casts and reference identities.
Container arbitrary-object/null properties use the existing shared runtime conversion.
Domain and original docking/window engine algorithms are unchanged.

New IHostWindow native descriptor exposes original HostWindowState, IsTracked and Window
plus Present/Exit/SetPosition/SetSize/SetWindowState/GetWindowState/SetTitle/SetLayout/
SetActive, all dispatching existing source implementations. IHostWindowState and inherited
IDockManagerState markers have Core metadata, original upcast and identity facts.
Original GetPosition/GetSize native out-parameter metadata remains unexposed; Rust trait
adaptations are unchanged. Do not present a different native signature as source parity.

Native host bindings reproduced shared WeakValue restoration failure for interface
handles. Isolated published0.3.0 base ValueTypes.from_object now reuses its existing
class-declared interface conversion helper, including inherited and nullable views.
No cast/object retention table is added. Compiled/runtime TwoWay container binding also
reproduced leaked Option<BoxedValue> transport wrappers. Shared markup
object_assignment_value normalizes registered nullable/object boxes and canonicalizes
controls; emitted DeclaredAccessors and runtime RuntimePropertyAccessors use it only
for System.Object setters. Runtime caches this declared type fact. No special container
workaround exists. Base/markup/runtime-loader patches have updated exact provenance
against verified untouched published sources and successful patch dry-runs; other two
patches and external versions are unchanged. Five dependency integrity guards pass.

Final locked workspace passes **1,628 Rust tests in 47 suites**, zero failed/ignored:
UI372, markup142, diagnostics49, settings33, sample27, ReactiveUI model474, Core SDK47,
recycling SDK2, backend SDK6, runtime3. Five new standalone Core cases cover source
marker membership/enclosing identity, container arbitrary values/nulls, host signatures/
nullable references, original interface argument ordering/results and method dispatch.
Seven independent consumer cases cover original native host geometry/layout/state/
presentation/exit, Core-qualified live model title and TwoWay tracking/command bindings,
container raw Control identity and TwoWay string/null setters, both compiler hosts,
weak host/nullable restoration, inherited brush restoration/collection, and object
setter null/nested boxes/nullable string/scalar/control normalization.

Unchanged original C# reference passes **470 assertions** (471-line TSV), including
28 new native host/container/marker cases and equivalent compiled source assets.
Standalone Core/recycling/backend/runtime pass47+2+6+3. Strict all-target workspace
Clippy and release runtime/recycling SDK/UI/Core/backend/UI/diagnostics/markup/sample
library Clippy pass. Normal trees confirm Core/recycling SDK/runtime exclude UI controls,
general Dock controls exclude concrete backend/SDK, backend SDK excludes general Dock
UI/runtime compiler, and Domain excludes native base/controls. Backend SDK retains
native controls through recycling. Formatting, 20 Python tests, both generators,
source/theme/inspector checks, source inventory and five dependency guards pass.
Completion correctly rejects222ported/464partial/1242pending of1928. Original tracked
source is clean; authored whitespace passes. New files are intent-to-add, with no staged
contents. Final bookkeeping checks pass after documentation edits.

Owned validation job22791 closed0; machine-readable summary has zero for every stage.
Current logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-reference.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-consumers.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-standalone-ferrodock-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-standalone-ferrodock-controls-recycling-model-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-standalone-ferrodock-model-reactiveui-ferro.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-standalone-ferrodock-runtime.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-dependencies.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-format.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-verifiers.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-whitespace.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-validation-summary.json, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-host-completion.log.

An earlier build was explicitly interrupted for verified memory pressure (24GiB RAM,
16GiB swap); only owned group34137 was stopped, handle18599 closed130 and all children
were confirmed gone before restarting-j1. A later app/tool interruption cleared /tmp;
old handle33244 and old process were verified missing before revalidation. Current
workspace/source reference and all final checks completed after that interruption.
Do not restart a live process solely because an observation timed out.

Next: continue original full objective with remaining model-controls/service/command-bar
metadata and original logical assembly/API ownership, plus generic/default/out/collection
metadata and serialization. Original HostWindowState keeps its strong readonly host
reference; do not weaken it or claim general CLR cycle collection. Broad CLR identity/
lifetimes, platform/input/rendering/performance parity and unported original sources
remain required. The completion gate is structural evidence of remaining work, not a
behavioral proof for records already marked ported. Do not mark the full goal complete.

## Previous validated checkpoint

# Current: Core dock contracts and recycling Model SDK

Previous FerroDock goal turn: PROGRESS (Core extraction and initial SDK tests).
This turn: PROGRESS (Core registration, original recycling interface API, independent
consumers/source reference and full validation). Full faithful-port goal remains active
and incomplete; no blocker exists. Preserve the running desktop sample/bundle/window/
layout. This checkpoint did not edit original C# or sibling FerroUI source, change
vendor patches, rebuild/restart the desktop sample, commit, push or create a PR.

Core owns IDockable, IDock and typed-list metadata under original FerroDock.Model;
UI aliases retain the same declarations. Original IDockable-to-IDock checked conversion
works without controls. List metadata forwards original mutations and INPC, and uses
source ListHandle.identity for separately boxed shared reference identity. No Domain
or ReactiveUI engine algorithm changed. Core now directly names the UI-independent
ReactiveUI command contract used by original IDock getters.

New FerroDock.Controls.Recycling.Model.Ferro owns the original recycling Model logical
assembly and source contract metadata. It exposes GetControlRecyclingId, TryToUseIdAsKey,
Add/Build/Clear through actual interface implementations and the shared runtime bridge.
TryGetValue bool/out metadata remains absent because the shared metadata cannot express
it; no alternative signature is passed off as that API. Core Context also uses the
shared runtime bridge. UI adds the existing raw Ref<Control> type fact, retaining raw
Control type/identity without importing controls into Core or copying an object table.

The main inspector and three property views now reference migrated Core contracts
through original Core assembly ownership. Source layout, binding and command behavior
are unchanged, guarded by exact source comparison and 20 Python regressions.

Final locked workspace passes **1,616 Rust tests in 47 suites**, zero failed/ignored:
UI 372, markup 135, diagnostics 49, settings 33, sample 27, ReactiveUI model 474,
Core SDK 42, recycling SDK 2, backend SDK 6 and runtime 3. Three dockable tests moved
from UI to Core. Three new Core cases test ownership/source casts, actual shared lists/
notifications/boxed identity and original command identity/navigation. Two recycling
SDK cases test interface forwarding and nullable IDs. Four downstream cases exercise
compiled Core interfaces, live inherited/count bindings, TwoWay fields, original native
command execution, raw Control Context/cache identity and both compiler hosts. Runtime
views are loaded with compiled bindings enabled and verified against live original models.

Original unchanged C# assemblies pass **442 assertions** (443-line TSV), including
18 new Core-contract/cache/reference cases and equivalent compiled assets. Standalone
Core/recycling/backend/runtime tests pass (42+2+6+3). Strict workspace all-target Clippy
and release runtime/recycling SDK/UI/Core/backend/UI/diagnostics/markup/sample library
Clippy pass. Normal trees prove Core/recycling SDK/runtime exclude UI controls, general
Dock controls exclude the concrete backend/SDK, backend SDK excludes general Dock UI/
runtime compiler and Domain excludes native base/controls. Backend retains native
controls through recycling. No external dependency version changed.

All 20 Python tests, both generators, inventory/theme/inspector checks and all five
vendor guards pass. Completion correctly rejects the unfinished inventory: 222 ported,
464 partial, 1242 pending of 1928. Authored whitespace and original-source checks pass;
new files are intent-to-add with no staged contents. Formatting also passes.

Logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-reference.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-standalone-sdk.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-verifiers.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-format.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-whitespace.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-contracts-completion.log.

Next: remaining Core metadata still owned by controls includes ILocalTarget,
IGlobalTarget and IDockItemContainerMetadata; move source-only contracts/identity facts
while retaining UI managed/native projection. IHostWindow and other broader contract
metadata, model-controls ownership, generic/default/out/collection metadata, serialization,
complete CLR identity/lifetimes and native platform parity remain required. Continue the
original goal; do not mark complete from this checkpoint. No owned Cargo process remains
after the final format check closes; an unrelated sibling-host Cargo may be active.

## Previous validated checkpoint

# Current: Core capability and window metadata ownership

Previous goal turn: PROGRESS (factory operations/lazy search, fully validated).
This turn: PROGRESS. Full faithful-port goal remains active and incomplete. Preserve
running desktop sample/bundle/window/layout. No original source or sibling FerroUI
edits, desktop restart/rebuild, commits/push/PR are part of this work.

Moved IDockCapabilityPolicy, DockCapabilityPolicy, DockCapabilityOverrides and
IDockWindow metadata from general controls into Core. UI retains aliases to the same
declarations, with old registration/path ownership removed. One unchanged window
metadata test moved into Core. Core capability casts and actual public model value
paths now register independently of controls.

Added original DockWindowOptions constructor/properties/ApplyTo (including null
failure), immutable DockCapabilityEvaluation eight-argument constructor/properties,
DockCapabilityResolver static methods and DockCapabilityValueSource as the 23rd
Core enum descriptor through the existing declaration-based generator. Domain adds
only a hidden declaration string; no engine algorithms or dependencies change.
Original optional IsEnabled parameter uses the existing arity-adapter approach;
full optional/generic/CLR enum/string/serialization/lifetime/platform scope remains open.

Six new Core tests check ownership/export, policy constructors/inheritance/nullable
values/shared identity, options taskbar/parent/apply behavior, immutable result tuples,
resolver context/precedence and custom policy Get virtual dispatch. Four new downstream
cases compile Core-qualified policy/options/evaluation roots and bindings, verify live
parent-window notifications, TwoWay edits and actual engine use, and exercise the
runtime compiler. That host now has explicit actual Rc<model> value paths for the two
new external constructors, matching the existing policy approach.

Inspector RootDockDebug and DockWindowDebugView now qualify only IDockWindow through
the original Core assembly. Other contracts stay at their current owners. The exact
source-comparison guard declares that namespace migration, and two new regressions
reject restoring the old owner or changing unrelated markup. All 20 Python tests pass.
Layouts, bindings and behavior are preserved; the running sample was not rebuilt.

Compiled consumer tests pass (4 cases), log /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-consumers.log.
The unchanged C# reference succeeds with 424 assertions, log
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-reference.log. Reference source and six assets are under
scripts/dock/factory-markup-reference. The final locked workspace passes **1,607 Rust tests in 45 suites**, zero failed/ignored:
native UI375, markup131, diagnostics49, settings33, sample27, ReactiveUI model474,
Core SDK36, backend SDK6 and runtime3. Strict all-target workspace Clippy and release
runtime/recycling/Core/backend/UI/diagnostics/markup/sample library Clippy pass.
All 20 Python tests, both generators, source/theme/inspector checks and all five vendor
guards pass. Completion correctly rejects the unfinished inventory.

Standalone Core/backend/runtime suites pass (36+6+3) without general Dock controls
or runtime compiler metadata. Normal dependency trees confirm Core/runtime exclude
native controls, general Dock controls exclude the concrete backend/SDK, backend SDK
excludes general Dock controls/runtime compiler, and Domain excludes native base/
controls. The backend SDK retains native controls through recycling. Formatting and
authored whitespace checks pass. New files are intent-to-add, with no staged contents,
commits or original tracked source changes.

Current logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-consumers.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-reference.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-standalone-sdk.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-python.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-verifiers.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-format.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-core-policies-completion.log.

Next ownership work includes IDockable/IDock and the remaining model contracts still
registered by controls, preserving their object/recycling bridges and source assembly
identities. Generic/optional/collection and wider CLR/serialization/platform scope
remain required by the original goal.
Inventory remains 222 ported, 464 partial, 1242 pending /1928. Do not mark full goal complete.

## Previous factory API checkpoint

# Current: factory operations and lazy search metadata

Previous goal turn: PROGRESS (factory API implementation and unchanged source probes).
This turn: PROGRESS (completed workspace validation and corrected interface metadata).
The full faithful port remains active and incomplete; no blocker exists. Preserve the
running desktop sample, bundle, window and live layout. No original C# or sibling
FerroUI edits, desktop rebuild/restart, commits/push/PR are part of this checkpoint.

Core IFactory and FactoryBase publish original collection mutation, container lookup,
window initialization/list/creation/splitting, pin lookup/reordering, collapse, split
layout and activation APIs. FactoryBase also exposes the original command overloads
already present on IFactory. Calls retain source arguments/results and enclosing
virtual dispatch. A structural comparison finds all 77 nongeneric method name/arity
pairs declared in original IFactory.cs on both descriptors; this does not prove full
parameter/default/generic/CLR API parity. Original optional arguments currently use
explicit arity adapters. General optional metadata and generic CreateList/GetDockable
remain incomplete.

factory_search.rs retains actual source Func predicates and DockableQuery. Checked
MarkupDelegate conversion validates input assignability and Boolean returns without
invocation. Query/cursor metadata has original interface kind; independent cursors
use the original lazy iterators, retain duplicate nested results and reread trees on
later enumerations. Current/MoveNext/unsupported Reset and explicit native IDisposable
preserve the tested end/fault/disposal behavior. General CLR iterator object identity,
non-generic collection contracts, interface variance, GC/lifetimes remain partial.
No replacement traversal, snapshots or second docking engine is introduced.

Eight new Core tests cover original production graphs and Base command behavior,
20 newly exported virtual overloads through both factory views, predicate validation/
variance, query/cursor behavior, pin order, ownership and activation tracking. Two new
compiled consumer cases cover original predicate declarations and a Core IFactory
ActivateWindow command binding. Both compiler hosts compile the new assets. The
source window fixture keeps owner and floating root separate: an accidental self-
containing layout recursed in unchanged C# and Rust and was corrected without any
engine change.

Final locked workspace: **1,597 Rust tests in 45 suites**, zero failed/ignored.
Native UI376, markup127, diagnostics49, settings33, sample27, ReactiveUI model474,
Core SDK29, backend SDK6 and runtime3. Unchanged C# reference: **275 assertions**.
Strict all-target workspace Clippy and release runtime/recycling/Core/backend/UI/
markup/sample library Clippy pass. All 18 Python verifier tests, both generators,
source/theme/inspector guards and all five exact vendor integrity guards pass.
There are no new dependencies or vendor modifications in this checkpoint.
Standalone Core/backend/runtime suites pass (29+6+3) without the general Dock control
library or runtime compiler. Normal dependency trees confirm Core/runtime exclude
native controls, general Dock controls exclude the concrete backend/SDK, the backend
SDK excludes general Dock controls/runtime compiler, and Domain excludes native base/
controls. The backend SDK retains native controls through its recycling integration.
Formatting and authored whitespace checks pass. New files are intent-to-add for review;
there are no staged contents, commits or original tracked source changes.

Current logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-core.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-reference.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-verifiers.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-signatures.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-completion.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-standalone-sdk.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-dependencies.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-api-format.log.

Remaining: complete Core contract/POCO logical ownership, generic/optional method and
collection/x:Class/name/root metadata, other backends/services/serializers, full CLR
EventArgs/delegate/boxing/custom identity/string/GC/lifetime and native platform parity.
Inventory remains 222 ported, 464 partial, 1242 pending /1928; completion must still reject.
Do not mark the full goal complete.

## Previous locator checkpoint

# Current: Core factory ownership and original locator APIs

Previous goal turn: PROGRESS (factory events and shared object interop).
This turn: PROGRESS. The full faithful-port goal remains active and incomplete.
Preserve the running desktop sample, bundle, window and live layout. No blocker exists.

The original IFactory and FactoryExtensions descriptors now belong to the Core SDK
and logical FerroDock.Model assembly. General controls retain legacy Rust aliases to
the same descriptors and explicitly register Core; they no longer export duplicate
factory types or overwrite Core public paths. Five command/helper tests moved with
their owning descriptors and run without controls. Source interface upcasts and
nullability are ordinary Core metadata facts over actual model references.

FactoryBase and IFactory now expose their source tracking fields, all five locator
properties, original GetContext/GetHostWindow, creators, initialization and existing
commands/events. Locator views retain the actual stored Rc<dyn Fn> and mutable source
dictionaries. Native method delegates are checked before assignment and remain deferred
until invoked. Dictionary constructors, Count, indexers, Add/ContainsKey/Remove/Clear
use those same containers. Errors preserve duplicate/missing-key behavior; replacement,
removal and clear release captured values outside RefCell borrows. Full collection,
CLR delegate Method/Target/equality and generic method metadata remain incomplete.

The domain locator aliases now represent nullable delegate entries explicitly.
Registered null delegates and callbacks returning null both return null without taking
an absent-key fallback. Existing callers/sample/tests wrap their actual functions in
Some; the functions and docking algorithms are preserved. The virtual default host
getter can change to null between its two source reads during InitLayout: the second
value is now stored as a null dictionary entry, matching the unchanged C# code instead
of panicking. The compiled subclass test uses the real Factory component and source
virtual dispatch, with no second engine. It explicitly clears callbacks after testing.

The original declarations reproduced a build/runtime metadata mismatch: runtime knew
checked shared-box casts while the shared scanner omitted them. A fifth exact published
0.3.0 package, ferroui-build-scan, is isolated under vendor. Two files record boxed_cast
as the same cast-pair facts as register_cast, normalizing read/unread counts in normal
code, declaration tokens and macro expansion. Runtime predicates still run through the
existing cast path. Two scanner regressions pass. Backend class projection logic remains
generic; its concrete closed registration facts are now visible to the scanner. No
sibling FerroUI or original C# source changes are needed.

Core tests cover delegate/container identity, deferred calls, null/missing behavior,
original dictionary mutation/failures, safe release and transient virtual getters.
Independent compiled consumers cover original delegate declarations, Core-qualified
bindings and both compiler hosts. The original C# reference includes matching cases.
The final locked workspace passes **1,587 Rust tests in 45 suites**, zero failed or
ignored: native UI376, downstream markup125, diagnostics49, settings33, sample27,
ReactiveUI model474, Core SDK21, backend SDK6 and runtime3. The unchanged C# reference
passes 236 assertions. Strict all-target workspace Clippy and release runtime/recycling/
Core/backend/UI/markup/sample library Clippy pass. All 18 Python tests, both generators,
source/theme/inspector guards and all five exact dependency provenance guards pass.
Standalone Core/backend/runtime tests pass (21+6+3) without the general Dock control
library or runtime compiler. Normal dependency trees confirm Core/runtime exclude
native controls, general Dock controls exclude the concrete backend/SDK, the backend
SDK excludes general Dock controls/runtime compiler, and Domain excludes native base/
controls. The backend SDK intentionally retains native controls through recycling.
Formatting and authored whitespace checks pass. New files are intent-to-add for
review, with no staged contents, commits or changes to original source trees.
The structural completion gate still rejects the unfinished inventory as expected.

Current logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-locators-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-locators-reference.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-locators-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-locators-release-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-locators-core-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-locators-scanner-tests.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-locators-verifiers.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-locators-preview-generator.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-locators-format.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-locators-standalone-sdk.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-locators-dependencies.log and
/Volumes/SSD/temp/ferrodock-port/ferrodock-locators-completion.log.

Remaining: complete Core contract/POCO ownership and IFactory members, generic method/
collection/x:Class/name/root metadata, other backends/services/serializers, complete
CLR EventArgs/delegate/boxing/custom identity/string/GC/lifetime and native platform
parity. Inventory remains 222 ported, 464 partial, 1242 pending /1928. Completion must
still reject this unfinished port; do not mark the full goal complete.

## Previous event checkpoint

# Current: factory event metadata and shared object interop

Previous goal turn: PROGRESS (Factory/DockWindow constructors). This turn: PROGRESS.
The original full faithful-port goal remains active and incomplete. No implementation
blocker exists. Keep the running desktop sample, bundle, window and layout intact.

FactoryBase and the current IFactory descriptor now publish all 28 original factory
events and every public On callback overload, through the original engine. Core SDK
adapters supply the actual factory sender, the same shared source arguments and
explicit disposable subscriptions. Metadata add accessors follow the CLR void-return
contract. Dropping a native subscription handle does not silently unsubscribe; an
explicit Dispose does. General metadata remove accessors remain a native framework
limitation, and arbitrary CLR delegate identity/GC parity is not established.

The Core SDK owns metadata for all 29 original event-argument classes and the weak
GlobalDockTrackingState class: actual constructors, all declared properties, mutable
Cancel/Context, Empty and source inner reference identity. Domain only adds immutable
identity-address hooks, without UI metadata dependencies. The original tracking reason
enum is the 22nd Core enum/flags descriptor and uses the established declaration-driven
generator. Full CLR EventArgs base casts remain outside this checkpoint.

The existing object-value bridge is extracted into ferrodock-runtime, a native base-
only support library. Recycling keeps its raw Control projection and existing helper
names; Core metadata and recycling share one representation and type registry. No
second object conversion engine or per-object table is added. Source dictionary,
factory, event and docking algorithms remain unchanged.

Core tests cover every source event, sender/payload identity, explicit disposal,
ignored handles, publisher release, cancellation/context changes and all argument
constructors/properties. Independent compiled consumers construct original argument/
snapshot roots and wire a handler that cancels an actual factory close; both compiler
hosts emit public source paths. The consumer's own assembly is explicitly registered,
without changing its existing resource assembly identity. A compiled binding checks
all ten tracking reasons on actual source event arguments.

Final locked workspace: **1,578 Rust tests in 45 suites**, zero failed/ignored.
Native UI381, downstream markup122, diagnostics49, settings33, sample27, ReactiveUI
model474, Core SDK10, backend SDK6 and runtime3. The unchanged C# reference passes
214 assertions. Source explicit-root activation is additionally checked: subsequent
focus restoration rereads Root.Window and may replace the initially tracked window.
Both implementations preserve that source ordering without engine changes.
Strict all-target workspace Clippy and release runtime/recycling/Core/backend/UI/
markup/sample library Clippy pass. Formatting, all 18 Python verifier tests, both
generators, inventory/theme/inspector checks and all four vendor guards pass. Standalone
Core/backend/runtime tests pass (10+6+3) without general controls or compiler metadata.
Normal dependency trees confirm Core/runtime exclude controls, UI excludes concrete
backend/SDK, SDK excludes general controls/runtime compiler and Domain excludes native
base/controls. The compiled event-owner test additionally clears its own retained
factory/sender references and verifies that both owner and publisher release. This
explicit teardown does not establish general CLR cycle collection.

Current logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-consumers.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-reference.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-standalone-sdk.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-python.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-completion.log and
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-events-{ui,core,sdk,runtime,domain}-dependencies.log.
The completion gate correctly rejects the unchanged unfinished inventory. No build
cache cleanup was needed in this checkpoint.

Original C# and sibling FerroUI sources are untouched; no app rebuild/restart,
commits, pushes or staged contents are part of this checkpoint.

Remaining scope includes moving the current IFactory descriptor to its complete
Core logical ownership, factory locators, full collection/generic metadata, remaining
FactoryBase operations, complete Core interface/POCO logical ownership, other
backends/services/serializers, all x:Class/name/root cases and full API/platform/CLR
boxing/custom identity/string/GC/lifetime parity. Inventory remains 222 ported,
464 partial and 1242 pending /1928. Completion must still reject this unfinished port.

## Previous constructor checkpoint

# Current: Factory and DockWindow class metadata

This turn: PROGRESS. The original full faithful-port goal remains active and
incomplete. No implementation blocker exists. Preserve the running desktop sample,
bundle, window and rearranged layout; this checkpoint changes libraries and tests.

The ReactiveUI integration SDK now exports Factory and DockWindow constructors in
the original logical assembly, alongside all 14 backend Controls classes. They
create the actual original models. Factory inherits a constructor-free FactoryBase
view owned by the Core SDK; DockWindow retains ReactiveBase and notifications.
Concrete, class, base and interface handles share their original enclosing identity.
Hidden static boxed-reference hooks on IFactory/IDockWindow permit safe enclosing
projections without UI dependencies in Domain or a per-object table. A composed
window test preserves source override callbacks and releases the original instance.

Factory exports its ten original registry overrides and sixteen nongeneric creation
methods. FactoryBase metadata exports both close-policy properties, the same
registries, original creators and selected initialization/activation/close methods.
All operations dispatch through the existing engine. Dictionary views retain their
actual mutable containers and expose Count; source observable lists forward live
Count notifications. Full dictionary/indexer/generic metadata, factory locators,
events, generic CreateList<T> and remaining FactoryBase methods are still partial.
Do not describe this as a complete FactoryBase or collection metadata port.

Five compiled assets and seven consumer tests cover constructors, all declared
window properties/method descriptors, actual concrete model creations/defaults,
source inheritance and shared identity, read-only registry properties, original
layout initialization/close, compiled live TwoWay enum/numeric/bool/title updates,
HostWindows Count mutation and factory/parent/owner/layout references. Composition
resources explicitly own graph objects. Original InitDockWindow sets Factory and
Owner but does not set Owner.Window; both reference implementations assert that.
These fixtures do not establish CLR/GC lifetime parity for weak backlink graphs.

The runtime compiler exposed missing public Rust paths on notification-bearing
copies of interface descriptors. All 23 existing UI descriptor copies, plus the two
new Core observable-list descriptors, now register the actual descriptor pointer's
existing public path through native compiler metadata. Both compiler hosts use the
same original registry and engine; no new vendor/compiler change was needed.
Core SDK now uses native System notification services, still without controls.
General controls retain no normal concrete ReactiveUI backend/SDK dependency.

The full locked workspace passes **1,567 Rust tests in 43 suites**, with zero
failed/ignored. Native UI381, downstream markup118, diagnostics49, settings33,
sample27, ReactiveUI model474, Core SDK6 and backend SDK6. Strict all-target
workspace Clippy and release Core/backend/control/markup/sample library Clippy pass. All 18 Python verifier tests, both generators, inventory/theme/
inspector checks and all four vendor provenance guards pass. The completion gate
correctly rejects the unchanged unfinished inventory. Formatting passes. Both SDKs
additionally pass standalone (6+6), without general controls or compiler metadata.
Normal dependency trees verify UI excludes concrete backend/SDK, Core excludes
controls, backend SDK excludes general controls/runtime compiler, and Domain
excludes native base/controls.
The unchanged original C# reference passes 111 assertions. Original C# and sibling
FerroUI source files remain untouched. No desktop sample rebuild/restart, commits
or pushes are part of this checkpoint. New files are intent-to-add for review only.

Storage pressure was resolved only after authoritative build termination. Selective
records: /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-cache-cleanup.json and
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-obsolete-tests-cleanup.json. The latter removed 61
obsolete duplicate test executables (19.35 GiB), retaining the newest two per target
and excluding sample binaries. No source, app bundle or active build was removed.
Cargo operations remain sequential.

Current logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-consumers.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-standalone-sdk.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-reference.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-python.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-completion.log and
/Volumes/SSD/temp/ferrodock-port/ferrodock-factory-window-{ui,core,sdk,domain}-dependencies.log.

Next scope: remaining FactoryBase/collection metadata and Core interface/POCO logical
ownership; every generic/nullable/x:Class/name/root case; other backends/services/
serializers; full source/API/native platform/custom identity/string/boxing/CLR GC
and lifetime parity. Inventory remains 222 ported, 464 partial and 1242 pending
/1928. Completion must still reject it; do not mark the full goal complete.

## Previous Controls checkpoint

# Current: Controls class metadata and model enum ownership

Previous goal turn: PROGRESS (separate ReactiveUI constructor SDK).
This turn: PROGRESS. The original full faithful-port goal remains active and incomplete.
No implementation blocker exists; completion must still reject the unchanged inventory.

All 14 concrete classes in src/Dock.Model.ReactiveUI/Controls now have ordinary native
constructor/class metadata. The ten new views are ToolDock, ProportionalDock, StackDock,
GridDock, DockDock, WrapDock, UniformGridDock, SplitViewDock, ProportionalDockSplitter
and GridDockSplitter, in src/Dock/FerroDock.Model.ReactiveUI.Ferro/reactive_metadata.
Their create/new/model/base APIs retain actual enclosing engine objects, original
base-class views, source properties, original methods and notifications. They do not
copy state or supply another engine. Source constructors, defaults, all new declared
properties/methods and shared class/interface/base identity are covered. The normal
source factory consumes loaded objects; ToolDock.AddTool preserves owner/active/focus.
SplitView pane methods, nested actual pane/content objects, nullable identity and
compiled enum/bool/numeric bindings use the same original models. Empty-model
retention/release tests cover all ten new class views; broader CLR GC scope remains open.

The new src/Dock/FerroDock.Model.Ferro package (ferrodock-model-ferro) owns the existing
19 model enum/flags descriptors moved from settings, plus the two original SplitView
enums, under logical assembly FerroDock.Model. Declaration-driven generator/parse
behavior and original domain value types are retained. Settings exports its own source
types and preserves public helper aliases to the same model descriptors. The moved
foreign enum tests now inspect their owning model export; settings alias/own-type tests
remain. New enum tests and assembly-qualified compiled assets verify original values.
XAML consumer manifests explicitly depend on the model SDK for generated public helper
paths. Core metadata has no controls dependency; general controls exclude the concrete
ReactiveUI backend/SDK; the backend SDK excludes the general control library and runtime
compiler; domain/reactive models exclude native base/controls. No Domain UI dependency
was added; two extra enum declaration text constants support the existing build generator.

A real SplitView asset reproduced invalid `ToolReference as Rc<dyn IDockableMetadata>`
emission. The shared compiler now distinguishes actual declaring trait handles from
foreign descriptor carriers, and checks that the source is its actual shared handle or
shared constructor value. Other representations use the registered cast path, matching
the runtime loader. A required/nullable policy consumer independently assigns actual
shared original overrides to the original interface. Native SetterBase is still its own
trait. The guide's `[T, Rc<T>, Option<Rc<T>>], this: Rc<T>` pattern has an executable
consumer and runtime-emitter regression: its valid native unsizing remains available
without requiring an extra runtime cast registration. Filtered tests register their own
assembly, removing an incidental ordering dependency. All use the shared compiler and
conversion registry. No alternate emitter/cast engine, model reflection or state table
is introduced. The existing five loader and one build changed files have refreshed
exact published/modified/patch hashes; base12/markup3 changes remain unchanged.

Final validation: **1,558 Rust tests pass**, zero failed/ignored, **43 suites**. Native UI381, downstream markup111, diagnostics49, settings33,
sample27, ReactiveUI model unit474, model metadata6 and backend SDK4. Full workspace,
strict all-target Clippy and release Core/backend/control/markup/sample library Clippy
pass after the strengthened multi-handle regression. Formatting passes. Standalone
SDK6+4 pass without the general control library or compiler metadata. The unchanged original C# assemblies independently pass 93 assertions,
including equivalent constructor/property/default/binding/method/identity/interface
assignment assets. Original C# sources and sibling FerroUI are untouched. No running
desktop sample/bundle/window/layout was rebuilt or restarted. New files are intent-to-add for review; no staged contents, commits or pushes.

All 18 Python verifier tests, both generators, source/theme/inspector links, inventory
and four exact vendor guards pass. Inventory remains 222 ported, 464 partial and
1242 pending /1928. Runtime/platform/compiler/CLR completeness is not established.

After verified build termination, storage pressure was resolved by removing obsolete
test executables and older duplicate base/control archives only. The exact record is
/Volumes/SSD/temp/ferrodock-port/ferrodock-layout-cache-cleanup.json. Source, live app bundles and current builds
were preserved; no broad cache cleanup or overlapping Cargo builds/tests occurred.

Next substantive scope: Factory and DockWindow class constructors; remaining Core
interface/POCO descriptor logical ownership; all enum/collection/generic/nullable/
x:Class/name/root cases; other backends/services/serializers and full source/API/
platform/custom identity/string boxing/CLR GC/lifetime parity. Preserve original engine
algorithms and API, passive views, actual model identity and metadata composition.
Do not mark the full goal complete from these control constructors or passing tests.

Current authoritative logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-layout-libraries.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-cast-regression.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-layout-standalone-sdk.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-reference.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-layout-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-release-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-layout-{ui,core,sdk,domain}-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-layout-python.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-layout-completion.log.

## Previous constructor SDK checkpoint

# Current ReactiveUI constructor integration SDK

This turn: PROGRESS. The full faithful-port goal remains unchanged and incomplete.
The latest goal read reports ACTIVE; no implementation blocker exists. The prior
blocked-status discrepancy below is historical and does not apply to this checkpoint.

The class-reference metadata formerly under src/Dock/FerroDock.Ferro/reactive_metadata
now belongs to src/Dock/FerroDock.Model.ReactiveUI.Ferro, package
ferrodock-model-reactiveui-ferro. It exports the logical original model assembly
FerroDock.Model.ReactiveUI and concrete constructors for Document, Tool, DocumentDock
and RootDock. Each returns the same enclosing original model instance through its
existing class reference; abstract bases remain constructor-free. The source engine
and public model APIs are unchanged. SDK owns one descriptor per source class; no
duplicate same-name registration, overlay constructor table or UI model is added.

The SDK depends on the original ReactiveUI backend and native base/ReactiveUI/System
services, plus the existing recycling conversion helpers. It has no dependency on the
general control library. General controls have no normal SDK/concrete backend
dependency; the converter test uses a dev dependency. Generic interface identities
remain available without selecting this backend. The SDK separately supplies its
own source interface identity facts for consumers using it alone. Applications
register controls and SDK explicitly; the sample and downstream test consumer now do
so. Sample-specific metadata inherits the moved source class references. The former
sample closed engine-model cast registrations now belong in the SDK and continue
using actual Rc<Document/Tool/DocumentDock/RootDock> handles and original engine casts.

The first consumer build reproduced private-module leakage in generated public root
return signatures. The published ferroui-build 0.3.0 package now has one isolated
source change, type_system/emit_types.rs: canonical TypeKey comparisons remain
unchanged, while emitted value signatures resolve complete type components through
existing public model export paths. Generic argument components resolve independently,
without substituting a carrier's path for an unrelated model. The current policy/
override/carrier fixtures remain in the same consumer. Four retained dependency
packages have exact source/modified/patch hashes, license/notices and original VCS
metadata, checked by scripts/dock/check_framework_patch.py. Base12/markup3/loader5 changes
from the previous checkpoint remain unchanged. Sibling FerroUI/original Dock sources
are untouched. No running desktop window or bundle was rebuilt/restarted.

Final validation: **1,547 Rust tests pass**, zero failed/ignored, **41 suites**.
Native UI381, downstream markup103, diagnostics49, settings38, sample27, ReactiveUI
model unit474 and SDK2. The new cases cover four assembly-qualified roots/properties,
actual concrete engine objects, original inheritance/abstract bases, fresh Load/name
scope completion, factory ownership/close, shared identity, native compiled live Title/
TwoWay IsOpen/Title, runtime-emitter public exports and rejection through the wrong
control assembly. Source defaults and actual CreateDocument command identity/null/
restore are checked through the ordinary metadata API. SDK2 additionally pass alone
without compiler metadata or control-library registration. The unchanged original C#
assemblies independently pass **69** assertions, including the equivalent four assets,
source defaults/commands, inheritance/fresh identity, live bindings and factory close.

Strict all-target workspace Clippy, release control/SDK/markup/sample library Clippy,
formatting, 18 Python verifier tests, both generators, inventory/theme/inspector links
and all four exact dependency guards pass. Normal dependency trees independently
confirm controls exclude SDK/concrete backend; SDK excludes the general control
library/runtime compiler; domain/reactive models exclude native base/controls.
Existing native dependency/linker warnings remain. Full whitespace checking only
flags retained original vendor source/patch context; authored non-vendor files pass.
The completion gate rejects the unfinished inventory as expected. Original Dock and
sibling FerroUI sources, running desktop window/bundle/layout and staged contents
remain untouched. New files are intent-to-add for review; no commits or pushes.

Still required: the other concrete original model constructors, complete logical
ownership for external Core and ReactiveUI descriptors, all x:Class/name/generic/
cross-package cases, full CLR type metadata/custom identity/string/GC cycles and the
wider source/API/platform/lifetime inventory. Current inventory remains 222 ported,
464 partial, 1242 pending /1928 and must reject completion. Do not declare the goal
complete from these constructor/metadata checks. Continue using original engines,
passive views, compiled metadata/bindings and explicit integration composition.

Final logs: /Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-workspace-tests.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-consumers.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-unit.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-reference.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-clippy.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-release-clippy.log, /Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-python.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-{ui,sdk,model}-dependencies.log,
/Volumes/SSD/temp/ferrodock-port/ferrodock-model-sdk-completion.log.

## Prior external carrier checkpoint

# Current external constructor metadata and original engine roots

Previous goal turn: PROGRESS (plain self-declared model roots and reference identity).
This turn: PROGRESS. The full faithful-port objective remains unchanged and incomplete.
Final authoritative goal reads unexpectedly report BLOCKED, without any reason field.
No implementation blocker was found and no status-update tool was called to block it.
The exposed goal tools cannot resume an unfinished goal; available updates only set
complete/blocked/paused. Do not infer that this metadata status means the port is at an
impasse. There is concrete SDK metadata/constructor work available below.

The actual unresolved carrier gap was reproduced before editing: two ordinary roots
for DockCapabilityPolicy and DockCapabilityOverrides emitted nonexistent
DockCapabilityPolicyMetadataImpl / trait-as-concrete return types. The baseline is
/Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-baseline.log. The old helper replaced a canonical model name
inside a different constructor-result type with the carrier's Rust path. That is now
removed. The model host uses the import-resolved declared value syntax, never the
carrier's path as the model type.

The runtime host reuses EXISTING register_type_rust_paths/rust_path_of_type for exact
return types. UI-owned model registration declares the public Rc<dyn policy-impl> and
Rc<overrides> value types. No public-path side registry is added. A compiler-only
MarkupType owner_rust_name slot, populated by ferro_markup_type, identifies the actual
Rust declaration that owns the generated functions. Runtime fallback only substitutes
complete occurrences of that owner through its public alias. Identifier/path suffixes,
including Unicode boundaries, cannot be rewritten into fabricated types. An external
carrier without a declared public model return path is refused until the shared path
registration is supplied. Normal self-declared root emission remains covered too.
The owner slot is NotRecorded without compiler metadata, with no model instantiation
or production runtime compiler dependency. It is static compiler metadata, not runtime
model reflection. All constructor/property operations still use the real shared pipeline.

Actual Core policy/override roots now compile and load through the ordinary native
loader and original Load extension. They are the original engine objects, not UI models.
The initialized source factory/layout consumes the loaded policy through the original
DockCapabilityResolver. Loaded overrides win; mutations of the same policy change later
evaluations. Scope completion, inherited properties, nullable defaults, fresh loads and
cross-interface reference identity are checked. Native tests retain an explicit owning
layout handle as normal app composition does: consuming/dropping the only root handle
would intentionally lose its existing weak-parent chain. No lifetime rule was changed.

A consumer-only Carrier/CarrierValue fixture independently covers a descriptor whose
name prefixes a different returned model name: the model compiler returns CarrierValue,
the runtime compiler refuses guessing before explicit registration, and emits the same
correct result after registration. The fixture holds original policy state; it is not a
production replacement model. Runtime emission also checks the actual declaration-owner
metadata and real policy/override return paths/one constructor, while the compiled model
host runs the resulting objects through the engine.

Final validation: **1,539 Rust tests pass**, zero failed/ignored, **39 suites**. Native UI
381, downstream markup 97, diagnostics 49, settings 38, sample 27 and ReactiveUI model unit
474. Four new integration tests cover the carrier regression. The unchanged original C#
assemblies independently pass **56** assertions, including both original Core root assets,
fresh identity, null/default fields, actual layout policy/override precedence and mutation.
Strict all-target workspace Clippy and release UI/markup library Clippy pass. Formatting,
18 Python verifier tests, both generators, inventory, original theme/inspector links and
all three exact dependency guards pass. Existing dependency/linker warnings remain.

Published 0.3.0 dependency copies remain isolated: base **12** modified files (adding
metadata/markup_type.rs and metadata/markup_macros.rs), markup 3, compiler 5. Exact original/
modified file and patch hashes are updated; licenses/notices/VCS metadata remain. See
vendor/NOTICE.md and vendor/ferroui-markup-xaml-patch.md. Original Dock source and the dirty
sibling FerroUI checkout are untouched. No desktop window/bundle was rebuilt or restarted,
preserving the user's arranged sample. No staged contents, commits or pushes; additions
are intent-to-add for review.

Still required: ordinary constructor exports for all concrete original model classes,
correct logical assembly ownership for UI-owned external descriptors, all x:Class/name/
generic/cross-package cases, full CLR polymorphic type metadata, arbitrary custom identity,
string boxing, managed GC cycles and the wider 1:1 source/platform/API/lifetime inventory.
Do not couple concrete ReactiveUI models into the general UI control library just to add
constructors, or introduce UI model surrogates. Domain must stay UI-independent. Further
SDK constructor declarations/metadata composition must preserve original class inheritance
and class-view identity, including custom source subclasses. This foundation now handles
separate constructor carriers correctly; it does not prove all model constructor exports.

Inventory remains **222 ported, 464 partial, 1,242 pending / 1,928**. No partial source unit
is promoted by these cases; the completion gate must reject. These are further tasks,
not blockers or a reduced goal.

Authoritative logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-final-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-final-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-final-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-reference.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-python.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-carrier-completion.log (expected rejection)

## Historical plain-root and identity checkpoint

# Current plain-model roots and enclosing reference identity

Previous goal turn: PROGRESS (shared object namescopes and named compiled models).
This turn: PROGRESS. Goal remains ACTIVE and the full faithful-port scope is unchanged.
The unrelated salon screenshot remains out of scope, as explicitly confirmed by the user.

The shared Rust emitter now returns the actual declared value type for a plain root,
uses that same instance for root/intermediate context, all property assignments, name
registration, scope completion and return, and only attaches a scope to styled roots.
It materializes a root constructor expression ONCE. The new root test initially proved
why this matters: re-evaluating the constructor created different objects for each step
and returned an uninitialized document. The final compiled root retains title and name
on the original engine-backed instance. Both model/build-time and registered runtime
emitter hosts have coverage; runtime-host source has the expected Rc return and exactly
one constructor. The ordinary native Load extension loads the compiled plain asset with
fresh objects each time. The unchanged C# reference loads an actual original Document
from PlainDocument.axaml, independently establishing source plain-root/fresh behavior.

Compiled document group records and imported build-method emission retain the declared
constructed handle, including plain values, rather than assuming a native Ref<Class>.
The exact public value syntax is read through the shared EmitMarkup abstraction in
model/runtime hosts. This is still the original transformer/AST/typed emitter pipeline;
no parser, loader or docking engine is substituted.

Shared ValueTypes metadata now supplies reference_identity/reference_equals and typed
register_reference_identity accessors. Existing register_reference<T> also supplies Rc<T>
identity. UI-owned registration declares original model interface and class-view identity;
Domain and its models gain no UI dependency. There is no per-object side registry: type
accessors return the enclosing object address, which is only compared, never dereferenced,
and retain no model instances. Equal scalar boxes remain different objects. Accessors run
outside registry and namescope borrows; native object projections run once per operand,
take precedence, and keep both native operands alive through comparison.

NameScope now uses that shared identity for plain values. Separately boxed IDockable,
IDocument, Document, DockableBase, ReactiveBase, ReactiveObject and concrete Rc<Document>
views register as the same original object. RootDock/IDock/IRootDock/base views do too.
Reference returns the stored original value. ReferenceEqualsMultiConverter uses the same
shared identity while retaining its prior custom-dockable projection fallback. It now
recognizes inherited class views rather than treating their wrapper boxes as new objects.
The original C# independently confirms interface/base class identity, mixed registration,
converter equality, different-model inequality and distinct boxed scalar inequality.

The runtime compiler is a test-only dependency of the downstream consumer, enabling the
real compiler-metadata configuration across its dependency graph. That exposed three
previously dormant short class paths in Fluent, Simple and Browser registration macros;
all now use the required crate-qualified paths. All ordinary theme tests also pass with
that feature configuration. No production runtime compiler dependency was introduced.

Final validation: **1,535 Rust tests pass**, zero failed/ignored, **39 suites**. Native UI
381, downstream markup 93, diagnostics 49, settings 38, sample 27, ReactiveUI model unit
474. The unchanged original C# harness passes **50** assertions. Eight new Rust tests
cover the actual plain root/load, both compiler hosts, document/root identity matrices,
registry/scope callback reentry, native projection invocation/priority and the converter.
Strict all-target workspace Clippy and release UI/markup library Clippy pass. Formatting,
18 Python verifier tests, both generators, source/theme/inspector links and all three
exact provenance guards pass. Existing dependency and linker warnings remain.

Three isolated published 0.3.0 packages remain: base 10 modified files, markup 3, compiler
5. The compiler adds declared value syntax in emit_types/runtime_types and group records
in compiled, in addition to its earlier emitter/runtime AST support. All licenses, notices,
original VCS/file hashes and exact diffs remain. Original Dock sources and the dirty sibling
FerroUI checkout are untouched. No desktop window/bundle was rebuilt or restarted, preserving
the user's arranged running sample. No files are staged, committed or pushed; additions
are intent-to-add for review.

Still required: constructor descriptor carriers whose Rust descriptor and returned model
are different types need complete public type-path declarations; plain-model x:Class/
generated names, every generic/cross-package root variant, arbitrary custom identity
representations, string boxing, polymorphic CLR type metadata and managed GC cycles remain
partial. The ordinary library constructor exports for all original concrete model classes
also remain to be completed. Do not generalize the self-declared named root fixture to all
of those contracts or substitute UI model surrogates. Full inventory remains **222 ported,
464 partial, 1,242 pending / 1,928** and the completion gate MUST reject. These are further
port tasks, not blockers or a smaller replacement objective.

Authoritative logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-root-identity-final-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-root-identity-final-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-root-identity-final-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-plain-root-reference.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-root-identity-python.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-root-identity-completion.log (expected rejection)

## Historical shared namescope checkpoint

# Current shared object namescope and compiled model references

This turn: PROGRESS. Goal remains ACTIVE and incomplete; full faithful-port scope is
unchanged. The salon screenshot was confirmed as intended for another chat.

ReferenceExtension now queries the genuine shared INameScope value contract. Plain
model handles retain their identity; controls retain the concrete native handle
restoration. This is immediate Find, without forward-reference deferral. No secondary
registry, UI surrogate for a model, parser or docking engine is introduced.

The native scope stores arbitrary values in its original map and pending searches.
Existing typed APIs remain compatible projections; native typed Find keeps its
allocation-free existing path. Registered reference handles use the shared ValueTypes
object conversion for duplicate identity rather than value equality. Mixed model/control
names conflict, completed scopes reject even duplicate registrations before null checks,
and callbacks run after map borrows are released. ChildNameScope uses the same value
contract so local plain objects shadow parent native objects for typed lookup too.
NameScopeLocator/NamedElementNode carry plain object values through normal compiled
bindings. The previous native weak attached-owner lifecycle is preserved; plain objects
are held strongly until their scope drops. Legacy native-only custom scopes retain the
existing API and explicitly reject unsupported plain registration.

The source INameScope metadata Register/Find methods now accept arbitrary objects.
Native ResolveByNameExtension uses the same scope and conversion/deferral path. Compiled
XAML registration accepts named plain application objects; the runtime AST evaluator
uses the same contract. The existing INamed/Name registration transformer is unchanged.
The normal downstream consumer fixture has an INamed application object backed by the
actual source Document, without a native control base. Compiled Reference resolves the
same object, and #Document.Title follows mutations of that original engine Document.
This fixture belongs only to the test consumer; production engine models are unchanged.

Twelve new independent cases cover identity/equal-value collisions, registered handles,
synchronous callbacks/reentry/completion, child fallback and mixed-type shadowing,
metadata invocation, compiled live binding, legacy scopes, null/completion precedence,
strong plain-object/weak owner lifetime and shared ResolveByName behavior. The unchanged
C# assemblies independently assert 42 outcomes, including the previously source-only
plain Document reference, duplicate identity, mixed-name conflict, pending callbacks
and completion. Native Cargo tests require no .NET.

Three isolated published 0.3.0 dependency packages are now declared: base (10 modified
files: the previous 4 plus 6 scope/binding/metadata files), markup (3: previous matching
fix plus ResolveByName and compiled runtime) and compiler (2: emitter, AST evaluator).
All original files, licenses/notices/VCS metadata and exact modified/patch hashes are
retained and guarded. See vendor/NOTICE.md and vendor/ferroui-markup-xaml-patch.md.
The sibling FerroUI checkout and original Dock source are untouched. No desktop window
was rebuilt or restarted, preserving the user's arranged live layout.

Still open: standalone plain-model XAML roots (compiler functions still require a native
class root), complete custom/interface/CLR identity and generated-name-field contracts,
arbitrary managed GC cycles, and the wider platform/API/lifetime source inventory.
In particular, concrete Rc<T> models registered with ValueTypes retain duplicate identity;
separate boxed trait-object/class-reference projections need further shared reference-
identity metadata before their duplicate equivalence can be claimed. Do
not generalize the compiled nested INamed fixture to unsupported standalone roots. These
are further port work, not a reason to mark the goal blocked or reduce the goal scope.

Clean final workspace: **1,527 Rust tests pass**, zero failed/ignored, **39 suites**.
Native UI 380, downstream markup 86, diagnostics 49, settings 38, sample 27 and
ReactiveUI model unit 474. Inventory remains **222 ported, 464 partial, 1,242 pending
/ 1,928**; completion correctly rejects. No source unit is promoted based on partial
runtime coverage. Strict all-target workspace Clippy and release UI/markup library Clippy pass. Formatting,
18 Python verifier tests, both generators, source/theme/inspector inventory links and all
three exact dependency provenance guards pass. Original dependency/linker warnings remain.
No files are staged, committed or pushed; new files are intent-to-add for review.

Authoritative logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-final-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-final-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-final-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-reference.log (42 unchanged-C# assertions)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-python.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-completion.log (expected rejection)

Next: extend the genuine shared compiler/root and reference-identity machinery for
plain model roots and separate interface/class-reference identity projections; keep
normal model metadata UI-owned and original engine objects unchanged. Do not create
UI surrogates or infer whole-library completion from the current named nested fixture.
An initial validation overlap invalidated an artifact during doc tests; that is superseded
by the clean final runs. Do not cite the overlapped /Volumes/SSD/temp/ferrodock-port/ferrodock-namescope-workspace-tests.log.

## Historical factory and markup-extension checkpoint

# Current factory helpers and native markup extensions

Previous goal turn: PROGRESS (complete main inspector/F11 and verified header grip).
This turn: PROGRESS. Authoritative goal state is ACTIVE again; no blocking condition
remains for continued work. Full faithful-port scope is unchanged and incomplete.
The unrelated salon screenshot was explicitly confirmed as intended for another chat.

Locked workspace: **1,515 Rust tests pass**, zero failed/ignored, **39 suites**.
Native UI 380, downstream markup 74, diagnostics 49, settings 38, sample 27;
ReactiveUI model unit suite 474. Inventory: **222 ported, 464 partial, 1,242 pending
/ 1,928**. The completion gate must reject. No new source unit is declared complete
merely because it compiles or passes its currently recorded cases.

The original FactoryExtensions.cs is now implemented in the UI-independent model
crate, with all five static helper bodies plus Rust extension syntax on concrete and
trait-object factories. It preserves null receivers, tracked-root priority even when
inactive, lazy first-active root search in factory order, current-document then root
focus casts, current/focused close dispatch and current dock/host lookup. Tools retain
the source IDocument contract. Seven actual engine/model tests include dispatch of a
non-document current item, live host replacement and real document closure through
the original factory. No extra eligibility filter replaces CloseDockable's policy.

UI-owned FactoryExtensionsMetadata exports all five original static names/parameter
contracts without adding any UI dependency to Domain. Its metadata invocation test
checks nulls and actual returned model identity. GetCurrentHostWindow exposed a missing
native nullable host-handle registration, now registered through shared ValueTypes.
Complete extension/CLR/custom-interface metadata remains partial.

New normal workspace library: src/Dock/FerroDock.MarkupExtension,
ferrodock-markup-extension. The original LoadExtension and ReferenceExtension retain
both constructors, optional Source/Name, ConstructorArgument attributes and ProvideValue.
The real shared scanner exports them via Cargo links metadata; the original default
framework XML namespace and explicit assembly/namespace both resolve in a separate
compiled consumer. The library uses existing base/markup dependencies only. No loader,
parser, registry or docking engine is substituted, and no dependency patch changed.

Load requires the calling URI context even for absolute URIs, skips BaseUri reads
without Source, delegates to FerroXamlLoader, retains fresh loaded objects and forwards
relative URI/error behavior. Source is read AGAIN after the external BaseUri getter:
a callback can replace Source or clear it after the guard. Actual C# confirms loading
the replacement URI and ArgumentNullException for the latter; native code retains the
replacement and corresponding programmer failure. No RefCell borrow crosses callbacks.
Reference uses immediate native name-scope Find, returning the same native object with
its concrete handle. It doesn't create a deferred forward-reference subscription.

Thirteen downstream Headless cases port the original eight constructor/provide/null/
identity tests and expand live base/name/provider, URI failure, callback reentry and
metadata coverage. The compiled consumer checks both namespace forms, positional and
property arguments, zero-argument extensions and missing names. These tests operate
across an actual package/metadata compilation boundary.

The new scripts/dock/factory-markup-reference uses the UNCHANGED original projects with
Avalonia Headless 12.1.3 / .NET 10.0.13. It asserts 33 supported source outcomes and one
additional source-only plain-model reference. Its probe merely exposes the original
protected tracking hook/records virtual close; real engine close is checked separately.
The saved assertion corpus is tests/Dock/data/factory-markup-reference.tsv. Normal Cargo
tests require no .NET; corpus assertions do not replace independent native tests.

Known next framework gap: original INameScope accepts arbitrary plain model objects,
confirmed by naming and resolving a C# Document. Current native INameScope only accepts
Ref<FerroObject>. Native UI references work, but plain-model reference parity is not
implemented. Extend the SHARED namescope/compiler contracts rather than introducing
an alternate registry or promoting plain domain models into UI objects. Other source
scope still includes five remaining native converters, RequiresDataTemplate metadata,
serializers, model variants and the broad original sample/test/API/platform inventory.

Full strict all-target workspace Clippy, strict release library checks for model/UI/
markup, formatting, 18 Python verifier tests, factory/coordinate generation, original
markup/theme-link, source inventory and both vendor provenance guards pass. Existing
native base/markup warnings, source SourceLink package warnings and macOS compact-unwind
warnings persist. Source originals, sibling FerroUI and both vendor patches are unchanged.
All changes are under FerroDock. No commits/pushes/PRs or agents were used.

Desktop sample was NOT rebuilt or restarted in this API/library turn; preserve the
user's live rearranged layout. The preceding inspector/grip checkpoint remains the
latest native screenshot/executable evidence. No new desktop behavior is claimed.

Logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-markup-workspace-tests.log (1,515 / 39 suites)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-extensions-tests.log (seven engine/model helpers)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-extensions-metadata.log (native static metadata)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-markup-extensions-tests.log (13 native consumer cases)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-markup-reference.log (33 source outcomes + known gap)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-markup-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-markup-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-markup-python.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-factory-markup-completion.log (expected incomplete)

## Historical main inspector/F11 and grip checkpoint

# Current main inspector, F11 and tool header grip

This turn: PROGRESS. The full faithful-port goal is unfinished. Its stored status is
BLOCKED without a supplied reason; implementation resumed on the user's instruction.
The available goal API cannot change blocked to active. The user's `/goal resume`
control restores automatic continuation; do not mark this goal complete or invent a
technical blocker. The previous locked-desktop visual obstacle is cleared.

Locked workspace: **1,494 Rust tests pass**, zero failed/ignored, **37 suites**.
Native UI 379, downstream markup 61, diagnostics 49, settings 38, sample 27.
Inventory: **222 ported, 455 partial, 1,251 pending / 1,928**; completion must reject.

The complete original RootDockDebug compiles with all five tabs, all 18 original
property-view templates and both recursive trees. RootDockDebug is passive; a focused
UI-independent ReactiveObject ViewModel owns the original Events/Filter/FilteredEvents,
26 factory subscriptions and ReactiveCommand clear action. Native attached behavior
handles context changes, tree selection, global picking and selection adorner lifetime.
Constructor-time context subscriptions precede caller DataContext assignment; native
Initialized can be delayed until attachment. Source factory replacement unsubscribes;
visual detach removes highlighting but retains logging, as the unchanged C# confirms.

The lazy event query captures the owner, reads its current ordinal-ignore-case filter
and retains original Events identity. At the native ItemsSource boundary, unfiltered
observable collections stay live and filtered IEnumerable queries materialize, matching
Avalonia ItemsSourceView. Clearing raises FilteredEvents even for an empty log. The clock
is injected through the library composition root and uses native local HH:mm:ss.fff.
The TitleOrTypeNameConverter and UI-owned ITool/IDocument/ILocalTarget descriptors use
original model handles; Domain gains no UI dependency. Source Tool implements IDocument
as well as ITool. Unregistered/custom CLR naming and full collection enumeration remain
explicitly partial.

All three original AttachDockDebug overloads are exported through shared metadata.
F11/default/custom gestures use actual native tunnel routing, lazy current-layout
lookup, original 400x680 default window, custom/null-factory fallback, Closed/main
Closing cleanup, canceled-close distinctions and repeatable explicit disposal.
The actual original DEBUG desktop App composition attaches F11 before F9. Its real
startup Headless test opens the complete inspector and toggles it off.

A native desktop check exposed an upstream FerroUI template-matching defect: registering
a conditional IDockable-to-IDock cast made every dockable match the first IDock template.
The isolated published ferroui-markup-xaml 0.3.0 copy changes only
`templates/data_template.rs` to check the actual cast before accepting assignability.
A Tool now selects DockableDebugView rather than DockDebugView. The new dependency has
exact file hashes and a one-file patch; the original four-file base patch is unchanged.
No sibling FerroUI source was edited. See vendor/ferroui-markup-xaml-patch.md.

The user-reported small header grip also occurs in the ORIGINAL Dock template:
the unchanged C# reference measures width 12 in a 600-wide header. Its columns were
*,Auto,Auto, assigning spare width to the left-aligned title panel. The user's requested
correction is an explicit source deviation: both linked Fluent/Simple templates use
Auto,*,Auto so the grip fills the available space between title and buttons. Native
regression checks assert a broad grip and +200 grip width when the window grows by 200.

The source-only inspector reference compiles the unchanged 18 view pairs and main
inspector/extensions/selection classes against original projects, Avalonia Headless
12.1.3 / .NET 10.0.13. Its corpus retains 18 grids, 24 enum edits and 26 log messages.
It asserts original filter/equality, factory replacement, detach logging, filtered UI
clear, F11/custom factory/gesture, user-close/reopen, canceled close, repeated disposal
and main-close behavior. The original Fluent project also reproduces the 12px grip.
The markup fidelity guard permits only declared namespace/assembly, explicit binding
scope and passive command/behavior migrations for the main inspector.

Strict all-target workspace Clippy and release diagnostics/sample library Clippy,
formatting, 18 Python verifier tests, factory/coordinate generation and original-source,
theme-link and both dependency provenance guards pass. Existing native base warnings,
two published markup warnings, original SourceLink package warnings and macOS debug
compact-unwind linker warning persist. Full custom/generic metadata, callback/reentry,
enumeration/exception/GC/culture/thread/process/platform scope, production serializers,
other model frameworks and the complete sample/application inventory remain partial.

The rebuilt physical system-volume bundle matches executable SHA256:
7262f885d1469c30ebacfafe765648cf0902266602f84a5241b8080e62d7d465
Native desktop verifies the expanded Tool1/Tool3 grips, all five inspector tabs and
live layout tree, correct Tool1 Selected property view, yellow source selection
highlight, F11 close/reopen and native close. The Events tab accepts filter input;
26 event output/filter/clear scenarios have independent C# and native Headless evidence,
not a separate populated desktop log demonstration. The app is left running, inspector
closed, F9 off, no pins. The user rearranged Document1 into a second document group
during validation; preserve that live state. Do not reset it to the startup graph.

App and screenshots (absolute directory):
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/
- FerroDock.app
- ferrodock-rust-header-grip-fixed.png
- ferrodock-rust-f11-inspector.png
- ferrodock-rust-f11-selected.png
- ferrodock-rust-f11-events.png

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-workspace-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-grip-tests.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-reference.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-release-clippy.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-desktop-build.log
- /Volumes/SSD/temp/ferrodock-port/ferrodock-main-inspector-python.log

No commits/pushes/PRs or agents. All source changes are under FerroDock.

## Historical inspector property-view checkpoint

# Current inspector property views

Previous goal turn: PROGRESS (unlocked native sample check: tab selection,
pin/open/unpin and F9 on/off). This turn: PROGRESS. Full faithful-port goal ACTIVE.
Locked workspace: **1,483 Rust tests pass**, zero failed/ignored, **37 suites**.
Native UI 379, downstream markup 61, diagnostics 38, settings 38, sample 27.
Inventory: **222 ported, 451 partial, 1,255 pending / 1,928**; completion must reject.

All 18 original passive inspector property views now have normal native class
implementations and compiled XAML. Each source file remains separately traceable.
Native constructed initializes the original component for base and derived classes;
views have no handlers or presentation logic. Public type/namespace/assembly/export
metadata and a separate downstream consumer construct every original view.
The source XAML is byte-identical after only namespace/assembly migration, including
all fields, binding modes, fallbacks, read-only states and original row definitions.
The new check_inspector_views.py guard and four verifier tests enforce this.

The UI-owned ISplitter, IProportionalDockSplitter, IGridDockSplitter,
IDocumentTemplate and IDocumentDockContent descriptors retain original domain handles,
forward all properties and CreateDocumentFromTemplate, and provide checked/nullable
casts. No copied model state or UI dependency in Domain is introduced. The optional
IDocumentTemplate notifier hook follows the existing IDockable/IDockWindow pattern.
Three missing Orientation/DockMode/GridResizeDirection enum descriptors are generated
from their original domain declarations. Standard parse/type-converter metadata lets
the existing native binding engine handle enum text edits; no SDK patch was added.
Undeclared numeric enum values remain the existing native plain-enum representation
gap. Do not equate the source-view checkpoint with full native/CLR/platform parity.

The new scripts/dock/inspector-view-reference compiles all 18 UNCHANGED original XAML/
code-behind pairs against real model/control projects and Headless 12.1.3 / .NET
10.0.13. It captures exact grid structures and 24 actual enum edits (case, whitespace,
numeric member, comma combination, invalid/empty/overflow). Native Headless compares
all saved outputs and checks real model edits, notifications, nested owner/active/
layout replacement, live pinned/hidden collection counts, read-only fields, nullable
content identity, factory dispatch, splitter preview and derived-view initialization.
The original reference also asserts title/permission edits and nested-owner updates.

Strict all-target workspace Clippy, formatting, 16 Python verifier tests, both
generators and original-source/theme/provenance/markup guards pass. Full workspace
passed before a redundant test .into() cleanup; all 38 diagnostics tests pass again
afterwards. The existing native base 23 warnings, macOS debug compact-unwind and
original NuGet SourceLink package warnings persist. Original C# and sibling FerroUI
sources and the four-file provenance-checked vendor patch are unchanged.

Main RootDockDebug/F11 remains pending. Next coherent work: port the original
TitleOrTypeNameConverter, then the complete RootDockDebug markup plus source behavior
through focused ReactiveUI presentation/behaviors (passive view requirement), then
RootDockDebugExtensions and the actual sample F11 composition root. Use all 18 actual
property views; do not create a reduced inspector. Source file:
../../../Dock/src/Dock.Avalonia.Diagnostics/Controls/RootDockDebug.axaml.cs (294 lines), original
26 factory events, live filter/clear, selection overlay, attach/detach and global
SelectDockableRequested routing. Existing selection/debug overlay foundations remain.

Current logs:
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-workspace-tests.log (1,483 / 37 suites)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-final-diagnostics.log (38 after test-only cleanup)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-expanded-tests.log (diagnostics/settings/consumer)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-reference.log (unchanged-source actual reference)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-clippy.log (strict all-target workspace)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-python.log (16 Python tests)
- /Volumes/SSD/temp/ferrodock-port/ferrodock-inspector-completion.log (expected incomplete)

The currently running native app/bundle is the previous F9 checkpoint, not rebuilt
for these new property views; no desktop F11 result is claimed. The unlocked visual
check left Document2/Tool1 selected, no pins, F9 OFF. Source unpin placed Tool1 after
Tool2 in its group. Screenshot:
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/ferrodock-rust-unlocked-check.png
The previous app hash and packaging details below remain the last desktop evidence.
No commits/pushes/PRs, goal status changes or agents were used.

## Historical F9 checkpoint

# Current original F9 diagnostics overlay

Previous goal turn: PROGRESS (selection foundations and verified native sample,
1,454 tests). This turn: PROGRESS. The full faithful-port goal remains ACTIVE.
The locked full workspace passes **1,468 Rust tests**, zero failed/ignored, **37
suites** (native UI 379, downstream markup 60, diagnostics 25, sample 27).
Inventory: **222 ported, 415 partial, 1,291 pending / 1,928**. Completion must reject.

The normal diagnostics library now implements all source bodies for
Controls/DebugOverlayAdorner.cs, Helpers/DebugOverlayHelper.cs,
DockDebugOverlayManager.cs and DockDebugOverlayExtensions.cs. Source native Control
construction, descendant traversal/root exclusion, bounds transforms, flag order,
red/green/blue one-pixel outlines and distinct alpha-80 hover fills remain intact.
The footer uses shared native object display, invariant FormattedText, white 12-point
text and the original four-pixel bottom/right inset. No custom drawing/layout/hit
engine replaces FerroUI. Native Headless tests check actual DrawingGroup pen/fill/
geometry and glyph output, marked hit order, pointer movement, hidden descendants,
strong callback ownership, repeat-add and removal behavior.

The manager uses the existing .NET-order System Dictionary and original initial,
current, key and disposal snapshots. Both public source extension overloads preserve
native tunnel routing, F9/default/custom gestures, lazy toggling, explicit cleanup,
existing-only desktop windows and top-level-only single views. Source ActionDisposable
remains callable on repeated disposal, without a once-only closure. Native undo token
bookkeeping uses weak publisher handles; event callbacks retain source strong owners.
Routed removal removes all matching helper subscriptions, while CLR LayoutUpdated -=
removes only its last matching subscription. No Drop disposal is substituted.

Shared scanner export, static/runtime namespace metadata and both original overloaded
method names are registered. The actual source App composition root attaches F9 only
under debug_assertions, as original DEBUG. A separate Headless startup assembly runs
real App/lifetime/compiled layout and verifies its actual hook, not a manual attachment.
Views remain passive. F11 is not replaced: its original inspector is still pending.

The independent scripts/dock/debug-overlay-reference links all FOUR unchanged original
classes and the actual original Dock native project under Avalonia 12.1.3 / .NET
10.0.13. It captures 16 flag/hover render cases and asserts original hotkey/dynamic/
disposal behavior. Every rectangle color, fill, width and bounds matches natively.
An initial test wrongly expected detached-dock removal to remove its layer child.
Both original C# and native output instead retain an orphan because the helper looks
for its layer AFTER detach. Tests now preserve that confirmed source behavior; the
production algorithm was not changed to hide it. SourceLink again requires its cache
path with a trailing slash via a global build property; original files stay unchanged.
The official framework RemoveHandler source also confirms all matching routed delegates
are removed: https://github.com/AvaloniaUI/Avalonia/blob/master/src/Avalonia.Base/Interactivity/Interactive.cs

Full all-target strict Clippy and strict release library Clippy for diagnostics/sample
pass. Formatting, 12 Python checks, both generators and source/theme/native provenance
guards pass. A Copy-only Clippy correction is followed by another 25-case diagnostics
pass. Existing 23 native base warnings and the compact-unwind linker warning persist.

The desktop executable rebuilt and the physical system-volume bundle matches SHA256:
e1cf99e392fd3a6112166c59afcb94698c524f23a8fa109f414d02146efaa94a
The old OWN app exited through native Quit; the refreshed app launched and is left
running. Actual native F9 showed the original area outlines; selecting Document2 still
worked with the overlay enabled, and F9 removed the outlines. Final state: Document2,
original three-document/eight-tool layout, no pinned items, overlay OFF. Hover filling
and footer glyphs are tested in native Headless; the desktop AX click did not establish
a separate pointer-hover/footer result. Do not claim a desktop hover screenshot.
Current app and evidence directory:
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/
FerroDock.app
ferrodock-rust-f9-overlay.png
ferrodock-rust-f9-tab-selection.png
ferrodock-rust-f9-off.png
ferrodock-rust-f9-off.ax.txt

Original inspector/F11, complete public/internal visibility and generic metadata,
arbitrary unregistered/custom CLR type naming/current-culture object display,
callback/reentry/enumeration/exception/GC/thread/process and broader platform scope
remain partial. Production serializers, remaining model variants/frameworks and the
full original sample scope remain incomplete. No unsafe, reflection, source stub,
alternate UI/docking engine, original C# or sibling FerroUI edit was introduced.

Current logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-workspace-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-overlay-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-overlay-reference.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-app-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-release-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-debug-desktop-build.log

## Historical diagnostics selection checkpoint

# Current diagnostics selection and verified native sample

Previous goal turn: PROGRESS (Browser theme/panel, 1,442 tests and rebuilt bundle).
This turn: PROGRESS. Full faithful-port goal remains ACTIVE. This checkpoint
supersedes the historical records below. The locked full workspace passes **1,454
Rust tests**, zero failed/ignored, **37 suites**. Native UI remains 379, downstream
markup 60 and sample 26; the new diagnostics library adds 12 cases.

New normal-workspace crate: src/Dock/FerroDock.Ferro.Diagnostics,
ferrodock-ferro-diagnostics. Its actual source SelectionOverlayAdorner and
SelectionOverlayHelper compile against native Control/AdornerLayer, logical parenting,
coordinate translation, DrawingContext and strong LayoutUpdated subscriptions. The
adorner retains the plain nullable HighlightRect property, yellow two-pixel outline,
base Render and source default hit testing. Drawing tests inspect the real native
DrawingGroup's pen, fill and geometry, without a replacement render surface.

The helper retains source same-root/no-layer behavior, pre-attach target retention,
root switches, layout-driven geometry, disconnected target clearing and explicit
subscription removal. Removal intentionally leaves the attached adorned value; if a
root has lost its layer, it does not guess the previous layer to remove its orphan.
Collection callback reentry preserves the original cleared-field failure. Native
subscriptions retain their owner until explicitly removed; no weak replacement or
Drop disposal changes the source contract. CLR collection/lifetime scope is still partial.

TryGetControl keeps original visible/pinned/tab map order, short-circuit evaluation,
identity and success with a valid root but null UI control. The framework-neutral
IDockableControl marker has the existing hidden boxed-reference projection pattern:
plain implementations preserve their Rc, the canonical native adapter returns its
exact original Ref. A UI-owned borrowed-type probe delegates to shared native runtime
object casts, including base/root handles and custom controls. No UI dependency,
reflection, replacement control/model or extra identity registry enters the engine.

The source-only scripts/dock/selection-overlay-reference links the UNCHANGED original two
classes and actual original native/ReactiveUI model projects. Under Avalonia 12.1.3 /
.NET 10.0.13 it captures **216** combinations of six states in all three factory map
pairs. Rust compares every returned root/control. Source attached/live/detached/removal/
reentry checks also pass. The original SourceLink build initially rejected its local
NuGet cache path lacking a trailing slash; a global NuGetPackageRoot command-line
property fixes the environment without editing original build/source files. Initial
native tests exposed an incorrect STATIC cast of an upcast root; runtime object casts
now preserve the actual class, including custom/native root handles. A drawing test
was corrected to inspect the native recording group's actual PlatformGeometry bounds.
Metadata tests account for native nullable boxing normalization.

The user unlocked the Mac and requested the visual check. The actual sample rebuilt,
was physically repackaged, and the old OWN app exited through its native Quit action.
The refreshed app launched through native app control and is RUNNING (observed PID
66997; re-check authoritatively rather than assuming the PID persists). Its refreshed
headers now have the original compact title/icon composition, with no body views in
the tabs. Actual native clicks verified Document2 selection, Tool2 selection, pinning
Tool2 to the left, opening the pinned host, unpinning back to its original dock and
returning to Document1. The app is left running with all three documents/eight tools,
Tool2 selected in its original group, no pinned items and Document1 selected.
This clears the previous manual-unlock/visual-refresh blocker. No unrelated app was used.

Current app:
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/FerroDock.app
Physical executable SHA256 (matches the normal Cargo desktop build):
f5e4460b009711b1ba36b0bcffde2e32e0bf162c4bda46aa74cedcf82f19a6af
Actual native screenshots and final accessibility state:
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/ferrodock-rust-running.png
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/ferrodock-rust-pinned.png
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/ferrodock-rust-running.ax.txt
These validate the recorded native flow on this machine, not full platform/input parity.

Inventory: **222 ported, 411 partial, 1,295 pending / 1,928**. The two complete
assembly namespace declarations are ported; implementation units remain partial.
Only selection foundations are implemented in the diagnostics library; the
original inspector views, debug overlay helper/manager/extensions and F9/F11 sample
integration remain pending. Internal modules have narrowly scoped dead-code allowances
until their original consumer is ported. Production serializers, remaining model
frameworks/native model variants, original sample units and complete API/metadata/
callback/collection/exception/GC/thread/process/platform scope also remain unfinished.
No reflection, unsafe code, source stubs or alternate docking engine is introduced.
The four-file dependency support patch and sibling FerroUI sources stay unchanged.

Final checks: strict all-target workspace Clippy and strict release library Clippy
for diagnostics, UI, model and the sample pass. Formatting, 12 Python verifier tests,
both generators and source/theme/provenance integrity guards pass. Full completion
correctly rejects unfinished scope. After a Clippy-only needless-borrow correction,
all 12 diagnostics tests pass again; the assembly test also verifies both exact
namespace definitions and shared registration identity. New source files are included
in whitespace verification; the exact vendor copy/literal patch is excluded as before.
The source C# tree outside FerroDock remains unchanged. Nothing is committed or pushed.

Current logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-overlay-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-overlay-reference.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-workspace-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-desktop-build.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-release-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-selection-final-clippy.log

## Historical Browser overlay and delayed panel checkpoint

# Current Browser overlay and delayed tab panel

Previous goal turn: PROGRESS (Simple theme library, 1,421 passing Rust tests).
This turn: PROGRESS. The full faithful-port goal remains ACTIVE. This checkpoint
supersedes the historical records below. The locked full workspace passes **1,442
Rust tests**, zero failed/ignored, in **35 suites** (native UI 379, downstream markup
60, Browser 5, Fluent 4, Simple 7 and sample 26). Strict all-target workspace Clippy
and strict release library Clippy for UI, all three themes and the sample pass.

New normal-workspace library: src/Dock/FerroDock.Ferro.Themes.Browser,
ferrodock-ferro-themes-browser. It preserves the original Fluent dependency and all
FIVE source documents. BrowserTabTheme retains the embedded DockFluentTheme, two
merged resource roots and all 21 original overlay styles (22 style children total).
The compact dictionary is intentionally EMPTY: changing Browser density notifies
its owner but does not invent metrics or change the nested Fluent density/cache.
Its host override keeps regular ContentPresenter, padding 6, full multi-tab tool
window decorations and extended client-area behavior. Original colors/platform
options, full local document selector, gutter/body compensation, z-order, close/icon/
compact states and disabled horizontal scrolling remain source-driven markup.

The Browser accent requires the original DelayedUniformTabPanel. Its complete initial
native class now implements the six registered properties, all source width/debounce
state, minimum/viewport precedence, pixel rounding, measure/arrange, attach/detach
observers and real DispatcherTimer/DoubleTransition/CubicEaseOut. Signed TimeSpan
properties are preserved; only the timer's required nonnegative interval uses the
existing native conversion. Managed foreach mutation detection uses immutable native
storage identity, without holding collection borrows across callbacks. A real child
Measure callback mutation test asserts the original failure, not snapshot continuation.
Source strong timer captures remain strong; stopped native Rc cycles are an explicit
CLR collection gap. No custom layout/timer/animation engine replaces the shared APIs.

The actual unchanged C# panel and actual VisualRootCompatExtensions are linked into
scripts/dock/delayed-tabs-reference. .NET 10.0.13 / Avalonia 12.1.3 captures desired extent
and every child's x/width for 24 layouts, including fractional/infinite measurement,
constraints, negative spacing and hidden children. Native checks compare every output.
The original asynchronous delay/close-burst/viewport/restart scenarios run through
real native dispatcher tasks. Further checks cover signed delay, transition/easing
configuration, observer teardown, reentry, empty reset and derived construction.
Actual animation frame/cadence and attached DPI/platform scenarios remain partial.

The source typed OnPlatform<double>/OnPlatform<Thickness> implementations already
exist in the published SDK but lack their closed markup declarations. Two UI-owned
metadata carriers declare the actual native handles/options and delegate every
operation to those shared implementations. The Browser assembly supplies the required
namespace/type paths; there is no alternate OS selection, parser, value storage or
new framework patch. Native option outputs and actual double/thickness handle types
are tested. The entire generic SDK interface surface is not claimed complete.

FerroUI's genuine constructed virtual now performs source constructor work for the
panel and all THREE theme classes (Fluent, Simple, Browser), including native derived
instances. Theme service-provider arguments are held only until population and then
consumed. Dedicated derived tests prove inherited timer callbacks, resource population
and density/cache APIs. The original sample keeps its Fluent configuration.

An independent compiled consumer exercises the actual original factory/model docking
graph, real native pointer tab selection, cached view identity and attached full-selector
value, Browser selector state matrix and original multi-tool floating-host behavior.
Library checks cover source cache/style identity and notifications, nested Fluent
invariants, genuine variant palettes/live dynamic background and typed platform values.

Inventory: **221 ported, 408 partial, 1,299 pending / 1,928**. Completion must still
reject. Existing 23 native base warnings and compact-unwind linker warning persist.
The four-file native support patch, original C# and sibling FerroUI checkout stay
unchanged. Source manager/full generic metadata, callback/exception/GC/thread/process
and broader input/platform coverage remain incomplete.

Build recovery: debug incremental caches exhausted the SSD. Generated dev outputs
were cleared; debug/test incremental caching is disabled to avoid repeating the large
cache growth, retaining normal debug/test semantics. An early rebuild was accidentally
started before cleanup ended and failed on a missing object. Cleanup then completed;
subsequent validation runs sequentially from fresh outputs. No source was removed.
The final GUI refresh still awaits the previously requested manual Mac unlock.

After all source changes, the actual desktop executable rebuilt successfully and
scripts/dock/bundle_macos_sample.py physically updated the existing system-volume app:
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/FerroDock.app
The bundled executable is not a symlink and matches the new build byte-for-byte
(SHA256 9b1467f87ed04b76324dec90bdecd08ccf6d8d7bd1b6f6925743aa3ab0b9cc95).
The old app process was not restarted while the Mac remained locked; the refreshed
binary/window is not claimed visually verified. The final serialization-only fixture
change writes '-' for empty child lists, removing trailing-tab whitespace; the actual
.NET generator and native corpus test pass again after this change. Formatting and
workspace whitespace checks pass (excluding the exact vendor source/literal patch).
Both generators, 12 Python tests and source/theme/provenance guards pass; completion
still correctly exits 1 for unfinished scope. No C# source outside FerroDock changed.

Current logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-browser-construction-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-browser-combined-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-delayed-reference.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-delayed-panel-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-browser-workspace-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-browser-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-browser-release-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-delayed-final-corpus.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-browser-desktop-build.log

## Historical Simple theme checkpoint

# Current original Simple theme library

Previous goal turn: PROGRESS (docking-state markup and source-view failure reference,
1,412 tests). This turn: PROGRESS. The full faithful-port goal remains ACTIVE.
New normal-workspace crate: src/Dock/FerroDock.Ferro.Themes.Simple,
package ferrodock-ferro-themes-simple. The original sample keeps its original Fluent
configuration; its code/bundle did not need replacement for this library addition.

The original Simple project links the same Controls/Assets as Fluent. The Rust Simple
package carries all 48 translated control dictionaries and five exact image files as
self-contained inputs, with only assembly-local image URIs changed. Its eight own
source documents retain the Simple accents, compact resources, all five presets and
root's exact 42 merged dictionaries/order. All 56 class/resource documents compile;
unused linked dictionaries are not inserted into the root. The class and presets use
the normal shared compiler/composite loader and original embedded asset names.
No Fluent theme is a normal dependency; the locked Cargo dependency graph confirms
that. Native ferroui-themes-simple is pinned to published =0.3.0 for validation.

DockSimpleTheme ports the actual source Styles class, optional service-provider
constructor, both DirectProperties, compact extraction/resource lookup, owner resource
invalidation and cached-document template override lifecycle/identity. Its resource/
style adapters retain the original native owner. DockSimpleThemeManager supplies the
original shared names/URIs/prefix and first direct Simple default-owner lookup to the
existing shared preset algorithm. No resource manager or docking engine was replaced.
Full manager/base CLR/markup class surface, callback/reentry/enumeration, derived-class
metadata and CLR lifetime/thread/process/platform parity remain partial.

Six new library cases cover all nine source normal/compact scalar expectations,
interface lookup, same-value no-ops and resource invalidation, cached style remove/
recreate/identity, shared source names/index/variant guards and app/default-owner
includes, all five preset loads, exact editor/sidebar colors in BOTH native Simple
light/dark variants (including Default aliases), and real nested-default grip pattern
clearing for VS Code Dark and Rider Dark. Optional service-provider construction and
native source class/style/resource identity are checked too.
Three independent compiled-consumer cases verify public class/property metadata,
actual title-bar template, the existing original Factory/models rendering root/split/
document/tool graph, cached tab view identities through selection, inline pinned
alignment updates, and all five PNG assets opened from the Simple assembly. The graph
uses existing original engine test infrastructure; it is not a production replacement.

Latest full workspace: **1,421 Rust tests pass**, zero failed/ignored, 33 suites.
Simple library 6; downstream UI markup 57; native UI 368; sample 26; settings 37 and
settings markup 2. A subsequent targeted matrix also passes after extending all preset
color assertions to both native light/dark variants. Strict all-target Clippy passes.
TWELVE Python tests pass, including a new three-case link guard failure suite. The
link guard rejects missing/extra/changed shared files and wrong asset URI owners.
Formatting, structural inventory and the unchanged four-file native patch guard pass.
The old 23 local dependency warnings and compact-unwind linker warning remain.
Inventory: **221 ported, 397 partial, 1,310 pending / 1,928**. New source units stay
partial; original unit-test records explicitly cover only the translated scenarios.
Completion must still reject.

Strict release checking of the new Simple library passes. Both source generators,
unchanged-original-source and workspace whitespace checks pass too (the latter excludes
the exact vendor copy/literal patch). Full completion was rerun and correctly rejects
remaining source scope. Release log: /Volumes/SSD/temp/ferrodock-port/ferrodock-simple-release-clippy.log.

Current logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-simple-workspace-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-simple-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-simple-color-matrix.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-simple-clippy.log

The final sample visual refresh still awaits the manual Mac unlock requested earlier.
No further unlock attempts or changes to system settings were made this turn. Library
work is independent of that UI limitation. No original C# source, sibling FerroUI
checkout or vendored framework file was changed. No commit, push or PR was created.

## Historical docking-state metadata checkpoint

# Current docking-state markup and source-view failure contract

Previous goal turn: PROGRESS (original native application/startup, executable/bundle,
1,410 tests). This turn: PROGRESS. The full faithful-port goal remains ACTIVE.
The locked workspace now passes **1,412 Rust tests**, zero failed/ignored, 31 suites:
settings 37, independent settings markup 2, native UI 368, downstream UI markup 54,
sample 26 and Fluent manager 3. Inventory is unchanged: **221 ported, 383 partial,
1,324 pending / 1,928**. Full completion remains rejected.

DockingWindowState now has UI-owned metadata generated from the ACTUAL domain
bitflags declaration through the existing shared TypeModel export. No UI dependency
was added to the model and no second enum type/values were created. The original six
PascalCase names and signed i32 storage, flags marker, nullable registration and typed
constant access are preserved. Downstream source markup now compiles named values,
comma-separated combinations, known-bit numeric values and x:Static references, all
returning the original engine type. Build/runtime descriptors are checked together;
all 98 source .NET fixtures round-trip through runtime integer conversion and nullable
assignment, including unknown/negative bits and i32 boundaries. Out-of-range i64
inputs and wrong value types are rejected. The typed emitter entry point returns the
original domain value, with no replacement parser or engine.

Remaining enum gap: the shared typed flags emitter only emits values made of declared
members (flags_compose / flags_value in ferroui-build / markup-xaml-loader). Runtime
integer metadata preserves all source i32 bits, but compiled unknown-bit literals and
the inherited System.Enum surface remain unverified/incomplete. Do not mark this enum
fully ported on the strength of known-combination compiled tests.

Original ProportionalStackPanelView stays PENDING and absent from the Rust application.
An independent reference now compiles the UNCHANGED original view/code-behind and
actual proportional controls with Avalonia/Headless/Fluent 12.1.3. Its original
constructor throws NullReferenceException at markup line 50, adding the first child
to null ItemsSource. The reference asserts construction still throws; the initial
unhandled stack identified the exact source line. Native unchanged-structure markup
instead fails compilation with FRN3000, IList.Add not a declared member. Preserve the
original operation/failure timing when shared emitter support exists. Do not add
explicit collection objects or change ItemsSource to Items to make this view work.
The original sample startup does not reference the view. Reference inputs/command and
attribution are in scripts/dock/proportional-view-reference/README.md; no C# input changed.
No uncompiled Rust view, reduced view or changed-collection variant remains in Cargo.

Current inventory notes were reconciled with actual App/theme/startup integration;
old deferred statements saying there is no runnable sample no longer remain in the
active inventory. Original input/lifecycle/platform scope stays explicitly incomplete.
The Mac is still locked according to native app control: final visual refresh requires
a manual unlock. This blocks only computer inspection, not library progress. The
previously packaged actual sample remains available at the path recorded below.

Strict all-target workspace Clippy and strict release library checks for settings,
native UI, Fluent and the sample pass. Formatting, both source generators, NINE Python
checks, unchanged-original-source and provenance checks pass; workspace whitespace
checks exclude the exact vendor copy/literal patch. Existing native warnings persist.
Strict log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-clippy.log.
Release log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-release-clippy.log.

The actual desktop executable was rebuilt and physically repackaged; the bundle
matches the new executable byte-for-byte. Visual refresh remains pending the manual
unlock. Desktop build log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-desktop-build.log.

Current full-regression log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-workspace-tests.log.
Targeted metadata log: /Volumes/SSD/temp/ferrodock-port/ferrodock-flags-metadata-tests.log.
Original view reference log: /Volumes/SSD/temp/ferrodock-port/ferrodock-original-proportional-view.log.

## Historical original application checkpoint

# Current original Rust application and native sample

The faithful-port goal remains ACTIVE. The actual original ReactiveUI sample now
has a desktop executable and has launched a native macOS window titled FerroDock Demo.
It uses the existing original docking engine/factory and three-document/eight-tool
layout, not a reduced application. This checkpoint supersedes older no-executable
statements below. Full original API/platform/lifetime parity is still unfinished.

New production work:
- App.xaml retains the original application resources, ViewLocator, full native Fluent
  and DockFluent themes, default preset, shared recycling, region/acrylic dictionaries,
  header templates and transparent ToolChrome. App is the FIFTEENTH compiled original
  class resource. Original ProportionalStackPanelView remains pending.
- Program follows original native platform detection, Inter, ReactiveUI registration,
  dock preview/0.6 opacity, warning trace and classic desktop lifetime configuration.
  The source Main returns void; the Rust main likewise discards the lifetime result.
  Desktop is the sample's default feature; --no-default-features retains library usage.
- App initializes the actual DockFluentThemeManager and source nullable static manager
  slot, then constructs the original MainWindowViewModel and actual window/single view
  for the matching native lifetime. MainView theme and window-close/exit logic live in
  attached behaviors and ReactiveCommands. Views remain passive. A UI-independent theme
  selection service contract separates presentation from the infrastructure adapter;
  concrete native services are wired in App's composition root.
- Shared DockPresetThemeManagerBase preserves constructor validation order, source
  lists/identities, index selection/guards, ordinal source matching, recursive removal
  through custom dictionaries, default-owner identity rules and separate source includes.
  DockFluentThemeManager supplies the original five names/URIs, prefix and owner lookup.
  Native snapshot enumeration is explicit; CLR callback/reentry/enumerator behavior
  and unrestricted derived-class API remain pending.
- Isolated native support lives in vendor/ferroui-base, an exact published 0.3.0 package
  except FOUR declared files. The resource changes add safe plain-provider Any recovery,
  optional dictionary projection and existing mutable merged-provider-list access.
  The fourth file fixes existing assignability in ValueTypes::try_convert, permitting
  object-form templates to nullable interface targets without invoking a failing direct
  converter twice. Dock's original object-valued IconTemplate API is retained.
  Original hashes/revision and exact declared edits are guarded by
  scripts/dock/check_framework_patch.py; no registry source or sibling checkout was edited.
  /Volumes/SSD/repos/FerroUI remains READ ONLY. Do not format or alter unrelated vendor
  files; the package is excluded from workspace formatting.
- Source ordinal string helpers use the existing frozen mapping and 3,005 independent
  .NET corpus cases. DockingWindowState Display matches 98 outputs from the ACTUAL
  original linked C# enum, captured with .NET 10.0.13. Unknown bits use signed decimal;
  known combinations use original names/order. Full flags/System.Enum markup API
  remains partial; its inventory record was downgraded rather than overclaimed.

Latest full regression: **1,410 Rust tests pass**, zero failed/ignored, 31 unit/doc/
binary-test suites; sample 26, native UI 368, downstream markup 54, new Fluent manager
cases 3. Sample cases cover actual App resources/rendering, all five presets and theme
command routing, both native lifetime branches, real window Closing and source Exit
cleanup, live source list/captured manager semantics and explicit empty icon templates.
The Headless desktop test uses the native window/state contracts with an independently
raised Exit event: stopping its actual dispatcher would cancel the test task. Production
uses the real lifetime interface. No fake shutdown test is described as native exit.

The actual first native window launch exposed full document/tool views falling back
into icon presenters; the fourth native support edit fixes that conversion. Headless
checks now assert empty Panel icon content, no body views inside either tab strip,
real template Rc identity, invalid cast rejection and one-call failing conversions.

Native launch evidence: direct terminal launch failed with native render timer
CV error -6661. A symlink bundle stalled in dyld before main and was terminated after
inspecting its own process stack. A PHYSICAL executable copy in the system-volume
FerroDock.app launched successfully through native app control and showed the actual
full window. These observations do not establish the cause of the failed launch modes.
The portable scripts/dock/bundle_macos_sample.py helper makes an atomic physical copy,
checks an existing bundle's identifier and never launches or replaces unrelated apps.
The verified output location is:
/Users/danw/.codex/visualizations/2026/10/20/01a159ac-1bde-7af3-b2d4-2898fb2041f3/FerroDock.app

Strict workspace all-target Clippy, formatting, structural inventory, native patch
integrity and NINE Python verifier cases pass. The local native dependency reports
23 existing warnings; the existing compact-unwind linker warning remains. Inventory:
**221 ported, 383 partial, 1,324 pending / 1,928**. Completion MUST still reject.
Strict release checks for the native UI/Fluent/sample libraries and both source
generators also pass. Workspace whitespace checks exclude the provenance-controlled
vendor copy and literal patch; their original whitespace is deliberately preserved.
The physical app executable was checked byte-for-byte against the desktop build.
The Mac then locked: a request to close/refresh the earlier running window was refused
by native app control. The refreshed bundle is ready, but a final visual check requires
the user to unlock the Mac. Do not describe the refreshed window as visually verified.
Release log: /Volumes/SSD/temp/ferrodock-port/ferrodock-app-release-clippy.log.
Desktop build log: /Volumes/SSD/temp/ferrodock-port/ferrodock-app-desktop-build.log.

Current full-regression log: /Volumes/SSD/temp/ferrodock-port/ferrodock-app-workspace-tests.log.
Strict workspace log: /Volumes/SSD/temp/ferrodock-port/ferrodock-app-clippy.log.

Remaining work includes original DEBUG F9/F11 diagnostics, full static-field/flags/
System.Enum/generic metadata, callback/delegate/collection/reentry/CLR lifetime parity,
original broader input/accessibility/platform scenarios, serializers, other model
frameworks and all remaining original units. No goal completion or blocked status is
justified while this work can progress. No commit, push or PR was created.

## Historical compiled sample view checkpoint

# Current compiled original sample views and native model metadata

The faithful-port goal remains ACTIVE. There is still **no Rust sample executable**;
no desktop sample was launched. The actual original sample window now composes its
real docking graph in Headless. App resources/startup and theme-control behavior still
need integration; do not substitute a reduced demonstration or claim desktop readiness.

New compiled work:
- Native class-reference metadata preserves the actual source chain: concrete sample
  Document/Tool -> DockableBase -> ReactiveBase -> ReactiveObject, and concrete Home ->
  RootDock -> DockBase -> DockableBase -> ReactiveBase -> ReactiveObject. All references
  retain the actual original Rc model and delegate existing source contracts and INPC;
  no parallel model/engine state was introduced. Tool is not the Document CLASS even
  though its source interfaces include IDocument. Full ReactiveObject observable/
  generic metadata and source base-class constructors remain pending.
- The UI library depends on abstract model contracts, without a new concrete ReactiveUI
  model-package dependency. Concrete source casts/reference declarations are CLOSED
  facts in each sample metadata file and markup/engine_models.rs. Do not restore the
  generic register_dockable<T>/register_document<T> helpers: the standard shared scanner
  cannot export unresolved T cast facts, which caused unrelated negative type checks
  to fail. The transient private ModelProbe only presents the source boxed_reference to
  standard ValueTypes casts, immediately returning the actual class reference. It is
  never model/application state. Generic custom-model and runtime-type parity stays open.
- The recycling value bridge supports explicit typed boundary registrations. Raw context
  instances and canonical model-interface boxes round-trip with exact Rc identity and
  native source class metadata. Registration stores only monomorphized functions, not
  instances. A non-Clone mutable-value test verifies state/identity and release. General
  unregistered arbitrary objects retain the existing fallback and are not a parity claim.
- All actual sample context classes and 12 original view-model types have native class
  metadata, constructors and existing inherited model contracts. MainWindowViewModel's
  actual Layout/status/commands bind without a substitute model. The internal binding
  fixture group is private (.internal(), cfg(test) reexport); it is not a public loader.
  `extern crate self as ferrodock_reactiveui_sample` permits standard generated paths
  for own-crate concrete metadata. Only the composite compiled_markup loader registers.
- FOURTEEN original view resources/classes now compile: Dashboard, Home, Document,
  eight distinct ToolN views, DockableOptionsView, MainView and MainWindow. Source layout
  and bindings are retained, with migrated names, an explicit root-menu binding scope and
  x:Object for the nullable enum-item template. Views only initialize compiled markup.
  The extra ProportionalStackPanelView sample unit remains pending.
- DockableOptionsView retains the source three-element nullable-object array and exact
  eleven flags combinations (1,2,4,9,10,12,17,18,20,25,28), including null as Default.
  Shared fixed-size mutable storage preserves identity and element writes. Native list
  views read that SAME storage and do not invent collection-change notifications.
  As with other native static state, process/thread-scope parity remains unverified.
- The source UI assembly's seven default XML namespace definitions are exported.
  MainView's original unprefixed DockControl now resolves through those declarations.
  Published native RenderOptions lacks its markup descriptor: a metadata-only carrier
  declares the TEN existing native Get/Set accessors, delegating all calls to the real
  native type. It adds no replacement rendering property/state or reflection. Other
  RenderOptions value members are not claimed complete. Original Inter assets and the
  native with_inter_font registration are used (published =0.3.0 font dependency).
- Source ViewLocator preserves null, exact-factory, broad IDockable Match and missing-
  type throw semantics. The original StaticViewLocator 0.1.0 generator was verified at
  b3d2d8c6738d371033eeeb50fd0e25e5ce897dc6: it replaces ViewModel with View in full names
  and accepts direct UserControl subclasses. It deliberately emits the ORIGINAL
  `Not Found: *.Views.MainWindowView` TextBlock factory for MainWindowViewModel, because
  that view does not exist. This is source behavior, not a new placeholder. Do not map
  it to MainWindow. No reflection, runtime naming lookup or source-generator substitute
  engine is used. Reference:
  https://github.com/wieslawsoltes/StaticViewLocator/blob/0.1.0/StaticViewLocator/StaticViewLocatorGenerator.cs

Twenty sample tests pass: the ten original model/factory tests plus source class/base
metadata and actual owner identity, inherited live/two-way binding, Dashboard command/
captured context and Home focus, context/value round-trips, actual main view-model
binding, exact static arrays, ALL nine document/tool views and ALL seven two-way
permission toggles plus nullable pinned/state selection, all eleven exact view factories
and original missing/error paths, and the ACTUAL original MainWindow/full docking graph
with real document views and live root replacement after reset. The source DockControl's
local default IRootDock templates precede application lookup: the Home node correctly
uses RootDockControl in that graph; standalone original HomeView is tested separately.
The test harness's synthetic layout identity probe uses Tag, so application ViewLocator
is not incorrectly asked to render an unmapped plain root in a generic ContentControl.

Latest locked validation: **1,399 Rust tests pass**, zero failed/ignored, 30 suites;
sample 20, native UI 368, downstream markup 54. Strict workspace all-target Clippy and
strict release checks for native UI/Fluent/sample libraries, fmt, structural inventory,
factory/coordinate generators, SEVEN Python verifier tests, source-name/placeholder scan,
whitespace and unchanged-original-C# checks pass. Inventory: **222 ported, 376 partial,
1,330 pending / 1,928**. Completion gate MUST still reject unfinished scope. The existing
macOS compact-unwind linker warning remains. Current logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-compiled-sample-workspace-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-compiled-sample-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-compiled-sample-release-clippy.log

Next work toward launch: implement the original MainView InitializeThemes behavior via
commands/behaviors (views remain passive), actual App resources/lifetimes and Program/
platform/bootstrap. First enable the faithful preset-manager algorithms: shared native
IResourceProvider needs safe plain-provider type recovery and dictionary interface
projection; IResourceDictionary needs its mutable merged-list handle (not just snapshot).
The two concrete managers are still ABSENT; keep source recursive matching/removal and
custom dictionary support. Do not add unsafe casts, type/identity side registries, or
recognize only dictionaries created by this application. Use an isolated native support
change; the dirty sibling /Volumes/SSD/repos/FerroUI remains READ ONLY. No dependency
sources or sibling files were edited. All actual dependencies are still published
=0.3.0, with no local framework patch/path override. Broader original source scope,
serializers, alternate models and native input/platform/callback/GC/lifetime parity are
still pending. Goal stays active; do not mark complete or blocked while work can progress.

## Historical sample model/factory checkpoint

# Current original ReactiveUI sample models and layout factory

Previous goal turn: PROGRESS (full native DockFluentTheme and window-resource graph).
This turn: PROGRESS. The full faithful-port goal stays active. There is still no Rust
sample executable; no desktop sample was launched. Do not replace the original sample
with the test graph or a reduced/demo application.

New NORMAL WORKSPACE member: samples/Dock/FerroDockReactiveUISample, package
ferrodock-reactiveui-sample. It is currently a compiled LIBRARY and carries source
attribution. All shared dependencies remain published =0.3.0. No uncompiled/staged
sample bodies or alternative engine were introduced.

Actual complete source unit translations now compiled:
- Models/DemoData, Models/Documents/DemoDocument, and eight Models/Tools/ToolN types:
  original empty context classes with distinct Rc object construction identity.
- DocumentViewModel and all eight distinct ToolNViewModel classes: original native
  ReactiveUI Document/Tool base composition and complete inherited contracts. Outer
  reactive sender/interface identity is retained; no duplicated model engine/state.
- DashboardViewModel uses original DockBase; HomeViewModel uses original RootDock,
  with full inherited contracts and source base observation initialization.
- CustomDocumentDock: original strong self-retaining ReactiveCommand, can-create guard,
  nullable visible-count interpolation, fresh DocumentViewModel, separately reread Factory
  for add/active/focus. With no list, source nullable count produces ID "Document", then
  "Document2" on the next creation. With no Factory it leaves the list absent.
- DockFactory: ALL original source methods/fields: complete 3-document/8-tool layout,
  left/right vertical groups, original proportions/alignments/grip modes/permissions,
  main horizontal splitters, Dashboard/Home/default root and four separate pinned lists;
  custom document docks/windows, source context/dockable/host locators and original
  InitLayout. Contexts are fresh where source creates them; Home keeps supplied object,
  Dashboard captures passed layout, while Root/Documents lambdas read LIVE factory
  fields. Host lambda reads UseManagedWindows at INVOCATION. Strong source captures
  remain strong; only native weak runtime identity thunks are internal construction.
- MainWindowViewModel: complete source state/commands/24 event subscriptions, original
  initialization + Navigate("Home"), global tracking status, close/reset guard logic,
  ReactiveUI equality/notifications. Source InitLayout selects DefaultDockable Home on
  BOTH startup/reset, even though CreateLayout alone initially selects Dashboard.
  NewLayout/global tracking callbacks retain the source owner strongly.
- IHostWindow now has a hidden native runtime class-name projection for source
  GetType().Name status without reflection. HostWindowHandle uses the ACTUAL native get_type().name(), preserving derived source
  class names too rather than its Rust adapter name. A real native subclass test passes.
- IDockThemeManager's four-member trait is present using the EXISTING domain shared
  IReadOnlyList adapter; published System has no IReadOnlyList interface. There is no
  placeholder/fake manager implementation.

Ten sample tests pass, covering exact full layout and leaf classes/capabilities,
source locator freshness/captured-vs-live identities, original document creation and
nullable/no-factory edges, selection/focus, native-vs-managed late host choice, original
custom floating window/dock/title, startup/reset/close, global status/notifications and
outer ReactiveUI sender/interface identity, plus actual derived native host class names.
Native tests keep explicit factory roots
alive; inherited weak Factory backlinks are an already-recorded lifetime parity gap.

IMPORTANT discovered shared dependency gaps before porting preset managers:
1. Published ferroui-base 0.3.0 IResourceProvider has no Any/safe plain-object type query.
   ResourceInclude is a plain Rc type; its provider implementation has no as_object.
   A merged provider cannot be recovered as ResourceInclude or inspected for Source,
   as original DockPresetThemeManagerBase does. Forward ValueTypes casts do not provide
   safe inverse trait-object recovery. No unsafe cast, identity side registry or reduced
   preset-recognition algorithm was added to bypass this gap.
2. IResourceDictionary exposes merged_dictionaries_snapshot(), but no original mutable
   MergedDictionaries list through the interface. Source recursive remove/add must work
   for custom dictionaries, not just concrete ResourceDictionary. This needs a faithful
   shared native API extension rather than silently ignoring custom providers.
The dirty sibling /Volumes/SSD/repos/FerroUI was READ ONLY inspected: its current source
has the SAME gaps. It was not edited. Exact published sources also remain unchanged.
Do not claim the actual managers are ported. Full manager types/methods remain absent.
Only their four-member interface is declared. This does not block progress on original
sample views/compiler integration and remaining source units. If fixing FerroUI support,
use an isolated reviewed dependency change and keep the original resource algorithms;
do not mutate the user's dirty sibling or introduce a replacement resource system.

Another explicit gap: published System has no Debug.WriteLine/listener API. Original
24 diagnostic event subscriptions/messages are kept; debug messages currently use
cfg(debug_assertions) stderr. Listener identity/current-culture numeric trace formatting
and general runtime-type/lifetime parity are unverified. Release elides debug arguments
as original Conditional DEBUG calls do. This is recorded as partial, not full parity.

Current validation: **1,388 Rust tests pass**, zero failed/ignored, 30 test/doc-test
suites; sample 10, native UI 368, downstream markup 54. Strict all-target Clippy, strict
release checks for sample/native UI/Fluent theme, fmt, structural inventory, source-name/
whitespace/original-source checks and factory/coordinate generators pass. Inventory guard
now also audits samples/**/*.rs (mapping, forbidden upstream names and placeholders),
with a meaningful new Python fixture; all SEVEN Python tests pass. Inventory: **222
ported, 345 partial, 1,361 pending / 1,928**. Completion gate must still reject scope.
Existing macOS compact-unwind warning persists. Logs:
/Volumes/SSD/temp/ferrodock-port/ferrodock-original-sample-workspace-tests.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-original-sample-clippy.log
/Volumes/SSD/temp/ferrodock-port/ferrodock-original-sample-release-clippy.log
Cargo clean -p ferrodock-ferro safely
removed old build artifacts after disk space fell to ~1 GiB; current space recovered.

Next work toward launch: compile the ACTUAL original sample views/resources, source
ViewLocator exact type mappings (StaticViewLocator is already generated/no reflection),
DockableOptionsView source option arrays and source theme-selection behavior, followed
by App/Program/native platform/bootstrap. Add proper native metadata/projections for
these exact custom model classes rather than registering all dockables as one fake
sample type. Views stay passive per AGENTS; source MainView events must move to source
behavior/command counterparts. Complete the original preset-manager API support above
without weakening its matching/removal/recursion semantics. Then launch and inspect the
real executable. Full source scope, serializers, alternate models, platform/input,
GC/CWT/delegate/boxing/custom collection/API gaps remain incomplete.

## Historical complete Fluent theme checkpoint

# Current full Fluent theme and original window-resource integration

The faithful full-port goal remains active. There is still no Rust sample executable;
no desktop sample was launched. This checkpoint completes the remaining source window
resource trees and the actual DockFluentTheme class/root. The next work is the original
application/bootstrap and its remaining services/views, using the existing engine/API.

New compiled production counterparts:
- HostWindowTitleBar: current-runtime native TemplatedControl, source style key and
  PART_Background resolution, actual strong-owner automation peer with original role,
  name/help and direct visual-root HostWindow/fallback focus invocation. The source
  legacy version-11-only branch remains unverified.
- IntLessThanConverter: source zero-default threshold, exact i32-only comparison,
  null/wrong-type false, native metadata/interface and unsupported ConvertBack error.
- All FOUR previously missing complete source dictionaries: HostWindow,
  HostWindowTitleBar, MdiDocumentWindow and WindowDrawnDecorations. Full source trees,
  styles and templates are retained. The four source MDI menu layout-command bindings
  use explicit IDocumentDock owner casts so every binding remains compiled, following
  the engineering guide; no reflection bindings or replacement controls were added.
- IDockWindow native descriptor: all 16 properties and eight methods forward to the
  original domain handle, with optional handle and source INPC query. DockWindowState
  uses the existing Settings foreign-enum generator over its original declaration;
  all four values/native handle roundtrip. The scanner REQUIRES explicit named imports;
  a wildcard core import leaves descriptor model handles unqualified and downstream
  compiled Window.Topmost resolution fails. Runtime getters/setters take the normalized
  nonnullable handle, while the notifier accepts original/optional boxes.
- DockDensityStyle: original Normal=0/Compact=1 and native enum metadata.
- Actual DockFluentTheme: native Styles class/direct density and cache properties,
  parent-first notification, compact resource extraction/removal, compact-first lookup,
  owner invalidation, original cached DocumentControl selector/style/setter add/remove
  order and strong storage. No borrow is held across source callbacks. Native IStyle
  and IResourceProvider wrappers retain theme identity and route compact lookup.
- COMPLETE original DockFluentTheme.xaml: all 50 merged dictionaries in source order,
  separately keyed CompactStyles and managed ToolChrome template override. Its public
  assembly namespace is mapped to the framework XML namespace as in the original.

Build/API detail: a CLASS group compiles only documents reachable from the root.
Presets therefore have a separate compiled_presets group, preserving all five public
preset exports. Reexport both groups and register the generated compiled_markup
COMPOSITE loader. Registering compiled_xaml::try_load afterward incorrectly hides
preset loading. Directory/file rerun inputs and forward-slash document paths are
explicit. Assets remain embedded by the shared compiler; dependencies stay =0.3.0.
The generated-emitter Clippy allowances remain confined to generated modules.

Nine new tests, all passing in the full locked workspace, prove:
- original resource count, compact extraction/priority and normal reset;
- owner notification on actual property changes and cache style create/remove/identity;
- downstream markup construction of the full theme and titlebar, real background part,
  native TitleBar peer/fallback focus; original integer converter contract;
- actual source MDI template button event routing to original model Minimized/Normal/
  Maximized states (a maximize click while minimized restores NORMAL, per source);
- runtime public loader for root and all five original presets;
- actual source HostWindow template/DockControl and live Window.Topmost INPC binding;
- actual original DockControl/factory graph with cached document template: both source
  document containers remain the SAME objects while engine selection switches visible
  content back/forth, with real root/split/tool/pinned controls still present.

Full locked workspace: **1,378 tests pass, zero failed/ignored**, 28 test/doc-test
suites; UI 368, downstream markup 54. Structural inventory: **212 ported, 329 partial,
1,387 pending / 1,928**. The completion gate correctly rejects unfinished scope.
Strict all-target Clippy and strict release checks for BOTH native UI and Fluent theme
libraries pass, as do fmt/inventory/generators/six Python/source-name/whitespace checks.
The existing macOS compact-unwind linker warning persists. Original C# remains unchanged.

Remaining direct path: port the actual original ReactiveUI sample application/factory/
views/models and bootstrap, including source DockFluentThemeManager and theme selection
behavior. App views must remain passive under the engineering guide; source view event
behavior belongs in commands/behaviors rather than copied code-behind handlers. Use the
published native desktop/platform and ReactiveUI APIs and the already-ported engine.
Do not substitute the test graph, reduced demo, alternate model implementation or
staged/uncompiled bodies for the original application. Launch/inspect the real executable
before reporting sample readiness. Full platform/input/identity/ownership parity, custom
and alternate model metadata, serializers and broader source scope remain outstanding.

Other still-open constraints from earlier checkpoints: EitherNotNull's borrowed-slice
native converter API copies its all-null list fallback and does not preserve the original
IList object identity. General boxing/interface casts, CWT ephemerons/CLR-GC cycles,
delegate Target/method identity, native weak adapter lifetimes and direct-root source
fallback/platform semantics are not proven complete. Do not weaken strong source edges
or invent ancestor fixes/drop cleanup to hide these differences.

## Historical pinned/layout and earlier checkpoints

# Current pinned host and complete dock-layout dictionary integration

The full faithful-port goal remains active. The previous goal turn made concrete
progress on original Fluent scroll/document/tool/chrome resources. This turn adds the
actual remaining pinned host/panel and layout resource graph. There is still no Rust
sample executable or complete DockFluentTheme; do not claim desktop sample readiness.

Compiled production source additions:
- PinnedDockControl: full initial 1,021-line source unit translated to native class,
  four source styled properties/validation, template parts, grid sizing and minima,
  stored bounds/half-pixel tolerance, per-tool override and priority native/INPC
  subscriptions, inline resize, native/managed overlays, owner/splitter/pointer and
  explicit detach/close behavior. Native and INPC property handlers and owner resize
  method remain separate source counterparts. No marker host or reduced sizing engine.
- PinnedDockHostPanel: original Panel child-resolution/effective alignment/override,
  empty/hidden logic and inline/overlay measure/arrange, including source sole-pinned
  child behavior and .NET NaN/signed-zero min/max. Shared native child COW snapshots
  avoid a new collection mirror or per-pass allocation.
- PinnedDockControlAutomationPeer: readonly strong owner, source class/Pane/name/ID/help,
  actual model expansion/collapse and source empty-list state semantics.
- Alignment, DockMode, Orientation, SplitViewDisplayMode, SplitViewPanePlacement and
  EitherNotNull converters with original singleton metadata. All declared enum mappings,
  both directions, null and unrelated-object passthrough identity are tested. The native
  multi-value converter signature borrows a slice: EitherNotNull's all-null list-valued
  fallback currently copies values, so original input IList reference identity is an
  explicit unresolved dependency/API representation gap. Do not replace it with null
  or count that reference behavior complete.
- UI-owned native descriptors for all nine layout/content interfaces, their INPC hooks,
  checked/base casts and original model projection. Every declared property/method is
  forwarded; content values use the existing UI/model value bridge. Global-target marker
  metadata and original proportional/erased-model casts are included. General reverse
  marker/interface recovery and custom/alternate model metadata remain partial.
- Twelve COMPLETE original dictionaries: PinnedDockControl, DockControl, DockDockControl,
  DocumentContentControl, GridDockControl, ProportionalDockControl, RootDockControl,
  SplitViewDockControl, StackDockControl, ToolContentControl, UniformGridDockControl,
  WrapDockControl. Native namespaces, assembly URIs and compiled binding scopes are
  explicit; original trees/styles/templates remain intact.

Actual integrated evidence: DockingGraphConsumer includes 46 available source resources
in the original DockFluentTheme order. Its tests construct real DockControl and original
ReactiveUI factory/root/proportional/document/tool models; default source data templates
create the actual native root, split and document/tool controls. Original factory selection
updates the real tab index. Real RootDockControl creates PinnedDockHostPanel/PinnedDockControl;
per-tool Inline override produces original pixel definitions, and original empty/replaced
pinned-dock/Left-to-Right alignment scenario rebuilds the SAME grid correctly. This is a
Headless original resource/control/engine graph test, not the original application or
full theme class. Four dispatcher/layout passes drain source deferred nested content.

Latest locked full workspace: **1,369 tests pass, zero failed/ignored** (UI 367; downstream
markup 46). Twenty-five additions: 11 pinned control, four original fixed-size host panel,
four converter, three native contract/property/content cases and three actual docking
graph integration tests. Strict all-target and release UI-library Clippy, formatting,
inventory, factory/coordinate generators, six Python verifier tests, source-name and
whitespace/original-source checks pass. Existing macOS linker unwind warning persists.
Inventory: **210 ported, 320 partial, 1,398 pending / 1,928**. Completion remains rejected.

Next direct path to the requested Rust sample:
1. Finish FOUR remaining source control dictionaries: HostWindow, HostWindowTitleBar,
   MdiDocumentWindow, WindowDrawnDecorations, including their real helper dependencies
   (e.g. IntLessThanConverter). The actual native classes mostly already compile.
2. Port DockFluentTheme root/class and source Themes density/cache API. Keep the full
   resource order, resource-node override, owner invalidation and cached document-template
   style behavior. Do not substitute the test consumer's partial include list for it.
3. Port an actual original ReactiveUI sample's application/factory/views/resources and
   native platform bootstrap through the shared compiler, then launch and inspect it.
   Preserve all chosen source sample behaviors; the full port objective remains broader.
4. Continue remaining original input/capture/drop/adorner/close/reentry/leak/platform
   scenarios, native public/generic/factory metadata, alternate models and serializers.

Fidelity cautions: keep original strong callbacks/owners until explicit detach; direct
visual-root casts must retain their source fallback, not an invented ancestor search.
Native pinned window construction can occur without a matching direct Window owner,
as source; disabling UsePinnedDockWindow alone does not close an existing one. Source
CloseWindow clears opacity/hit-test local values rather than caching/restoring old values.
Only actual GridSplitter parts receive original resize handlers; custom Thumbs retain
source exclusion. Source completion reads effective mode although both branches update
bounds; the merged call retains that read. NaN delta must not silently use Rust f64::max.

Native model notification priority uses the existing IDockable.boxed_interface hook for
Ref<FerroObject> before INPC. Actual alternate framework-property models/native branch
still need source fixture validation. CLR GC/CWT/delegate identity, native interface weak
expiration, arbitrary string/custom reference boxing, invalid raw enum boxes and compiled
indexer Item versus Item[] differences remain open. Original C# and =0.3.0 dependencies
are unchanged. No sibling checkout edits, agents, commit, push or PR were used.

## Earlier Fluent checkpoint (superseded by the current integration above)

# Fluent scroll/document/chrome checkpoint

The full faithful-port goal remains active. The user wants a runnable Rust sample.
The original C# sample is not the requested outcome. No Rust executable or complete
DockFluentTheme exists yet; complete engine/API/desktop parity remains unproven.

The previous goal turn made progress: the actual host/control/adorner/tab/pinned stack
entered Cargo. This turn extends that real stack with the following source units:
- ScrollViewerLineCommand and its original enum: native ICommand, no-op change event,
  native receiver check and original four ScrollViewer line calls. Base-typed receiver
  recovery uses ValueTypes::as_object then the native class cast; generic try_cast to
  an unregistered target handle did not recover that viewer and was corrected.
- ScrollViewerScrollButtonVisibilityConverter: source visibility/enum/default parsing,
  invariant NumberStyles.Float using the shared native parser, finite-value guards,
  source parameter parsing and strict endpoint rules. Read-only Instance remains a
  static property, not a field. IsMaximized/ReferenceEquals also preserve source static
  Instance properties; other original singleton fields stay fields.
- GripModeConverters (all five native FuncValueConverter fields), IsMaximizedConverter,
  DocumentTabOrientationConverter, DocumentTabDockConverter, DocumentLayoutModeMatchConverter
  and ReferenceEqualsMultiConverter. Reference equality checks original box/native/model
  identity; equal scalar boxes remain distinct. General reboxed strings/custom Rc/interface
  identity without shared runtime support remains open; no content-equality fallback is used.
- Six COMPLETE source dictionaries: ScrollViewer, ToolChromeControl, DocumentControl,
  ToolControl, DocumentDockControl and ToolDockControl. Original trees/styles/templates/
  bindings remain, with framework/assembly qualification and explicit binding scopes.
  Fluent now directly depends on the existing deferred-content crate because compiled
  original chrome/content templates instantiate those real source controls. Cargo.lock
  records that internal edge; published dependencies are still exactly =0.3.0.

Downstream tests now use the expanded original resource graph. All four original
DocumentEmptyContentTests assertion sets pass with actual DocumentControl: empty/nonempty,
add/remove visibility and original empty-template identity/text. Implicit type-key theme
resolution is exercised; no replacement template is assigned. ToolDockControl realizes
original chrome/grip/buttons/ToolControl; drain the native dispatcher twice after layout
for the source DeferredContentPresenter before inspecting its child. ScrollViewer source
templates contain FOUR command buttons (horizontal and vertical); use actual Grid.Row=1
for the two horizontal buttons. The original command/viewer parameter, execution and
visibility changes are tested through those real compiled buttons.

Locked workspace: **1,344 passed, zero failed/ignored** (UI 345; downstream markup 43).
Fifteen new tests cover converter boundaries, real native command directions and six
source resource integration scenarios. Strict all-target Clippy, strict release UI-library
Clippy, formatting, inventory, factory/coordinate generators, six Python verifier tests
and source-name/whitespace checks pass. The shared 0.3.0 emitter repeats nullable command
setter branches; only generated Fluent code now allows if_same_then_else alongside its
existing clone_on_copy allowance. Handwritten code retains strict warnings.
Inventory: **210 ported, 295 partial, 1,423 pending / 1,928**. Newly integrated source units
remain partial. The completion gate still rejects the unfinished port.

Next work toward the original sample:
1. Port remaining source theme helpers/converters and dictionaries. Missing control
   dictionaries: DockControl, DockDockControl, DocumentContentControl, GridDockControl,
   HostWindow, HostWindowTitleBar, MdiDocumentWindow, PinnedDockControl, ProportionalDockControl,
   RootDockControl, SplitViewDockControl, StackDockControl, ToolContentControl,
   UniformGridDockControl, WindowDrawnDecorations and WrapDockControl. PinnedDockControl's
   actual 1,021-line source class and peer remain absent: port them, not a marker or
   reduced pinned host. Most other referenced container classes already compile.
2. Port the original DockFluentTheme class/root graph/density/cache API and any missing
   native public metadata. Then port the original ReactiveUI sample application/factory/
   views/resources through the shared compiler and native platform bootstrap, launch and
   inspect it. Do not replace the original sample/theme/engine with a reduced showcase.
3. Continue broad original input/capture/drag/drop/float/adorner/pinned/close/reentry/
   accessibility/factory cleanup scenarios and remaining engine/API/model/serializer units.

Previous runtime cautions still apply: source strong callbacks/owners, explicit detach,
repeated getter/event ordering, direct root casts and quirks must remain. CLR GC/CWT
cycles, arbitrary delegate.Target identity, native weak-interface expiration, custom/alternate
model metadata, compiled indexer Item versus Item[] notifications and desktop/platform
parity are open. IDragOffsetCalculator markup, ManagedHostWindow public metadata and
remaining generic/factory API still need completion. No reflection or native dependency
checkout edits were introduced. No agents, commit, push or PR were used.

## Earlier native stack checkpoint (superseded by the current integration above)

# Native stack integration checkpoint

The full port goal remains active. The user wants the original Rust sample to run;
launching the C# sample is not the requested outcome. There is still no Rust sample
executable or complete DockFluentTheme, and full API/behavior parity is unverified.

All 26 formerly staged Rust bodies are now in normal Cargo modules. Duplicate staging
sources were removed. Actual DockControl, HostWindow/State, ManagedHostWindow/State,
DockControlState/ManagerState, AdornerHelper, GlobalDockingService, factory service,
WindowDragHelper, concrete DockHelpers, ManagedWindowLayer.TryGetLayer and preview
Show/host discovery compile against published =0.3.0 dependencies. Existing tested
static/context/preview/helper bodies were retained when merging continuations.

DocumentTabStrip/Item and ToolTabStrip/Item are actual full initial native source
translations with properties, containers, pointer/scroll/window behavior and peers.
The original generic tab peers use macros generating closed native specializations
with typed strong owners. Their internal runtime names differ from CLR generic names;
this representation and broader provider/lifetime parity remain partial.

DockableControl, DocumentControl, ToolControl and both host peers now compile too.
Tracking registers actual source models/controls/root values in original factory maps,
updates original bounds/pointer fields and keeps source explicit subscriptions. The
source document host observes the original model/collection without a mirror. Added
BorderFillMargin, DocumentTabLayoutMatch and OwnerIsToolDock source converters and native
metadata. TrackingMode metadata is generated from the original domain enum declaration
through the existing shared build bridge, without a copied enum or reflection.

Seven complete original dictionaries now compile: WindowChromeResources, document/tool
tab strips/items and pinned item/container. Namespace/assembly qualification changed;
original structures/bindings/styles remain. A downstream consumer realizes actual tab
and pinned containers, original DockableControl tracking modes and factory registration.
Tool strip's original single-item rule hides its presenter; use two actual tool items
when validating that template. The complete custom ScrollViewer/chrome/document/tool/
dock/window theme graph is still pending; this test is not a full themed application.

Latest locked workspace: 1,329 tests pass, zero failed/ignored (UI 336; markup consumer 37).
Twenty-one additions cover original host geometry/NaNs/fractional restore/state/reopen,
foreign receiver/close-state identity, explicit move acceptance/cancellation/release,
actual manager/context identity, factory callback transfer/release, tab state/providers,
both original pinned tracking cases, all three tracking registries/context/detach,
source visible-list replacement/detach, document/tool peer first-item activation,
converter boundaries and downstream control/property/template metadata. Tests with
ReactiveUI models do not validate the unported alternate framework-property models.
Strict all-target and release UI-library Clippy, formatting, structural inventory, factory/
coordinate generators, six Python verifier tests and source-name/whitespace checks pass.
Inventory: 210 ported, 280 partial, 1,438 pending / 1,928; no newly integrated file is
claimed fully ported merely because it compiles. Original C# and dependencies unchanged.

Next work:
- Complete remaining source Fluent dictionaries and their actual helper converters/
  commands, DockFluentTheme class/density/cached-content API and required native types.
  Begin with ScrollViewer and its actual source command/converter, then document/tool
  content and dock containers, host chrome/window and the complete theme root graph.
- Port the original ReactiveUI sample application/factory/views/resources through the
  shared model compiler and native platform bootstrap. Launch and inspect that Rust
  application only after original resources/engine wiring work together.
- Finish remaining public/contract metadata, including IDragOffsetCalculator markup,
  ManagedHostWindow and arbitrary factory callback/delegate conversion/identity. Native
  class registration alone does not establish the entire original API surface.
- Port broader original drag/capture/drop/float/policy/adorner geometry/close/reentry/
  external-surface/selector/pinned/container/accessibility and factory cleanup scenarios.
  Existing compiled paths and limited native tests do not establish those behaviors.

Keep source strong owners/callbacks until explicit detach/dispose, original repeated
getter/read/event ordering, direct visual-root casts and source quirks. Do not replace
CLR GC cycles with weak edges. Remaining CWT ephemeron, native canonical weak-interface
expiration, arbitrary delegate.Target/method equality, alternate/custom model metadata,
IList Item versus Item[] notifications, serializer and desktop/platform gaps remain open.
Native/model boxed values use to_recycling_value(UI value) and
from_recycling_value(&model value); these directions were corrected during integration.
ControlTheme native handles are Ref, TopLevel::get_top_level takes Option<&Visual>, and
attached property identities compare their underlying as_property().

## Historical record (superseded by the current integration above)

The following earlier staging and test-count statements describe previous checkpoints.
Do not recreate removed staging sources or treat earlier NEXT statements as current.

# Remaining work

The full port remains active. Existing tests establish only the implemented
foundation, reactive models and dock containers, navigation, MDI layout, host
lifecycle, busy service and workspace snapshots/dirty tracking. A docking UI
and complete engine/API parity remain unverified.

Next implementation path:

LATEST HOST/CHROME/PINNED PASS (supersedes earlier NEXT statements):

Full initial source classes are now staged outside Cargo:
- controls/host_window.rs, internal/host_window_state.rs (with WindowDragContext), and
  automation/peers/host_window_automation_peer.rs.
- controls/tool_chrome_control.rs and automation/peers/tool_chrome_control_automation_peer.rs.
- controls/tool_pin_item_control.rs, controls/tool_pinned_control.rs, and both source peers.
- internal/dock_helpers.rs (ONLY remaining concrete methods; MERGE into compiled helper)
  and controls/managed_window_layer.rs (ONLY TryGetLayer; MERGE into actual compiled class).

All 26 staged Rust files are syntax-formatted only, NOT type-checked/validated; no new
runtime member is complete. Current runtime tests: 1,308 passed, zero failed/ignored;
UI crate 317. Strict all-target/release UI-library Clippy/fmt/inventory/generators/six Python
checks pass. Inventory: 210 ported, 253 partial, 1,465 pending of 1,928. Full gate remains red.
One new domain test covers IHostWindowCloseState default native receiver lookup, reference
identity, live flag and strong view lifetime. Its new hidden boxed_interface hook preserves
UI independence; PartialEq enables native property/interface values. Actual new native host
class/casts are uncompiled. Existing C# and =0.3.0 dependencies unchanged; no agents.

HostWindow is the ACTUAL full native Window source translation, not a marker or fallback.
It retains source strong constructor callbacks, host/state and peer/owner edges. OnceCell
installs state after native Ref allocation; temporary constructor options are released into
the real DockManager. Foreign IHostWindow/IHostWindowCloseState factories share a weak cached
native adapter whose live handle owns Ref<HostWindow>. Source weak/interface/CLR-GC cycle
observations remain pending. Native IHostWindowCloseState also registers object recovery
through its UI-independent boxed hook. Do not implement domain interfaces directly on native
class structs/Refs; use the supported ferro_class_info foreign factories and adapters.

Host preserves original cancelable/closed ordering: snapshots closing model/factory/policy,
marks closed, releases model presenters, removes current logical layout, cancels external
drag, then base Closed handlers, then preserve/non-preserve cleanup. Preserve guard checks
whether the original model still points at this native host after subscribers could install
a replacement. Factoryless Exit invokes model OnClose once; a factory uses closing policy.
Native move failure cancels and rethrows its native unwind. Reentry and original lifecycle
scenarios remain untested. Public Rust overload suffixes set_position_xy,
set_dock_window_state and set_host_title distinguish source host operations from inherited
UI setters; metadata keeps source method names. GetPosition/GetSize tuple/out metadata and
all class invokers need validation. Current source runtime branch is translated; legacy
version-11 conditional titlebar branch is not included here.

Normal geometry retains all original NaN/infinity/zero/partial-dimension behavior and source
assignment/event ordering. Magnetism snapshots one original rect, walks original factory
hosts and uses original last matching axis updates/truncating casts. Do not invent nearest
snap, effective-visibility changes or new geometry guards. Native host drag context initially
has DragAction::NONE; Start/End set MOVE. Managed context source explicitly initializes MOVE.
Validation uses FocusedDockable; Drop uses ActiveDockable. Drop true ignores manager Execute
return as source. WindowChrome scope uses external-only hit testing. Preserve original screen
sum, threshold against WindowStartPosition, float fallback and capture-loss ordering. Debug
arguments are cfg-elided, but source local control-name read remains outside the debug call.

Tool chrome preserves TitleProprty typo and original string-null default (Option<String>),
constructor pseudo classes, template/detach event order, active-model focus and strong source
helper/callback receivers. It uses existing ToolMenuLifetime and complete staged original
WindowDragHelper. Do not deduplicate AttachToWindow or enable the #if-false ordinary-window
branch; detach does NOT reset floating/grip/close fields in source. Source maximize toggles
native host state; no invented IsMaximized synchronization. Host button token metadata uses
weak native button keys and native weak-receiver disposables, while the actual click handler
captures host strongly (source button->host edge). Removing last matching subscription mirrors
source delegate subtraction; actual duplicate/lifetime/reentry tests remain required.

Pinned source controls preserve original engine calls and strong closures. Optional start
args from native ItemDragHelper use expect on impossible source null, not a new zero fallback.
ToolPinItem does NOT capture pointer. ToolPinned container orientation is set only during
create/prepare; no new live propagation. Peers keep readonly original owners STRONGLY and
preserve roles/name/help/IDs/invoke/menu/selection logic with live nested index loops.
All class/default/property/metadata/handler/peer behavior remains uncompiled/unvalidated.

Remaining DockHelpers extension implements source visual host-mode/owner/external search,
managed Z and repeatable ordering. ResolveDockControl checks visual ANCESTORS first (not self),
then external interfaces from control/visual parents. Native real interface casts use shared
ValueTypes; no guessed names/markers. GetZOrderedDockControls returns Ref<DockControl>, not
Rc<dyn IDockControl>; staged main transfer caller is repaired accordingly. Windows snapshot
and native sort happen eagerly at call; each subsequent query enumeration reads the original
list, computes tuple keys, stable-sorts, maps lazily (no extra final Vec). Tests for snapshot/
repeatability/custom equality/reentry and platform Z order still pending. Managed layer lookup
uses self-or-ancestor DockControl descendants then original visual-root descendants; direct
root behavior must remain (prior C# probe proved TopLevelHost in both runtimes).
TopLevel::get_top_level takes Option<&Visual>; staged callers now use Some(...). DragAction
is flags: uppercase NONE/MOVE/COPY/LINK repaired throughout. Other native API/borrow/metadata
errors can remain until the real group enters Cargo.

NEXT: full actual DocumentTabStrip/DocumentTabStripItem and ToolTabStrip/ToolTabStripItem,
plus their source peers. Source class sizes: 683/202/330/154 C# lines. They are the remaining
immediate control types referenced by staged main state and hosts. Then register all classes/
contracts, MERGE extension bodies and staged class/context continuations into compiled modules,
compile/repair the whole group and port original routed-input/host/drag/adorner/factory/selector/
external surface/pinned/peer tests. Complete source themes and the original Rust sample.
Do not keep testing only helper subsets or replace actual control/host classes with markers.
There is still NO runnable Rust sample. Goal stays active.

The previous main-control pass remains relevant below.

LATEST (supersedes older NEXT statements): full initial DockControl source class is now
staged at staging/controls/dock_control.rs, plus IExternalDockSurface and the source
DockControlAutomationPeer. Native class is NOT compiled, and no staged member is complete.
The staged DockControlAutomationPeer retains the original readonly owner STRONGLY in its
own Ref<DockControl>, even though the shared native base is weak. Preserve that source edge;
actual owner/cache/CLR-GC cycle behavior remains unverified.

Independent original static methods compile at src/Dock/FerroDock.Ferro/controls/dock_control.rs:
MatchesSelectorGesture, ToDragAction (modifier input extracted), CollectDockables/AddDockables
and title-comparer continuation. The stage imports that same module intentionally: MERGE
rather than replace the compiled functions when the class can enter Cargo.

Nine tests added. Current validation: 1,307 Rust tests, 317 UI, zero failed/ignored; strict
all-target/release UI-library Clippy/fmt/inventory/generators/six Python tests pass. Inventory
210 ported / 244 partial / 1,474 pending / 1,928. Full gate remains red; no runnable sample.

The stage now implements the four factory callback methods requested in older notes. Weak
metadata tracks only library-created callbacks; those closures capture native Ref<DockControl>
STRONGLY. No return-value/closure invocation is used for receiver identity. Arbitrary custom
managed delegate.Target equality, HostWindowFactory property wrapper equality and markup
callback conversion remain unverified. Full native class uses actual ferro_class_info!
foreign interface factories and a weak canonical DockControlHandle cache, retaining actual
native Ref in live handles. Selector interface now has a compiled default as_object hook:
native receiver equality/shared ValueTypes conversion is Headless-tested on independent
adapters; plain services retain address equality. IDockControl canonical/native weak-handle
observations still need tests on the real main class.

The stage external registry uses weak NATIVE receiver references (plus real interface query
on upgrade) for native surfaces, Weak<Rc trait> for plain custom surfaces. Do not replace
this with Weak<temporary adapter>; the adapter could expire while its native source lives.
Actual declaring surface classes must register IExternalDockSurface in ferro_class_info!
and expose as_object. Lazy iterator prunes on first MoveNext, then source list version guards
apply. Reentry and native/CLR weak-reference observations remain unvalidated. Source callback
receivers (constructor routed input, factory and attached window) stay strong until original
explicit detach/removal. No helper Drop cleanup or source-edge weakening is introduced.

Five main static-method tests use real original models to verify all traversal scopes,
source order, identity/cycles/retention and ignored PinnedDock content. Source windows and
visible/pane getters are re-read in original locations. Shared System Dictionary supplies
add-only visited identities because no shared HashSet exists. Gesture extras use raw bits
and from_bits_retain: bitflags ! would hide unknown modifier bits that source rejects.
DragAction is FLAGS: use uppercase NONE/MOVE/COPY/LINK. Staged main's native API spellings
were repaired where checked, but entire class remains uncompiled; more errors can remain.

scripts/dock/selector-reference captures 21 actual .NET 10 title comparison signs in
 tests/Dock/data/selector-title-order.tsv. Existing workspace simple-uppercase scalar map is
copied into controls/dock_control_ordinal_data.rs, not a custom framework StringComparer.
Use scalar comparison, NOT UTF-16 ordinal sorting: supplementary/BMP order differs in
original OrdinalIgnoreCase too. No full case expansion. Unpaired UTF-16 strings are not
representable in Rust; wider original Unicode/runtime ordering remains unverified.
Three original diagnostic event tests now compile through the shared System Event. Static
native event is UI-thread scoped; process-wide CLR/cross-thread/GC/event encapsulation and
metadata are not claimed equivalent. Actual control diagnostic hit testing remains staged.

The FULL initial instance WindowDragHelper now lives in staging/internal/window_drag_helper.rs,
syntax-formatted only and uncompiled/unvalidated, with no instance member complete. MERGE its
state/impl with the existing compiled IsChildOfType, replacing the unit holder, not that tested
static method. Strong owner/callback/self references follow source. Shared routed disposables
hold their target weakly, so the staged RetainedSubscription adds the original strong receiver
until explicit Dispose (no Drop cleanup). Verify disposal/receiver retention/reentry on actual
native windows. Source position-handler removal slot is declared but never installed: do not
invent a position subscription. Source release retains last press args; a new accepted left
press cancels stale move state BEFORE can-start evaluation. Below-threshold lost button clears
only original flags/args. Ordinary native-window branch intentionally does not set drag_window
or is_dragging; its release early-return/handled semantics must remain. A declined HostWindow
start leaves cached args/scope, as source does. Native PointerPressedEventArgs is already a
shared-event handle when cloned, NOT Rc<PointerPressedEventArgs>. WindowDragDockScope already
exists in ferrodock_settings (defined in original combined DockSettings.cs).

NEXT: actual native HostWindow/HostWindowState, integrate the full WindowDragHelper, then real
DocumentTabStrip/Item, ToolTabStrip/Item, ToolPinItemControl, ToolChromeControl and related
pinned/Dockable consumers. Their concrete APIs are required by the full staged main/state/
managed-host/adorner group. Continue that source dependency group, then compile and repair
it together, register all native classes/contracts, finish source DockHelpers/layer lookups,
merge preview Show and run original full main-control/host/drag/adorner/factory/selector
scenarios. Complete original themes and original Rust sample after real integration. Do
not keep accumulating only small helper subsets or invent a marker/reduced DockControl.

The preceding pass remains relevant below.


Latest pass: original DockDragContext now compiles in internal/dock_control_state.rs.
All fields/defaults/Start/End/ClearResolvedOperation/CopyTo preserve exact native references
and cached DragOffset (NOT reset by Start/End). Three Headless tests cover every field,
self-copy and strong retention through transfer until explicit End. Extracted original Drop
screen rounding and GetPreferredPreviewSize max-zero expressions compile there too; a fourth
test matches 17 .NET 10 cases in tests/Dock/data/drag-state-numeric.tsv. The test-only
scripts/dock/drag-state-reference probe checks expressions only, not the full state class.
.NET 10 NaN-to-int is 0, overflow saturates, rounding is ties-to-even, Max(0,-0) is positive
zero and NaN payload/sign are retained. Do not use f64::max, which changes NaN behavior.

Locked workspace: 1,298 Rust tests (308 UI), zero failed/ignored. Strict all-target/release
UI-library Clippy, fmt, inventory, factory generator and six Python checks pass. Inventory:
210 ported, 240 partial, 1,478 pending of 1,928. Complete-port gate still rejects unfinished
work. Original C# and =0.3.0 dependencies unchanged; no agents without authorization.

STAGED OUTSIDE CARGO, syntax-formatted only and no completed-member claims:

- staging/internal/dock_control_state.rs: initial FULL original DockControlState and operation
  resolution record. Includes constructor/calculator, capability/preview sizing, StartDrag,
  cached placement, ownership chain/HasActiveDrag/transfer, Enter/Over/resolution/cache, every
  Drop/global/local/float branch, validation/visibility/Execute and all Process/debug phases.
  MERGE into the compiled matching module when dependencies exist; its context and numeric
  helpers are already there. Imports currently refer to that matching module on purpose.
- staging/contract/i_drag_offset_calculator.rs and internal/default_drag_offset_calculator.rs:
  original screen-corner offset versus PART_Grip zero offset, requiring real DockControl.
- staging/services/i_dock_control_factory_service.rs and dock_control_factory_service.rs:
  full original recycling/cache/default-locator lifecycle branches, requiring real control
  and callback receiver identity support.

State source details to preserve on integration: BeginDrag does not clear cached offset or
adorners; HasResolvedOperation depends on selected != None, not validity; release uses the
cached selected operation before any fresh resolution. Source Drop ignores Execute's manager
return. Leave clears resolution then removes adorners. Transfer performs Leave plus an extra
clear, copies all fields before ending source, retains a STRONG target link, and makes the
new control flag match the transferred DoDragDrop. Ordinary release/capture/disable do not
clear _activeDockControl or _transferTarget. ResolveCurrentOwner has the original self-link
stop, not a new general cycle guard. Debug call arguments remain cfg-excluded in release;
source preview-size geometry reads outside Debug remain present. Root casts remain direct:
the prior pinned C# reference already confirmed TopLevelHost in original Headless too.
No source state input/capture/owner-handoff test is claimed yet; the subclass in the original
DockControlStateTests merely exposes protected Execute, not a new override.

Factory service staging preserves same-layout guard (it does NOT exclude owner itself in
that helper), eight dictionary prune calls in source order, snapshots before removal and
all hidden/pinned/windows/visible/split/custom dock-window traversal. Shared System has no
HashSet: add-only visited identities use existing Dictionary to retain references/insertion
slots; no model collection is mirrored. Rc values leave dictionary borrows before drop.
Control-recycling class cast uses shared ValueTypes interface-object metadata, not private
handle downcasts. Weak factory keys retain strong cache values; ephemeron/value-to-key cycles,
TLS/process scope and custom CLR equality remain unresolved. Duplicate map Add uses Entry so
it throws without replacing an existing value, including setter callback reentry.

NEXT must focus on the ACTUAL main DockControl source class and its native HostWindow/State/
WindowDragHelper/tab/pinned dependencies, then compile/repair the staged group. No marker
control or reduced state/host fallback. Implement the four receiver-aware callback methods
used by the staged factory service: default_context_callback, default_host_window_callback,
owns_context_callback, owns_host_callback. They must identify original delegate.Target owner
without invoking callbacks or comparing results. Stored ContextFactory/HostWindowFactory are
currently Rc<dyn Fn> aliases. One viable native representation is metadata weak references to
library-created bound callbacks on the real control while those callbacks capture the owner
STRONGLY. Weak metadata does not replace the actual strong callback edge. Do not store strong
self-capturing callbacks on the control solely as an identity cache; avoid inventing a new
self-cycle. Opaque/covariant/custom delegate receiver semantics still require verification.
Native UI/interface conversions need supported ferro_class_info interface factories and
actual Ref identity; canonical/weak observations across interface queries must be tested.
HostWindowFactory styled-function representation needs native property-value equality while
preserving its public callback API; the plain Rc<dyn Fn> alias is not PartialEq.

Then finish concrete DockHelpers/layer lookup, merge preview Show/discovery, source themes,
full routed/managed/adorner/factory cleanup scenarios and original Rust sample. No runnable
Rust sample exists yet. Goal remains active.

The preceding preview pass remains relevant below:

The latest pass compiles the original DragPreviewHelper mechanics at
src/Dock/FerroDock.Ferro/internal/drag_preview_helper.rs. Public Show and TryGetManagedLayer ONLY
are staged at staging/internal/drag_preview_helper.rs; MERGE that impl into the compiled
helper once actual DockControl/layer lookup exists (other methods remain module-private).
The floating continuation of Show is extracted into show_floating without a replacement
host path. All other source methods/static defaults compile and are tested on real native
windows/controls/layers, the actual dispatcher and production original model identities.

Seventeen new native tests bring the locked workspace total to 1,294 (304 UI), zero failed/
ignored. Strict all-target/release UI-library Clippy, fmt, inventory, factory generator and
six Python checks pass. Inventory: 210 ported, 235 partial, 1,483 pending of 1,928. Complete
port gate remains red. C# source and =0.3.0 dependencies unchanged; no agents without approval.

Tests preserve the two source NativePreview brush cases and original abandoned-freeze case,
plus native size/owner/managed defaults, signed-zero opacity, 14 templates and partial-add
failure/retry, original floating reuse/offset/premeasure/partial preferred size, managed
measurement/chrome-overage/private Build and actual source model references. Move retains
original macOS-only coalescing, duplicate suppression, epoch/sequence guards, reentrant
pending overwrite (do not "fix" the source lost nested move), unguarded freeze posts and
four-retry abandonment. Real zero-size visible-window retries and Headless scaling changes
exercise pixel-size retention, same-scale early return and NaN skip behavior. Hide's managed
branch returns while an existing floating window remains; preserve this original branch.
No helper-instance Drop cleanup is introduced. No borrow spans native/model callbacks.

IMPORTANT REFERENCE RESULT supersedes the earlier presumed native root difference:
scripts/dock/preview-root-reference compiles the EXACT original GetManagedPosition body and the
actual VisualRootCompatExtensions against source-pinned Avalonia 12.1.3 Headless. The original
extension uses GetPresentationSource()?.RootVisual and returns TopLevelHost too. Four committed
TSV cases (detached, attached render scale 1/2, hidden) match the native helper, including
(0,0) from the direct TopLevel cast. Preserve this observed source behavior. Do NOT change it
into ancestor/top-level-owner lookup based on the assumption C# returns Window. The probe
uses a Visual parameter alias because this method reads only base Visual APIs, with a real
Border coordinate surface; it does not compile the whole helper or prove public Show/layer
lookup, positive TopLevel-root or desktop/mixed-DPI behavior. generate.py --check verifies
the copied body. Normal Rust tests use tests/Dock/data/preview-root.tsv without .NET.

Native preview state uses UI-thread Rc/Cells/RefCells to permit original reentry with no
borrow over callbacks. Process-wide Monitor/cross-thread/CLR GC parity remains open. Debug
messages use native debug stderr; size-mode names come from shared native enum metadata,
but ambient-culture/debugger transport/unknown runtime cases are not claimed equivalent.

NEXT: actual DockControl/DockControlState and their full native HostWindow/State/window-drag,
factory service, tab/pinned consumers. Finish DockHelpers concrete visual/external/Z-order
members and layer TryGetLayer with actual classes. Then merge preview Show/discovery and
integrate the staged manager/managed-host/state/adorner group, repair APIs and run original
public-entry managed preview rejection/context/registry/overlay and host/drag/adorner tests.
The staged adorner's MANUAL flag spelling is corrected from the published API; it remains
otherwise uncompiled. Complete source themes and original Rust sample remain required.
There is still no runnable Rust sample. Goal stays active.

The preceding target/host-state pass remains relevant below:

The latest pass adds native same-reference local/global target queries and runtime model
names required by the original state engine. Default marker queries use actual existing
IDocument/ITool/IDocumentDock/IToolDock/IProportionalDock supertraits, not name/kind guesses.
Original concrete, routable and managed document classes expose direct views. Composed
models using existing subtype queries inherit the correct marker. Marker-only custom models
supply their explicit query alongside the marker impl. No stored marker flag or UI-domain
coupling. Eight tests cover positive/negative/both roles, inherited global/tool-only views,
outer identity, query retention/release and concrete diagnostic names.

Locked workspace validation: 1,277 Rust tests (287 UI), zero failed/ignored. Strict all-target
Clippy, release UI-library Clippy, format, inventory, factory generator and six Python checks
pass. Inventory: 210 ported, 234 partial, 1,484 pending of 1,928 inputs. The full completion gate
remains red. Source C# and published dependencies are unchanged; no agents without approval.

The initial source dependency group is STAGED OUTSIDE CARGO:

- staging/internal/dock_manager_state.rs: original base validation/adorners/execute/float and
  managed coordinate/content-offset/layer/root algorithms.
- staging/internal/services/global_docking_service.rs: original preset target resolution,
  outermost actual global target, operation precedence and repeated-owner proportion writes.
- staging/controls/managed_host_window.rs: original host fields/construction, presentation,
  full close cancellation/reentry/preserve-layout path, release, geometry/state/title/layout/
  activation, private dispatch and actual source DockControl document content construction.
- staging/internal/managed_host_window_state.rs: original ManagedWindowDragContext and drag
  phases/threshold/target search/preview/drop/floating/validation/managed layer/offset bodies.

The original IGlobalDockingService declaration is compiled in internal/services; concrete
implementation and state consumers remain staged. The namespace has a temporary dead-code
allowance in all configurations because those callers are excluded from Cargo.

All staged files are syntax-formatted ONLY, not type-checked or behaviorally validated; no
member is claimed complete. Native API errors/reentry bugs can remain. No marker control,
reduced engine/lookup or feature-gated stub is used. Preserve source host/state mutual STRONG
ownership: OnceCell permits post-allocation initialization of that edge; runtime Weak<Self>
only recovers self identity. Do not weaken the state -> host edge to hide Rc/CLR GC cycles.
Native singleton TLS versus source process-wide state and CLR generic/alias diagnostic names
remain unverified. Read staging/README.md and the original source beside every staged file.

NEXT: translate actual DockControl/DockControlState and DragPreviewHelper; integrate the
complete native HostWindow/State/window-drag and tab/pinned dependencies they reference.
Then move the staged group into matching source modules, repair native APIs and run original
global target, adorner geometry/lifecycle, managed close/reentry/preserve-layout and drag
scenarios. DockHelpers still needs concrete visual/external/Z-order members; layer TryGetLayer
needs actual DockControl. Do not replace those with simplified lookups. Complete original
source themes and original sample remain required. There is still no runnable Rust sample;
the full porting goal stays active.

Native interface integration evidence for the next control pass: published
ferroui-base-0.3.0/type_system.rs defines Ref<T> over a PRIVATE Rc<dyn Any>, with native
cast/upcast/ptr_eq/downgrade APIs but no public Rc<T> extraction. ferro_class_info! explicitly
supports foreign interface factories (`interfaces: [Rc<dyn Trait> => function]`). Published
WindowNotificationManager uses ManagerHandle(Ref<WindowNotificationManager>) to expose its
foreign interface. Use that supported native registration path for actual DockControl and
HostWindow views. A freshly allocated Rc wrapper is NOT the native object's identity:
verify source factory-list membership, ReferenceEquals, weak observations and cleanup across
repeated interface queries. Do not silently compare wrapper allocations or weaken source
ownership/callback edges. Delegate .Target identity in factory default-locator cleanup also
needs faithful native representation. These are integration requirements, not proven blockers.
Visual.FindAncestorOfType's published native overload requires include_self explicitly;
source calls default to false (the new staged calls now pass false).

The preceding helper/layer pass remains relevant below:

The latest pass ports the original DockHelpers model-resolution/hit-test/screen/diagnostic/
ShowWindows/proportional-search members and all GlobalDockingHelper source bodies. Fourteen
new tests bring the locked workspace total to 1,269 (286 in the UI crate). Strict all-target
Clippy, release UI-library Clippy, format, inventory, factory generator and six Python tests
pass. Inventory: 209 ported, 230 partial, 1,489 pending of 1,928 inputs. The three original
DockHelpersTests assertion sets use real native windows and production ReactiveUI models;
the alternate framework-property fixtures remain pending.

Hit testing retains raw-before-filtered calls, raw fallback ONLY when no filtered Control
exists, explicit IsSet/value and effective visibility/root rules, and hidden-window retained
tree behavior. Native panic messages retain an empty stack entry; CLR stack/inner exception
transport is absent. Debug call arguments are removed in release, but LogDropAreas still
gathers/reads controls when diagnostics is enabled. Screen conversion uses actual screen
scaling, then top-level scaling, then 1; mixed-DPI desktop comparison remains open. Source
model host resolution reads Owner once in the repeated pattern and again in the separate
factory branch. scripts/dock/helper-reference reproduces that compiler behavior for root,
unrelated and null owners; it is not a full original helper/runtime comparison.

GlobalDockingHelper preserves the returned directional tuple, wrapping int counters,
splitter/single-child-wrapper exclusions, direct proportional-child recursion, repeated
live list reads, threshold exits and custom enumerator disposal. Original manager callers
are pending; production dead-code allowances are scoped to the new helper modules.

AdornerHelper is staged at staging/internal/adorner_helper.rs, OUTSIDE Cargo. It is only
syntax-formatted, not type-checked/tested, and no member is claimed complete. Do not add
marker hosts or simplified lookups to compile it. Move it into the matching source module
after real DockHelpers VISUAL host-mode resolution and ManagedWindowLayer.TryGetLayer exist.
Then port original AdornerGeometry/Lifecycle scenarios with actual target/floating/managed
windows. Review reentrant callbacks/live field reads, native logical/adorned-parent APIs,
subscriptions and top-level roots. Preserve source strong captures until explicit detach.

NEXT remains the actual DockControl/State/ManagerState/factory-service and ManagedHostWindow/
State group. Missing DockHelpers members: visual host-mode overloads, ResolveDockControl,
GetControlIncludingExternal/GetExternalControl, IndexOf/GetManagedWindowZOrder and
GetZOrderedDockControls. They require actual DockControl/external-surface contracts. Then
finish tracking/DockableControl/tab/pinned controls and complete original MdiDocumentWindow/
global themes into the original sample. No usable Rust sample exists yet. Original C# and
dependencies are unchanged. No agents without authorization. The goal remains active.

The preceding managed-layer pass remains relevant below:

The latest pass adds the ACTUAL ManagedWindowLayer native TemplatedControl, the original
ManagedWindowRegistry operations and the COMPLETE source ManagedWindowLayer dictionary.
All original layer fields/properties/template/dock/list/window subscription, overlay,
content-offset and topmost ordering bodies are present EXCEPT TryGetLayer(Visual). That
static ancestor/root search is deliberately absent until the actual DockControl exists;
no marker control or reduced lookup substitutes for it. The registry preserves all source
same-layer/replacement/unregister branches, original dock defaults/identity and visibility.

The layer forwards the original typed source list events for Add/Remove/Replace/Reset;
Move preserves subscriptions and still reorders. No mirror list or reset approximation.
Source callbacks remain strong until detach. Native System Dictionary preserves keyed
window subscription and overlay order. Replaced source lists/windows detach their original
tokens. Topmost ordering retains the normal/topmost list instances passed to a custom
manager, computes the offset from their Count AFTER both callbacks, and enumerates the
same topmost list. Tests cover custom list mutation and reentry into Dock replacement
without modifying the original outer snapshot/order.

Overlay keys retain the original control; a replacement argument is ignored. Missing canvas
returns without fabricating one. Null size writes NaN. Template replacement retains source
overlay dictionary entries without migrating the old controls, and Hide removes only from
the current canvas. Content offsets choose the first visible native MDI window, cache only
successful measurements until OnApplyTemplate, and otherwise create the original hidden
OffsetProbeDockable/MdiDocumentWindow probe. Probe overlay cleanup, failure retry and success
caching are tested. Test-only content-part templates exercise measurement; they are NOT a
replacement production MDI window theme.

The complete layer dictionary creates actual ItemsControl/MdiLayoutPanel/MdiDocumentWindow
frames for original ManagedDockWindowDocument objects. Two downstream compiled cases verify
source model identity, list add/remove/rebind/stale-source exclusion, live topmost/MdiState/
ZIndex, original canvas ZIndex=10000 and overlay geometry/removal. The consumer binds Dock
from an explicit outer element because the source layer owns its own DataContext. Native
property conversion does not traverse metadata assignability's reference/nullable casts:
managed_model_registration now states direct conversions from original raw/Rc managed dock
values to nullable IDock through shared ValueTypes. Raw, handle and nullable handle identity
are tested; no private binding engine is introduced.

REGISTRY LIFETIME GAP: shared System 0.3.0 has no ConditionalWeakTable/ephemeron GC. The
native registry retains original strong values and ManagedWindowDock.Factory remains
STRONG, exactly as the source model. Thus normal cached docks retain their factory keys
through value-to-key cycles; access pruning alone cannot collect those cycles. Tests
explicitly characterize that retained key, then clear Factory ONLY in the noncyclic-pruning
fixture before verifying expiry/pruning. Do not weaken Factory, values or source event
captures to make the leak disappear. Native thread-local versus CLR process-wide identity,
GC timing/forced-GC tests and full runtime lifetime parity remain partial.

This pass adds 18 tests (16 layer/registry/conversion/callback native tests and two compiled
source-theme consumers). Locked workspace validation passes 1,255 Rust tests, zero failed/
ignored. Strict all-target Clippy, formatting, structural inventory, factory generator and
six Python tests pass. Inventory: 208 ported, 227 partial and 1,493 pending of 1,928 source
inputs. Complete-port gate remains red. The known nonfatal macOS compact-unwind warning is
unchanged. Original C# and published =0.3.0 dependencies are unchanged; do not touch the dirty
sibling FerroUI checkout. No agents unless explicitly authorized. The goal remains active.

Earlier passes supply all own IDock/IDocumentDock/IToolDock/policy/IRootDock native metadata,
source capability/removal/close-visibility converters, 47 factory COMMAND overloads/two
close flags, source enum descriptors, original live model ItemsSource and the COMPLETE MDI
host dictionary. Factory non-command creator/search/collection/locator/tracking/property/
event/generic metadata remains pending. Native source item views retain original storage/
identity/typed events; counts-only custom notifying adapters need their original typed hooks.

KNOWN BINDING GAP unchanged: compiled IList integer indexers listen for Item, while System
ObservableCollection emits Item[]. The old indexed item remains subscribed until list-path
replacement; dock_model consumer explicitly characterizes it. Do not falsify IList metadata
with INCC, rename source notifications or add a private accessor. No DockableList wrapper
remains. Generic IList/CopyTo/erased discovery/version and original desktop comparison remain
open. Managed window template matching retains explicit checked root/other opaque aliases.

NEXT: continue the original DockControl/State/ManagerState/factory-service and ManagedHostWindow/
State group. The real managed layer/registry can now serve their overlay/window-list needs;
finish Layer.TryGetLayer with the actual DockControl search. Port DockableControl together
with Internal/DockHelpers.cs and Controls/ToolPinItemControl.axaml.cs/ToolPinnedControl.axaml.cs.
DockHelpers.GetScreenPoint needs original mixed-screen scaling, host-mode resolution needs
actual DockControl, pinned bounds exclusion needs actual ToolPinItemControl, and drag-out
must invoke the real state StartDrag/Process. Do not use marker controls or reduced tracking.
Then compile COMPLETE MdiDocumentWindow dictionary with context menus/commands, DockableControl,
deferred/recycling content and all buttons/resize parts. Remaining global ChromeButton/
DockFluentTheme, adorner/preview/window-drag/tracking, full tab/pinned integration and other
model/factory metadata still precede the original Rust sample. Preserve source tunnel
CaptureLost and native Window/TopLevelHost observations. No runnable sample is claimed.

The preceding pass adds the actual MdiDocumentWindow native class and automation peer,
all source properties/fields and full input/state/subscription/layout/drag/resize bodies.
Sixteen new Headless tests use the real Factory, capability resolver and MDI layout manager:
original header/content activation, factory focus and Z order, all eight resize directions,
custom layout managers, drag guards/cancellation/managed notifications, coordinate/screen
dispatch, normal release, minimize/restore/double-tap, magnetism and minimized-strip clamp,
template replacement/content offset, live automation/invoke/focus fallback, rebind/detach
and native release with dependencies alive. Test-only part templates are not a substitute
for the complete source Fluent theme, which is still pending.

WindowActivationHelper now contains all original branches, identity deduplication, native
Z sort, visible guard, stable managed sort and final initiator activation. Host/control
class views use the existing boxed-interface query; IDockControl now has the optional
hidden query with a default None. The production concrete hosts and DockControl must
expose their real views when ported. ManagedHostWindowAccess now also requires
ManagedZIndex and ProcessDrag; only in-crate probes implement it until the actual host
exists. EventType retains all seven numeric phases/default. WindowDragHelper currently
contains only original IsChildOfType<T>; its other move-drag members are still pending.

The source PointerCaptureLost handler is registered for Tunnel, while native capture
loss raises a Direct event. Native tests confirm the subscription does not observe that
route. Do not silently change it to Direct or add a class override: normal release is
verified, and the original capture-loss handler body is separately exercised. The exact
original desktop/runtime comparison remains open. Native double-tap itself bubbles and
therefore toggles once despite the source requesting both Tunnel and Bubble. Native
Window/TopLevelHost root differences remain explicit for activation-helper child discovery.

The existing source-driven foreign-enum generator now exports original MdiWindowState
metadata and nullable handles without adding UI to Domain. Runtime automation names and
downstream compiled two-way MdiState/template property binding are verified using the
original managed model. No published framework packages or original C# files changed.

Latest validation: 1,188 Rust tests passed, zero failed/ignored. Strict locked all-target
Clippy, formatting, structural inventory, factory generator and six Python tests pass.
Inventory: 209 ported, 211 partial and 1,508 pending of 1,928 inputs. Full completion still
fails and there is still no runnable Rust docking sample. The known nonfatal macOS
compact-unwind linker warning remains unchanged.

Earlier next-step plan, superseded by the MDI host pass above: continue the remaining
metadata and DockableControl/tracking consumers so the COMPLETE original MDI dictionaries
can be compiled. Then connect ManagedWindowLayer/Registry and the full host/controller,
adorner/preview/factory group. ManagedWindowRegistry lives under Internal, not Controls.
MdiDocumentWindow is now real but its full original theme is not present. Do not replace
the theme with the small test templates. Preserve source menu/capability/tool headers,
all commands, deferred/recycling body, resize edges and every pseudo-state selector.
Concrete ManagedHostWindow must expose the original private access view and native
HostWindow/DockControl must expose their existing native class views. Full DockControl,
HostWindow, window-drag helper, tab/pinned consumers and real samples still follow.

The preceding pass ports ManagedDockWindowDocument and ManagedWindowDock with every
original model/content/subscription/activation branch present. Fifteen new native
tests cover source defaults, nullable/reference members, title/focus/layout changes
and stale-source detach, full-screen state guard, ordinary/managed close and geometry
branches, explicit Dispose retention/release with dependencies alive, all supported
parent types, unsupported-parent fallback order, template/delegate/self-return guards,
preview visual brushes, commands, add/remove/focus and real factory activation order.
The original private managed-host members use a UI-private access trait through the
existing boxed-interface query pattern; the concrete ManagedHostWindow is still pending.
The host probe is test-only and is not a production substitute.

The downstream compiled consumer uses the actual MdiDocumentWindow deferred/recycling
content fragment. It verifies inherited/two-way Title and own live MdiZIndex, compiled
deferred template materialization, canonical IDockable then IMdiDocument context rebinding,
and live notifications after each rebind. Native register_boxed_cast registrations recover
the same original model from interface handles, reject unrelated models and honor the
existing application boxed-interface query for user-owned recycling templates. UI-owned
IMdiDocument metadata exposes the original three fields and optional System notifier.
IDockWindow now has an optional notifier query; real/composed reactive windows expose
the actual enclosing event, so document lifecycle subscriptions work without reflection.
The new direct UI dependency is the already-pinned ferroui-markup-xaml =0.3.0 for shared
DataTemplate type matching and TemplateContent loading; no registry crates were edited.

Two published framework details need to remain explicit. The 0.3.0 typed-property probe
selects a nonnotifying specialization for these plain Rc fields; documented try_set
metadata routes mutable fields through the standard compiled INPC accessor. Original
setters remain unchanged. Static is_assignable also treats a registered conditional
interface cast as type-wide membership: this document Match checks the actual cast
for those opaque-interface targets. General native template matching on such aliases,
arbitrary CLR/runtime object projection and original framework-property model variants
remain partial. Do not claim complete API or native runtime parity from current tests.

Latest validation: 1,172 Rust tests passed, zero failed/ignored. Strict locked all-target
Clippy, formatting, structural inventory, factory generator and six Python tests pass.
Inventory: 208 ported, 204 partial and 1,516 pending of 1,928 source inputs. A disk-full
build was terminal; only FerroDock's rebuildable cargo artifacts were cleaned, recovering
space, and the full suite was rerun successfully. The known nonfatal compact-unwind
warning remains. No runnable Rust docking sample exists yet.

Earlier next-step plan, superseded by the MDI control pass above: finish the full MDI
theme and ManagedWindowLayer/Registry, adorner/preview and host/controller state group.
MdiDocumentWindow needs the newly ported document and original managed-host drag dispatch;
its source path includes .axaml.cs. Preserve all branches and use the existing host/state
contracts for native class membership/dispatch. The concrete managed host must expose the
private access view for LastCloseCanceled/UpdateBoundsFromDocument when connected; until
then only its original branch contract has a native test probe. Full DockControl,
HostWindow and tab/pinned consumers and actual samples still follow. Keep nested
ManagedDockWindowDocumentContent in ManagedHostWindow.cs, with original DockControl
creation and all initialization/managed-layer flags, when porting that host.

The newest pass adds the complete original PinnedDockWindow subclass, its automation
peer and full Fluent dictionary. The downstream compiled root-window consumer checks
all original setters, dimensions, fallback/title/explicit names, live state and invoke
on hidden/visible/hidden-again windows. A derived native window retains the original
style key and peer. Focus is tested with Focusable enabled: the source calls Focus()
but does not guarantee that a default nonfocusable window accepts focus. Desktop
activation/automation behavior remains unverified, and PinnedDockControl is still pending.

Both original tab-strip scroll helpers are now present. Five native Headless tests
cover null/no-range/edge behavior, fractional ceiling steps, the Double.Epsilon
subnormal threshold (not Rust f64::EPSILON), both axes/directions, event handling,
disposal/orientation replacement, detach/reattach and retained-viewer release. Actual
child routing verifies the native ScrollContentPresenter handles usable vertical wheel
movement before Dock's bubbling hook; horizontal-only overflow reaches the Dock hook.
Do not change this to a tunnel handler or force already-handled events through it.
The seven original DocumentTabStrip/ToolTabStrip template tests remain separately pending.
Original explicit disposal and strong event retention are preserved; unreachable Rc
cycles remain an open CLR lifetime difference.

Latest validation: 1,156 Rust tests, strict locked all-target Clippy, formatting,
structural inventory, factory generator and six Python tests pass. Inventory is
208 ported, 201 partial and 1,519 pending of 1,928 source inputs. The full completion
gate still fails as expected and there is still no runnable Rust docking sample.

The next main implementation is the mutually dependent host group, not a reduced
DockControl. Its source members include DockControl, DockControlState, DockManagerState,
DockControlFactoryService/contract, ManagedDockWindowDocument, ManagedWindowDock,
ManagedHostWindow and its state/context, ManagedWindowLayer/Registry, AdornerHelper,
DragPreviewHelper, MdiDocumentWindow, HostWindow and window-drag/activation helpers.
ManagedDockWindowDocumentContent is a nested class in ManagedHostWindow.cs, not a
separate source file; preserve its original DockControl creation/bindings. Existing
ManagedDockableBase supplies original POCO notifications and virtual callbacks.
The complete command-bar and selector dependencies are ready. Tab/pinned source
controls and actual samples follow this group; all floating/managed/geometry branches
must remain present as they are implemented.

The latest pass ports the original command-bar definition/item/provider contracts,
default adapter, full merge manager, native host/automation peer and complete Fluent
host dictionary. The manager preserves all original merge and active-document branches,
strong callbacks until explicit Detach, same-layout refresh semantics and original factory
subscription capture. Do not replace these with a separate command or merge engine.
The default adapter uses the published ReactiveCommandAdapter and boxed_parameter so
arbitrary model command parameters reach the original command object unchanged.

Fifteen new tests cover domain defaults/null/reference identity, per-kind replace
precedence, stable order, append targets/fallbacks, group insertion, shallow list copies,
active document/split-pane lookup, provider priority/context discovery, real factory and
provider events, enable/disable behavior, event cleanup and reentrant adapter detach.
The downstream compiled consumer renders the complete host theme with all three bar
kinds, executes an actual toolbar button via its native invoke provider and clears
nullable lists live. A direct lifetime counterpart verifies the manager is released
after Detach while dependencies remain alive. Strict CLR collection, the original full
host input/lifetime test and desktop command/automation behavior remain unverified.

Shared MarkupEvent metadata exposes add only and discards subscription return values.
The native metadata event is tested for sender/args/dispatch; direct subscription
helper disposal and original System Event token removal are tested separately. The
provider uses Event<()>; arbitrary CLR sender/EventArgs identity remains partial.
Context discovery accepts canonical provider Rc handles and native values/casts passed
through the existing recycling bridge. Automatic metadata for arbitrary raw domain Any
objects is still pending. Keep these gaps explicit instead of adding a private runtime.

Current validation is 1,150 Rust tests; inventory is 205 ported, 198 partial and 1,525
pending of 1,928 source inputs. Strict locked all-target Clippy, formatting, inventory,
factory generator and six Python tests pass. The Rust docking sample remains unavailable.
Next port the full managed document/dock/window/state/layer/registry/adorner group and
DockControl/factory-service dependencies, then original tab/pinned consumers and actual
sample. Preserve every managed/floating/transfer/geometry branch in the mutually dependent
group; do not introduce reduced stubs to produce a runnable demonstration.

The previous selector pass ports the complete DockSelectorOverlay, item/mode/service
contracts, four automation providers and full Fluent dictionary. Its compiled consumer
verifies title/badges, selection, observable mutations/replacement, nullable Items and
delegated scrolling. Its typed IReadOnlyList-to-ItemsSource projection is now shared
with command bars; from_markup_value and original Rc identity still handle both model
objects and boxed handles. Register model references before consumers; unresolved casts
in shared metadata can break unrelated bindings. All selector regression tests pass.

The published ItemsControlAutomationPeer exposes scroller discovery as a separate
scroller() hook, while get_provider returns a provider without invoking that hook.
Dock's peer preserves source delegation. The selector consumer explicitly initializes
the shared hook before verifying scrolling; automation-client discovery remains partial.
Full DockControl keyboard selector navigation and CLR string/ownership/equality,
observable fail-fast enumeration/version parity also remain pending.

The previous pass completes the original DragPreviewControl Fluent dictionary and
NotNullConverter bodies. Its real compiled consumer verifies live original-model
Title, status icon/text selectors, translated dynamic resources, both preview-content
branches, ShowContent/dimensions and the same recycled body across content switches.
UI-owned IDockable metadata exposes all 33 source properties and 18 methods (including
inherited recycling identity). Shared MarkupType notification discovery borrows the
original optional notifier from canonical Rc/nullable Rc handles. There is no new
model or binding runtime. A separate compiled consumer verifies two-way Title and
nested Owner.Id updates, replacement and stale-source detachment. Concrete nullable
casts cover all optional metadata property types; the inherited recycling-interface
cast retains the same Rc. Tests caught and fixed missing nullable factory conversion.
Validation is 1,128 Rust tests. Inventory is 200 ported, 186 partial and 1,542 pending
of 1,928 inputs.

The prior native DragPreviewControl/DragPreviewWindow and both automation peers retain
all original bodies; both complete preview Fluent dictionaries now compile. The old
small preview template remains explicitly a separate test fixture. NotNullConverter's
native UI-thread singleton and CLR exception/string/runtime differences remain partial.
DragPreviewHelper still depends on real ManagedDockWindowDocument/ManagedWindowLayer/
DockControl hosting, and all platform coalescing/size-freeze branches must be retained.
Do not implement a reduced helper that drops managed/floating/geometry paths.

The shared compiler resolves explicit imports for concrete cast declarations but did
not resolve IDockable through a glob import. That produced an unread cast and poisoned
unrelated consumer bindings. Keep all shared ValueTypes registrations concrete and
fully resolvable, and verify emitted .xamlmeta plus the complete compiled consumer set.
Register the enriched IDockable descriptor before consumer descriptors: the published
registry preserves the first registered handle, and the callback uses only canonical
Rc/nullable Rc handles to avoid recursively rediscovering itself.

An earlier pass supplied all fourteen original default templates, lazily and
repeatably, using shared compiled binding accessors. Their source grid cross-enum
conversion failure/native Auto direction remains deliberate (verified against 12.1.3).
Do not introduce a name/number mapping in DockDataTemplateHelper. Eleven native dock
container/content controls supply its target types; complete source themes and
full DockControl/default-template consumption remain pending.

Core IDockable now provides optional System INotifyPropertyChanged discovery; original
ReactiveUI and managed composition macros return the same original notifier. Native
canonical model handles retain their Rc identity and borrow this event; arbitrary raw
models use the published metadata notification hook and shared ValueTypes registration.
No replacement view model, event/binding engine or generic runtime registry was added.
Full raw/nullable generic/interface discovery, exact CLR error/lifetime/performance
parity and full DockControl/template/theme consumption remain explicit partials.

Next complete the original mutually dependent managed document/dock/host/layer/
registry classes and real state/drag/adorner dependencies, followed by original tab/
pinned consumers and complete themes/sample. ManagedHostWindow constructs
ManagedHostWindowState and a DockControl for content; ManagedDockWindowDocument
calls concrete managed-host close/bounds paths; ManagedWindowDock activates those
actual documents; managed state/layer/adorner code references DockControl. Port
that dependency group together or make further real prerequisite progress. Do not
invent reduced hosts/controls/interfaces or omit their managed/floating/transfer/
geometry branches to make an isolated class compile. Earlier host cleanup, preview,
pinned resolver, state, root and menu helpers have all source bodies, with remaining
runtime/metadata differences explicit in the inventory. Remove scoped production
dead-code allowances when original callers are ported; ToolDockControl.GetPanel
currently also has no caller in the source itself.
The user confirmed the order: dock targets and adorner stack, then tab and pinned
items using the original engine and API. When asked about launching the existing
C# reference sample, the user explicitly chose to CONTINUE TOWARD A RUNNABLE RUST
SAMPLE. Do not repeat that question or launch C# in place of the requested port.
There is still no Rust sample executable, full DockFluentTheme/App.axaml or usable
docking application. The separate markup consumer is a Headless integration test,
not the sample or a replacement docking UI. Continue the original source port.
Remaining themes, runtime metadata, dictionary enumeration version/removal semantics,
cross-thread/provider/repost, ownership/GC, serializers, alternate models,
diagnostics/samples, platform/performance and earlier requirements stay in scope.
Earlier pass notes below describe their state at the time; the latest notes and
ports.json govern current implemented/deferred members.

TrackingAdapter, DockPoint/DockRect runtime behavior, DockCapabilityEvaluation,
IDockSelectorInfo, IDockableDockingRestrictions, IDockingWindowState,
IHostWindowCloseState and both control recycling model interfaces are now present.
The core IDockable/IDock/IFactory/control/window interface graph is now declared,
with all 29 lifecycle event argument types, weak GlobalDockTrackingState,
DockWindowOptions, docking-group validation/inheritance, and capability resolution
for all defined capabilities. ReactiveBase, DockableBase, Document, Tool, both
splitters and NavigateAdapter now have implementations. DockBase and all ten
reactive containers (RootDock, DocumentDock, ToolDock, DockDock, StackDock,
WrapDock, GridDock, UniformGridDock, ProportionalDock and SplitViewDock) are also
present, with source navigation/reactive subscriptions, document creation and MDI
layout commands. DockWindow and HostAdapter now implement the source defaults,
properties and host lifecycle branches, with serialization and one late-null
layout edge still partial. FactoryBase now has locator/search algorithms,
repeatable Find queries, visible-list normalization and both InitDockWindow
overloads, active-selection initialization and docking state resolution/request
synchronization, recursive initialization, full focus transfer and global tracking.
All factory event bodies, lifecycle notifications and window/options methods are
now present. Collection add/insert/helpers, move/swap overloads, pinned reordering,
close ranges and document layout commands are also present. Removal, collapse,
pin/preview, hide/restore, split/window construction, floating and document-pane
creation, close handling and ItemsSource tracking now have source bodies. The
concrete reactive Factory and typed locator dispatch are now present.
DockService/DockManager/DockState and outer splitting also have source bodies.
Next are workspace/fluent/service integration and framework models/controls, with
generic interop, weak-table lifetime and untyped value/class metadata gaps open.
Use the existing complete contracts instead of narrower replacement interfaces.

Explicit follow-ups from the contract/helper pass:

- Resolve generic CreateList/GetDockable dispatch through erased factory handles;
  current generic declarations are limited to concrete factories with Self: Sized.
- Extend command parameter adaptation beyond the canonical boxed erased
  Rc<dyn IDockable> handle when concrete model/markup registrations are added.
- Find/Find-in-dock now return DockableQuery: lazy, repeatable and independently
  enumerable. Preserve the source duplicate container yields, visible-only Find
  traversal, callback failure cleanup and source factory lifetime. FindDockable
  separately includes split-view extras and floating window layouts.
- Resolve undefined numeric DockCapability evaluation without invalid Rust enum
  values. The original test using `(DockCapability)777` remains explicitly pending.
- Plain lists now supply fail-fast versions; custom lists supply their own
  enumerators. Group/navigation/MDI helpers use these enumerators. The wrapped
  FerroUI ObservableCollection 0.3.0 still lacks an exposed mutation version.
  Resolve that shared dependency boundary without adding notification handlers
  that would change ObservableCollection's reentrancy subscriber count.
- Attach RequiresDataTemplate metadata during compiled UI/markup registration.
- Port serialization metadata for all reactive models, including DockWindow. Context
  equality currently handles primitive/core values and canonical erased reference
  handles; arbitrary boxed custom values or custom Equals overrides are pending.
- The observable-backend version gap also applies to live factory search and
  navigation/MDI enumeration. Source snapshot points remain explicit; do not
  replace live enumeration with snapshots to hide the gap.
- Preserve Present when a host callback clears Layout before SetLayout. The
  current non-null IHostWindow.SetLayout Rust contract rejects this late null;
  source C# forwards it. Resolve the nullable interop boundary without replacing
  the source live lookup with a stale layout snapshot.

All 44 source docking-group cases pass through full-contract test fixtures.
Capability tests cover 324 policy combinations, but source concrete Factory/model
integration remains pending. The recording factory and node fixtures are compiled
only in tests; production Factory integration tests now run alongside them.

1. Port the remaining framework-independent core interfaces, geometry, events,
   policies and control contracts. Keep their complete inheritance/API contracts;
   do not add narrower replacement interfaces to make an isolated algorithm pass.
2. Reuse the concrete reactive model/window layer for the production factory.
   DockWindow owns its layout and host and uses the source HostAdapter algorithm.
   Standard dock constructors initialize reactive observations before returning;
   custom composed docks initialize DockBase after Rc::new_cyclic. Public component
   constructors and forwarding macros preserve outer sender identity and virtual
   callbacks, including embedded leaf/window parts. RootDock.Window and the
   owner/original-owner/factory/parent-window backlinks are weak. Public standalone
   adapter constructors retain their associated model as in C#; model-owned
   adapters explicitly use from_weak to avoid cycles. Keep both lifetime paths.
3. FactoryBase, adapters, DockService, DockManager and DockState now have native
   algorithms and integration tests. Continue workspace/fluent/service integration
   and original model/UI test harnesses. Cover group inheritance, restrictions/capabilities,
   split/outer split policies, pin/unpin, close cancellation, floating hosts,
   focus/activation, window relationships and layout ownership transitions.
4. Port proportional layout/splitter controls and the UI integration using the
   framework class/property system. Port themes and resources as XAML, retain
   template parts, compiled binding scopes, managed/native hosts and adorners.
   Validate with FerroUI Headless and real desktop interaction/rendering.
5. Port serialization (including metadata/source generation), persistence,
   workspace services, navigation/overlays, diagnostics, model alternatives and
   samples. Resolve framework/dependency gaps using existing shared ports first.
6. Reconcile every inventory entry/member, all upstream test cases and remaining
   differences. Run full unit/headless/integration coverage, examples and target
   builds. Full completion requires evidence for each original feature/API, not
   only a zero-pending inventory or a passing subset of tests.

Immediate open dependency questions are documented in DEPENDENCIES.md:
ServiceDispatcher.InvokeAsync continuation timing and C# cross-thread service
entry points versus FerroUI's thread-affine reactive model. Neither prevents
progress on the remaining engine/model/control files.

The local sibling FerroUI checkout has unrelated uncommitted work; do not depend
on it or modify it implicitly. Use the pinned published crates and the recorded
source/doc versions for reproducibility.

Factory entry point notes: the source partial files live directly under
src/Dock.Model (not Core). The reactive Factory constructor/creation overrides
live in src/Dock.Model.ReactiveUI/Factory.cs. Preserve all source methods across
FactoryBase.cs, .Factory, .Controls, .Locator, .Init, .Events, .Window,
.WindowOptions, .Dockable, .DockingState, .DockingWindowStateSync, .Tracking and
.Lifecycle. There is a partial FactoryBase component and a concrete production ReactiveUI
Factory. Test-only RecordingFactory implements the full interface but must never
become a fallback production factory or hide missing members behind panics. The window-host optional
factory and close-state hooks are now available for the source initialization and
window lifecycle branches.

Collection boundary and factory continuation:

- IList properties and factory list returns now use the shared ListHandle/IList
  bridge, admitting plain/custom/read-only/observable backends. FerroUI System
  0.3.0 has no general IList contract; the observable adapter reuses its existing
  implementation with no extra event subscription or copied notification engine.
- Do not revert these public contracts to ObservableCollection. Native callers
  can wrap an existing observable collection without copying its identity. Plain
  lists support custom element comparers, versioned enumeration and callbacks
  after internal borrows are released.
- FactoryBase::ensure_observable_dockables implements the source private generic
  helper's actual dockable use site. It preserves notifying custom lists, calls
  the enclosing concrete generic CreateList through a typed weak thunk, falls
  back to FerroUI's observable implementation when needed, then copies elements.
  InitDockable now invokes this helper in the original source location.
- FactoryBase::construct requires a complete IFactory + FactoryBaseImpl implementation. Its weak
  runtime is the composition owner; use virtual calls on that runtime where the
  source calls virtual methods/properties. The nonvirtual Find overloads retain
  their own recursion algorithm. Typed GetDockable<T> is now implemented; erased dispatch and composed concrete
  base-class casting remain partial.
- FactoryBase::init_dock_window calls the optional full-window host factory or
  legacy locator, then the virtual explicit-host overload. The explicit overload
  wires host/window, owner and factory before invoking layout initialization with
  the layout's existing owner. The original window fixture tests record that final
  call; the newer initialization tests execute recursive InitDockable.

State/selection continuation:

- FactoryBaseImpl defines native protected virtual hooks; its factory_base getter
  returns the component. Override methods can invoke the component body directly
  for source base-method calls. Keep virtual recursion on hooks and source
  nonvirtual Find recursion on the component. The state-sync interface bridge on
  FactoryBase invokes the enclosing protected property-change override.
- DockingState and DockingWindowStateSync source algorithms are implemented in
  factory_base_state.rs and factory_base_state_sync.rs. They remain partial in
  inventory because observable mutation-version checks and production movement
  runtime/collection integration is still pending. Actual movement and floating
  now run in both native model and production Factory integration tests. Plain versioned list invalidation is tested.
- Source IsDockablePinned/UnpinDockable/IsOwnerAttached live in factory_base_ownership.rs; other
  Dockable partial-file members remain pending. Reuse these exact checks.
- InitActiveDockable now raises OnActiveChanged, calls runtime InitDockable and
  OnSelected, conditionally requests focus via ShouldFocusActiveDockable, then
  reconciles siblings. ActivateWindow/SetActiveDockable are also implemented.
- RecordingFactory now has test-only operation/resolve/update hooks and records
  move requests. Its opt-in execute_base flag now forwards the implemented
  initialization/focus/tracking/event bodies. It now also forwards movement,
  pin/removal/collapse and hide/restore; older explicit simulations stay alongside
  real graph integration tests. Split/floating also dispatch to actual bodies; full
  close and ItemsSource tracking now run actual bodies too. Production Factory
  dispatch is present; weak-table/value-metadata parity remains pending.
- Owned-dockable enumeration retains seen Rc identities (not merely pointer
  integers), including across callbacks, matching the source HashSet lifetime.
  Coalescing and suppression have separate guards; preserve cleanup both during
  operation failure and post-request flag synchronization failure.

The source IsDockablePinned and UnpinDockable methods are nonvirtual. Internal
state resolution/unpin calls invoke the component directly. UnpinDockable only
calls virtual PinDockable when pinned; do not replace this with a separate unpin
operation or capability bypass. Both protected pin-core overloads are now implemented.

Initialization/focus/tracking continuation:

- All Init.cs algorithms are present, including SetFocusedDockable in
  factory_base_focus.rs. The Init record remains partial for observable mutation
  invalidation and complete concrete Factory integration, not missing Init bodies.
- factory_base_empty.rs supplies IsDockableEmpty, split-view empty predicates,
  UpdateIsEmpty and checked Int32 opened-count sums. Source order updates a split
  container's empty state before separately initializing unlisted pane/content.
- factory_base_tracking.rs implements all Tracking.cs algorithms, including the
  distinct root initialization resolver (return an existing focused container
  directly) and generic resolver (descend through focused/active containers).
  Keep reference cycle guards on active traversal and no invented owner-cycle
  behavior. Tracking snapshots remain weak and compare resolved identities.
- factory_base_events.rs implements init/context, active/focused, dockable
  activation/deactivation, window activation/deactivation and anchored/stale-root
  policy. All other subscriptions/callbacks in Events.cs are now implemented, including
  lifecycle notifications, close policy, dragging and mutation events. The
  concrete production Factory and framework/host integration remain deferred.
- EventSubscription holds a weak System Event plus its token; disposal is explicit
  and idempotent, dropping an ignored handle leaves the event subscribed.
- RecordingFactory.execute_base is test-only opt-in dispatch to implemented
  component methods. Do not mistake it for a production Factory or enable
  placeholder remaining methods. Existing observation-only tests remain valid.
- Shared TestControl replaces the earlier duplicated search-control fixture,
  preserves layout read counts and supplies registered roots for focus/tracking.

Lifecycle/window continuation:

- FactoryBase now implements the entire IFactoryEvents interface and all inherent
  event methods. It still does not implement IFactory; never fill that gap with
  production panics or no-op mutations. RecordingFactory forwards events and
  window operations to the component, retaining observation-only alternatives
  for the earlier active/focused/init tests through execute_base.
- IFactory.as_dock_window_close_policy returns the optional full window policy.
  OnWindowClosing skips the model hook only when preservation returns true; the
  cancellable event still runs. Do not replace the final event cancellation
  decision with an AND of model/policy/observer decisions.
- IWindowLifecycleService and FactoryBase.Lifecycle are ported. Service list
  entries are Option<Rc<dyn IWindowLifecycleService>> to admit the source null
  skip. Keep a live plain list, no snapshots, factory-side deduplication or catch
  blocks. NotifyWindowClosed/Removed are protected hooks on FactoryBaseImpl.
- Window methods preserve source RemoveWindow ordering: capture layout, Exit,
  check current owner/host-close cancellation, remove registration, event/services,
  conditionally clear the captured layout's matching Window, then clear parent,
  owner, factory and layout. Respect callback replacement of layouts/window links.
  CloseWindow uses visible-only per-dock snapshots and descendant-first requests;
  CloseDockable now executes actual eligibility/core/source bodies through the
  test factory and the production concrete Factory.
- FactoryBase binds a second typed weak generic CreateList thunk for window lists.
  Preserve both dockable/window overrides until general erased generic dispatch
  is addressed. Do not normalize window lists to observable; source Add/Insert
  uses the concrete CreateList result directly.
- ApplyWindowOptions, PrepareWindowOptionsForDockable and
  ResolveWindowCollectionRoot have native protected hooks ready for floating
  operations. Floating/split construction now uses those hooks.
- Dock.Model.ReactiveUI.Services.Lifecycle.DockWindowLifecycleService and its
  overlay/router/disposal tests remain pending; the newly ported core service
  contract/notifications do not supply that implementation.

Collection/mutation continuation:

- factory_base_collection.rs ports AddDockable/InsertDockable, the protected
  visible-list helpers, GetActiveDockableAfterRemoval and CleanupOrphanedSplitters.
  The helpers are nonvirtual and do not initialize/raise events. Replacement
  selection is a protected hook on FactoryBaseImpl and returns null for a
  preceding splitter rather than searching farther. Wire that hook in removal.
- Splitter cleanup deliberately retains candidate duplicates from the source
  consecutive-splitter scan; callbacks may reinsert a candidate before a later
  duplicate is examined. It rechecks the current list after each callback and
  repeats until no removal occurs. Initial null/empty lists return immediately
  without changing ActiveDockable, as in the source.
- factory_base_move.rs ports both move and swap overloads and pinned reordering.
  Do not merge the overloads: their notification order, transient duplicates and
  final selection differ. Four-argument same-owner moves preserve selection.
  With a different owner, a missing/null target in a nonempty target list inserts
  at Count-1; an explicit existing target inserts after its index.
- Cross-owner moves call the real full IFactory RemoveDockable/CollapseDock
  boundary. Native implementations of those dependencies are now present.
  A floating source root is removed with collapse=false, then collapsed only
  after target insertion/init/selection/events/cleanup have succeeded.
- factory_base_close_range.rs uses reverse live indices, not snapshots. It
  delegates to actual CloseDockable through the runtime. Preserve reference
  exclusion and source index failure behavior when callbacks remove extra items.
- factory_base_document_layout.rs ports all seven mode/tabs setters/wrappers.
  Nonvirtual wrappers dispatch the virtual setter on the enclosing IFactory.
- RecordingFactory now forwards add/insert/move/swap/pinned-reorder/range/layout
  bodies as appropriate. on_remove and on_collapse can override those operations
  for the older controlled-dependency tests; execute_base now uses the actual
  implementations by default. Do not turn fixture callbacks into production substitutes.
- Removal/pinning/preview/hide/restore and main CollapseDock/proportional cleanup
  are implemented, using the visible-list and empty/count helpers. Split/window
  construction, floating, ItemsSource tracking and CloseDockable/Core are now
  connected. Native optional ItemsSource contracts and interface-query hooks are
  present; source-backed framework controls/lifecycle remain pending.

Pin/removal/collapse continuation:

- factory_base_pin.rs and factory_base_preview.rs port pin/core overloads, unpin
  internals, aligned-target selection, bounds, preview hide/open/toggle and pin
  strip cleanup. Protected core overloads are on FactoryBaseImpl. Pin command
  policy is intentionally bypassed by removal/hide lifecycle cleanup.
- Source duplicate strip cleanup removes once from the selected alignment and
  once from each strip; it does not repeatedly remove all duplicates. Preserve
  this count/order. Source active/expanded/autohide TODO behavior is not invented.
- Same-root pins retain the original tool owner. Pins into another root retain
  OriginalOwner and establish ownership in the pin root; repeated previews must
  not replace that history with the preview dock or floating root.
- Unpin chooses explicit destination, attached/aligned original/current owner,
  aligned visible query result or newly created tool dock in source order. Empty
  root creation and existing aligned dock reuse run end to end. Anchor splitting
  now executes actual SplitToDock for all four alignment directions.
- factory_base_remove.rs now performs actual hidden/visible removal, protected
  replacement selection, focus reconciliation, splitter cleanup and collapse.
  Removed items keep source owner backlinks; do not clear them prematurely.
- factory_base_collapse.rs ports IsDockPinned, CleanupProportionalDockTree and
  CollapseDock from the main partial. A nonempty aligned pin strip blocks collapse
  even without an item owned by that dock. Proportional flattening stops when its
  owner is a root; direct parent list edits keep source event/count behavior.
- IDockable.as_proportional_dock_splitter and the concrete reactive splitter hook
  preserve source proportional-only type tests; do not use the generic splitter
  marker for neighbor selection or proportional-child counting.
- factory_base_hide.rs ports hide/restore and both ID wrappers. Preserve restore
  event order: remove hidden membership, restored event, add visible/added event,
  then update Owner/OriginalOwner and recursive state. Hide does not collapse the
  original container. Search enumeration ends before HideDockable(string) mutates.
- RecordingFactory execute_base now uses actual pin/preview/remove/collapse/hide/
  restore operations, retaining explicit old-test overrides. The shared reactive
  fixture configures a real ToolDock creator; a separate production Factory now
  supplies all source reactive model creators.
- New factory_pin_tests/factory_remove_tests cover real graphs/hosts. The original
  headless/framework-property tests still need FerroUI framework model variants
  and Headless integration; native model tests do not substitute for those.


Split/floating continuation:

- factory_base_split.rs ports CreateSplitLayout and SplitToDock, CopyDockGroup,
  IsAssignedProportion and ResolveEffectiveProportion. Compatible proportional
  owners get direct insertion and share splitting; fallback replaces the old
  default/active dock with a wrapper. Keep their distinct initialization/events.
  Optimized splitting initializes the incoming dock only, leaving the new splitter
  without owner/factory. The fallback Removed/Added payload is the incoming item.
- Shares treat positive infinity as assigned, ignore only proportional splitters,
  infer remaining shares for unassigned children, and fall back to equal shares
  when assigned shares exhaust the total. CreateSplitLayout itself returns a
  splitter-only layout for unsupported operations; SplitToDock rejects them.
- factory_base_create_window.rs ports both CreateWindowFrom and SplitToWindow
  overloads. Classify ITool before IDocument (Tool implements both). Reuse only
  the four source container types; a root uses its active child, including null.
  Creation alone never removes or initializes old/new ownership. Preserve source
  Added events and root/window defaults before returning the uninitialized model.
- Splitting prepares owner options before removal, registers in the outer workspace,
  initializes through AddWindow, sets geometry, presents, emits Docked, then focuses.
  Unsupported creation after removal still leaves the removal/Undocked committed.
- factory_base_float.rs ports single/all floating, DockAsDocument and new horizontal/
  vertical document panes. Single floating captures its owner before unpinning;
  preview-owner geometry survives that change. Coordinates fall back per-axis only
  on NaN; infinite coordinates survive. Dimensions must be finite and strictly
  above sixteen. Whole-pane floating snapshots siblings but rereads IsCollapsable
  per removal. Appended callback siblings stay behind; removed snapshot siblings
  are still transferred. Whole-pane floating emits Moved, without extra Docked.
- factory_base_metadata.rs copies optional document/tool templates, container theme,
  selector and unregister policy, plus document-factory delegates. No ItemsSource,
  generator or generated membership is copied. Window/FloatAll omit LayoutMode;
  new document-pane operations copy it. Full-contract composed test docks verify
  non-null identity and null overwrite. Plain ReactiveUI models omit these mixins.
- Complete IItemsSourceDock/IToolItemsSourceDock, container-generator and both
  selector interfaces are present with IDockable query hooks. The pinned System
  0.3.0 has no non-generic IEnumerable/IEnumerator; collections/i_enumerable.rs is
  the full declaration bridge including optional IDisposable, with no production
  iterator implementation. Do not substitute Vec for a live source enumerable.
- The recording factory now configures real root/proportional/document/tool/dock/
  splitter/window creators and forwards the new operations. CloseDockable now
  executes actual bodies by default; simulate_close explicitly retains older
  request-only tests. Separate production Factory tests now exercise these bodies.
- Thirty-six new tests bring the workspace to 572 passing tests. Headless split/UI
  tests, framework-property ItemsSource controls and production factory/host
  remain pending. Close/tracking source bodies are now present; existing native
  lifetime, observable-version, erased-generic and boxed-equality gaps remain open.


Close/ItemsSource continuation:

- factory_base_close.rs ports CloseDockable, protected CloseDockableCore and
  HandleItemsSourceDockableClosing, with private source dock resolvers. Null,
  last-child and capability checks happen before virtual core dispatch. The core
  bypasses eligibility but retains model/event cancellation, generated-source
  handling, hide/remove and Closed ordering. Close ranges and CloseWindow now
  exercise actual native close bodies as well as earlier explicit request tests.
- Resolve documents before tools, including Tool's IDocument membership. Resolve
  source owner in current-owner, protected tracked-owner, then visible-root Find
  order. Keep source nonvirtual Find semantics, not FindDockable's broader walk.
  Recognition suppresses hiding even with null Context or false source removal.
  RemoveItemFromSource gets current Context, not the originally tracked item.
- Source removal can synchronously untrack/remove a container before CloseCore's
  own RemoveDockable call. Do not add duplicate removal notifications or auto-untrack
  in FactoryBase.Close: source controls own that lifecycle. Failure propagates and
  skips Closed. Never hold tracking/collection borrows across callbacks or drops.
- factory_base_items_source.rs ports public tracking overloads, untracking,
  GetContainerFromItem and protected tracked-owner lookup. The implicit overload
  dispatches virtual explicit-item tracking with Context. Per-item lists are plain
  strong lists (not CreateList overrides), preserve duplicate insertion order and
  remove by reference identity; retracking moves that container to the end.
- GetContainerFromItem accepts Option<BoxedValue> across IFactory/FactoryBase to
  preserve the actual null guard despite the C# nonnullable annotation. Non-null
  lookup uses the published System Dictionary and removes stale containers lazily.
- The two native identity tables use weak keys and strong values, as required for
  live tracked-owner retention. Expired noncyclic keys are pruned on access and
  retained values are dropped outside RefCell borrows. CLR ConditionalWeakTable's
  ephemeron collection of owner→key cycles is NOT implemented. Explicit untrack
  releases values; do not mark leak/lifetime parity complete. Pruning currently
  scans the table; native lifetime/performance integration remains open.
- value_semantics.rs shares the existing Context equality with item-key hashing.
  Scalars, geometry (NaN/signed zero), defined enums and canonical erased handles
  have consistent value/identity hashing. Custom boxed value Equals/GetHashCode,
  concrete/subtype reference-box metadata and shared framework value registration
  remain pending. Do not claim arbitrary object-dictionary equivalence.
- RecordingFactory uses actual close by default when execute_base is enabled.
  simulate_close is set only by older reverse-index/snapshot dependency tests;
  source/core/tracked-owner/track-item callbacks test virtual dispatch explicitly.
  Metadata fixtures now have synchronous source observers for reentrant cleanup.
- Thirty-four new tests bring the workspace to 606 passing tests. These cover
  null/eligibility/cancellation/hide rules, source resolution after move/float,
  duplicate/retracking lookup and retention/release, native reentrant stale lookup,
  current Context, source failures, selection/focus, source metadata and hash keys.
  Original framework-property/headless ItemsSource controls and tests remain open.
- The concrete production Factory, abstract creation/control contracts and typed
  GetDockable are now present. Continue toward the remaining engine/adapters and
  UI, preserving the source-controlled typed CreateList thunks. General erased
  generic/value/class metadata work remains open; no recording bodies are reused.


Production factory/locator continuation:

- ReactiveUI factory.rs composes FactoryBase, eight source dictionaries and
  observable DockControls/HostWindows. Every model creator uses the concrete
  reactive model with source defaults. Root creation calls the enclosing generic
  CreateList separately for all four pinned lists; CreateLayout calls the virtual
  CreateRootDock on the enclosing runtime. Neither operation implicitly initializes.
- FactoryImpl supplies typed static virtual defaults; impl_factory! implements
  the complete IFactory/IFactoryEvents/IDockingWindowStateSync contracts. User
  subclasses implement FactoryBaseImpl::factory_base and FactoryImpl::factory.
  They can call component.base() for inherited engine base bodies or component
  methods for concrete reactive creation/registry bodies. Engine virtual recursion
  remains on the enclosing runtime. Nonvirtual source methods/subscriptions forward
  directly to the shared base. Optional host/close-policy membership has native hooks.
- scripts/dock/generate_factory.py emits dispatch declarations only. Its --check
  validates native contracts, C# virtual/nonvirtual classification and the exact
  ReactiveUI-versus-inherited base-body targets, then compares the formatted output.
  Regenerate after interface changes; never generate alternate docking algorithms.
- FactoryBase.GetDockable<T> now clones the ID locator outside registry borrows,
  invokes it once, and checks requested interfaces or exact concrete/custom Rc
  references. IDockable.boxed_reference is a hidden static-lived native bridge;
  boxed_interface lets applications declare additional interface casts without
  cloning the object or changing locator implementations;
  grid splitter membership has its own hook. It never clones a model or invents
  a new identity. The test recording factory forwards this body too.
- Concrete base-class casts of composed subclasses are NOT solved by exact
  reference boxing. They need native class/reference metadata that preserves
  outer identity and lifetime. Generic calls through erased IFactory remain
  Self:Sized; arbitrary generic virtual dispatch is also incomplete.
- CreateList<T> uses the pinned System observable collection and its PartialEq
  constraint. Fully general boxed/unregistered element types and source default
  comparer semantics (including float NaNs) remain a shared dependency/type gap.
  Preserve this explicit partial status; do not claim unrestricted generic API parity.
- Eighteen new tests bring the workspace to 624 passing tests. Production tests
  cover shared registries, every concrete model creator, root pin lists, locators,
  actual split/move/close/focus, modal host presentation and the original full
  tool/document/pin/float/hide/restore state sequence using source defaults.
  Six downstream tests prove composition, generic/public/protected overrides,
  exact custom/interface locator identity, optional host membership and absence
  of default ownership cycles. Existing recording tests remain explicit fixtures.
- DockService, DockManager, outer splitting and DockState are now implemented
  using original algorithms and native counterparts of their tests. Workspace,
  fluent/service and UI integration remain pending, alongside class/generic/value/
  lifetime metadata and original Headless tests. No usable docking control exists.


Service/manager/outer/state continuation:

- dock_service.rs ports every source operation and helper. Empty/global movement
  uses global group validation; populated/local movement selects the last nested
  leaf and validates the entire actual target dock. Swap prefers ActiveDockable.
  Tool split policy is queried only during execution. Target tool settings take
  precedence over source settings, while document splits copy owner metadata and
  omit LayoutMode. Preserve Move-before-Split order and source factory selection.
- Window service sizing is deliberately different from FactoryBase.Float: only
  NaN triggers fallback, so negative/infinite/small tracked sizes survive. Window
  validation requires the source root's active dock. No broader validation or
  capability checks are invented for source methods that do not perform them.
- dock_manager.rs plus docking/pinned modules port options/pointer properties,
  restrictions, policy/capability order, diagnostics, all type classification,
  root wrappers, pane/group routing, pinned-target behavior and visibility checks.
  Guarded tool-owner patterns must fall through to document-owner matching when
  Tool also implements IDocument. Whole-pane operations snapshot children, while
  group validation uses reverse live indices, skips non-docks and never short
  circuits; the first capability failure is restored after the loop.
- Operation observers preflight before Begin, then revalidate during execution.
  Complete happens only on true, followed by Dispose on all paths. Native unwind
  capture surrounds execution/Complete so disposal failure replaces an earlier
  failure without Rust double-panic abort. Do not add rollback or deduplication.
- dock_outer_split.rs ports directional mapping, containment/remaining-content
  guards, focused/selected content selection and execution. It retains panes and
  nested groups, can reorient a single-child proportional parent, removes a source
  container without collapse, splits, then collapses its old owner. Leaf wrapping
  assigns the effective group (including null) and restores destination focus.
- dock_state.rs ports content/template preservation, not layout serialization.
  Four maps separate tool/document content and tool/document templates. Traverse
  hidden, four pin strips, preview, windows, split-view extras, then visible items;
  cache a listed dock before recursing, but not the entry dock itself. Last ID wins.
  Saved null content suppresses fallback; missing content uses the current owner's
  template. Reset releases retained values outside mutable native borrows.
- IFactory exposes native split/drop/outer/observer/FactoryBase membership hooks.
  create_dockable_list is an object-safe specialization of CreateList<IDockable>:
  production macros and the recording factory invoke the actual generic override.
  The downstream consumer verifies dispatch. General erased generics remain open.
- IDockable tool/document content hooks also support typed locator casts. Plain
  reactive models do not invent content members; complete composed content and
  metadata probes exercise state/copy behavior before framework models are ported.
- Forty-one new tests bring the workspace to 665 passing tests. They cover source
  policy/proportion scenarios, metadata/delegate copying, snapshots/order, size
  conflicts, pin sides, observer failures, every outer direction/shape/rejection,
  native content/template state and reset/reentrancy. Original framework-property/
  Headless harnesses, observable mutation versions and custom IList snapshot/default
  equality remain explicit partial behavior, not completed UI/engine parity.
- Workspace snapshots/manager/tracking options now have native implementations.
  FluentExtensions is now implemented; next inspect remaining model services and ReactiveUI
  lifecycle/navigation/overlay integration,
  then the FerroUI property-model/control/theme layer. Preserve the full inventory
  and revisit prior generic/class/value/lifetime gaps before completion.


Workspace/serializer-contract continuation:

- DockWorkspace, DockWorkspaceTrackingOptions and dirty event arguments preserve
  shared identity and source defaults. Re-capture updates the existing snapshot,
  replacing payload/state and cleaning its flag; blank update metadata keeps the
  old value. New snapshots default their name to the original untrimmed ID.
- DockWorkspaceManager ports capture/state-before-serialize, successful and null
  restore paths, external workspace restores, lookup/remove/clear, active dirty
  state, all 16 event subscriptions, same/different factory tracking, root scope,
  shared live options and source transition-only events. Preserve the surprising
  exact active-ID removal check after case-insensitive dictionary removal. Removing
  the active entry clears effective layout dirty state but does not clean that
  now-detached workspace. Null event contexts skip root checks and reach filters.
- Source root checks reread tracked scope after FindRoot/window getters. Filter and
  event callbacks run outside native borrows and may stop tracking or clear the
  active workspace. Native callbacks retain only a weak manager and unsubscribe on
  manager drop, per the ownership guide; StopTracking preserves dirty state.
- IDockSerializer declares all four genuinely generic source operations. The native
  manager carries its concrete serializer type to preserve typed generic calls.
  Arbitrary erased generic dispatch/class metadata remain open; do not replace the
  contract with dock-only serializer methods. DockStream uses standard borrowed
  native Read/Write/Seek; the complete System.IO.Stream runtime remains deferred.
  There is still no production serializer or verified real serialized round trip.
- Workspaces is a live read-only view over the published System Dictionary. Native
  enumeration captures initial keys and reads current values as it advances,
  preserving source slot order, overwrite/removal/clear/new-insertion behavior.
  The key capture adds allocation because the runtime lacks a shareable slot
  enumerator; shared runtime implementation/performance verification remain open.
- The shared runtime has no OrdinalIgnoreCase comparer. A generated simple-case
  table preserves the current .NET runtime's scalar-string equivalence; 3,005
  independent actual .NET comparisons check upper/lower candidates and special
  Unicode cases. scripts/dock/workspace-reference documents reproduction. Other ICU/
  NLS versions/globalization modes and unpaired UTF-16 strings are unverified.
- Sixteen new Rust tests bring the workspace to 681 passing tests: 15 production
  model/factory integration tests cover all 12 original workspace test scenarios
  plus failures, live collection/options, reentrancy and release; one model test
  checks the complete independent .NET comparator corpus. The serializer fixture
  preserves identity exactly as the source test stub, and is not production JSON.
- FluentExtensions is now implemented; continue remaining model/ReactiveUI service integration,
  followed by framework-property models, controls/themes/markup. Previous generic,
  class, collection version/equality, value and lifetime gaps remain in the full
  goal and the completeness gate must continue rejecting the unfinished port.


Fluent creation/configuration continuation:

- fluent_extensions.rs ports all 129 public source overloads (106 distinct names)
  and EnsureList using extension trait groups on Rc handles. Import
  ferrodock_model::fluent_extensions::* to use them. Each helper returns the same
  handle; native types are retained through chains. Factory helpers invoke actual
  virtual Create* methods on enclosing factories. No docking algorithm is replaced.
- Native `_into` creator overloads write &mut Option<Rc<dyn Contract>> and return
  the original factory. Document/Tool assign output AFTER configure. Every other
  creator assigns output BEFORE initial properties and configure. Preserve this
  on both callback and property-setter failures. Regular creators assign initial
  orientation/alignment/splitter flags before configure. `_default` adapters cover
  the three option-bearing creator pairs; WithGrid's defaults have their own adapter.
- Plain-list fallback is intentional: source EnsureList uses new List, never
  factory.CreateList or a new ObservableCollection. Reassign the list property even
  for a nonnull list, then reread it for EVERY Add, including after iterator advance
  and collection notifications. Append; do not replace, deduplicate, snapshot,
  assign ownership/factory or initialize the layout. Source params/IEnumerable
  forms share the native IntoIterator implementation. AppendDocument/AppendTool
  invoke actual enclosing virtual methods and retain their source side effects.
- Fifteen new tests bring the workspace to 696 passing tests: 13 fluent regression
  tests, one enclosing model override test and one downstream factory/typed-handle
  test. Every creator pair and property helper is exercised; all 14 original
  document/split-view source scenarios have counterparts. Additional cases cover
  defaults, exact returned identities, nullable clearing, geometry order, arbitrary
  values, plain/observable list identity, duplicate/self-enumeration behavior,
  reentrant list replacement/iterator advance and output/property/callback failures.
- Template and concrete reactive model fixtures remain test-only; no templates or
  commands are invented in production. Keep a strong factory reference in tests:
  native model factory/owner links are weak per the port guide.
- Fluent source records remain partial for shared observable mutation-version and
  broader native collection/class/generic metadata gaps, plus original framework-
  property/Headless integration. There is still no usable docking UI or production
  serializer. Next inspect remaining Model service contracts and ReactiveUI
  navigation/lifecycle/overlay dependencies, then framework-property models and
  controls/themes/markup. Use the real published ReactiveUI IScreen/routing APIs;
  inspect their cast/type hooks before designing the navigation service boundary.


Overlay request and busy/global service continuation:

- Model services now include all nine IDockBusyService members, both global
  dialog/confirmation contracts, IDockDialogContent and IReloadable. Request
  models and the source internal DelegateCommand use the actual shared System
  TaskCompletionSource/Task and event implementations. Metadata, nullable content/
  title/result/handle distinctions and task identity are preserved. Confirm/Cancel
  call the supplied close action with the same request and true/false; Dialog.Close
  forwards its result. Commands do not complete/dispose requests by themselves.
  Complete is one-shot TrySetResult. Replacing/dropping a GlobalHandle does not
  dispose it. The per-host dialog/confirmation services still need implementation.
- Native ICommand receives borrowed Any. Dialog.Close currently requires a
  borrowed canonical BoxedValue handle to retain arbitrary result object identity.
  General raw/unregistered borrowed-value promotion remains a shared metadata gap;
  no cloned object, substituted result or null fallback is supplied. Model-owned
  command callbacks are weak per the port guide; external retained-command/CLR GC
  lifetime equivalence remains unresolved. Keep those source files partial.
- DockGlobalDialogService/DockGlobalConfirmationService preserve source default
  messages, counter/latest-message logic, explicit one-shot scopes, captured Post
  ordering and state-before-message notifications. Closing one of several scopes
  retains the latest message. Reentrant Begin during the final false notification
  can reopen the flag while the outer Message assignment still clears the message;
  this surprising source assignment order is tested and must remain intact.
- DockBusyService ports scopes, messages, visibility, reload properties/command,
  dispatcher paths and eager RunAsync. Global Begin precedes local count/updates;
  local final false/message updates precede global Dispose. Source RunAsync is
  implemented with actual shared tasks/continuations: pending work retains the
  scope, completion disposes before finishing, errors keep shared identity and
  cancellation state, and cleanup failure supersedes work failure. If synchronous
  local cleanup notification throws, global disposal is skipped just as upstream.
  Do not invent a finally around that later global Dispose to 'fix' source behavior.
- Reload uses actual WhenAnyValue and ReactiveCommand.CreateFromTask. Its gate is
  CanReload (handler presence), independently of button visibility and busy state;
  ReactiveCommand also controls execution state. A running handler is captured;
  replacing the handler affects subsequent runs. Handler identity uses shared Rc;
  full CLR delegate equality still needs native metadata. Callback reads/writes
  release RefCell borrows before reactive user notifications and allow reentry.
- threading/task_action.rs captures synchronous native async-boundary panics into
  the framework's standard TaskWorkError diagnostics. Returned task errors retain
  their original Error object. Rich CLR exception metadata and cross-thread
  ConfigureAwait/Interlocked semantics beyond native thread-affine tasks/reactive
  services remain explicit gaps, not completed parity.
- Twenty-eight new tests bring the workspace to 724 passing tests: eight model
  request/command/contract tests, eight global-scope tests and twelve busy/reload
  tests. They cover actual tasks and commands, nested/out-of-order/reentrant scopes,
  queued and same/foreign context paths, notifications, pending reload replacement,
  task errors/cancellation, begin/cleanup failures and release. Original multiwindow/
  Headless overlay controls and lifecycle tests remain separately pending.
- Next implement the remaining Model dialog/confirmation/host contracts and
  ReactiveUI per-host dialog/confirmation services, overlay hosting/lifecycle and
  navigation. The pinned System runtime has ObservableCollection but no
  ReadOnlyObservableCollection; do not expose a mutable collection or a snapshot
  substitute. ServiceDispatcher.InvokeAsync now has a native pending-await adapter; shared
  TaskCreationOptions API/performance integration remains open. IScreen has only Router and no Any/type-query hook;
  source screen/model/service interface casts need a proper native representation.
  These are design work, not an external blocker: continue other required port
  work while resolving them. Framework-property models, controls/themes/markup,
  serializers and all previous engine/runtime/API gaps remain in the full goal.


Per-host dialog/confirmation stack continuation:

- IDockConfirmationService, IDockDialogService and IHostOverlayServices are now
  declared. HostOverlayServicesAdapter retains all six supplied actual service
  references; no resolver/provider factory is invented. Hosting provider, owner-chain resolution and lifecycle are now present;
  navigation, dependency registration and BusyRootDock remain next service work.
- ReadOnlyObservableCollection in Model/collections is a live read-only adapter
  over the actual shared System observable storage. Exactly one collection and
  one property forwarding subscription are attached. Typed/untyped view listeners
  run in registration order with original payloads. IList mutators always throw,
  including Remove(absent). Do not expose the source through as_observable_collection
  or replace the view with a snapshot. Native weak forwarding detaches on drop;
  CLR event/GC lifetime, shared observable mutation versions and binding metadata
  remain partial. Multiple view listeners still count as one source subscription.
- Dispatcher.InvokeAsync now uses actual System TCS completion and a private
  DispatcherInvocation Future adapter. A pending await's Send Waker is transported
  through a worker, so completion does not inline its continuation. System forwards
  it to its owner's queue/poster; no Rc/value crosses threads. Already-completed
  awaits remain inline. Cached wakers retain System's repeated-poll deduplication.
  Task result/status storage is not copied. A worker is created for each pending
  wake: transport performance and shared TaskCreationOptions API integration remain
  open, alongside complete ConfigureAwait/cross-thread/exception metadata parity.
- spawn_catching captures panics during every poll at native async boundaries,
  preserving task faults for setup, posting or content callback failures. Returned
  task errors keep their identity; native panics still use TaskWorkError diagnostics.
- DockConfirmationService/DockDialogService port all source stack algorithms.
  Create the request/global scope eagerly, register content close action before
  insertion, then await dispatcher and request completion. Dialog alone skips an
  already-completed request before Add. Confirmation deliberately has no guard.
  Close removes first, disposes global, completes request, then recomputes active
  from the current last item and always raises Has. Foreign/already-removed requests
  still Dispose/Complete without active/Has changes. CancelAll snapshots insertion
  order and keeps requests opened reentrantly. No rollback/extra cleanup is added.
- Preserve source oddities: reentrant Close during Add may leave Active pointing
  at the now-removed request when the outer Add resumes; a throwing global Dispose
  leaves removal applied and completion/active reset unfinished. Completing a
  removed request later does not repair Active. Tests retain these source states.
- Typed ShowAsync is a generic extension over the object-safe actual raw-result
  service core. Source value/default behavior uses native values, nullable values
  use Option<T>, reference results Option<Rc<T>>, and object results Option<BoxedValue>.
  References retain identity. Known dockable interface/exact-model casts reuse
  GetDockable's reference query, extracted unchanged to native_reference.rs. General
  CLR base/interface/value metadata remains open; no new class-cast claim is made.
- DialogContent carries an original object and optional explicit native
  IDockDialogContent projection. Requests receive the original object, never the
  metadata wrapper. from_dialog_content supplies concrete membership without
  reflection; boxed interface handles also work. Automatic membership of arbitrary
  unregistered erased content remains open. Callback request/service ownership is
  weak to prevent native cycles; complete retained-callback/CLR GC parity is open.
- Thirty-two new tests bring the workspace to 756 passing tests: four read-only
  adapter tests, one model result-cast test, one actual reactive-model cast test,
  twenty-four stack/content/result/host/context regressions and two dispatcher
  await tests. Source defaults, exact ordering, real task continuation/reentry,
  queued/early close, cancellation snapshots, failures without rollback and native
  release are covered. Original framework-property/multiwindow/Headless tests,
  controls/themes/markup, serializers and all earlier engine/runtime/API gaps
  remain in the full goal. Keep the completeness gate rejecting the full port.


Host resolution/provider/lifecycle continuation:

- HostTypeRegistry is an instance-scoped native query registry shared by service
  composition, not another IScreen/router. register/register_interface retain weak
  projections of the actual same Rc allocation and reject unrelated-instance
  projections. Convenience screen/dockable-screen/routable registration retains
  known existing root membership. HostServiceValue keeps the original service
  identity and canonical typed reference, with dockable metadata where available.
  Existing dockable boxed_interface hooks also participate. There is no global
  mutable state or reflection. Automatic membership of arbitrary unregistered
  objects and CLR class/base/interface metadata remain unresolved; never treat
  a missing registration as proof that full runtime interface parity is achieved.
- OwnerChainHostServiceResolver ports the original algorithms and order: hosted
  window layout, LAST owner-chain match, factory primary then floating layouts;
  after that, reverse live router stack and successive routable host screens with
  visited-screen detection. Find is obtained afresh for floating roots. Unhosted
  routable roots may normalize to a different outer root; roots with Window stay
  unchanged. No owner-chain cycle guards or alternative active-document heuristics
  are invented. Source's unused first-match helper is retained. Native screen
  visited keys use identity; CLR custom Equals/GetHashCode remains a metadata gap.
- IHostServiceResolver has an object-safe query and typed extension; provider has
  both source overloads with native suffixes. HostOverlayServicesProvider uses
  resolver-before-cache, evicts the successfully resolved SCREEN key, creates a
  per-screen fallback only when needed and preserves both constructor overloads.
  Reentrant fallback creation returns an already-inserted inner cached value.
  Dockable context first tries its screen resolver and evicts that key, then walks
  to a factory and FindRoot with an always-true predicate, then root/window services,
  before the separate fallback screen. Nonfocusable roots are deliberately eligible.
- Provider fallback cache uses weak identity keys and strong values, with expired
  noncyclic entries pruned and values released outside borrows. It does NOT solve
  CLR ConditionalWeakTable ephemeron cycles (value retains key), exact GC eviction
  timing or native pruning performance. Keep the source provider partial. Lifecycle
  cleaned-window markers contain only weak window references and are pruned too.
- DockWindowLifecycleService checks nullable window/layout/screen before marking.
  It marks once BEFORE service resolution and callbacks. Services resolve from
  the screen directly, optional resolver, then concrete-provider CACHE ONLY or a
  custom provider's GetServices. Do not create fallback services during cleanup of
  an uncached built-in provider. Missing services and thrown cancellation still
  consume the marker. Cancel dialogs, then confirmations, then REREAD window.Layout
  and optionally Dispose that current object. Reentrant events remain idempotent.
- Fifteen new tests bring the workspace to 771 passing tests (77 services). Both
  original lifecycle source scenarios run with actual production dialog/confirmation
  services and complete composed reactive root models. Production factory window
  events also invoke cleanup after event delivery. Tests cover direct/concrete
  interface identity, false-membership rejection/release, last-owner/reverse-router/
  host-cycle/window/factory/floating/normalization resolution, per-screen and
  reentrant fallback caches, correct-key eviction, nearest nonfocusable root/window
  context, no-fallback cleanup, layout replacement, failures and reentry. Expired
  noncyclic cache values are released in a separate native test.
- The services crate has a DEV-only dependency on the production reactive model
  crate for these integration tests; production service dependencies and all
  published FerroUI pins remain unchanged. Test-only controls/hosts/root composition
  implement full native contracts; they are not production UI implementations.
- Next port navigation service/routable models/BusyRootDock, activation/dispatcher
  and source DI registration, and proceed to framework-property model/control/
  theme/compiled markup integration. Use the actual shared routing and service
  engines with this lookup path. Automatic reference/value/class metadata, cache
  cycle collection, shared collection versions/equality, cross-thread semantics,
  production serializers and all prior gaps remain in the full goal. No usable
  docking UI is claimed and the completeness gate must keep rejecting the port.


Navigation/routable/busy-root continuation:

- All ten navigation source files now have explicit mappings. The static helpers
  and IDockNavigationService contract are ported. Concrete/abstract models,
  DockNavigationService, DockViewModel and BusyRootDock stay partial for recorded
  native metadata/lifetime and original application integration differences.
- Services now use the production reactive model crate in normal dependencies,
  matching the source assembly; all external published pins remain 0.3.0.
- DockNavigationService resolves first factory and document dock independently,
  uses an attached factory only after owner traversal, and tries a distinct attached
  host only after the supplied context fails. A non-dockable screen fails direct
  resolution even when an attached factory exists. Root fallback prefers an
  unlisted explicit document default, then the first direct visible document dock;
  it does not search nested children. Existing owner membership reactivates/focuses
  and activates the window; adding does not explicitly activate a window. No ID
  deduplication is introduced. Optional Float is last in either branch.
- Routable models compose existing model state and own real RoutingState instances.
  Typed constructors retain outer identity and simple class-name defaults; empty
  explicit routes remain empty. Abstract document/tool helpers call the provider
  with the complete enclosing model as both screen and dockable, navigate their own
  router and close through the nearest factory. RoutableDockBase adds IRoutable
  metadata only. Initialize inherited dock observations after Rc construction.
- DockViewModel calls virtual CreateLayout then InitLayout for a nonnull result,
  then sets Layout. Setter replacements never initialize implicitly; six overlay
  getters query the current layout each time and retain source error behavior.
- BusyRootDock invokes the services factory once and installs actual busy reload.
  FindReloadable immediately descends a dock's nonnull active child, without parent
  fallback; otherwise it checks only the router top for IReloadable, then the
  dockable itself. Reload awaits the actual Task, preserving error identity.
  Dispose marks first, removes reload then cancels dialogs then confirmations;
  failure/reentry cannot cause retries or invent missing cleanup.
- HostTypeRegistry's routed projection can now forward through registered dockable
  boxed_interface hooks. Register same-object screen/dockable/routable projections
  for erased custom application objects. Automatic unregistered CLR interfaces,
  base classes and generic names, IgnoreDataMember/serialization/compiled metadata,
  weak native host versus CLR host/GC/retained-callback lifetime and all earlier
  runtime gaps remain unresolved. No alternative router/task/docking engine exists.
- Seventeen navigation/model tests include all seven original cases; six busy-root
  tests cover precedence, pending/error tasks, real command execution, pending
  dialogs/confirmations, cancellation order and failure/reentry. Workspace validation
  is now 794 tests. The complete-port gate must continue to reject this subset.


Activation/dispatch/Splat continuation:

- IActivationBridge and IDockDispatcher contracts are present. IDockDispatcher
  remains in the reactive-model services crate/module as in source; its action is
  an Rc Fn and its result the real shared System Task. No UI dependency is added.
- DockActivationBridge simply returns the actual activator's Activate result. It
  does not inspect, subscribe to, change or retain the view. Native shared weak
  activator scope lifetime versus CLR retention after model release remains partial;
  future UI activation/fetcher/leak tests must validate it.
- MainThreadDispatcher reads RxSchedulers.MainThreadScheduler on every call and
  always schedules through it, regardless of synchronization context. Do not use
  ServiceDispatcher's captured-context/inline path here. Action failures fault the
  task; scheduler failures propagate synchronously. System 0.3.0 raw Task lacks
  RunContinuationsAsynchronously, still a required unresolved source behavior.
  The private overlay DispatcherInvocation Future does not close this public gap.
- Services now directly depend on shared Splat exactly 0.3.0. Nine lazy singleton
  registrations preserve source order and default-contract HasRegistration checks:
  three globals, resolver, Func, provider, window lifecycle, activation, dispatcher.
  Existing registrations, even null-returning ones, must never be overwritten.
- HostOverlayServicesFactoryFn is the Splat generic key for the unsized Func; its
  Rc result is the existing HostOverlayServicesFactory alias. Every invocation
  creates fresh local busy/dialog/confirmation services, then passes globals in
  source order: busy, dialogs, confirmations, busy, dialogs, confirmations. Do not
  prefetch/deduplicate the second lookup set or resolve dependencies at registration.
- Typed concrete or erased IDependencyResolver handles support the extension method.
  Arbitrary erased IMutableDependencyResolver requires explicit same-allocation
  IReadonly projection. The mutable_only overload represents the source nonreadonly
  branch and reads the CURRENT shared Locator on every required lookup. Missing
  local readonly services fail, without silently falling back to Locator.
- Registration and Func closures retain the original resolver even when copied by
  ModernDependencyResolver. Preserve this ownership/lookup context. Weak captures
  would break copied registrations after the original external handle is released.
  Native unreachable resolver/closure cycles still lack CLR cycle collection;
  explicit container Dispose releases registrations. Do not add another container
  or pretend native pruning resolves GC cycles. FullName/exception metadata and
  cross-thread semantics also remain partial.
- Ten DI, four dispatcher and three activation cases bring the workspace to 811
  tests. The registered production factory/provider/lifecycle/root integration
  cancels pending dialog/confirmation tasks and disables root reload after event
  delivery. This is not a Headless/UI/leak test or a full parity claim.
- DockOverlayServiceCollectionExtensions.cs stays pending: the inspected published
  System/Splat crates contain no Microsoft IServiceCollection, and the reactive
  guide excludes Splat DI adapters. Resolve the shared dependency faithfully; do
  not map its API to a narrower homemade container or waive that source file.
- Next inspect framework-property models and control integration, including the
  framework ReactiveUI control/resolver adapter, then real Dock controls, themes
  and compiled markup. Keep all source members, engine callbacks and named template
  parts, validate with FerroUI Headless and eventually desktop interactions. All
  original serializers, alternative model stacks, samples and earlier gaps remain.


Reactive framework control / Headless continuation:

- New crate: FerroDock.Model.ReactiveUI.Services.Ferro, with published Base,
  Controls and ReactiveUI.Controls exactly 0.3.0. Headless is dev-only, also 0.3.0.
  The older dirty sibling lacks this release's integration; use registry sources
  under ~/.cargo/registry for the actual pinned API, not sibling source guesses.
- DockReactiveUserControlBase derives from actual ReactiveUserControlBase through
  ferro_class and complete inherited virtual contracts. DockReactiveUserControl<VM>
  is its shared ReactiveView typed handle, matching the framework's generic-view
  convention and PartialEq requirement. A derived class test preserves outer native
  identity, inherited ViewModel/DataContext sync and activation.
- Parent construction must remain. The source WhenActivated scope resolves the
  bridge ONCE, adds SerialDisposable BEFORE subscribing to actual ViewModel changes,
  gets the next bridge/fallback activation before assigning/disposal of the prior
  handle, and uses Empty for null/nonactivatable models. Detach disposes resources
  and the observation subscription; reactivation gets the then-current bridge.
- Existing HostTypeRegistry now exposes reference<T> for canonical Rc<dyn Any>.
  Register same-object IActivatableViewModel membership BEFORE binding/activation;
  the view helper also calls shared ReactiveUI register_activatable_view_model.
  The shared automatic query is private; unregistered CLR interface discovery is
  still a metadata gap. Do not duplicate the shared activation/type engine.
- Runtime type, namespace, assembly and public Rust paths are registered. Abstract
  class metadata deliberately has no default constructor. Shared build-scan 0.3.0
  exports the library through Cargo links metadata and its three framework metadata
  dependencies. Actual dependent compiled-XAML consumption is still unverified.
- Four real Headless tests execute windows, Loaded/Unloaded, property changes and
  dispatcher jobs; they cover captured bridge/replacement/order, original control
  argument, fallback, empty/nonactivatable/null, detach/unsubscribe, reactivation,
  model release with the view alive and subclass metadata/behavior. Workspace total
  is 815 tests. They do NOT port DockControlActivationOrderLeakTests or prove the
  original full docking/application/desktop/leak harness.
- macOS debug linking emits a nonfatal __eh_frame compact-unwind-offset size warning.
  Tests pass; exception-path performance and broader target builds remain unverified.
  No warning suppression or broad profiling changes were introduced.
- Source framework host resolver is still pending because real DockControl,
  OverlayHost, HostWindow and ManagedWindowLayer are absent. Port those controls,
  preserving template parts/property contracts and actual engine callbacks, before
  wiring its exact tracked-control/ancestor/top-level/descendant lookup algorithm.
  No narrower fixture-based production controls or alternative layout/UI engine
  have been introduced. All themes, persistence, model alternatives, samples and
  earlier runtime/metadata/ownership/DI requirements remain in scope.


Proportional panel/splitter/compiled theme continuation:

- New crate FerroDock.Controls.ProportionalStackPanel contains the actual Panel,
  Thumb splitter and preview Control, eight source internal helpers, real XAML
  ResourceDictionary and native metadata. Uses published Base/Controls/Markup.Xaml/
  Build/Headless exactly 0.3.0. No separate layout/input/renderer engine is added.
- Main panel preserves Vertical default, TwoWay attached Proportion/IsCollapsed/
  CollapsedProportion, current-value proportion updates, source class handlers and
  finally-style assigning flag. Five virtual sizing/resize hooks dispatch through
  the full native class. Measure uses the source repeated dimension/fraction calls;
  indexed children remain live. Keep source normalization/collapse/constraint order.
- Splitter retains all six properties/defaults/inheritance, source parent-first
  input/property/attachment calls, presenter child discovery, cursor updates and
  accumulating orientation pseudo-classes. Six private constraint cases are tested
  via native child test access, with no introduced reflection.
- Preview construction must finish the object initializer before assigning the
  splitter field. An opacity callback regression checks that ordering. Reread fields
  at source callback boundaries after removal/property callbacks. Native raw release
  can also deliver capture loss; preserve source cleanup guards/order rather than
  inventing different cancellation or rollback behavior.
- Preview uses actual adorners/logical parent, explicit brush then host accent then
  Gray, and source rectangle geometry. A test recording core delegates all operations
  to real Headless drawing while checking geometry/brush commands. Opacity/property
  updates, disabled-drag capture cleanup and ordinary commit with intervening layout
  jobs are tested. These are not desktop pixel/GC/performance proofs.
- The source splitter ResourceDictionary is XAML with explicit typed template/theme
  and compiled-binding scopes. Shared Build Model compiles it against library and
  dependency metadata. Its actual compiled URI is
  ferres://FerroDock.Controls.ProportionalStackPanel/ProportionalStackPanelSplitter.xaml.
  A Headless case loads/applies it and verifies TemplateBinding on the actual Border.
- Use the standard named NAMESPACES constant in register_types.rs, not an inline
  runtime-only table: custom XML type resolution failed until the scanner could
  read that constant. The earlier reactive framework crate now follows the same
  pattern. Generated Rust uses the standard crate-self alias for this library's
  own class references. Public classes, namespace, assembly, paths and compiled
  loader registration are present; full external docking consumers remain pending.
- Source non-splitter enumeration uses an internal iterator over shared COW storage
  with identity checks. Content replacement invalidates, with no added notification
  subscriber or copied collection engine. Exact original mutation version for
  capacity-only/storage changes remains unresolved, so utils/manager stay partial.
  Original Debug.Assert diagnostics are also deferred (do not silently turn them
  into new user-input panics).
- 26 new cases cover all four source policy methods/ten rows, both sizing methods/
  three rows, six private splitter constraint cases, exact basic/nested rectangles,
  all eight border rows, collapse/expand and min/max assignment, numerical edge
  behavior, source mutation detection, presenter and skipped-sibling navigation,
  native pointer/preview, compiled theme/binding and reentrant initialization. Total
  workspace tests: 841. Native case grouping differs from xUnit theory test counts.
- The original ProportionalStackPanelTests ItemsControl case is pending, as are the
  remaining full MinMax and SplitterHeadless suites, runtime model-binding tests,
  desktop/pixel/leak and broader target checks. Do not claim those from the smaller
  supported presenter or view lifetime tests. Main Dock controls/hosts/OverlayHost/
  resolver/themes, persistence, model alternatives, samples and earlier gaps remain.


Proportional integration and downstream markup continuation:

- All eleven ProportionalStackPanelTests methods now have native counterparts.
  The original ItemsControl test has NO template: its expected item bounds are
  zero. Preserve this case; the separate real ItemsPresenter regression arranges
  nested proportional panels in a Headless window and checks exact rectangles.
- All seventeen MinMax cases and all fourteen SplitterHeadless cases are mapped.
  Rows consolidate equivalent source cases but retain input dimensions, limits,
  proportions, drag deltas, operation order and movement/zero-delta assertions.
  Private splitter access is a child test module, not reflection.
- binding_tests.rs uses the production Document model, native compiled paths made
  from typed accessors, and shared INPC observation. Both orientations at local
  and style priority prove default TwoWay metadata, normalization/resize writes
  to the model, later source updates, DataContext replacement, old subscription
  removal and disposal. No view-model adapter or alternative binding engine.
  This does not prove automatic erased IDockable/model metadata or the full
  generated container style from the original docking theme.
- New workspace test crate tests/Dock/FerroDock.Markup.Tests compiles against exported
  dependency metadata with published Build Model. ProportionalConsumer.xaml uses
  the actual custom namespace, attached property literals/compiled paths and a
  ResourceInclude of the source library's splitter dictionary. Native loading,
  implicit theme/Border creation, exact layout and two-way updates all pass.
- Its own document URI uses the compiler's default assembly name:
  ferres://ferrodock_markup_tests/ProportionalConsumer.xaml. The splitter library
  URI retains its explicit FerroDock.Controls.ProportionalStackPanel assembly.
  Compiled resource boxes can hold base object handles: use shared ValueTypes
  projections instead of assuming every object is boxed as Ref<Border>. An
  implicit theme does not set the explicit Theme property; verify its built
  visual child rather than requiring Theme to be non-null.
- The test crate is publish=false. Its fixtures are integration evidence, not
  replacement production docking controls or an application. Proportional crate
  reactive-model dependencies are dev-only; UI-independent production model
  dependencies stay unchanged. Workspace passes 862 tests, Clippy, formatting,
  generator and inventory checks; structural count is 152/120/1656. The macOS
  debug linker warning is unchanged. All other full-port requirements remain.


Native recycling controls continuation:

- New FerroDock.Controls.Recycling uses actual FerroObject classes, native direct/
  styled/attached properties, shared System Dictionary, shared templates, actual
  visual/logical parents and SetCurrentValue. No alternative template/property/
  visual tree/cache engine. All original source cache, dispatcher, detach, fallback
  and template wrapper bodies are present; production records stay partial for
  the explicit native/runtime/complete-consumer gaps.
- Keep source order: cache hit updates a recycling template with cached control;
  null update retains old control; a different update is cached BEFORE detach.
  Cache misses pass null existing to the first template even when Build received
  an existing control. Failed detach builds fallback, trying existing then null
  for recycling templates, then a fresh rebuild if needed. Null fallback leaves
  prior cached value intact. Wrapper templates have their own initial-null and
  existing/rebuilt checks; do not conflate these with cache fallback semantics.
- Detach visual ContentPresenter first, even with a different logical ContentControl
  host Content. Then logical/visual fallback handles Panel, presenter, matching
  ContentControl and matching Decorator; unsupported parents return false. Clearing
  a presenter/content uses current value so binding survives, followed by UpdateChild.
- ID lookup applies to Build/Remove only. Add/TryGetValue take literal keys. Empty/
  whitespace identifiers keep the original data key; identifiers are not trimmed.
  Key equality uses shared AnyValue and native object identity; scalar hashes and
  native class handles use real hashes. Shared AnyValue has no custom hash API,
  so other same-type values share a bucket: equality is retained, arbitrary custom
  hash performance and equality reentrancy remain incomplete.
- Model contracts stay UI-independent, using Rc<dyn Any>. Native properties use
  Rc<dyn AnyValue>. to_recycling_value/from_recycling_value preserve native values
  through the original contract; raw scalars and registered native class handles
  also project correctly. A transient AnyValue class probe uses shared ValueTypes
  to recover the actual native handle and is NEVER stored as a model/property.
  Unknown raw domain objects retain reference identity but automatic typed UI/
  equality metadata remains open. Test covers raw derived native IdData/Border
  through the original IControlRecycling trait, not just the inherent native API.
- Canonical weak-cached Rc adapters retain native class objects without an adapter
  self-cycle. They implement ORIGINAL model/shared template traits. The model
  interface adds only default as_any interop and ID-provider reference PartialEq.
  Source strong Parent/cache ownership remains; CLR cycle collection is not proven.
- Foreign model traits cannot implement Base's foreign MarkupTyped trait directly
  under Rust's orphan rule. UI-owned EMPTY metadata carriers expose the ORIGINAL
  Rc trait handles, native namespace and identity. They have no behavioral objects
  and do not replace the contracts. Standard shared BuildScan exports them. Full
  interface member/binding metadata remains open. Do not add a Base/Controls
  dependency to the domain model just to make its metadata declaration compile.
- Tests cover all 15 original cache cases, five wrapper cases, actual ManualPresenterHost
  subclass/default template, compiled binding preservation and extra fallback/null/
  existing/cache ordering, metadata, inheritance, keys, retention and native projections.
  Theory-like rows are grouped into 19 native functions. Full workspace: 882 tests.
- RecyclingConsumer.xaml in the separate downstream package loads the exported
  classes, direct bool property, inherited attached cache and compiled #Presenter
  Parent binding, then changes content away/back and proves cached identity. Its
  same-control forward StaticResource resolves during actual native initialization;
  query the assigned cache AFTER attaching/showing the window.
- Shared 0.3.0 generated XAML Rust emits identical nullable setter branches and a
  Clone::clone for a direct bool literal. Only the generated module in the test
  package allows clippy::if_same_then_else and clippy::clone_on_copy. Handwritten
  port/test code remains under workspace -D warnings. The previous nonfatal macOS
  debug compact-unwind size warning remains unchanged.


Native overlay host/layers continuation:

- New src/Dock/FerroDock.Ferro (ferrodock-ferro) contains actual Decorator/FerroObject
  classes, owned original layer/content-host/factory contracts, original layer
  keys, provider registry, and native collection wrappers. All UI calls go through
  shared Base/Controls/System 0.3.0. The full Dock UI assembly is still partial.
- OverlayHost preserves all five properties/defaults and Content/DependsOn metadata.
  Constructor creates the native notifying overlay list. OverlayLayers getter
  falls back to the current private list; a structurally equivalent replacement
  changes the PUBLIC property but retains the OLD private subscribed collection
  and pipeline. Count, z, visibility, input, style and exact native control type
  govern equivalence; do not change this to reference equality or use the new list.
- Rebuild requests while rebuilding set the follow-up flag; ordinary requests
  coalesce to one normal-priority native dispatcher job. Queued callbacks retain
  the host as source does. Attachment rebuilds synchronously; detached requests
  do nothing. A Drop guard resets the rebuilding flag on errors. Native callbacks
  reproduce the original detach-time mutation probe with maximum depth one.
- Source ordering: service factories first, then provider (preserve a nonempty
  native collection's identity/subscriptions), then explicit layers, then raw
  overlay controls. A selected nonempty collection containing only invisible/
  null-overlay layers suppresses lower-priority sources. Cached service collection
  becoming empty falls back without reinvoking the provider; later mutation of
  that SAME collection is still observed. Null factories/results are filtered in
  the original snapshot/create order. Reset unsubscribes before clearing the field.
- GetOrderedLayers returns a repeatable query: first next buffers all source
  entries BEFORE invoking any key getter, captures z/index and sorts stably.
  Getter mutation does not change that buffer; a new enumeration sees changes.
  Entry projection retains repeated getters in source order (including the second
  IsVisible read whose stored value is unused): z-sort, visible/overlay filter,
  overlay,z,visible,blocks,style. A trace regression protects those callbacks.
- Pipeline branches retain internal IOverlayContentHost before ContentControl,
  Decorator, then Grid. Wrapping clears ContentTemplate; plain overlays receive
  IsHitTestVisible, and nonzero z is assigned without resetting z on later zero.
  Themes are looked up on the host and applied before insertion. Strings, native
  ResourceKey, type metadata/registered type handles and theme-variant keys work;
  arbitrary raw unregistered object keys/equality remain an explicit gap.
- OverlayHost detach helper deliberately differs from recycling: logical parent
  precedes visual parent, Panel.Remove result is ignored, and matching ContentControl
  content is cleared by a normal value. Presenter content uses current value plus
  UpdateChild, retaining the source visual-parent checks. Do not merge the two
  libraries' helpers and thereby change these source branches.
- OverlayControlList keeps actual native FerroList or a complete original
  ListHandle backend, including plain/custom/read-only/notifying lists. The host
  subscribes directly to the chosen engine (no added version-bridge subscriber).
  Native COW/storage checks are not exact original mutation versions. Collection/
  interface inheritance, failure/retry enumeration and broader metadata remain open.
- Registry keeps source static API through shared native thread-affine storage and
  shared Event. IServiceProvider is the existing Base contract, not a new DI
  container. Cross-thread global provider replacement/UI repost and complete CLR
  delegate equality remain partial; do not claim this from native UI-thread tests.
- A third downstream consumer compiles real host/layers, both collection Add
  boundaries and original interface projections, content metadata and PascalCase
  static BusyLayerThemeKey. Plain collection nullable forms MUST be registered
  with shared ValueTypes::register_nullable in the host's static constructor, so
  runtime and BuildScan both see them. No custom compiler/type-system model is added.
- All four OverlayHostRebuildPipelineTests method bodies have native behavioral
  counterparts. Keep that source test file PARTIAL: its ORIGINAL App.axaml loads
  FluentTheme plus DockFluentTheme, including implicit built-in service overlays;
  default service layers/request templates/full theme aggregation are still
  incomplete. Additional native cases validate
  explicit class/service/list behavior and acyclic release after detach. The full
  original leak/GC, themed app, desktop/input/pixel and target harnesses stay pending.
- Validation: 901 Rust tests, Clippy (only previously scoped generated-emitter
  allowances), format, structural inventory/generator and six verifier tests pass.
  Nonfatal macOS debug unwind warning is unchanged. Original C# remains unchanged.

Service overlay controls and compiled source themes continuation:

- BusyOverlayControl, DialogOverlayControl and ConfirmationOverlayControl now use
  native TemplatedControl, all original styled/read-only direct properties,
  original field defaults, Content/DependsOn metadata and parent-first visual
  lifecycle. The new domain IVisualTreeLifecycle contract stays UI independent.
  Canonical native adapters implement that original contract and the existing
  internal IOverlayContentHost, so OverlayHost composes the real controls.
- Service property changes subscribe even before visual attachment. Attachment
  rebinds; detachment unsubscribes without resetting busy state. Dialog and
  confirmation detachment clear only the request collection, retaining the other
  direct fields. Same-instance assignment does not add a second subscription.
- Preserve source getter/notification order. Sync captures initial local/global
  booleans but rereads attached fields between direct-property notifications.
  Busy ReloadCommand is read twice; ShowReloadButton skips the service visibility
  getter when explicitly set; CanReload is short-circuited. A trace probe and
  reentrant service-removal test protect these details. Empty/unrelated names,
  including ReloadCommand, do not trigger busy synchronization.
- Keep ReloadCommand as the ORIGINAL ReactiveUI ICommand handle. Do not create
  an adapter on every sync: that changes identity/notification behavior. The UI
  binding conversion uses the PUBLISHED ReactiveCommandAdapter instead. Shared
  0.3.0 exposes two ICommand traits where C# has one; their metadata handles are
  distinct (ReactiveUI versus native System.Windows.Input). Full generic/erased
  command/interface metadata remains partial.
- UI-owned empty foreign-contract carriers expose original domain service,
  lifecycle, command and specialized read-only request collection handles. They
  do not replace behavioral contracts or introduce Base into the domain layer.
  Full service member/binding notification metadata is still pending.
- request_items.rs adapts ReadOnlyObservableCollection directly to shared
  IItemsList/ItemsSource. Count/index are live, identity is the underlying
  read-only wrapper identity, notifications forward borrowed typed item views,
  and unsubscribe removes the original token. No copied collection/notification
  engine. A domain identity accessor supplies this native adaptation boundary.
- New FerroDock.Ferro.Themes.Fluent compiles the original three service overlay
  dictionaries using the shared Model compiler, exact source visual hierarchy,
  resource keys, z-order and bindings with explicit typed scopes. Its assembly
  name is FerroDock.Ferro.Themes.Fluent. The full DockFluentTheme class and resource
  aggregation DO NOT exist yet. Palette/geometry resources and request templates
  remain external source resource keys, not replacement visuals.
- ServiceOverlayConsumer.xaml is the fourth separate downstream consumer. Actual
  Headless windows prove original themes load, busy text/progress/input/reload
  track production services and execute their original command, request lists
  preserve identity/live Add/Remove notifications, and null service replacement
  clears commands/items. Eleven native service-control tests plus this consumer
  raise the full workspace total to 913; 29 native overlay and four XAML tests.
- Native subscriptions capture weak controls and remove tokens when native
  ownership ends. This avoids stale native callbacks but is NOT proof of the
  original strong event retention/CLR cycle collection semantics. Arbitrary
  custom event-accessor reentrancy and full original leak harness stay open.
- Next: port original DialogShellControl and ConfirmationDialogControl, their
  themes plus OverlayDataTemplates and required palette/geometry dictionaries;
  then OverlayLayerDefaults/Styles with typed IHostOverlayServices bindings.
  Retain the original themed App.axaml fixture as incomplete until full theme
  aggregation and default service layers are present. DockControl/real native
  hosts/resolver, full desktop/input/pixel/target work and other inventory items
  remain pending. Do not mark the goal complete.
- Validation: 913 tests pass, no failed/ignored; locked Clippy, fmt, structural
  audit, generator and six verifier tests pass. Inventory: 1,928 inputs, 161 ported,
  137 partial, 1,630 pending. The completion gate correctly rejects unfinished work.
  The shared theme emitter also clones Copy literals; only its generated module
  allows clippy::clone_on_copy. Handwritten sources remain under -D warnings.
  Existing nonfatal macOS debug unwind warning remains. C# source is unchanged.

Rendered request shells and default service-layer continuation:

- DialogShellControl and ConfirmationDialogControl now are actual native
  TemplatedControls with every source styled property/default/get/set, native
  class export and original Content/DependsOn metadata for the dialog shell.
  These shell command properties use the existing Base UI ICommand, just as
  native Buttons do. Request bindings use the published ReactiveCommandAdapter;
  no new command execution engine or click handler is introduced.
- UI-owned class-descriptor carriers expose the ORIGINAL Rc<DialogRequest> and
  Rc<ConfirmationRequest> objects with the immutable properties used by the
  source DataTemplates. The carriers have no behavioral instances. Content uses
  the existing recycling native value bridge, preserving real control identity;
  arbitrary raw domain object metadata remains incomplete. The model crate does
  not depend on UI. GlobalHandle/Task/Complete and full request metadata are still
  tracked separately; do not mistake these binding descriptors for the full API.
- Source DialogShellControl, ConfirmationDialogControl and OverlayDataTemplates
  dictionaries are compiled with native namespaces and explicit typed scopes.
  Original visuals, resource keys, title/message/labels, close visibility,
  command bindings, shadows and content composition remain intact. Full Compact
  dictionary is also present with ALL original tokens, not an extracted subset.
  The source confirmation uses DockConfirmationDialogMaxWidth (440 compact),
  whereas the ordinary dialog uses DockDialogMaxWidth (560 compact).
- Headless consumer application now includes the PUBLISHED FerroUI FluentTheme
  0.3.0. Without its ItemsControl template no actual request presenter is created;
  item counts alone do not validate rendered dialogs. Cargo.lock records its
  transitive Dialogs/platform dependencies. No source is copied from shared crates.
- Two additional downstream tests display source request templates and use actual
  headless keyboard Space press/release on native Buttons. Close completes the
  production dialog task with null; confirmation/cancel complete true/false.
  They also verify original native content identity, title/label display, runtime
  title/close-visibility updates, command identity and compact sizing. These are
  actual input flows, not direct command invocation. Nonnull raw native command
  parameters still have the earlier domain borrowed-object/canonical-Rc gap;
  null close is the actual parameter of the original shell template.
- OverlayLayerStyles and OverlayLayerDefaults are compiled from source. Defaults
  retain source nonshared collection, three source z/style/order entries and the
  implicit OverlayHost theme. Reflection bindings were replaced by compiled
  scopes of the ORIGINAL IHostOverlayServices. Its UI-owned descriptor exposes
  the six original getters; register_nullable for all original Rc service handles
  supplies normal nonnull-to-nullable styled binding conversions.
- DefaultOverlayConsumer.xaml is a fifth separate XAML fixture. The two-host test
  proves fresh layers/controls, all six original service identities, actual alias
  themes, local/global busy behavior, source rendered confirmation keyboard result
  and DataContext rebinding. INSPECT THE DISPLAYED PIPELINE: source equivalent
  collection replacements may keep the OLD private/subscribed pipeline while the
  public property holds NEW detached equivalent overlay objects. Do not change
  production behavior or assume public-list controls are the displayed controls.
- Full workspace: 918 tests, no failed/ignored (31 native overlays, seven external
  XAML tests). Locked Clippy, formatting, inventory, generator and six verifier
  tests pass. Inventory: 1,928 inputs, 162 ported, 144 partial, 1,622 pending. Known
  nonfatal macOS debug compact-unwind warning remains. C# source stays unchanged.
- Next: port the full accent/palette/geometry resource dictionary and remaining
  native docking controls/resources needed for the true DockFluentTheme and
  original App.axaml harness. Accents/Fluent includes an original Domain
  DockOperation enum resource; its full UI-owned foreign enum metadata/export
  needs resolution without adding UI dependencies to Domain, replacing the enum,
  substituting an integer resource, or dropping that resource. Do not create a
  partial DockFluentTheme class that silently excludes unported dictionaries.
  DockControl/native-managed hosts/exact visual resolver, full model metadata,
  serialization, diagnostics, samples, GC/desktop/pixel/target requirements and
  every earlier inventory item remain in scope. The goal remains active.

Full accent/preset resources and foreign-enum metadata continuation:

- The complete original Accents/Fluent dictionary now compiles unchanged apart
  from native/System/model namespaces and the compiled-binding setting. ALL source
  colors, aliases, dimensions, booleans, icon geometries, ItemsPanelTemplate and
  typed DockOperation resource remain; no integer substitute or omitted key.
  Both rendered overlay consumers now load it before compact overrides.
- All five original Presets/Ide dictionaries and all ControlStrings are also
  present. Presets must load through their original resource dependency order:
  a ResourceInclude of the accent dictionary followed by the preset, in a shared
  native owner context. Loading Default alone loses its eager StaticResource
  lookups; do not change original aliases or inject replacement values to hide it.
- Rust's orphan rule prevents the UI crate implementing Base MarkupTyped for the
  foreign original model enum. Shared BuildScan 0.3.0 also explicitly cannot read
  handwritten MarkupTyped implementations. The new build-only foreign_enum_build
  bridge generates an EMPTY local descriptor marker, standard runtime MarkupType
  and standard build TypeModel of Enum kind from the SAME original Rust enum.
  Every member constant/getter returns the ORIGINAL DockOperation type, not a
  wrapper enum. Generated const assertions compare all numeric values with what
  Rust actually compiles; numeric conversion calls the original TryFrom.
- DockOperation has one hidden immutable declaration-source constant. The UI
  build dependency reads that constant from the actual model package, so no
  sibling source checkout is needed for packaging. Domain gains no UI dependency.
  syn reads only this original Rust syntax during the build: no runtime reflection,
  alternate XAML parser, type system, enum engine or serializer is introduced.
  The current generator handles this enum's explicit i32 unit-variant source
  form and fails for review if that form changes; other foreign enums remain work.
- The UI build still runs the standard Export source scan. Its output is kept
  under OUT_DIR/scanned; the additional enum TypeModel is appended through shared
  AssemblyModel APIs and written in the normal .xamlmeta schema. Shared Model
  compiler/loader and existing class/interface exports remain in use. The runtime
  descriptor and exported model are generated from one declaration, not maintained
  as two hand-written value tables. Generated PascalCase constants are source
  markup names; their non_upper_case_globals allowance is confined to that impl.
- Two new unit tests validate original runtime handles/nullable type, all eleven
  original values, original TryFrom failure boundaries, descriptor registration
  and build-model/runtime parity. ThemeResourcesConsumer compiles all eleven
  original enum values in a DIFFERENT Cargo package, proving export/emission and
  actual payload type rather than just descriptor strings.
- Downstream resource tests also verify source dialog dimensions, brush alias
  BOX identity, live DynamicResource color updates, every source icon geometry,
  control string access-key text, all preset resources and distinct editor colors.
  Constant brushes may be immutable values in the shared compiler: inspect them
  through the native IBrush/ISolidColorBrush contracts. Dynamic workbench brush
  remains the native mutable brush and updates after resource replacement.
- Workspace validation: 922 tests, no failed/ignored (33 native UI tests including
  enum metadata, nine external XAML tests). Locked Clippy/format/inventory/generator
  and six verifier tests pass. Inventory: 1,928 inputs, 169 ported, 144 partial,
  1,615 pending. Existing nonfatal macOS debug unwind warning remains. Source C#
  stays unchanged. Undefined numeric enum values and broader raw/erased/generic
  metadata/runtime/GC limitations remain explicitly outside current evidence.
- Next: implement remaining native docking controls/behaviors, template parts,
  accessibility peers and host/resolver dependencies; assemble the REAL complete
  DockFluentTheme only when its source dictionaries/types exist. Palette metadata
  no longer blocks it. Full original themed App.axaml, theme-manager API,
  DockControl/native-managed hosts, full desktop/input/pixel/target integration,
  serializers, alternate models, diagnostics/samples and every earlier inventory
  item remain in scope. No partial replacement DockFluentTheme is supplied.
- Incremental verification: touch-only change to the bridge build input caused an
  actual rerun (new shared scan report). All 21 existing generated runtime/model
  artifact hashes and mtimes stayed unchanged. No content mutation was needed.

Native visual-tree lifecycle behavior continuation:

- VisualTreeLifecycleBehavior now is a native static attached-property owner in
  FerroDock.Ferro.Behaviors. Its original IsEnabled property/default/get/set and
  Control-filtered class handlers use shared Base property/events/type metadata.
  A separate compiled XAML consumer resolves and loads the original attached API.
- Preserve exact source order: attach invokes the control hook FIRST and reads
  its CURRENT context afterward; detach invokes the context hook before control.
  Both helpers exclude a context representing the same element. Native base and
  derived handles and canonical original interface handles share this identity.
  DataContext change handling deliberately does NOT have that exclusion: old
  detaches then the EVENT's new context attaches, even if either is the element.
- Do not snapshot the context before the control attachment callback or reread
  current context instead of the immutable notification's NewValue. Tests cover
  control callback replacement and nested old-context replacement, retaining the
  source's apparently repeated callbacks. Do not add cancellation/deduplication.
- Disabling first removes attached then detached subscriptions, before calling
  any lifecycle hook. A context detach callback can reparent the element without
  an extra tree-event callback. Conversely, disabling INSIDE a control attach hook
  does not cancel the source helper's remaining context attach hook. A regression
  preserves that exact continuation after the reentrant disable.
- DetachedFromVisualTree fires before PresentationSource is cleared in both the
  published shared Visual implementation and the actual source dependency version
  (Directory.Packages.props names 12.1.3). Therefore a context replacement INSIDE
  a detach hook still follows the attached-context branch: old-, old-, new+,
  control-. This is SOURCE behavior, not a dependency mismatch or a new bug to
  "fix". Verified against the primary versioned source:
  [Visual.OnDetachedFromVisualTreeCore, 12.1.3](https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/Visual.cs#L527).
- Native event APIs remove IDisposable tokens instead of static C# delegates.
  Private thread-affine weak token bookkeeping bridges that API: no added
  observable control properties, strong control/model ownership or reordered
  class handlers. Removal takes the last pair, as delegate -= does; expired weak
  entries prune on subsequent enable changes. No map borrow spans user callbacks.
  This is native subscription bookkeeping, not a new lifecycle or property engine.
  Full CLR delegate identity, GC and pruning cost equivalence remain unverified.
- Native interface discovery uses original Rc<IVisualTreeLifecycle>, actual
  native class/interface projections and registered typed model casts through
  shared ValueTypes. Automatic discovery for arbitrary UNREGISTERED raw models
  and self identity through NONCANONICAL custom adapters remain open. Do not add
  a smaller replacement lifecycle interface or UI dependencies to the model.
- Ten native Headless tests include the complete original show/remove/reattach
  method body, an original-contract native control and raw typed model fixture,
  immediate enable/disable/default/repeat behavior, callback ordering/reentrancy,
  same-element contexts in three native forms, detached/disabled/non-lifecycle
  handling and supported acyclic release while behavior remains enabled.
  The source finally Close is represented by a window Drop guard. The separate
  consumer adds compiled attached-property and original erased-model integration.
- Validation: 933 Rust tests, no failed/ignored (43 native UI, ten external XAML).
  Locked Clippy, formatting, inventory, generator and six verifier tests pass.
  Inventory: 1,928 inputs, 169 ported, 146 partial, 1,613 pending. Original C# is
  unchanged; the known nonfatal macOS debug compact-unwind warning remains.
- The original VisualTreeLifecycleBehaviorTests FILE remains partial because its
  exact App.axaml includes the complete DockFluentTheme, still unported. The method
  body is covered, not the full original app fixture. Next implement remaining
  native control/template/accessibility peers, host/resolver and source dictionaries
  needed by that genuine theme; do not substitute a reduced theme or app. Every
  broader engine/API/metadata/GC/serializer/sample/platform requirement remains.


Native root control and accessibility continuation:

- RootDockControl is a real native TemplatedControl with the original optional
  PART_MainContent ContentControl part. Apply and detach call their native parent
  first, remove the previous part handler, then replace/clear the field. The
  pointer handler is Tunnel and skips already handled events. It reads CURRENT
  DataContext and CURRENT original root factory and invokes the original engine.
- Preserve source detach behavior: it clears the part and removes the handler.
  Reattaching the same existing template does NOT re-register it; a fresh template
  does. Tests also reparent the old template part and verify it no longer invokes
  preview hiding. Do not add an attach-time subscription as a speculative fix.
- RootDockControlAutomationPeer uses shared ControlAutomationPeer: original fixed
  class name, Pane role, base-first name (including labels), explicit/model/current
  context name/ID precedence and base ID fallback. Help state retains every key,
  source order and a pinned-dock snapshot evaluated before subsequent getters.
  Source owners have a strong readonly field, but shared native peers retain weak
  owners. Do NOT add an Rc cycle and claim GC parity; peer-owner retention after
  releasing the control and desktop provider/leak behavior remain unverified.
- Original interface handles use model_projection::to_root_value/to_dockable_value
  at the native BoxedValue boundary and retain the original model Rc. Raw Rc<T>
  and boxed Rc<T> contexts additionally work with shared ValueTypes registrations
  in the UI composition root. No Domain UI dependency or replacement ViewModel.
  Broad automatic interface discovery and full generated root binding metadata
  remain pending. Generic cast helpers (including uninstantiated exported macros)
  cause the published scanner to mark casts unread and prevent downstream XAML
  proofs. Therefore none are exported; concrete application registrations belong
  at the composition root. Explicit interface boxing needs no type registration.
- All accessibility helper method bodies are present: name/ID/state, activation,
  container lookup and selection list. Activation re-reads the ORIGINAL owner
  factory after each callback. A source composed factory subclass hands each step
  to a different factory and verifies order. Actual engine activation is tested
  separately. Container lookup first uses ContainerFromItem, then scans LIVE
  items and stops at the FIRST equal/reference match even if unrealized. Real
  native Fluent item containers and a forced-fallback collection cover both paths.
- Registered enum metadata names precede general object display, matching the
  explicit source Enum branch. Full flags/undefined enum and arbitrary invariant
  Convert.ToString/IFormattable/float formatting remain partial. Do not infer
  general object parity from root state, which uses bool/integer/string values.
  Three otherwise unused helper methods have narrowly scoped dead-code allowances
  until their real source selection/items peers are ported; no placeholder bodies.
- Twelve native Headless cases include the original root automation method and
  actual preview dismissal/KeepVisible, current context/factory, optional/renamed
  part, handled input, retemplate/detach, derived controls, explicit/label/model
  names, IDs, raw/erased contexts, live counts, helper algorithms and callback
  factory replacement. One external consumer compiles/loads the exported class,
  retains original model identity and dismisses a preview with real pointer input.
  Its template is TEST-ONLY, not a reduced production RootDockControl theme.
- The source root theme is still pending: DockableControl, ToolPinnedControls,
  PinnedDockHostPanel, DeferredContentControl, PinnedDockControl, DockProperties,
  typed original root bindings and full resource/application integration remain
  needed. Original source App.axaml and all other automation test cases remain
  tracked as incomplete. Broader engine/API/metadata/GC/serialization/sample and
  platform requirements continue unchanged.


Native deferred-content continuation:

- Added the separate FerroDock.Controls.DeferredContentControl library with the
  original opt-out interface, budget enum, timeline/settings, inherited scheduling
  API and actual native ContentControl/ContentPresenter subclasses. All timing
  properties use shared signed 100ns TimeSpan; the original clamps/defaults and
  plain timeline fields are preserved. Settings static properties and default
  timeline are exported through standard shared metadata, not added properties.
- Preserve source queue behavior: enqueuing an existing target mutates its shared
  pending entry, due time/order/sequence; a flush snapshots due ENTRY identities
  and sorts by order then sequence. A removed/replaced entry is skipped, but an
  entry re-enqueued IN PLACE remains eligible in that snapshot. Item-count mode
  limits SUCCESSES, so blocked targets do not starve ready targets and are visited
  once. Time mode ignores item-count limit and checks elapsed work after EVERY
  attempted target; zero time still permits one attempt. Drop not-ready native
  controls instead of hot-polling; test targets may retain pending work.
- Native entry/token adapters are canonical while queued. Queue callbacks do not
  retain a RefCell borrow across arbitrary realization/property callbacks. Shared
  due-list version checks reproduce invalidated enumeration on recursive flush;
  do not silently snapshot replacement objects or alter source callback order.
- Source schedule versions invalidate old Background posts/timer callbacks. The
  earliest pending due time schedules work, earlier new work supersedes later
  work, and due leftovers use FollowUpDelay. Positive delays use native shared
  DispatcherTimer, then Background post. Delayed tests MUST use dispatch_task and
  yield to the platform loop: blocking the UI thread while only calling RunJobs
  does not pump native timer signals. No new scheduler, task or animation engine.
- Native host/presenter hooks preserve base-call order. Presenter Content and
  ContentTemplate changes capture requested values, restore applied state under
  suppression, then queue realization; Measure retains last DesiredSize until
  applied. Suppression is restored on unwind, matching source finally. A host's
  nested presenter does not enqueue independently. Default host template carries
  all eight original visual TemplateBindings and the named presenter part.
- ClearPresentation cancels pending work and removes the child while retaining
  host Content/ContentTemplate; repeat clear and pre-template clear are valid.
  Property changes or reattachment realize again. Standard ContentPresenter
  templates use the original fallback branch. Source custom templates must set
  both the presenter Name and its NameScope registration for the inherited
  ContentControl.Presenter API, as the original RegisterInNameScope fixture does.
- Reveal starts at 0.85 only when replacing an existing presented child. The
  source owned DoubleTransition subclass updates duration/easing on reuse;
  existing user DoubleTransition opacity entries remain unchanged. Actual shared
  CubicEaseOut/Transitions and render ticks complete opacity to one. First
  materialization and zero duration do not stage a reveal. Content opt-out skips
  queueing and opacity staging for original interface handles/registered casts.
- Twenty-eight Headless cases include queue algorithms, defaults/settings clamps,
  re-enqueue and callback replacement, delayed scheduling/version cancellation,
  host batching/reattachment, all four clear-presentation source theory cases,
  standard presenter fallback, real build limits, stale presenter child/measure,
  actual Fluent ItemsControl time budgeting, native template bindings, scoped
  inheritance/order/delay/null override and independent timelines, retemplating,
  opt-out, user/owned transitions and actual render-driven reveal completion.
  A separate consumer resolves unprefixed library types via the original assembly
  XML namespace, constructs/inherits a timeline, reads DefaultTimeline via x:Static
  and realizes/replaces native content after positive delay. The existing root
  consumer now uses REAL DeferredContentControl as PART_MainContent; its remaining
  outer template is still explicitly test-only, not the full source root theme.
- Remaining fidelity gaps: queue entry is UI-thread affine and verifies access,
  rather than source background-thread repost; default settings are native
  thread-local. Queue owner/target EnqueuedTimeline references are weak to avoid
  native Rc cycles, so CLR retention/GC is not established. Native timer vs
  Task.Delay transport timing, SystemTime vs DateTimeOffset precision/range/clock
  shifts, undefined enum values, overflow/exception edges, arbitrary unregistered
  raw opt-out objects, full runtime metadata and internal default-queue forwarding
  facade remain partial. No desktop, pixel, performance or complete platform claim.
- Original full-themed-app integration for document/tool content/control, MDI,
  SplitView, pinned/Dock/root themes, ToolChrome and HostWindow remains pending,
  as do alternate-model tests. Separate v11 project/package/signing compatibility
  is not inferred from this native crate. The full original objective stays active.
- Validation: 975 passed, zero failed/ignored; locked Clippy all-targets, format,
  inventory, generator and six verifier tests pass. Completion gate correctly
  rejects the unfinished port. Inventory 1,928: 170 ported, 153 partial, 1,605
  pending. C# unchanged; existing nonfatal macOS compact-unwind warning unchanged.


Native Settings dependency continuation:

- Added FerroDock.Settings as the separate source library. DockProperties is the
  actual nonsealed native FerroObject owner with all nine original attached
  registrations, defaults and TwoWay binding modes. IsDragEnabled/IsDropEnabled,
  DockAdornerHost and DockGroup inherit; target/area/indicator/operation do not.
  The source fields typed StyledProperty still register attached properties; the
  native AttachedProperty descriptor dereferences to StyledProperty and exports
  the real attached markup API. Keep the source getter/setter host asymmetry.
- DockSettings retains the original 26 public fields/properties and defaults,
  including process-wide state. A OnceLock/RwLock value store is not thread-local;
  storage locks end before user callbacks or capture destructors run. All ordinary
  values remain unvalidated, including negative/NaN distances and opacity. Do not
  add coercion, notifications or override synchronization not in the source.
- Host mode preserves root override, explicit global field, then UseManagedWindows.
  Source reads the global mode twice in its branch, so do not snapshot it. Owner
  policy resolves AlwaysOwned/NeverOwned before the legacy owner flag. Drag
  thresholds use strict greater-than and OR; PixelPoint minimum integer absolute
  value overflows and the second coordinate is skipped after a successful first.
- All 21 options are nullable/unset initially. Native plain Rc/Cell options expose
  constructor and property metadata, keeping shared object identity. All source
  WithDockSettings assignments stay in source order, skip unset properties and
  preserve the actual shared AppBuilder. Fourteen fluent methods are present;
  native Rust calls supply boolean parameters explicitly (source defaults enable
  with true). Callback registration through the returned builder is observed by
  the original, proving shared native builder identity.
- Original policy/preset/merging/window-drag enums retain exact numeric values and
  zero defaults, plus checked native conversion. Global settings deliberately use
  their explicit source defaults, which differ from enum-zero for some fields.
  Shared native KeyGesture retains original key/modifier defaults and parsing.
- DockLogger retains the enable gate, exact [Dock] category: message text and
  callback preference. LogDebug is NOT conditionally omitted by the source; both
  methods invoke callbacks in all configurations. The native global callback is
  Arc<Fn(String)+Send+Sync>, as process-wide native sinks require. Tests cover a
  worker thread, reentrant changes, last-capture destruction after releasing the
  settings lock and source exception propagation. Default debug-build stderr is
  a partial substitute for source Debug.WriteLine; platform trace equivalence is
  not claimed. Diagnostics callback/untyped writable-static-field metadata and
  broad CLR delegate/gesture/reference/invalid-null/enum edges remain partial.
- The foreign model enum bridge moved from FerroDock.Ferro into Settings, the
  lowest native dependency that uses DockOperation. The old hidden Ferro alias
  remains. Domain has no UI dependency; immutable declaration text travels via a
  packaged build dependency for DockOperation, DockFloatingWindowHostMode and
  DockWindowOwnerMode. Shared TypeModel.rust_path MUST be the ORIGINAL enum, while
  public_path addresses the metadata carrier. Using the carrier as declaration
  breaks Option<original enum> string conversion in the shared compiler. Keep
  original enum handles/runtime nullable descriptors and concrete registrations.
- Native UI build now uses normal shared Export; only Settings augments its own
  standard scanned model. Shared parsing/emission/loading remains unchanged.
  Native Fluent and both consumers using original core enum values declare a
  direct Settings dependency because emitted Rust names the descriptor owner.
  No duplicate enum descriptor, private UI/type engine or sibling patch is added.
- Settings test harness is the original EMPTY App subclass, source configure/
  Headless builder factory and native assembly fixture. Both property tests, all
  ten group cases and all three builder cases run with corresponding source
  assertions. All four settings resolver method bodies use the real ReactiveUI
  RootDock; the original Model.Avalonia root remains a separate pending fixture.
  Extra tests cover property metadata, reparent/clear/null, constructor/derived
  owner, defaults, 18-combination mode matrix, unvalidated drag/option values,
  integer overflow, all option branches, globals/threading and callback behavior.
- A NEW independent consumer depends on Settings/Model and shared framework ONLY,
  not FerroDock.Ferro or themes. It resolves unprefixed DockProperties through the
  original XML namespace, creates options with original nullable core/settings
  enum text and gestures, reads a true static field and proves actual default
  TwoWay compiled binding updates. It also verifies inherited flags/group and
  explicit adorner host. Existing twelve UI consumers still pass after metadata
  relocation; the settings consumer is the thirteenth across both packages.
- Actual incremental rerun verified: touching the Settings bridge input caused
  Cargo to execute its build script and refresh the shared scan report. The active runtime descriptor and scanned/final models retained hashes AND
  mtimes; all 18 recorded artifact files across existing build variants stayed
  unchanged. Source content did not change for that probe.
- Validation: 1,009 passed, no failed/ignored; locked all-target Clippy, formatting,
  inventory, factory generator and six verifier tests pass. Inventory 1,928 inputs:
  179 ported, 157 partial, 1,592 pending. The completion gate rejects the unfinished
  port; C# remains unchanged and the nonfatal macOS compact-unwind warning remains.
  Original package/target/signing/asset distribution and remaining controls/themes/
  hosts/selectors/diagnostics/engine-setting composition are still in scope.


Native item dragging continuation:

- Added native internal ItemDragHelper with every source method body: attach/
  detach, policy discovery, press/release/capture-loss, transform/ZIndex management,
  axis thresholds, target offsets/index selection, outside callback, bounds and
  scrolling. Actual original Factory.MoveDockable/MovePinnedDockable perform model
  changes. No replacement docking algorithm or mock source control is used.
- IFactory now has optional as_dock_item_drag_policy discovery for the existing
  IDockItemDragPolicy. Generated FactoryImpl/impl_factory! forward this hook,
  preserving composed downstream policy identity and virtual behavior. Default
  production factories return None. Generator drift checking still applies.
- FerroUI native IItemsList has no mutable API. The source IList fallback uses
  shared ValueTypes conversion to the original ListHandle<Option<BoxedValue>>,
  not copied items or a second drag engine. Public mutable_items_list::from_ferro_list
  wraps the SAME native FerroList; mutations keep native notifications alive.
  Native registration provides concrete String/BoxedValue/Option<BoxedValue>
  casts. Custom generic sources require explicit concrete composition registration.
  A test-only i32 registration demonstrates the path; no generic registration
  is placed in library code (which would poison unread generic markup casts).
  Original IList adapter notifications/version metadata remain incomplete.
- Native helper owns its original current dragged container/items/pressed state.
  Its owner is weak and event handlers retain helper; timer callback is weak to
  avoid introducing an inactive native timer cycle. Native event clones preserve
  shared routed Handled/Source state. Broad CLR identity/active-cycle/GC parity
  is deferred, not established by the inactive detach-release test.
- Attach registers all four original handlers with TUNNEL and false handledEventsToo.
  Native PointerCaptureLost is Direct; it does not invoke that source Tunnel handler.
  This is preserved. Tests prove registration behavior, then separately invoke the
  original capture-loss body with native arguments. Do not silently change event
  routes as a port fix.
- Detach removes one last matching subscription set and stops scrolling, without
  source Released/field resets. Nonleft release clears captured/pressed args but
  leaves enabled drag state; capture-loss callback body later cleans/commits it.
  ZIndex max excludes dragged item, unchecked i32 overflow wraps, SetCurrentValue
  becomes ClearValue on release; old transforms are overwritten with Translate(0,0).
- Auto-scroll uses exact 20px edge, 200px/s intensity, 16ms timer/fixed delta,
  source double min/max including NaNs/signed zeros, and Vector.Equals-like NaN
  equality for timer reuse. Tests cover replacement/clamping, actual native timer
  ticks, cancellation, descendant scroll extent and transformed explicit bounds.
  The source starts scrolling before testing scroll/container bounds.
- All eighteen source policy theory cases have helper-boundary counterparts using
  actual native TabStrip/TabStripItem and original ReactiveUI models/factories.
  Those are NOT the full original Model.Avalonia/DockControl/DockFluentTheme harness.
  Original ItemDragAutoScrollLeakTests and real tool/document tab subclasses are
  still pending. ItemDragHelper has a temporary module-only production dead_code
  allowance until source consumers are present; remove it when they are ported.
- Validation: 1,027 Rust tests; inventory 179 ported, 159 partial, 1,590 pending.
  Full completion gate must continue rejecting unfinished source inputs. The known
  macOS debug compact-unwind linker warning remains unmodified.


Native dock targets and floating adorner window continuation:

- Ported all DockTargetBase source methods, original abstract native class, all
  eighteen optional template parts, five styled properties, observable pseudo
  classes, protected operation dictionaries and extension helpers. The virtual
  default uses the shared native class vtable, including a real derived test class;
  local returns Window and global returns None. No substitute docking algorithm.
- Original template change invalidates/clears mappings before parent processing.
  OnApplyTemplate runs parent first, records a weak shared template and weak part
  arrays. Restoring the same template before layout restores only surviving visual
  descendants; retemplating discards removed mappings. Registration uses ATTACHED
  operation values, not part-name meaning. GetSelector verifies access, restores
  registrations without applying a template and returns hidden registered parts.
- Selector mode follows source dictionary order (Top, Bottom, Left, Right, Center,
  then outer parts). Last valid selector wins, other indicators are cleared, each
  registered selector remains visible unless policy/direction hides it. Direction
  gating precedes visible/validate callbacks. Indicators-only uses drop-control
  operation and coordinate translatability; it deliberately bypasses validation and
  hit containment. Shared-root transforms are composed through TopLevel; separate
  windows use actual screen/client translation; native visuals-at plus local bounds
  fallback matches the source algorithm.
- Uses shared System Dictionary, NOT HashMap, preserving ordinary source order.
  Get/set protected properties return/replace shared dictionary handles. Iteration
  reads live entries and releases RefCell access before callbacks; the published
  shared dictionary lacks exposed enumeration version/removal-slot state, so exact
  reentrant add/remove invalidation/slot behavior is still PARTIAL. Do not replace
  it with snapshots to hide this boundary or mark the base complete.
- Original IDockTarget, IDockAdornerHostProvider, DockOperationHandler and
  IDockAdornerWindowFactory are present; interface descriptors/nullable handles are
  registered in shared metadata. IDockTarget conversion uses a weak cached native
  adapter retaining its owner when held. Broad raw CLR implementer/method/delegate
  metadata and cross-adapter identity/lifetime remain open.
- Local DockTarget provides inherited host provider/theme descriptors and read-only
  notifying TargetMargin. The original internal setter is tested but has a narrow
  production dead_code allowance while AdornerHelper is pending. Remove it when
  that real caller is ported. Reset preserves ShowIndicatorsOnly/margin and resets
  all indicator/selector opacities and four original directional/global flags.
- DockAdornerWindow derives the actual native Window, constructing through native
  PlatformManager and preserving inherited window initialization. Its style key
  override pins the source DockAdornerWindow key. Factory attached property inherits
  through StyledElement scopes. DockTargetAutomationPeer provides the source
  focus-only Invoke contract; floating window peer stays decorative. Names/IDs,
  state/visibility and original native enum formatting are tested. Native base weak
  owner semantics do not prove source strong readonly owner/CLR GC parity.
- Fluent now compiles 19 complete source dictionaries. Added DockTarget,
  GlobalDockTarget and DockAdornerWindow, retaining source previews, templates,
  proportional global layout, selector images, conditional styles and window
  defaults. Five source PNGs are byte-for-byte identical and embedded via shared
  Build::embed_assets. Image URIs MUST use the real assembly name
  ferres://FerroDock.Ferro.Themes.Fluent/Assets/..., not the crate name. Native
  ResourceDictionary has one owner: external window-theme test looks up its native
  ControlTheme and assigns Theme, instead of sharing a parented dictionary.
- Sixteen native tests include the source headless target cases and additional
  algorithm/extension/property/accessibility/coordinate/lifetime checks. The
  source CLR-GC restoration step and original complete themed leak harness remain
  PARTIAL. The external consumer uses actual complete source target dictionaries
  and verifies all local selector operations, real images and sizes, horizontal/
  global flag styles, indicator-only panel visibility and original window defaults.
  This is not full DockFluentTheme, managed-window or desktop/pixel verification.
- Validation: 1,044 Rust tests, no failures/ignored; locked all-target Clippy, fmt,
  inventory, factory generator, six Python tests and diff/source checks pass.
  Inventory: 186 ported, 167 partial, 1,575 pending. Completion gate still rejects
  the unfinished full port. Existing macOS compact-unwind linker warning remains.


Native MDI layout continuation:

- Added original MdiResizeDirection flags, MdiLayoutEntry immutable native control/
  model pair, full IMdiLayoutManager, ClassicMdiLayoutManager and MdiLayoutPanel.
  All original classic manager method bodies use existing model list handles,
  actual native arrange calls and original ReactiveUI documents. No alternate
  engine, copied replacement input contract or simplified docking sample.
- Source minimized Where/OrderBy/ToList completes before a SECOND normal Where
  enumeration. Ordering buffers states before reading each key once, and source
  .NET 10 OrderBy.ToList does not read keys for zero/singleton input. An actual
  temporary .NET probe confirmed 0/0/2/3 key calls for lengths 0/1/2/3. Preserve
  model getter side effects between passes and stable equal-key ordering.
- Source reserves 28+4px for minimized windows; their model bounds stay untouched.
  Default cascaded model bounds are written only for invalid dimensions, before
  visual clamp and state reread. Maximized visual bounds use all available space.
  Drag deliberately does not read its document argument. Resize checks Left,
  Right, Top, Bottom separately, reads selected non-NaN limits twice, anchors the
  left/top edge, and clamps available space last. NaN/signed-zero double min/max,
  contradictory constraints, negative/infinite geometry and unknown flags retain
  source branches. UpdateZOrder snapshots before setters and moves only the first
  active member; duplicate model references remain present.
- Panel has actual native Panel inheritance/default styled optional manager.
  Property changes invalidate arrange after parent processing; measure visits all
  children and returns available size. Arrange filters original IMdiDocument
  DataContexts and selects configured manager or shared classic fallback. Native
  Controls provides count/get without original foreach version state: current
  child reads are live, not snapshots, but exact mutation invalidation is PARTIAL.
- Canonical model_projection::to_mdi_document_value retains the original document;
  canonical dockable handles and explicit shared ValueTypes casts are also read.
  No new document ViewModel or generic cast registration is used. Unregistered raw
  concrete model interface discovery remains partial.
- Native struct/flags/class/interface metadata exports entry constructor/read-only
  properties, nullable handles, original Instance readonly field, manager default
  constructor/interface conversion and all four method invokers. Explicit concrete
  nullable registrations are necessary for compiled manager null/new/Instance
  assignments; just listing optional reference handles is insufficient. Shared
  Build/Export/scanner/emitter are unchanged. The independent compiled consumer
  proves default/new/shared/null manager choices, combined flags and actual model/
  visual bounds, including minimized reservation and maximized size.
- IReadOnlyList generic contracts are absent from published System. The original
  model ListHandle provides read-only use over plain/custom/read-only/observable
  lists and live custom/fail-fast enumeration. Its broader mutable type/signature,
  generic raw interface discovery and observable version gaps remain PARTIAL.
  Source default(MdiLayoutEntry) null fields/null constructor arguments are not
  representable by current non-null Rust fields. Custom Equals/value-type/CLR
  exception metadata remains incomplete. Shared Classic.Instance uses native
  UI-thread TLS Rc, so source process-wide/cross-thread identity is not established.
  Shared flags metadata accepts all declared combinations but rejects unnamed
  numeric bits; Rust flags preserve those bits in original resize calculations.
  The published flags macro emits an assign-op Clippy pattern; its allowance is
  confined to the enum/descriptor module, without suppressing handwritten logic.
- Fifteen native Headless tests and one flags metadata test exercise original algorithm/control/metadata
  behavior, callback counts/order, reentrant source list mutation, all sixteen
  resize edge combinations plus unknown bits, numeric boundaries, source Z-order
  snapshot/duplicates, model identity, custom manager and arrange invalidation.
  One separate compiled consumer checks application dependency exports. Full
  original MdiDocumentWindow/ManagedWindowLayer/DockControl/theme/application,
  ownership/GC and desktop/pixel/performance paths still await their source ports.
- Validation: 1,061 Rust tests; zero failed/ignored. Locked all-target Clippy,
  formatting, inventory, factory generator, six verifier tests and diff/source
  checks pass. Inventory 1,928: 186 ported, 172 partial, 1,570 pending. Completion
  gate rejects the unfinished port. C# unchanged. Existing macOS debug compact-
  unwind linker warning remains. A runnable Rust sample is still pending.


Original managed dockable base continuation:

- Ported ManagedDockableBase as the original plain abstract model component.
  It has all source dockable/selector/restriction properties/defaults, lazy original
  TrackingAdapter, optional PinnedBounds and every lifecycle/tracking callback.
  Original owner/original-owner/factory/context references stay STRONG. Source
  property setters do not become ReactiveObject setters or replacement view models.
- Source SetProperty compares before assignment, then dispatches the derived
  OnPropertyChanged. Equality preserves NaN and signed-zero behavior; equal writes
  retain the old value/reference and do not notify. None names the empty property
  in the explicit native generic helper. Internal setters supply all source names.
  Native Event<str> is the shared System INotifyPropertyChanged contract, retaining
  handler snapshot order, nested/reentrant updates and exception propagation.
- Composition uses ManagedDockableBaseImpl and its object-safe callback adapter.
  impl_managed_dockable! forwards the full original IDockable, recycling, selector,
  restrictions and notification contracts with the enclosing object's identity.
  It uses public hidden dependency reexports and is compiled in a downstream test
  crate; no new production dependency on the ReactiveUI model library is added.
- TrackingAdapter is created only on first tracked getter/setter access. Setters
  store before callback, always call the original virtual hook even for repeated
  coordinates, and do not invent property notifications. Optional pinned bounds
  require finite dimensions only: zero/negative dimensions and NaN/infinite x/y
  remain accepted. Null clears all four coordinates to NaN and invokes the hook.
- Native abstract class metadata has no default constructor, exposes all 37 source
  properties and 18 public methods, and forwards the original changed event.
  ManagedDockableBaseReference is only a notification-capable native reference to
  the SAME Rc model; it holds no state or substitute view model. A wrapper is needed
  because System's external trait cannot be implemented on foreign Rc<T>. Public
  model_projection::to_managed_dockable_value boxes this reference; core dockable/
  root projection recognizes it. Shared concrete ValueTypes conversion also lifts
  canonical Rc<dyn ManagedDockableBaseImpl> handles into that reference. The shared
  metadata notification hook recognizes the adapter; raw trait handles need that
  conversion/helper first. Automatic raw trait-handle notification discovery is
  still partial, with the direct/conversion boundary checked in a native test.
- Context uses existing original domain equality first, then the established native
  recycling-value transport and native object/value equality. Native equal scalar/
  NaN contexts suppress duplicate changes; native derived/base handles for the same
  control keep reference identity. Arbitrary unregistered raw custom Equals and
  interface subtype behavior remain partial. No independent value or type engine.
- Twelve native model/API tests cover every property/default, unchanged writes,
  callback storage order, all tracking spaces, finite-dimension edges, NaNs/zeros,
  strongly retained original owners/factory, handler mutation/reentrancy/failure,
  generic comparer reentrancy, original reference release and runtime descriptors.
  The separate compiled consumer uses a TEST subclass of the original abstract
  base (not a production managed-document substitute). It proves source strings/
  booleans, notifications, TwoWay title editing, rebind/old-model detachment and
  native object Context identity. Other property type bindings/full managed models
  and the original complete headless application scenarios are not yet established.
- Remaining gaps: generic helper requires Clone/PartialEq and clones comparison
  values outside storage borrows; arbitrary generic Clone/Equals behavior, automatic
  CallerMemberName, invalid string nulls, undefined numeric enum bindings, full
  protected/ref/event sender/args metadata and performance are unverified. Retained
  strong Rc cycles do not provide CLR cycle collection. No full managed hosting,
  adorner, docking control/theme/application or runnable sample claim.
- MDI registration moved into the central registration file so mdi/mod.rs only
  declares/reexports modules as the guide requires. Shared framework/build is
  unchanged. Validation: 1,074 Rust tests, zero failed/ignored; locked all-target
  Clippy, formatting, inventory, generator, six verifier tests and diff/source checks
  pass. Inventory 186 ported, 173 partial, 1,569 pending. Completion gate rejects
  unfinished source inputs; C# unchanged and existing macOS unwind warning unchanged.


Original host/preview/menu dependency continuation:

- Ported full source DockWindowStateHelper, DragPreviewContext,
  DragDockableResolver, HostWindowContentCleanup, ToolMenuLifetime and
  VisualRootCompatExtensions bodies in matching native internal files.
  Callers remain pending; dead-code allowances are confined to those source helper
  modules, as with ItemDragHelper, and must be removed when callers exist.
- State mappings preserve all native/Dock/MDI variants and the original FullScreen
  to Normal loss in MDI conversion. Caller-specific FullScreen preservation policy
  is not invented in this helper. Undefined CLR numeric enum interop stays partial.
  Preview context holds the original strong dockable reference, checks activation
  before identity and clears activation before releasing the reference. Its native
  UI-thread TLS differs from source process-wide cross-thread state; keep partial.
- DragDockableResolver preserves root/active/pinned identity pattern order, whole-
  dock regular drags, resolved owner self-drops and Fill-only direct self no-ops.
  IsNoOpDrop resolves again after the first self-drop test unless Fill short-circuits;
  do not cache that second lookup. Three exact source resolver scenarios now run
  through production original ReactiveUI factory/model contracts. Additional cases
  cover all eleven defined operations and missing/mismatched preview prerequisites.
  Full original DockControlState and framework-property model harness stay pending.
- Host cleanup snapshots visual model ContentControls before changes, adds detached
  logical model presenters and clears the final list in reverse. Original visited
  reference identity handles cycles/duplicates. Traverse visible lists, hidden and
  all four pinned lists, pinned dock and split pane/content in source order; each
  collection uses live Count/get, preserving its own handle across property changes.
  Keep host/content/visual-root/duplicate guards. Plain reactive models have no
  logical parent; native logical models project through original boxed_reference,
  existing recycling transport and shared class registry, not reflection.
- Tests prove reverse clear after a callback removes all visual children, nested
  snapshot ordering, nonmodel/host exclusion, all detached logical root/split paths
  and source cycles, and other-window guards. A native logical-model TEST adapter
  registers a real class/core interface and retains the same source model reference;
  production framework-property models remain unported. Native visual collection
  enumeration-version fidelity and arbitrary unregistered interface discovery remain
  partial. RootVisual is TopLevelHost in this published backend: nullable root helper
  returns that actual source root, while direct cleanup root/host guards are kept.
  Complete source host-root/desktop/mixed-DPI parity remains unverified.
- A real compiled content binding realizes DeferredContentControl before cleanup.
  Source Content=null clears current content; explicit ClearPresentation releases
  materialized child state. The binding REMAINS and later source changes can refill
  an attached presenter. Do NOT add unbinding. This was verified against original
  framework 12.1.3 ValueStore.SetLocalValue/EffectiveValue.SetLocalValueAndRaise:
  https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/PropertyStore/ValueStore.cs
  https://github.com/AvaloniaUI/Avalonia/blob/12.1.3/src/Avalonia.Base/PropertyStore/EffectiveValue%601.cs
  An initial test assumed permanent unbinding; source inspection corrected the
  test without changing the original helper body. Native regression verifies both
  immediate clearing and later bound refill, then a repeated cleanup.
- ToolMenuLifetime captures flyout before current-value clear, keeps original menu
  identity, prefers actual binding UpdateTarget on attach, respects a replacement
  current flyout and clears its retained cache after attach. State borrows end
  before native property/binding callbacks. Tests cover null/repeated lifecycle,
  button/menu replacements, reentrant button replacement during detach and retained
  compiled property binding. ToolChromeControl/HostWindow/WindowDragHelper are still
  original pending dependencies; no simplified chrome or host is introduced.
- Validation: 1,089 Rust tests, zero failed/ignored; locked all-target Clippy,
  formatting, inventory, factory generator, six verifier tests and diff/source checks
  pass. Inventory 189 ported, 177 partial, 1,562 pending. Completion gate rejects
  the unfinished port; C# unchanged. Known macOS debug unwind warning remains.
  Full managed hosts/documents/layer/adorner/docking controls/themes and runnable
  Rust sample remain unfinished; goal stays active.


## Native container and content controls pass

Eleven original source classes now use the published native class/template/event
systems and public markup registration. The source optional metadata interface has
its two opaque properties plus a UI-owned descriptor using the existing recycling
transport; the core remains UI independent. Source ReactiveUI models do not implement
this metadata interface, so no metadata state was added to those models. Test-only
metadata models compose original ManagedDockableBase for control-boundary coverage.

Document/ToolContentControl retain synchronous initial DataContext observation plus
explicit repeated theme application, actual factory/native control identity, source
attach-only registration and current-context removal. Context changes intentionally
do not move dictionary entries; duplicate hosts retain source last-attach/any-detach
behavior. Themes use shared ControlTheme/ResourceKey lookup. Arbitrary raw custom
keys and unregistered interface discovery remain partial; explicit native custom
ResourceKey objects work. Metadata changes alone do not trigger reapplication.

SplitViewDockControl removes old template handlers and preserves source close/open
policy, short-circuit getter order and try/finally changing-guard reset. It does not
add detach cleanup absent from the source. GridDockControl preserves Loaded handler
addition, native ItemsPresenter/Grid realization, null definitions, columns-before-
rows parse/assignment and partial assignment on failure. Source CLR exception and
strong-delegate/GC semantics remain partial in native Result/panic/weak callbacks.

All fourteen control types referenced by DockDataTemplateHelper now exist; the helper
itself is the next original source prerequisite. Full source Fluent dictionaries,
property-framework generated models, DockItemContainerGenerator/ItemsSource tests,
managed host group, adorner/state consumers, tabs/pinned items and Rust executable
remain pending. ContainerControlsConsumer.xaml uses clearly marked test templates,
not substitutes for these production dictionaries. Original GeneratedItemContainer-
ThemeTests scenarios remain unclaimed; the sixteen new tests cover control boundaries.

Locked workspace tests: 1,106 passed, zero failed/ignored. Locked all-target Clippy,
format, inventory, factory generator, six inventory unit tests and diff checks pass.
The complete-port gate still fails as intended: 197 ported, 181 partial, 1,550 pending.
The existing macOS debug compact-unwind warning is unchanged.


## Original default data templates pass

DockDataTemplateHelper now creates the original fourteen templates in source order:
document content, tool content, proportional splitter, grid splitter, document dock,
tool dock, split view, proportional, stack, grid, wrap, uniform grid, dock and root.
A repeatable IntoIterator sequence supplies lazy fresh shared FuncDataTemplate objects;
reference-interface predicates retain original membership and null acceptance. Build
ignores the model argument and does not assign DataContext; the native presenter owns
that step. Recycling remains disabled, and invalid nonmatching builds fail.

Original splitter properties are compiled accessor closures reading original models.
Optional System notifications are forwarded from the same reactive/managed object,
with canonical interface borrowed views in model_projection and shared metadata lookup
for declared raw models. The standard InpcPropertyAccessor owns event filtering,
subscription replacement, dispatch and disposal. Bindings stay one way by the original
native defaults. Tests verify event identities, live changes, replacement/clear and
old-source removal, actual presenter-selected controls and original DataContext identity.

The source model GridResizeDirection and UI GridResizeDirection are different enums.
The original default converter rejects both model values; the port retains that error
and native Auto default while maintaining the live source subscription. Source tests
for this quirk and generic reference-type null matching were confirmed with the console
probe in scripts/dock/template-reference, using the actual source enum and framework 12.1.3.
Do not silently repair the source by adding enum conversion inside DockDataTemplateHelper.

Six new tests plus the managed notifier identity assertions pass. Locked workspace
validation is 1,112 tests, zero failed/ignored; strict all-target Clippy, format, structural
inventory, generator, six Python tests and diff checks pass. Complete-port gate remains
red as intended (197 ported, 182 partial, 1,549 pending). Full DockControl consumers,
managed hosts/layers/adorner state, original themes, tab/pinned controls and runnable
Rust sample remain pending. Native debug compact-unwind warning remains unchanged.


## Native drag-preview controls and accessibility pass

DragPreviewControl is an actual native TemplatedControl with the seven source styled
properties: ContentTemplate, Status, ShowContent, PreviewContentWidth/Height,
ControlRecycling and PreviewContent. Null reference and NaN defaults remain intact;
no new dimension validation is added. Its constructed callback creates the original
native recycling cache per instance, including derived controls. Tests use that real
cache and retain the exact cached native control identity across the domain bridge.

Status changes call the parent first and notify only an already-created matching
preview peer. The peer exposes Pane, original class/name/help behavior and the native
IValueProvider over the same peer/owner. It is always read-only; setter errors retain
the source message through native ProviderError. Status events normalize null/empty,
compare ordinal strings and support reentrant handlers after state assignment. Tests
cover suppression, case/whitespace, other-property exclusion and explicit disposal.
Exact CLR exception type, invalid UTF-16 values and shared ownership/GC remain partial.

DragPreviewWindow uses the native Window factory and preserves its original style
key even for derived classes. Its peer retains source base-title-first naming, then
fallback naming, original visibility/state help, and false control/content roles.
The source complete window theme is compiled; all nine setters are checked through
a separate consumer, including transparency, automatic size, Topmost=true, disabled
activation/taskbar/resize/hit testing and no decorations/background.

Eight native tests include original DragPreview_And_Adorner_Windows_Are_NonInteractive_
Panes and DragPreviewControl_Exposes_ReadOnly_Value_Provider_And_Change_Event scenarios.
One compiled consumer verifies live native property bindings using an explicitly
limited test template; the complete original DragPreviewControl Fluent dictionary is
not claimed. It requires original IDockable compiled Title metadata and the existing
recycling template bindings. Full DragPreviewHelper/managed host/layer/control/adorner
state, desktop mixed-DPI/accessibility/performance and the Rust sample remain pending.

Locked workspace validation: 1,121 tests, no failures/ignored; strict all-target
Clippy, format, inventory/generator, six Python tests and diff checks pass. Inventory
is 199 ported, 185 partial, 1,544 pending; complete-port gate remains red as intended.
The nonfatal macOS debug compact-unwind warning remains unchanged.
