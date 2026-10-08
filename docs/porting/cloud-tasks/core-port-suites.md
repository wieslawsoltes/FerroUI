# Task: the remaining upstream suites of the core port

Branch: `core-port-12`. Pull request title: `Core port: Color, focus and glyph typeface suites`.

Task 3 of `docs/porting/CONTINUATION.md` lists the upstream test suites that the port still lacks by name, with what is known about each. Each suite is ported exactly, under upstream's test names, with framework fixes where a test exposes a difference from upstream. One commit per suite, in this order:

1. `Media/ColorTests.cs` (18 of 66 tests missing): an exact `media/color_tests.rs`; remove the inline duplicates in `media/color.rs`, `hsl_color.rs` and `hsv_color.rs`; fix any Color, HSL or HSV divergence the tests expose.
2. `Input/InputElement_Focus.cs` (21 of 46 missing). Expect gaps in the focus manager; port what the tests need from upstream exactly.
3. `Media/GlyphTypefaceTests.cs` (26 of 42 missing). Add the font-table test assets it needs, with their licences in the `NOTICE.md` of the crate.
4. The clean-ups listed under "Also" in task 3: remove `deferred_text_tests.rs` and `presenters/content_presenter_text_tests.rs` where they duplicate tests that pull request 40 ported exactly; close the template gaps against `TypeUtilities.CanCast<T>` recorded in `DEVIATIONS.md`; make `LayoutManager` call the dispatcher's access check as upstream does and move its deviation row to "Corrected divergences".
5. `Data/Core/Parsers/BindingExpressionVisitorTests.cs` (36 tests): do not port it. Write the design note that task 3 asks for (what in the compiled-binding path parser of the port stands for upstream's expression trees, and which of the 36 tests have a counterpart) as a section of `docs/porting/xaml.md` or of the binding design page it belongs to, and list the decision under "Decisions waiting for the owner" in `CONTINUATION.md`.

Then recount the upstream tests missing by name for `Avalonia.Base.UnitTests` and `Avalonia.Controls.UnitTests` and put the next batches into task 3 of `CONTINUATION.md`. Other workers are active in `src/Skia` (branch `skia-test-suites`), in a new colour picker crate (branch `color-picker`) and in the markup compiler (branch `xaml-e5-includes`): stay out of those.

End the pull request description with a "Continuation" section.
