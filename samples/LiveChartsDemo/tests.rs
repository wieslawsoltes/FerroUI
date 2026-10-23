use super::*;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

/// The build-model compiler and the runtime loader construct the same six
/// native chart families, collections, axes and drawable geometry.
#[test]
fn compiled_and_runtime_markup_produce_the_same_charts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let compiled = DemoView::new().expect("compiled view");
    let runtime: Ref<Grid> =
        FerroRuntimeXamlLoader::parse_as(include_str!("MainView.xaml"), Some(&ASSEMBLY))
            .expect("runtime view");
    let runtime = DemoView::from_root(runtime);
    compiled.layout(VIEW_SIZE);
    runtime.layout(VIEW_SIZE);
    assert_eq!(
        compiled.data_counts().unwrap(),
        runtime.data_counts().unwrap()
    );
    assert!(compiled
        .data_counts()
        .unwrap()
        .iter()
        .all(|(_, count)| *count > 0));
    for index in [0, 2] {
        assert_eq!(compiled.chart(index).x_axes().len(), 1);
        assert_eq!(compiled.chart(index).y_axes().len(), 1);
        assert_eq!(runtime.chart(index).x_axes().len(), 1);
        assert_eq!(runtime.chart(index).y_axes().len(), 1);
    }
    assert!(compiled
        .chart(1)
        .series()
        .to_vec()
        .iter()
        .all(|series| series.inner_radius() == 56.));
    for index in 0..6 {
        let a = compiled.chart(index);
        let b = runtime.chart(index);
        assert_eq!(a.chart_kind(), b.chart_kind(), "{}", CHART_NAMES[index]);
        let a = a.scene(a.bounds().size()).unwrap();
        let b = b.scene(b.bounds().size()).unwrap();
        assert_eq!(a.plot_bounds, b.plot_bounds);
        assert_eq!(a.hits, b.hits);
        let a_geometry: Vec<_> = a.primitives.iter().map(|p| &p.geometry).collect();
        let b_geometry: Vec<_> = b.primitives.iter().map(|p| &p.geometry).collect();
        assert_eq!(a_geometry, b_geometry, "{}", CHART_NAMES[index]);
    }
}

/// A routed click takes the same action as the renderer's deterministic update
/// and invalidates every family through the normal native observation path.
#[test]
fn live_button_updates_all_six_native_charts() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let view = DemoView::new().expect("compiled view");
    view.layout(VIEW_SIZE);
    let previous: Vec<_> = (0..6)
        .map(|index| {
            let chart = view.chart(index);
            (
                chart.revision(),
                chart.scene(chart.bounds().size()).unwrap(),
            )
        })
        .collect();
    view.root
        .get_control::<Button>("UpdateButton")
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(view.step(), 1);
    for (index, (revision, old_scene)) in previous.iter().enumerate() {
        let chart = view.chart(index);
        assert!(
            chart.revision() > *revision,
            "{} did not invalidate",
            CHART_NAMES[index]
        );
        let scene = chart.scene(chart.bounds().size()).unwrap();
        assert!(!Rc::ptr_eq(old_scene, &scene));
        let old_geometry: Vec<_> = old_scene.primitives.iter().map(|p| &p.geometry).collect();
        let geometry: Vec<_> = scene.primitives.iter().map(|p| &p.geometry).collect();
        assert_ne!(
            old_geometry, geometry,
            "{} did not change",
            CHART_NAMES[index]
        );
    }
    assert_eq!(view.chart(0).series().get(0).values().get(5).y(), 58.);
    assert_eq!(view.chart(3).sankey_links().get(0).weight(), 44.);
    assert_eq!(
        view.chart(4)
            .treemap_roots()
            .get(0)
            .children()
            .unwrap()
            .get(0)
            .value(),
        57.
    );
}

/// The gradient in the compiled document remains a mutable native brush, and a
/// stop edit propagates to the owning chart without replacing its series.
#[test]
fn compiled_native_gradient_remains_live() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let view = DemoView::new().expect("compiled view");
    let chart = view.chart(0);
    let brush = chart
        .series()
        .get(0)
        .fill_brush()
        .expect("native fill brush");
    let gradient = brush
        .as_object()
        .and_then(|object| object.downcast_ref::<ferroui_base::media::LinearGradientBrush>())
        .expect("the native LinearGradientBrush from the markup");
    assert_eq!(gradient.gradient_stops().len(), 2);
    assert_eq!(
        gradient.gradient_stops().get(0).color(),
        Color::from_rgb(72, 199, 200)
    );
    let revision = chart.revision();
    gradient
        .gradient_stops()
        .get(0)
        .set_color(Color::from_rgb(91, 158, 232));
    assert!(chart.revision() > revision);
}
