# Isolated FerroUI resource and object-namescope support

`ferroui-base` is an exact copy of the published 0.3.0 Cargo package with the
twelve source changes listed in `ferroui-base-provenance.json`. Original source
hashes and Git revision are retained there. Its MIT license and original notices
remain in the package. Copyright (c) Wiesław Šoltés.

The patch supplies safe `Any` type recovery for plain resource providers, the
optional original dictionary-interface projection, and the shared mutable merged
provider list through IResourceDictionary. ResourceDictionary exposes its existing
list through that contract. No alternate resource storage or side registry is used.
These APIs are required by DockPresetThemeManagerBase's original algorithms.
All undeclared source files must remain identical to the published package. The user's
separate FerroUI checkout is not modified.

The fourth source file fixes the existing assignability path in `ValueTypes::try_convert`: an object-form reference can be assigned to a nullable interface target, just as `try_cast` already supports. This preserves object-valued IconTemplate APIs and prevents native icon presenters from falling back to full application views. A direct registered conversion is not invoked twice on failure.

The shared namescope extension restores the original arbitrary-object contract in
six further base files: INameScope, NameScope, ChildNameScope, NameScopeLocator,
NamedElementNode and the INameScope markup metadata. Native typed methods remain
available as projections, while the value methods share the very same map and pending
searches. Model identity uses the existing shared reference-handle conversion. Plain
objects are held strongly; the existing attached native-owner weak rule remains.
Native-only third-party custom scopes remain compatible and report UnsupportedValue
for plain registration until they implement the value contract. Registered null values
are rejected after the original completion guard. There is no second namescope.

The two corresponding markup runtime changes and two compiler changes are declared
in the separate published-package provenance manifests and patches. See
ferroui-markup-xaml-patch.md. Neither the original Dock nor sibling FerroUI source is
modified. All retained packages are MIT, copyright Wiesław Šoltés.

The existing modified data/core/value_type.rs now also supports shared reference identity
metadata. Concrete Rc reference registration supplies its original object identity;
explicit typed interface/class-view accessors can refer to the same enclosing instance.
NameScope and the Dock reference converter use this shared metadata, never value equality.
Native projections run once per operand and remain alive through comparison. Identity
addresses are compared without dereferencing them; no model instance is held by the
metadata registration. The existing original-resource/cast changes are retained.

Two additional files record the actual declaring Rust owner in
MarkupType/ferro_markup_type when compiler metadata is enabled. This compiler-only slot
prevents an external-model metadata carrier being mistaken for the constructor's model
type. Normal non-compiler builds retain NotRecorded storage. The runtime emitter reuses
the existing public type-path registry; complete-component substitution avoids fabricated
identifier suffixes. Exact published/modified hashes and compiler diffs are declared.

The published `ferroui-build` 0.3.0 package is additionally retained with exactly
one modified source file, `type_system/emit_types.rs`. Canonical model keys remain
unchanged; only emitted root return signatures use each declaration's existing
public export path. Complete type components are resolved independently, including
generic arguments, so private modules do not leak into consumer signatures and a
metadata carrier cannot rename another returned model. The original license, notices
and VCS metadata remain. `ferroui-build-provenance.json` and its declared patch
record every original/modified hash; the shared integrity guard checks all four copies.

The existing loader/build patches additionally distinguish a native declaring
trait's own shared handle from an external interface metadata carrier. A direct
Rust Rc unsizing coercion is emitted only for the actual shared source handle or constructor value
and actual declaring target trait. Model class views and foreign contract carriers
use the existing registered cast path, matching the runtime loader. This fixes
original SplitView pane/content assignments without creating a second conversion
engine or treating descriptor traits as original model traits. Source/modified and
patch hashes cover these changes in the existing five loader and one build files.

The shared-source check also accepts the actual constructor value when a declaration
lists its unshared form first (`[T, Rc<T>, Option<Rc<T>>], this: Rc<T>`), as in the
porting guide. Model and runtime metadata report that value's shared representation
explicitly. An executable consumer and runtime-emitted native trait assignment
confirm valid unsizing is retained while external descriptor casts remain rejected.

The published ferroui-build-scan 0.3.0 package is additionally retained with two
modified files, model.rs and scanner/source.rs. Both typed and checked shared-box
casts register the same assignability pairs at runtime; the scanner now records
both, including declaration tokens and normalized unread-call counts. Two scoped
scanner regressions cover the change. No registration predicate is bypassed: emitted
loads use the existing runtime checked cast. Concrete SDK projections now declare
their closed registration types while retaining the same shared generic projection
function. Exact original/modified/patch hashes, license and VCS data are recorded in
ferroui-build-scan-provenance.json. The integrity guard checks all five packages.

The published `ferroui-native` 0.3.0 package is retained with four native input conversions fixed: bookmark creation, opening/closing security scopes and file-reference URI resolution borrow caller-owned `IFrnString` using `GetNSStringWithoutRelease`. Rust retains and releases its original COM reference. The old consuming conversion caused a native double release after macOS Save As. A real Objective-C++ boundary test checks input lifetime before and after caller release. The license, source hashes and exact patch are retained in `ferroui-native-provenance.json` and `ferroui-native-borrowed-storage-uri.patch`.

The same native package now guards native accessibility queries/actions against released controls using ControlAutomationPeer.try_owner(). A live owner is held only for each callback; stale peers return unavailable/default results, without retaining removed visual trees or propagating an owner-missing panic into the dispatcher. A regression retains a native wrapper after its button is released and verifies query/action rejection. Native node bookkeeping retains its original behavior.
