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

When it applies the fix, the build script also passes `graphite_metal_ownership=fixed` to the
crates that depend on the binding (`links = "skia"`): `ferroui-skia` reads it as
`DEP_SKIA_GRAPHITE_METAL_OWNERSHIP`, warns at build time and logs at run time when it is missing,
and answers `ferroui_skia::graphite_metal_ownership_fixed()` with it (#622). An application built
from the published crates takes the fix with a `[patch.crates-io]` of its own (`docs/apps.md`,
"The Graphite Metal ownership fix"). Builds for iOS take the published archive and do not get
the fix.

## How the published crates use it

`skia-bindings` is rust-skia's crate name, so this copy is never published to crates.io: it stays
excluded from the FerroUI workspace and `publish = false`. The published FerroUI crates depend on
`skia-bindings` and `skia-safe` 0.153.3 from crates.io, exactly as written in their manifests
(`ferroui-skia` requires `skia-bindings = "=0.153.3"`), and take the fix only where the
application's root manifest replaces the binding:

```toml
[patch.crates-io]
skia-bindings = { path = "third_party/skia-bindings" }
```

The binding for that patch is the one of the FerroUI release the application uses. Each release
publishes it as a file of its GitHub release, `skia-bindings-0.153.3-ferroui-<version>.tar.gz`, with
its SHA-256, made by `scripts/release/skia_bindings_archive.py` from this folder at the commit of the
release. The application unpacks it into `third_party/` (it unpacks as `skia-bindings/`); an
application with FerroUI as a submodule gets the patch from `sync-workspace.py` instead. The steps
are in `docs/apps.md`, "The Graphite Metal ownership fix". `skia_bindings_archive.py --check`, which
CI runs, keeps the version of this copy, of `ferroui-skia`'s requirement and of `Cargo.lock` equal.

Cargo applies a `[patch]` only in the workspace that declares it, so nothing a published crate
states can apply it for the application. The changes are offered upstream from `upstream/` so that
a released `skia-bindings` can take the place of this copy.

Measured failures, attribution and native reruns are recorded in
`docs/media/MACOS-VERIFICATION.md` in the repository. Retest the resize/reset
soak before removing this patch or changing the pinned Skia revision.
