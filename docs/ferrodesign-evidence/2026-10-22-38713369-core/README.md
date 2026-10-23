# Core geometry and history at `38713369`

Source: [`38713369933656306b833f97fc02ff2c9ef01202`](https://github.com/wieslawsoltes/FerroUI/commit/38713369933656306b833f97fc02ff2c9ef01202).

[Core run 39857152118](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857152118), job [120098216085](https://github.com/wieslawsoltes/FerroUI/actions/runs/39857152118/job/120098216085), completed with **success**. The complete document/editor library suite passed **498 tests, 0 failed, 0 ignored**, in **2.45 seconds**. The subsequent documentation-test command contained zero tests.

All five new corner-radius core regressions passed:

- Box handles share normalized independent radii while retaining authored values.
- Polygon and concave Star handle centers agree with actual circular-arc endpoints across geometry clamps.
- Handle preparation has bounded work and excludes nonfinite or degenerate geometry.
- Independent Rectangle edits detach the correct binding and retain exact atomic history.
- Batch editing validates every ID before mutation and skips unsupported or effectively locked nodes.

The decoded [core-job.log](core-job.log) identifies the checkout, pinned compiler and actual commands. This is a model/geometry/history pass, not execution of the new native canvas or browser interaction scenarios. The five test results do not establish Figma numerical or visual equality. Native application and browser validation require their own exact-source results.

The log is **83,633 UTF-8 bytes**, SHA-256 `c1b47504b70a752abbf45d650975f784c69758a6bff10c34b6f4291b37ad09d8`. The library result is at line **689**; the five new cases appear at lines **212, 215–216 and 219–220**. It is ordinary connector-decoded text saved locally with a final newline; this hash identifies the saved file, not the original transport bytes. No official artifact ZIP was downloaded; no raw runtime, screenshot or artifact-CRC claim follows from this evidence.
