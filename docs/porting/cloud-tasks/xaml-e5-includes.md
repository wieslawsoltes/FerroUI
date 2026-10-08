# Task: XAML compiler stage E5, includes across crates

Branch: `xaml-e5-includes`. Pull request title: `XAML compiler E5, step 2: includes across crates`.

Read first: `docs/porting/xaml.md` (section 9, and 9.7.3, 9.10.1 and 9.12 in particular), `docs/porting/xaml-compiler/HANDOVER.md`, task 2 of `docs/porting/CONTINUATION.md`, and the "Continuation" section of pull request 43 (`gh pr view 43`), which names the upstream files of each remaining step. Row 21 of `CRITICAL-PATH.md` is the row this task advances.

Do, in this order, one commit per item:

1. `StyleInclude`, `ResourceInclude` and `MergeResourceInclude` resolve the compiled documents of another crate at build time, through that crate's `.xamlmeta` `documents[]` (`xaml.md` 9.7.3), as upstream resolves them through the loader table of the referenced assembly.
2. Port upstream's `XamlIncludeGroupTransformer` and `XamlMergeResourceGroupTransformer` exactly, with their diagnostics.
3. A two-crate test fixture (a theme library and an application that includes its documents) that runs upstream's `ResourceIncludeTests`, `StyleIncludeTests` and `MergeResourceIncludeTests` against compiled documents, under upstream's test names.
4. Regenerate the committed compiler output (the commands are in `CONTINUATION.md`, task 2; regenerate twice and diff before every push) and update the stage table of `xaml.md` 9.10.1, row 21 of `CRITICAL-PATH.md` and task 2 of `CONTINUATION.md`.

Do not start build integration (`compile_xaml()` in build scripts) or the compiled catalog: they are the next steps and get their own task. If time remains after item 4, measure `themed_view` and `control-catalog-browser` for the change of pull request 43 (item 4 of the remaining list in `CONTINUATION.md`) and add the table to section 20 of `browser-platform.md`.

End the pull request description with a "Continuation" section: what is done, what is left, and anything the next worker must know.
