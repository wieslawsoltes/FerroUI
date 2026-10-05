//! The tests of the acrylic border. The reference has none; these run the
//! border over a compositing renderer and look at the acrylic composition
//! visual it creates.

use crate::platform::ITopLevelImpl;
use crate::test_support::TestRoot;
use crate::testing::{CompositorTestServices, MockWindowImpl, MockWindowingPlatform, TestServices};
use crate::{AcrylicPlatformCompensationLevels, Control, ExperimentalAcrylicBorder, Window, WindowTransparencyLevel};
use ferroui_base::media::{Color, ExperimentalAcrylicMaterial};
use ferroui_base::rendering::composition::{CompositingRenderer, CompositionExperimentalAcrylicVisual};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{CornerRadius, Ref};
use std::rc::Rc;

const LEVELS: AcrylicPlatformCompensationLevels = AcrylicPlatformCompensationLevels::new(0.25, 0.5, 0.75);

fn material(tint_color: Color) -> Ref<ExperimentalAcrylicMaterial> {
    let material = ExperimentalAcrylicMaterial::new();
    material.set_tint_color(tint_color);
    material.set_tint_opacity(0.4);
    material.set_fallback_color(Color::from_rgb(1, 2, 3));
    material
}

fn window_mock(services: &CompositorTestServices, level: WindowTransparencyLevel) -> Rc<MockWindowImpl> {
    let window_impl = MockWindowingPlatform::create_window_mock();
    services.setup(&window_impl);
    window_impl.acrylic_compensation_levels.set(LEVELS);
    window_impl.transparency_level.set(level);
    window_impl
}

fn shown_window(
    services: &CompositorTestServices,
    window_impl: &Rc<MockWindowImpl>,
    content: &Ref<ExperimentalAcrylicBorder>,
) -> Ref<Window> {
    let window = Window::with_impl(window_impl.clone());
    window.set_content(Some(Control::boxed(content.clone())));
    window.show();
    services.run_jobs();
    assert!(window.presentation_source().typed_renderer().as_any().is::<CompositingRenderer>());
    window
}

/// Makes the platform report a new transparency level.
fn change_transparency_level(window_impl: &Rc<MockWindowImpl>, level: WindowTransparencyLevel) {
    window_impl.transparency_level.set(level);
    let changed = ITopLevelImpl::transparency_level_changed(&**window_impl).expect("the top level listens");
    changed(level);
}

fn acrylic_visual(target: &ExperimentalAcrylicBorder) -> CompositionExperimentalAcrylicVisual {
    let visual = target.composition_visual().expect("the border is attached to a composited tree");
    CompositionExperimentalAcrylicVisual::from_visual(&visual).expect("the border has an acrylic composition visual")
}

/// Checks that the composition visual of `target` has the values of
/// `material` as they are now.
fn assert_visual_material(target: &ExperimentalAcrylicBorder, material: &ExperimentalAcrylicMaterial) {
    let actual = acrylic_visual(target).material();
    let expected = material.to_immutable();
    assert_eq!(expected.background_source(), actual.background_source());
    assert_eq!(expected.tint_color(), actual.tint_color());
    assert_eq!(expected.tint_opacity(), actual.tint_opacity());
    assert_eq!(expected.material_color(), actual.material_color());
    assert_eq!(expected.fallback_color(), actual.fallback_color());
}

#[test]
fn composition_visual_is_an_acrylic_visual_with_the_material_and_the_corner_radius() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let material = material(Color::from_rgb(10, 20, 30));
    target.set_material(material.clone());
    target.set_corner_radius(CornerRadius::new(1.0, 2.0, 3.0, 4.0));

    assert!(target.composition_visual().is_none());

    let window_impl = window_mock(&services, WindowTransparencyLevel::none());
    let window = shown_window(&services, &window_impl, &target);

    let visual = acrylic_visual(&target);
    assert_eq!(CornerRadius::new(1.0, 2.0, 3.0, 4.0), visual.corner_radius());
    // The tint color of the immutable material is the effective one: the
    // tint color with the tint opacity applied.
    assert_ne!(Color::default(), visual.material().tint_color());
    assert_eq!(material.to_immutable().tint_color(), visual.material().tint_color());
    assert_eq!(0.4, visual.material().tint_opacity());
    assert_eq!(Color::from_rgb(1, 2, 3), visual.material().fallback_color());
    assert_visual_material(&target, &material);

    window.close();
}

#[test]
fn composition_visual_without_a_material_has_the_default_material() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let window_impl = window_mock(&services, WindowTransparencyLevel::none());
    let window = shown_window(&services, &window_impl, &target);

    let visual = acrylic_visual(&target);
    assert_eq!(CornerRadius::default(), visual.corner_radius());
    assert_eq!(Color::default(), visual.material().tint_color());
    assert_eq!(0.0, visual.material().tint_opacity());

    window.close();
}

#[test]
fn composition_visual_follows_the_changes_of_the_material() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let first = material(Color::from_rgb(10, 20, 30));
    target.set_material(first.clone());
    let window_impl = window_mock(&services, WindowTransparencyLevel::none());
    let window = shown_window(&services, &window_impl, &target);
    assert_eq!(1, first.property_changed_subscriber_count());

    // A property of the material.
    let before = acrylic_visual(&target).material().tint_color();
    first.set_tint_color(Color::from_rgb(40, 50, 60));
    assert_ne!(before, acrylic_visual(&target).material().tint_color());
    assert_visual_material(&target, &first);
    first.set_tint_opacity(0.9);
    assert_eq!(0.9, acrylic_visual(&target).material().tint_opacity());
    assert_visual_material(&target, &first);
    first.set_fallback_color(Color::from_rgb(7, 8, 9));
    assert_eq!(Color::from_rgb(7, 8, 9), acrylic_visual(&target).material().fallback_color());
    assert_visual_material(&target, &first);
    // The border listens once, however often the material changed.
    assert_eq!(1, first.property_changed_subscriber_count());

    // Another material.
    let second = material(Color::from_rgb(70, 80, 90));
    target.set_material(second.clone());
    assert_visual_material(&target, &second);
    assert_eq!(0, first.property_changed_subscriber_count());
    assert_eq!(1, second.property_changed_subscriber_count());

    // The replaced material is no longer followed.
    first.set_tint_color(Color::from_rgb(1, 1, 1));
    assert_visual_material(&target, &second);

    // No material: the visual keeps what it was given last (nothing is
    // synchronized without a material, as in the reference).
    target.set_material(None);
    assert_eq!(0, second.property_changed_subscriber_count());
    assert_eq!(second.to_immutable().tint_color(), acrylic_visual(&target).material().tint_color());

    services.run_jobs();
    window.close();
}

#[test]
fn composition_visual_follows_the_corner_radius() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let window_impl = window_mock(&services, WindowTransparencyLevel::none());
    let window = shown_window(&services, &window_impl, &target);

    target.set_corner_radius(CornerRadius::uniform(8.0));
    assert_eq!(CornerRadius::uniform(8.0), acrylic_visual(&target).corner_radius());

    // Also without a material: the visual then gets the default one.
    assert_eq!(Color::default(), acrylic_visual(&target).material().tint_color());

    services.run_jobs();
    window.close();
}

#[test]
fn material_is_compensated_for_the_transparency_level_of_the_top_level() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let material = material(Color::from_rgb(10, 20, 30));
    target.set_material(material.clone());
    assert_eq!(0.0, material.platform_transparency_compensation_level());

    let window_impl = window_mock(&services, WindowTransparencyLevel::blur());
    let window = shown_window(&services, &window_impl, &target);
    assert_eq!(WindowTransparencyLevel::blur(), window.actual_transparency_level());

    // The level the top level had when the border was attached.
    assert_eq!(LEVELS.blur_level(), material.platform_transparency_compensation_level());
    assert_visual_material(&target, &material);

    let cases = [
        (WindowTransparencyLevel::acrylic_blur(), LEVELS.acrylic_blur_level()),
        (WindowTransparencyLevel::transparent(), LEVELS.transparent_level()),
        (WindowTransparencyLevel::blur(), LEVELS.blur_level()),
        (WindowTransparencyLevel::none(), LEVELS.transparent_level()),
        (WindowTransparencyLevel::acrylic_blur(), LEVELS.acrylic_blur_level()),
        // No compensation is defined for this level: the last one stays.
        (WindowTransparencyLevel::mica(), LEVELS.acrylic_blur_level()),
    ];
    for (level, expected) in cases {
        change_transparency_level(&window_impl, level);

        assert_eq!(level, window.actual_transparency_level());
        assert_eq!(expected, material.platform_transparency_compensation_level(), "{level}");
        // The compensation changes the material color the visual draws.
        assert_visual_material(&target, &material);
    }

    services.run_jobs();
    window.close();
}

#[test]
fn a_material_set_later_is_compensated_at_the_next_level_change() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let window_impl = window_mock(&services, WindowTransparencyLevel::blur());
    let window = shown_window(&services, &window_impl, &target);

    let material = material(Color::from_rgb(10, 20, 30));
    target.set_material(material.clone());
    assert_eq!(0.0, material.platform_transparency_compensation_level());
    assert_visual_material(&target, &material);

    change_transparency_level(&window_impl, WindowTransparencyLevel::acrylic_blur());
    assert_eq!(LEVELS.acrylic_blur_level(), material.platform_transparency_compensation_level());
    assert_visual_material(&target, &material);

    services.run_jobs();
    window.close();
}

#[test]
fn detaching_releases_the_subscriptions() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let target = ExperimentalAcrylicBorder::new();
    let material = material(Color::from_rgb(10, 20, 30));
    target.set_material(material.clone());
    let window_impl = window_mock(&services, WindowTransparencyLevel::blur());
    let window = shown_window(&services, &window_impl, &target);
    assert_eq!(1, material.property_changed_subscriber_count());
    assert_eq!(LEVELS.blur_level(), material.platform_transparency_compensation_level());

    window.set_content(None);
    services.run_jobs();

    assert!(target.composition_visual().is_none());

    // The top level is not followed any more.
    change_transparency_level(&window_impl, WindowTransparencyLevel::acrylic_blur());
    assert_eq!(LEVELS.blur_level(), material.platform_transparency_compensation_level());

    // As in the reference, the border still has its composition visual
    // when it is told that it was detached, so it keeps listening to the
    // material until the material changes once more.
    assert_eq!(1, material.property_changed_subscriber_count());
    material.set_tint_color(Color::from_rgb(40, 50, 60));
    assert_eq!(0, material.property_changed_subscriber_count());
    material.set_tint_color(Color::from_rgb(41, 51, 61));
    assert_eq!(0, material.property_changed_subscriber_count());

    // Attached again, the border follows both again.
    window.set_content(Some(Control::boxed(target.clone())));
    services.run_jobs();
    assert_eq!(1, material.property_changed_subscriber_count());
    assert_eq!(LEVELS.acrylic_blur_level(), material.platform_transparency_compensation_level());
    assert_visual_material(&target, &material);

    window.close();
}

#[test]
fn material_is_not_compensated_without_a_top_level() {
    let _scope = Dispatcher::unit_test_scope();
    let target = ExperimentalAcrylicBorder::new();
    let material = material(Color::from_rgb(10, 20, 30));
    target.set_material(material.clone());

    let _root = TestRoot::with_child(target.clone());

    assert_eq!(1.0, material.platform_transparency_compensation_level());
}
