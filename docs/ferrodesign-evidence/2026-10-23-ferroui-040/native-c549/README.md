# Current c549 native validation

[Native feedback job 120410483091](https://github.com/wieslawsoltes/FerroUI/actions/runs/39952473854/job/120410483091)
passed at exact source `c549f6bcafe614a6373c743f5efbf2a3cb0eccd2`, tree
`8c5bbcc8ed5e5a61d287708cf4e2c0e7c912259d`. This includes the native hover/pressed
diagnostics added for reliable browser pointer entry after the FerroUI 0.4.0
integration.

| Suite | Passed | Test execution time |
| --- | ---: | ---: |
| Vello unhinted glyph geometry and pixels | 8 | 0.34 s |
| Base paragraph layout | 17 | 0.04 s |
| Base text line bounds | 8 | 0.02 s |
| Controls paragraph editing | 16 | 0.16 s |
| Controls TextBox | 174 | 2.57 s |
| Controls text history | 5 | 0.06 s |
| Vello GPU fragment capture and replay | 7 | 9.09 s |
| FerroDesign application | 239 | 43.60 s |

All eight invocations have zero failures and zero ignored tests. The Base
filters overlap and must not be summed as unique tests. Times exclude
compilation. The application compilation took 38.92 seconds; framework binaries
were reused through the content-verified cache.

The GPU suite enables `gpu` and sets `FERROUI_VELLO_REQUIRE_GPU=1`; an absent or
non-compute adapter fails the helper. The seven successful cases therefore
exercised actual adapter-backed fragment rendering. This does not establish
browser compositor behavior or hardware-GPU performance.

The native
`shell::layout::padding::tests::native_padding_diagnostics_publish_real_fields_and_export_controls`
case passed with the new hover/pressed boolean projections and the real
inspector controls. The core source tree is byte-identical to the earlier
533-test `1c7151f5` checkpoint; core tests were not rerun by this native job.

[provenance.json](provenance.json) records the exact source, complete workflow
step outcomes, all test-result counts, core tree equivalence and log digests.
[native-feedback.log.gz](native-feedback.log.gz) is the complete decoded GitHub
job log, compressed without loss with a zero gzip timestamp and verified by
decompression. Exact-source browser build and runtime validation are separate
gates.
