# Complete core suite at `7f813672`

Exact source [`7f81367248e26dfb2d07cfa73619646fa4de6ad1`](https://github.com/wieslawsoltes/FerroUI/commit/7f81367248e26dfb2d07cfa73619646fa4de6ad1) passed the complete document/editor core suite: **511 tests passed, 0 failed, 0 ignored, 0 measured and 0 filtered out, in 1.65 seconds**. The subsequent documentation-test run contained **zero tests**. These durations describe the test executions, not editor latency or rendering performance.

[Run 39864431287](https://github.com/wieslawsoltes/FerroUI/actions/runs/39864431287), [job 120122942802](https://github.com/wieslawsoltes/FerroUI/actions/runs/39864431287/job/120122942802) has terminal status **completed / success** in the preserved [jobs API response](jobs-api.json). The [decoded log](core-job.log) independently contains 511 distinct successful test lines and the complete passing result. [summary.json](summary.json) records machine-readable counts, exact new test names, extraction lines and file hashes.

## Exact-source proof

- Checkout requests the full commit at line 35; the post-checkout `git log -1 --format=%H` prints that same commit at line **125**.
- The pinned toolchain is Rust **1.99.0**, with the installed compiler printed at line **137**.
- Line **140** runs `cargo +1.99.0 test --locked -p ferrodesign-core --jobs 1` without a test-name filter.
- The complete core result appears at line **702**, timestamp **2026-10-22T22:23:09.0435976Z**. The zero-test documentation result appears at line **708**.

Line numbers count the original decoded log's lines from 1, including its first line and retained UTF-8 BOM. ANSI color sequences were ignored only while extracting results; the saved log was not normalized or rewritten.

## New gradient and padding regressions

All **five** `gradient_edit::tests` cases passed at lines **262–265 and 267**. They cover supported box geometry and affine projection, stop ordering/ties and limits, bounded endpoint translation, editing one fill with metadata detachment only on change, and atomic invalid/locked edits with preserved redo.

All **eight** `layout_edit::padding_tests` cases passed at lines **310–317**. They cover independent fractional padding with exact history/serialization, explicit grid-track space, nested Hug reflow and sibling positions, empty Hug frames, invalid/overflow rejection with real redo, equal-value no-ops on unsettled geometry, inherited locks and transaction ownership, and component-instance layout overrides.

These 13 cases are included in the 511 total; they are not additional test counts. The evidence validates the core setters and their geometry/history behavior at this source. It does **not** establish a native application, WASM, browser or Figma visual-parity pass, and does not validate later application controls or pointer gestures.

## Preserved bytes

| File | UTF-8 bytes | SHA-256 |
| --- | ---: | --- |
| [core-job.log](core-job.log) | 85,424 | `7f16bd6612007a39b33d81510bb59c5cb21d2b376a7cbb662f4ba7e148b70f58` |
| [jobs-api.json](jobs-api.json) | 1,216 | `bbd079570f11980eb2cacc8ab7803d9eb556bd9de975d18923725626769b16c1` |

The log retains its original decoded BOM, timestamps, ANSI sequences and final newline. The API file is the saved structured response. These hashes identify the exact saved files, not the original network transport or an official artifact archive. No artifact ZIP, executable, browser report, screenshot or archive CRC was retrieved or verified for this record.
