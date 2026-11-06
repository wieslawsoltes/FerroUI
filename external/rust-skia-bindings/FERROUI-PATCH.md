# Graphite Metal ownership hotfix

This directory preserves the published MIT-licensed `skia-bindings` 0.153.3
crate, including its authors, version and original VCS metadata. The original
crate SHA-256 is
`eb325787c91b3f6c34a7c6f2b8a2297b20292d733ca53a08c233e5028aec49af`.
Its Skia source revision is `m153-0.101.2`.

The reviewed source change is in
`patches/graphite-metal-ownership.patch`. Graphite's Metal backend uses manual
reference counting. Its pipeline creation passed retained shader functions,
a retained vertex descriptor and a retained color attachment to owning
properties without balancing the caller's references. The fix adopts those
four references with Skia's existing `sk_cfp` helper.

`build_support/graphite_metal_lifetime.rs` applies the two exact source hunks
idempotently and rejects an unknown/partially changed dependency. The macOS
Graphite/Metal build bypasses the affected published binary archive and builds
this source fix. Other target/backend selections retain upstream download and
build behavior. This patch does not enable ARC for Graphite: its other ownership
helpers require manual reference counting.

macOS source builds require Xcode command-line tools, Python and Ninja. Upstream
`SKIA_SOURCE_DIR` and `SKIA_NINJA_COMMAND` overrides remain available. Supplying
`SKIA_LIBRARY_SEARCH_PATH` deliberately imports a caller-managed archive; that
archive must include the same fix before claiming this profile's acceptance.

The local crate is excluded from the FerroUI workspace and marked non-publishable.
Before publishing independently consumed FerroUI crates, move this fix into an
upstream/published Skia dependency or distribute a reviewed patched dependency.
The root Cargo patch is a source-checkout fix and is not inherited automatically
by downstream crates.io consumers.

Measured failures, attribution and native reruns are recorded in
`docs/media/MACOS-VERIFICATION.md` in the repository. Retest the resize/reset
soak before removing this patch or changing the pinned Skia revision.
