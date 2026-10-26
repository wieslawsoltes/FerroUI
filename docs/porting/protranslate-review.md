# ProTranslate portability and correctness review

This review compares the first Rust port (`9d4339e3`) with upstream
`b088669a22c8979d54dd81098991f8d16f880156`. It covers the source libraries and the
UI-independent services in the Translation Studio sample. Findings are corrected
in separate commits on the original port PR. All findings recorded below are
addressed; the remaining host/runtime boundaries are documented in the
[port mapping](protranslate.md).

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
| Generator | Infer Android resource-directory (including script/region qualifiers) and Apple localization-directory cultures; include empty declared cultures in coverage diagnostics; expose runtime manifests, key-declaration provenance and service extension accessors; validate direct provider keys; resolve only unambiguous constants in their actual syntax scope. |
| Native adapter | Match UI/format culture semantics, native fallback behavior and scoped/ambient replacement. Respect explicit flow direction and release automatic values when disabled. Preserve custom DI registrations and factory dependencies. |
| Compiled sample | Exercise generated strings through actual compiled XAML property bindings, including globalization override notifications and view-model release. |
| Portable authoring | Port catalog editing/review/filtering, coverage, undo/redo, session/settings/recovery/backup persistence, and AI request/suggestion orchestration into a reusable UI-independent package. |

## Additional findings resolved during integration

- Custom numeric formats preserve .NET rounding, original numeric widths,
  permissive literal parsing and checked decimal scaling. Four-digit N/C/P/custom
  grouping is forced where the source does so, including Polish and Spanish.
- Globalization and format-error policy capture one coherent snapshot before
  calling injected services or diagnostic subscribers.
- Native boxed date arguments retain unspecified or explicitly UTC kinds, as well
  as offset-bearing values. Culture-selected calendars replace the first port's
  Gregorian-only boundary.
- Key-only declarations retain source/line provenance without inventing runtime
  translations. A nested Rust module cannot resolve constants from an enclosing
  function's local block; static facade/accessor analysis stays explicitly scoped.
- Studio sessions preserve complete catalog and document metadata. Stable IDs and
  monotonically changing revisions prevent a delayed suggestion from overwriting
  newer edits, undo/redo state, recreated entries or a restored session.
- Session, recovery, backup and settings namespaces must be disjoint. The validated
  paths cannot be mutated afterward. A recovery cleanup therefore cannot delete a
  saved session. Backups use atomic create-if-absent, including across concurrent
  storage clients and restart/rotation sequences.
- AI responses must exactly match the requested identities. Refusal, truncation,
  malformed/partial output, pagination failures and cancellation remain errors.
  Live discovery never substitutes historical models; a host must select a model.
  Tests do not need provider credentials or make real network calls.
- Native state metadata remains intact during ordinary Studio export. Explicit
  review-state projection is available and reports destination representation
  losses rather than silently overwriting unfamiliar format states.

## Validation

Each correction includes a regression for its observable behavior. Validation
includes generated Rust compilation and execution, native binding/compiled XAML
tests, provider/culture lifecycle tests, deterministic catalog round trips, and
authoring transactions with fake storage/transport failures. Strict Clippy is
scoped to the selected packages; existing framework regression suites cover the
generic engine corrections. The focused workflow tests portable services on
Linux, Windows and macOS and checks WebAssembly compilation.

Final local regression totals:

| Package | Passing tests |
| --- | ---: |
| Core | 61 |
| Formats | 46 |
| Build | 31 |
| Studio | 38 |
| Native adapter with dependency injection | 34 |
| Compiled XAML sample | 11 |
| Total package tests | 221 |

The core tests include 1,404 frozen numeric reference vectors from .NET 10.0.14;
they validate invariant-culture algorithms, not identical bytes across every
host's locale database. Calendar tests cover 16 families. Build tests compile and
execute generated providers/models. Studio includes 17 fake-transport AI tests.
Seven focused generic binding-engine regressions also passed. The final checks
include scoped strict Clippy, rustfmt, a desktop binary check, portable WebAssembly
compilation, source conventions and metadata/notice checks for all 76 published
workspace packages. The PR's workflow records the cross-platform results.

Runtime-specific objects such as managed assemblies, `ResourceManager`, and
`IStringLocalizer` still cross the existing provider bridge. Platform windowing,
clipboard access, file pickers and host scheduling belong to the FerroUI app or
host. They do not remove any catalog, formatting, authoring or persistence policy
from the portable packages.
