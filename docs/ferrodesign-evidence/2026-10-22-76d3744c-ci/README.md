# Complete predecessor CI checkpoint: 76d3744c

Source: [76d3744cca13ab1580d0dc4478d403fff8b3239c](https://github.com/wieslawsoltes/FerroUI/commit/76d3744cca13ab1580d0dc4478d403fff8b3239c).
Run: [39825808707](https://github.com/wieslawsoltes/FerroUI/actions/runs/39825808707).

Both the native and browser jobs completed successfully. The browser job built the actual Vello WebAssembly application and completed the full interaction/pixel suite in both modes: **141 Software2D checks and 138 WebGL2 checks**, all **28 feature scenarios per mode**. The printed browser summary has `passed: true` and `scope.full: true`. The native job passed **394 core tests**, **155 application tests**, and the separate framework/host gates recorded in its log.

The browser job also passed the real Chromium persistence checks. Its text-pan benchmark reports **14.49 ms median / 18.53 ms p95** for Software2D and **20.18 ms median / 1412.95 ms p95** for WebGL2, each from 36 native pan observations. WebGL2 uses the CI software GPU stack; this is not physical-GPU timing. The high WebGL2 tail does not satisfy a general fast-rendering claim. Later glyph-cache and benchmark phase-separation changes are not covered by this older measurement.

Two existing guarded compositor capture retries occurred for native local styles and native grid style. The full suite subsequently passed; this is not an all-first-attempt capture claim.

## Evidence

- [Browser job log](browser-job.log): ordinary UTF-8 job-log text returned by the GitHub connector.
- [Native job log](native-job.log): ordinary UTF-8 job-log text returned by the GitHub connector.
- [Printed browser summary](browser-summary.json): parsed by removing the timestamp prefixes from the logged JSON, with no assertion/result edits.
- [Job and step statuses](jobs-api.json).
- [Official artifact metadata](artifacts-api.json): published identifiers, sizes, digests and expiry.

The official browser build artifact is **12453671277**, the complete validation artifact is **12453004286**, and the native validation artifact is **12447058008**. Their ZIP bytes were not retrieved, so local artifact CRC checks, raw report/capture inspection and runtime-byte comparisons are not claimed. Decoded logs and API status establish this exact source's successful run.

This checkpoint predates the current continuous corners, variable-width strokes, paragraph/native text history, polygon fillets, guide bindings, planarization and text-preview integration. Its pass must not be used as a latest-head or full Figma parity claim.

