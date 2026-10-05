# Task: ControlCatalog follow-ups

Branch: `catalog-follow-ups`. Pull request title: `ControlCatalog: load the documents unblocked by the latest markup fixes`.

The sample under `samples/ControlCatalog` is ported page by page from upstream's `samples/ControlCatalog`. Documents that cannot be loaded yet are listed in its `excluded.txt` with the gap that blocks them; `temporary.rs` holds workarounds; the gap tests are under `samples/ControlCatalog/tests/`. Recent markup fixes unblocked a batch:

1. Run the ignored document tests and take every document that now loads off `excluded.txt` (expected: the 22 Tabbed/Carousel page documents and `Pages/CalendarPage.xaml`); regenerate whatever lists are generated from it (`scripts/sync-control-catalog.sh`, its `--check` must pass).
2. Remove the two workarounds in `temporary.rs` that drop the native menu item icon (gap C003), and restore the corresponding elements, now that an icon can be set from text.
3. `Pages/TableViewPage.xaml` does not load because the view model declares `Countries` as an untyped items source. Declare the list with `ferroui_controls::ferro_markup_list!` as the cursor page does, typed as upstream types it, and register it; un-ignore the page.
4. Two repro tests are wrong and stay ignored for that reason: the C009 test sets `ShowWeekNumbers`, which exists neither upstream nor in the port (the real page only uses the week-number rule), and the C311 test lacks the `x:DataType` that compiled bindings need. Correct both so they test what the real documents do, and un-ignore them.
5. `sample_gallery_page_shows_a_card_per_sample_under_its_theme` still fails on a group-count assertion (gap C101). Find the cause. Fix it if it is in the sample; if it is in the framework, write the smallest failing test in the crate that owns the bug, fix it there as upstream behaves, and say so in the pull request.
6. Port the page classes that are still missing and whose controls exist: `ClipboardPage`, the pull and swipe gesture pages, `NavigationPage` `BackButtonPage` (unfinished handler). A page that needs something the framework lacks stays excluded with a precise gap entry in `GAPS.md` (create it under `samples/ControlCatalog/` from the entries still open in `excluded.txt`).

State in the pull request how many of the 219 documents load before and after. Suites that must be green: control-catalog, mini-mvvm, controls, the XAML tests, both themes, and the workspace check.
