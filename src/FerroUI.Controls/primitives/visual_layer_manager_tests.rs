//! Port of the reference `VisualLayerManagerTests`, with tests of the
//! layers and of their wiring into the top-level added. The reference
//! tests use a button as the layer manager's child; a border stands in for
//! it here.

use crate::embedding::EmbeddableControlRoot;
use crate::primitives::{
    AdornerLayer, LightDismissOverlayLayer, OverlayLayer, PopupOverlayLayer, TextSelectorLayer, VisualLayerManager,
};
use crate::test_support::TestRoot;
use crate::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication};
use crate::{Border, Control, Panel};
use ferroui_base::media::RectangleGeometry;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Matrix, Rect, Ref, Size, StyledElement, Thickness, Visual};

fn layout(root: &TestRoot) {
    root.measure(Size::new(100.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
}

#[test]
fn get_adorner_layer_returns_dedicated_adorner_layer_for_controls_inside_overlay_layer() {
    let _scope = Dispatcher::unit_test_scope();
    let button = Border::new();
    let vlm = VisualLayerManager::new();
    vlm.set_enable_overlay_layer(true);
    vlm.set_child(&button);
    let root = TestRoot::with_child(vlm.clone());

    layout(&root);

    let overlay_layer = vlm.overlay_layer().expect("the overlay layer is enabled");

    let overlay_child = Border::new();
    overlay_layer.children().add(overlay_child.clone());

    // The adorner layer for a control inside the OverlayLayer
    // should be the dedicated one, not the main VLM adorner layer.
    let overlay_adorner_layer = AdornerLayer::get_adorner_layer(&overlay_child).expect("an adorner layer");
    assert_eq!(overlay_layer.adorner_layer().unwrap(), overlay_adorner_layer);

    // The main VLM adorner layer should be different.
    let main_adorner_layer = AdornerLayer::get_adorner_layer(&button).expect("an adorner layer");
    assert_ne!(overlay_adorner_layer, main_adorner_layer);
}

#[test]
fn get_adorner_layer_returns_same_adorner_layer_for_visual_layer_manager() {
    let _scope = Dispatcher::unit_test_scope();
    let vlm = VisualLayerManager::new();
    let root = TestRoot::with_child(vlm.clone());

    layout(&root);

    let adorner_layer = vlm.adorner_layer().expect("the adorner layer is enabled");

    let target = AdornerLayer::get_adorner_layer(&vlm).expect("an adorner layer");
    assert_eq!(adorner_layer, target);
}

#[test]
fn get_adorner_layer_returns_same_adorner_layer_for_child() {
    let _scope = Dispatcher::unit_test_scope();
    let button = Border::new();
    let vlm = VisualLayerManager::new();
    vlm.set_child(&button);
    let root = TestRoot::with_child(vlm.clone());

    layout(&root);

    let adorner_layer = vlm.adorner_layer().expect("the adorner layer is enabled");

    let target = AdornerLayer::get_adorner_layer(&button).expect("an adorner layer");
    assert_eq!(adorner_layer, target);
}

#[test]
fn get_overlay_layer_returns_same_overlay_layer_for_visual_layer_manager() {
    let _scope = Dispatcher::unit_test_scope();
    let vlm = VisualLayerManager::new();
    vlm.set_enable_overlay_layer(true);
    let root = TestRoot::with_child(vlm.clone());

    layout(&root);

    let overlay_layer = vlm.overlay_layer().expect("the overlay layer is enabled");

    let target = OverlayLayer::get_overlay_layer(&vlm).expect("an overlay layer");
    assert_eq!(overlay_layer, target);
}

#[test]
fn get_overlay_layer_returns_same_overlay_layer_for_child() {
    let _scope = Dispatcher::unit_test_scope();
    let button = Border::new();
    let vlm = VisualLayerManager::new();
    vlm.set_enable_overlay_layer(true);
    vlm.set_child(&button);
    let root = TestRoot::with_child(vlm.clone());

    layout(&root);

    let overlay_layer = vlm.overlay_layer().expect("the overlay layer is enabled");

    let target = OverlayLayer::get_overlay_layer(&button).expect("an overlay layer");
    assert_eq!(overlay_layer, target);
}

// --- added tests ------------------------------------------------------------

#[test]
fn layers_are_disabled_by_default_except_the_adorner_and_light_dismiss_layers() {
    let _scope = Dispatcher::unit_test_scope();
    let vlm = VisualLayerManager::new();

    assert!(vlm.enable_adorner_layer());
    assert!(!vlm.enable_overlay_layer());
    assert!(!vlm.enable_popup_overlay_layer());
    assert!(!vlm.enable_text_selector_layer());

    assert!(vlm.overlay_layer().is_none());
    assert!(vlm.popup_overlay_layer().is_none());
    assert!(vlm.text_selector_layer().is_none());
    assert_eq!(0, vlm.get_visual_children().len());

    assert!(vlm.adorner_layer().is_some());
    vlm.light_dismiss_overlay_layer();
    assert_eq!(2, vlm.get_visual_children().len());

    vlm.set_enable_adorner_layer(false);
    assert!(vlm.adorner_layer().is_none());
}

#[test]
fn layers_are_created_once_with_their_z_order_and_logical_parent() {
    let _scope = Dispatcher::unit_test_scope();
    let child = Border::new();
    let vlm = VisualLayerManager::new();
    vlm.set_enable_overlay_layer(true);
    vlm.set_enable_popup_overlay_layer(true);
    vlm.set_enable_text_selector_layer(true);
    vlm.set_child(&child);
    let root = TestRoot::with_child(vlm.clone());
    layout(&root);

    let adorner = vlm.adorner_layer().unwrap();
    let overlay = vlm.overlay_layer().unwrap();
    let light_dismiss = vlm.light_dismiss_overlay_layer();
    let text_selector = vlm.text_selector_layer().unwrap();
    let popup = vlm.popup_overlay_layer().unwrap();

    assert_eq!(adorner, vlm.adorner_layer().unwrap());
    assert_eq!(overlay, vlm.overlay_layer().unwrap());
    assert_eq!(light_dismiss, vlm.light_dismiss_overlay_layer());
    assert_eq!(text_selector, vlm.text_selector_layer().unwrap());
    assert_eq!(popup, vlm.popup_overlay_layer().unwrap());

    // The child and the five layers.
    assert_eq!(6, vlm.get_visual_children().len());
    // The layers are not logical children.
    assert_eq!(vec![child.clone().upcast::<StyledElement>()], vlm.logical_children().to_vec());

    let overlay_panel = overlay.visual_parent().unwrap().cast::<Panel>().expect("the overlay layer is in a panel");
    assert_eq!(2, overlay_panel.children().count());
    assert_eq!(overlay.adorner_layer().unwrap(), overlay_panel.children().get(1));

    assert_eq!(i32::MAX - 100, adorner.z_index());
    assert_eq!(i32::MAX - 98, overlay_panel.z_index());
    assert_eq!(i32::MAX - 97, light_dismiss.z_index());
    assert_eq!(i32::MAX - 96, text_selector.z_index());
    assert_eq!(i32::MAX - 95, popup.z_index());

    for layer in [
        adorner.clone().upcast::<Control>(),
        overlay_panel.clone().upcast(),
        light_dismiss.clone().upcast(),
        text_selector.clone().upcast(),
        popup.clone().upcast(),
    ] {
        assert_eq!(vlm.clone().upcast::<StyledElement>(), layer.parent().unwrap());
        assert!(layer.is_attached_to_logical_tree());
        assert!(layer.is_attached_to_visual_tree());
    }

    assert_eq!(popup, PopupOverlayLayer::get_popup_overlay_layer(&child).unwrap());
    assert_eq!(text_selector, TextSelectorLayer::get_text_selector_layer(&child).unwrap());
    assert_eq!(light_dismiss, LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&child).unwrap());
}

#[test]
fn layers_are_measured_and_arranged_with_the_size_of_the_manager() {
    let _scope = Dispatcher::unit_test_scope();
    let child = Border::new();
    child.set_width(30.0);
    child.set_height(20.0);
    let vlm = VisualLayerManager::new();
    vlm.set_enable_overlay_layer(true);
    vlm.set_enable_popup_overlay_layer(true);
    vlm.set_enable_text_selector_layer(true);
    vlm.set_child(&child);
    let root = TestRoot::with_child(vlm.clone());

    let adorner = vlm.adorner_layer().unwrap();
    let overlay = vlm.overlay_layer().unwrap();
    let popup = vlm.popup_overlay_layer().unwrap();
    let text_selector = vlm.text_selector_layer().unwrap();
    let light_dismiss = vlm.light_dismiss_overlay_layer();
    let registration = light_dismiss.register(None);

    root.set_client_size(Size::new(100.0, 100.0));
    root.execute_initial_layout_pass();

    // The layers do not take part in the desired size of the manager.
    assert_eq!(Size::new(30.0, 20.0), vlm.desired_size());

    // The test root is arranged with its desired size.
    let expected = Rect::new(0.0, 0.0, 30.0, 20.0);
    assert_eq!(expected, vlm.bounds());
    assert_eq!(expected, adorner.bounds());
    assert_eq!(expected, overlay.bounds());
    assert_eq!(expected, popup.bounds());
    assert_eq!(expected, text_selector.bounds());
    assert_eq!(expected, light_dismiss.bounds());
    assert_eq!(Size::new(30.0, 20.0), overlay.available_size());
    assert_eq!(Size::new(30.0, 20.0), popup.available_size());
    assert_eq!(Size::new(30.0, 20.0), text_selector.available_size());

    registration.dispose();
}

#[test]
fn light_dismiss_layer_is_visible_while_registered() {
    let _scope = Dispatcher::unit_test_scope();
    let vlm = VisualLayerManager::new();
    let pass_through = Border::new();
    vlm.set_child(&pass_through);
    let root = TestRoot::with_child(vlm.clone());
    layout(&root);

    let layer = vlm.light_dismiss_overlay_layer();
    assert!(!layer.is_visible());
    assert!(layer.background().is_some());
    assert!(layer.input_pass_through_element().is_none());

    let first = layer.register(None);
    assert!(layer.is_visible());
    assert!(layer.input_pass_through_element().is_none());
    // Without a pass-through element every point hits the layer.
    assert!(layer.hit_test(ferroui_base::Point::new(10.0, 10.0)));

    let second = layer.register(Some(&pass_through.clone().upcast()));
    assert_eq!(pass_through, layer.input_pass_through_element().unwrap());

    second.dispose();
    second.dispose();
    assert!(layer.is_visible());
    assert!(layer.input_pass_through_element().is_none());

    first.dispose();
    assert!(!layer.is_visible());
}

#[test]
fn adorner_property_adds_the_adorner_to_the_layer_and_positions_it_over_the_adorned_element() {
    let _scope = Dispatcher::unit_test_scope();
    let adorned = Border::new();
    adorned.set_width(40.0);
    adorned.set_height(30.0);
    adorned.set_margin(Thickness::new(10.0, 20.0, 0.0, 0.0));
    adorned.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
    adorned.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    let vlm = VisualLayerManager::new();
    vlm.set_width(100.0);
    vlm.set_height(100.0);
    vlm.set_child(&adorned);
    let root = TestRoot::with_child(vlm.clone());
    root.set_client_size(Size::new(100.0, 100.0));
    root.execute_initial_layout_pass();

    let adorner = Border::new();
    AdornerLayer::set_adorner(&adorned, adorner.clone());

    let layer = vlm.adorner_layer().unwrap();
    assert_eq!(vec![adorner.clone().upcast::<Control>()], layer.children().to_vec());
    assert_eq!(adorned.clone().upcast::<Visual>(), AdornerLayer::get_adorned_element(&adorner).unwrap());
    assert_eq!(adorned.clone().upcast::<StyledElement>(), adorner.parent().unwrap());
    assert!(AdornerLayer::get_is_clip_enabled(&adorner));

    root.layout_manager().execute_layout_pass();

    assert_eq!(Rect::new(0.0, 0.0, 40.0, 30.0), adorner.bounds());
    assert_eq!(Matrix::create_translation(10.0, 20.0), adorner.render_transform().unwrap().value());
    // Nothing clips the adorned element.
    assert!(adorner.clip().is_none());

    // Moving the adorned element moves the adorner.
    adorned.set_margin(Thickness::new(25.0, 5.0, 0.0, 0.0));
    root.layout_manager().execute_layout_pass();
    assert_eq!(Matrix::create_translation(25.0, 5.0), adorner.render_transform().unwrap().value());

    // A clipping ancestor clips the adorner.
    vlm.set_clip_to_bounds(true);
    root.layout_manager().execute_layout_pass();
    let clip = adorner.clip().expect("the adorner is clipped by the layer manager");
    let clip = clip.cast::<RectangleGeometry>().expect("the clip is the bounds of the layer manager");
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), clip.rect());
    assert_eq!(Matrix::create_translation(-25.0, -5.0), clip.transform().unwrap().value());

    AdornerLayer::set_is_clip_enabled(&adorner, false);
    root.layout_manager().execute_layout_pass();
    assert!(adorner.clip().is_none());

    // Detaching the adorned element removes the adorner, attaching it
    // again restores it.
    vlm.set_child(None);
    assert!(layer.children().is_empty());
    assert!(adorner.parent().is_none());

    vlm.set_child(&adorned);
    assert_eq!(1, layer.children().count());

    AdornerLayer::set_adorner(&adorned, None);
    assert!(layer.children().is_empty());
    assert!(adorner.parent().is_none());
}

#[test]
fn adorner_without_adorned_element_is_arranged_like_a_canvas_child() {
    let _scope = Dispatcher::unit_test_scope();
    let vlm = VisualLayerManager::new();
    let root = TestRoot::with_child(vlm.clone());
    root.set_client_size(Size::new(100.0, 100.0));

    let layer = vlm.adorner_layer().unwrap();
    let child = Border::new();
    child.set_width(10.0);
    child.set_height(12.0);
    crate::Canvas::set_left(&child, 5.0);
    crate::Canvas::set_top(&child, 7.0);
    layer.children().add(child.clone());

    root.execute_initial_layout_pass();

    assert_eq!(Rect::new(5.0, 7.0, 10.0, 12.0), child.bounds());
    assert!(child.render_transform().is_none());
}

#[test]
fn top_level_finds_the_layer_manager_of_its_template_and_the_embeddable_root_enables_the_layers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = MockWindowImpl::bare(MockImplKind::TopLevel);
    let target = EmbeddableControlRoot::with_impl(platform_impl);
    let content = Border::new();
    target.set_content(Some(Control::boxed(content.clone())));

    assert!(target.visual_layer_manager().is_none());

    target.prepare();

    let vlm: Ref<VisualLayerManager> = target.visual_layer_manager().expect("the template has a layer manager");
    assert_eq!(Some("PART_VisualLayerManager".to_string()), vlm.name());
    assert!(vlm.enable_adorner_layer());
    assert!(vlm.enable_overlay_layer());
    assert!(vlm.enable_popup_overlay_layer());
    assert!(vlm.enable_text_selector_layer());

    assert_eq!(vlm.adorner_layer().unwrap(), AdornerLayer::get_adorner_layer(&content).unwrap());
    assert_eq!(vlm.overlay_layer().unwrap(), OverlayLayer::get_overlay_layer(&content).unwrap());
    assert_eq!(vlm.popup_overlay_layer().unwrap(), PopupOverlayLayer::get_popup_overlay_layer(&content).unwrap());
    assert_eq!(vlm.text_selector_layer().unwrap(), TextSelectorLayer::get_text_selector_layer(&content).unwrap());

    // From the top-level itself the layers are found through its template.
    assert_eq!(vlm.overlay_layer().unwrap(), OverlayLayer::get_overlay_layer(&target).unwrap());
    assert_eq!(vlm.popup_overlay_layer().unwrap(), PopupOverlayLayer::get_popup_overlay_layer(&target).unwrap());
    assert_eq!(
        vlm.light_dismiss_overlay_layer(),
        LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&target).unwrap()
    );
    assert_eq!(
        vlm.light_dismiss_overlay_layer(),
        LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&content).unwrap()
    );
}

#[test]
fn light_dismiss_layer_lets_input_through_to_the_pass_through_element() {
    let _scope = Dispatcher::unit_test_scope();
    let pass_through = Border::new();
    pass_through.set_width(40.0);
    pass_through.set_height(40.0);
    pass_through.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
    pass_through.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    let transparent: std::rc::Rc<dyn ferroui_base::media::IBrush> = ferroui_base::media::Brushes::transparent();
    // The layer lets through what is inside the pass-through element: the
    // element itself is not its own visual ancestor.
    let inner = Border::new();
    inner.set_background(Some(transparent.clone()));
    pass_through.set_child(&inner);
    let other = Border::new();
    other.set_width(40.0);
    other.set_height(40.0);
    other.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Right);
    other.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Bottom);
    other.set_background(Some(transparent));
    let panel = Panel::new();
    panel.children().add(pass_through.clone());
    panel.children().add(other);
    let vlm = VisualLayerManager::new();
    vlm.set_width(100.0);
    vlm.set_height(100.0);
    vlm.set_child(&panel);
    let root = TestRoot::with_child(vlm.clone());
    root.set_client_size(Size::new(100.0, 100.0));

    let layer = vlm.light_dismiss_overlay_layer();
    let registration = layer.register(Some(&pass_through.clone().upcast()));
    root.execute_initial_layout_pass();

    // Over the content of the pass-through element the layer is not hit.
    assert!(!layer.hit_test(ferroui_base::Point::new(10.0, 10.0)));
    // Over another element, and over nothing, it is.
    assert!(layer.hit_test(ferroui_base::Point::new(90.0, 90.0)));
    assert!(layer.hit_test(ferroui_base::Point::new(50.0, 50.0)));

    registration.dispose();
}

#[test]
fn adorner_and_adorned_element_do_not_keep_each_other_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let adorned = Border::new();
    let vlm = VisualLayerManager::new();
    vlm.set_child(&adorned);
    let root = TestRoot::with_child(vlm.clone());
    root.execute_initial_layout_pass();

    let adorner = Border::new();
    AdornerLayer::set_adorner(&adorned, adorner.clone());
    assert!(AdornerLayer::get_adorned_element(&adorner).is_some());

    // Out of the tree: the adorner leaves the layer, the pair is only
    // referenced from here.
    vlm.set_child(None);
    assert!(vlm.adorner_layer().unwrap().children().is_empty());
    // The layout queue references the invalidated controls until the pass.
    root.layout_manager().execute_layout_pass();

    let weak_adorned = adorned.downgrade();
    let weak_adorner = adorner.downgrade();
    drop(adorned);
    drop(adorner);

    assert!(weak_adorned.upgrade().is_none());
    assert!(weak_adorner.upgrade().is_none());
}
