# Verified FerroUI 0.4.0 browser build

[Browser build job 120403095053](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194298/job/120403095053)
successfully built the real WebAssembly editor at exact integration source
`1c7151f57614915796e4aa54f9b497f1ac842aad`, tree
`aef243ffe3c0c035b9e1686079b19d635f78a626`. The build step ran from
12:03:37 to 12:31:54 UTC on 2026-10-23, taking 28 minutes 17 seconds.

The complete uploaded artifact is
`FerroDesign-browser-build-1c7151f57614915796e4aa54f9b497f1ac842aad`,
artifact ID `12502190198`. Its 12,636,328-byte ZIP has SHA-256
`74601c652bce2108f7bd2396154d59d43040d1b25ef4a01edbc521cd186ee545`.
The extracted `ferrodesign_app.wasm` is 53,785,479 bytes with SHA-256
`0f3dfcbe958a619519abdce19ad2bd0378c30778cfccdf660f002c34b4314543`.

Verification checked the GitHub archive digest and byte length, ZIP CRCs and
safe entry paths, exact source/tree/workflow identity, pinned Rust 1.99.0,
Emscripten 6.0.10 and wasm-bindgen 0.2.129, all 16 payload file hashes and
sizes, the complete file set including the provenance manifest, and the WASM
header. The active application dependency graph contains FerroUI Vello and
contains no Skia renderer dependency.

The host's relative CSS and JavaScript references resolve inside the delivered
directory. A read-only module parse followed all static imports from the
application and host, including generated inline snippets. It also checked
`storage.js` explicitly because the host loads that module dynamically. Seven
modules were parsed, and the generated application module references the actual
delivered WASM filename. Framework source `src/FerroUI.Fonts.Inter/assets.rs`
embeds six Inter TTF files with `include_bytes!`; the app uses `with_inter_font()`
and the `fonts:Inter#Inter` default. Those assets require no external font
directory. This source observation does not itself establish runtime glyph
rendering quality.

This is build and delivery evidence. No editor interaction or performance run
was made against this `1c` runtime. The later `c549f6bc` build includes native
hover/pressed diagnostics and the relative favicon; its runtime validation is
recorded separately.

## Records

- [summary.json](summary.json): completed gate, timings, cache observations,
  artifact identity and precise verification scope.
- [build-provenance.json](build-provenance.json): embedded source/toolchain and
  complete payload hash manifest, plus local archive verification.
- [artifact-metadata.json](artifact-metadata.json): GitHub artifact ID, source,
  digest, size and retention metadata.
- [delivery-verification.json](delivery-verification.json): local resource and
  JavaScript dependency closure, including dynamic storage.
- [browser-build.log.gz](browser-build.log.gz): complete decoded GitHub build
  log, losslessly compressed and compared byte-for-byte after decompression.
