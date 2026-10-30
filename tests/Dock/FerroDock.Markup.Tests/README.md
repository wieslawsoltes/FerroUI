# Downstream markup tests

This unpublished workspace package compiles XAML against FerroDock library
metadata exported by a different Cargo package. It uses the published FerroUI
0.3.0 compiler, loader and Headless runtime.

`ProportionalConsumer.xaml` resolves the exported panel/splitter types, attached
properties, compiled two-way attached-property paths, and a ResourceInclude from
the proportional library. The test checks native layout, the implicit splitter
theme's generated Border, resize writes to the source and later source updates.
Its compiler-default assembly URI is
`ferres://ferrodock_markup_tests/ProportionalConsumer.xaml`.

Run from the FerroDock directory:

```sh
cargo test -p ferrodock-markup-tests --locked
```

These checks cover this dependency boundary. Full docking themes, interface model
metadata, the reactive control's consumer markup, desktop targets and application
integration are still tracked separately in the port inventory.

`RecyclingConsumer.xaml` additionally consumes the native recycling cache/template,
sets its direct bool and inherited attached cache properties, and compiles the
named-presenter Parent binding. The Headless window initializes the forward
StaticResource before inspection; changing content away/back proves cache reuse.
The shared emitter's duplicate nullable branches and cloned direct bool literal
have two Clippy allowances scoped to the generated module only.

`OverlayHostConsumer.xaml` consumes the native UI assembly's actual host, both
collection boundaries, layer contract/content metadata and original static style
key field. Headless checks the pipeline, service-layer disable flag, explicit
layer precedence, z-index and hit testing. The full original themed application
remains separate pending work.

`ServiceOverlayConsumer.xaml` loads the original busy/dialog/confirmation resource
files from the native Fluent theme assembly. Its Headless window verifies compiled
bindings update actual template controls from production services, execute reload,
preserve live request collection identity/notifications and clear commands/items
on null service replacement. The full DockFluentTheme remains incomplete. The theme emitter's Copy-literal clones have a `clone_on_copy`
allowance confined to that library's generated module.

The Headless application for service overlays now loads published FerroUI
FluentTheme 0.3.0, so actual ItemsControl presenters display the original request
DataTemplates and shell controls. New tests assert content/title/label and compact
dimensions, then simulate keyboard input on buttons to complete original dialog/
confirmation tasks. `DefaultOverlayConsumer.xaml` is a fifth fixture; its two-host
case verifies source nonshared default layers, six service identities, displayed
style aliases, local/global busy state, rendered confirmation input and DataContext
rebinding. These tests do not stand in for the unfinished full DockFluentTheme or
original App markup harness.

`ThemeResourcesConsumer.xaml` compiles all eleven original DockOperation values
through dependency-exported enum metadata and includes the full source palette
and control strings. Tests verify actual enum payloads, sizing, brush alias
identity, dynamic-color updates and icon geometry objects. Five preset consumer
fixtures load the source accent dictionary followed by each original preset;
all preset resources resolve in their original owner context. The resource tests
add two functions; this package now runs nine Headless tests in total.

`LifecycleConsumer.xaml` consumes the library's native lifecycle attached
property. Its Headless test supplies the original domain interface and verifies
attachment, removal, reattachment and immediate disable. This adds the tenth
external test; full original DockFluentTheme/App markup integration remains pending.

`RootControlConsumer.xaml` is a test-only template for the exported native
RootDockControl class. Its eleventh external Headless test boxes the original
ReactiveUI root interface, verifies native accessibility metadata and dismisses
an original engine preview through pointer input. Both boxing helpers retain the
original model identity. The original complete RootDockControl theme and full
DockFluentTheme/App markup remain pending; this fixture is not a substitute theme.

`DeferredConsumer.xaml` is the twelfth external Headless test. It resolves an
unprefixed library class via the original framework XML namespace, compiles a
plain timeline object with inherited attached scheduling and reads the shared
default through x:Static. Real native dispatcher timers materialize and replace
content after positive delay. The root consumer now also uses the actual
DeferredContentControl main part. These consumers do not complete the original
full root/docking themes, alternate models or application harness.

Original core-enum metadata now comes from the Settings dependency, while earlier
Ferro descriptor access remains a hidden alias. This package directly references
Settings for the emitted resource code. Its twelve cases remain; a separate
FerroDock.Settings.Markup.Tests package adds a thirteenth external case and proves
settings-only compilation without the docking UI or themes.

The package now runs 30 tests. `MdiThemeConsumer.xaml` loads the complete original host
dictionary with actual production ReactiveUI models and native MdiDocumentWindow children.
Four source empty-content scenarios retain attachment/template order, plus live item/state/
stacking checks. The full document window/content theme and global DockFluentTheme remain
pending. `DockModelConsumer.xaml` verifies inherited and own model bindings, native policy
resources, live Count/list replacement, two-way fields/null object transport and original
CascadeDocuments execution through a native Button. Its indexed path explicitly records
the published Item vs Item[] stale-subscription gap; path replacement recovery is checked.
It does not claim that shared-runtime defect is fixed.

The package now runs 33 tests. `CapabilityConsumer.xaml` compiles the source capability,
removal and close-visibility converter fields and original nested factory command overloads.
Three Headless cases verify live base/dock/root/override invalidation and source rebinding,
list Count/close mode/actual native MDI frame active-state changes, plus original layout/tab/
close commands. The complete source MDI window/global themes remain pending; this fixture
adds no replacement production template or docking engine.

The package now runs 35 tests. `ManagedLayerConsumer.xaml` loads the COMPLETE original managed
layer dictionary and binds its owning dock from an explicit outer element. Two cases verify
actual ManagedDockWindowDocument/MdiDocumentWindow frames, live original list/model/state/
topmost/ZIndex behavior, rebinding/stale-source exclusion and original overlay canvas position/
size/removal. Native nullable Dock conversion retains original raw/handle/nullable model
identity. The complete document window/global themes and actual host/tracking/pinned sample
remain pending; these tests add no replacement production template or engine.
