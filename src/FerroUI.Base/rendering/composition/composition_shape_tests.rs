//! Tests of the WinUI shapes (not from upstream): shape visuals, sprite and
//! container shapes, the geometries with their trimming, view boxes and
//! geometric clips. A child module of the compositor tests, whose fixture
//! renders the composition visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::animation::easings::LinearEasing;
use crate::media::imaging::Bitmap;
use crate::media::{Colors, IDashStyle, IPen, PenLineCap, PenLineJoin};
use crate::platform::{AlphaFormat, LtrbRect, PixelFormats};
use crate::rendering::composition::server::{
    trim_geometry, view_box_mapping, ServerCompositionContainerShape, ServerCompositionSpriteShape,
    ServerCompositionViewBox, ServerGeometricClip, ServerShapeVisual,
};
use crate::rendering::composition::{
    CompositionContainerShape, CompositionPath, CompositionSpriteShape, CompositionStretch, CompositionStrokeCap,
    CompositionStrokeLineJoin, CompositionVisual, ElementComposition, GeometricClip, ICompositionObjectAnimations,
    ICompositionSurface, LoadedImageSurface, ShapeVisual,
};
use crate::rendering::testing::{DrawingLog, MockGeometryImpl, MockPlatformRenderInterface};
use crate::{Matrix, PixelSize, Point, Rect, Vector};
use std::rc::Rc;
use std::sync::Arc;
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

/// A shape visual of `size` with one sprite shape that fills the rectangle
/// `(x, y, width, height)` with blue.
fn rectangle_scene(
    s: &CompositorCanvas,
    size: Vector,
    rect: Rect,
) -> (ShapeVisual, CompositionSpriteShape, crate::rendering::composition::CompositionRectangleGeometry) {
    let visual = s.compositor.create_shape_visual();
    visual.set_size(size);
    let geometry = s.compositor.create_rectangle_geometry();
    geometry.set_offset(Vector::new(rect.x, rect.y));
    geometry.set_size(Vector::new(rect.width, rect.height));
    let shape = s.compositor.create_sprite_shape_with_geometry(Some((*geometry).clone()));
    shape.set_fill_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    visual.shapes().append((*shape).clone());
    (visual, shape, geometry)
}

/// The matrix `m` applied to `(x, y)`.
fn map(m: Matrix, x: f64, y: f64) -> Point {
    Point::new(x, y).transform(m)
}

fn assert_point(expected: (f64, f64), actual: Point) {
    assert!(
        (expected.0 - actual.x).abs() < 1e-9 && (expected.1 - actual.y).abs() < 1e-9,
        "expected {expected:?}, got {actual}"
    );
}

// --- shape visual and sprite shape ---------------------------------------------------

#[test]
fn a_rectangle_shape_with_a_color_fill_draws_its_geometry() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (visual, shape, geometry) = rectangle_scene(&s, Vector::new(100.0, 100.0), Rect::new(10.0, 20.0, 30.0, 40.0));
    show(&s, &visual);

    let entries = log.entries();
    assert_eq!(1, log.count("DrawGeometry Blue none 10, 20, 30, 40"), "{entries:?}");
    // The shapes are clipped to the size of the visual.
    assert!(has_entry(&entries, "PushClip 0, 0, 100, 100"), "{entries:?}");
    assert!(!visual.clip_to_bounds(), "a shape visual does not clip its children");
    assert!(ShapeVisual::from_visual(&visual).is_some());
    assert!(ShapeVisual::from_visual(&s.compositor.create_sprite_visual()).is_none());
    assert_eq!("ShapeVisual", crate::rendering::composition::AsCompositionObject::composition_type_name(&**visual));
    assert_eq!("CompositionSpriteShape", shape.type_name());
    assert_eq!("CompositionRectangleGeometry", geometry.type_name());
    assert!(CompositionSpriteShape::from_shape(&shape).is_some());
    assert!(CompositionContainerShape::from_shape(&shape).is_none());
    assert!(Rc::ptr_eq(&shape.geometry().expect("the geometry is set"), &geometry));
    assert_eq!(1, visual.shapes().count());

    let server = s.server_object(visual.server());
    let content = server.content_as::<ServerShapeVisual>().expect("the server visual is a shape visual");
    assert_eq!(1, content.shapes().len());
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 100.0, 100.0)), server.sub_tree_bounds());

    // A change of the geometry redraws the shape with the new geometry.
    log.clear();
    geometry.set_size(Vector::new(50.0, 40.0));
    s.run_jobs();
    assert_eq!(1, log.count("DrawGeometry Blue none 10, 20, 50, 40"), "{:?}", log.entries());

    // A change of the brush redraws it.
    log.clear();
    let brush = shape.fill_brush().expect("the fill is set");
    crate::rendering::composition::CompositionSolidColorBrush::from_brush(&brush)
        .expect("a colour brush")
        .set_color(Colors::RED);
    s.run_jobs();
    assert_eq!(1, log.count("DrawGeometry Red none 10, 20, 50, 40"), "{:?}", log.entries());

    // Without shapes nothing is drawn.
    log.clear();
    visual.shapes().clear();
    s.run_jobs();
    assert_eq!(0, log.count("DrawGeometry"), "{:?}", log.entries());
    assert!(content.shapes().is_empty());
}

#[test]
fn a_shape_visual_without_a_size_draws_nothing() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (visual, _shape, _geometry) = rectangle_scene(&s, Vector::new(0.0, 0.0), Rect::new(0.0, 0.0, 30.0, 40.0));
    show(&s, &visual);
    assert_eq!(0, log.count("DrawGeometry"), "{:?}", log.entries());
}

#[test]
fn stroke_properties_reach_the_pen() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (visual, shape, _geometry) = rectangle_scene(&s, Vector::new(100.0, 100.0), Rect::new(10.0, 20.0, 30.0, 40.0));

    // The WinUI defaults.
    assert_eq!(1.0, shape.stroke_thickness());
    assert_eq!(2.0, shape.stroke_miter_limit());
    assert_eq!(CompositionStrokeLineJoin::Miter, shape.stroke_line_join());
    assert_eq!(CompositionStrokeCap::Flat, shape.stroke_start_cap());
    assert!(shape.stroke_dash_array().is_empty());

    shape.set_fill_brush(None);
    shape.set_stroke_brush(Some((*s.compositor.create_color_brush_with_color(Colors::RED)).clone()));
    shape.set_stroke_thickness(4.0);
    shape.set_stroke_start_cap(CompositionStrokeCap::Round);
    shape.set_stroke_end_cap(CompositionStrokeCap::Square);
    shape.set_stroke_dash_cap(CompositionStrokeCap::Triangle);
    shape.set_stroke_line_join(CompositionStrokeLineJoin::Bevel);
    shape.set_stroke_miter_limit(3.0);
    shape.set_stroke_dash_array(vec![2.0, 1.0]);
    shape.set_stroke_dash_offset(0.5);
    show(&s, &visual);
    assert_eq!(1, log.count("DrawGeometry none Red@4 10, 20, 30, 40"), "{:?}", log.entries());

    let server = s.compositor.server().get::<ServerCompositionSpriteShape>(shape.server()).expect("the server shape exists");
    let props = server.props();
    assert_eq!(CompositionStrokeCap::Square, props.stroke_end_cap());
    assert_eq!(CompositionStrokeCap::Triangle, props.stroke_dash_cap());
    let pen = server.pen(Matrix::IDENTITY, None).expect("the stroke has a thickness");
    assert_eq!(4.0, pen.thickness());
    assert_eq!(PenLineCap::Round, pen.line_cap());
    assert_eq!(PenLineJoin::Bevel, pen.line_join());
    assert_eq!(3.0, pen.miter_limit());
    let dashes = pen.dash_style().expect("the stroke is dashed");
    assert_eq!(Some(vec![2.0, 1.0]), IDashStyle::dashes(&*dashes));
    assert_eq!(0.5, IDashStyle::offset(&*dashes));

    // A non-scaling stroke keeps its thickness under a scale.
    assert_eq!(4.0, server.pen(Matrix::create_scale(2.0, 2.0), None).unwrap().thickness());
    shape.set_is_stroke_non_scaling(true);
    s.run_jobs();
    assert_eq!(2.0, server.pen(Matrix::create_scale(2.0, 2.0), None).unwrap().thickness());

    // A zero thickness draws no stroke.
    shape.set_stroke_thickness(0.0);
    s.run_jobs();
    assert!(server.pen(Matrix::IDENTITY, None).is_none());

    // The caps and joins the platform pen does not have.
    assert_eq!(PenLineCap::Round, CompositionStrokeCap::Triangle.to_pen_line_cap());
    assert_eq!(PenLineJoin::Miter, CompositionStrokeLineJoin::MiterOrBevel.to_pen_line_join());

    // The animatable members are known to StartAnimation.
    for name in ["StrokeThickness", "StrokeDashOffset", "StrokeMiterLimit", "Offset", "Scale", "TransformMatrix"] {
        assert!(shape.get_composition_property(name).is_some(), "{name}");
    }
}

#[test]
fn a_surface_brush_fill_is_painted_inside_the_geometry() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (visual, shape, _geometry) = rectangle_scene(&s, Vector::new(100.0, 100.0), Rect::new(10.0, 20.0, 30.0, 40.0));
    let bitmap = Bitmap::from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &[0u8; 20 * 10 * 4],
        PixelSize::new(20, 10),
        Vector::new(96.0, 96.0),
        20 * 4,
    );
    let surface: Rc<dyn ICompositionSurface> = LoadedImageSurface::from_bitmap(&s.compositor, &bitmap);
    let brush = s.compositor.create_surface_brush_with_surface(surface);
    shape.set_fill_brush(Some((*brush).clone()));
    show(&s, &visual);

    let entries = log.entries();
    let push = entries.iter().position(|e| e == "PushGeometryClip 10, 20, 30, 40").unwrap_or_else(|| panic!("{entries:?}"));
    let draw = entries
        .iter()
        .position(|e| e == "DrawBitmap 1 0, 0, 20, 10 10, 20, 30, 40")
        .unwrap_or_else(|| panic!("{entries:?}"));
    let pop = entries.iter().position(|e| e == "PopGeometryClip").unwrap_or_else(|| panic!("{entries:?}"));
    assert!(push < draw && draw < pop, "{entries:?}");
    // The surface brush is not handed to the drawing context.
    assert_eq!(0, log.count("DrawGeometry"), "{entries:?}");
}

// --- geometries ----------------------------------------------------------------------

#[test]
fn the_geometries_build_their_platform_geometry() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let visual = s.compositor.create_shape_visual();
    visual.set_size(Vector::new(200.0, 200.0));
    let fill = || Some((*s.compositor.create_color_brush_with_color(Colors::GREEN)).clone());

    let ellipse = s.compositor.create_ellipse_geometry();
    ellipse.set_center(Vector::new(50.0, 50.0));
    ellipse.set_radius(Vector::new(10.0, 20.0));
    let rounded = s.compositor.create_rounded_rectangle_geometry();
    rounded.set_offset(Vector::new(100.0, 0.0));
    rounded.set_size(Vector::new(40.0, 20.0));
    rounded.set_corner_radius(Vector::new(30.0, 5.0));
    let line = s.compositor.create_line_geometry();
    line.set_start(Vector::new(0.0, 150.0));
    line.set_end(Vector::new(80.0, 150.0));
    let path = CompositionPath::parse("M 120,120 L 160,120 L 160,180 Z").expect("valid path data");
    assert_eq!(Rect::new(120.0, 120.0, 40.0, 60.0), path.bounds());
    let path_geometry = s.compositor.create_path_geometry_with_path(Some(path));
    assert!(path_geometry.path().is_some());
    assert!(s.compositor.create_path_geometry().path().is_none());

    for geometry in [&*ellipse, &*rounded, &*line, &*path_geometry] {
        let shape = s.compositor.create_sprite_shape_with_geometry(Some(geometry.clone()));
        shape.set_fill_brush(fill());
        visual.shapes().append((*shape).clone());
    }
    show(&s, &visual);

    let entries = log.entries();
    assert!(has_entry(&entries, "DrawGeometry Green none 40, 30, 20, 40"), "{entries:?}");
    assert!(has_entry(&entries, "DrawGeometry Green none 100, 0, 40, 20"), "{entries:?}");
    assert!(has_entry(&entries, "DrawGeometry Green none 0, 150, 80, 0"), "{entries:?}");
    assert!(has_entry(&entries, "DrawGeometry Green none 120, 120, 40, 60"), "{entries:?}");
    // Drawn in the order of the collection.
    let ellipse_draw = entries.iter().position(|e| e == "DrawGeometry Green none 40, 30, 20, 40").unwrap();
    let path_draw = entries.iter().position(|e| e == "DrawGeometry Green none 120, 120, 40, 60").unwrap();
    assert!(ellipse_draw < path_draw);

    // The corner radii are limited to half the size.
    let server = s
        .compositor
        .server()
        .get::<crate::rendering::composition::server::ServerCompositionRoundedRectangleGeometry>(rounded.server())
        .expect("the server geometry exists");
    assert_eq!((20.0, 5.0), server.radii());

    // The animatable members are known to StartAnimation.
    for name in ["TrimStart", "TrimEnd", "TrimOffset", "Center", "Radius"] {
        assert!(ellipse.get_composition_property(name).is_some(), "{name}");
    }
}

#[test]
fn trim_geometry_cuts_the_outline() {
    let render_interface = MockPlatformRenderInterface::new(DrawingLog::new());
    let line = || -> Arc<dyn crate::platform::IGeometryImpl> {
        MockGeometryImpl::line(Point::new(0.0, 0.0), Point::new(100.0, 0.0))
    };
    let trim = |start: f32, end: f32, offset: f32| {
        trim_geometry(line(), start, end, offset, &*render_interface).map(|geometry| geometry.bounds())
    };
    assert_eq!(Some(Rect::new(0.0, 0.0, 100.0, 0.0)), trim(0.0, 1.0, 0.3));
    assert_eq!(Some(Rect::new(0.0, 0.0, 50.0, 0.0)), trim(0.0, 0.5, 0.0));
    assert_eq!(Some(Rect::new(25.0, 0.0, 50.0, 0.0)), trim(0.0, 0.5, 0.25));
    // The start after the end: swapped.
    assert_eq!(Some(Rect::new(20.0, 0.0, 40.0, 0.0)), trim(0.6, 0.2, 0.0));
    // Nothing left.
    assert_eq!(None, trim(0.4, 0.4, 0.0));
    // A part that wraps around the end is two figures.
    assert_eq!(Some(Rect::new(0.0, 0.0, 100.0, 0.0)), trim(0.0, 0.5, 0.75));
    let wrapped = trim_geometry(line(), 0.0, 0.2, 0.9, &*render_interface).expect("something is left");
    assert!((wrapped.bounds().width - 100.0).abs() < 1e-9);
    // A geometry that cannot be cut is drawn whole.
    let rectangle = MockGeometryImpl::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0));
    let whole = trim_geometry(rectangle, 0.0, 0.5, 0.0, &*render_interface).expect("drawn whole");
    assert_eq!(Rect::new(0.0, 0.0, 10.0, 10.0), whole.bounds());
}

#[test]
fn trimming_a_line_shape_draws_a_shorter_line_and_follows_its_animation() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let visual = s.compositor.create_shape_visual();
    visual.set_size(Vector::new(200.0, 100.0));
    let line = s.compositor.create_line_geometry();
    line.set_start(Vector::new(0.0, 10.0));
    line.set_end(Vector::new(100.0, 10.0));
    assert_eq!((0.0, 1.0, 0.0), (line.trim_start(), line.trim_end(), line.trim_offset()));
    let shape = s.compositor.create_sprite_shape_with_geometry(Some((*line).clone()));
    shape.set_stroke_brush(Some((*s.compositor.create_color_brush_with_color(Colors::RED)).clone()));
    shape.set_stroke_thickness(2.0);
    visual.shapes().append((*shape).clone());
    show(&s, &visual);
    assert_eq!(1, log.count("DrawGeometry none Red@2 0, 10, 100, 0"), "{:?}", log.entries());

    log.clear();
    line.set_trim_end(0.5);
    s.run_jobs();
    assert_eq!(1, log.count("DrawGeometry none Red@2 0, 10, 50, 0"), "{:?}", log.entries());

    log.clear();
    line.set_trim_offset(0.25);
    s.run_jobs();
    assert_eq!(1, log.count("DrawGeometry none Red@2 25, 10, 50, 0"), "{:?}", log.entries());

    // An animation of TrimEnd redraws frame by frame.
    line.set_trim_offset(0.0);
    let animation = s.compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, 0.2, Rc::new(LinearEasing::new()));
    animation.set_duration(Duration::from_millis(50));
    line.start_animation("TrimEnd", &*animation);
    log.clear();
    // The animation is committed before its time is measured.
    s.run_jobs();
    run_for(&s, Duration::from_millis(150));
    assert!(has_entry(&log.entries(), "DrawGeometry none Red@2 0, 10, 20, 0"), "{:?}", log.entries());
}

// --- container shapes ----------------------------------------------------------------

#[test]
fn container_shape_transforms_compose() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let visual = s.compositor.create_shape_visual();
    visual.set_size(Vector::new(200.0, 200.0));

    let container = s.compositor.create_container_shape();
    container.set_offset(Vector::new(10.0, 10.0));
    let geometry = s.compositor.create_rectangle_geometry();
    geometry.set_size(Vector::new(20.0, 20.0));
    let inner = s.compositor.create_sprite_shape_with_geometry(Some((*geometry).clone()));
    inner.set_fill_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    inner.set_offset(Vector::new(5.0, 0.0));
    inner.set_scale(Vector::new(2.0, 2.0));
    container.shapes().append((*inner).clone());
    visual.shapes().append((*container).clone());
    assert_eq!("CompositionContainerShape", container.type_name());
    assert!(CompositionContainerShape::from_shape(&container).is_some());
    show(&s, &visual);

    // The inner shape is scaled, moved by its offset, then by the offset of
    // the container: (0, 0) -> (15, 10), (20, 20) -> (55, 50).
    let server_inner = s.compositor.server().get::<ServerCompositionSpriteShape>(inner.server()).unwrap();
    let server_container = s.compositor.server().get::<ServerCompositionContainerShape>(container.server()).unwrap();
    assert_eq!(1, server_container.shapes().len());
    // Composed as the renderer does: the shape, the container, the (absent)
    // view box, then the transform of the visual (the identity here).
    let world = server_inner.transform().unwrap() * (server_container.transform().unwrap() * Matrix::IDENTITY);
    assert_point((15.0, 10.0), map(world, 0.0, 0.0));
    assert_point((55.0, 50.0), map(world, 20.0, 20.0));
    let entries = log.entries();
    assert!(has_entry(&entries, &format!("SetTransform {}", world * Matrix::IDENTITY)), "{entries:?}");
    assert_eq!(1, log.count("DrawGeometry Blue none 0, 0, 20, 20"), "{entries:?}");

    // Degrees are a view of the radians.
    container.set_rotation_angle_in_degrees(90.0);
    assert!((container.rotation_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}

#[test]
fn animating_the_offset_of_a_shape_redraws_it() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (visual, shape, _geometry) = rectangle_scene(&s, Vector::new(100.0, 100.0), Rect::new(0.0, 0.0, 10.0, 10.0));
    show(&s, &visual);

    let animation = s.compositor.create_vector_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, Vector::new(40.0, 0.0), Rc::new(LinearEasing::new()));
    animation.set_duration(Duration::from_millis(50));
    shape.start_animation("Offset", &*animation);
    log.clear();
    s.run_jobs();
    run_for(&s, Duration::from_millis(150));

    let server = s.compositor.server().get::<ServerCompositionSpriteShape>(shape.server()).unwrap();
    assert_eq!(Vector::new(40.0, 0.0), server.shape_props().offset());
    let entries = log.entries();
    assert!(log.count("DrawGeometry Blue none 0, 0, 10, 10") >= 1, "{entries:?}");
    let world = server.transform().expect("the shape is moved") * Matrix::IDENTITY;
    assert_point((40.0, 0.0), map(world, 0.0, 0.0));
    assert!(has_entry(&entries, &format!("SetTransform {}", world * Matrix::IDENTITY)), "{entries:?}");
}

// --- view box ------------------------------------------------------------------------

#[test]
fn view_box_mapping_follows_stretch_and_alignment() {
    let area = Rect::new(0.0, 0.0, 200.0, 100.0);
    let size = Vector::new(50.0, 50.0);
    let origin = Vector::new(0.0, 0.0);

    let uniform = view_box_mapping(origin, size, area, CompositionStretch::Uniform, 0.5, 0.5).expect("not empty");
    assert_point((50.0, 0.0), map(uniform.transform, 0.0, 0.0));
    assert_point((150.0, 100.0), map(uniform.transform, 50.0, 50.0));
    assert_eq!(Rect::new(50.0, 0.0, 100.0, 100.0), uniform.clip);

    let fill = view_box_mapping(origin, size, area, CompositionStretch::Fill, 0.5, 0.5).unwrap();
    assert_point((200.0, 100.0), map(fill.transform, 50.0, 50.0));

    let left = view_box_mapping(origin, size, area, CompositionStretch::Uniform, 0.0, 0.5).unwrap();
    assert_point((0.0, 0.0), map(left.transform, 0.0, 0.0));

    // The offset of the view box is its top left corner in shape
    // coordinates.
    let moved = view_box_mapping(Vector::new(10.0, 10.0), size, area, CompositionStretch::Uniform, 0.5, 0.5).unwrap();
    assert_point((50.0, 0.0), map(moved.transform, 10.0, 10.0));

    // UniformToFill is clipped to the area.
    let cover = view_box_mapping(origin, size, area, CompositionStretch::UniformToFill, 0.5, 0.5).unwrap();
    assert_eq!(area, cover.clip);

    // An empty view box maps nothing.
    assert!(view_box_mapping(origin, Vector::new(0.0, 10.0), area, CompositionStretch::Fill, 0.5, 0.5).is_none());
}

#[test]
fn a_view_box_maps_the_shapes_onto_the_visual() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let (visual, _shape, _geometry) = rectangle_scene(&s, Vector::new(200.0, 100.0), Rect::new(0.0, 0.0, 50.0, 50.0));
    let view_box = s.compositor.create_view_box();
    assert_eq!(CompositionStretch::Fill, view_box.stretch());
    assert_eq!((0.5, 0.5), (view_box.horizontal_alignment_ratio(), view_box.vertical_alignment_ratio()));
    view_box.set_size(Vector::new(50.0, 50.0));
    view_box.set_stretch(CompositionStretch::Uniform);
    visual.set_view_box(Some(view_box.clone()));
    assert!(Rc::ptr_eq(&visual.view_box().expect("the view box is set"), &view_box));
    show(&s, &visual);

    let entries = log.entries();
    assert!(has_entry(&entries, "PushClip 50, 0, 100, 100"), "{entries:?}");
    assert_eq!(1, log.count("DrawGeometry Blue none 0, 0, 50, 50"), "{entries:?}");
    let server = s.compositor.server().get::<ServerCompositionViewBox>(view_box.server()).unwrap();
    let mapping = server.mapping(Vector::new(200.0, 100.0)).expect("the view box maps");
    assert!(has_entry(&entries, &format!("SetTransform {}", mapping.transform * Matrix::IDENTITY)), "{entries:?}");

    // A change of the view box redraws through the new mapping.
    log.clear();
    view_box.set_horizontal_alignment_ratio(0.0);
    s.run_jobs();
    assert!(has_entry(&log.entries(), "PushClip 0, 0, 100, 100"), "{:?}", log.entries());
}

// --- geometric clip ------------------------------------------------------------------

#[test]
fn a_geometric_clip_pushes_a_geometry_clip() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 100.0));
    sprite.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    let ellipse = s.compositor.create_ellipse_geometry();
    ellipse.set_center(Vector::new(50.0, 50.0));
    ellipse.set_radius(Vector::new(25.0, 25.0));
    let clip = s.compositor.create_geometric_clip_with_geometry(Some((*ellipse).clone()));
    assert_eq!("GeometricClip", clip.type_name());
    sprite.set_composition_clip(Some((*clip).clone()));
    assert!(GeometricClip::from_clip(&sprite.composition_clip().expect("the clip is set")).is_some());
    show(&s, &sprite);

    let entries = log.entries();
    let push = entries.iter().position(|e| e == "PushGeometryClip 25, 25, 50, 50").unwrap_or_else(|| panic!("{entries:?}"));
    let draw = entries.iter().position(|e| e == "DrawRectangle Blue none 0, 0, 100, 100 shadows=0").unwrap();
    assert!(push < draw && entries[draw..].iter().any(|e| e == "PopGeometryClip"), "{entries:?}");
    let server = s.server_object(sprite.server());
    assert_eq!(Some(LtrbRect::new(25.0, 25.0, 75.0, 75.0)), server.sub_tree_bounds());
    assert!(s.compositor.server().get::<ServerGeometricClip>(clip.server()).is_some());

    // A change of the geometry changes the clip.
    log.clear();
    ellipse.set_radius(Vector::new(10.0, 10.0));
    s.run_jobs();
    assert!(has_entry(&log.entries(), "PushGeometryClip 40, 40, 20, 20"), "{:?}", log.entries());
    assert_eq!(Some(LtrbRect::new(40.0, 40.0, 60.0, 60.0)), server.sub_tree_bounds());

    // A view box maps the geometry onto the visual.
    let view_box = s.compositor.create_view_box();
    view_box.set_size(Vector::new(200.0, 200.0));
    clip.set_view_box(Some(view_box));
    s.run_jobs();
    assert_eq!(Some(LtrbRect::new(20.0, 20.0, 30.0, 30.0)), server.sub_tree_bounds());

    // Without a geometry the clip clips nothing.
    log.clear();
    clip.set_geometry(None);
    s.run_jobs();
    assert_eq!(0, log.count("PushGeometryClip"), "{:?}", log.entries());
    assert_eq!(Some(LtrbRect::new(0.0, 0.0, 100.0, 100.0)), server.sub_tree_bounds());
}
