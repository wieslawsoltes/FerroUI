# Repeater acceptance boundaries

The acceptance target is the portable behavior of the pinned Repeater family,
implemented through native FerroUI services. [SOURCES.md](SOURCES.md),
[API.md](API.md), [API-SHAPES.md](API-SHAPES.md) and [TESTS.md](TESTS.md) describe
that scope; [VALIDATION.md](VALIDATION.md) records executed evidence by revision.

## Implementation closeout

The continuation addressed lined-flow outer-width constraints, partial
realization and metadata, frozen rows, timer/context/cache lifetime, native
focus order, transition cancellation and recycled-element identity. It also
corrected scheduler cancellation within an active frame, template destructor
reentrancy, hierarchical source mutation, selection bindings after source
removal, and animated scroll target ownership. Offscreen pinned rows now obey
collection removal/replacement/reset, including stable-key restoration.

Integration includes native class metadata and Rust View builders, runtime and
compiled XAML, a notifying 10,000-item catalog with all four layouts and
constraint-aware lined-flow metadata, and a browser interaction/rendering gate.
The previously undeclared flow-layout upstream test module is compiled by the
native suite. Diagnostic item selectors route through the existing logger;
Windows ETW and class-registration profiler instrumentation are explicitly
separate from portable control behavior.

The source map must contain no unexplained runtime or declaration omission.
Native service substitutions and deliberate source corrections are recorded in
[DEVIATIONS.md](DEVIATIONS.md). The generator is a completeness/navigation gate;
matching a file, identifier or scenario name does not itself prove behavior.

## Final revision gates

The current results and remaining failures, if any, are recorded in
[VALIDATION.md](VALIDATION.md), not inferred from this checklist's scope:

- Execute the complete Controls suite, including every Repeater scenario and
  regression, without filtering out a known failure.
- Execute the affected Base, XamlX, runtime/compiled XAML, native View and
  metadata-scanner suites; verify generated compiler snapshots and builders.
- Build the catalog's compiled markup and execute its Repeater collection,
  lifetime, resizing and viewport assertions.
- Run the browser gate against a build of the current implementation, in
  WebGL2 and Software2D, with screenshots and reported realization counts;
  exercise both single-threaded and threaded hosts in CI.
- Regenerate the inventories after the last test/code changes and pass the
  source-convention, composition, Rust-path and thread-local consistency gates.

## Deliberate boundaries and further platform evidence

1. **Platform transports.** Native automation peers and keyboard traversal
   cover portable tree/order/focus contracts. Windows UI Automation clients,
   individual screen readers, native wheel transports and platform composition
   need their own host-level runs. A headless test does not establish those
   observations.
2. **Visual and performance equivalence.** Browser screenshots and bounded
   realization establish behavior for their recorded workloads. They do not
   measure every layout against a side-by-side pinned WinUI run, universal
   allocation bounds, or large-source throughput on every renderer. Keep any
   future benchmark's revision, target, data, viewport and measured output.
3. **Original sample applications.** All 302 upstream files remain in the
   inventory, including TestUI pages, packaging and assets. FerroUI's native
   catalog and scenario tests demonstrate the control family. The exact WinUI
   test applications, package activation and sample artwork are reference
   material, not separately reproduced applications.
4. **Source defects and platform diagnostics.** The port records corrections
   required by native ownership, callbacks and bindings instead of silently
   presenting them as pinned C++ behavior. ETW providers, debugger-specific
   formatting and Windows profiler registration are not portable APIs. Native
   boundary diagnostics do not claim a line-for-line copy of every trace.

Ignored upstream concerns remain visible in the scenario map. Where the pinned
runtime disagrees with an ignored desired-result assertion, the record names
which behavior is retained. No test count or inventory total waives these
boundaries.
