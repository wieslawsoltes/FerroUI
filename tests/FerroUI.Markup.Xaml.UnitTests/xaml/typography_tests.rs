use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::loader::{parse, try_load};
use ferroui_base::media::{
    FontCapitals, FontEastAsianLanguage, FontEastAsianWidths, FontFraction, FontNumeralAlignment,
    FontNumeralStyle, FontVariants,
};
use ferroui_base::Ref;
use ferroui_controls::documents::{Run, Span, Typography, TypographyFormat};
use ferroui_controls::{Control, RichEditBox, RichTextBlock, TextBlock, UserControl, Window};

const FIXTURE: &str = include_str!("../emitter/fixtures/rich_typography.xaml");

#[test]
fn typography_markup_loads_all_43_properties_and_nested_explicit_defaults() {
    let _scope = xaml_test_base();
    let root = parse::<Ref<UserControl>>(FIXTURE);
    let view = root.get_control::<RichTextBlock>("fullTypography");
    let expected = TypographyFormat {
        annotation_alternates: Some(1),
        east_asian_expert_forms: Some(true),
        east_asian_language: Some(FontEastAsianLanguage::Jis90),
        east_asian_widths: Some(FontEastAsianWidths::Half),
        standard_ligatures: Some(false),
        contextual_ligatures: Some(false),
        discretionary_ligatures: Some(true),
        historical_ligatures: Some(true),
        standard_swashes: Some(2),
        contextual_swashes: Some(3),
        contextual_alternates: Some(false),
        stylistic_alternates: Some(4),
        stylistic_set1: Some(true),
        stylistic_set2: Some(true),
        stylistic_set3: Some(true),
        stylistic_set4: Some(true),
        stylistic_set5: Some(true),
        stylistic_set6: Some(true),
        stylistic_set7: Some(true),
        stylistic_set8: Some(true),
        stylistic_set9: Some(true),
        stylistic_set10: Some(true),
        stylistic_set11: Some(true),
        stylistic_set12: Some(true),
        stylistic_set13: Some(true),
        stylistic_set14: Some(true),
        stylistic_set15: Some(true),
        stylistic_set16: Some(true),
        stylistic_set17: Some(true),
        stylistic_set18: Some(true),
        stylistic_set19: Some(true),
        stylistic_set20: Some(true),
        capitals: Some(FontCapitals::AllSmallCaps),
        capital_spacing: Some(true),
        kerning: Some(false),
        case_sensitive_forms: Some(true),
        historical_forms: Some(true),
        fraction: Some(FontFraction::Slashed),
        numeral_style: Some(FontNumeralStyle::OldStyle),
        numeral_alignment: Some(FontNumeralAlignment::Tabular),
        slashed_zero: Some(true),
        mathematical_greek: Some(true),
        variants: Some(FontVariants::Superscript),
    };
    assert_eq!(TypographyFormat::from_object(&view), expected);
    assert_eq!(view.text(), "InheritedReset");
    let paragraph = view.document().paragraphs()[0].clone();
    let inherited = paragraph.inlines().get(0).cast::<Run>().unwrap();
    assert_eq!(TypographyFormat::effective(&inherited), expected);
    assert!(TypographyFormat::from_object(&inherited).is_empty());
    let span = paragraph.inlines().get(1).cast::<Span>().unwrap();
    let reset = span.inlines().get(0).cast::<Run>().unwrap();
    assert_eq!(Typography::get_capitals(&reset), FontCapitals::Normal);
    assert_eq!(Typography::get_standard_swashes(&reset), 0);
    assert!(Typography::get_kerning(&reset));
    assert!(Typography::get_slashed_zero(&reset));
    let editor = root.get_control::<RichEditBox>("editorTypography");
    assert_eq!(
        Typography::get_numeral_alignment(&editor.text_presenter()),
        FontNumeralAlignment::Tabular
    );
    assert_eq!(
        editor.document().snapshot().paragraphs[0].inlines[0]
            .format
            .typography
            .slashed_zero,
        Some(true)
    );
}

#[test]
fn typography_markup_style_inheritance_and_local_clear_use_normal_property_precedence() {
    let _scope = styled_window_application();
    let root = parse::<Ref<UserControl>>(FIXTURE);
    let styled = root.get_control::<TextBlock>("styledTypography");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root)));
    window.show();
    assert!(!Typography::get_kerning(&styled));
    assert_eq!(Typography::get_capitals(&styled), FontCapitals::SmallCaps);
    Typography::set_kerning(&styled, true);
    assert!(Typography::get_kerning(&styled));
    styled.clear_value(Typography::kerning_property());
    assert!(!Typography::get_kerning(&styled));
    window.close();
}

#[test]
fn typography_markup_rejects_unknown_enums_and_out_of_range_integer_parameters() {
    let _scope = xaml_test_base();
    for xaml in [
        r#"<TextBlock xmlns="https://github.com/ferroui" Typography.Capitals="Unknown" />"#,
        r#"<TextBlock xmlns="https://github.com/ferroui" Typography.StandardSwashes="2147483648" />"#,
        r#"<Run xmlns="https://github.com/ferroui" Typography.Kerning="sometimes" />"#,
    ] {
        assert!(try_load(xaml).is_err(), "{xaml}");
    }
    let text = parse::<Ref<TextBlock>>(
        r#"<TextBlock xmlns="https://github.com/ferroui" Typography.StandardSwashes="-2147483648" />"#,
    );
    assert_eq!(Typography::get_standard_swashes(&text), i32::MIN);
}
