//! Port of `Xaml/FerroIntrinsicsTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use std::time::Duration;

use ferroui_base::animation::SignedDuration;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{AcrylicBrush, Color, IBrush, TextDecorations, TextTrimming};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{CornerRadius, Matrix, Point, Ref, RelativePoint, RelativeUnit, Size, Thickness, Vector};
use ferroui_controls::{GridLength, GridUnitType, WindowTransparencyLevel};
use ferroui_markup_xaml::{RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration};
use xamlx::exceptions::XamlError;

use crate::support::app::xaml_test_base;
use crate::support::helpers::object_of;
use crate::support::loader::{describe, document_without_uri, parse, try_load_document, try_parse, uri, xaml_error};
use crate::support::xaml::ferro_intrinsics_tests::TestIntrinsicsControl;

#[test]
fn all_intrinsics_are_parsed_and_set() {
    let _base = xaml_test_base();
    let xaml = "<local:TestIntrinsicsControl 
            xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
            TimeSpanProperty='00:10:10'
            ThicknessProperty='1 1 1 1'
            PointProperty='15, 15'
            VectorProperty='16.6, 16.6'
            SizeProperty='20, 20'
            MatrixProperty='1 0 0 1 0 0'
            CornerRadiusProperty='4'
            ColorProperty='#44ff11'
            RelativePointProperty='50%, 50%'
            GridLengthProperty='10*'
            IBrushProperty='#44ff11'
            TextTrimmingProperty='CharacterEllipsis'
            TextDecorationCollectionProperty='Strikethrough'
            WindowTransparencyLevelProperty='AcrylicBlur'
            UriProperty='https://ferroui.net/'
            ThemeVariantProperty='Dark'
            PointsProperty='1, 1, 2, 2' />";

    let target = parse::<Ref<TestIntrinsicsControl>>(xaml);

    let color = Color::parse("#44ff11").expect("a color");
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(color));

    assert_eq!(SignedDuration::from_seconds(610.0), target.time_span_property());
    assert_eq!(Thickness::uniform(1.0), target.thickness_property());
    assert_eq!(Thickness::uniform(1.0), target.thickness_property());
    assert_eq!(Point::new(15.0, 15.0), target.point_property());
    assert_eq!(Vector::new(16.6, 16.6), target.vector_property());
    assert_eq!(Size::new(20.0, 20.0), target.size_property());
    assert_eq!(Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0), target.matrix_property());
    assert_eq!(CornerRadius::uniform(4.0), target.corner_radius_property());
    assert_eq!(color, target.color_property());
    assert_eq!(RelativePoint::new(0.5, 0.5, RelativeUnit::Relative), target.relative_point_property());
    assert_eq!(GridLength::new(10.0, GridUnitType::Star), target.grid_length_property());
    assert_eq!(Some(brush), target.i_brush_property());
    assert!(Some(<dyn TextTrimming>::character_ellipsis()) == target.text_trimming_property());
    assert!(Some(TextDecorations::strikethrough()) == target.text_decoration_collection_property());
    assert_eq!(WindowTransparencyLevel::acrylic_blur(), target.window_transparency_level_property());
    assert_eq!(Some(uri("https://ferroui.net/")), target.uri_property());
    assert_eq!(Some(ThemeVariant::dark()), target.theme_variant_property());
    assert_eq!(
        Some(vec![Point::new(1.0, 1.0), Point::new(2.0, 2.0)]),
        target.points_property().map(|points| points.to_vec())
    );
}

#[test]
fn all_intrinsics_report_errors_if_failed() {
    let _base = xaml_test_base();
    let xaml = "<local:TestIntrinsicsControl 
            xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'
            TimeSpanProperty='00:00:10,1'
            ThicknessProperty='1 1 1'
            PointProperty='15% 15%'
            VectorProperty='16.6. 16.6'
            SizeProperty='20%, 20%'
            MatrixProperty='1 0 1 0 0'
            CornerRadiusProperty='4 1 4'
            ColorProperty='#44ff1'
            RelativePointProperty='50, 50%'
            GridLengthProperty='10%'
            PointsProperty='1, 1, 2' />";
    // TODO: double check why we don't throw error on other supported types. Should it be warnings?

    let diagnostics: Rc<RefCell<Vec<RuntimeXamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    let collected = diagnostics.clone();
    configuration.diagnostic_handler = Some(Rc::new(move |diagnostic: &RuntimeXamlDiagnostic| {
        collected.borrow_mut().push(diagnostic.clone());
        diagnostic.severity
    }));
    match try_load_document(document_without_uri(xaml), Some(configuration)) {
        Ok(_) => panic!("Expected an AggregateException"),
        Err(error) => assert!(
            matches!(xaml_error(&error), Some(XamlError::Aggregate(_))),
            "Expected an AggregateException: {}",
            describe(&error)
        ),
    }

    let expected = [
        "time span",
        "thickness",
        "point",
        "vector",
        "size",
        "matrix",
        "corner radius",
        "color",
        "relative point",
        "grid length",
        "points list",
        // Compiler attempts to parse PointsList twice - as a list and as a point.
        "point",
    ];
    let diagnostics = diagnostics.borrow();
    assert_eq!(expected.len(), diagnostics.len(), "the diagnostics: {diagnostics:?}");
    for (diagnostic, contains) in diagnostics.iter().zip(expected) {
        assert_diagnostic(diagnostic, contains);
    }
}

#[track_caller]
fn assert_diagnostic(runtime_xaml_diagnostic: &RuntimeXamlDiagnostic, contains: &str) {
    assert_eq!(RuntimeXamlDiagnosticSeverity::Error, runtime_xaml_diagnostic.severity);
    assert!(
        runtime_xaml_diagnostic.title.to_lowercase().contains(&contains.to_lowercase()),
        "the title {:?} does not contain {contains:?}",
        runtime_xaml_diagnostic.title
    );
}

/// GitHub issue #15320 of the project this is ported from.
#[test]
fn should_parse_formatted_color_tag() {
    let _base = xaml_test_base();
    let _target = parse::<Ref<ResourceDictionary>>(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"
                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
    <Color x:Key="ColorKey">
        White
    </Color>
</ResourceDictionary>"#,
    );
}

// --- the two Rust forms of a time span ----------------------------------------------------
//
// Not from upstream (C5 of #423, docs/idiomatic-rust-signatures.md): a property that declares
// a `Duration` is a `System.TimeSpan` property, as one that declares a `SignedDuration`.
// `AcrylicBrush.TintTransitionDuration` is the one of the framework. A negative time span is
// rejected, never clamped (decision O2). The compiled forms of these tests are in
// `emitter::differential_tests`.

#[test]
fn a_time_span_literal_sets_a_duration_property() {
    let _base = xaml_test_base();
    let brush = parse::<Ref<AcrylicBrush>>("<AcrylicBrush xmlns='https://github.com/ferroui' TintTransitionDuration='0:0:1.5'/>");
    assert_eq!(brush.tint_transition_duration(), Duration::from_millis(1500));

    // The shorthand of seconds, and zero.
    let brush = parse::<Ref<AcrylicBrush>>("<AcrylicBrush xmlns='https://github.com/ferroui' TintTransitionDuration='0.25'/>");
    assert_eq!(brush.tint_transition_duration(), Duration::from_millis(250));
    let brush = parse::<Ref<AcrylicBrush>>("<AcrylicBrush xmlns='https://github.com/ferroui' TintTransitionDuration='0:0:0'/>");
    assert_eq!(brush.tint_transition_duration(), Duration::ZERO);
}

#[test]
fn a_negative_time_span_literal_for_a_duration_property_is_an_error() {
    let _base = xaml_test_base();
    let error = match try_parse::<Ref<AcrylicBrush>>("<AcrylicBrush xmlns='https://github.com/ferroui' TintTransitionDuration='-0:0:1'/>") {
        Ok(_) => panic!("a negative time span is not a duration"),
        Err(error) => error,
    };
    let described = describe(&error);
    assert!(described.contains("The TimeSpan -00:00:01 is negative and cannot be converted to a Duration."), "{described}");
}

#[test]
fn a_time_span_resource_sets_a_duration_property() {
    let _base = xaml_test_base();
    let resources = parse::<Ref<ResourceDictionary>>(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"
                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                    xmlns:sys="using:System">
    <sys:TimeSpan x:Key="Transition">0:0:2</sys:TimeSpan>
    <AcrylicBrush x:Key="Brush" TintTransitionDuration="{StaticResource Transition}"/>
</ResourceDictionary>"#,
    );
    let brush = object_of::<AcrylicBrush>(&resources.get(&"Brush".into()));
    assert_eq!(brush.tint_transition_duration(), Duration::from_secs(2));
}

/// The documents of the timer properties of the controls (step T3 of C5): the caret blink
/// interval of the text controls, the delays of the scroll bar, the minimum populate delay of
/// the auto-complete box. `{}` is the time span.
const CONTROL_TIMER_DOCUMENTS: &[(&str, &str)] = &[
    ("TextBox.CaretBlinkInterval", "<TextBox xmlns='https://github.com/ferroui' CaretBlinkInterval='{}'/>"),
    ("TextPresenter.CaretBlinkInterval", "<TextPresenter xmlns='https://github.com/ferroui' CaretBlinkInterval='{}'/>"),
    ("RichEditBox.CaretBlinkInterval", "<RichEditBox xmlns='https://github.com/ferroui' CaretBlinkInterval='{}'/>"),
    ("ScrollBar.HideDelay", "<ScrollBar xmlns='https://github.com/ferroui' HideDelay='{}'/>"),
    ("ScrollBar.ShowDelay", "<ScrollBar xmlns='https://github.com/ferroui' ShowDelay='{}'/>"),
    ("AutoCompleteBox.MinimumPopulateDelay", "<AutoCompleteBox xmlns='https://github.com/ferroui' MinimumPopulateDelay='{}'/>"),
];

/// Not from upstream (C5 of #423, step T3): the timer properties of the controls declare a
/// `Duration`; a time span literal sets them as the duration of the same value.
#[test]
fn a_time_span_literal_sets_the_timer_properties_of_the_controls() {
    let _base = xaml_test_base();
    let text_box = parse::<Ref<ferroui_controls::TextBox>>(&CONTROL_TIMER_DOCUMENTS[0].1.replace("{}", "0:0:0.25"));
    assert_eq!(text_box.caret_blink_interval(), Duration::from_millis(250));
    let presenter = parse::<Ref<ferroui_controls::presenters::TextPresenter>>(&CONTROL_TIMER_DOCUMENTS[1].1.replace("{}", "0.3"));
    assert_eq!(presenter.caret_blink_interval(), Duration::from_millis(300));
    let editor = parse::<Ref<ferroui_controls::RichEditBox>>(&CONTROL_TIMER_DOCUMENTS[2].1.replace("{}", "0:0:0"));
    assert_eq!(editor.caret_blink_interval(), Duration::ZERO);
    let scroll_bar = parse::<Ref<ferroui_controls::primitives::ScrollBar>>(&CONTROL_TIMER_DOCUMENTS[3].1.replace("{}", "0:0:1.5"));
    assert_eq!(scroll_bar.hide_delay(), Duration::from_millis(1500));
    let scroll_bar = parse::<Ref<ferroui_controls::primitives::ScrollBar>>(&CONTROL_TIMER_DOCUMENTS[4].1.replace("{}", "0:0:0.4"));
    assert_eq!(scroll_bar.show_delay(), Duration::from_millis(400));
    let auto_complete_box = parse::<Ref<ferroui_controls::AutoCompleteBox>>(&CONTROL_TIMER_DOCUMENTS[5].1.replace("{}", "00:00:01"));
    assert_eq!(auto_complete_box.minimum_populate_delay(), Duration::from_secs(1));
}

/// Not from upstream (C5 of #423, step T3, decision O2): a negative time span literal for a
/// timer property of the controls is a load error, as for any `Duration` property.
#[test]
fn a_negative_time_span_literal_for_a_timer_property_of_the_controls_is_an_error() {
    let _base = xaml_test_base();
    for (property, document) in CONTROL_TIMER_DOCUMENTS {
        let error = match crate::support::loader::try_parse_untyped(&document.replace("{}", "-0:0:1"), None) {
            Ok(_) => panic!("{property}: a negative time span is not a duration"),
            Err(error) => error,
        };
        let described = describe(&error);
        assert!(
            described.contains("The TimeSpan -00:00:01 is negative and cannot be converted to a Duration."),
            "{property}: {described}"
        );
    }
}

// --- the two Rust forms of a culture ------------------------------------------------------
//
// Not from upstream (C5 of #423, docs/idiomatic-rust-signatures.md section 3.4): a property
// that declares a `Locale` is a `System.Globalization.CultureInfo` property, as one that
// declares a `CultureInfo`. `RichTextBlock.Language` (the `TextElement.Language` it owns) is
// one of the framework. A tag whose culture is no locale is an error. The compiled forms of
// these tests are in `emitter::differential_tests`.

#[test]
fn a_language_literal_sets_a_locale_property() {
    use ferroui_base::utilities::{locale, Locale};
    use ferroui_controls::RichTextBlock;

    let _base = xaml_test_base();
    let block = parse::<Ref<RichTextBlock>>("<RichTextBlock xmlns='https://github.com/ferroui' Language='ja-JP'/>");
    assert_eq!(block.language(), locale!("ja-JP"));

    // The case of the subtags is normalized, and the empty tag is the invariant culture.
    let block = parse::<Ref<RichTextBlock>>("<RichTextBlock xmlns='https://github.com/ferroui' Language='zh-hant-tw'/>");
    assert_eq!(block.language(), locale!("zh-Hant-TW"));
    let block = parse::<Ref<RichTextBlock>>("<RichTextBlock xmlns='https://github.com/ferroui' Language=''/>");
    assert_eq!(block.language(), Locale::UNKNOWN);

    // A culture of the runtime library casts to its locale.
    let block = parse::<Ref<RichTextBlock>>(
        "<RichTextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' \
         xmlns:g='using:System.Globalization' Language='{x:Static g:CultureInfo.InvariantCulture}'/>",
    );
    assert_eq!(block.language(), Locale::UNKNOWN);
}

#[test]
fn an_invalid_language_tag_for_a_locale_property_is_an_error() {
    use ferroui_controls::RichTextBlock;

    let _base = xaml_test_base();
    let error = |xaml: &str| match try_parse::<Ref<RichTextBlock>>(xaml) {
        Ok(_) => panic!("the tag is not a locale: {xaml}"),
        Err(error) => describe(&error),
    };
    // An IETF tag whose culture is no locale: the error of the cast.
    let described = error("<RichTextBlock xmlns='https://github.com/ferroui' Language='x-private'/>");
    assert!(described.contains("The culture name 'x-private' is not a BCP-47 language tag"), "{described}");
    // A name that is no IETF tag: the error of the conversion of the culture.
    let described = error("<RichTextBlock xmlns='https://github.com/ferroui' Language='zh-CHT'/>");
    assert!(described.contains("Culture IETF Name zh-CHT is not a recognized IETF name."), "{described}");
}
