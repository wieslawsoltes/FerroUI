# Native feedback at `3ba7d1b7`: framework passes, application XAML build failure

The native feedback job for exact source
`3ba7d1b76c3a28d31f41e4bf7ef410af885d6d7f` completed with **failure**.
All five configured framework test invocations passed. The subsequent
FerroDesign application build failed because the compiled XAML could not resolve
the `Watermark` property on `TextBox`. The application test binary did not run.
The saved log therefore provides no application or browser pass for this source.

- Workflow: **FerroDesign native feedback**.
- Run: [39866066908](https://github.com/wieslawsoltes/FerroUI/actions/runs/39866066908).
- Job: [120130749670](https://github.com/wieslawsoltes/FerroUI/actions/runs/39866066908/job/120130749670), `native`, completed/failure.
- Terminal observation retained from the original capture: **2026-10-22 22:53:45 UTC**.
- The full checkout SHA is printed at log line **129**, following `git log -1 --format=%H` at line 128.
- The pinned `rustc 1.99.0 (b940084d7 2026-09-28)` installation is printed at line **196**.

[summary.json](summary.json) contains the derived results and extraction anchors.
The source evidence is [native-feedback-job.log](native-feedback-job.log) and
[jobs-api.json](jobs-api.json). The latter is the saved normalized terminal jobs
response; it confirms successful framework steps and a failed application step.
These original files have been left byte-for-byte unchanged.

## Executed framework tests

All invocations used the pinned toolchain, `--locked`, one Cargo job and one test
thread. Counts below are filtered-suite results. Some Controls filters overlap,
so the rows should not be added into a claim about distinct tests.

| Package and filter | Passed | Failed / ignored | Test execution time | Result log line |
| --- | ---: | --- | ---: | ---: |
| `ferroui-vello`, `unhinted_` | 8 | 0 / 0 | 0.38 s | 1411 |
| `ferroui-base`, `paragraph` | 15 | 0 / 0 | 0.03 s | 2309 |
| `ferroui-controls`, `paragraph` | 16 | 0 / 0 | 0.17 s | 2607 |
| `ferroui-controls`, `text_box_tests::` | 174 | 0 / 0 | 2.64 s | 3061 |
| `ferroui-controls`, `text_box_history::` | 5 | 0 / 0 | 0.06 s | 3346 |

The individual passing test names were independently counted in each suite:
lines **1402–1409**, **2293–2307**, **2590–2605**, **2886–3059** and
**3340–3344** respectively. Each count agrees with its printed test result.
The reported durations are the test harness's execution durations; they exclude
the preceding compilation and job setup.

## Exact application failure

The application command begins at log line **3348**:

```sh
cargo +1.99.0 test --locked -p ferrodesign-app --lib --jobs 1 -- --test-threads=1
```

The same compiled-XAML diagnostic appears at log lines **3741, 3807 and 3810**:

```text
ui/Editor.xaml(224,103): error FRN2000: Unable to resolve suitable regular or attached property Watermark on type FerroUI.Controls:FerroUI.Controls.TextBox Line 224, position 103.
```

Line **3742** reports the failed `ferrodesign-app` custom build command. Its build
script exits with status **1** at line **3745**, and the Cargo step exits with
code **101** at line **3811**. There is no application test launch or application
test result after the invocation. Dependencies and the build script were reached,
but the application did not complete compilation.

This evidence does not execute the full document-engine suite, a WebAssembly
build, browser tests, or the subsequently added source timestamp cache helper.

## Follow-up source correction and its validation boundary

The follow-up changes `PaddingField` to `PlaceholderText="Mixed"` and updates
the existing native assertion to call `placeholder_text()`. Read-only source
inspection confirms that this is the established public API:

- [TextBox's property definition](../../../src/FerroUI.Controls/text_box.rs)
  registers **`PlaceholderText`** at lines 1377–1378. `watermark_property()` at
  lines 1384–1387 forwards to that property as a deprecated Rust alias; it does
  not register a separate XAML property named `Watermark`. The public
  `placeholder_text()` getter is at lines 1875–1876, and deprecated `watermark()`
  forwards to it at lines 1885–1887.
- The checked-in [Simple theme](../../../src/FerroUI.Themes.Simple/compiled_xaml.rs)
  and [Fluent theme](../../../src/FerroUI.Themes.Fluent/compiled_xaml.rs) bind the
  TextBox `placeholder_text_property()` at lines 16542 and 55800 respectively.
- The corrected [Editor XAML](../../../samples/FerroDesign/crates/ferrodesign-app/ui/Editor.xaml)
  uses `PlaceholderText="Mixed"` at line 224. The corresponding
  [native padding assertion](../../../samples/FerroDesign/crates/ferrodesign-app/src/shell/layout_padding_tests.rs)
  uses `placeholder_text()` at line 181 and retains its expected value.

This explains the source correction. A new completed build is required to
establish its application and browser behavior; no such success is attributed
to this failed `3ba7d1b7` job.

## Byte integrity and provenance

| Preserved file | Bytes | SHA-256 |
| --- | ---: | --- |
| `native-feedback-job.log` | 593,668 | `4d394cea69a3c0c888e78be13b118b41d5b5ef673c5fb3258a8221a833a84cd3` |
| `jobs-api.json` | 2,198 | `54bade039e23b5159a5288c30722dd322d686343d1d7829c26d722f5b3169199` |

The saved decoded log is valid UTF-8 with a byte-order mark and contains **3,853
lines**. Line references use one-based numbering after strict UTF-8 BOM decoding,
without removing or reordering lines. The JSON summary was derived from these
saved text files and read-only source inspection. No binary artifact was
downloaded or claimed as verified for this evidence record.
