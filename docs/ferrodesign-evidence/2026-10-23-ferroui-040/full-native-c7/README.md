# Completed full-workflow native job at c7

The **Native validation job passed** for
`c7dbf26f1e184fc1113f9c96fcdc1dd0e2627834`, whose production and browser harness
inputs are unchanged from `c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`.
Run `39955698269`, job `120423823557`, completed at 13:17:24 UTC on
2026-10-23 after starting at 12:52:08.

| Gate | Result |
| --- | --- |
| Editor core | 533 passed, 2.37 s; zero failed or ignored |
| Native editor/application | 239 passed, 43.13 s; zero failed or ignored |
| Native launcher and shared UI compilation | Passed |
| Controls preedit / rich ranges / layout invalidation | 15 / 2 / 1 passed |
| Public compositing recording / Vello pixels | 7 / 8 passed |
| Geometry path / shaped text / complete glyph / unhinted outlines | 2 / 3 / 2 / 8 passed |

The longer job duration includes a cold native build cache: no matching cache
or source-timestamp manifest was available. Application test compilation took
5m39s before the 43.13-second test execution. The job then recorded 10,501
content-verified source timestamps and saved a new native build cache. This is
distinct from the browser cache bridge; a successful native job does not prove
the running browser job has passed.

[summary.json](summary.json) retains individual filter counts, durations, exact
raw/compressed log hashes and original test-result lines.
[job.json](job.json) retains every job step and conclusion.
[native.log.gz](native.log.gz) preserves the complete decoded job log, verified
by decompression and byte comparison with its 656,156-byte original.

The browser interaction job was still running when this record was collected.
The source also precedes the newly diagnosed first-composition-clause ordering
correction. Passing the existing native preedit tests does not replace that
new regression or the failed [exact c549 browser replay](../browser-runtime-c549/README.md).
