//! Tests of the drop shadows of phases 5 and 7 (not from upstream): the
//! shadow of the alpha of a mask or of the content of a visual, the same
//! operations on every drawing context with effects, and the Z of the
//! offset. A child module of the compositor tests, whose fixture renders the
//! composition visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::media::imaging::Bitmap;
use crate::media::Colors;
use crate::numerics::{Quaternion, Vector3};
use crate::platform::{AlphaFormat, LtrbRect, PixelFormats};
use crate::rendering::composition::server::ServerDropShadow;
use crate::rendering::composition::{
    CompositionDropShadowSourcePolicy, CompositionVisual, ElementComposition, ICompositionSurface, LoadedImageSurface,
    SpriteVisual,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{PixelSize, Vector, Vector3D};
use std::rc::Rc;

/// Shows `visual` as the child composition visual of the canvas and renders
/// a frame.
fn show(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) {
    ElementComposition::set_element_child_visual(&s.canvas, Some(visual.clone()));
    s.run_jobs();
}

fn position(entries: &[String], entry: &str) -> usize {
    entries.iter().position(|e| e == entry).unwrap_or_else(|| panic!("{entry} is drawn: {entries:?}"))
}

fn position_starting(entries: &[String], prefix: &str) -> usize {
    entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix} is drawn: {entries:?}"))
}

fn count_starting(entries: &[String], prefix: &str) -> usize {
    entries.iter().filter(|e| e.starts_with(prefix)).count()
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
fn blue_sprite(s: &CompositorCanvas, width: f64, height: f64) -> SpriteVisual {
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(width, height));
    sprite.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    sprite
}

/// Renders the server visual of `visual` into a recording drawing context
/// with effects, which records the effects and the render options.
fn render_with_effects(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    context.log_render_options = true;
    s.server_object(visual.server()).render(&mut context, crate::platform::LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

/// The entries of the shadow of an alpha: from its blur to the end of the
/// blur. Asserts the order of the operations: the blur, the layer, the
/// source, the bitmap of the colour drawn with source-in blending, the end of
/// the layer, the end of the blur.
fn shadow_of_alpha(entries: &[String]) -> &[String] {
    let blur = position_starting(entries, "PushEffect");
    let layer = blur + position_starting(&entries[blur..], "PushLayer");
    let options = layer + position_starting(&entries[layer..], "PushRenderOptions");
    let pop_layer = options + position(&entries[options..], "PopLayer");
    let pop_blur = pop_layer + position(&entries[pop_layer..], "PopEffect");
    assert!(entries[blur].contains("blur("), "the shadow is blurred: {entries:?}");
    assert!(entries[options].contains("SourceIn"), "the colour is drawn with source-in blending: {entries:?}");
    assert_eq!(
        1,
        count_starting(&entries[options..pop_layer], "DrawBitmap 1 1, 1, 2, 2"),
        "the bitmap of the colour is drawn once over the source: {entries:?}"
    );
    assert!(!entries.iter().any(|e| e.contains("DestinationOut")), "nothing is removed: {entries:?}");
    &entries[blur..=pop_blur]
}

// --- the shadow of an alpha --------------------------------------------------------

#[test]
fn a_masked_shadow_is_the_alpha_of_its_mask_coloured_moved_and_blurred() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let image = LoadedImageSurface::from_bitmap(&s.compositor, &test_bitmap());
    let surface: Rc<dyn ICompositionSurface> = image.clone();
    let mask = s.compositor.create_surface_brush_with_surface(surface);
    let shadow = s.compositor.create_drop_shadow();
    shadow.set_offset(Vector3D::new(5.0, 6.0, 0.0));
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

    // With effects: the alpha of the mask in the colour of the shadow, at its
    // offset, blurred by its radius (9 by default), and over it the brush.
    let entries = render_with_effects(&s, &sprite);
    let shadow_entries = shadow_of_alpha(&entries);
    assert_eq!("PushEffect -4, -3, 118, 68 blur(9)", shadow_entries[0], "{entries:?}");
    assert!(shadow_entries.iter().any(|e| e == "SetTransform { {M11:1 M12:0} {M21:0 M22:1} {M31:5 M32:6} }"), "{entries:?}");
    // The mask (a bitmap) is painted once, for its shadow only.
    assert_eq!(2, count_starting(shadow_entries, "DrawBitmap"), "{entries:?}");
    let brush = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    assert!(brush > position(&entries, "PopEffect"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.ends_with("shadows=1")), "{entries:?}");

    // A change of the mask brush redraws the visual.
    log.clear();
    mask.set_offset(Vector::new(5.0, 0.0));
    s.run_jobs();
    assert!(log.count("DrawRectangle Blue none 0, 0, 100, 50") > 0, "{:?}", log.entries());
}

#[test]
fn a_color_mask_casts_the_shadow_of_its_alpha_too() {
    let s = CompositorCanvas::new();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_mask(Some((*s.compositor.create_color_brush_with_color(Colors::BLACK)).clone()));
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    let entries = render_with_effects(&s, &sprite);
    let shadow_entries = shadow_of_alpha(&entries);
    assert!(shadow_entries.iter().any(|e| e == "DrawRectangle Black none 0, 0, 100, 50 shadows=0"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushOpacityMask")), "{entries:?}");

    // The source policy `InheritFromVisualContent` wins over the mask: the
    // shadow is the one of the brush.
    shadow.set_source_policy(CompositionDropShadowSourcePolicy::InheritFromVisualContent);
    s.run_jobs();
    let entries = render_with_effects(&s, &sprite);
    let shadow_entries = shadow_of_alpha(&entries);
    assert!(shadow_entries.iter().any(|e| e == "DrawRectangle Blue none 0, 0, 100, 50 shadows=0"), "{entries:?}");
    assert!(!shadow_entries.iter().any(|e| e.starts_with("DrawRectangle Black")), "{entries:?}");
}

#[test]
fn a_shadow_that_inherits_from_the_content_is_the_alpha_of_the_brush_under_the_brush() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_source_policy(CompositionDropShadowSourcePolicy::InheritFromVisualContent);
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    // Without effects: the box shadow of the rectangle.
    assert!(
        log.entries().iter().any(|e| e == "DrawRectangle none none 0, 0, 100, 50 shadows=1"),
        "{:?}",
        log.entries()
    );

    // With effects: the brush for its shadow, then the brush itself.
    let entries = render_with_effects(&s, &sprite);
    let shadow_entries = shadow_of_alpha(&entries);
    assert_eq!(1, count_starting(shadow_entries, "DrawRectangle Blue"), "{entries:?}");
    assert_eq!(2, count_starting(&entries, "DrawRectangle Blue"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.ends_with("shadows=1")), "{entries:?}");

    // A sprite without a brush has no content to cast a shadow.
    sprite.set_brush(None);
    s.run_jobs();
    let entries = render_with_effects(&s, &sprite);
    assert!(!entries.iter().any(|e| e.starts_with("PushEffect") || e.ends_with("shadows=1")), "{entries:?}");
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
    // the shadow of the alpha of the mask.
    let entries = render_with_effects(&s, &layer);
    let shadow_entries = shadow_of_alpha(&entries).len();
    let blur = position_starting(&entries, "PushEffect");
    let child = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    assert!(blur + shadow_entries <= child, "{entries:?}");
    assert_eq!(1, count_starting(&entries, "PushEffect"), "{entries:?}");
}

#[test]
fn an_invisible_shadow_draws_nothing() {
    let s = CompositorCanvas::new();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_opacity(0.0);
    shadow.set_mask(Some((*s.compositor.create_color_brush_with_color(Colors::BLACK)).clone()));
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    let entries = render_with_effects(&s, &sprite);
    assert!(!entries.iter().any(|e| e.starts_with("PushEffect") || e.starts_with("PushLayer")), "{entries:?}");
    assert_eq!(1, count_starting(&entries, "DrawRectangle"), "{entries:?}");
}

#[test]
fn the_color_of_the_shadow_carries_its_opacity() {
    let s = CompositorCanvas::new();
    let shadow = s.compositor.create_drop_shadow();
    shadow.set_color(crate::media::Color::from_argb(200, 10, 20, 30));
    shadow.set_opacity(0.5);
    let sprite = blue_sprite(&s, 10.0, 10.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    let server = s.compositor.server().get::<ServerDropShadow>(shadow.server()).expect("the server shadow exists");
    assert_eq!(crate::media::Color::from_argb(100, 10, 20, 30), server.effective_color());
}

// --- the Z of the offset -----------------------------------------------------------

#[test]
fn the_z_of_the_offset_moves_nothing_in_the_plane() {
    let s = CompositorCanvas::new();
    let shadow = s.compositor.create_drop_shadow();
    shadow.set_offset(Vector3D::new(2.0, 3.0, 40.0));
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    let server_shadow = s.compositor.server().get::<ServerDropShadow>(shadow.server()).expect("the server shadow exists");
    let server = s.server_object(sprite.server());
    assert_eq!(Vector::new(2.0, 3.0), server_shadow.plane_offset(Some(&server)));
    // The bounds of the shadow: offset by (2, 3), grown by the blur of 9.
    assert_eq!(Some(LtrbRect::new(-7.0, -6.0, 111.0, 62.0)), server.sub_tree_bounds());
}

#[test]
fn the_z_of_the_offset_moves_the_shadow_of_a_visual_turned_out_of_its_plane() {
    let s = CompositorCanvas::new();
    let shadow = s.compositor.create_drop_shadow();
    shadow.set_offset(Vector3D::new(0.0, 0.0, 10.0));
    let sprite = blue_sprite(&s, 100.0, 50.0);
    sprite.set_shadow(Some(shadow.clone()));
    // Turned by 60 degrees about the X axis: a point at the depth z is drawn
    // z sin 60 higher, which is z tan 60 in the space of the visual, whose
    // height is shortened by cos 60.
    let angle = std::f32::consts::PI / 3.0;
    sprite.set_orientation(Quaternion::create_from_axis_angle(Vector3::new(1.0, 0.0, 0.0), angle));
    show(&s, &sprite);

    let server_shadow = s.compositor.server().get::<ServerDropShadow>(shadow.server()).expect("the server shadow exists");
    let server = s.server_object(sprite.server());
    let offset = server_shadow.plane_offset(Some(&server));
    assert!(offset.x.abs() < 1e-4, "{offset:?}");
    assert!((offset.y - (-10.0 * (angle as f64).tan())).abs() < 1e-3, "{offset:?}");

    // The Z is part of the offset the shadow is drawn at.
    let entries = render_with_effects(&s, &sprite);
    let box_shadow = entries.iter().find(|e| e.ends_with("shadows=1"));
    assert!(box_shadow.is_some(), "{entries:?}");
    let bounds = server.compute_own_content_bounds().expect("the sprite has bounds");
    assert!(bounds.top < -10.0 * (angle as f64).tan() - 8.0, "{bounds:?}");

    // A Z of 0 leaves the shadow where X and Y put it.
    shadow.set_offset(Vector3D::new(4.0, 0.0, 0.0));
    s.run_jobs();
    assert_eq!(Vector::new(4.0, 0.0), server_shadow.plane_offset(Some(&server)));
}
