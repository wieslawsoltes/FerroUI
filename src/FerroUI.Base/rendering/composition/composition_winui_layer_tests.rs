//! Tests of the WinUI visuals and brushes of phase 2 (not from upstream):
//! layer and redirect visuals, nine-grid and mask brushes, the WinUI members
//! of the gradient brushes and the remaining members of `Visual`. A child
//! module of the compositor tests, whose fixture renders the composition
//! visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::media::imaging::Bitmap;
use crate::media::{Colors, GradientSpreadMethod, IBrush};
use crate::platform::{AlphaFormat, LtrbRect, PixelFormats};
use crate::rendering::composition::hit_testing::PointCompositionHitTester;
use crate::rendering::composition::server::{
    nine_grid_rects, ServerCompositionLinearGradientBrush, ServerLayerVisual, ServerRedirectVisual, NINE_GRID_CENTER,
};
use crate::rendering::composition::{
    AsCompositionObject, CompositionBackfaceVisibility, CompositionBorderMode, CompositionCompositeMode,
    CompositionGradientExtendMode, CompositionMappingMode, CompositionVisual, ElementComposition, ICompositionSurface,
    LayerVisual, LoadedImageSurface, RedirectVisual,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{Matrix, PixelSize, Point, Rect, RelativePoint, RelativeUnit, Vector, Vector3D};
use std::rc::Rc;

/// Shows `visual` as the child composition visual of the canvas and renders
/// a frame.
fn show(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) {
    ElementComposition::set_element_child_visual(&s.canvas, Some(visual.clone()));
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

// --- layer visual ------------------------------------------------------------------

#[test]
fn a_layer_visual_composes_its_children_into_a_layer() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let layer = s.compositor.create_layer_visual();
    layer.set_size(Vector::new(100.0, 50.0));
    let child = blue_sprite(&s, 30.0, 20.0);
    layer.children().insert_at_top((*child).clone());
    show(&s, &layer);

    let entries = log.entries();
    let push = position(&entries, "PushLayer 0, 0, 30, 20");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    let pop = entries[draw..].iter().position(|e| e == "PopLayer").map(|index| index + draw);
    assert!(push < draw && pop.is_some(), "the child is drawn into the layer: {entries:?}");

    assert!(!layer.clip_to_bounds(), "a layer visual does not clip to its bounds");
    assert!(LayerVisual::from_visual(&layer).is_some());
    assert!(LayerVisual::from_visual(&child).is_none());
    assert_eq!("LayerVisual", AsCompositionObject::composition_type_name(&**layer));
    let server = s.server_object(layer.server());
    assert!(server.content_as::<ServerLayerVisual>().is_some());
}

#[test]
fn the_shadow_of_a_layer_visual_without_effects_is_the_box_shadow_of_its_rectangle() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let shadow = s.compositor.create_drop_shadow();
    shadow.set_blur_radius(10.0);
    shadow.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    let layer = s.compositor.create_layer_visual();
    layer.set_size(Vector::new(100.0, 50.0));
    layer.set_shadow(Some(shadow.clone()));
    assert!(Rc::ptr_eq(&layer.shadow().expect("the shadow is set"), &shadow));
    layer.children().insert_at_top((*blue_sprite(&s, 30.0, 20.0)).clone());
    show(&s, &layer);

    // The mock drawing context has no effects: the box shadow of the
    // rectangle, then the layer.
    let entries = log.entries();
    let shadow_draw = position(&entries, "DrawRectangle none none 0, 0, 100, 50 shadows=1");
    let push = position_starting(&entries, "PushLayer");
    assert!(shadow_draw < push, "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushEffect")), "{entries:?}");

    // The bounds include the area of the shadow.
    let server = s.server_object(layer.server());
    let bounds = server.sub_tree_bounds().expect("the layer has bounds");
    assert!(bounds.left <= -5.0 && bounds.right >= 115.0, "{bounds:?}");

    // A drawing context with effects draws the shadow of the composed
    // children through a drop shadow effect instead.
    let effect_log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(effect_log.clone());
    context.supports_effects = true;
    server.render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    let entries = effect_log.entries();
    let push = position_starting(&entries, "PushEffect");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    let pop = position(&entries, "PopEffect");
    assert!(push < draw && draw < pop, "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushLayer") || e.ends_with("shadows=1")), "{entries:?}");
}

// --- redirect visual ---------------------------------------------------------------

#[test]
fn a_redirect_visual_draws_its_source_in_its_own_place() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let root = s.compositor.create_container_visual();
    root.set_size(Vector::new(300.0, 100.0));
    let source = blue_sprite(&s, 30.0, 20.0);
    source.set_offset(Vector3D::new(10.0, 10.0, 0.0));
    let redirect = s.compositor.create_redirect_visual_with_source_visual(Some((*source).clone()));
    redirect.set_offset(Vector3D::new(200.0, 0.0, 0.0));
    root.children().insert_at_top((*source).clone());
    root.children().insert_at_top((*redirect).clone());
    show(&s, &root);

    // The source where it is, and again in the place of the redirect visual.
    assert_eq!(2, log.count("DrawRectangle Blue none 0, 0, 30, 20"), "{:?}", log.entries());
    assert!(Rc::ptr_eq(&redirect.source().expect("the source is set"), &source));
    assert!(RedirectVisual::from_visual(&redirect).is_some());
    assert!(!redirect.clip_to_bounds());
    assert_eq!("RedirectVisual", AsCompositionObject::composition_type_name(&**redirect));

    // The bounds of the redirect visual cover the content of the source, in
    // its own coordinates (the offset of the source is not applied).
    let server = s.server_object(redirect.server());
    let content = server.content_as::<ServerRedirectVisual>().expect("the server visual is a redirect visual");
    assert!(content.source().is_some() && content.source_surface().is_some());
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 30.0, 20.0)), server.sub_tree_bounds());

    // A change of the source redraws both.
    log.clear();
    let brush = s.compositor.create_color_brush_with_color(Colors::RED);
    source.set_brush(Some((*brush).clone()));
    s.run_jobs();
    assert_eq!(2, log.count("DrawRectangle Red none 0, 0, 30, 20"), "{:?}", log.entries());

    // Without a source the redirect visual draws nothing.
    log.clear();
    redirect.set_source(None);
    root.set_opacity(0.5);
    s.run_jobs();
    assert_eq!(1, log.count("DrawRectangle Red none 0, 0, 30, 20"), "{:?}", log.entries());
}

#[test]
fn a_redirect_visual_inside_its_source_draws_nothing() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let source = blue_sprite(&s, 30.0, 20.0);
    let redirect = s.compositor.create_redirect_visual();
    redirect.set_source(Some((*source).clone()));
    redirect.set_offset(Vector3D::new(40.0, 0.0, 0.0));
    source.children().insert_at_top((*redirect).clone());
    show(&s, &source);

    // The source only: a redirect visual inside its own source draws nothing
    // (its bounds leave the source out, so it is not drawn inside itself).
    assert_eq!(1, log.count("DrawRectangle Blue none 0, 0, 30, 20"), "{:?}", log.entries());
    let server = s.server_object(redirect.server());
    assert_eq!(None, server.sub_tree_bounds());
}

// --- nine-grid brush ---------------------------------------------------------------

#[test]
fn nine_grid_rects_cut_the_source_and_the_destination() {
    let source = Rect::new(0.0, 0.0, 30.0, 30.0);
    let dest = Rect::new(0.0, 0.0, 100.0, 60.0);
    let cells = nine_grid_rects(source, dest, [10.0; 4], [1.0; 4]);
    assert_eq!((Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(0.0, 0.0, 10.0, 10.0)), cells[0]);
    assert_eq!((Rect::new(10.0, 0.0, 10.0, 10.0), Rect::new(10.0, 0.0, 80.0, 10.0)), cells[1]);
    assert_eq!((Rect::new(20.0, 0.0, 10.0, 10.0), Rect::new(90.0, 0.0, 10.0, 10.0)), cells[2]);
    assert_eq!((Rect::new(0.0, 10.0, 10.0, 10.0), Rect::new(0.0, 10.0, 10.0, 40.0)), cells[3]);
    assert_eq!((Rect::new(10.0, 10.0, 10.0, 10.0), Rect::new(10.0, 10.0, 80.0, 40.0)), cells[NINE_GRID_CENTER]);
    assert_eq!((Rect::new(20.0, 20.0, 10.0, 10.0), Rect::new(90.0, 50.0, 10.0, 10.0)), cells[8]);

    // The scales size the insets in the destination only.
    let cells = nine_grid_rects(source, dest, [10.0; 4], [2.0, 1.0, 1.0, 0.5]);
    assert_eq!((Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(0.0, 0.0, 20.0, 10.0)), cells[0]);
    assert_eq!(Rect::new(20.0, 10.0, 70.0, 45.0), cells[NINE_GRID_CENTER].1);

    // Insets that do not fit are scaled down to meet.
    let cells = nine_grid_rects(source, Rect::new(0.0, 0.0, 10.0, 60.0), [10.0; 4], [1.0; 4]);
    assert_eq!(Rect::new(0.0, 0.0, 5.0, 10.0), cells[0].1);
    assert_eq!(0.0, cells[NINE_GRID_CENTER].1.width);
    assert_eq!(Rect::new(5.0, 0.0, 5.0, 10.0), cells[2].1);
}

#[test]
fn a_nine_grid_brush_with_a_color_source_fills_the_cells() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let nine_grid = s.compositor.create_nine_grid_brush();
    assert_eq!((1.0, 1.0), (nine_grid.left_inset_scale(), nine_grid.bottom_inset_scale()));
    nine_grid.set_source(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    nine_grid.set_insets(10.0);
    assert_eq!((10.0, 10.0, 10.0, 10.0), (
        nine_grid.left_inset(),
        nine_grid.top_inset(),
        nine_grid.right_inset(),
        nine_grid.bottom_inset()
    ));
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 60.0));
    sprite.set_brush(Some((*nine_grid).clone()));
    show(&s, &sprite);

    let entries = log.entries();
    assert_eq!(9, log.count("DrawRectangle Blue"), "{entries:?}");
    assert!(has_entry(&entries, "DrawRectangle Blue none 10, 10, 80, 40 shadows=0"), "{entries:?}");
    assert!(has_entry(&entries, "DrawRectangle Blue none 90, 50, 10, 10 shadows=0"), "{entries:?}");

    // A hollow centre leaves the centre cell out; the scales size the insets.
    log.clear();
    nine_grid.set_is_center_hollow(true);
    nine_grid.set_inset_scales_with(2.0, 1.0, 1.0, 1.0);
    s.run_jobs();
    let entries = log.entries();
    assert_eq!(8, log.count("DrawRectangle Blue"), "{entries:?}");
    assert!(has_entry(&entries, "DrawRectangle Blue none 0, 0, 20, 10 shadows=0"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("DrawRectangle Blue none 20, 10")), "{entries:?}");
}

#[test]
fn a_nine_grid_brush_with_a_surface_source_cuts_the_surface() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let image = LoadedImageSurface::from_bitmap(&s.compositor, &test_bitmap());
    let surface: Rc<dyn ICompositionSurface> = image.clone();
    let surface_brush = s.compositor.create_surface_brush_with_surface(surface);
    let nine_grid = s.compositor.create_nine_grid_brush();
    nine_grid.set_source(Some((*surface_brush).clone()));
    nine_grid.set_insets(4.0);
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 60.0));
    sprite.set_brush(Some((*nine_grid).clone()));
    show(&s, &sprite);

    let entries = log.entries();
    assert_eq!(9, log.count("DrawBitmap"), "{entries:?}");
    // The top left corner keeps its size: the whole surface at its natural
    // size, clipped to the corner.
    let clip = position(&entries, "PushClip 0, 0, 4, 4");
    let draw = position(&entries, "DrawBitmap 1 0, 0, 20, 10 0, 0, 20, 10");
    assert!(clip < draw, "{entries:?}");
}

// --- mask brush --------------------------------------------------------------------

#[test]
fn a_mask_brush_paints_its_source_through_an_opacity_mask() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let gradient = s.compositor.create_linear_gradient_brush();
    let mask = s.compositor.create_mask_brush();
    mask.set_source(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    mask.set_mask(Some((**gradient).clone()));
    assert!(mask.mask().is_some() && mask.source().is_some());
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 50.0));
    sprite.set_brush(Some((*mask).clone()));
    show(&s, &sprite);

    let entries = log.entries();
    let push = position(&entries, "PushOpacityMask brush 0, 0, 100, 50");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    let pop = position(&entries, "PopOpacityMask");
    assert!(push < draw && draw < pop, "{entries:?}");

    // A surface mask is painted over the source in a layer with the
    // destination-in blending of bitmaps.
    log.clear();
    let image = LoadedImageSurface::from_bitmap(&s.compositor, &test_bitmap());
    let surface: Rc<dyn ICompositionSurface> = image.clone();
    let surface_brush = s.compositor.create_surface_brush_with_surface(surface);
    mask.set_mask(Some((*surface_brush).clone()));
    s.run_jobs();
    let entries = log.entries();
    let layer = position(&entries, "PushLayer 0, 0, 100, 50");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    let options = position(&entries, "PushRenderOptions");
    let bitmap = position_starting(&entries, "DrawBitmap");
    let pop = position(&entries, "PopLayer");
    assert!(layer < draw && draw < options && options < bitmap && bitmap < pop, "{entries:?}");
}

// --- gradient brushes --------------------------------------------------------------

#[test]
fn the_winui_gradient_members_map_to_the_relative_points() {
    let s = CompositorCanvas::new();

    let linear = s.compositor.create_linear_gradient_brush();
    assert_eq!(CompositionMappingMode::Relative, linear.mapping_mode());
    linear.set_start_point_vector(Vector::new(0.2, 0.3));
    linear.set_end_point_vector(Vector::new(1.0, 0.0));
    assert_eq!(RelativePoint::new(0.2, 0.3, RelativeUnit::Relative), linear.start_point());
    assert_eq!(Vector::new(1.0, 0.0), linear.end_point_vector());

    // The mapping mode changes the unit and keeps the numbers.
    linear.set_mapping_mode(CompositionMappingMode::Absolute);
    assert_eq!(RelativePoint::new(0.2, 0.3, RelativeUnit::Absolute), linear.start_point());
    linear.set_end_point_vector(Vector::new(100.0, 0.0));
    assert_eq!(RelativePoint::new(100.0, 0.0, RelativeUnit::Absolute), linear.end_point());

    // The extend mode is a view of the spread method.
    linear.set_extend_mode(CompositionGradientExtendMode::Mirror);
    assert_eq!(GradientSpreadMethod::Reflect, linear.spread_method());
    assert_eq!(CompositionGradientExtendMode::Mirror, linear.extend_mode());

    let radial = s.compositor.create_radial_gradient_brush();
    radial.set_ellipse_center(Vector::new(0.5, 0.5));
    radial.set_gradient_origin_offset(Vector::new(0.1, 0.0));
    assert_eq!(RelativePoint::new(0.6, 0.5, RelativeUnit::Relative), radial.gradient_origin());
    radial.set_ellipse_center(Vector::new(0.25, 0.25));
    assert_eq!(RelativePoint::new(0.25, 0.25, RelativeUnit::Relative), radial.center());
    let origin = radial.gradient_origin();
    assert!((origin.point.x - 0.35).abs() < 1e-12 && (origin.point.y - 0.25).abs() < 1e-12, "{origin:?}");
    radial.set_ellipse_radius(Vector::new(0.4, 0.2));
    assert_eq!((0.4, 0.2), (radial.radius_x().scalar, radial.radius_y().scalar));
    assert_eq!(Vector::new(0.4, 0.2), radial.ellipse_radius());

    // The colour stop factory and the alias of the stops.
    let stop = s.compositor.create_color_gradient_stop(0.5, Colors::RED);
    assert_eq!((0.5, Colors::RED), (stop.offset(), stop.color()));
    assert_eq!(linear.gradient_stops().count(), linear.color_stops().count());
}

#[test]
fn the_brush_transform_of_a_gradient_reaches_the_drawing_context() {
    let s = CompositorCanvas::new();

    let linear = s.compositor.create_linear_gradient_brush();
    assert_eq!(Vector::new(1.0, 1.0), linear.scale());
    linear.set_offset(Vector::new(10.0, 0.0));
    linear.set_anchor_point(Vector::new(0.5, 0.0));
    linear.set_rotation_angle_in_degrees(0.0);
    s.run_jobs();

    let server = s
        .compositor
        .server()
        .get::<ServerCompositionLinearGradientBrush>(linear.server())
        .expect("the server brush exists");
    let transform = IBrush::transform(&*server).expect("the brush has a transform");
    assert_eq!(Matrix::create_translation(10.0, 0.0), transform.value());
    // The anchor point is a translation of the unit square of the area.
    let relative = IBrush::relative_transform(&*server).expect("the brush has a relative transform");
    assert_eq!(Matrix::create_translation(-0.5, 0.0), relative.value());
}

// --- visual members ----------------------------------------------------------------

#[test]
fn the_relative_adjustments_follow_the_size_of_the_parent() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let parent = s.compositor.create_container_visual();
    parent.set_size(Vector::new(200.0, 100.0));
    let child = blue_sprite(&s, 0.0, 0.0);
    child.set_relative_size_adjustment(Vector::new(0.5, 0.5));
    child.set_relative_offset_adjustment(Vector3D::new(0.5, 0.0, 0.0));
    parent.children().insert_at_top((*child).clone());
    show(&s, &parent);

    assert!(has_entry(&log.entries(), "DrawRectangle Blue none 0, 0, 100, 50 shadows=0"), "{:?}", log.entries());
    assert_eq!(Vector::new(100.0, 50.0), child.effective_size());
    assert_eq!(Vector::default(), child.size(), "Size itself is unchanged");
    let server = s.server_object(child.server());
    assert_eq!(Some(Matrix::create_translation(100.0, 0.0)), server.own_transform());

    // A change of the size of the parent resizes and moves the child.
    log.clear();
    parent.set_size(Vector::new(400.0, 100.0));
    s.run_jobs();
    assert!(has_entry(&log.entries(), "DrawRectangle Blue none 0, 0, 200, 50 shadows=0"), "{:?}", log.entries());
    assert_eq!(Some(Matrix::create_translation(200.0, 0.0)), server.own_transform());
}

#[test]
fn a_visual_that_is_not_hit_test_visible_is_skipped_by_hit_testing() {
    let s = CompositorCanvas::new();

    let sprite = blue_sprite(&s, 50.0, 50.0);
    sprite.set_offset(Vector3D::new(10.0, 10.0, 0.0));
    assert!(sprite.is_hit_test_visible());
    show(&s, &sprite);

    let hits = |s: &CompositorCanvas| {
        s.renderer
            .composition_target()
            .try_hit_test::<PointCompositionHitTester>(&Point::new(20.0, 20.0), None, None)
            .expect("the tree is rendered")
    };
    assert!(hits(&s).iter().any(|(_, visual)| Rc::ptr_eq(visual, &sprite)));

    sprite.set_is_hit_test_visible(false);
    s.run_jobs();
    assert!(!hits(&s).iter().any(|(_, visual)| Rc::ptr_eq(visual, &sprite)));
}

#[test]
fn the_rotation_axis_pixel_snapping_and_the_stored_modes() {
    let s = CompositorCanvas::new();

    let sprite = blue_sprite(&s, 50.0, 50.0);
    sprite.set_rotation_angle_in_degrees(90.0);
    assert!((sprite.rotation_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert!((sprite.rotation_angle_in_degrees() - 90.0).abs() < 1e-4);
    assert_eq!(Vector3D::new(0.0, 0.0, 1.0), sprite.rotation_axis());
    sprite.set_rotation_axis(Vector3D::new(1.0, 0.0, 0.0));
    show(&s, &sprite);

    // A rotation about X folds Y onto Z: the 2D projection keeps X only.
    let server = s.server_object(sprite.server());
    let transform = server.own_transform().expect("the visual is rotated");
    assert!((transform.m11 - 1.0).abs() < 1e-6 && transform.m22.abs() < 1e-6, "{transform:?}");

    // Pixel snapping rounds the offset to device pixels.
    sprite.set_rotation_angle(0.0);
    sprite.set_offset(Vector3D::new(10.4, 3.6, 0.0));
    sprite.set_is_pixel_snapping_enabled(true);
    s.run_jobs();
    assert_eq!(Some(Matrix::create_translation(10.0, 4.0)), server.own_transform());

    // The 3D modes are stored.
    sprite.set_border_mode(CompositionBorderMode::Hard);
    sprite.set_backface_visibility(CompositionBackfaceVisibility::Hidden);
    sprite.set_composite_mode(CompositionCompositeMode::MinBlend);
    s.run_jobs();
    assert_eq!(CompositionBorderMode::Hard, server.border_mode());
    assert_eq!(CompositionBackfaceVisibility::Hidden, server.backface_visibility());
    assert_eq!(CompositionCompositeMode::MinBlend, sprite.composite_mode());
}

#[test]
fn a_parent_for_transform_places_the_visual_in_its_space() {
    let s = CompositorCanvas::new();

    let root = s.compositor.create_container_visual();
    root.set_size(Vector::new(300.0, 100.0));
    let anchor = s.compositor.create_container_visual();
    anchor.set_offset(Vector3D::new(50.0, 0.0, 0.0));
    let follower = blue_sprite(&s, 10.0, 10.0);
    follower.set_offset(Vector3D::new(10.0, 0.0, 0.0));
    root.children().insert_at_top(anchor.clone());
    root.children().insert_at_top((*follower).clone());
    follower.set_parent_for_transform(Some(anchor.clone()));
    assert!(Rc::ptr_eq(&follower.parent_for_transform().expect("it is set"), &anchor));
    show(&s, &root);

    let server = s.server_object(follower.server());
    assert_eq!(Some(Matrix::create_translation(60.0, 0.0)), server.own_transform());

    // A move of the visual it is relative to moves it.
    anchor.set_offset(Vector3D::new(80.0, 0.0, 0.0));
    s.run_jobs();
    assert_eq!(Some(Matrix::create_translation(90.0, 0.0)), server.own_transform());

    // Without it the transform is relative to the parent again.
    follower.set_parent_for_transform(None);
    s.run_jobs();
    assert_eq!(Some(Matrix::create_translation(10.0, 0.0)), server.own_transform());
}
