use super::*;
use crate::tests::{layout, run, scene};
#[test]
fn set_target_proportion_respects_target_min_width_constraint() {
    run(|| {
        let (panel, a, splitter, _) = scene(Orientation::Horizontal, 600.0, 0.4, 0.6);
        a.set_min_width(150.0);
        layout(&panel);
        splitter.set_target_proportion(-200.0);
        layout(&panel);
        assert!(a.bounds().width >= 150.0);
    });
}
#[test]
fn set_target_proportion_respects_target_max_width_constraint() {
    run(|| {
        let (panel, a, splitter, _) = scene(Orientation::Horizontal, 600.0, 0.3, 0.7);
        a.set_max_width(250.0);
        layout(&panel);
        splitter.set_target_proportion(200.0);
        layout(&panel);
        assert!(a.bounds().width <= 250.0);
    });
}
#[test]
fn set_target_proportion_respects_neighbor_min_width_constraint() {
    run(|| {
        let (panel, _, splitter, b) = scene(Orientation::Horizontal, 600.0, 0.4, 0.6);
        b.set_min_width(200.0);
        layout(&panel);
        splitter.set_target_proportion(250.0);
        layout(&panel);
        assert!(b.bounds().width >= 200.0);
    });
}
#[test]
fn set_target_proportion_respects_neighbor_max_width_constraint() {
    run(|| {
        let (panel, _, splitter, b) = scene(Orientation::Horizontal, 600.0, 0.6, 0.4);
        b.set_max_width(150.0);
        layout(&panel);
        splitter.set_target_proportion(-200.0);
        layout(&panel);
        assert!(b.bounds().width <= 150.0);
    });
}
#[test]
fn set_target_proportion_works_in_vertical_orientation() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Vertical, 600.0, 0.4, 0.6);
        a.set_min_height(150.0);
        a.set_max_height(300.0);
        b.set_min_height(100.0);
        layout(&panel);
        splitter.set_target_proportion(-200.0);
        layout(&panel);
        assert!(a.bounds().height >= 150.0);
        assert!(b.bounds().height >= 100.0);
        splitter.set_target_proportion(200.0);
        layout(&panel);
        assert!(a.bounds().height <= 300.0);
    });
}
#[test]
fn set_target_proportion_handles_conflicting_constraints_gracefully() {
    run(|| {
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 300.0, 0.5, 0.5);
        a.set_min_width(200.0);
        b.set_min_width(150.0);
        layout(&panel);
        splitter.set_target_proportion(50.0);
        layout(&panel);
        assert!(a.bounds().width >= 200.0);
        assert!(b.bounds().width >= 150.0);
    });
}

#[test]
fn preview_field_is_assigned_after_object_initializer_callbacks() {
    run(|| {
        use crate::splitter_preview_adorner::SplitterPreviewAdorner;
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        use ferroui_base::media::Brushes;
        use ferroui_base::{Point, Visual};
        use ferroui_controls::primitives::VisualLayerManager;
        use ferroui_controls::templates::FuncControlTemplate;
        use ferroui_controls::{Border, Control, Window};
        use ferroui_headless::HeadlessWindowExtensions;
        use std::cell::Cell;
        use std::rc::Rc;
        let (panel, _, splitter, _) = scene(Orientation::Horizontal, 600.0, 0.4, 0.6);
        splitter.set_preview_resize(true);
        splitter.set_preview_opacity(0.5);
        splitter.set_template(Some(FuncControlTemplate::new(|_, _| {
            let border = Border::new();
            border.set_background(Some(Brushes::gray()));
            border.upcast()
        })));
        let weak = splitter.downgrade();
        let observed = Rc::new(Cell::new(false));
        let seen = observed.clone();
        let subscription = Visual::opacity_property()
            .changed()
            .add_class_handler::<SplitterPreviewAdorner>(move |_, _| {
                let splitter = weak.upgrade().unwrap();
                assert!(splitter.preview_adorner.borrow().is_none());
                seen.set(true);
                splitter.set_preview_background(Some(Brushes::blue()));
            });
        let manager = VisualLayerManager::new();
        manager.set_child(panel);
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(200.0);
        window.set_content(Some(Control::boxed(manager.clone())));
        window.show();
        crate::tests::update(&window);
        let point = splitter
            .translate_point(Point::new(2.0, 20.0), &window)
            .unwrap();
        window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        assert!(observed.get());
        assert!(splitter.current_preview().background().is_none());
        assert!(splitter.preview_background().is_some());
        subscription.dispose();
        window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        assert_eq!(manager.adorner_layer().unwrap().children().count(), 0);
        window.close();
    });
}

#[test]
fn content_presenter_splitter_discovers_panel_and_skips_collapsed_and_splitter_siblings() {
    run(|| {
        use ferroui_controls::presenters::ContentPresenter;
        use ferroui_controls::{Border, Control};
        let (panel, a, splitter, b) = scene(Orientation::Horizontal, 600.0, 0.4, 0.6);
        panel.children().clear();
        let collapsed = Border::new();
        ProportionalStackPanel::set_is_collapsed(&collapsed, true);
        let presenter = ContentPresenter::new();
        presenter.set_content(Some(Control::boxed(splitter.clone())));
        panel.children().add(a.clone());
        panel.children().add(collapsed);
        panel.children().add(presenter.clone());
        panel.children().add(ProportionalStackPanelSplitter::new());
        panel.children().add(b.clone());
        layout(&panel);
        assert_eq!(
            ProportionalStackPanelSplitter::is_splitter(Some(&presenter.upcast())),
            Some(splitter.clone())
        );
        assert_eq!(splitter.get_panel(), Some(panel.clone()));
        splitter.set_target_proportion(50.0);
        let expected = 0.4 + 50.0 / panel.bounds().width;
        assert!((ProportionalStackPanel::get_proportion(&a) - expected).abs() < 1e-9);
        assert!((ProportionalStackPanel::get_proportion(&b) - (1.0 - expected)).abs() < 1e-9);
    });
}
