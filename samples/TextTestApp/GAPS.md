# TextTestApp: gaps

What the port of the sample found in the framework. Every gap has a reproduction in
`tests/gaps.rs` or in the test of the shell named with it; a gap that is fixed keeps its test,
which then asserts what the framework does now.

```sh
cargo test -p text-test-app --lib
```

Status: 2 documents (`App.xaml`, `MainWindow.xaml`), converted without an edit, both compiled by
the build, both load with the run-time loader too and show their class.

## Gaps of the framework

| Gap | What the sample does | The upstream statement | The framework | Status |
|---|---|---|---|---|
| T001 | The font box lists the fonts of the system: its items are a property of the font manager, which the document reaches through a static member. | `MainWindow.xaml`: `<ComboBox Name="_font" ItemsSource="{Binding SystemFonts, Source={x:Static FontManager.Current}}" />` | `FontManager` was not declared for markup, and the compiler refused the document (`Unable to resolve type FontManager`). Fixed at the root: the font manager is a markup type with the static property `Current` and the properties `SystemFonts` and `DefaultFontFamily`, the font collection is a contract of markup (`IFontCollection`, a read-only list of font families upstream), both compare by reference so that a handle is an untyped value, and a binding delivers a font collection to an items source as its font families. Files: `src/FerroUI.Base/markup_types/plain.rs`, `src/FerroUI.Base/markup_types/contracts.rs`, `src/FerroUI.Base/media/font_manager.rs`, `src/FerroUI.Base/media/fonts/i_font_collection.rs`, `src/FerroUI.Base/rust_paths.rs` (regenerated), `src/FerroUI.Controls/markup_types/plain.rs`; unit test `the_font_manager_declares_its_current_instance_and_the_system_fonts` (`src/FerroUI.Base/markup_types/markup_types_tests.rs`). | Fixed: `gaps::t001_the_font_box_lists_the_fonts_of_the_system` |
| T002 | The entry point attaches a type converter to the font feature collection type at run time, so that the binding of the features box (text) to the `FontFeatures` property of the line converts. | `Program.cs`: `TypeDescriptor.AddAttributes(typeof(FontFeatureCollection), new TypeConverterAttribute(typeof(FontFeatureCollectionConverter)));` | No gap. Upstream the value conversion of a binding (`TypeUtilities.TryConvert`) converts text through the type converter of the target type only (`TypeDescriptor.GetConverter(toUnderl)`), and the collection type has none: the sample attaches one whose conversion is `FontFeatureCollection.Parse`. Here the collection type states that conversion itself (`parse:` of its markup metadata), and the value conversions of a binding use it: the binding of the features box delivers what the converter of the sample converts the text to. The statement is left out of `program.rs` (there is no run-time registration of a converter for a type, and none is needed); the converter class is ported (`ferroui_markup_xaml::converters::TypeConverter`) and tested. | No gap: `gaps::t002_the_text_of_the_features_box_converts_as_the_converter_of_the_sample_converts_it` |
| T003 | The font family of the line is bound to the selected value of the font box, which is null until a font is chosen. A null is a value of the property upstream (a reference type), and a typeface of a null family is a typeface of the default family (`FontFamily = fontFamily ?? FontFamily.Default`). | `MainWindow.xaml`: `FontFamily="{Binding SelectedValue, ElementName=_font}"`; `InteractiveLineControl.cs`: `new Typeface(FontFamily, FontStyle, FontWeight, FontStretch)` | The property is `StyledProperty<FontFamily>` (the attached property of `TextElement`, with a new owner), whose value is never null: a design of the port, not local to a file. The default family stays, which is the line upstream formats, and the binding reports the null it cannot convert (`Could not convert '(null)' (null) to '...FontFamily'`), which the desktop host prints once at start. Upstream reports nothing. Should become: no report. | Open: `gaps::t003_the_null_of_the_font_box_is_reported_and_the_default_family_stays`; the report is held in `GAP_T003` of `tests/shell.rs`, apart from the accepted list (which is empty) |
| T004 | The list of the shaped buffers names each run of the line by the name of its class. | `MainWindow.xaml.cs`: `run.GetType().Name` (twice in `ListBuffers`) | A text run is a trait object and does not state the name of its class. `main_window.rs` names the runs of the framework (`ShapedTextRun`, `TextEndOfParagraph`, `TextEndOfLine`, `TextCharacters`, `UnshapedTextRun`) and any other run by its base class (`DrawableTextRun`, `TextRun`): a run of a class of an application would be named by its base class. | Open (a design of the port): `shell::the_rows_of_the_lists_show_the_metrics_of_the_line` asserts the name of the shaped run |
| T005 | The row that names a run is tagged with the run. | `MainWindow.xaml.cs`: `Tag = run` | A tag is an untyped value, which has equality, and `dyn TextRun` has none. The tag is `TextRunTag`, a wrapper that compares runs by identity (reference equality, as upstream). Nothing of the sample reads this tag. | Open (a design of the port): the same test asserts the tag |
| T006 | A debug build attaches the developer tools to the application. | `Program.cs`: `#if DEBUG .WithDeveloperTools() #endif` | The application builder has no developer tools extension. Left out of `program.rs`. | Open: nothing to reproduce |

## Differences that are not gaps

- **Culture of the numbers.** The distance of a character hit, the advance of a glyph and its
  offset and bounds are written with `ToString()` upstream, which uses the culture of the thread;
  the port writes them with the invariant culture (`Display`), as the framework does everywhere.
- **The text source and its control.** The text source of the line control holds the control
  strongly upstream (a cycle its collector frees); here it holds it weakly, and so do the handler
  of the font feature collection and the handler of the window for the line changed event.
- **The static brushes of the rows** (`TransparentAliceBlue`, `TransparentAntiqueWhite`) are
  per thread, as every object of the object model is.
- **`AffectsRender` in the constructor.** `SelectionAdorner` registers the properties that affect
  its rendering in its instance constructor upstream (once per instance); the port does the same
  in `constructed`.
- **The second call of the base class** in `InteractiveLineControl.OnPropertyChanged` is kept.
- **The pens of the extent and of the distances** are never reset when their strokes change
  (upstream has no case for `ExtentStroke` and `DistanceStroke`); kept.
- **`[STAThread]`**, the Windows application manifest and the project reference to the Inter
  font library (which no statement of the sample uses) have no counterpart in the crate.
- **The named elements.** The fields the upstream build generates for the named elements of the
  document (`_rendering`, `_buffer`, `_hits`, ...) are lookups in the name scope of the window.

## Read and confirmed by the tests

- The formatted line does not count its end of paragraph: `ListHits` reads one character of the
  text per position of the line, and the list has one row per character.
- `TextLayout::from_text_source`, `GlyphRun::new` from glyph infos with `build_geometry()`, and the
  dashed pen are what the upstream constructors are: the hit tests, the glyph bounds and the
  frames of `tests/shell.rs` rest on them.
