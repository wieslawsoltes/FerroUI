//! Tests of the WinUI shadows and lights of phase 5 (not from upstream): the
//! mask and the source policy of a drop shadow, the targets and exclusions of
//! a light, the light math and the light map drawn over a lit visual. A child
//! module of the compositor tests, whose fixture renders the composition
//! visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::animation::easings::LinearEasing;
use crate::media::imaging::Bitmap;
use crate::media::Colors;
use crate::platform::{AlphaFormat, LtrbRect, PixelFormats};
use crate::rendering::composition::server::{
    distant_light_incidence, light_map_of, point_light_incidence, spot_light_cone, IServerCompositionLight,
    LightAttenuation, LightColor, LightMap, LightSource, ServerAmbientLight, ServerPointLight,
};
use crate::rendering::composition::{
    AmbientLight, CompositionDropShadowSourcePolicy, CompositionVisual, ElementComposition, ICompositionObjectAnimations,
    ICompositionSurface, LoadedImageSurface, PointLight, SpotLight,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{Matrix, PixelSize, Point, Rect, Vector, Vector3D};
use std::rc::Rc;
use std::time::Duration;

/// Shows `visual` as the child composition visual of the canvas and renders
/// a frame.
fn show(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) {
    ElementComposition::set_element_child_visual(&s.canvas, Some(visual.clone()));
    s.run_jobs();
}

/// Renders frames until `duration` of server time has passed, then commits
/// and renders once more.
fn run_for(s: &CompositorCanvas, duration: Duration) {
    let started = s.compositor.server().server_now();
    while s.compositor.server().server_now() < started + duration {
        s.render_loop.tick();
    }
    s.run_jobs();
}

fn has_entry(entries: &[String], entry: &str) -> bool {
    entries.iter().any(|e| e == entry)
}

fn position(entries: &[String], entry: &str) -> usize {
    entries.iter().position(|e| e == entry).unwrap_or_else(|| panic!("{entry} is drawn: {entries:?}"))
}

fn position_starting(entries: &[String], prefix: &str) -> usize {
    entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix} is drawn: {entries:?}"))
}

/// A 20 x 10 pixel bitmap at 96 DPI.
fn test_bitmap() -> Bitmap {
    Bitmap::from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &[0u8; 20 * 10 * 4],
        PixelSize::new(20, 10),
        Vector::new(96.0, 96.0),
        20 * 4,
    )
}

/// A blue sprite visual of the given size.
fn blue_sprite(s: &CompositorCanvas, width: f64, height: f64) -> crate::rendering::composition::SpriteVisual {
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(width, height));
    sprite.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    sprite
}

/// Renders the server visual of `visual` into a recording drawing context
/// with effects.
fn render_with_effects(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    s.server_object(visual.server()).render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

// --- drop shadow masks and source policies -----------------------------------------

#[test]
fn a_masked_shadow_is_the_shadow_of_its_mask_through_an_effect() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let image = LoadedImageSurface::from_bitmap(&s.compositor, &test_bitmap());
    let surface: Rc<dyn ICompositionSurface> = image.clone();
    let mask = s.compositor.create_surface_brush_with_surface(surface);
    let shadow = s.compositor.create_drop_shadow();
    shadow.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    shadow.set_mask(Some((*mask).clone()));
    assert!(shadow.mask().is_some());
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    // The drawing context of the canvas has no effects: the box shadow of
    // the rectangle, then the brush.
    let entries = log.entries();
    let box_shadow = position(&entries, "DrawRectangle none none 0, 0, 100, 50 shadows=1");
    let brush = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    assert!(box_shadow < brush, "{entries:?}");

    // With effects: the mask through a drop shadow effect in a layer, the
    // mask removed again with destination-out blending, then the brush.
    let entries = render_with_effects(&s, &sprite);
    let layer = position_starting(&entries, "PushLayer");
    let effect = position_starting(&entries, "PushEffect");
    let pop_effect = position(&entries, "PopEffect");
    let options = position_starting(&entries, "PushRenderOptions");
    let pop_layer = position(&entries, "PopLayer");
    let brush = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    assert!(
        layer < effect && effect < pop_effect && pop_effect < options && options < pop_layer && pop_layer < brush,
        "{entries:?}"
    );
    let bitmaps = entries[effect..pop_layer].iter().filter(|e| e.starts_with("DrawBitmap")).count();
    assert_eq!(2, bitmaps, "the mask is painted for its shadow and removed again: {entries:?}");
    assert!(!entries.iter().any(|e| e.ends_with("shadows=1")), "{entries:?}");

    // A change of the mask brush redraws the visual.
    log.clear();
    mask.set_offset(Vector::new(5.0, 0.0));
    s.run_jobs();
    assert!(log.count("DrawRectangle Blue none 0, 0, 100, 50") > 0, "{:?}", log.entries());
}

#[test]
fn a_color_mask_is_the_opacity_mask_of_the_box_shadow() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_mask(Some((*s.compositor.create_color_brush_with_color(Colors::BLACK)).clone()));
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    let entries = log.entries();
    let push = position_starting(&entries, "PushOpacityMask");
    let draw = position(&entries, "DrawRectangle none none 0, 0, 100, 50 shadows=1");
    let pop = position(&entries, "PopOpacityMask");
    let brush = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    assert!(push < draw && draw < pop && pop < brush, "{entries:?}");

    // The source policy `InheritFromVisualContent` wins over the mask.
    log.clear();
    shadow.set_source_policy(CompositionDropShadowSourcePolicy::InheritFromVisualContent);
    s.run_jobs();
    assert!(!log.entries().iter().any(|e| e.starts_with("PushOpacityMask")), "{:?}", log.entries());
}

#[test]
fn a_shadow_that_inherits_from_the_content_follows_the_brush() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_source_policy(CompositionDropShadowSourcePolicy::InheritFromVisualContent);
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    // Without effects: the box shadow of the rectangle.
    assert!(has_entry(&log.entries(), "DrawRectangle none none 0, 0, 100, 50 shadows=1"), "{:?}", log.entries());

    // With effects: the brush drawn through a drop shadow effect, once.
    let entries = render_with_effects(&s, &sprite);
    let push = position_starting(&entries, "PushEffect");
    let brush = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    let pop = position(&entries, "PopEffect");
    assert!(push < brush && brush < pop, "{entries:?}");
    assert_eq!(1, entries.iter().filter(|e| e.starts_with("DrawRectangle Blue")).count(), "{entries:?}");
    assert!(!entries.iter().any(|e| e.ends_with("shadows=1")), "{entries:?}");
}

#[test]
fn the_masked_shadow_of_a_layer_visual_is_drawn_under_its_layer() {
    let s = CompositorCanvas::new();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_mask(Some((*s.compositor.create_color_brush_with_color(Colors::BLACK)).clone()));
    let layer = s.compositor.create_layer_visual();
    layer.set_size(Vector::new(100.0, 50.0));
    layer.set_shadow(Some(shadow.clone()));
    layer.children().insert_at_top((*blue_sprite(&s, 30.0, 20.0)).clone());
    show(&s, &layer);

    // Even with effects the children are composed into a plain layer, over
    // the shadow of the mask.
    let entries = render_with_effects(&s, &layer);
    let mask = position_starting(&entries, "PushOpacityMask");
    let draw = position(&entries, "DrawRectangle none none 0, 0, 100, 50 shadows=1");
    let push = position_starting(&entries, "PushLayer");
    let child = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    assert!(mask < draw && draw < push && push < child, "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushEffect")), "{entries:?}");
}

// --- the light math ----------------------------------------------------------------

#[test]
fn a_point_light_is_strongest_under_it_and_falls_off_to_the_edge() {
    let position = Vector3D::new(50.0, 50.0, 100.0);
    assert_eq!((1.0, 100.0), point_light_incidence(position, Point::new(50.0, 50.0)));
    let (edge, distance) = point_light_incidence(position, Point::new(150.0, 50.0));
    assert!((edge - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9, "{edge}");
    assert!((distance - 100.0 * std::f64::consts::SQRT_2).abs() < 1e-9, "{distance}");
    // A light in or below the plane of the visual lights nothing.
    assert_eq!(0.0, point_light_incidence(Vector3D::new(0.0, 0.0, -10.0), Point::new(0.0, 0.0)).0);
    assert_eq!(0.0, point_light_incidence(Vector3D::new(0.0, 0.0, 0.0), Point::new(10.0, 0.0)).0);

    let source = LightSource::Point { color: LightColor::WHITE, position, attenuation: LightAttenuation::NONE };
    assert_eq!(LightColor::WHITE, source.light_at(Point::new(50.0, 50.0)));
    let edge = source.light_at(Point::new(150.0, 50.0));
    assert!(edge.r > 0.65 && edge.r < 0.75 && edge.r == edge.g && edge.g == edge.b, "{edge:?}");

    // The light map of a visual under the light: full in the middle, dark
    // towards the corners.
    let mut map = LightMap::new();
    map.add(
        LightSource::Point {
            color: LightColor::WHITE,
            position: Vector3D::new(50.0, 50.0, 50.0),
            attenuation: LightAttenuation::NONE,
        },
        Matrix::IDENTITY,
    );
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let size = LightMap::size_for(rect);
    assert_eq!(PixelSize::new(25, 25), size);
    let pixels = map.pixels(rect, size).expect("the corners are darker");
    let alpha = |column: i32, row: i32| pixels[((row * size.width + column) * 4 + 3) as usize];
    assert_eq!(0, alpha(12, 12), "the centre is fully lit");
    assert!(alpha(0, 0) > 90, "a corner is darker: {}", alpha(0, 0));
    assert_eq!(alpha(0, 0), alpha(24, 24));
}

#[test]
fn attenuation_cutoffs_and_cones() {
    let attenuation =
        LightAttenuation { constant: 1.0, linear: 0.01, quadratic: 0.0, min_cutoff: 0.0, max_cutoff: 120.0 };
    assert_eq!(0.5, attenuation.factor(100.0));
    assert_eq!(0.0, attenuation.factor(121.0));
    let near = LightAttenuation { min_cutoff: 50.0, ..attenuation };
    assert_eq!(attenuation.factor(50.0), near.factor(10.0));
    assert_eq!(1.0, LightAttenuation::NONE.factor(1.0e6));

    // A distant light straight at the visual lights it fully; from behind
    // not at all.
    assert_eq!(1.0, distant_light_incidence(Vector3D::new(0.0, 0.0, -1.0)));
    assert_eq!(0.0, distant_light_incidence(Vector3D::new(0.0, 0.0, 1.0)));
    let slanted = distant_light_incidence(Vector3D::new(1.0, 0.0, -1.0));
    assert!((slanted - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);

    // A spot light 100 above the origin, straight down, inner cone 0.1 rad,
    // outer cone 0.5 rad.
    let position = Vector3D::new(0.0, 0.0, 100.0);
    let down = Vector3D::new(0.0, 0.0, -1.0);
    assert_eq!(1.0, spot_light_cone(position, down, 0.1, 0.5, Point::new(0.0, 0.0)));
    assert_eq!(0.0, spot_light_cone(position, down, 0.1, 0.5, Point::new(100.0, 0.0)));
    let between = spot_light_cone(position, down, 0.1, 0.5, Point::new(100.0 * 0.3f64.tan(), 0.0));
    assert!(between > 0.0 && between < 1.0, "{between}");

    // The pixels of the light map: black for no light, nothing for full
    // light, the grey part as alpha and the rest of a colour as colour.
    assert_eq!([0, 0, 0, 255], LightColor::BLACK.atop_pixel());
    assert_eq!([0, 0, 0, 0], LightColor::WHITE.atop_pixel());
    assert_eq!([0, 0, 0, 128], LightColor::new(0.5, 0.5, 0.5).atop_pixel());
    assert_eq!([128, 0, 0, 255], LightColor::new(0.5, 0.0, 0.0).atop_pixel());
    // Lights add and are clamped.
    assert_eq!(LightColor::WHITE, LightColor::new(0.75, 0.5, 2.0).add(LightColor::new(0.5, 0.5, 0.0)).clamped());
    assert_eq!(LightColor::new(0.5, 0.0, 0.0), LightColor::from_color(Colors::RED, 0.5));
}

// --- lights on visuals -----------------------------------------------------------------

#[test]
fn a_light_keeps_its_targets_and_exclusions() {
    let s = CompositorCanvas::new();

    let light = s.compositor.create_ambient_light();
    assert_eq!("AmbientLight", light.type_name());
    assert!(light.is_enabled());
    assert_eq!((Colors::WHITE, 1.0), (light.color(), light.intensity()));
    assert!(AmbientLight::from_light(&light).is_some());
    assert!(PointLight::from_light(&light).is_none());

    let root = s.compositor.create_container_visual();
    root.set_size(Vector::new(200.0, 100.0));
    let child = blue_sprite(&s, 30.0, 20.0);
    let excluded = blue_sprite(&s, 30.0, 20.0);
    let grandchild = blue_sprite(&s, 10.0, 10.0);
    root.children().insert_at_top((*child).clone());
    root.children().insert_at_top((*excluded).clone());
    excluded.children().insert_at_top((*grandchild).clone());

    light.targets().add(root.clone());
    light.targets().add(root.clone());
    assert_eq!(1, light.targets().count(), "a visual is added once");
    assert!(light.targets().contains(&root));
    light.exclusions_from_targets().add((*excluded).clone());
    assert_eq!(1, light.exclusions_from_targets().count());
    show(&s, &root);

    let server_light =
        s.compositor.server().get::<ServerAmbientLight>(light.server()).expect("the server light exists");
    let server_root = s.server_object(root.server());
    let target_of = |visual: &Rc<CompositionVisual>| server_light.target_of(&s.server_object(visual.server()));
    assert!(target_of(&root).is_some_and(|target| Rc::ptr_eq(&target, &server_root)));
    assert!(target_of(&child).is_some_and(|target| Rc::ptr_eq(&target, &server_root)));
    assert!(target_of(&excluded).is_none(), "an exclusion is not lit");
    assert!(target_of(&grandchild).is_none(), "the subtree of an exclusion is not lit");

    // Removing the exclusion lights its subtree.
    assert!(light.exclusions_from_targets().remove(&excluded));
    assert!(!light.exclusions_from_targets().remove(&excluded));
    s.run_jobs();
    assert!(target_of(&grandchild).is_some());

    // A disabled light lights nothing.
    light.set_is_enabled(false);
    s.run_jobs();
    assert!(light_map_of(&s.server_object(child.server())).is_none());
    light.set_is_enabled(true);
    s.run_jobs();
    assert!(light_map_of(&s.server_object(child.server())).is_some());

    // Without targets the light reaches nothing.
    light.targets().remove_all();
    assert_eq!(0, light.targets().count());
    s.run_jobs();
    assert!(target_of(&child).is_none());
    assert!(light_map_of(&s.server_object(child.server())).is_none());
}

#[test]
fn an_ambient_light_of_no_intensity_darkens_its_target() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let light = s.compositor.create_ambient_light();
    light.set_intensity(0.0);
    let sprite = blue_sprite(&s, 30.0, 20.0);
    light.targets().add((*sprite).clone());
    show(&s, &sprite);

    // The content in a layer, then the light map over it: an 8 x 5 cell map
    // of the 30 x 20 visual, with source-atop blending.
    let entries = log.entries();
    let push = position(&entries, "PushLayer 0, 0, 30, 20");
    let content = push + position(&entries[push..], "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    let options = content + position_starting(&entries[content..], "PushRenderOptions");
    let map = options + position(&entries[options..], "DrawBitmap 1 0, 0, 8, 5 0, 0, 30, 20");
    let pop = map + position(&entries[map..], "PopRenderOptions");
    assert!(entries[pop..].iter().any(|e| e == "PopLayer"), "{entries:?}");

    // At full intensity the light changes nothing: no layer.
    log.clear();
    light.set_intensity(1.0);
    s.run_jobs();
    let entries = log.entries();
    assert!(has_entry(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushLayer") || e.starts_with("DrawBitmap")), "{entries:?}");

    // A disposed light lights nothing.
    light.set_intensity(0.0);
    s.run_jobs();
    log.clear();
    light.dispose();
    s.run_jobs();
    let entries = log.entries();
    assert!(has_entry(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushLayer")), "{entries:?}");
}

#[test]
fn a_point_light_is_placed_in_its_coordinate_space() {
    let s = CompositorCanvas::new();

    let root = s.compositor.create_container_visual();
    root.set_size(Vector::new(200.0, 100.0));
    let child = blue_sprite(&s, 100.0, 100.0);
    child.set_offset(Vector3D::new(40.0, 0.0, 0.0));
    root.children().insert_at_top((*child).clone());

    let light = s.compositor.create_point_light();
    assert_eq!((1.0, 0.0, 0.0), (light.constant_attenuation(), light.linear_attenuation(), light.quadratic_attenuation()));
    assert_eq!(f32::MAX, light.max_attenuation_cutoff());
    light.set_offset(Vector3D::new(50.0, 50.0, 10.0));
    light.set_coordinate_space(Some(root.clone()));
    assert!(light.coordinate_space().is_some_and(|space| Rc::ptr_eq(&space, &root)));
    light.targets().add((*child).clone());
    show(&s, &root);

    // (10, 50) of the child is (50, 50) of the root, under the light.
    let map = light_map_of(&s.server_object(child.server())).expect("the light reaches the child");
    let under = map.light_at(Point::new(10.0, 50.0));
    assert!((under.r - 1.0).abs() < 1e-9 && (under.b - 1.0).abs() < 1e-9, "{under:?}");
    assert!(map.light_at(Point::new(90.0, 50.0)).r < 0.2);

    // Without a coordinate space the light is in the space of the target.
    light.set_coordinate_space(None);
    s.run_jobs();
    let map = light_map_of(&s.server_object(child.server())).expect("the light reaches the child");
    assert!((map.light_at(Point::new(50.0, 50.0)).r - 1.0).abs() < 1e-9);

    // A spot light has its own defaults and views of its angles.
    let spot = s.compositor.create_spot_light();
    assert_eq!(Vector3D::new(0.0, 0.0, -1.0), spot.direction());
    assert!((spot.outer_cone_angle_in_degrees() - 45.0).abs() < 1e-4);
    spot.set_inner_cone_angle_in_degrees(10.0);
    assert!((spot.inner_cone_angle() - 10f32.to_radians()).abs() < 1e-6);
    assert!(SpotLight::from_light(&spot).is_some());
}

#[test]
fn animating_the_offset_of_a_light_draws_its_target_again() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let light = s.compositor.create_point_light();
    light.set_offset(Vector3D::new(0.0, 0.0, 10.0));
    let sprite = blue_sprite(&s, 100.0, 100.0);
    light.targets().add((*sprite).clone());
    show(&s, &sprite);
    assert!(has_entry(&log.entries(), "PushLayer 0, 0, 100, 100"), "{:?}", log.entries());

    let server_light = s.compositor.server().get::<ServerPointLight>(light.server()).expect("the server light exists");
    let animation = s.compositor.create_vector3d_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, Vector3D::new(100.0, 100.0, 10.0), Rc::new(LinearEasing::new()));
    animation.set_duration(Duration::from_millis(50));
    light.start_animation("Offset", &*animation);
    // The animation is committed before its time is measured.
    s.run_jobs();
    log.clear();
    run_for(&s, Duration::from_millis(150));
    assert_eq!(Vector3D::new(100.0, 100.0, 10.0), server_light.props().offset());
    // The light map is drawn again while the light moves.
    assert!(log.count("DrawBitmap 1 0, 0, 25, 25 0, 0, 100, 100") >= 2, "{:?}", log.entries());
}
