use super::clip::{clip_triangle, ClipVertex, Clipped, ATTRIBUTES};
use super::*;
use crate::test_scenes::{
    diffuse, emissive, model, orthographic, quad, reversed_quad, snapshot, white_ambient, SIZE,
};
use ferroui_base::media::Color;
use ferroui_base::rendering::scene3d::Scene3DSnapshot;

const RED: Color = Color::from_argb(255, 255, 0, 0);
const BLUE: Color = Color::from_argb(255, 0, 0, 255);
const GREEN: Color = Color::from_argb(255, 0, 255, 0);
const GRAY: Color = Color::from_argb(255, 128, 128, 128);
const WHITE: Color = Color::from_argb(255, 255, 255, 255);

const ALIASED: CpuRenderOptions = CpuRenderOptions { antialiased: false };

fn vertex(x: f32, y: f32, z: f32, w: f32, attribute: f32) -> ClipVertex {
    let mut attributes = [0.0; ATTRIBUTES];
    attributes[0] = attribute;
    ClipVertex { clip: [x, y, z, w], attributes }
}

fn draw(scene: &Scene3DSnapshot, options: CpuRenderOptions) -> Scene3DImage {
    render(scene, SIZE, 1.0, options)
}

#[test]
fn a_triangle_inside_the_planes_is_not_clipped() {
    let triangle = [vertex(-0.5, -0.5, 0.5, 1.0, 0.0), vertex(0.5, -0.5, 0.5, 1.0, 0.0), vertex(0.0, 0.5, 0.5, 1.0, 0.0)];
    assert!(matches!(clip_triangle(&triangle), Clipped::Inside));
}

#[test]
fn a_triangle_behind_the_near_plane_or_beyond_the_far_plane_is_dropped() {
    let behind = [vertex(-0.5, -0.5, -0.1, 1.0, 0.0), vertex(0.5, -0.5, -0.2, 1.0, 0.0), vertex(0.0, 0.5, -0.3, 1.0, 0.0)];
    assert!(matches!(clip_triangle(&behind), Clipped::Outside));
    let beyond = [vertex(-0.5, -0.5, 1.1, 1.0, 0.0), vertex(0.5, -0.5, 1.2, 1.0, 0.0), vertex(0.0, 0.5, 1.3, 1.0, 0.0)];
    assert!(matches!(clip_triangle(&beyond), Clipped::Outside));
    let not_finite = [vertex(f32::NAN, 0.0, 0.5, 1.0, 0.0), vertex(0.5, -0.5, 0.5, 1.0, 0.0), vertex(0.0, 0.5, 0.5, 1.0, 0.0)];
    assert!(matches!(clip_triangle(&not_finite), Clipped::Outside));
}

#[test]
fn a_triangle_across_the_near_plane_is_cut_at_it_with_its_attributes() {
    // One vertex behind the near plane (z = -0.5), two in front (z = 0.5): the polygon is a
    // quadrilateral whose new vertices are at z = 0, halfway, with the attribute halfway.
    let triangle = [vertex(0.0, 0.5, -0.5, 1.0, 0.0), vertex(-0.5, -0.5, 0.5, 1.0, 1.0), vertex(0.5, -0.5, 0.5, 1.0, 1.0)];
    let Clipped::Polygon(polygon) = clip_triangle(&triangle) else { panic!("the triangle is cut") };
    let vertices = polygon.as_slice();
    assert_eq!(4, vertices.len());
    for vertex in vertices {
        assert!(vertex.clip[2] >= -1e-6, "{vertex:?} is in front of the near plane");
    }
    let cut: Vec<_> = vertices.iter().filter(|vertex| vertex.clip[2].abs() < 1e-6).collect();
    assert_eq!(2, cut.len());
    for vertex in cut {
        assert!((vertex.attributes[0] - 0.5).abs() < 1e-6, "{vertex:?}");
        assert!((vertex.clip[1] - 0.0).abs() < 1e-6, "{vertex:?}");
    }
}

#[test]
fn a_triangle_across_the_far_plane_is_cut_at_it() {
    let triangle = [vertex(0.0, 0.5, 1.5, 1.0, 0.0), vertex(-0.5, -0.5, 0.5, 1.0, 0.0), vertex(0.5, -0.5, 0.5, 1.0, 0.0)];
    let Clipped::Polygon(polygon) = clip_triangle(&triangle) else { panic!("the triangle is cut") };
    for vertex in polygon.as_slice() {
        assert!(vertex.clip[2] <= vertex.clip[3] + 1e-6, "{vertex:?} is before the far plane");
    }
    assert_eq!(4, polygon.as_slice().len());
}

#[test]
fn a_quad_beyond_the_far_plane_draws_nothing() {
    // The camera is at z = 5 with its far plane at 10: z = -6 is beyond it.
    let scene = snapshot(orthographic(), vec![model(quad(1, -1.0, -1.0, 1.0, 1.0, -6.0), vec![diffuse(RED)], vec![])], vec![white_ambient()]);
    assert!(draw(&scene, CpuRenderOptions::default()).is_transparent());
}

#[test]
fn a_shared_edge_is_covered_once_and_the_rule_is_top_left() {
    // A quad from x = -0.51 to 0.51 covers the pixels from 24.5 to 75.5 of 100: its left and top
    // edges pass through the centres of column and row 24, its right and bottom edges through
    // those of column and row 75, and its diagonal (shared by its two triangles) through the
    // centres of the pixels with x + y = 99. An emissive layer is added: a sample covered twice
    // would be twice as bright.
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, -0.51, -0.51, 0.51, 0.51, 0.0), vec![emissive(GRAY)], vec![])],
        vec![],
    );
    let image = draw(&scene, ALIASED);
    for y in 0..100 {
        for x in 0..100 {
            let inside = (24..=74).contains(&x) && (24..=74).contains(&y);
            let expected = if inside { [128, 128, 128, 128] } else { [0, 0, 0, 0] };
            assert_eq!(expected, image.pixel(x, y), "pixel {x}, {y}");
        }
    }
}

#[test]
fn an_edge_through_a_pixel_is_resolved_from_four_samples() {
    // The left edge of the quad at 50.5 pixels: of the samples of pixel 50, those at 50.75 are
    // covered and those at 50.25 are not.
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, 0.01, -1.0, 1.0, 1.0, 0.0), vec![emissive(WHITE)], vec![])],
        vec![],
    );
    let image = draw(&scene, CpuRenderOptions::default());
    assert_eq!([0, 0, 0, 0], image.pixel(49, 50));
    assert_eq!([128, 128, 128, 128], image.pixel(50, 50));
    assert_eq!([255, 255, 255, 255], image.pixel(51, 50));
    // Aliased: the centre of pixel 50 is on the left edge, which covers it.
    let aliased = draw(&scene, ALIASED);
    assert_eq!([255, 255, 255, 255], aliased.pixel(50, 50));
    assert_eq!([0, 0, 0, 0], aliased.pixel(49, 50));
}

#[test]
fn the_nearer_surface_hides_the_farther_in_either_order() {
    let near = || model(quad(1, -0.5, -0.5, 0.5, 0.5, 0.0), vec![diffuse(RED)], vec![]);
    let far = || model(quad(2, -0.8, -0.8, 0.8, 0.8, -1.0), vec![diffuse(BLUE)], vec![]);
    for models in [vec![near(), far()], vec![far(), near()]] {
        let image = draw(&snapshot(orthographic(), models, vec![white_ambient()]), ALIASED);
        assert_eq!([255, 0, 0, 255], image.pixel(50, 50));
        assert_eq!([0, 0, 255, 255], image.pixel(15, 50));
        assert_eq!([0, 0, 0, 0], image.pixel(5, 50));
    }
}

#[test]
fn the_later_layers_of_a_material_draw_where_its_first_diffuse_layer_did() {
    // The near model's emissive layer adds to its diffuse layer (LessEqual on the same depth); the
    // far model behind it draws nothing where it is hidden, its emissive layer included.
    let near = model(quad(1, -0.5, -0.5, 0.5, 0.5, 0.0), vec![diffuse(GRAY), emissive(BLUE)], vec![]);
    let far = model(quad(2, -0.8, -0.8, 0.8, 0.8, -1.0), vec![diffuse(RED), emissive(GREEN)], vec![]);
    let image = draw(&snapshot(orthographic(), vec![near, far], vec![white_ambient()]), ALIASED);
    assert_eq!([128, 128, 255, 255], image.pixel(50, 50));
    assert_eq!([255, 255, 0, 255], image.pixel(15, 50));
}

#[test]
fn a_back_face_draws_the_back_material_and_is_culled_without_one() {
    let with_back = model(reversed_quad(1, -0.9, -0.5, -0.1, 0.5, 0.0), vec![diffuse(RED)], vec![diffuse(BLUE)]);
    let without_back = model(reversed_quad(2, 0.1, -0.5, 0.9, 0.5, 0.0), vec![diffuse(RED)], vec![]);
    let front_only = model(quad(3, -0.5, 0.6, 0.5, 0.9, 0.0), vec![diffuse(GREEN)], vec![diffuse(RED)]);
    let lights = vec![ferroui_base::rendering::scene3d::Scene3DLight::Directional {
        color: WHITE,
        direction: ferroui_base::numerics::Vector3::new(0.0, 0.0, -1.0),
    }];
    let image = draw(&snapshot(orthographic(), vec![with_back, without_back, front_only], lights), ALIASED);
    // The back face's normal (-z) is turned toward the camera, and lit by the light from it.
    assert_eq!([0, 0, 255, 255], image.pixel(25, 50));
    assert_eq!([0, 0, 0, 0], image.pixel(75, 50));
    assert_eq!([0, 255, 0, 255], image.pixel(50, 12));
}

#[test]
fn the_attributes_are_interpolated_with_the_perspective() {
    // An edge from a vertex at w = 1 to one at w = 3: the point a quarter of the way on the
    // screen from the far end is the middle of the edge in the world (screen weights 0.25 and
    // 0.75 become 0.5 and 0.5), and the attribute there is the mean of its ends.
    let mut attributes = [[0.0; ATTRIBUTES]; 3];
    attributes[1][0] = 1.0;
    attributes[2][0] = 7.0;
    let triangle = SetupTriangle {
        step_x: [0; 3],
        step_y: [0; 3],
        origin: [0; 3],
        bias: [0; 3],
        inverse_area: 1.0,
        depth: [0.0; 3],
        inverse_w: [1.0, 1.0 / 3.0, 1.0],
        attributes,
        bounds: [0; 4],
        front: true,
    };
    let middle = interpolate(&triangle, [0.25, 0.75, 0.0]);
    assert!((middle[0] - 0.5).abs() < 1e-6, "{}", middle[0]);
    // Affine interpolation would give 0.75: the perspective matters.
    let at_vertex = interpolate(&triangle, [0.0, 0.0, 1.0]);
    assert!((at_vertex[0] - 7.0).abs() < 1e-6);
}

#[test]
fn an_empty_scene_and_a_scene_without_a_camera_are_transparent() {
    let empty = snapshot(orthographic(), vec![], vec![white_ambient()]);
    let image = draw(&empty, CpuRenderOptions::default());
    assert_eq!((100, 100), (image.width, image.height));
    assert!(image.is_transparent());

    let mut without_camera = (*snapshot(orthographic(), vec![model(quad(1, -1.0, -1.0, 1.0, 1.0, 0.0), vec![diffuse(RED)], vec![])], vec![white_ambient()])).clone();
    without_camera.camera = None;
    assert!(draw(&without_camera, CpuRenderOptions::default()).is_transparent());

    let no_pixels = render(&empty, PixelSize::new(0, 10), 1.0, CpuRenderOptions::default());
    assert!(no_pixels.rgba.is_empty());
}

#[test]
fn rendering_is_deterministic_and_counted() {
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, -0.7, -0.3, 0.6, 0.8, 0.0), vec![diffuse(RED)], vec![])],
        vec![white_ambient()],
    );
    let before = scenes_rasterized();
    let mut rasterizer = CpuRasterizer::new();
    let first = rasterizer.render(&scene, SIZE, 1.0, CpuRenderOptions::default());
    let second = rasterizer.render(&scene, SIZE, 1.0, CpuRenderOptions::default());
    assert_eq!(first, second);
    assert_eq!(first, render(&scene, SIZE, 1.0, CpuRenderOptions::default()));
    assert_eq!(before + 3, scenes_rasterized());
}

#[test]
fn a_large_image_is_drawn_in_tiles_without_seams() {
    // 300 by 200 pixels: five by four tiles. A quad over everything is one colour everywhere.
    let scene = snapshot(
        orthographic(),
        vec![model(quad(1, -2.0, -2.0, 2.0, 2.0, 0.0), vec![diffuse(GREEN)], vec![])],
        vec![white_ambient()],
    );
    let image = render(&scene, PixelSize::new(300, 200), 1.0, CpuRenderOptions::default());
    assert!(image.rgba.chunks_exact(4).all(|pixel| pixel == [0, 255, 0, 255]));
}

// Step D6: the brushes of the layers.

/// Red to blue from the left of the box of the texture coordinates to its right.
fn horizontal_gradient(stops: usize) -> ferroui_base::media::immutable::ImmutableLinearGradientBrush {
    use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
    let stops: Vec<ImmutableGradientStop> = (0..stops)
        .map(|index| {
            let f = index as f64 / (stops - 1) as f64;
            let level = (255.0 * f).round() as u8;
            ImmutableGradientStop::new(f, Color::from_argb(255, 255 - level, 0, level))
        })
        .collect();
    ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        ferroui_base::media::GradientSpreadMethod::Pad,
        Some(ferroui_base::RelativePoint::TOP_LEFT),
        Some(ferroui_base::RelativePoint::new(1.0, 0.0, ferroui_base::RelativeUnit::Relative)),
        None,
    )
}

/// The red to blue of a pixel column of the quad over the whole image (its texture coordinate
/// `u` is the centre of the column): within a level.
#[track_caller]
fn assert_gradient_column(image: &Scene3DImage, x: u32, y: u32, alpha: u8) {
    let u = (f64::from(x) + 0.5) / 100.0;
    let pixel = image.pixel(x, y);
    let expected = [((1.0 - u) * 255.0).round() as u8, 0, (u * 255.0).round() as u8];
    for i in 0..3 {
        assert!(pixel[i].abs_diff(expected[i]) <= 1, "column {x}: {pixel:?}, expected {expected:?}");
    }
    assert_eq!(alpha, pixel[3], "column {x}");
}

#[test]
fn a_diffuse_layer_paints_its_gradient_over_the_texture_coordinates() {
    use crate::test_scenes::brush_layer;
    use ferroui_base::rendering::scene3d::Scene3DMaterialKind;
    for stops in [2, 12] {
        let brush = horizontal_gradient(stops);
        let layers = vec![brush_layer(Scene3DMaterialKind::Diffuse, &brush)];
        let quad = model(quad(1, -1.0, -1.0, 1.0, 1.0, 0.0), layers, vec![]);
        let before = crate::layers_with_unsampled_brushes();
        let image = draw(&snapshot(orthographic(), vec![quad], vec![white_ambient()]), CpuRenderOptions::default());
        for x in [0, 25, 50, 77, 99] {
            assert_gradient_column(&image, x, 50, 255);
        }
        assert_eq!(before, crate::layers_with_unsampled_brushes(), "{stops} stops are sampled");
    }
}

#[test]
fn the_specular_and_emissive_layers_of_a_group_add_their_brushes() {
    use crate::test_scenes::{brush_layer, image_brush, PixelBitmap};
    use ferroui_base::media::{Stretch, TileMode};
    use ferroui_base::rendering::scene3d::{Scene3DLight, Scene3DMaterialKind};
    use ferroui_base::RelativeRect;
    let black = Color::from_argb(255, 0, 0, 0);
    // Left: a black diffuse layer and the gradient as the brush of a specular layer, lit head on
    // (the highlight is the light's colour everywhere: N, L and H along +z).
    let gradient = horizontal_gradient(2);
    let specular = model(
        quad(1, -1.0, -1.0, 1.0, 0.0, 0.0),
        vec![diffuse(black), brush_layer(Scene3DMaterialKind::Specular, &gradient)],
        vec![],
    );
    // Right half of the top: a black diffuse layer and an image (green) as the brush of an
    // emissive layer.
    let green = PixelBitmap::rgba(2, 2, [0, 255, 0, 255].repeat(4));
    let image = image_brush(&green, Stretch::Fill, TileMode::None, RelativeRect::FILL, RelativeRect::FILL);
    let emissive_model = model(
        quad(2, -1.0, 0.0, 1.0, 1.0, 0.0),
        vec![diffuse(black), brush_layer(Scene3DMaterialKind::Emissive, &image)],
        vec![],
    );
    let direction = ferroui_base::numerics::Vector3::new(0.0, 0.0, -1.0);
    let light = Scene3DLight::Directional { color: WHITE, direction };
    let scene = snapshot(orthographic(), vec![specular, emissive_model], vec![light]);
    let image = draw(&scene, CpuRenderOptions::default());
    // The bottom half: the gradient of the specular layer added to black.
    for x in [10, 40, 90] {
        assert_gradient_column(&image, x, 75, 255);
    }
    // The top half: the image of the emissive layer added to black.
    assert_eq!([0, 255, 0, 255], image.pixel(50, 25));
}

#[test]
fn the_texels_of_a_brush_are_read_once_while_the_rasterizer_keeps_them() {
    use crate::test_scenes::{brush_layer, image_brush, PixelBitmap};
    use ferroui_base::media::{Stretch, TileMode};
    use ferroui_base::rendering::scene3d::Scene3DMaterialKind;
    use ferroui_base::RelativeRect;
    let bitmap = PixelBitmap::rgba(1, 1, vec![255, 0, 0, 255]);
    let brush = image_brush(&bitmap, Stretch::Fill, TileMode::None, RelativeRect::FILL, RelativeRect::FILL);
    let gradient = horizontal_gradient(12);
    let models = vec![
        model(quad(1, -1.0, -1.0, 0.0, 1.0, 0.0), vec![brush_layer(Scene3DMaterialKind::Diffuse, &brush)], vec![]),
        model(quad(2, 0.0, -1.0, 1.0, 1.0, 0.0), vec![brush_layer(Scene3DMaterialKind::Diffuse, &gradient)], vec![]),
    ];
    let scene = snapshot(orthographic(), models, vec![white_ambient()]);
    let mut rasterizer = CpuRasterizer::new();
    let before = brush_textures_read();
    let first = rasterizer.render(&scene, SIZE, 1.0, CpuRenderOptions::default());
    assert_eq!(before + 2, brush_textures_read(), "the image and the lookup texture");
    assert_eq!([255, 0, 0, 255], first.pixel(25, 50));
    rasterizer.render(&scene, SIZE, 1.0, CpuRenderOptions::default());
    assert_eq!(before + 2, brush_textures_read(), "kept");
    // New pixels of the bitmap: a new version, read again.
    bitmap.set_pixels(vec![0, 0, 255, 255]);
    let third = rasterizer.render(&scene, SIZE, 1.0, CpuRenderOptions::default());
    assert_eq!(before + 3, brush_textures_read());
    assert_eq!([0, 0, 255, 255], third.pixel(25, 50));
}

#[test]
fn the_content_of_a_drawing_brush_is_rendered_once_per_generation_and_sampled_as_an_image() {
    use ferroui_base::media::{Brushes, DrawingBrush, GeometryDrawing, IBrush, RectangleGeometry, SharedBrush, Stretch};
    use ferroui_base::rendering::scene3d::{Scene3DBrushContent, Scene3DMaterialKind, Scene3DMaterialLayer};
    use ferroui_base::rendering::testing::MockPlatformRenderInterface;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    let (_scope, _) = MockPlatformRenderInterface::install();
    let drawing = GeometryDrawing::new();
    drawing.set_brush(Some(Brushes::red()));
    let rect = ferroui_base::Rect::new(0.0, 0.0, 20.0, 10.0);
    drawing.set_geometry(RectangleGeometry::with_rect(rect).upcast::<ferroui_base::media::Geometry>());
    let brush = DrawingBrush::new();
    brush.set_drawing(&drawing);
    brush.set_stretch(Stretch::Fill);
    let handle: Rc<dyn IBrush> = brush.into();
    let layer = |generation: u64| {
        let content = Scene3DBrushContent::record(handle.as_scene_brush().unwrap(), 1, generation).unwrap();
        Scene3DMaterialLayer {
            brush: SharedBrush::from_scene_content(&*handle, content),
            ..crate::test_scenes::layer(Scene3DMaterialKind::Diffuse, WHITE)
        }
    };
    let scene = |generation: u64| {
        snapshot(orthographic(), vec![model(quad(1, -1.0, -1.0, 1.0, 1.0, 0.0), vec![layer(generation)], vec![])], vec![
            white_ambient(),
        ])
    };

    // The renderer stands for the 2D renderer of a backend: it paints the left half of the
    // content green and its right half blue, and records the sizes it was asked for.
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let asked = sizes.clone();
    let mut rasterizer = CpuRasterizer::new();
    rasterizer.set_content_renderer(Some(Box::new(move |content: &Scene3DBrushContent, (width, height)| {
        asked.lock().unwrap().push((content.generation(), width, height));
        let mut rgba = Vec::new();
        for _ in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(if x < width / 2 { &[0, 255, 0, 255] } else { &[0, 0, 255, 255] });
            }
        }
        Some(crate::brush::BrushTexels { width, height, rgba, premultiplied: true })
    })));

    let first = rasterizer.render(&scene(1), SIZE, 2.0, CpuRenderOptions::default());
    // The content of 20 by 10 units at 2 pixels a unit.
    assert_eq!(vec![(1, 40, 20)], *sizes.lock().unwrap());
    assert_eq!([0, 255, 0, 255], first.pixel(20, 50));
    assert_eq!([0, 0, 255, 255], first.pixel(80, 50));
    // The same generation in a new snapshot: not rendered again.
    rasterizer.render(&scene(1), SIZE, 2.0, CpuRenderOptions::default());
    assert_eq!(1, sizes.lock().unwrap().len());
    // A new generation is rendered again.
    rasterizer.render(&scene(2), SIZE, 2.0, CpuRenderOptions::default());
    assert_eq!(vec![(1, 40, 20), (2, 40, 20)], *sizes.lock().unwrap());

    // Without a renderer the layer is drawn with its colour, and counted.
    let before = crate::layers_with_unsampled_brushes();
    let plain = CpuRasterizer::new().render(&scene(3), SIZE, 1.0, CpuRenderOptions::default());
    assert_eq!([255, 255, 255, 255], plain.pixel(50, 50));
    assert!(crate::layers_with_unsampled_brushes() > before);
}
