# Current c549 browser build and delivery verification

[Build job 120415321864](https://github.com/wieslawsoltes/FerroUI/actions/runs/39952473917/job/120415321864)
successfully built exact source `c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`, tree
`8c5bbcc8ed5e5a61d287708cf4e2c0e7c912259d`. The actual WASM build step ran from
12:34:06 to 12:44:16 UTC on 2026-10-23: 10 minutes 10 seconds. The content-verified
cache restored 10,459 matching inputs and skipped nine changed files; Cargo
rebuilt the application.

Artifact `12502207600` is
`FerroDesign-browser-build-c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`.
Its 12,636,471-byte ZIP has SHA-256
`551d16171234356d004490971b6f2909c938aa04efd89aa2c227ec4add6849ab`.
The 53,785,830-byte WASM payload has SHA-256
`64bd8f8b2dd9f3bc951776c9db7e3cb4bfc973ad754230219606d514bebc4115`.

The verifier checked the GitHub archive digest and size, ZIP CRCs and safe entry
paths, exact source/tree/run identity, pinned Rust 1.99.0, Emscripten 6.0.10 and
wasm-bindgen 0.2.129, every declared payload hash and size, and the complete file
set. This artifact contains 17 payload files plus its provenance manifest; the
count was derived from that verified manifest. The active application graph
contains FerroUI Vello and no Skia renderer dependency.

All five copied web source files, the application Cargo.lock and the loader
were also compared directly to exact `c549` Git objects. This includes
`index.html`'s relative `./favicon.svg` link and the SVG bytes themselves.
Relative CSS, main module, document store, loader and WASM references resolve
inside the artifact. A read-only parse follows the complete static import
closure, including generated snippets, and explicitly checks dynamically loaded
`storage.js`; seven JavaScript modules were parsed. No source file or assertion
was modified during verification.

## Records and scope

- [summary.json](summary.json): exact gate, archive/WASM hashes, source-copy
  comparisons and cache observations.
- [build-provenance.json](build-provenance.json): complete source, toolchain and
  payload manifest with archive verification.
- [artifact-metadata.json](artifact-metadata.json): GitHub artifact source,
  digest, byte length and retention metadata.
- [delivery-verification.json](delivery-verification.json): relative assets,
  static imports and explicit dynamic storage dependency.
- [browser-build.log.gz](browser-build.log.gz): complete decoded GitHub job log,
  compressed without loss and verified by decompression.

This folder records the build and delivery checks. The strict traced text-pan
benchmark and selected actual browser interactions have separate result folders;
successful compilation alone does not establish their outcomes.
