# Exact dc361996 browser build and delivery

The independent browser build [39976024338](https://github.com/wieslawsoltes/FerroUI/actions/runs/39976024338), job `120490704316`, completed successfully on 23 October 2026. Its source is `dc36199693472ac8297bafdaad6d4a0701acddf0`, tree `57c2625a948b9401b9517b2724b2715d8e7ca92b`. Later documentation and CI changes do not change this artifact's source identity.

The build used pinned Rust/Cargo 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129. Cargo completed the optimized `browser` profile in **11 minutes 29 seconds**. The verified cache restored 10,573 unchanged source timestamps from the preceding `4d3c796c` build and skipped five changed inputs; the native Controls and downstream application were rebuilt. The job also passed host TypeScript checking and all ten host input tests.

Artifact `12512495659` contains **17 payloads plus its provenance manifest**. The downloaded ZIP has 12,643,773 bytes and SHA-256 `62525059621c4aa1aff6998db2a1bdcd02c5b906bfc956fa434fd350abad8537`. Its actual WebAssembly module has 53,795,035 bytes and SHA-256 `b531f97834ce74271b85e368b48a8428063fec301c7419abc53eeeb0e53a656a`.

Local verification checked the GitHub archive digest, CRC, safe unique paths, the complete manifest file set, every payload size and hash, and copied host files against exact Git source bytes. Extraction was bounded by the manifest and available disk space. The delivered module closure contains seven static modules; the relative favicon, CSS, main module, generated snippets, actual referenced WASM and dynamically loaded `storage.js` are present. The active dependency graph includes FerroUI Vello and excludes the Skia renderer and bindings.

This folder proves the build and delivery checks. Actual native test results are recorded separately in [native-dc361](../native-dc361/README.md); browser interaction and timing evidence are separate records and must not be inferred from successful compilation or static import checks.

## Preserved evidence

- [summary.json](summary.json) records source identity, completed gates, timings, cache behavior and raw log digest.
- [job.json](job.json) and [artifact-metadata.json](artifact-metadata.json) preserve GitHub job and artifact metadata.
- [artifact-verification.json](artifact-verification.json) contains the complete build manifest and archive/source verification results.
- [delivery-verification.json](delivery-verification.json) records the static import closure and explicit dynamic storage dependency.
- [build.log.gz](build.log.gz) is a lossless copy of the complete decoded job log, including warnings; its uncompressed SHA-256 is `3cc3429e2bf19e748d5ed26f5de970f7e935c503ae5391e94199bc84ad324d50`.
