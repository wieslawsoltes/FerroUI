//! Port of `Xaml/FerroIntrinsicsTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::animation::TimeSpan;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush, TextDecorations, TextTrimming};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{CornerRadius, Matrix, Point, Ref, RelativePoint, RelativeUnit, Size, Thickness, Vector};
use ferroui_controls::{GridLength, GridUnitType, WindowTransparencyLevel};
use ferroui_markup_xaml::{RuntimeXamlDiagnostic, RuntimeXamlDiagnosticSeverity, RuntimeXamlLoaderConfiguration};
use xamlx::exceptions::XamlError;

use crate::support::app::xaml_test_base;
use crate::support::loader::{describe, document_without_uri, parse, try_load_document, uri, xaml_error};
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

    assert_eq!(TimeSpan::from_seconds(610.0), target.time_span_property());
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
