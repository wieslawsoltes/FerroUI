# ProTranslate portability and correctness review

This review compares the first Rust port (`9d4339e3`) with upstream
`b088669a22c8979d54dd81098991f8d16f880156`. It covers the source libraries and the
UI-independent services in the Translation Studio sample. Findings are corrected
in separate commits on the original port PR.

## Review contract

Inputs include separate formatting and UI cultures, mutable provider catalogs,
reentrant notifications, native binding values, imported platform files, explicit
build inputs, translation edits and host-provided persistence/AI transports.

Outputs must preserve resolved translation metadata, native binding diagnostics,
catalog data, generated API contracts and reversible authoring state. Invalid
input must return structured diagnostics or errors without publishing a partial
catalog or corrupting a saved session.

The runtime core stays independent of UI and authoring packages. Core callbacks
run outside internal borrows, target ownership stays weak, and source replacement
or disposal disconnects subscriptions. Generated providers and native compiled
bindings remain the normal application path. Authoring services use explicit
storage and transport boundaries instead of process-global settings.

## Findings and required behavior

| Area | Review requirement |
| --- | --- |
| Native binding engine | Preserve converter errors, validation notifications and fallback values; clear diagnostics after recovery. An empty property notification must invalidate indexers as well as named properties. |
| Observable translations | Expose resolved values, culture and diagnostics even when the rendered text is unchanged; support explicit disconnection and deterministic release. |
| Cache invalidation | Track composite provider revisions without lossy hashing; reentrant clear/remove operations must prevent stale lookup results from reentering the cache. |
| Numeric formatting | Preserve original numeric widths and binary/decimal rounding semantics; implement portable custom masks, scientific notation and culture-specific symbols. Validate representative cases against a pinned .NET runtime. |
| Dates and calendars | Use culture-selected calendars and source-compatible standard/custom date behavior; preserve explicit offset semantics and validate representative runtime outputs. |
| Catalog converters | Retain document metadata, Android flags, PO headers/escape semantics, Apple variation structures and XLIFF structure where representable. Reject malformed or ambiguous data instead of silently changing its meaning. |
| Generator | Infer Android resource-directory and Apple localization-directory cultures; include empty declared cultures in coverage diagnostics; expose runtime manifests and service extension accessors; validate direct provider keys. |
| Native adapter | Match UI/format culture semantics, native fallback behavior and scoped/ambient replacement. Respect explicit flow direction and release automatic values when disabled. Preserve custom DI registrations and factory dependencies. |
| Compiled sample | Exercise generated strings through actual compiled XAML property bindings, including globalization override notifications and view-model release. |
| Portable authoring | Port catalog editing/review/filtering, coverage, undo/redo, session/settings/recovery/backup persistence, and AI request/suggestion orchestration into a reusable UI-independent package. |

## Validation

Each correction includes a regression for its observable behavior. Validation
includes generated Rust compilation and execution, native binding/compiled XAML
tests, provider/culture lifecycle tests, deterministic catalog round trips, and
authoring transactions with fake storage/transport failures. Strict Clippy is
scoped to the selected packages; existing framework regression suites cover the
generic engine corrections. The focused workflow tests portable services on
Linux, Windows and macOS and checks WebAssembly compilation.

Runtime-specific objects such as managed assemblies, `ResourceManager`, and
`IStringLocalizer` still cross the existing provider bridge. Platform windowing,
clipboard access, file pickers and host scheduling belong to the FerroUI app or
host. They do not remove any catalog, formatting, authoring or persistence policy
from the portable packages.
