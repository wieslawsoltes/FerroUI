//! Regression for the native WinUI scroll surface, using the compiled dependency theme.
use crate::tests::run;
use ferroui_base::{
    Point, Ref, Vector, Visual,
    controls::ResourceKey,
    input::{KeyModifiers, Pointer, PointerPointProperties, PointerType, PointerWheelEventArgs},
    media::{Brushes, Color, IBrush, SolidColorBrush},
    metadata::from_markup_value,
    threading::Dispatcher,
};
use ferroui_controls::{
    Application, Control, Grid, ScrollViewer, TextBox, Window, presenters::ScrollContentPresenter,
    templates::IControlTemplate,
};
use std::rc::Rc;

fn presenter(root: &Ref<Visual>) -> Option<Ref<ScrollContentPresenter>> {
    if let Some(value) = root.cast::<ScrollContentPresenter>() {
        return Some(value);
    }
    for index in 0..root.visual_children().count() {
        if let Some(value) = presenter(&root.visual_children().get(index)) {
            return Some(value);
        }
    }
    None
}

fn check_scroll_surface(barless: bool) {
    let viewer = ScrollViewer::new();
    if barless {
        let resource = Application::current()
            .unwrap()
            .try_get_resource(
                &ResourceKey::from("ScrollViewerScrollBarlessTemplate"),
                None,
            )
            .expect("WinUI scrollbarless resource");
        viewer.set_template(Some(
            from_markup_value::<Rc<dyn IControlTemplate>>(&resource)
                .expect("compiled control template"),
        ));
    }
    let content = TextBox::new();
    content.set_height(800.0);
    viewer.set_content(Some(Control::boxed(content.clone())));
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(200.0);
    window.set_content(Some(Control::boxed(viewer.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let surface = presenter(&viewer.clone().upcast()).expect("scrolling presenter");
    let background = viewer
        .background()
        .expect("default transparent hit surface");
    // The selected WinUI theme paints the owner's background on the parent Grid.
    // Its presenter has a separate transparent hit surface in both templates.
    let painted_surface = surface
        .visual_parent()
        .unwrap()
        .cast::<Grid>()
        .expect("WinUI painted scroll surface");
    let hit_background = surface
        .background()
        .expect("transparent presenter hit surface");
    assert!(Rc::ptr_eq(
        &background,
        &painted_surface.background().unwrap()
    ));
    let replacement: Rc<dyn IBrush> =
        SolidColorBrush::with_color(Color::parse("#FF123456").unwrap()).into();
    viewer.set_background(Some(replacement.clone()));
    assert!(Rc::ptr_eq(
        &replacement,
        &painted_surface.background().unwrap()
    ));
    assert!(Rc::ptr_eq(&hit_background, &surface.background().unwrap()));
    viewer.set_background(Some(Brushes::transparent()));
    assert!(content.focus());
    // Raise through the actual InputElement virtual/class dispatch and bubble route.
    // Renderer hit testing is verified separately in the native diagnostic.
    for (delta, expected, handled) in [(-2.0, 100.0, true), (2.0, 0.0, true), (2.0, 0.0, false)] {
        let event = PointerWheelEventArgs::new(
            &surface,
            Pointer::new(1, PointerType::Mouse, true),
            &window,
            Point::new(200.0, 100.0),
            0,
            PointerPointProperties::default(),
            KeyModifiers::empty(),
            Vector::new(0.0, delta),
        );
        surface.raise_event(&event);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(viewer.offset().y, expected);
        assert_eq!(event.handled(), handled);
        assert!(content.is_focused());
    }
    window.close();
}

#[test]
fn winui_default_scroll_surface_forwards_background_and_preserves_wheel_focus() {
    run(|| check_scroll_surface(false));
}

#[test]
fn winui_barless_scroll_surface_forwards_background_and_preserves_wheel_focus() {
    run(|| check_scroll_surface(true));
}
