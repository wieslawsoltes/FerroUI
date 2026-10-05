//! Tests of the adorner layer over a compositing renderer: the composition
//! visual of an adorner knows the composition visual it adorns.

use crate::primitives::AdornerLayer;
use crate::testing::{CompositorTestServices, MockWindowingPlatform, TestServices};
use crate::{Border, Control, Window};
use ferroui_base::media::Brushes;
use ferroui_base::rendering::composition::{CompositingRenderer, CompositionVisual};
use ferroui_base::{Ref, Visual};
use std::rc::Rc;

fn shown_window(services: &CompositorTestServices, content: &Ref<Border>) -> Ref<Window> {
    let window_impl = MockWindowingPlatform::create_window_mock();
    services.setup(&window_impl);
    let window = Window::with_impl(window_impl);
    window.set_content(Some(Control::boxed(content.clone())));
    window.show();
    services.run_jobs();
    assert!(window.presentation_source().typed_renderer().as_any().is::<CompositingRenderer>());
    window
}

fn composition_visual(visual: &Visual) -> Rc<CompositionVisual> {
    (*visual.composition_visual().expect("the visual is attached to a composited tree")).clone()
}

fn assert_adorns(adorner: &Visual, adorned: &Visual, is_clipped: bool) {
    let adorner_visual = composition_visual(adorner);
    let adorned_visual = adorner_visual.adorned_visual().expect("the adorner has an adorned visual");
    assert!(Rc::ptr_eq(&composition_visual(adorned), &adorned_visual));
    assert_eq!(is_clipped, adorner_visual.adorner_is_clipped());
}

#[test]
fn adorner_composition_visual_gets_the_adorned_visual() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let adorned = Border::new();
    adorned.set_background(Some(Brushes::red()));
    let window = shown_window(&services, &adorned);

    let adorner = Border::new();
    assert!(AdornerLayer::get_is_clip_enabled(&adorner));
    AdornerLayer::set_adorner(&adorned, adorner.clone());
    services.run_jobs();

    let layer = AdornerLayer::get_adorner_layer(&adorned).expect("the window has an adorner layer");
    assert!(layer.children().contains(&adorner.clone().upcast()));
    assert_adorns(&adorner, &adorned, true);

    window.close();
}

#[test]
fn adorner_composition_visual_follows_is_clip_enabled() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let adorned = Border::new();
    let window = shown_window(&services, &adorned);

    let adorner = Border::new();
    AdornerLayer::set_is_clip_enabled(&adorner, false);
    AdornerLayer::set_adorner(&adorned, adorner.clone());
    services.run_jobs();

    assert_adorns(&adorner, &adorned, false);

    window.close();
}

#[test]
fn adorner_set_before_the_tree_is_composited_gets_the_adorned_visual_on_attach() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let adorned = Border::new();
    let adorner = Border::new();
    AdornerLayer::set_adorner(&adorned, adorner.clone());
    assert!(adorner.composition_visual().is_none());

    let window = shown_window(&services, &adorned);

    assert_adorns(&adorner, &adorned, true);

    AdornerLayer::set_adorner(&adorned, None);
    services.run_jobs();
    assert!(adorner.composition_visual().is_none());

    window.close();
}
