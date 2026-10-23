# Browser build at 4d3c796c

The exact source WebAssembly build passed. Local archive integrity verification passed for all 17 payloads, but **no browser replay was run for this revision**. Native editing validation separately found three failing new lifecycle regressions; see the [native 4d3 evidence](../native-4d3/). Browser execution was held for a correction, and the downloaded archive was not extracted locally.

| Identity | Value |
| --- | --- |
| Source commit | `4d3c796c7d5b76eb1e9d7f4f24b523841adbd994` |
| Source tree | `f5870c7d41ef51600b068829a3c40838e701a8ef` |
| Workflow run / job | `39972889233` / `120480440853` |
| Artifact | `12510294431` |
| Compressed archive | 12,640,159 bytes |
| WebAssembly payload | 53,794,092 bytes |
| Payload count | 17, plus the provenance manifest |

The [build job](https://github.com/wieslawsoltes/FerroUI/actions/runs/39972889233/job/120480440853) ran its build step from 14:50:56 to 15:06:20 UTC on 2026-10-23. Cargo reported 15m 14s. The host TypeScript typecheck, all 10 host input tests, web host build and static import check of six modules passed. The active dependency graph contains FerroUI Vello and excludes the application's Skia dependencies. Tool versions remain Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129.

Local verification checked the GitHub archive digest and size, all ZIP CRCs, safe unique member paths, the exact source commit/tree/run in the manifest, every declared payload hash and size, and the absence of unlisted payloads. Copied host resources match their Git objects, including the relative favicon, loader and lockfile. The retained ZIP also contains `storage.js` and the generated module/snippet files; no local browser or additional static module traversal was executed for this held revision.

The build restored the preceding 0aab9d7c cache, restored timestamps for 10,512 content-identical inputs and skipped seven changed inputs. Its successful guarded snapshot recorded 10,578 inputs and was saved for the correction build.

[summary.json](summary.json) records the exact archive, WASM and decoded-log hashes. [artifact-verification.json](artifact-verification.json) retains the full 17-payload manifest and verification scope. [artifact-metadata.json](artifact-metadata.json) and [job.json](job.json) preserve GitHub's identities and step results. [build.log.gz](build.log.gz) is the complete decoded job log in a lossless gzip container; its uncompressed SHA-256 is `c0d357b26a8f08d4593c2d3cd3d63e9d1e39fc3f5299cb8c04fb995b936197b3`.
