# Native release benchmark at f2bb1d61

Source: [`f2bb1d618843bf777b76859d2e06bf361bef009b`](https://github.com/wieslawsoltes/FerroUI/commit/f2bb1d618843bf777b76859d2e06bf361bef009b). [GitHub run **39799180117**, successful native job **119894677519**](https://github.com/wieslawsoltes/FerroUI/actions/runs/39799180117/job/119894677519). Command: `cargo run --locked --release -p ferrodesign-core --example performance -- 10000`.

The release executable measured 10,000 flat filled rectangles with one selected node. Scoped and explicit full-document transaction reference operations ran in the same process, with alternating order for the paired nudge and gesture workloads. The reference is a workload in this executable, not a prior application version. Release mode, complete document/history equivalence, serialization round trip and component stable-ID/history checks passed.

## Scoped and full-document transaction measurements

All times are milliseconds. These are native document-engine and transaction costs; they exclude browser input/bridge work, native control layout, scene drawing, GPU execution, display frame rate and durable-storage completion.

| Operation | Samples per path | Reference p50 | Reference p95 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Nudge | 48 | 17.284794 | 18.183355 | 0.019659 | 0.023785 |
| Undo | 48 | 5.563399 | 6.265276 | 0.012579 | 0.016715 |
| Redo | 48 | 5.515978 | 6.208762 | 0.012298 | 0.015704 |
| Gesture begin | 12 | 4.617348 | 4.938897 | 0.012539 | 0.013300 |
| Each gesture update | 288 | 5.920511 | 6.255021 | 0.000751 | 0.001162 |
| Gesture commit | 12 | 14.630404 | 14.842241 | 0.007411 | 0.009635 |
| Complete 24-update gesture | 12 | 161.505031 | 166.606091 | 0.048162 | 0.054552 |

The gesture workload contains 12 gestures with 24 updates each. Begin, commit and total each have 12 observations; the update distribution has all 288 individually timed updates. Phase medians need not sum to the complete-gesture median. These measurements do not benchmark the new guide overlay, selection-matching query, bucket geometry or full browser frame latency.

## Serialization and layout

| Operation | Samples | p50 ms | p95 ms | Serialized bytes |
| --- | ---: | ---: | ---: | ---: |
| Portable pretty JSON | 5 | 29.115533 | 35.530442 | 20,152,206 |
| Compact autosave JSON | 5 | 13.962979 | 18.123064 | 12,802,055 |
| Parse / validate / index | 3 | 44.227866 | 44.586871 | — |
| Dense layout, 9,999 text children | 5 | 7.344085 | 11.483240 | — |

Compact JSON was **36.5% smaller**, preserving exact document round-trip equivalence. The separate dense-layout fixture has 9,999 mixed fixed/fill text children. These measurements include serialization or layout only where named and exclude IndexedDB commit latency.

## Structural component costs

Component editing retains **whole-document Editor history snapshots**. This separate 10,000-node fixture has two matching 32-child variants, text/visibility properties, vector paths and embedded images, including **5,592,566 bytes of unrelated embedded-image URI data**. Each operation kind has eight samples. The numbers below are actual Editor operations including history; the faster document-only operations in the raw JSON are not substituted for production Editor costs.

| Editor operation | Samples | p50 ms | p95 ms |
| --- | ---: | ---: | ---: |
| Variant switch | 8 | 22.029776 | 24.553801 |
| Text property | 8 | 22.909969 | 28.683793 |
| Visibility property | 8 | 27.464443 | 54.469802 |
| Component undo | 24 | 6.622801 | 12.610033 |
| Component redo | 24 | 6.721378 | 11.930899 |

All 24 component operations undo and redo exactly, preserving materialized instance IDs. The small sample counts and this single runner describe this observation only; timings from different CI runners are not evidence of an implementation speed change.

## Extraction provenance

- Full decoded timestamped log: **545,093 bytes**, SHA-256 `d45c000c41472622b2e0fa931edab61d866c69810279f8cd4099f52d99c2392a`.
- Raw benchmark step: log lines **826–1154**, **17,897 bytes**, SHA-256 `c2648edd66d447eb48b46df64170330007d5076c851e850a282ede3dceae81a6`.
- Exact schema-5 JSON payload after timestamp-prefix removal: log lines **841–1154**, **7,561 bytes**, SHA-256 `758f544841855f50008f394f57e7b905618bb2b2387ec125be856b17fb2afe61`.
- Files: [extracted JSON](performance-extracted-log.json), [timestamped step](performance-step.log), and [provenance](provenance.json).
- This is decoded-job-log extraction; byte identity with a separately downloaded benchmark artifact is not asserted.
