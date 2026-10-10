//! The minimal reproductions of the gaps of the framework the sample found (`GAPS.md`): a
//! test of a gap that is fixed asserts what the framework does now, a test of an open gap
//! asserts what it does today and says what it should become.

use super::shell::{rendering, reports, start, GAP_T003};
use crate::FontFeatureCollectionConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::media::{FontFamily, FontFeatureCollection, FontManager};
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_controls::ComboBox;
use ferroui_markup_xaml::converters::TypeConverter;
use std::rc::Rc;

/// T001 (fixed): markup names the current font manager and binds the items of the font box
/// to the fonts of the system
/// (`ItemsSource="{Binding SystemFonts, Source={x:Static FontManager.Current}}"`). The font
/// manager had no markup metadata, and the compiler refused the document.
#[test]
fn t001_the_font_box_lists_the_fonts_of_the_system() {
    let (shell, window) = start();
    let font = window.get_control::<ComboBox>("_font");

    let families = FontManager::current().system_fonts().font_families();
    assert!(!families.is_empty(), "the font manager of the test has fonts");
    assert_eq!(font.item_count() as usize, families.len(), "the font box has an item for every font of the system");
    let first = font.items().view().get_at(0).expect("the first item of the font box");
    let first: &dyn AnyValue = &*first;
    assert_eq!(first.downcast_ref::<FontFamily>(), Some(&families[0]), "the items of the font box are font families");

    // The selected value of the font box is the font family of the line.
    let rendering = rendering(&window);
    let last = families.len() - 1;
    font.set_selected_index(last as i32);
    shell.settle();
    assert_eq!(rendering.font_family(), families[last], "the chosen font is the font family of the line");
    assert!(rendering.text_line().is_some(), "the line is formatted with the chosen font");

    window.close();
    drop(shell);
}

/// T002 (no gap): the upstream sample attaches its type converter to the font feature
/// collection type at run time, because the value conversion of a binding converts through
/// the converter of the target type only. Here the collection type states its conversion
/// from text, which is the conversion of the converter: the binding of the features box
/// delivers what the converter of the sample converts the text to.
#[test]
fn t002_the_text_of_the_features_box_converts_as_the_converter_of_the_sample_converts_it() {
    let (shell, window) = start();
    let rendering = rendering(&window);

    let converter = FontFeatureCollectionConverter;
    assert!(converter.can_convert_from(None, ValueType::of::<String>()));
    let text: BoxedValue = Rc::new(String::from("calt clig kern liga"));
    let converted = converter.convert_from(None, None, Some(&text)).expect("the text converts").expect("a collection");
    let converted: &dyn AnyValue = &*converted;
    let converted = converted.downcast_ref::<FontFeatureCollection>().expect("a font feature collection");

    let bound = rendering.font_features().expect("the binding of the features box delivers a collection");
    assert_eq!(bound.to_vec(), converted.to_vec(), "the binding converts the text as the converter does");
    assert_eq!(bound.len(), 4);

    let number: BoxedValue = Rc::new(1_i32);
    assert!(converter.convert_from(None, None, Some(&number)).is_err(), "a value that is not text is an invalid cast");

    window.close();
    drop(shell);
}

/// T003 (open): the font family of the line is bound to the selected value of the font box,
/// which is null until a font is chosen. Upstream a null is a value of the property and the
/// typeface of a null family is the typeface of the default family, without a report. Here
/// the property is never null: the default family stays, and the binding reports the null.
/// What it should become: no report.
#[test]
fn t003_the_null_of_the_font_box_is_reported_and_the_default_family_stays() {
    let (shell, window) = start();
    let rendering = rendering(&window);

    assert!(window.get_control::<ComboBox>("_font").selected_value().is_none(), "no font is chosen");
    assert_eq!(rendering.font_family(), FontFamily::default_family(), "the line has the default font family");
    assert!(rendering.text_line().is_some(), "the line is formatted with the default font family");

    let found = reports(&shell, "MainWindow");
    assert_eq!(
        found,
        [(GAP_T003.0.to_string(), GAP_T003.1.to_string(), GAP_T003.2)],
        "today the binding reports the null (upstream reports nothing)"
    );

    window.close();
    drop(shell);
}
