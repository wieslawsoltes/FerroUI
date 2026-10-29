# Hot reload development engine

Integration is explicit: supported adapters and registered functions define the
contract; arbitrary application side effects are not reversible. The
[completion audit](hot-reload-completion-audit.md) maps requirements to verification.
Results below retain their original source scope. Designer tooling, every catalog
page and a general supervisor for arbitrary applications remain later work.

Native Windows verification passed all 21 acceptance commands and a focused
catalog shutdown check. The [Windows report](hot-reload-windows-verification.md)
records the environment, the results, the expected skips, the native identity and
state observations and the fixes they led to. Earlier platform notes below retain
their original source scope; results for later revisions must be recorded
separately.

## Implemented

- `ferroui-hot-reload`: UI-thread sessions, coherent multi-document source snapshots,
  coalesced revisions, weak adapter ownership, structured outcomes and stage timings.
- `ReloadTarget` / `PreparedReload`: prepare every affected target before committing;
  reverse rollback includes the adapter whose commit failed. Failed drafts survive
  until the next edit, without retrying the same failed revision on every frame.
- `HotReloadSession::apply_pending_with`: an external commit callback runs after all
  target commits and before sources publish or old graphs are released. Preparation
  or target-commit failure skips it; callback failure rolls back all target graphs.
  The callback must leave its external state unchanged on failure, and adapter
  cleanup after success must not panic. This is not generic rollback of arbitrary
  user side effects. Like `apply_pending`, it does nothing without a new revision.
- `ContentReloadTarget`: classless XAML rebuilt inside a stable content host. The
  host, window and host data context survive. Invalid construction retains the old
  content. Custom class roots require an application adapter.
- `ComponentReloadTarget`: opt-in content replacement for declarative ContentPage,
  ContentControl and UserControl components. It retains the existing component,
  application-owned root values and view model, transfers a fresh namescope and
  rolls back content/scope on failure. Implicit content and explicit framework
  `Content` property elements use the same transaction. Opt-in inline `Resources` and `Styles`
  ownership is described below. Unregistered root assignments, other root property elements,
  named roots and unclaimed XAML handlers are conservatively rejected. `with_refresh` accepts a `ComponentRefresh` implementation for
  cached controls and application-installed subscriptions: prepare validates the
  candidate namescope, commit refreshes references after the graph swap, rollback
  restores them with the previous graph, and finish disposes old subscriptions only
  after every target succeeds. Cleanup must not panic; arbitrary side effects cannot
  be rolled back generically. It is not a general code-behind reload adapter.
- Runtime include collection and URI loading read the candidate source overlay before
  compiled assets. URI loads preserve the base URI, service provider and assembly
  identity. Accepted snapshots remain published for later lazy loads until session
  destruction; failed drafts never publish and competing sessions cannot own the
  same document. The session follows
  registered include dependencies; resource changes rebuild consumers, including
  static resource uses. Edits without a live validation adapter require restart.
  Failed resource/style loads reset their recursion guard and can be retried.
  Direct application resource/style includes can be discovered from development
  source metadata; arbitrary global owner and consumer discovery remains unfinished.
- `PanelReloadTarget`: preserves a framework panel and replaces an explicitly owned
  contiguous child range. Application children outside that range retain their
  identity/order; empty and growing ranges are supported. The adapter retains the
  old children and namescope for rollback, refreshes references through
  `ComponentRefresh`, and restores compatible child state after the dispatcher runs.
  Explicit framework `Children` property elements, including inherited
  `Panel.Children`, have the same ownership rules as implicit children. The
  lifecycle tests alternate the two forms while preserving application siblings,
  input state and fresh namescopes.
  Its root namescope must be XAML-owned; named application children need separate
  scopes. External interleaving/removal of owned children, items-host panels, root
  assignment edits without explicit ownership and unsupported handler/resource
  ownership require restart.
- Component and panel adapters expose `with_owned_handlers` for routed XAML events.
  Call after initial population; generated markup and the runtime interpreter both
  record the exact subscriptions they install. Replacement delegates invoke the
  preserved component instance, with a fresh namescope, without rerunning its
  constructor. New subscriptions stay inactive through preparation and commit;
  old subscriptions are suspended during commit and restored on rollback. Once
  every target succeeds, old subscriptions are removed and new ones activated.
  Ordinary application `add_handler` subscriptions are never claimed or removed.
  Deferred templates/resources retain their declaring subscription generation
  without retaining their root; an old factory built after retirement cannot
  install a new live XAML handler. Panel opt-in requires the complete initial XAML
  child range, before adding application children. Partial document ownership,
  non-routed metadata events with only add accessors, and irreversible user
  callbacks require an explicit adapter. Handler contract changes still restart.
  Use `ComponentRefresh` for application caches and child subscriptions; handler
  ownership does not infer which fields application code has cached.
- Component and panel adapters expose `with_owned_resources`, called immediately
  after initial XAML population. The root dictionary retains its identity.
  Replacement removes old XAML entries, merged dictionaries and theme dictionaries,
  installs the new contributions, and retains the previous graph until commit
  succeeds. Deferred factories remain deferred; values already realized during
  preparation retain the identity captured by static-resource consumers.
  Later application keys survive. Detached candidates see application resources
  and parent/application scopes, but cannot fall back to removed XAML keys.
  Root static values deferred until initialization are resolved on the detached
  candidate before capturing their commit changes, without rerunning the component
  constructor. Root `Theme` still needs `with_owned_root_static_resource_property`.
  Its selected root template is built for validation while candidate resources
  are available, without attaching the template or raising `TemplateApplied`.
  Deferred resource/template declarations transfer their weak root and dictionary
  context at commit, so later instantiations resolve through the preserved owners.
  Rollback restores the previous context. Data bindings remain pending during
  deferred-value preparation; conversion/setter errors fail preparation.
  Non-control `DynamicResource` consumers realized during preparation, such as
  brushes, transfer their temporary-root anchors and subscriptions to the preserved
  root. Captured extension values also use that root for future expressions.
  Rollback restores those anchors; retired resource values are not retained by
  the capture scopes.
  External writes to owned keys, dictionary replacement, ambiguous provider
  ownership, and collisions with application keys require restart. Mixed application
  and XAML merged/theme providers use a composed candidate preview: direct keys
  precede theme dictionaries, theme variants preserve their inheritance/fallback
  order, and new merged providers occupy the original XAML-owned range between
  application providers. Application values retain their normal precedence.
  Merged/theme providers claimed during initial population are replaced as whole
  contributions; add application resources to the parent dictionary or separate
  providers. Mutating the contents of a claimed provider requires an application
  ownership adapter.
  This lookup also follows explicitly assigned candidate dictionaries, so deferred
  factories use the same resource order as the committed view. Preview scopes never
  transfer provider ownership; failed preparation leaves live owners unchanged.
  Lower-level adapters can pair `ResourceDictionaryOwnership::preview_resource`
  with `prepare_with_preview`; the ordinary `prepare` entry point still rejects
  mixed providers when the caller has not supplied that composed lookup.
  Custom dictionaries and irreversible notification callbacks need a custom adapter;
  collection observers must not panic or mutate the collections during commit.
  This API does not claim inline `Styles` ownership.
- `with_owned_styles` claims the initial inline style entries and the root
  `Styles.Resources` contributions. Component and panel adapters replace only that
  contiguous range, preserve later application styles, and transfer owners using
  the framework's normal style removal/addition notifications. Removal detaches
  old frames; addition invalidates styling and affected templates. A failed commit
  restores the old entries and resources before the next layout.
  Candidate resource lookup composes new XAML styles with application styles in
  their eventual order, retains root-resource precedence, and excludes retired
  XAML style resources. Scoped candidate previews preserve other documents'
  dependency previews and restore them on unwind. Deferred declarations and
  dynamic-resource bindings transfer their context with the same transaction.
  The catalog's component adapter opts into both resource and style ownership.
  Normal reflection/compiled one-way and two-way setter bindings are admitted
  without starting their expressions. Source-only, delayed, direct-property,
  observable and custom bindings require an explicit adapter; root DataContext
  and direct-property style setters require an adapter. Validation includes the effective
  root theme and its BasedOn chain. If a style selects another theme, preparation
  validates and settles that theme before checking its template; unstable theme
  selection fails before commit. Custom setters and irreversible observers also
  require an adapter. External removal, duplication or interleaving of claimed
  style entries requires restart. Do not mutate the contents of a claimed style
  from application code; add separate application entries instead.
- Component and panel adapters expose `with_owned_root_property` for literal styled
  properties. Register immediately after initial population to claim the attribute
  as XAML-owned. Edits use detached, converted candidate values; removal clears only
  that local contribution, exposing a style or default. Commit failure restores the
  previous local value together with the graph. Unrelated application values and
  the root data context survive. Binding/resource expressions, animations,
  current-value overrides and application writes (including equal-value assignments)
  require restart. Development-only write versions distinguish ownership changes
  from value equality. Lifecycle
  properties such as Content, DataContext, Theme and Template use their own adapters.
  Custom property callbacks must support reversible assignment. This explicit API
  does not infer ownership from arbitrary application setters.
- Component and panel adapters expose `with_owned_root_binding_property` for
  literal or synchronous reflection/compiled bindings on declared root properties.
  The runtime loader captures these descriptions without activating bindings on
  the candidate root. Admission therefore rejects unsupported modes before a
  one-way-to-source binding can write to its model. The adapter creates an
  independent description for the preserved root, retaining the candidate
  namescope for child references and rebasing a direct Source reference to the
  temporary root. Strong root references in converter parameters, fallback values
  or target-null values require an adapter, avoiding candidate retention or a
  live-root ownership cycle. Old subscriptions stay paused until the entire revision succeeds.
  Rollback reinstates their original identity; success disposes them. Literal
  replacement, removal and subsequent re-addition use the same transaction.
  Unchanged bindings depending only on the preserved data context keep their
  existing expression and user input during child edits. Named/relative sources
  and resource-dependent options are prepared against the new context instead.
  Explicit assignments (including equal-value writes) invalidate ownership;
  ordinary `SetCurrentValue` input does not transfer binding ownership.
  Custom binding kinds, delayed/explicit/lost-focus updates, lifecycle properties,
  arbitrary captured extension state and irreversible converters require their
  own adapter. ControlCatalog registers Width, Height and Margin through this API.
- The development-only `ferroui_base::data::LocalBindingReload` primitive prepares
  an inactive replacement binding for the preserved object, then supports commit,
  rollback and finish. Explicit adapters must provide the binding description and
  its correct source, namescope and service context. It supports adding, replacing
  and removing local synchronous OneWay/TwoWay bindings, including the optimized
  compiled-binding expression. Delayed, explicit/lost-focus update triggers,
  other modes, observable bindings and animated properties are not admitted.
  The old expression stays subscribed but paused until finish; rollback restores
  its original expression identity, pending invalid input and validation, then
  publishes genuine source/context changes received during the pause without
  stale two-way writes. Completion of a detached expression cannot clear its
  replacement's validation. A reentrant application write invalidates ownership;
  rollback reports the conflict rather than overwriting it. Custom observers still
  require reversible, non-panicking cleanup. The component and panel binding
  ownership adapters use this property-store primitive.
- `with_owned_root_static_resource_property` explicitly claims a literal or
  `StaticResource` root assignment, including Theme and Template. Unlike literal
  ownership, it compares the detached candidate value even when the attribute text
  is unchanged, so dependency revisions replace old resource instances on the
  preserved root. Commit and rollback use the same ownership/version checks;
  removing the attribute clears its local contribution. Dynamic-resource bindings,
  animations, application overrides and unrelated root lifecycle properties are
  rejected. Property callbacks must support reversible assignment. ControlCatalog
  claims its root Theme this way. Selected template validation and compatible
  template-part state transfer are covered by the engine and catalog tests.
- `ResourceReloadTarget::merged`: replaces an explicitly owned merged dictionary
  contribution while preserving its container, direct application resources and
  other merged entries. Normal owner notifications update dynamic consumers,
  including removed keys. Register static consumers as dependencies so they prepare
  in the same transaction. Failed preparation or commit retains the prior providers,
  owners and source snapshot. Cached provider references need an application refresh
  adapter; arbitrary provider subscriptions are not transferred automatically.
  Candidate preparation checks deferred static-resource lookups after construction,
  allowing forward references while rejecting missing resources before commit.
  Lazy templates are checked only when instantiated during preparation.
  Resource and style adapters register lazy candidate previews before any view is
  prepared. Application-resource lookups see those previews without replacing live
  owners or publishing notifications. Consumers share the exact resource instance
  that commits, including chained resource dependencies. Duplicate ownership is
  rejected. Component, panel, style and resource adapters expose `with_dependencies`
  for resource documents outside inline includes. `discover_merged` and style
  `discover` find direct include contributions using canonical source metadata,
  without loading assets. `HotReloadSession::track_resource_includes` and
  `track_style_includes` keep weak references to explicitly registered containers
  and discover their direct include contributions before pending revisions.
  Only documents registered with that session are adopted. Accepted replacements
  stay tracked; removed contributions and closed containers are retired. A new
  include retries a previously unvalidated draft without another save.
  Every registered target conservatively depends on the tracked include documents,
  so static consumers prepare in the same transaction as resource/style owners.
  Register each contribution either through container tracking or an explicit
  adapter, not both. Repeated container registration is idempotent and retains
  the first runtime configuration. ControlCatalog uses this tracking for its
  application resource and style containers. Inline application resources, theme
  dictionary slots and unregistered static consumers still need integration.
  Custom resource adapters can implement `ReloadTarget::resource_preparation`
  and return `ResourcePreparation::new(original, factory)`. The factory returns
  the detached resource node and its reversible `PreparedReload`; the node must
  be the same instance that commit publishes. Factories must not install live
  owners or mutate application state. The session registers every preview before
  evaluating factories, resolves dependencies lazily, and rejects recursive
  candidate dependencies. Explicit document dependencies still determine which
  targets participate in a revision.
- `StyleReloadTarget::contribution`: replaces one explicitly owned `Style` or
  nested `Styles` entry. Removal detaches old selector subscriptions and setter
  frames; addition invalidates styling and templates on existing controls. Other
  application styles remain in place. Resources in the replaced styles transfer
  ownership and notify dynamic consumers. Rollback also handles an observer failure
  between removal and insertion. Template edits rebuild through the normal layout
  lifecycle; detached template parts are released. Registered component targets
  capture compatible template-part state. Deferred templates are validated when
  preparation instantiates them; this is not eager validation of every declaration.
- Conservative transfer of uniquely identified text-control state, scroll offset and
  focus after updating layout, before the queued render phase. Existing property values
  and bindings take precedence. TextBox transfer retains the caret and both selection
  endpoints, including backwards selections. The three cursor properties are
  admitted together: a binding, existing value or authored change/removal on any
  member prevents restoring the group, because their setters update each other.
  Internal selection models restore single/multiple
  selection and anchors by unique item equality across reordering. Missing or
  ambiguous items are skipped, as are custom models, authored/bound selection,
  changed selection modes and controls that already have a selection. Direct-property
  bindings are detected even when no styled-property value exists, preventing stale
  writes through programmatic two-way selection bindings. Template instances carry
  their document/declaration identity and owner scope. The same part name in two
  named owners remains distinct; repeated anonymous instances of the same template
  are ambiguous and skipped. A single anonymous owner can use its component scope.
  Nested template declarations have independent authored-value guards. Logical and
  visual traversal captures each actual object once, and retained objects are not
  restored onto themselves. Newly assigned focus, including on an unnamed control,
  takes precedence. Unknown template provenance is skipped.
  Development keys take precedence over names, and stable named/keyed scope owners
  qualify descendants. Keys are recorded on initial compiled/runtime population
  and on replacement, including template instances, without adding public names.
  Ambiguous matches and anonymous nested namescope owners are skipped.
- Content, component and panel adapters accept `with_state_adapter` for
  application-defined control state. `ControlStateAdapter::capture` returns an
  optional `CapturedControlState`; restoration receives the compatible control
  and `ControlStateRestoreContext`. The context exposes its scoped identity and
  `may_restore_property`, which rejects changed/removed authored values, existing
  property values and local bindings. Applications must also check their own
  storage and coupled-property rules. Snapshots must not retain old controls;
  capture must not mutate live state and restore must not panic.
  Callbacks run only after the whole revision succeeds. If another revision arrives
  before the queued restoration, the session completes accepted state first,
  against accepted sources and before candidate resource previews exist. Each
  snapshot runs once; replaced graphs and externally replaced panel children are
  skipped. This also prevents rapid edits from capturing unrestored default values.
  The catalog's named, unbound `ReloadCheckBox` demonstrates an application adapter
  in [the catalog integration](../../samples/ControlCatalog/hot_reload.rs).
  Authored-value comparisons follow the captured name on the old side and the new
  name on the replacement, so stable development keys remain authoritative when
  names are exchanged.
- Persistent populate overrides and weak component tracking in generated class
  populate functions and ControlCatalog's component-loading helper. Scoped runtime
  constructor overrides restore on unwind and take precedence over persistent ones.
  `HotReloadSession::bind_class` populates new instances from accepted source;
  live roots still need explicit adapters. Session destruction removes its hooks.
- The previewer retains its previous window when replacement loading fails.
- The runtime compiler reuses transformed document groups keyed by all collected
  source texts, document/base URIs, root types and loader configuration. Changing
  an included document prepares another group; building from a cached group still
  creates fresh objects. Each thread retains at most 64 groups with a 4 MiB source
  byte admission budget and least-recently-used eviction. This limits cached edit
  history; the source budget is not an estimate of AST heap size. Oversized groups
  still load but are not cached. Live templates retain the transformed bodies they
  need independently of cache eviction. Dynamic dependency discovery beyond the
  collected include group remains separate work.
  After this cache change, the macOS suites passed all 52 reload-session tests and
  404 loader tests (one existing loader test ignored), including fresh-instance,
  weak-reference release, grouped-source invalidation and eviction checks.
- `Build::hot_reload_manifest(true)` writes a versioned JSON development manifest
  and `FERROUI_HOT_RELOAD_MANIFEST` only when the target enables debug assertions.
  Manifest generation requires Cargo 1.93 or newer to detect the target profile.
  The gate also handles custom profiles, build-dependency overrides and explicit
  `-C debug-assertions` rustflags (the last override wins);
  the build script's own debug configuration is not used. The dependency-free
  `python scripts/hot-reload/profile_probe.py` checks these boundaries with Cargo.
  It records the build's source baseline, paths,
  classes, names, expanded data types, member attributes and includes. Handler
  contracts include resolved event signatures, inherited events and qualified
  routed-event owners. Classification uses the build's type system for both model
  and runtime hosts, without invoking constructors or event accessors.
  `ReloadManifest` reads it and resolves transitive dependencies. Manifest sessions
  freeze that type classification so lazy runtime registration cannot change their
  baseline. Changed handler assignments and attributes on unresolved types require
  restart before candidate preparation. Session handler contracts also inspect registered class and markup-type metadata,
  including inherited events and owner-qualified routed-event definitions. A common-event
  fallback remains for manually registered documents without build metadata.
  Applications creating those sessions must register their runtime metadata before
  registering documents and supply compatibility checks for unknown custom types.
- `ferroui-hot-patch`: explicit callback indirection through Subsecond 0.7.10,
  body-only synchronous-function admission, full supplied input-set comparison,
  stale patch rejection and an unsafe controlled-apply API. It rejects application
  during a wrapped callback. No network client automatically loads patches.
  `hot_reloadable_with(function, argument)` supports typed synchronous handlers
  and commands; pass a tuple for multiple arguments. The stable subscription must
  call it on every invocation. It resolves the function pointer through the current
  patch table, preserves nested callback quiescence and releases its guard on panic.
  Argument/return types remain in the stable library. This invocation helper does
  not register a source function, authorize a patch or request view reapplication.
  Its 19 patch-crate tests and release compilation pass on macOS. The updated native
  callback probe passed two actual patches through the argument-taking function
  pointer while retaining the existing subscription and stable framework object/type.
  The updated native typed probe also passed on Linux ARM64 with Rust 1.93.0/dx
  0.7.10. Windows execution remains required.
- `PatchSupervisor::apply_with` installs admitted code provisionally, invokes an
  explicit reversible view transaction using the new function bodies, and publishes
  the code revision only on success. Failure restores the previous dispatch table;
  a panic restores it before unwinding. Rejection of the first patch restores calls
  to the original executable. Callers must retain accepted artifacts, own dispatch
  exclusively, and undo their UI mutations before returning an error or unwinding.
  Independent Subsecond reload handlers and asynchronous/cross-thread patchable
  work are outside this contract. Failed dispatch rollback blocks further admission
  through `restart_required()` until process replacement. A macOS native probe
  passed two accepted patches plus initial, subsequent and panicking transaction
  rollback, verifying the candidate's pure layout function and subsequent handler
  behavior. Linux/Windows execution of that standalone transaction probe remains
  pending; this is not a generic rollback mechanism for arbitrary user code.
- `FunctionRegistration::callback` declares handlers/commands which take effect on
  subsequent invocation; `FunctionRegistration::view_builder` declares the exact
  view identifiers to reapply. `PatchSupervisor::with_registrations` validates
  top-level synchronous functions against the compiled baseline and rejects missing,
  duplicate or unsupported declarations and empty builder view lists. Admission
  tickets expose only the deduplicated views requested by changed builders;
  callback-only edits request none. `apply` rejects builder tickets, requiring an
  explicit `apply_with` transaction. Application adapters still resolve these view
  identifiers. The macOS native transaction probe now uses this registration API,
  the 21 patch tests pass, and the combined sample uses explicit callback registration.
  The sample also registers `build_view` for its `main` view: this pure function
  derives candidate markup from authored XAML, while the stable library owns
  preparation, contract checks, commit and rollback. Session sources retain the
  authored text, and the sample adapter tracks all of its manifest documents.
  Builder edits install code provisionally and reapply that view using
  `apply_with`; failed candidate markup rolls back code and UI. The expanded
  native Linux/Xvfb probe passed invalid-builder rollback, successful live view
  replacement, and retention of that derived view through a later handler-only
  patch. The same expanded probe also passed on macOS with an active display.
  Windows execution remains open.
- `HotReloadSession::submit_texts` accepts editor/supervisor document snapshots
  without rebuilding unchanged accepted markup. Reverting one document discards
  its failed draft without discarding neighboring drafts; resubmitting a pending
  draft retries the transaction. The combined sample uses this path so a full
  markup snapshot accompanying a handler-only patch does not rebuild the view.
  Explicit `submit` remains available for intentional reapplication.
- `BuildInputs`: recursive byte-exact input capture, explicit output-directory
  exclusions, binary-asset/configuration comparisons and isolated source staging.
  Unix executable permissions are preserved and compared; symlink inputs fail closed.
  `StagedInputs::verify` rejects build-time source changes and generated files in the
  input tree. Admission tickets belong to one supervisor and accepted revision.
  This is not Cargo dependency discovery or a build sandbox: callers must include
  local dependencies and supply compiler/environment inputs, build outside the
  staged source tree, and verify before applying. External mutable inputs remain
  outside this contract until a complete collector is integrated.
- `RestartSupervisor`: build-before-restart with opaque build tickets, monotonically
  increasing revisions, startup timeout, stale replacement rejection, and explicit
  readiness before retiring the old process. Failed builds and failed startup keep
  the current process alive. The caller supplies isolated build/output commands.
- `RestartState` / `RestartStartup`: versioned application data, navigation identity,
  and keyed window placement, with explicit acknowledgement after initialization.
  The desktop sample restores its counter/navigation/window and acknowledges after
  valid markup loads and its window opens. Application state capture remains explicit.

## Trying markup reload

With the normal desktop prerequisites installed:

```sh
cargo run -p hot-reload-demo
```

Edit `samples/HotReload/View.xaml` or its included `Colors.xaml`. The demo embeds
the build-generated development manifest as its compatibility baseline, registers
all manifest documents, and watches their directories. Edits are read at runtime
and do not launch Cargo. A resource edit rebuilds the dependent view so static
resource consumers refresh. Startup first applies the build baseline; the watcher
then submits any disk changes as a new revision. The sample keeps its application types and counter in a library;
`program.rs::counter_click` and the pure `program.rs::build_view` are the
executable's explicit Rust patch boundaries. `build_view` initially returns the
authored markup unchanged; a body-only edit can derive a replacement view without
recreating the window, component root, or application state.
The button title counter is ordinary working code, but it does **not** receive
machine-code updates without a patch supervisor. The demo uses a stable content
root and a `ComponentRefresh` hook (`samples/HotReload/refresh.rs`) for its cached
counter label. The hook preserves newly authored Text/bindings and refreshes the
application reference without rerunning startup.

The sample uses a 40 ms polling interval and requires two stable reads. The watcher
reopens paths, so atomic-save replacements work. It debounces the entire watched set
and emits a complete `FileBatch` in one message. `HotReloadSession::submit_files`
submits that snapshot as one revision; missing files block the entire transaction
until recovery, retaining neighboring edits and the last accepted graph. Samples
consume the latest queued snapshot before preparing a revision. ControlCatalog and the demo use manifest-driven directories; the catalog's explicit probe overlay watches known files. `ReloadManifest::watcher` and
`FileWatcher::with_directories` additionally discover XAML recursively beneath
document directories, preserve assembly URI prefixes, and report scan failures as
transaction-blocking diagnostics. Newly discovered files require admission; absent a
registered compatibility baseline they report `RestartRequired`. Symlink discovery
requires explicit document paths. Directory watching still uses portable polling,
not native filesystem notification APIs. Pass the loaded sources to
`FileWatcher::with_baseline` to avoid missing saves during startup.

Each watcher batch carries a `WatchRevision`. A different observed read invalidates
it immediately, before debounce completes. The session checks it before preparation
and again before commit: superseded candidates are dropped without publishing
sources, mutating the live graph, or invoking a coordinated code finalizer. Failed
drafts remain available for the next stable batch. A transient edit followed by a
revert issues a fresh token even when the text matches the previously delivered
batch. `has_pending_changes` also reports an invalidated read awaiting debounce,
so a coordinated handler patch cannot bypass that outstanding markup revision.
Editor-owned `FileBatch` values can set `revision: None` when no watcher owns their
source snapshot. The two watcher regressions pass in the 57-test macOS and Linux
session suites.

Use explicit development keys when a control needs an identity independent of its
name (the prefix is arbitrary):

```xml
<StackPanel xmlns="https://github.com/ferroui"
            xmlns:hr="https://github.com/ferroui/hot-reload"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
            mc:Ignorable="hr">
  <TextBox hr:Key="draft" />
</StackPanel>
```

Both compilers preserve the literal key as development metadata. Enabled debug
runtimes store it against the object's weak identity, including on initial loading;
release runtimes retain no key registry. A key neither assigns Name nor registers
an extra namescope entry. Empty keys and markup expressions are rejected. The
`__ferroui_reload_...` prefix remains reserved for internal source matching and
must not be used as an application name. State belongs
to one document/component adapter, then to the chain of stable namescope owners,
then to its explicit key or name. Differing declarations behind duplicate source
names are conservatively excluded from property restoration. Authored changes and
bindings take precedence, including removal of a previously authored property.

For an application integration:

1. Register baseline document sources and create a session on the UI thread.
2. Retain each `Rc<dyn ReloadTarget>` in the component lifetime; the session retains
   only weak references. Use `ContentReloadTarget` only for an isolated classless
   content subtree. Custom constructors/setters with external side effects need an
   application adapter and cannot be made transactional by catching panics.
3. Submit changed text (disk or unsaved editor buffer), then call `apply_pending`
   between UI callbacks. Keep one transaction for related document changes.
4. A custom adapter must retain the old state through commit, undo a partially
   failed commit in `rollback`, and refresh cached component references explicitly.
   Rollback must not panic. Do not rerun arbitrary component constructors.
5. Treat `RestartRequired` as an unsupported contract change, not permission to
   clear and partially repopulate a root.

Development session code and patch application are debug-only. A normal compiled
markup build without the runtime interpreter has no populate-override TLS lookup.
Applications deliberately linking the runtime loader still need constructor hooks.

## ControlCatalog integration

```sh
cargo run -p control-catalog-desktop --features hot-reload
```

Supported pages are **Border, Slider, ProgressBar, CheckBox, Canvas,
RadioButton, TextBlock, Viewbox, Expander, ListBox, and AdornerLayer**. Edit their documents under
`samples/ControlCatalog/Pages`. The custom catalog build emits a 220-document
manifest, and the session resolves local include dependencies from that baseline.
Build-time placeholder geometry substitution is shared with runtime updates, so
unchanged branding is not misclassified as a source edit.

Compatible edits retain the page root and data context. Invalid markup retains its
content and namescope. Width, Height and Margin are explicitly owned root literals
or synchronous bindings; they can be replaced or removed transactionally under the
binding ownership rules above. New instances populate from accepted
sources. Registering a newly opened target retries its pending draft without another
save. Source directories are watched for new documents; unsupported documents and
changes without a live validation adapter require restart.

Application resource/style includes are rediscovered when a revision is pending,
including contributions added after the catalog session starts. Source documents
must belong to the catalog manifest; dependency-assembly themes are not adopted
without their own registered sources. Discovery retains neither the application
container nor retired dictionaries/styles.

`FERROUI_HOT_RELOAD_ROOT` supplies a partial source overlay for acceptance probes:
existing overlay files replace their manifest paths, and remaining files use the
original source tree. This explicit overlay watches known manifest files, avoiding
ambiguous discovery of two files under one document URI.

Pages with cached references and custom side effects still need explicit refresh
adapters. ExpanderPage supplies one: its cached
named controls and cancellation subscriptions are replaced transactionally. The old
handlers remain available for rollback and are removed after successful commit;
the page constructor and view-model initialization are not rerun. This remains
partial catalog integration. AdornerLayerPage uses owned XAML routed handlers and
an explicit refresh that clears its cached removed adorner on success, retains it
for rollback, and permits the new markup's adorner to take precedence.
`python3 scripts/hot-reload/catalog_probe.py` exercises all eleven registered pages;
`FERROUI_CATALOG_RELOAD_PAGES=Border,Slider` selects a subset. Each case changes an
owned root attribute, rejects invalid syntax, recovers through atomic replacement,
and removes the attribute. The probes also wait for the corresponding compositor
batch to finish rendering and report save-to-render timing. Preparation/application
timings remain separate; they alone exclude watching, layout and rendering.

ExpanderPage's native macOS probe passed valid edits, invalid syntax, atomic-save
recovery and root-attribute removal in one process. Its adapter test separately
exercises ten prepare/commit/rollback cycles: cached controls revert on rollback,
old cancellation handlers remain until success, retired handlers are removed, and
detached controls and the page can be released. This validates the page-specific
adapter; it does not automatically enable other constructor-configured pages.

## Render-completion measurements

`FERROUI_RELOAD_TIMINGS=1` enables `record_frame_timing` at the sample/catalog
transaction boundary. The hook requests the pending compositor batch and stamps
its completion on the render thread. It neither forces layout nor waits on the UI
thread. `FERROUI_RELOAD_FRAME` JSON records revision, documents, batch identity,
preparation/application microseconds, commit-to-render microseconds and a wall-clock
completion timestamp. The probes measure from immediately before writing the file,
wait for that document's frame, and reject measurements with detected clock changes
or revisions sharing a batch. Ordinary sessions do not enable this instrumentation.

The following initial macOS arm64 debug-build baseline was measured on 2026-10-19,
serially with no compiler running. The fixture polls at 40 ms and the catalog at
60 ms; save-to-render includes file writing, watching/debounce, UI preparation,
application and rendering. Atomic-save recovery and root-attribute removal are
included among the catalog samples. Values are milliseconds; stage columns are
medians and do not sum to the total because other stages intervene.

| Fixture/document | Saves | Save-to-render median (range) | Prepare | Apply | Commit-to-render |
| --- | ---: | ---: | ---: | ---: | ---: |
| Small fixture, View.xaml | 2 | 123.00 (106.74–139.27) | 5.22 | 1.36 | 18.01 |
| Small fixture, Colors.xaml include | 1 | 106.28 | 6.79 | 1.42 | 15.44 |
| Catalog Border | 3 | 165.40 (137.66–243.33) | 19.91 | 2.07 | 9.66 |
| Catalog TextBlock | 3 | 237.86 (168.56–290.94) | 50.89 | 4.55 | 23.49 |
| Catalog Expander | 3 | 177.43 (168.76–209.01) | 25.50 | 8.01 | 40.60 |

Reproduce with `markup_probe.py` and
`FERROUI_CATALOG_RELOAD_PAGES=Border,TextBlock,Expander python3 scripts/hot-reload/catalog_probe.py`
after building the binaries. Each emits `FERROUI_RELOAD_MEASUREMENT` JSON for the
individual saves. These are small-sample baselines, not percentile estimates or
latency guarantees. Compositor completion does not prove physical display scan-out
or pixel correctness. Hidden/stopped targets may not complete; probes time out
instead of substituting commit time. These numbers describe only runtime XAML edits;
Rust patch measurements are recorded separately below.
The [raw baseline records](hot-reload-baseline-macos.json) include the compiler,
platform, polling intervals and all individual measurements.

## Rust patch measurements

The owning supervisor probe recorded one first-patch observation on macOS arm64
on 2026-10-19 using Rust 1.93.0 and dx 0.7.10. It changed the registered synchronous
`counter_click` function body and XAML spacing together, waited for that accepted
markup revision's compositor batch, and then verified the original subscription
invoked the updated function with unchanged host/framework identity.

| Stage | Milliseconds |
| --- | ---: |
| Initial source capture | 607.98 |
| Patch input preparation | 80.23 |
| Admission acknowledgement | 91.52 |
| Private source staging | 3.77 |
| Compiler invocation | 1284.44 |
| Source revalidation | 675.83 |
| Patch artifact decoding | 0.24 |
| Application acknowledgement | 285.33 |
| Supervisor update through application acknowledgement | 2421.35 |
| Save start through compositor render completion | 3056.99 |

The UI transaction separately reported 5.50 ms preparation, 187.52 ms application
(including code installation), and 33.77 ms from the frame observer to render
completion. These intervals overlap the supervisor measurements and must not be
summed into them. Compiler invocation includes dx startup, compilation/linking and
compiler transport; acknowledgement stages include pipe transport and UI dispatch.
The initial fat-binary build and watch polling/debounce are excluded: this probe
invokes `Supervisor.update` directly after saving and capturing the changed files.
This is one measured patch, not a distribution or a compilation-free latency claim.

Run `python3 scripts/hot-reload/supervisor_probe.py` with the pinned tools to repeat
the measurement. It emits `FERROUI_PATCH_MEASUREMENT` JSON and also checks failed
replacement builds, state restoration and generation cleanup. The
[raw patch baseline](hot-patch-baseline-macos.json) records the stages and scope.
One Linux ARM64 combined-patch observation under Xvfb took 9,236.71 ms from save
to compositor completion: 3,053.93 ms initial source capture, 2,543.69 ms compiler
invocation, and 3,300.32 ms source revalidation. The UI transaction reported
11.98 ms preparation, 7.40 ms application and 16.95 ms to render completion.
The [Linux raw baseline](hot-patch-baseline-linux.json) records every stage. This
ran in an Ubuntu 24.04 container with 7 GiB memory inside a two-CPU, 8 GiB Colima
VM, with host-mounted sources/builds. It is not a bare-metal Linux timing result
and does not establish completion of the subsequent restart checks.
The separate Linux catalog run recorded 27 XAML save-to-render observations across
nine pages, with three-observation page medians from 229.99 to 281.42 ms. It ran
under the same VM/container limits and Xvfb, without concurrent builds; the final
debug executable was linked with LLD 18.1.3. The
[raw XAML baseline](hot-reload-baseline-linux.json) includes preparation, application,
render completion, and atomic-save cases. These small samples are not percentile
estimates. Windows timing and sustained-edit distributions remain unmeasured.
The small Linux fixture subsequently passed valid edits, invalid-XAML recovery,
atomic replacement, and an included-resource update with the supersession guards
enabled. Its two view observations were 143.03 and 97.44 ms save-to-render; the
included-resource observation was 145.09 ms. The separate
[fixture baseline](hot-reload-fixture-baseline-linux.json) records all stages and
the same VM/Xvfb limitations.

## Rust patch acceptance gate

```sh
python3 scripts/hot-reload/probe.py
```

Requires Rust 1.93.0, `dx` 0.7.10 and Cargo on PATH, as pinned in
`scripts/hot-reload/toolchain.json`; `FERROUI_DX` may name the CLI executable.
The probe builds a temporary standalone executable, checks `VALUE=1`, modifies
one function, invokes `dx tools hotpatch`, and checks that a registered callback applies increments of 1, then 2, then 3
to one retained counter in the same PID. It also requires the stable library’s
framework object address and both `TypeInfo` pointers to remain unchanged. Each patch
passes `PatchSupervisor` admission before compilation and controlled application
after rereading the fixture's entire input tree. A stable-library edit must be
rejected before invoking the compiler, with the original callback still callable.
The fixture's external FerroUI path dependencies are assumed unchanged during this
controlled test; a production supervisor must collect those inputs too.
It never edits repository source files.

On macOS arm64, Rust 1.93.0 with Subsecond/dx 0.7.10 passed a fresh
standalone patch build and two consecutive result changes in the same running process. The
probe removes process-local linker environment entries from the captured build
metadata before replay. This is required for dx 0.7.10: its replay overlays stale
`DX_LINK_ARGS_FILE` and `DX_LINK_ERR_FILE` paths over the new build's paths, causing
it to read an empty linker argument file and produce a symbol-less library.
Cargo jobserver descriptors are likewise not valid in the new process.

The unmodified CLI failed with both Rust 1.99.1 and 1.93.0. Rust 1.99.1 has not
been revalidated with the workaround. The tested patch combination is Rust 1.93.0
and Subsecond/dx 0.7.10. Platform results and exact source revisions are recorded
in the completion audit. This fixture tests patching; the owning sample supervisor
described below provides application build/restart orchestration.
The desktop demo has a separate `patch-control` feature for a trusted supervising
parent. Its private stdin channel admits a source snapshot before a build, returns
a request identity, rejects unmatched/cancelled applications, and pumps approved
patches between UI callbacks. Markup reapplication waits while a build is pending;
cancellation or parent disconnection releases that wait. The parent must guarantee
the exact compiled baseline, isolated admitted inputs, pinned compiler and patch
artifact provenance. The channel is not a sandbox or an independent file watcher.
An admission command can also carry unsaved markup documents. Application prepares
and commits that markup, installs the admitted patch through the external commit
callback, then publishes the accepted markup. The owning supervisor below connects
this channel to compilation, watching and restart. Test channel admission, cancellation, existing event subscriptions,
root/type identity and explicit state capture with `cargo build -p hot-reload-demo
--features patch-control` followed by `python3 scripts/hot-reload/control_probe.py`.
The channel probe does not apply machine code. The separate
`python3 scripts/hot-reload/desktop_patch_probe.py` driver builds an isolated copy
of the actual desktop sample and applies two admitted patches through that channel.
On macOS arm64 with the pinned toolchain it passed increments of 1, 2 and 3 through
the originally installed routed-event subscription, retaining the process, component
root, framework type identity and counter state. The driver fingerprints framework
source/configuration inputs around compilation and never edits repository sources.
It remains an acceptance driver, not an automatic watch/restart supervisor.
The extended desktop probe also passed two combined Rust/unsaved-XAML revisions
on macOS: invalid markup prevented patch installation, and a missing patch library
rolled back the newly committed markup. After each rejection it invoked the old
handler and checked the old live spacing, then applied the valid pair and checked
the new spacing and increment. This verifies the explicit command transaction.
The owning supervisor submits a coherent source snapshot; separate saves observed
in different snapshots remain separate revisions.

This desktop test exposed a second dx 0.7.10 replay problem: structured build output
chooses the first captured binary invocation, which can be `build_script_build`
when a package has a build script. The driver selects the exact named application
capture from dx's captured-argument directory, verifies its source path, and rejects
missing or ambiguous captures. Replaying the build-script invocation had failed
before patch application with a missing build-script object. The selection and
process-local environment fixes have separate Python regression tests.
A manual `Hot reload acceptance` workflow defines separate Rust patch and catalog
XAML jobs on macOS, Windows and Linux. Catalog coverage includes manifest/runtime
tests, compiled/runtime parity, the subscription-refresh adapter, shared-selection regression, all eleven native
page probes, and release compilation with development features selected. Parity
runs without `runtime-markup`, which otherwise excludes those tests. The workflow
has not been dispatched from this worktree. Adding it is not evidence of passing
those platforms.

The official macOS CLI also links OpenSSL 3 dynamically. Its source installation
failed on the investigation machine without OpenSSL development headers; its
binary could run with an existing compatible OpenSSL library supplied through
`DYLD_FALLBACK_LIBRARY_PATH`. That machine-specific setting is not a repository
requirement or a substitute for normal dependency installation.

## Experimental owning sample supervisor

`scripts/hot-reload/supervisor.py` connects the sample's private development channel
with source capture, patch compilation and process replacement. Run it with the
pinned Rust 1.93.0 / dx 0.7.10 toolchain on PATH; `FERROUI_DX` selects the dx executable:

```sh
python3 scripts/hot-reload/supervisor.py
```

It captures Git-visible workspace files, including untracked sources, into private
build directories. Ignored build inputs must be made explicit; links escaping the
workspace are rejected. Internal links and executable permissions are preserved.
The editor checkout is never compiled or modified by this driver. Cargo output stays
inside each generation's private directory, including during replacement builds.

Markup-only edits go directly to the UI session. A Rust edit first goes through the
running application's conservative admission check. Accepted function edits compile
against that process's original fat binary and install only after a fresh source
comparison. Combined markup and code share the UI transaction. Unsupported edits
build a replacement; the old process remains alive through build failure or failed
startup. After successful compilation the driver pauses input and markup in the
old application and captures versioned state on its UI thread. It starts the candidate,
waits for its restored-window acknowledgement, then stops the old process. Failed or
superseded startup resumes the previous application, including when final source
verification fails. Ctrl-C stops the owned application and removes the private builds.

On macOS and Linux, compiler invocations run in a dedicated process group. Timeout
or interruption kills the group before returning, including descendants whose
immediate parent has exited. macOS tests exercise actual child writers through
timeout and Ctrl-C. Windows uses an unnamed, non-inheritable kill-on-close job. A
gated launcher starts the compiler only after job assignment succeeds; assignment
failure aborts the build. This follows the Windows [job-object lifetime contract](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
Tests cover an exited compiler parent, rejected assignment and supervisor termination.
The Windows-specific tests are present but have not been executed on this macOS host;
Windows process-tree cancellation remains an unverified acceptance gate.

This driver currently targets `samples/HotReload`, polls file metadata, and captures
complete compiler input snapshots for code changes and final admission checks.
Ignored files or directories can be declared with repeatable `--input` options,
for example `--input generated/schema.bin --input local-assets`. Paths are relative
to `--source`; selected directories include all files, empty directories, modes and
internal symlinks. These inputs are copied into each private build and included in
watching and admission fingerprints. Changes to them require a replacement build;
missing selected inputs retain the running app. External paths, links escaping the
workspace and nested Git metadata are rejected. External input mapping and reusable
application registrations remain unfinished.
The tooling suite passes 24 tests on macOS and Linux with two Windows-only skips,
including ignored-input capture, directory/symlink staging, watching and restart admission.
Markup-only revisions reread changed documents. Unregistered build inputs travel as
SHA-256 fingerprints; registered executable source remains text for AST admission.
Failed and superseded build directories are discarded. The running application's
generation stays available for patch replay; a retired generation is removed only
after its process stops. Failed shutdown retains the process and directory for a
retry during session shutdown. Filesystem cleanup failures are reported and retried
by the session's temporary-directory cleanup. It does not yet discover
external/ignored build inputs or provide a general application registration format.
The sample handoff rejects click/patch commands while paused. `InputPause` adds a
temporary IsEnabled contribution rather than replacing local values or bindings;
resume exposes the latest application value. Existing animation/pause contributions
are rejected. `RestartHandoff::begin` also invokes registered `RestartParticipant`
callbacks before state capture. Each successful callback returns a `RestartResume`
guard; failure or cancellation resumes prior participants in reverse order before
releasing input. The sample's stable library exposes `register_restart_participant`
for application-lifetime work. For example, a UI timer participant can record
`timer.is_enabled()`, stop it, and return a guard that restarts it only if it was
previously running. Both already-paused timers and successful timer pauses retain
their prior state on resume.

Participants must finish draining their own work before returning, leave their
own state unchanged on error, and provide non-panicking resume callbacks. Asynchronous
work that is still draining should reject the handoff for retry, rather than block
the UI thread. These callbacks do not suspend arbitrary threads or automatically
serialize view models. Applications must not animate IsEnabled during handoff.
Broader integrations, cross-platform execution and latency distributions remain release
work. `supervisor_probe.py` exercises the driver against a private editable checkout;
`test_supervisor.py` covers routing, cancelled work and failed build retention.

## Validation recorded

Native Linux ARM64 validation is now in progress in an isolated Ubuntu 24.04
container under a dedicated Colima VM, with Rust 1.93.0 and the checksum-verified
dx 0.7.10 release binary. All 52 reload-session tests and 17 patch/admission tests
passed. The Python tooling suite passed 20 tests, skipping two Windows-only tests.
The native callback probe retained the framework object, type identity,
subscription and counter through two machine-code patches. The desktop probe
under Xvfb retained its live root/type/handler through two combined Rust/XAML
revisions and passed invalid-markup and failed-patch-artifact rollback checks.
The initial 3.5 GiB container limit caused a compiler OOM; the unchanged tests and
full-debug native builds passed after increasing that dedicated container to
6 GiB within an 8 GiB VM. This validates Linux execution of those paths, not the
remaining Linux catalog gates or any Windows execution. The larger
supervisor restart probe exceeded the 6 GiB limit during linking; its 7 GiB retry
has passed markup-only acceptance, invalid-markup retention, a combined Rust/XAML
patch, and retention of the running application after a failed replacement build.
The valid replacement then acknowledged startup and preserved its saved counter
before the old process retired; the probe verified the next click advanced it from
7 to 8 and retained the accepted markup. Failed and retired generation directories were
removed in the expected order, and shutdown removed the accepted generation.
The native supervisor acceptance completed successfully under Xvfb.
The original nine supported catalog pages also passed native Linux reload, invalid-markup
retention/recovery, atomic-save, and owned root-property-removal checks. GNU ld
exceeded the container's memory limit on this larger full-debug executable;
`cargo rustc --locked -p control-catalog-desktop --features hot-reload --bin
control-catalog-desktop -- -Clink-arg=-fuse-ld=lld` linked successfully with LLD
18.1.3. This catalog result does not cover compiled/runtime parity on Linux.
The macOS `restart_state_probe.py` separately changed the live window to 520×300
at (80, 100), captured state after two clicks, and started another instance from
that state. It verified the exact placement and navigation, then the next click
advanced the restored counter to three. Invalid dimensions and placement changes
during handoff were rejected. This probe uses one ordinary `patch-control` build;
the supervisor probe separately verifies build-before-retire ordering.
The native macOS backend requires an active display: an initial attempt failed in
its render timer with `-6661` while the display was inactive. Repeating with
`caffeinate -u -t 30 python3 scripts/hot-reload/restart_state_probe.py` passed.
The placement probe also passed natively under Linux/Xvfb, including exact
non-default placement/navigation/counter restoration and invalid/frozen placement
rejection. It is included in the three-platform workflow; Windows execution
remains pending.
The development manifest profile probe passed all ten cases on macOS and Linux,
including custom profiles, build-script overrides and explicit target rustflags.
The subsequent Linux regression run passed 53 reload tests, 21 patch tests, and
404 loader tests, with one existing ignored loader test. This includes unchanged
text submission, failed-draft recovery, explicit function registration, and the
bounded runtime preparation cache. The macOS text-submission regression also
passed. The expanded native handler-only desktop probe passed under Linux/Xvfb:
it applied two combined revisions and checked both failure paths, then verified
a third, handler-only patch retained child identity and markup revision. Its
macOS attempt stopped at startup with native render-timer error `-6661`.
After a successful native display-link preflight, the expanded macOS probe passed
using `caffeinate -d -i -u` for the build and desktop run. It verified the combined
updates, both rollback paths, handler-only child identity, invalid-builder
dispatch/UI rollback, successful builder replacement, and preservation of that
derived view through a later handler patch. This supersedes the earlier
display-blocked attempt without treating that attempt as a product pass.
The subsequent Linux builder run also passed candidate-markup rejection with
dispatch/UI rollback, successful builder replacement, and a later handler-only
patch retaining the derived view. All 54 macOS session tests now pass, including
explicit reapplication retaining unsaved drafts and read errors. The tooling suite
passes 25 tests with two Windows-only skips after adding exact CRLF/UTF-8 source
snapshot coverage.
An additional macOS regression exercises a real background producer: handoff
waits for its pause acknowledgement, production requests remain queued while
state is captured, cancellation releases the queued work, a later participant
failure resumes it, and a pre-existing pause survives handoff. FIFO barriers
make these checks independent of sleeps or thread scheduling. This validates
the application participant protocol; native application-specific asynchronous
draining still requires integration coverage.

The macOS compiled/runtime catalog parity suite passed all 226 tests after the
ExpanderPage integration (`cargo test --locked -p control-catalog --lib
tests::compiled_markup`, without `runtime-markup`). It compares generated markup
with runtime loading, including application resources, shown visual trees, handler
dispatch and image assets. This covers initial-loading parity; reload ownership and
state transfer are validated separately. The same 226 compiled/runtime tests also
passed on Linux ARM64 under the isolated validation VM (Rust 1.93.0, LLD 18.1.3);
the test executable used only the default and placeholder-branding features,
without runtime-markup or hot-reload.

The full macOS native desktop patch probe was rerun after the root-property write
tracking change. It passed combined XAML/Rust revisions, invalid-markup
and missing-library rollback, handler-only graph preservation, invalid registered
view-builder rollback, and valid view rebuilding followed by another handler patch.
The original event subscription, component root and framework type identity
survived in the same process.

Root-property ownership regression coverage now checks equal-value application
writes both between reloads and between preparation and commit. The macOS run
passed all 58 reload tests, including rollback followed by retry; Linux passed
the same 58 tests. macOS also passed all 4,380
base tests with development write tracking enabled. Release compilation also
passed; the tracking field, API and mutation hooks are gated out without both
`debug_assertions` and the base `hot-reload` feature.

The macOS desktop demo passed initial loading, a valid edit, invalid XML rejection,
and recovery through atomic replacement in one process. Reproduce after building
the demo with `python3 scripts/hot-reload/markup_probe.py`. This checks reported
transactions, including an included color-resource revision; the unit tests additionally
check retained object identity on failure and static-resource values.
The initial build, patch-admission and reload suites passed (25, 6 and 11 tests respectively),
and release-mode checking passed for both new crates and the sample. Release checking
also passed after restart integration. The patch crate now has 17 passing tests,
including subprocess build failure, acknowledged restoration, startup timeout/version
failure, supersession, supervisor ticket ownership, input capture/staging, binary
assets, configuration changes and executable-permission preservation.
The reload suite has 52 passing tests, including restart participant recovery and
timer quiescence, metadata-based handler admission before target preparation, compiled-binding
type namespace changes, keyed unnamed controls, separate
named scopes, authored removals, transactional cached-reference/subscription
refresh over ten reloads (including failure in a later target), and a working two-way binding that remains writable
after state restoration. Further tests exercise 20 transitive-resource reloads across
two instances, release of detached roots, failed-revision retention, accepted-only
lazy source publication and session ownership. The markup suite passed 201 tests
after adding overlay precedence and failed-include retry coverage.
The native desktop restart startup probe is `python3 scripts/hot-reload/restart_probe.py`.

The macOS owning-supervisor probe passed compilation-free markup application,
invalid-markup retention, a combined admitted function/markup patch, retention of
that process after a broken replacement build, and a library-edit restart with the
counter restored. `supervisor_probe.py` runs that scenario in a private editable
checkout. The tooling suite now has 20 passing tests on macOS and two Windows-only
tests skipped, including compiler
process-group cancellation and thaw after startup failure,
supersession and failed source verification. The native handoff probe additionally
checks disabled interaction, rejected commands and explicit cancel/resume before the
stateful replacement. Subsequent polling/transport
optimization is covered by those tests: an observed 318 MB admission payload became
2.2 MB (76 ms serialization), and an idle metadata poll took 80 ms on this checkout.
Those measurements exclude debounce, UI layout and rendering; they are not
save-to-visible latency baselines.

The native macOS supervisor probe was rerun after process-tree cancellation and
generation cleanup changes. It passed the actual combined patch, invalid-markup
retention, failed replacement build, and acknowledged restart restoring counter
state. It also verified that the failed build directory disappeared while the
running generation remained, that successful handoff removed the retired generation,
and that shutdown removed the accepted generation. This is a macOS result, not a
substitute for the Windows/Linux acceptance runs.

These results do not establish complete support on all three desktop platforms, complete UI state transfer,
or leak freedom. The XAML and Rust render-completion baselines above are limited
to their recorded fixtures, platforms, sampling methods and sample counts.

## Release gate and historical implementation notes

The [completion audit](hot-reload-completion-audit.md) separates requirements from
optional extensions and supersedes this section's former expanding checklist.
Windows verification of the final implementation is still open. Existing Windows
runs are evidence for their recorded revisions only.

Native filesystem notification backends, every ControlCatalog page, a general
supervisor for arbitrary applications and restoration of ambiguous repeated
template instances are follow-ups, not additional first-release requirements.
Issues #233/#234 remain open pending the three-platform release gate. The dated
implementation notes below preserve the findings and limitations of earlier pins;
use the completion audit for current status.

### Shared selection during subtree replacement

Content, component and panel transactions now preserve inherited data contexts on
retired trees while detaching them. On rollback, the reattached tree resumes
inheritance. Explicit application data contexts remain untouched. This avoids
publishing empty binding values into view models shared with the replacement tree.
It does not promise rollback of arbitrary application property callbacks.

ListBoxPage is now registered for catalog reload. Its integration regression uses
the actual 10,000-item view model and shared selection model in a shown test window.
Three failed transactions followed by successful replacements preserve selection
index 42 and the view-model identity; each retired tree is released. The regression
first reproduced a selection-source panic and loss of selection, and passes with
the context handoff. The 58 engine tests also pass on macOS. Native macOS ListBoxPage acceptance also passed valid edits, invalid-markup
recovery, atomic save and root-attribute removal. Its three save-to-render
observations were 1,826.92 ms, 213.57 ms and 186.57 ms under concurrent compilation;
the first may include startup/navigation work. Raw stage timings and limitations
are in `hot-reload-listbox-baseline-macos.json`. Linux/Windows coverage of the new
handoff remains pending.

Enabling automatic catalog tracking exposed a TLS destruction-order bug: a
thread-local session could outlive its source registry. Registration cleanup now
uses fallible TLS access, and persistent populate cleanup removes only the owning
session's hook even while a constructor-scoped override is active.

After the teardown fix, macOS passed all 58 reload tests, all 203 markup tests
(including TLS destruction and persistent-hook ownership), and the catalog
shared-selection regression with automatic tracking enabled. Release checks for
both the engine and ControlCatalog with development features selected also passed.
The Linux standalone registered-function probe passed two accepted patches and
three view-transaction rollbacks, including a panicking candidate, while retaining
its original framework object, type identity, subscription and counter.

### Application-resource transaction coverage

The global-resource regression initially accepted a new dictionary while a static
consumer retained its previous value. Candidate-only resource resolution now fixes
that mismatch. The macOS engine suite passes 61 tests, including application-style static
resources, registration-order
independence, removal of required resources, chained dependencies, shared resource
identity after commit, rollback after a later failure, no notifications from failed
preparation, and rejection of duplicate ownership. New previews are scoped to the
UI thread and disappear on unwind; ordinary release lookup retains its original
behavior. macOS also passes 203 markup tests, 4,380 base tests and a release
check with development features selected. Linux passes all 61 engine and 203
markup tests including the final ambient-lookup and application-style additions.
These are unit/integration results; native desktop acceptance of this resource
change remains outstanding on all three platforms.

The Linux native desktop patch probe passed the shared-context handoff build:
combined revisions, both rollback paths, handler-only graph preservation and
registered view-builder replacement. That snapshot predates the later TLS cleanup
and resource-preview changes; it is not final acceptance of the current tree.

### Include discovery integration

Development resource-provider metadata exposes an include's canonical source URI
without loading its document. Absolute sources retain their assembly; relative
sources resolve against the include's original base URI. The interface addition
and implementations are gated by both debug assertions and the development
feature. Discovery adapters retain weak owners and preserve application-owned
siblings. The application-resource/style integration regressions now use actual
cached includes and discover their targets, checking subsequent replacement,
missing-resource rejection and duplicate ownership.

The real-application regression exposed group compilation removing include
wrappers. Runtime document builds now record weak resource-root provenance so
optimized includes remain discoverable. Provenance records do not retain the
resource graph and are excluded from ordinary release builds. This covers the
runtime-markup development path; emitted compiled resource factories do not yet
record equivalent provenance.

macOS validation for include discovery: 62 reload tests, 204 markup tests, both
ControlCatalog reload integration tests, and the ControlCatalog release check
with development features selected passed. Linux also passed all 62 reload and
204 markup tests including the linked-root provenance change. These results do
not replace native desktop acceptance.

### Native application-resource acceptance

After building ControlCatalog with `hot-reload`, run
`python scripts/hot-reload/resource_probe.py` (under Xvfb on Linux). The probe
uses isolated copies of BorderPage and CustomThemes, adds a resource, makes the
page consume it through StaticResource, and changes the resource through atomic
saves. It checks actual content width and stable native-window, component-root
and view-model identities. Removing the required key must retain the previous
content object and value; restoring it must recover. It finally removes the
consumer and resource again. Opt-in `FERROUI_RELOAD_TARGET_STATE=1` emits only
object identities, content width and revision outcome for these assertions.

The macOS native run passed on the include-discovery engine with the new probe
instrumentation. Its six save-to-render observations are recorded in
`hot-reload-resource-baseline-macos.json`; four resource updates took 227–380 ms,
and the two page updates took 177–187 ms. No compiler ran concurrently. These
are individual observations, not percentile estimates or display scan-out
measurements. They validate static resources on replacement children, not every
root resource expression or deferred template. The same probe subsequently passed
on Linux/Xvfb before the root-theme extension; Windows acceptance remains outstanding.

A fresh native macOS desktop patch probe also passed against the include-discovery
engine using Rust 1.93.0 and dx/Subsecond 0.7.10. The process, live root, framework
type identity and existing handler survived two combined Rust/XAML revisions and
both rollback paths. Handler-only updates retained the child and markup revision;
registered view-builder updates rejected invalid candidate markup with code/UI
rollback, replaced valid views, and retained the derived view through a subsequent
handler-only patch. This rerun covers the current engine through include discovery;
Windows and the equivalent latest Linux native patch rerun remain outstanding.

### Preserved-root static resources

A regression initially accepted an application-theme edit while the preserved root
still held its previous StaticResource theme. Explicit static-resource ownership
now refreshes the root from the same prepared resource instance that commits.
The regression checks style and template effects, rollback after a later failure, missing-key
rejection, preservation of the previous template graph on rollback, release of
the retired theme/template parts, attribute removal/reapplication and
same-value application writes transferring ownership. All 63 reload tests pass
on macOS and Linux; both macOS catalog integration tests and the release check
also pass. The native resource probe now also requires root-theme identity to change
on successful dependency revisions and remain unchanged on failure; these stronger
native runs passed on macOS and Linux/Xvfb. Their raw observations, compiler
versions and source hashes are in `hot-reload-root-theme-baselines.json`; other
validation builds were running, so these are correctness-run measurements rather
than a clean performance benchmark. Windows remains unverified.

### Preserved-root bindings

Component and panel root-binding integration now has 85 passing engine tests on
macOS and Linux, including original-expression rollback, fresh child namescopes,
compiled bindings, multiple instances, pre-activation admission, resource-supplied
fallbacks and paths, unchanged data-context subscriptions, direct-source rebasing
and application ownership after a rejected commit. macOS also passes all three
catalog integration tests, 4,381 base tests, 204 markup tests and 404 loader tests
(one existing metadata diagnostic is ignored), plus release compilation.

The native `root_binding_probe.py` passes on macOS and Linux/Xvfb. It checks
literal-to-binding replacement, updates through the preserved root's self source,
invalid/unsupported edits retaining the last-good graph, atomic recovery and
binding removal while retaining window/root/model identities. The workflow runs
this probe on all three desktop platforms. Raw stage timings and source hashes are
in [the root-binding observations](hot-reload-root-binding-baselines.json);
these functional runs were not isolated latency benchmarks. Windows execution,
latest-revision native Rust-patch validation, and the other documented integration
and release gates remain open.

## Native verification after root-binding integration

After the root-binding integration, fresh desktop patch and
owning-supervisor probes passed on macOS arm64 and Linux arm64/Xvfb with Rust
1.93.0 and dx/Subsecond 0.7.10. The desktop probes covered combined revisions,
code/UI rollback, preserved subscriptions and type identity, and explicit view
builders. The supervisor probes retained the running application after a failed
replacement build, restored state on acknowledged restart, and removed failed,
retired and shutdown generations. The standalone macOS probe also passed its
three rollback cases, including a deliberate panic.

The existing catalog binaries built for the same implementation passed all ten
pages and the resource/root-theme probe on both platforms. Python harness tests
ran 27 cases on each platform: 25 passed and two Windows Job Object tests were
skipped as expected. The acceptance workflow now includes the base property-store
suite and disables incremental output to reduce temporary build storage.

[Recorded observations](hot-reload-native-acceptance-macos-linux.json) identify the
tested source, toolchains and individual gates. The supervisor observations took
5.68 seconds on macOS and 10.89 seconds on Linux from save to render completion,
including 3.06 and 2.82 seconds in the patch compiler respectively. These are
single functional observations with potentially overlapping validation work,
excluding watch debounce and physical display scan-out.

These runs predate the Windows watcher-shutdown fix below. The Windows results
are recorded separately; the full current revision and remaining integration work
still require acceptance before release.

## Windows fixes and CI repairs

The Windows verification led to the following fixes. The catalog session now belongs to the application
locator scope; its thread-local handle is weak. Explicit desktop exit cleanup
releases the watcher before thread teardown, avoiding the Windows loader-lock
deadlock found on Windows. Compiler failures also keep their full diagnostics,
the build tests use portable paths, the supervisor uses shorter temporary paths
and the sample has opt-in click diagnostics.

After these fixes, four catalog tests, 85 reload tests, 19 patch tests, the
Python harness and a fresh native root-binding probe passed on Windows (the
[Windows report](hot-reload-windows-verification.md)); its complete 21-command run
applies to an earlier revision. The full current 22-command Windows matrix
remains open.

The release groups and license/notice links of both development
crates are repaired, the generated README table is current, and theme/handler
reference files have been regenerated using their existing generators. These
address the inherited package and generated-reference CI failures. A local
49-crate packaging dry run passed with `--no-verify --allow-dirty`; it neither
compiled every packaged crate nor published anything.

Local macOS integration checks passed all four catalog reload tests, all 25
build-task tests and 598 markup tests (17 existing ignored tests). Python ran
27 tests, with 25 passing and two Windows-only skips. The theme suites passed
200 Fluent, 203 Simple and 286 WinUI tests, retaining their existing ignored
tests. Each theme run explicitly filtered out the two managed file chooser
tests: a preceding unfiltered Fluent run was stopped after both tests blocked
in `BclMountedVolumeInfoProvider::listen`, inside the OS directory-open call
while enumerating mounted volumes. This is a local verification limitation,
not a passing result for those six cases. No test assertions or repository
test filters were changed.

All four catalog reload tests also passed on Linux arm64 with Rust 1.93.0.
GNU ld was killed while linking under the local container's memory limit.
Rebuilding the test target with
`cargo rustc --locked -p control-catalog --features hot-reload --lib --profile test -- -Clink-arg=-fuse-ld=lld`
and running that binary with the `tests::hot_reload` filter succeeded.

The subsequent caret-state correction passed 89 engine cases on macOS (the
88-case suite followed by the additional session regression) and the full
89-case Linux suite. Before the fix, two regressions reproduced the lost active
caret and stale selection after removing an authored caret position. Coverage
also checks each two-way cursor binding and six successful replacement/failed
commit cycles, including release of retired text boxes. These are engine tests;
native Windows verification of this follow-up remains outstanding.

Live include tracking passed all 92 engine tests and all four
catalog reload integration tests on macOS and Linux. The new cases exercise
late include attachment, failed-draft retry, repeated replacement and rollback,
removal/reinsertion, retired contribution release and weak container lifetimes.
A fresh macOS catalog build also passed the native resource/root-theme probe
through session container tracking, including missing-key rollback and atomic
recovery with stable window/root/model identities. [The recorded run](hot-reload-include-tracking-macos.json)
includes source/compiler/binary identities and six functional timing observations.
It does not test native late attachment/removal or replace current-revision
Windows/Linux native acceptance.

These checks do not establish a green remote CI matrix or final feature
acceptance. Current-revision native patch/supervisor runs and the remaining
resource, style, state and handler integration boundaries still apply.

Application control-state callbacks and the rapid-revision restoration correction
passed 97 engine tests and five catalog reload integration tests on both macOS
and Linux. The regression first reproduced a lost checked value when a second
revision arrived before the first restoration callback. Restoration now completes
once, against the accepted source snapshot, before preparing another candidate;
the scheduled path updates layout before restoring state and precedes rendering.
Tests also cover authored changes/removal, two-way bindings, ambiguous identities,
changed types, replaced graphs, rollback and release of retired controls. These
are engine/integration results, not native Windows acceptance of this change.

Build compatibility metadata subsequently passed 27 build tests and 99 engine
tests on both macOS and Linux using
`cargo test --locked -p ferroui-build -p ferroui-hot-reload --features ferroui-build/runtime-host --lib`.
The build cases exercise both compiler hosts, custom/inherited/qualified events,
markup-extension resolution, binding aliases and sharing metadata across documents.
Session cases reject handler edits before preparation and retain the same contract
across lazy runtime registration; generic type-argument changes also require restart.
All six macOS catalog reload integration tests passed, including actual generated
manifest admission for a `<Binding>` path edit and application state restoration.
ControlCatalog's manifest contains 220 documents, 240 shared type contracts and
495 handler assignments; sharing the table avoids duplicating inherited event
metadata in every document. These focused results do not replace final native
acceptance on Windows or Linux.
