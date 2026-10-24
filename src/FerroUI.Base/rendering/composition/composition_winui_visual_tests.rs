//! Tests of the WinUI visual building blocks (not from upstream): sprite
//! visuals, surface brushes, visual and loaded image surfaces, drop shadows
//! and the clip objects. A child module of the compositor tests, whose
//! fixture renders the composition visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::animation::easings::LinearEasing;
use crate::media::imaging::Bitmap;
use crate::media::Colors;
use crate::platform::{AlphaFormat, LtrbRect, PixelFormats};
use crate::rendering::composition::server::{
    inset_rect, surface_brush_content_rect, ServerCompositionSurfaceBrush, ServerCompositionVisualSurface,
    ServerDropShadow, ServerInsetClip, ServerRectangleClip, ServerSpriteVisual,
};
use crate::rendering::composition::{
    CompositionBitmapInterpolationMode, CompositionDropShadowSourcePolicy, CompositionStretch, CompositionVisual,
    ElementComposition, ICompositionObjectAnimations, ICompositionSurface, InsetClip, LoadedImageSourceLoadStatus,
    LoadedImageSurface, RectangleClip, SpriteVisual,
};
use crate::{Matrix, PixelSize, Rect, Size, Vector, Vector3D};
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

// --- sprite visual ---------------------------------------------------------------

#[test]
fn a_sprite_visual_with_a_color_brush_renders_a_rectangle() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(30.0, 20.0));
    sprite.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    let brush = s.compositor.create_color_brush_with_color(Colors::BLUE);
    sprite.set_brush(Some((*brush).clone()));
    show(&s, &sprite);

    assert_eq!(1, log.count("DrawRectangle Blue none 0, 0, 30, 20"), "{:?}", log.entries());
    assert!(!sprite.clip_to_bounds(), "a sprite visual does not clip to its bounds");
    assert!(SpriteVisual::from_visual(&sprite).is_some());
    assert!(SpriteVisual::from_visual(&s.compositor.create_solid_color_visual()).is_none());
    assert!(Rc::ptr_eq(&sprite.brush().expect("the brush is set"), &brush));
    assert_eq!("SpriteVisual", crate::rendering::composition::AsCompositionObject::composition_type_name(&**sprite));

    let server = s.server_object(sprite.server());
    let content = server.content_as::<ServerSpriteVisual>().expect("the server visual is a sprite visual");
    assert!(content.brush().is_some());
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 30.0, 20.0)), server.sub_tree_bounds());

    // A change of the brush redraws the visual.
    log.clear();
    brush.set_color(Colors::RED);
    s.run_jobs();
    assert_eq!(1, log.count("DrawRectangle Red none 0, 0, 30, 20"), "{:?}", log.entries());

    // Without a brush nothing is drawn.
    log.clear();
    sprite.set_brush(None);
    s.run_jobs();
    assert_eq!(0, log.count("DrawRectangle Red"), "{:?}", log.entries());
    assert!(content.brush().is_none());
}

#[test]
fn the_color_brush_factories_are_the_solid_color_brush_factories() {
    let s = CompositorCanvas::new();
    let brush = s.compositor.create_color_brush();
    assert_eq!("CompositionSolidColorBrush", brush.type_name());
    let brush = s.compositor.create_color_brush_with_color(Colors::GREEN);
    assert_eq!(Colors::GREEN, brush.color());
}

// --- drop shadow -------------------------------------------------------------------

#[test]
fn a_drop_shadow_is_drawn_under_the_sprite_and_extends_its_bounds() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let shadow = s.compositor.create_drop_shadow();
    assert_eq!(9.0, shadow.blur_radius());
    assert_eq!(Colors::BLACK, shadow.color());
    assert_eq!(1.0, shadow.opacity());
    shadow.set_blur_radius(10.0);
    shadow.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    shadow.set_opacity(0.5);
    shadow.set_source_policy(CompositionDropShadowSourcePolicy::InheritFromVisualContent);

    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 50.0));
    sprite.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    sprite.set_shadow(Some(shadow.clone()));
    show(&s, &sprite);

    let entries = log.entries();
    let shadow_draw = entries
        .iter()
        .position(|e| e == "DrawRectangle none none 0, 0, 100, 50 shadows=1")
        .unwrap_or_else(|| panic!("the shadow is drawn: {entries:?}"));
    let brush_draw = entries
        .iter()
        .position(|e| e == "DrawRectangle Blue none 0, 0, 100, 50 shadows=0")
        .unwrap_or_else(|| panic!("the brush is drawn: {entries:?}"));
    assert!(shadow_draw < brush_draw, "the shadow is under the content");

    // The server shadow: the opacity is in the alpha of the box shadow.
    let server_shadow = s.compositor.server().get::<ServerDropShadow>(shadow.server()).expect("the server shadow exists");
    let box_shadow = server_shadow.box_shadow();
    assert_eq!((5.0, 5.0, 10.0), (box_shadow.offset_x, box_shadow.offset_y, box_shadow.blur));
    assert_eq!(128, box_shadow.color.a);
    assert_eq!(CompositionDropShadowSourcePolicy::InheritFromVisualContent, server_shadow.props().source_policy());

    // The bounds of the visual include the shadow: (0, 0, 100, 50) moved by
    // the offset and inflated by the blur radius.
    let server = s.server_object(sprite.server());
    assert_eq!(Some(LtrbRect::new(-5.0, -5.0, 115.0, 65.0)), server.sub_tree_bounds());

    // A change of the shadow changes the bounds.
    shadow.set_blur_radius(20.0);
    s.run_jobs();
    assert_eq!(Some(LtrbRect::new(-15.0, -15.0, 125.0, 75.0)), server.sub_tree_bounds());
}

// --- clips -------------------------------------------------------------------------

#[test]
fn an_inset_clip_pushes_its_rectangle_and_follows_its_animations() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 50.0));
    sprite.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    let clip = s.compositor.create_inset_clip_with_insets(10.0, 5.0, 20.0, 15.0);
    sprite.set_composition_clip(Some((*clip).clone()));
    assert!(InsetClip::from_clip(&sprite.composition_clip().expect("the clip is set")).is_some());
    show(&s, &sprite);

    let entries = log.entries();
    let push = entries.iter().position(|e| e == "PushClip 10, 5, 70, 30").unwrap_or_else(|| panic!("{entries:?}"));
    let draw = entries.iter().position(|e| e == "DrawRectangle Blue none 0, 0, 100, 50 shadows=0").unwrap();
    assert!(push < draw && entries[draw..].iter().any(|e| e == "PopClip"), "{entries:?}");

    // The clip reaches the server and limits the bounds of the visual.
    let server_clip = s.compositor.server().get::<ServerInsetClip>(clip.server()).expect("the server clip exists");
    assert_eq!(
        (10.0, 5.0, 20.0, 15.0),
        (
            server_clip.props().left_inset(),
            server_clip.props().top_inset(),
            server_clip.props().right_inset(),
            server_clip.props().bottom_inset()
        )
    );
    let server = s.server_object(sprite.server());
    assert_eq!(Some(LtrbRect::new(10.0, 5.0, 80.0, 35.0)), server.sub_tree_bounds());

    // A direct change of an inset redraws with the new rectangle.
    log.clear();
    clip.set_left_inset(20.0);
    s.run_jobs();
    assert!(has_entry(&log.entries(), "PushClip 20, 5, 60, 30"), "{:?}", log.entries());

    // An animation of an inset changes the pushed rectangle frame by frame.
    let animation = s.compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, 40.0, Rc::new(LinearEasing::new()));
    animation.set_duration(Duration::from_millis(50));
    clip.start_animation("LeftInset", &*animation);
    log.clear();
    // The animation is committed before its time is measured.
    s.run_jobs();
    run_for(&s, Duration::from_millis(150));
    assert_eq!(40.0, server_clip.props().left_inset());
    assert!(has_entry(&log.entries(), "PushClip 40, 5, 40, 30"), "{:?}", log.entries());
    assert_eq!(Some(LtrbRect::new(40.0, 5.0, 80.0, 35.0)), server.sub_tree_bounds());

    // Removing the clip removes the push.
    sprite.set_composition_clip(None);
    log.clear();
    s.run_jobs();
    assert!(!log.entries().iter().any(|e| e.starts_with("PushClip 40")), "{:?}", log.entries());
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 100.0, 50.0)), server.sub_tree_bounds());
}

#[test]
fn a_rectangle_clip_pushes_a_rounded_rectangle_in_its_own_space() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 100.0));
    sprite.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    let clip = s.compositor.create_rectangle_clip_with_radii(
        5.0,
        5.0,
        55.0,
        45.0,
        Vector::new(4.0, 4.0),
        Vector::new(4.0, 4.0),
        Vector::new(4.0, 4.0),
        Vector::new(4.0, 4.0),
    );
    assert_eq!((5.0, 5.0, 55.0, 45.0), (clip.left(), clip.top(), clip.right(), clip.bottom()));
    sprite.set_composition_clip(Some((*clip).clone()));
    assert!(RectangleClip::from_clip(&sprite.composition_clip().unwrap()).is_some());
    show(&s, &sprite);
    assert!(has_entry(&log.entries(), "PushRoundedClip 5, 5, 50, 40"), "{:?}", log.entries());

    let server_clip = s.compositor.server().get::<ServerRectangleClip>(clip.server()).expect("the server clip exists");
    assert_eq!(Vector::new(4.0, 4.0), server_clip.props().top_left_radius());

    // The transform of the clip moves the clipped area.
    clip.set_offset(Vector::new(10.0, 20.0));
    s.run_jobs();
    let server = s.server_object(sprite.server());
    assert_eq!(Some(LtrbRect::new(15.0, 25.0, 65.0, 65.0)), server.sub_tree_bounds());
    assert_eq!(Vector::new(10.0, 20.0), server_clip.clip_props().offset());

    // Degrees are a view of the radians.
    clip.set_rotation_angle_in_degrees(90.0);
    assert!((clip.rotation_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}

#[test]
fn inset_rect_is_the_area_inside_the_insets() {
    assert_eq!(Rect::new(10.0, 5.0, 70.0, 30.0), inset_rect(Vector::new(100.0, 50.0), 10.0, 5.0, 20.0, 15.0));
    // Insets that overlap leave nothing.
    assert_eq!(Rect::new(60.0, 0.0, 0.0, 50.0), inset_rect(Vector::new(100.0, 50.0), 60.0, 0.0, 60.0, 0.0));
}

// --- surface brush -----------------------------------------------------------------

#[test]
fn surface_brush_layout_follows_stretch_and_alignment() {
    let natural = Size::new(20.0, 10.0);
    let area = Rect::new(0.0, 0.0, 100.0, 100.0);
    let layout = |stretch, h, v| surface_brush_content_rect(natural, area, stretch, h, v).expect("not empty");

    assert_eq!(Rect::new(40.0, 45.0, 20.0, 10.0), layout(CompositionStretch::None, 0.5, 0.5));
    assert_eq!(Rect::new(0.0, 0.0, 20.0, 10.0), layout(CompositionStretch::None, 0.0, 0.0));
    assert_eq!(Rect::new(80.0, 90.0, 20.0, 10.0), layout(CompositionStretch::None, 1.0, 1.0));
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), layout(CompositionStretch::Fill, 0.5, 0.5));
    assert_eq!(Rect::new(0.0, 25.0, 100.0, 50.0), layout(CompositionStretch::Uniform, 0.5, 0.5));
    assert_eq!(Rect::new(0.0, 50.0, 100.0, 50.0), layout(CompositionStretch::Uniform, 0.5, 1.0));
    assert_eq!(Rect::new(-50.0, 0.0, 200.0, 100.0), layout(CompositionStretch::UniformToFill, 0.5, 0.5));
    assert_eq!(Rect::new(0.0, 0.0, 200.0, 100.0), layout(CompositionStretch::UniformToFill, 0.0, 0.5));
    assert_eq!(Rect::new(-100.0, 0.0, 200.0, 100.0), layout(CompositionStretch::UniformToFill, 1.0, 0.5));

    // The area may be anywhere.
    let moved = surface_brush_content_rect(natural, Rect::new(10.0, 20.0, 40.0, 40.0), CompositionStretch::Uniform, 0.5, 0.5);
    assert_eq!(Some(Rect::new(10.0, 30.0, 40.0, 20.0)), moved);

    // Nothing to lay out.
    assert!(surface_brush_content_rect(Size::new(0.0, 10.0), area, CompositionStretch::Fill, 0.5, 0.5).is_none());
    assert!(surface_brush_content_rect(natural, Rect::new(0.0, 0.0, 0.0, 10.0), CompositionStretch::Fill, 0.5, 0.5).is_none());
}

#[test]
fn a_surface_brush_paints_a_loaded_image_surface() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let bitmap = test_bitmap();
    let image = LoadedImageSurface::from_bitmap(&s.compositor, &bitmap);
    assert_eq!(LoadedImageSourceLoadStatus::Success, image.load_status());
    assert_eq!(Size::new(20.0, 10.0), image.natural_size());
    assert_eq!(Size::new(20.0, 10.0), image.decoded_physical_size());
    // The surface keeps its own reference to the bitmap.
    bitmap.dispose();
    assert_eq!(Size::new(20.0, 10.0), image.natural_size());

    let surface: Rc<dyn ICompositionSurface> = image.clone();
    let brush = s.compositor.create_surface_brush_with_surface(surface);
    assert_eq!(CompositionStretch::Fill, brush.stretch());
    assert_eq!((0.5, 0.5), (brush.horizontal_alignment_ratio(), brush.vertical_alignment_ratio()));
    assert_eq!(CompositionBitmapInterpolationMode::Linear, brush.bitmap_interpolation_mode());
    assert_eq!(Vector::new(1.0, 1.0), brush.scale());
    brush.set_stretch(CompositionStretch::Uniform);

    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 100.0));
    sprite.set_brush(Some((*brush).clone()));
    show(&s, &sprite);

    let entries = log.entries();
    assert!(has_entry(&entries, "DrawBitmap 1 0, 0, 20, 10 0, 25, 100, 50"), "{entries:?}");
    // The content is clipped to the area of the brush.
    assert!(has_entry(&entries, "PushClip 0, 0, 100, 100"), "{entries:?}");

    // The brush transform moves the content; the alignment changes its
    // place.
    log.clear();
    brush.set_offset(Vector::new(5.0, 0.0));
    brush.set_vertical_alignment_ratio(0.0);
    s.run_jobs();
    let entries = log.entries();
    assert!(has_entry(&entries, "DrawBitmap 1 0, 0, 20, 10 0, 0, 100, 50"), "{entries:?}");
    assert!(entries.iter().any(|e| e.starts_with("SetTransform")), "{entries:?}");

    // An interpolation mode other than the default is pushed as a render
    // option.
    log.clear();
    brush.set_bitmap_interpolation_mode(CompositionBitmapInterpolationMode::NearestNeighbor);
    s.run_jobs();
    assert!(has_entry(&log.entries(), "PushRenderOptions"), "{:?}", log.entries());
}

#[test]
fn surface_brush_properties_reach_the_server() {
    let s = CompositorCanvas::new();
    let brush = s.compositor.create_surface_brush();
    assert!(brush.surface().is_none());
    brush.set_stretch(CompositionStretch::UniformToFill);
    brush.set_horizontal_alignment_ratio(0.0);
    brush.set_vertical_alignment_ratio(1.0);
    brush.set_bitmap_interpolation_mode(CompositionBitmapInterpolationMode::MagLinearMinLinearMipLinear);
    brush.set_anchor_point(Vector::new(0.5, 0.5));
    brush.set_center_point(Vector::new(3.0, 4.0));
    brush.set_offset(Vector::new(1.0, 2.0));
    brush.set_rotation_angle(0.25);
    brush.set_scale(Vector::new(2.0, 3.0));
    brush.set_transform_matrix(Matrix::create_translation(7.0, 8.0));
    brush.set_snap_to_pixels(true);
    s.run_jobs();

    let server = s.compositor.server().get::<ServerCompositionSurfaceBrush>(brush.server()).expect("the server brush exists");
    let props = server.props();
    assert_eq!(CompositionStretch::UniformToFill, props.stretch());
    assert_eq!((0.0, 1.0), (props.horizontal_alignment_ratio(), props.vertical_alignment_ratio()));
    assert_eq!(CompositionBitmapInterpolationMode::MagLinearMinLinearMipLinear, props.bitmap_interpolation_mode());
    assert_eq!(Vector::new(0.5, 0.5), props.anchor_point());
    assert_eq!(Vector::new(3.0, 4.0), props.center_point());
    assert_eq!(Vector::new(1.0, 2.0), props.offset());
    assert_eq!(0.25, props.rotation_angle());
    assert_eq!(Vector::new(2.0, 3.0), props.scale());
    assert_eq!(Matrix::create_translation(7.0, 8.0), props.transform_matrix());
    assert!(props.snap_to_pixels());
    // A surface brush is not a brush of the drawing context.
    let object = s.compositor.server().get_object(brush.server()).expect("the server brush exists");
    assert!(object.as_brush().is_none());

    // The animatable members are known to StartAnimation.
    for name in ["HorizontalAlignmentRatio", "Offset", "Scale", "TransformMatrix", "Opacity"] {
        assert!(brush.get_composition_property(name).is_some(), "{name}");
    }
}

#[test]
fn a_stream_that_is_not_an_image_gives_an_empty_loaded_image_surface() {
    let s = CompositorCanvas::new();
    let mut data: &[u8] = &[1, 2, 3];
    let surface = LoadedImageSurface::start_load_from_stream(&s.compositor, &mut data);
    assert_eq!(LoadedImageSourceLoadStatus::InvalidFormat, surface.load_status());
    assert!(surface.bitmap().is_none());
    assert_eq!(Size::default(), surface.natural_size());
}

// --- visual surface ------------------------------------------------------------------

/// A root sprite with a hidden blue source (with a green child) and a
/// sprite that shows the source through a visual surface.
struct VisualSurfaceScene {
    root: SpriteVisual,
    source: crate::rendering::composition::CompositionSolidColorVisual,
    child: crate::rendering::composition::CompositionSolidColorVisual,
    surface: Rc<crate::rendering::composition::CompositionVisualSurface>,
    sprite: SpriteVisual,
}

fn visual_surface_scene(s: &CompositorCanvas) -> VisualSurfaceScene {
    let root = s.compositor.create_sprite_visual();
    root.set_size(Vector::new(200.0, 200.0));

    let source = s.compositor.create_solid_color_visual();
    source.set_size(Vector::new(40.0, 20.0));
    source.set_offset(Vector3D::new(150.0, 150.0, 0.0));
    source.set_color(Colors::BLUE);
    source.set_visible(false);
    root.children().add((*source).clone());
    let child = s.compositor.create_solid_color_visual();
    child.set_size(Vector::new(10.0, 10.0));
    child.set_color(Colors::GREEN);
    source.children().add((*child).clone());

    let surface = s.compositor.create_visual_surface();
    surface.set_source_visual(Some((*source).clone()));
    let brush = s.compositor.create_surface_brush_with_surface(surface.clone());

    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(80.0, 40.0));
    sprite.set_offset(Vector3D::new(10.0, 10.0, 0.0));
    sprite.set_brush(Some((*brush).clone()));
    root.children().add((*sprite).clone());

    VisualSurfaceScene { root, source, child, surface, sprite }
}

#[test]
fn a_visual_surface_renders_its_hidden_source() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let scene = visual_surface_scene(&s);
    show(&s, &scene.root);

    // The source is hidden: its subtree is drawn once, through the surface,
    // scaled into the sprite and clipped to it.
    let entries = log.entries();
    assert_eq!(1, log.count("DrawRectangle Blue none 0, 0, 40, 20"), "{entries:?}");
    assert_eq!(1, log.count("DrawRectangle Green none 0, 0, 10, 10"), "{entries:?}");
    assert_eq!(2, entries.iter().filter(|e| *e == "PushClip 0, 0, 80, 40").count(), "{entries:?}");

    // The server surface shows the source visual.
    let server = s.compositor.server().get::<ServerCompositionVisualSurface>(scene.surface.server()).expect("the server surface exists");
    let source = server.source_visual().expect("the source reaches the server");
    assert!(Rc::ptr_eq(&source, &s.server_object(scene.source.server())));
    assert_eq!(Some(Rect::new(0.0, 0.0, 40.0, 20.0)), server.source_rect());

    // The surface follows the changes of the source subtree.
    log.clear();
    scene.child.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    s.run_jobs();
    assert_eq!(1, log.count("DrawRectangle Green none 0, 0, 10, 10"), "{:?}", log.entries());

    // The source area: an offset and a size in the coordinates of the source.
    scene.surface.set_source_offset(Vector::new(10.0, 0.0));
    scene.surface.set_source_size(Vector::new(20.0, 20.0));
    s.run_jobs();
    assert_eq!(Some(Rect::new(10.0, 0.0, 20.0, 20.0)), server.source_rect());
    assert_eq!(Vector::new(10.0, 0.0), server.props().source_offset());
}

#[test]
fn a_frozen_visual_surface_shows_the_image_it_took() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let scene = visual_surface_scene(&s);
    show(&s, &scene.root);

    scene.surface.freeze();
    log.clear();
    s.run_jobs();
    let server = s.compositor.server().get::<ServerCompositionVisualSurface>(scene.surface.server()).unwrap();
    assert!(server.is_frozen());
    // The source is rendered into a layer once, and the layer is shown.
    let entries = log.entries();
    assert_eq!(1, log.count("DrawRectangle Blue none 0, 0, 40, 20"), "{entries:?}");
    assert!(has_entry(&entries, "DrawBitmap 1 0, 0, 40, 20 0, 0, 80, 40"), "{entries:?}");

    // Changes of the source are not followed any more.
    log.clear();
    scene.child.set_offset(Vector3D::new(5.0, 5.0, 0.0));
    scene.sprite.set_offset(Vector3D::new(12.0, 10.0, 0.0));
    s.run_jobs();
    assert_eq!(0, log.count("DrawRectangle Green"), "{:?}", log.entries());
    assert!(has_entry(&log.entries(), "DrawBitmap 1 0, 0, 40, 20 0, 0, 80, 40"), "{:?}", log.entries());
}

#[test]
fn a_visual_surface_that_shows_an_ancestor_of_its_sprite_does_not_recurse() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let scene = visual_surface_scene(&s);
    // The sprite shows the root, which contains the sprite.
    scene.source.set_visible(true);
    scene.surface.set_source_visual(Some((*scene.root).clone()));
    show(&s, &scene.root);

    // The source is drawn by the root and once more inside the surface; the
    // sprite inside the surface does not draw the surface again.
    let drawn = log.count("DrawRectangle Blue none 0, 0, 40, 20");
    assert!(drawn >= 2, "{:?}", log.entries());
    let server = s.compositor.server().get::<ServerCompositionVisualSurface>(scene.surface.server()).unwrap();
    assert!(!server.is_rendering());
}
