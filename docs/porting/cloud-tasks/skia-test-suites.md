# Task: the remaining suites of the Skia backend

Branch: `skia-test-suites`. Pull request title: `Skia: the remaining upstream test suites and the typeface hook`.

Row 16 of `CRITICAL-PATH.md` lists what the Skia backend still lacks: the remaining text test suites and the typeface downcast hook. Upstream's tests are `tests/Avalonia.Skia.UnitTests`; the port's are `src/Skia/FerroUI.Skia/tests.rs`, `text_tests.rs` and the modules they include. Everything here runs on Linux with the raster path.

Do:

1. An inventory first, committed as a table in the pull request description: every test of `tests/Avalonia.Skia.UnitTests` (the root files, `Media/` and `Media/TextFormatting/`) by upstream name, and whether the port has it.
2. Port the missing suites exactly, under upstream's names, one commit per upstream file, with the font files they need under `test_assets` and their licences in the crate's `NOTICE.md`: the font collection suites (`CustomFontCollectionTests`, `EmbeddedFontCollectionTests`, `FontCollectionTests`, `FontCollectionTryMatchCharacterTests`, `FontCollectionDeterminismTests`), `FontManagerTests`, `GlyphRunTests`, `GlyphTypefaceShapingTests`, `BitmapSaveTests`, `ImmutableBitmapTests`, the `TextFormatting` suites, and whatever of the root files is missing. Tests that upstream runs only on Windows keep upstream's condition.
3. Fix what the tests expose, in the backend or in the text code of `ferroui-base`, as upstream behaves. A fix outside `src/Skia` is its own commit and is named in the pull request.
4. The typeface downcast hook that row 16 names: find the seam comment, port the upstream member it stands for, and remove the seam.
5. Update row 16 of `CRITICAL-PATH.md` and `docs/porting/tracking/Avalonia.Skia.md`.

Never change the Skia feature set of a target (see the standing brief). Another worker is porting `Media/GlyphTypefaceTests.cs` of the base tests on the branch `core-port-12`: do not port that file here.

End the pull request description with a "Continuation" section.
