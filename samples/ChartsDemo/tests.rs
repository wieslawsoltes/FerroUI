use super::*;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::Size;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, Window};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

#[test]
fn compiled_document_activates_every_chart_and_shares_live_sources() {
    let _text = TextTestScope::new();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let demo = ChartsDemo::new();
    assert_eq!(
        demo.charts().each_ref().map(|chart| chart.series().count()),
        [2, 2, 2, 1]
    );
    let first = demo.charts()[0].series().get(0).unwrap();
    let area = demo.charts()[1].series().get(0).unwrap();
    assert_eq!(first.x_values(), area.x_values());
    assert_eq!(first.y_values(), area.y_values());
    assert_eq!(demo.sample_count(), 6);
    demo.add_sample();
    assert_eq!(demo.sample_count(), 7);
    assert_eq!(area.y_values().unwrap().count(), 7);
    let source = first.y_values().unwrap();
    let original = source.items_source();
    demo.replace_sources();
    assert_eq!(demo.sample_count(), 6);
    assert_eq!(first.y_values(), Some(source.clone()));
    assert_ne!(source.items_source(), original);
    for chart in demo.charts() {
        let layout = chart.layout_snapshot(Size::new(460., 280.));
        assert!(!layout.points.is_empty());
        assert!(layout.plot_area.width > 0. && layout.plot_area.height > 0.);
    }
}

#[test]
fn commands_change_native_theme_legend_and_orientation() {
    let _text = TextTestScope::new();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let demo = ChartsDemo::new();
    demo.toggle_theme();
    assert_eq!(
        demo.find::<ThemeVariantScope>("ThemeScope")
            .requested_theme_variant(),
        Some(ThemeVariant::dark())
    );
    demo.toggle_legend();
    assert!(demo.charts().iter().all(|chart| !chart.show_legend()));
    demo.toggle_bars();
    for series in demo.charts()[2].series().snapshot().iter() {
        assert_eq!(
            series.cast::<BarSeries>().unwrap().orientation(),
            BarOrientation::Horizontal
        );
    }
    let window = Window::new();
    window.set_width(1120.0);
    window.set_height(880.0);
    window.set_content(Some(Control::boxed(demo.view())));
    window.show();
    window.update_layout();
    assert!(demo.charts().iter().all(|chart| chart.bounds().width > 0.));
    window.close();
}

#[test]
fn runtime_loader_accepts_the_same_document_as_the_build_compiler() {
    let _text = TextTestScope::new();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let view: Ref<UserControl> =
        FerroRuntimeXamlLoader::parse_as(include_str!("MainView.xaml"), Some(&ASSEMBLY))
            .expect("same sample document should load through the runtime loader");
    let names = NameScope::get_name_scope(&view).unwrap();
    assert_eq!(names.get_as::<Chart>("LineChart").series().count(), 2);
    assert_eq!(names.get_as::<Chart>("AreaChart").series().count(), 2);
    assert_eq!(names.get_as::<Chart>("BarChart").series().count(), 2);
    assert_eq!(names.get_as::<Chart>("DateChart").axes().count(), 2);
}
