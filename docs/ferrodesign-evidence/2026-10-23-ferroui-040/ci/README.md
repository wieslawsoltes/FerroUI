# Completed upstream-integration CI evidence

These records validate the FerroUI 0.4.0 integration at
`1c7151f57614915796e4aa54f9b497f1ac842aad` and the identical production source at
`2d6284d98550eb1029542ced06d2be346cdb8d27`. The latter changes only the native
workflow's YAML encoding. They precede the later `c549f6bc` browser-readiness
diagnostics and harness corrections; browser execution has separate evidence.

| Gate | Source / job | Completed result |
| --- | --- | --- |
| Core engine | `1c7151f5`, [job 120402492690](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194410/job/120402492690) | 533 passed, zero failed or ignored; 2.41 seconds of test execution; zero doc-tests |
| Generated references | `1c7151f5`, [job 120402493015](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950194402/job/120402493015) | All Rust-path, Fluent and Simple generators and drift checks passed. All five generated file hashes match committed source; the generated patch is empty. |
| Native framework and editor | `2d6284d9`, [job 120403288368](https://github.com/wieslawsoltes/FerroUI/actions/runs/39950436412/job/120403288368) | All filtered framework suites, seven mandatory GPU fragment cases and 239 application tests passed. |

## Native test detail

| Suite | Passed | Test execution time |
| --- | ---: | ---: |
| Vello unhinted glyph geometry and pixels | 8 | 0.21 s |
| Base paragraph layout | 17 | 0.02 s |
| Base text line bounds | 8 | 0.01 s |
| Controls paragraph editing | 16 | 0.13 s |
| Controls TextBox | 174 | 1.83 s |
| Controls text history | 5 | 0.05 s |
| Vello GPU fragment capture and replay | 7 | 7.14 s |
| FerroDesign application | 239 | 26.03 s |

These timings exclude compilation. Every listed invocation has zero failures and
zero ignored tests. The two Base filters share one case, so their counts must
not be summed as unique tests. The paragraph group includes the new
`paragraph_styles_inherit_trimmed_line_bounds_across_wraps_breaks_and_empty_paragraphs`
regression.

The GPU invocation enables the `gpu` feature and sets
`FERROUI_VELLO_REQUIRE_GPU=1`. Its helper fails when a suitable adapter is absent
or lacks compute support. All seven cases executed successfully; these are
adapter-backed fragment pixel tests. They do not establish browser surface
behavior or physical-GPU performance. The complete test names and required
environment are recorded in [completed-gates.json](completed-gates.json).

## Source and raw records

[completed-gates.json](completed-gates.json) contains exact source trees, run and
job IDs, every test-result line, and SHA-256 digests and byte lengths for the
losslessly compressed decoded GitHub logs:

- [Core log](1c-core.log.gz).
- [Generated-reference log](1c-generated-references.log.gz).
- [Native feedback log](2d-native-feedback.log.gz).

The logs preserve the UTF-8 bytes returned by the decoded-log API, including its
initial byte-order mark. Gzip timestamps are fixed to zero. All three archives
were decompressed and compared byte-for-byte before publication.

[Generated-reference provenance](1c-generated-references-provenance.json)
records all five generated and source hashes, pinned Rust/Cargo versions, the
root lockfile hash and individual generator/check outcomes. Writer tests are
deliberately ignored in the subsequent check-only invocations. This framework
generator dev-test graph includes Simple theme's Skia development dependency;
the application's active Vello rendering graph is verified separately in its
browser build.

The first native workflow at `1c7151f5` failed before creating any jobs because
an inline YAML command ended a filter token with a colon followed by a space.
The exact local parser diagnosis and GitHub zero-job failure status are retained
in [1c-native-workflow-parse-failure.json](1c-native-workflow-parse-failure.json).
The corrected block scalar at `2d6284d9` then ran the successful native suites
above. No test success is attributed to the failed workflow.
