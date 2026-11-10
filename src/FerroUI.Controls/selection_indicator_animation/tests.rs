use super::*;
use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::presenters::ItemsPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{
    Border, Canvas, Dock, Grid, NavigationView, NavigationViewItem, NavigationViewPaneDisplayMode, Panel, StackPanel,
    TabControl, TabItem,
};
use ferroui_base::media::Brushes;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use std::time::Instant;

fn visual(element: &Control) -> Rc<CompositionVisual> {
    ElementComposition::get_element_visual(element).expect("composited indicator")
}

fn server(element: &Control) -> Rc<ServerCompositionVisual> {
    let visual = visual(element);
    visual
        .compositor()
        .server()
        .get::<ServerCompositionVisual>(visual.server())
        .unwrap()
}

fn indicator(width: f64, height: f64, x: f64, y: f64) -> Ref<Control> {
    let element = Border::new();
    element.set_width(width);
    element.set_height(height);
    element.set_background(Some(Brushes::blue()));
    element.set_opacity(0.0);
    Canvas::set_left(&element, x);
    Canvas::set_top(&element, y);
    element.upcast()
}

fn scene(horizontal: bool) -> (CompositorTestServices, Ref<Canvas>, Ref<Control>, Ref<Control>) {
    let services = CompositorTestServices::new(Size::new(500.0, 400.0));
    let canvas = Canvas::new();
    let first = if horizontal {
        indicator(40.0, 3.0, 10.0, 10.0)
    } else {
        indicator(3.0, 16.0, 10.0, 10.0)
    };
    let second = if horizontal {
        indicator(80.0, 3.0, 110.0, 10.0)
    } else {
        indicator(3.0, 16.0, 10.0, 90.0)
    };
    canvas.children().add(&first);
    canvas.children().add(&second);
    services.set_content(&canvas);
    services.run_jobs();
    (services, canvas, first, second)
}

/// Sample actual render-side animations through composition controllers, independently of
/// wall-clock scheduling. The pivot switches at 200 ms of the 600 ms transition.
fn seek(element: &Control, horizontal: bool, progress: f32) {
    let visual = visual(element);
    let axis = if horizontal { "X" } else { "Y" };
    for property in [
        format!("Offset.{axis}"),
        format!("Scale.{axis}"),
        format!("CenterPoint.{axis}"),
        "Opacity".to_owned(),
    ] {
        if let Some(controller) = visual.try_get_animation_controller(&property) {
            controller.pause();
            controller.set_progress(progress);
        }
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.05, "{actual} != {expected}");
}

fn rendered_edges(element: &Control, horizontal: bool) -> (f64, f64) {
    let visual = server(element);
    let (offset, pivot, scale, dimension) = if horizontal {
        (visual.offset().x, visual.center_point().x, visual.scale().x, element.bounds().width)
    } else {
        (visual.offset().y, visual.center_point().y, visual.scale().y, element.bounds().height)
    };
    let start = offset + pivot * (1.0 - scale);
    (start, start + dimension * scale)
}

#[test]
fn pivot_keeps_both_indicators_inside_the_travel_span() {
    for horizontal in [false, true] {
        for reverse in [false, true] {
            let (services, canvas, first, second) = scene(horizontal);
            if horizontal {
                Canvas::set_left(&second, 370.0);
            } else {
                Canvas::set_top(&second, 370.0);
            }
            services.run_jobs();
            let end = 370.0 + if horizontal { second.bounds().width } else { second.bounds().height };
            let animation = SelectionIndicatorAnimation::default();
            let (from, to) = if reverse { (&second, &first) } else { (&first, &second) };
            animation.update(&canvas, Some(from.clone()), horizontal);
            animation.update(&canvas, Some(to.clone()), horizontal);
            services.run_jobs();
            for milliseconds in [199.7, 199.9, 200.0, 200.1, 200.3] {
                for indicator in [&first, &second] {
                    seek(indicator, horizontal, milliseconds / 600.0);
                }
                services.run_jobs();
                services.run_jobs();
                for indicator in [&first, &second] {
                    let (start, finish) = rendered_edges(indicator, horizontal);
                    assert!(
                        start >= 9.95 && finish <= end + 0.05,
                        "{milliseconds} ms, horizontal={horizontal}, reverse={reverse}: {start}..{finish}"
                    );
                }
            }
        }
    }
}

#[test]
fn distant_selection_bounds_edge_motion_at_60_and_120_hz() {
    for hz in [60, 120] {
        for reverse in [false, true] {
            let (services, canvas, first, second) = scene(false);
            Canvas::set_top(&second, 370.0);
            services.run_jobs();
            let animation = SelectionIndicatorAnimation::default();
            let (from, to) = if reverse { (&second, &first) } else { (&first, &second) };
            animation.update(&canvas, Some(from.clone()), false);
            animation.update(&canvas, Some(to.clone()), false);
            services.run_jobs();
            let mut previous: Option<(f64, f64)> = None;
            for frame in 0..=hz * 600 / 1000 {
                let progress = frame as f32 * 1000.0 / (hz as f32 * 600.0);
                seek(from, false, progress);
                services.run_jobs();
                services.run_jobs();
                let edges = rendered_edges(from, false);
                if let Some((start, end)) = previous {
                    let movement = (edges.0 - start).abs().max((edges.1 - end).abs());
                    assert!(
                        movement < 72.0,
                        "{hz} Hz, reverse={reverse}, frame={frame}: {movement} px"
                    );
                }
                previous = Some(edges);
            }
            close(server(from).scale().y, 1.0);
        }
    }
}

#[test]
fn vertical_stretch_and_pivot_run_on_compositor_without_moving_layout() {
    let (services, canvas, first, second) = scene(false);
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&canvas, Some(first.clone()), false);
    let first_bounds = first.bounds();
    let second_bounds = second.bounds();
    let offset = visual(&first).offset();
    animation.update(&canvas, Some(second.clone()), false);
    services.run_jobs();
    seek(&first, false, 1.0 / 3.0 + 0.000001);
    seek(&second, false, 1.0 / 3.0 + 0.000001);
    services.run_jobs();
    services.run_jobs();
    close(server(&first).scale().y, 6.0); // 80 px travel / 16 px indicator + 1.
    close(server(&second).scale().y, 6.0);
    close((server(&first).offset().y - first.bounds().y), 80.0);
    close(server(&first).center_point().y, 16.0); // Offset and pivot switch together at 200 ms.
    seek(&first, false, 0.5);
    seek(&second, false, 0.5);
    first.invalidate_visual(); // Renderer synchronization must not erase relative movement.
    services.run_jobs();
    services.run_jobs();
    close(server(&first).center_point().y, 16.0);
    close((server(&first).offset().y - first.bounds().y), 80.0);
    assert!(server(&first).scale().y > 1.0 && server(&first).scale().y < 6.0);
    assert!(server(&first).opacity() > 0.0 && server(&first).opacity() < 1.0);
    assert_eq!(visual(&first).offset(), offset);
    assert_eq!(first.bounds(), first_bounds);
    assert_eq!(second.bounds(), second_bounds);
}

#[test]
fn horizontal_motion_preserves_unequal_widths_in_both_directions() {
    for reverse in [false, true] {
        let (services, canvas, first, second) = scene(true);
        let (from, to) = if reverse { (second, first) } else { (first, second) };
        let animation = SelectionIndicatorAnimation::default();
        animation.update(&canvas, Some(from.clone()), true);
        animation.update(&canvas, Some(to.clone()), true);
        services.run_jobs();
        seek(&from, true, 0.0001);
        seek(&to, true, 0.0001);
        services.run_jobs();
        services.run_jobs();
        close(server(&to).scale().x * to.bounds().width, from.bounds().width);
        let rendered_start = server(&to).offset().x + server(&to).center_point().x * (1.0 - server(&to).scale().x);
        close(rendered_start, from.bounds().x);
        seek(&from, true, 1.0);
        seek(&to, true, 1.0);
        services.run_jobs();
        services.run_jobs();
        close(server(&to).scale().x, 1.0);
        close((server(&to).offset().x - to.bounds().x), 0.0);
    }
}

#[test]
fn different_depth_indicators_shrink_and_grow_without_traversing_content() {
    let (services, canvas, first, second) = scene(false);
    Canvas::set_left(&second, 41.0);
    services.run_jobs();
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&canvas, Some(first.clone()), false);
    animation.update(&canvas, Some(second.clone()), false);
    services.run_jobs();
    seek(&first, false, 0.5);
    seek(&second, false, 0.5);
    services.run_jobs();
    services.run_jobs();
    assert!(server(&first).scale().y > 0.0 && server(&first).scale().y < 1.0);
    assert!(server(&second).scale().y > 0.0 && server(&second).scale().y < 1.0);
    close((server(&first).offset().y - first.bounds().y), 0.0);
    close((server(&second).offset().y - second.bounds().y), 0.0);
    close(server(&first).center_point().y, 16.0);
    close(server(&second).center_point().y, 0.0);
}

#[test]
fn repeated_targets_keep_the_batch_and_interruption_resets_old_visuals() {
    let (services, canvas, first, second) = scene(false);
    let third = indicator(3.0, 16.0, 10.0, 170.0);
    canvas.children().add(&third);
    services.run_jobs();
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&canvas, Some(first.clone()), false);
    animation.update(&canvas, Some(second.clone()), false);
    let batch = animation.state.borrow().running.as_ref().unwrap()._batch.clone();
    animation.update(&canvas, Some(second.clone()), false);
    assert!(Rc::ptr_eq(
        &batch,
        &animation.state.borrow().running.as_ref().unwrap()._batch
    ));
    animation.update(&canvas, Some(third.clone()), false);
    services.run_jobs();
    close(server(&first).opacity() as f64, 0.0);
    close(server(&first).scale().y, 1.0);
    close((server(&first).offset().y - first.bounds().y), 0.0);
    assert!(animation.state.borrow().running.is_some());
    animation.clear();
    services.run_jobs();
    assert!(animation.state.borrow().running.is_none());
    close(server(&second).opacity() as f64, 0.0);
    close(server(&third).opacity() as f64, 1.0);
    close(server(&third).scale().y, 1.0);
    close(server(&third).center_point().y, 0.0);
    close((server(&third).offset().y - third.bounds().y), 0.0);
}

#[test]
fn scoped_batch_completion_restores_both_indicators() {
    let (services, canvas, first, second) = scene(false);
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&canvas, Some(first.clone()), false);
    animation.update(&canvas, Some(second.clone()), false);
    let started = Instant::now();
    while animation.state.borrow().running.is_some() && started.elapsed() < Duration::from_secs(2) {
        services.run_jobs();
    }
    services.run_jobs();
    assert!(animation.state.borrow().running.is_none());
    for (element, opacity) in [(&first, 0.0), (&second, 1.0)] {
        close(server(element).opacity() as f64, opacity);
        close(server(element).scale().y, 1.0);
        close(server(element).center_point().y, 0.0);
        close((server(element).offset().y - element.bounds().y), 0.0);
    }
}

#[test]
fn empty_unarranged_and_uncomposited_targets_fall_back_without_animation() {
    let _scope = crate::test_support::test_scope();
    let owner = Canvas::new();
    let first = indicator(3.0, 16.0, 0.0, 0.0);
    let second = indicator(3.0, 16.0, 0.0, 30.0);
    owner.children().add(&first);
    owner.children().add(&second);
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&owner, Some(first.clone()), false);
    animation.update(&owner, Some(second.clone()), false);
    assert_eq!(first.opacity(), 0.0);
    assert_eq!(second.opacity(), 1.0);
    assert!(animation.state.borrow().running.is_none());
    animation.update(&owner, None, false);
    assert_eq!(second.opacity(), 0.0);
}

struct ReducedMotion(ferroui_base::platform::DefaultPlatformSettings);

impl IPlatformSettings for ReducedMotion {
    fn get_tap_size(&self, kind: ferroui_base::input::PointerType) -> Size {
        self.0.get_tap_size(kind)
    }
    fn get_double_tap_size(&self, kind: ferroui_base::input::PointerType) -> Size {
        self.0.get_double_tap_size(kind)
    }
    fn get_double_tap_time(&self, kind: ferroui_base::input::PointerType) -> Duration {
        self.0.get_double_tap_time(kind)
    }
    fn hold_wait_duration(&self) -> Duration {
        self.0.hold_wait_duration()
    }
    fn hotkey_configuration(&self) -> Rc<ferroui_base::input::platform::PlatformHotkeyConfiguration> {
        self.0.hotkey_configuration()
    }
    fn preferred_application_language(&self) -> String {
        self.0.preferred_application_language()
    }
    fn animations_enabled(&self) -> bool {
        false
    }
    fn get_color_values(&self) -> ferroui_base::platform::PlatformColorValues {
        self.0.get_color_values()
    }
    fn color_values_changed(
        &self,
        handler: Rc<dyn Fn(&ferroui_base::platform::PlatformColorValues)>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.0.color_values_changed(handler)
    }
    fn preferred_application_language_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.0.preferred_application_language_changed(handler)
    }
}

#[test]
fn disabled_motion_settles_an_existing_transition_and_future_selections() {
    let (services, canvas, first, second) = scene(false);
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&canvas, Some(first.clone()), false);
    animation.update(&canvas, Some(second.clone()), false);
    services.run_jobs();
    assert!(animation.state.borrow().running.is_some());
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformSettings>()
        .to_constant(Rc::new(ReducedMotion(Default::default())));
    animation.update(&canvas, Some(second.clone()), false);
    services.run_jobs();
    assert!(animation.state.borrow().running.is_none());
    close(server(&second).scale().y, 1.0);
    close((server(&second).offset().y - second.bounds().y), 0.0);
    animation.update(&canvas, Some(first.clone()), false);
    services.run_jobs();
    assert!(animation.state.borrow().running.is_none());
    close(server(&first).opacity() as f64, 1.0);
    close(server(&second).opacity() as f64, 0.0);
}

#[test]
fn resizing_the_target_and_changing_orientation_cancel_old_motion() {
    let (services, canvas, first, second) = scene(false);
    let animation = SelectionIndicatorAnimation::default();
    animation.update(&canvas, Some(first.clone()), false);
    animation.update(&canvas, Some(second.clone()), false);
    services.run_jobs();
    second.set_height(24.0);
    services.run_jobs();
    animation.update(&canvas, Some(second.clone()), false);
    services.run_jobs();
    assert!(animation.state.borrow().running.is_none());
    close(server(&second).scale().y, 1.0);
    animation.update(&canvas, Some(first.clone()), false);
    services.run_jobs();
    animation.update(&canvas, Some(first.clone()), true);
    services.run_jobs();
    assert!(animation.state.borrow().running.is_none());
    close((server(&first).offset().y - first.bounds().y), 0.0);
    close(server(&first).scale().y, 1.0);
}

#[test]
fn tab_control_starts_motion_after_selection_and_cleans_up_on_detach() {
    for placement in [Dock::Top, Dock::Bottom, Dock::Left, Dock::Right] {
        let services = CompositorTestServices::new(Size::new(500.0, 400.0));
        let tabs = TabControl::new();
        tabs.set_tab_strip_placement(placement);
        let mut items = Vec::new();
        for _ in 0..2 {
            let item = TabItem::new();
            item.set_template(Some(FuncControlTemplate::new(|_, scope| {
                let root = Panel::new();
                root.set_width(80.0);
                root.set_height(40.0);
                let pipe = indicator(3.0, 16.0, 0.0, 0.0);
                pipe.set_name(Some("PART_SelectedPipe".to_owned()));
                root.children().add(pipe.register_in_name_scope(&**scope));
                root.upcast()
            })));
            tabs.items().add(Some(Control::boxed(&item)));
            items.push(item);
        }
        tabs.set_template(Some(FuncControlTemplate::new(|owner, scope| {
            let presenter = ItemsPresenter::new();
            presenter.set_items_panel(owner.cast::<TabControl>().unwrap().items_panel());
            presenter.set_name(Some("PART_ItemsPresenter".to_owned()));
            presenter.register_in_name_scope(&**scope).upcast()
        })));
        services.set_content(&tabs);
        services.run_jobs();
        assert_eq!(tabs.selected_index(), 0);
        let first_indicator = items[0].selection_indicator().unwrap();
        assert_eq!(first_indicator.opacity(), 1.0, "initial target should be tracked");
        tabs.set_selected_index(1);
        services.run_jobs();
        let next = items[1].selection_indicator().unwrap();
        let horizontal = matches!(placement, Dock::Top | Dock::Bottom);
        assert!(visual(&next)
            .try_get_animation_controller(if horizontal { "Scale.X" } else { "Scale.Y" })
            .is_some(), "placement={placement:?} selected={} from={:?} to={:?} first_opacity={} next_opacity={} attached={} container={}",
            tabs.selected_index(), first_indicator.bounds(), next.bounds(), first_indicator.opacity(), next.opacity(), tabs.is_attached_to_visual_tree(), tabs.container_from_index(1).is_some());
        let next_visual = visual(&next);
        services.top_level().set_content(None);
        services.run_jobs();
        assert!(next_visual
            .try_get_animation_controller(if horizontal { "Scale.X" } else { "Scale.Y" })
            .is_none());
        tabs.items().clear();
        items[1].set_is_selected(false);
        assert_eq!(
            next.opacity(),
            0.0,
            "a retained tab still updates its indicator after detach"
        );
    }
}

#[test]
fn navigation_tracks_selection_and_collapsed_ancestors_after_layout() {
    let services = CompositorTestServices::new(Size::new(1100.0, 600.0));
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    nav.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let host = StackPanel::new();
        host.set_name(Some("MenuItemsHost".to_owned()));
        host.register_in_name_scope(&**scope).upcast()
    })));
    let parent = NavigationViewItem::new();
    let child = NavigationViewItem::new();
    let sibling = NavigationViewItem::new();
    for item in [&parent, &child, &sibling] {
        item.set_height(40.0);
        item.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let root = Grid::new();
            root.set_name(Some("NVIRootGrid".to_owned()));
            root.set_row_definitions("40,Auto".parse().unwrap());
            let pipe = indicator(3.0, 16.0, 0.0, 0.0);
            pipe.set_name(Some("SelectionIndicator".to_owned()));
            root.children().add(pipe.register_in_name_scope(&**scope));
            let host = StackPanel::new();
            host.set_name(Some("NavigationViewItemMenuItemsHost".to_owned()));
            Grid::set_row(&host, 1);
            root.children().add(host.register_in_name_scope(&**scope));
            root.register_in_name_scope(&**scope).upcast()
        })));
    }
    parent.menu_items().add(Some(Control::boxed(&child)));
    nav.menu_items().add(Some(Control::boxed(&parent)));
    nav.menu_items().add(Some(Control::boxed(&sibling)));
    nav.set_selected_item(Some(Control::boxed(&parent)));
    services.set_content(&nav);
    services.run_jobs();
    nav.set_selected_item(Some(Control::boxed(&sibling)));
    services.run_jobs();
    let sibling_indicator = sibling.selection_indicator().unwrap();
    assert!(visual(&sibling_indicator)
        .try_get_animation_controller("Scale.Y")
        .is_some());
    parent.set_is_expanded(false);
    nav.set_selected_item(Some(Control::boxed(&child)));
    services.run_jobs();
    assert_eq!(parent.selection_indicator().unwrap().opacity(), 1.0);
    assert!(!child.is_effectively_visible());
    parent.set_is_expanded(true);
    parent.set_height(100.0);
    services.run_jobs();
    assert_eq!(child.selection_indicator().unwrap().opacity(), 1.0);
    let child_visual = visual(&child.selection_indicator().unwrap());
    assert!(child_visual.try_get_animation_controller("Scale.Y").is_some());
    services.top_level().set_content(None);
    services.run_jobs();
    assert!(child_visual.try_get_animation_controller("Scale.Y").is_none());
}
