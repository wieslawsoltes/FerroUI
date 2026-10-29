# Hot reload completion audit

Audited on 2026-10-22. Implementation is complete; Windows verification of the
final implementation is still open. Historical validation notes retain their
original source revisions.

## Implemented, with recorded verification

| Requirement | Current evidence and limits |
| --- | --- |
| Runtime XAML and last-good replacement | Session, overlay, detached preparation, reversible content/component/panel adapters and failure tests exist. Native catalog probes ran on all three platforms at recorded revisions. |
| Sessions and build/loading integration | Manifest, persistent/scoped populate hooks, weak targets, document buffers, directory polling/debounce, superseded revisions, transformed-document cache and debug/release gates exist. Build handler contracts include signatures and conservative unresolved-type admission, verified by build/engine tests on macOS/Linux and generated-manifest catalog tests on macOS/Linux. |
| Stable windows, roots and view models | Native identity probes and lifecycle tests cover supported adapters. Root literal/static-resource/binding ownership supports removal and rollback. |
| Compatible UI state | Focus, text/caret/selection, scroll and selection-model tests exist. Application state callbacks and queued-revision state preservation are implemented. |
| Cached controls and subscriptions | Transactional refresh hooks and the Expander sample exercise rollback, disposal and retired-tree release. Eleven catalog pages are registered. This is representative integration, not all-page support. |
| Included resources/styles and static consumers | Registered application containers discover direct live includes; dependency revisions prepare resources and consumers coherently. Resource/theme probes and include lifetime tests exist. Standard inline resource/style ownership and routed handler ownership are implemented; see final acceptance below. |
| Explicit Rust patching | Real Subsecond patches update existing callback subscriptions, with stable framework identity, conservative admission and controlled application. Native patch evidence exists for macOS, Linux and Windows at recorded revisions. |
| Supervisor and restart | The dedicated sample owns admission/build/application, retains the old process on failed replacement builds and restores versioned application/window/navigation state. Native supervisor evidence exists. |
| Combined updates and measurements | Combined XAML/Rust acceptance and stage/save-to-render measurements exist for a small fixture and representative catalog pages. These are functional baselines, not statistical performance guarantees. |

## Final acceptance and remaining gate

The engine, loader, markup parity, catalog lifecycle/integration, real patch,
combined transaction, restart and release-exclusion checks pass on macOS and Linux.
The implementation guide, README and XAML design document describe the supported
contract, extension APIs and restart boundaries.

The remaining release requirement is **Windows verification of the final
implementation**; earlier Windows results apply to their recorded revisions only.

## Historical increment evidence

The notes below describe the state and test counts at each increment. References
to work then outstanding are superseded by the section above.

The Windows fixes are integrated. The full 21-command Windows acceptance run
applies to an earlier revision; focused Windows checks followed the fixes, without
a full acceptance run of newer code. See [the Windows report](hot-reload-windows-verification.md).

The application-state callbacks and rapid-reload correction passed 97 engine
tests and five catalog integration tests on both macOS and Linux. Windows has not
tested that revision.

The subsequent compatibility metadata change passed 27 build tests and 99 engine
tests on macOS and Linux, and six catalog reload integration tests on macOS.
The latter include actual generated-manifest admission for binding-path edits and
application state restoration. Final native acceptance remains a separate gate.

The inline-resource implementation passed 106 engine tests on macOS and Linux.
It covers component/panel dictionary identity, application keys/children, removed
keys, repeated commit/rollback, deferred resource identity and ownership transfer.
A root inline-theme test builds its template before commit, rejects missing or
invalid template resources, and verifies that a later successful template resolves
against the preserved root dictionary. Markup tests additionally check weak context
rebasing, nested scopes on unwind, and deferred setter errors without activating
data bindings: 271 markup and 205 XAML tests pass on both macOS and Linux.
The engine suite also reproduces and fixes a realized brush losing its
DynamicResource anchor when the candidate root dies; repeated rollback/commit,
subsequent resource changes and retired-brush/root release are covered.
All seven ControlCatalog reload integration tests pass on macOS, including
resource replacement/removal through the registered component adapter with the
catalog's assembly identity and assets. Commands:

```sh
cargo test --locked -p ferroui-hot-reload --lib
cargo test --locked -p ferroui-markup -p ferroui-markup-xaml --features ferroui-markup-xaml/hot-reload --lib
cargo test --locked -p control-catalog --features hot-reload --lib tests::hot_reload
```

The Linux release-mode check also passes with development features requested:

```sh
cargo check --locked -p control-catalog -p ferroui-hot-reload -p ferroui-hot-patch --features control-catalog/hot-reload --release
```

The inline-style increment passed 112 engine tests on macOS and Linux and eight
ControlCatalog reload integration tests on macOS, using the engine/catalog commands
above. Coverage includes repeated commit/rollback, application style ordering,
resource precedence, retired style/template release, and rejecting source-writing
setter bindings before preparation changes the model. A reproduced regression
where a style-selected theme bypassed template validation is covered: invalid
resources fail before commit, and a corrected template applies after layout.
The catalog test exercises inline styles through its actual component adapter.
The Linux release check above also passes for this increment with the development
feature requested; active reload modules remain gated out by `debug_assertions`.

The mixed-dictionary increment covers candidate/committed lookup parity for
application direct keys, nulls, inherited/default themes, and merged providers on
both sides of the replaced range. It retains owners across rollback and releases
retired providers. Explicitly assigned candidate dictionaries participate in the
preview, including deferred brush factories; unwind removes the scoped resolver.
All 115 engine tests pass on macOS and Linux, and all nine ControlCatalog reload
integration tests pass on macOS. The new catalog test exercises application merged
provider precedence, repeated replacement, rollback and removal through the actual
component adapter. The commands are the engine/catalog commands recorded above.
The Linux release check also passes with the development feature requested.

The routed-handler increment passes 120 engine tests on macOS and Linux and all
10 catalog reload integration tests on macOS. Coverage includes child/root handlers,
commit failure, activation after publication, repeated replacement without duplicate
handlers, application subscriptions, multiple component instances, panel ownership,
retired-tree release, and deferred factories built after their generation retired.
Non-routed add-only metadata events require an adapter before candidate population.
The AdornerLayerPage integration starts from compiled markup, reloads through the
interpreter, retains its page instance, and refreshes its cached removed adorner.

The markup suites pass on macOS and Linux: 271 markup tests, 205 XAML runtime tests, and
598 XAML/compiler integration tests (17 pre-existing ignored tests). These include
compiled/runtime event parity and exact emitted-source checks for the updated
context-aware event setter. Command:

```sh
cargo test --locked -p ferroui-markup -p ferroui-markup-xaml -p ferroui-markup-xaml-tests --features ferroui-markup-xaml-loader/hot-reload --lib
```

The Linux release check above also passes with the development feature requested;
event ownership storage and services are excluded from release builds.

These checks do not close the remaining template-state or final native acceptance
requirements.

## Follow-ups outside the first-release requirement

- Native filesystem notification APIs: directory polling already implements the
  requested watching behavior; atomic saves and debounce correctness remain required.
- Registering every ControlCatalog page rather than representative integrations.
- Generalizing the dedicated sample supervisor to arbitrary application layouts.
- Restoring state for ambiguous repeated-template identities: the plan requires
  compatible, unambiguous matches, so deliberately skipping ambiguous matches is valid.
- Patching arbitrary closures, async state machines or compositor callbacks.
- Visual designer tooling, explicitly deferred in the original plan.
- Broader statistical latency distributions beyond the required separate measured
  XAML and Rust baselines.

These follow-ups must not keep the original goal open after its required work passes.

## Template state increment (2026-10-22)

Compiled and runtime template factories now record weak declaration provenance.
State capture traverses logical and template visual children once, with separate
source fragments and document/declaration/owner identities. The compiled source
baseline remains immutable across accepted and scoped edits. Preserved component
roots with no Content participate, and unchanged objects are not restored onto
themselves. Unknown provenance and ambiguous repeated instances are skipped.

All 128 engine tests pass on macOS and Linux. Eight template regressions cover
root templates, nested and repeated instances, data templates, unique anonymous
owners, ambiguity, text/caret/focus, scroll, selection, two-way binding protection,
authored changes, rollback, rapid revisions and retired-part release. Declaration
locations use the parser's UTF-16 columns and XML line-ending rules; a Unicode
regression failed before that correction and passes afterward.

All 11 catalog reload integration tests pass on macOS. The added test starts from
the actual compiled BorderPage and included CustomThemes.xaml, then replaces the
included theme twice with interpreted markup, preserving its named page scroll
part's offset and releasing retired parts. The macOS markup suites pass 271 markup,
206 XAML runtime and 598 compiler/integration tests (17 existing ignored tests).
Final native/Windows acceptance and initial unnamed-key integration remain open.

The template increment's Linux markup suites also pass 271 markup, 206 XAML
runtime and 598 compiler/integration tests, with the same 17 ignored tests. The
Linux release check passes with the development features requested. These are
checks of the template increment; later key integration requires its own verification below.

## Initial development keys (2026-10-22)

The remaining standard integration item is implemented: both compiler back ends
record explicit development keys during initial population and template creation.
The registry uses weak object identities and exists only in an enabled debug
runtime. Keys do not assign public names or add namescope entries. State matching
prefers the key even when application code changes Name, and understands escaped
literal keys. Duplicate keys within one scope remain ambiguous rather than
causing duplicate namescope registration. Empty keys and expressions are rejected.

The catalog's included page-scroll template now uses an unnamed `hr:Key`; its
compiled-to-runtime state test verifies the initial compiled metadata as well as
replacement. The original initial-load key regression failed before this change.
The engine suite now has 132 passing tests on macOS. All 11 catalog integration
tests and all 404 loader tests pass there (one existing ignored loader test).
The broader markup suites passed 271 + 206 + 598 tests on both macOS and Linux,
with 17 existing ignored tests. The loader pipeline assertion now includes the
new transformer. Enabling development tracking also exposed an evaluator-host
regression: only a population with an event generation may intercept routed
handler registration; ordinary evaluator hosts keep metadata method dispatch.
The regression and owned-handler engine tests pass together on macOS.

The workflow now requests development-enabled loader tests and the full markup
parity suite. Python harness tests pass on macOS (27 run, two platform skips),
and the pinned Rust 1.93.0 profile probe and thread-local portability check pass.
Native acceptance of a frozen current candidate, including Windows, is still
required before release. Historical runs retain their original pins.

The current Linux snapshot passes all 404 loader tests and 132 engine tests as
well. A stale snapshot timestamp initially reused a binary missing the new table
row; verification copied and compared the changed files and refreshed timestamps
before rebuilding. Future snapshot syncs must not preserve mtimes older than a
previous failed build. These reruns include the owned-handler dispatch correction
and escaped-key/name-precedence regression.
