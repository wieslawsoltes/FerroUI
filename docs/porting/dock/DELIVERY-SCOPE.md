# FerroDock delivery scope

The user narrowed the delivery target on 2026-10-23 to core docking, standard
serialization, the normal theme and the Browser tabs theme, with their supporting
functionality working. This scope supersedes earlier requirements to finish every
project/file in the original repository before delivery.

The normal theme is interpreted as Fluent, which the existing native sample uses.
Standard serialization uses the existing SystemTextJson port and ordinary JSON layout
save/load. Implemented docking behavior and APIs must continue to follow the original
engine. Views remain passive, with ReactiveUI commands/behaviors and compiled markup.

## Acceptance requirements

| Requirement | Current evidence | Remaining delivery work |
| --- | --- | --- |
| Original docking engine and ReactiveUI model backend | Native sample and engine tests; downstream graph tests instantiate actual root, split, document and tool models with original factory initialization. Composed native pointer tests now move, float into actual themed HostWindows and cross-window redock after JSON restoration, preserving identities and cleaning up the empty host and registry. | Six composed tests now also cover rendered selector splitting, actual splitter resizing, document/tool reorder, pin/preview/unpin, factory hide/restore and close modes under both themes after JSON restore. Physical document reorder and tool floating/docking pass by user confirmation. Final Browser-specific docking/flyout confirmation remains pending. |
| Standard JSON layout persistence | A production ReactiveUI adapter supplies14 standard layout models plus DockWindow to the existing serializer. Default wires, full graph/configured containers, stream save/load, shared references and original factory use pass against source. Restored layouts support native tab selection and pin/unpin under both themes. | The sample now registers its12 application types; programmatic load restores original classes/contexts/locators/commands and actual views under both themes. File menu commands now pass end-to-end routed menu tests through the actual native storage adapter, controlled platform picker results and real files under both themes, including late bindings, cancellation/errors and restored views/locators/commands. Native Save/Open/cancel/malformed-file checks pass on the clean delivery build. Supported limits remain recorded: closed Rust layout enums currently accept declared values only; template/custom/unknown-interface cases remain partial. |
| Fluent theme | The native sample uses Fluent. Headless graph, tab, pinned host, resource and chrome tests pass. The grip width issue has a native resize regression; restored JSON layouts pass native tab/pin interaction. | Clean desktop restored layout, grip/resize, pin/preview/unpin and user-confirmed floating/docking checks pass; themed HostWindow roundtrips/redocking pass in composed Headless tests. |
| Browser tabs theme | A library compiles all five source documents. Native tests exercise tab pointer selection, cached views, delayed panel, selector states, light/dark resources and floating-host overrides. Restored JSON layouts pass native tab/pin interaction. The source sample has a live Fluent/Browser selector with original model/pointer/preset/variant checks. | Sample JSON persistence is integrated and tested through compiled menus under both themes. Browser light/dark, selection retention, pin/preview/unpin and document close pass on desktop. Floating/redocking pass in composed tests; final physical Browser docking/flyout confirmation is pending. |
| Required supporting features | Settings, shared panels, resources, native metadata and diagnostics are already present; the sample includes the main inspector/F11 wiring. | Runnable clean native example, F11/F9 and README usage are verified. Native storage/accessibility defects are fixed with provenance and regressions. |

Completion requires working library behavior and a runnable native example, meaningful
automated checks of the listed scenarios, and visual verification of the two themes.
An implemented feature or a passing subset of tests alone does not prove the whole row.
The existing desktop executable/window remains protected during this checkpoint; its
hash and timestamp are unchanged.

## Deferred scope

Additional model/MVVM backends, additional serializers, unrelated samples, exhaustive
porting of all upstream projects and inherited framework APIs are outside this delivery
unless a concrete dependency of the accepted features requires them. Complete general
compiler/catalog automation is likewise not a prerequisite for an ordinary usable
JSON model catalog. Existing implementations and their regression checks are retained.

`ports.json` continues to record broader source coverage and known omissions. Its
1,928-file denominator and `check_port.py --require-complete` are full-repository audits;
they are not the completion gate or a percentage estimate for this delivery scope.

## Current desktop checkpoint (2026-10-24)

The Mac was unlocked and the separate Rust sample was launched. F11 shows the actual
modified layout; F9 renders the overlay. Header grip width stretches across the available
space after native pane resize. Switching Fluent to Browser retains the selected
document; Tool1 pin/preview/unpin and native Open cancellation work.

Actual Save As exposed a borrowed URI double release in FerroUI's native storage backend.
Closing/replacing popup/layout controls also exposed stale native accessibility queries.
Both fixes are isolated in a seventh vendored 0.3.0 package, with exact provenance and
regression tests. The original Dock engine, models and sample production code are unchanged.
All127 native library tests and1967 workspace tests pass; strict all-target Clippy and
source/format/Python/seven-vendor checks pass. See VALIDATION.md and CONTINUATION.md.

The clean rebuilt example, `/Volumes/SSD/temp/ferrodock-port/FerroDock Core Review.app`,
was launched and actual native Save/Open passed. Reopening a real saved layout restored
three documents and their views; CreateDocument and F11 work afterward. Save/Open cancel
and malformed JSON preserve the layout. The user confirmed physical reorder and tool
floating/docking; changed document/tool order is visible. Browser light/dark rendering,
pin/preview/unpin, document close, F9, pane resize and grip stretching pass on the clean
build. All204 markup tests pass, including actual flyout binding and pointer opening after
reload and pin/unpin under both themes. Final physical Browser docking/flyout confirmation
remains pending. The broader1,928-file audit remains deferred.
