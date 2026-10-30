# Original factory and markup reference

Run from this directory:

```sh
TMPDIR=/Volumes/SSD/temp/ferrodock-port dotnet run -p:NuGetPackageRoot=/Volumes/SSD/NuGetCaches/packages/ -- ../../../tests/Dock/data/factory-markup-reference.tsv
```

This standalone reference uses the UNCHANGED original Dock.MarkupExtension,
Dock.Model.ReactiveUI, Dock.Model and Dock native project assemblies with Avalonia
Headless 12.1.3 / .NET 10.0.13. It asserts 893 source outcomes across LoadExtension,
ReferenceExtension and all five FactoryExtensions methods. The probe factory only
exposes the source protected tracking hook and records the source virtual close call;
a separate real factory case closes a document through the original engine. All corpus rows now have matching Rust coverage; no SOURCE-ONLY rows remain.

Source URI loading, missing context/source, base URI access short circuit, absolute/
relative/null-base loading, fresh object identity, Source rereading after BaseUri callback (including null failure) and missing-resource failure are
checked. Name lookup checks source exact object identity, current providers, null/name/
missing cases and immediate retry. Factory checks cover nulls, active search order,
tracked inactive roots, current/focused document resolution (including Tool's original
IDocument contract), close dispatch/fallback and live window/host lookup.

The saved corpus records assertion names/results; variable runtime version is a comment.
Native tests independently exercise these same contracts, plus a separate compiled
consumer of the native metadata library. Normal Cargo tests do not require .NET.
The corpus alone does not prove complete runtime, type-system or platform parity.

If the existing NuGet cache path lacks its required trailing slash, pass
`-p:NuGetPackageRoot=/path/to/package-cache/` with the trailing slash. The local
`global.json` chooses the installed compatible .NET10 SDK. Original source/build files
and the sibling FerroUI checkout are not edited. Published SourceLink dependency
warnings remain visible in the capture log.

Plain Document naming/resolution is now a normal supported assertion. Further source
cases cover identity-preserving duplicate registration, conflicts across model/control
values, pending asynchronous lookup, synchronous callback reentry and scope completion.
The native shared scope and metadata APIs exercise the same outcomes, and the native
compiled fixture additionally verifies a named application object backed by the actual
Document model and a live compiled title binding. CLR/GC/platform equivalence is still
partial; no second registry or engine is introduced.

The PlainDocument.axaml asset constructs the unchanged original ReactiveUI Document
as an ordinary non-control root. Load checks its title and fresh identity. Further
source assertions compare the same engine model through IDockable, IDocument,
DockableBase, ReactiveBase and ReactiveObject, both for duplicate-name registration
and ReferenceEqualsMultiConverter; distinct models and scalar boxes remain unequal.

CapabilityPolicyRoot.axaml and CapabilityOverridesRoot.axaml construct the original
Core POCO classes. Additional checks cover nullable/default/set state, fresh identity,
actual initialized layout policy resolution, dockable override precedence and mutation
of the same loaded policy. Native tests retain the layout's explicit owning handle,
as required by the existing weak-parent port representation.

Four assembly-qualified model assets construct the original Document, Tool,
DocumentDock and RootDock classes with inherited/source properties. The reference
checks fresh instances, base classes, actual factory ownership/close, live compiled
Title notification and TwoWay Title/IsOpen edits. The Rust SDK tests these same
objects through its separate exported integration assembly and compiler hosts.

Ten further original model-root assets cover ToolDock, every layout dock and both
splitters. Additional source checks cover inherited titles, original scalar/enum/
reference properties and defaults, ToolDock.AddTool through the real factory,
SplitView.OpenPane/ClosePane/TogglePane, compiled live enum/bool/numeric bindings,
pane reference identity/null and the original Core enum assembly. A consumer-only
plain class holds the genuine original capability overrides through required and
nullable IDockCapabilityPolicy properties; it validates foreign interface assignment
without introducing a replacement engine or production model.

Five factory/window assets cover original constructors, scalar and reference
properties, live compiled TwoWay bindings, source base classes, all concrete model
creation methods and shared registry identities. The graph asset retains its factory,
parent and owner in resources. InitDockWindow sets Factory and Owner but does not
set Owner.Window; both native and unchanged C# assertions preserve that behavior.

The factory-event checkpoint covers all 28 original events with sender/payload
identity and unsubscription, all 29 event-argument constructors and original mutable
Cancel/Context behavior. Equivalent compiled argument/snapshot roots, an application
handler that cancels the actual source factory close, and ten tracking reason bindings
run against unchanged source assemblies. The reference uses explicit typed handlers;
it introduces no reflection or substitute engine.

The locator checkpoint adds source-style deferred method declarations, original
context/host/dockable reference identity, null delegate/null result/missing-key
fallbacks, dictionary mutation/failures, Core-qualified tracking and TwoWay bindings.
A real composed native subclass is compared with the unchanged C# Factory subclass.
The transient default-host getter case reads a non-null value first and null second;
InitLayout preserves the second null as an actual dictionary entry.

The factory API checkpoint adds the original mutation and window operations, pin
lookup/reordering, FindRoot rejection ordering, lazy/repeatable queries and independent
cursors. The source repeats nested dock results intentionally. Cursor Current is null
before its first result and retains its last value after end, fault and disposal;
Reset throws NotSupportedException. Compiled method predicates remain deferred and a
Core-qualified ActivateWindow command binding updates the original tracking state.
Window-list fixtures use separate owning and floating roots; a self-containing layout
recurses in the unchanged source too and is not used as a valid graph.

The Core capability/window checkpoint adds explicitly assembly-qualified policy,
overrides, options and evaluation roots, including the original eight-argument immutable
result constructor. Bindings cover parent-window title notifications and TwoWay option
edits. Source checks cover all six policy values, nullable inheritance, virtual Get
overrides, option application/null rejection, capability context/precedence matrices
and original operation/drop-target dock resolution. These assertions do not establish
full serialization, CLR enum/string or general lifetime/platform parity.

The Core-contract checkpoint adds original Core-qualified IDock/IDockable compiled
bindings, live observable-list counts, TwoWay fields/title, source navigation command
identity and history, native control Context, recycling ID and native cache
Add/Build/Clear identity. Original source engines and assemblies remain unchanged.

The host-contract checkpoint adds original Core IHostWindow bindings and method
commands, nested live window titles, TwoWay tracking, state identity, native geometry
(including independently ignored NaN dimensions), all four window states, layout
identity, title fallback, presentation/exit, container Control identity/TwoWay selector
and eight original local/global marker membership cases.


The nullable-factory checkpoint compares original null-aware factory overrides,
three-call document/tool order, factory replacement/removal between calls, owner
identity, initialization override dispatch before mutation, default null failures
and compiled null-command bindings. SOURCE-ONLY-nullable-base-add-list-element proves
that base AddDockable appends a null element after an initialization override accepts
it. The current Rust dockable list representation cannot store that element; this
reference case is evidence of unfinished scope, not a matched Rust scenario.

NullableMutationReference.cs adds twenty assertions against the actual original
Factory. Negative insertion indices return before either null argument is accessed.
Custom Init receives null before list access, null insertion preserves list identity,
indices, event payloads and empty/opened state, and invalid insertion creates the
source list before failing. Removal resolves root and performs virtual pin cleanup
before its original Owner dereference. Both protected pin overloads preserve nullable
item dispatch. FloatAll passes null snapshot entries through the original Remove,
Add and moved-event slots when custom overrides allow them. No window is opened.

NullableTargetReference.cs adds twenty-seven assertions using the unchanged original
Factory methods. A null AddDockable target reaches Init first, including actual Owner
and Factory assignments when the default Init runs. Six protected visible-list helper
targets fail at the source list access, before processing index/item arguments. The
protected active-selection method ignores its removed-item argument and retains its
original null-child, splitter, identity and invalid-index behavior. The explicit pin
root can remain null while membership lookup resolves a different root: no-op branches
return, the visible branch updates pinned bounds before collection access fails, and
pinned branches fail at the original unpin root's PinnedDock access. Custom protected
overrides receive the actual nullable arguments. No window is opened.

NullableMoveReference.cs adds38 assertions for nullable move/swap/pin/capability calls.
Within-dock movement and reentrant swapping preserve original null list values,
selection and exact removed/added/moved/swapped/docking callbacks. Added/removed
nonnull payloads include the original state synchronization's root lookups; those
callbacks are not filtered. Cross-dock swaps commit both lists before virtual Init,
including default null failure; the between-docks overload still initializes twice
when given the same owner and null slots. Movement preserves unpin before target
access, target list creation before source access, and the same-owner insertion-before-
removal order. Pinned moves use the first matching strip. Fixed Unpin dispatch reaches
actual virtual Pin only after membership succeeds. Capability entry points retain
source operation/flag access failures. No window is opened.

NullablePreviewReference.cs adds27 assertions for actual nullable pinned preview/hide
calls. They distinguish typed null lookup from the interface object-null no-op,
existing cleanup before incoming Owner access, fixed toggle relookup, KeepVisible
short circuit and partial null-snapshot cleanup. Live preview removal and cached
preview/list final selection remain distinct across reentry. Clearing PinnedDock when
the visible list is created forwards actual null into Init: a custom override can
restore a live preview, while default Init fails after the original list commit.
Clearing the preview inside nonnull Init preserves the later ActiveDockable access
failure. Keep-visible reselection retains the first nonsplitter's identity. No window
is opened.

NullableHiddenReference.cs adds27 assertions for hidden-item lifecycle and ID helpers.
Actual null reaches virtual Hide/Restore. Hide preserves membership/pin cleanup,
repeated root lookup and hidden-list creation before Owner access. Restore short
circuits absent membership, otherwise removes the original null entry and raises its
event before OriginalOwner access. Collection removal retains its null payload/index,
and observer replacement is not rolled back. Removed-event hidden-list replacement
and Restored-event destination redirection use the live source properties and actual
shared lists/models. ID helpers on real uninitialized DockControls fail at the first
null.Id in source query/root order, before later matches; result guards remain the
source nonnull checks. No window is opened.

NullableFloatingReference.cs adds27 assertions for actual nullable floating/window APIs.
Plain null/unsupported creation returns null before root lookup/creation, while options
preparation retains ownership short circuits and can capture the original parent even
when creation returns no window. Virtual Float/FloatAll/Split wrappers forward actual
null and coordinates. Default operation access failures and custom no-root split no-op
remain distinct. Preparation can abort/mutate options before plain creation, and Apply
runs only for an actual returned window (including null-options calls). Parent/result/
options identities, modality/taskbar flags, null split lifecycle and original nonnull
coordinate fallback are checked. No window is presented or opened.

The query cursor reference also checks 32 recorded traces in
`tests/Dock/data/query-cursor-reference.tsv`, plus disposal before first advancement.
These cover direct and all-controls queries, root/child/nested predicate callbacks,
true and false predicates, repeated disposal and recursive MoveNext. A source null
result after disposal is distinct from query completion; later failures preserve the
previous Current. The model and both native factory views use the same trace corpus.
Rust checks source access panics for the recorded CLR null-reference outcomes; the
corpus does not establish CLR exception-object or custom enumerator cleanup parity.

NumericEnumReference also compiles the five CoreNumeric*Root.axaml resources with the
original XAML compiler. Seven source cases cover unnamed owner/state numbers, signed
endpoints, nullable settings, original options ApplyTo/parent identity and the original
Int32 narrowing of oversized literals. A further namescope case verifies boxed-scalar
identity. The retained900 earlier reference assertions remain unchanged; the fresh output
has908 assertions. Native numeric cases live in tests/Dock/data/numeric-enum-markup-reference.tsv.
All scratch/build outputs use /Volumes/SSD/temp/ferrodock-port.

The numeric-enum reference now also compiles real RootDock and settings-option properties
with unnamed docking-mode/pinned-display/host-mode values. Six additional assertions cover
those values, nullable storage, original root/global host precedence and the model DockMode
converter's unset fallback. All previous 908 assertions remain; total 914.
