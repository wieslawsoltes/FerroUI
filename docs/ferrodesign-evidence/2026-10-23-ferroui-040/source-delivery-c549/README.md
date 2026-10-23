# FerroDesign source-delivery audit

The tracked source and dependency checks found no missing build input for the
documented full-repository setup. The source checkpoint is
`c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`, tree
`8c5bbcc8ed5e5a61d287708cf4e2c0e7c912259d`.

The initial audit and offline graph command ran at the later documentation
checkpoint `3d7b8fc6da73485acbcfbe80e2abf6868cd5fb36`. Exact Git object equality
confirms that its application, framework, native sources, build scripts, browser
host, test harness, manifests, locks and Cargo configuration match `c549`.
[audit.json](audit.json) preserves those object IDs, required-input/font hashes,
the actual graph command output and scope. The whole pinned `c549` tree was
also checked for LFS pointer headers when this record was written.

| Check | Observed result |
| --- | --- |
| Conservative normal/build path closure | 30 manifests, 103 path edges; every referenced manifest tracked and inside the repository |
| Rust source and literal embedded-file references | 3,950 tracked Rust files; 70 references to 58 unique files; none missing or outside the repository |
| Embedded Inter fonts | Six tracked sfnt font files, embedded through `src/FerroUI.Fonts.Inter/assets.rs`; actual bytes, not LFS pointers |
| Pinned repository tree | 10,585 entries; 10,469 regular files checked for standard LFS pointer headers; zero pointers or missing regular files |
| Repository links | 116 symlinks resolve inside the repository; zero submodule gitlinks |
| Documented entry points | Executable setup/build/renderer-check scripts, `Editor.xaml`, browser-loader sources and Cargo/npm lockfiles present |

The path closure includes all target conditions and optional normal/build
dependencies; it does not traverse development-only edges. It is a source
delivery check, not a declaration that all optional renderers are activated.
Generated `OUT_DIR` includes are build outputs, not absent source assets.

## Active renderer graphs and public API

The actual command `CARGO_NET_OFFLINE=true ./scripts/check-renderer.sh`, run
from `samples/FerroDesign` with the existing pinned Rust 1.99.0 toolchain,
exited 0. All five locked normal/build graphs contain FerroUI Vello and exclude
`ferroui-skia`, `skia-safe` and `skia-bindings`: browser Emscripten, Linux x86_64,
Windows x86_64, macOS aarch64 and macOS x86_64. This command compiled nothing
and used no network access.

Source inspection confirms public `BrowserPlatformOptions`/`BrowserRenderer`
selection for WebGL2 and Software2D, and public `VelloApplicationExtensions`
with Hybrid/CPU modes on desktop. Artwork uses FerroUI `DrawingContext`,
geometry, brushes, text, compositing layers and `RenderTargetBitmap`. The color
emoji font fixture located under the framework's Skia test-assets directory
does not introduce a Skia rendering dependency.

## Reproduction and distribution boundaries

Keep the complete repository layout, including its root `.cargo/config.toml`:
the sample's path dependencies refer to framework sources outside the sample.
XAML and browser bundles are generated from tracked inputs. Inspected build
logic does not require FerroUI's `.git` directory. Setup still requires the
`git` executable to clone Emscripten and downloads the pinned Rust/Emscripten/
wasm-bindgen tools, Cargo crates and npm dependencies. The macOS native build
retains its fallback to `native/FerroUI.Native` if a local native-source link
was flattened during extraction.

The browser artifact must contain its relative host resources, including
`favicon.svg`, the framework and application JavaScript, every generated inline
snippet and the exact referenced WASM filename. Check `storage.js` explicitly:
FerroUI loads it dynamically, so static-import traversal alone cannot discover
it. The six Inter fonts are embedded in the native module; no external font
directory is needed. Source/tree/toolchain identity, the full file set and all
payload hashes remain separate distribution checks. Serve the complete build
over HTTP; source-build downloads are separate from runtime resource loading.

**No independently downloaded source ZIP was extracted or built for this
audit.** Source inspection and offline graph checks do not establish a clean
archive build, browser behavior, visual parity or performance. Actual native
and browser-build evidence is recorded in the adjacent checkpoint-specific
folders.
