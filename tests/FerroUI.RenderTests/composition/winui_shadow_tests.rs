//! Render tests of the drop shadow of an alpha mask (not from upstream;
//! docs/porting/winui-composition.md, phase 7): the trace pointer shadow of
//! the Windows Calculator (`AddTracePointerShadow` of
//! `Views/GraphingCalculator/GraphingCalculator.xaml.cs`: a `DropShadow` with
//! `BlurRadius` 6, `Opacity` 0.33 and `Offset` (2, 2, 0) whose `Mask` is
//! `CursorPath.GetAlphaMask()`, on a sprite visual set with
//! `SetElementChildVisual`), rendered through the compositor at several scales
//! and DPIs. The same assertions run against the Skia backend and, with the
//! feature `vello`, against the Vello backend: the shadow follows the alpha of
//! the path, not its rectangle, on both.

use crate::manual_render_timer::ManualRenderTimer;
use crate::test_render_helper::{begin_test, end_test, get_output_directory, Rgba32Image};
use crate::test_render_root::TestRenderRoot;
use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::media::{Brushes, Geometry, MediaContext, ScaleTransform, Shadow, ThemeShadow};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{IPlatformRenderInterface, IWriteableBitmapImpl};
use ferroui_base::rendering::composition::{CompositingRenderer, Compositor, RenderSurfaces};
use ferroui_base::rendering::{IRenderer, RenderLoop};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Ref, RelativePoint, RelativeUnit, Vector, Vector3D};
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::shapes::Path;
use ferroui_controls::{Border, Canvas, Control};
use std::any::Any;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

struct BitmapSurface {
    bitmap: Arc<dyn IWriteableBitmapImpl>,
}

impl IPlatformRenderSurface for BitmapSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for BitmapSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let bitmap = self.bitmap.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || bitmap.lock()))
    }
}

/// Renders `target` through the compositor at `scaling` into `name`.png of the output directory
/// and returns the image. `compose` runs once the tree is laid out and attached to the renderer
/// (where the application calls `GetAlphaMask` and `SetElementChildVisual`).
fn render(target: &Ref<Control>, scaling: f64, name: &str, compose: impl FnOnce()) -> Rgba32Image {
    let factory = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
    let pixel_size =
        PixelSize::new((target.width() * scaling).round() as i32, (target.height() * scaling).round() as i32);
    let compositor = Compositor::with_scheduler(
        RenderLoop::from_timer(Arc::new(ManualRenderTimer::new())),
        None,
        true,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );
    let bitmap = factory.create_writeable_bitmap(
        pixel_size,
        Vector::new(96.0 * scaling, 96.0 * scaling),
        factory.default_pixel_format(),
        factory.default_alpha_format(),
    );
    let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(BitmapSurface { bitmap: bitmap.clone() });
    let surfaces: RenderSurfaces = Arc::new(move || vec![surface.clone()]);
    let root = TestRenderRoot::new(scaling);
    let renderer = CompositingRenderer::new(&root.source(), &compositor, surfaces);
    root.initialize(&renderer, target);
    renderer.start();
    Dispatcher::ui_thread().run_jobs(None);
    compose();
    Dispatcher::ui_thread().run_jobs(None);
    renderer.paint(ferroui_base::Rect::from_size(root.bounds().size()));
    renderer.dispose();
    root.set_child(None);
    root.release();

    let directory: PathBuf = get_output_directory().join("Composition").join("WinUIShadows");
    std::fs::create_dir_all(&directory).expect("the output directory is created");
    let path = directory.join(format!("{name}.composited.out.png"));
    let mut file = std::fs::File::create(&path).expect("the output file is created");
    bitmap.save(&mut file, &PngBitmapEncoderOptions::default().into()).expect("the image is saved");
    file.flush().expect("the image is written");
    bitmap.dispose();
    Rgba32Image::load(&path)
}

/// The pixel at the point (`x`, `y`) of the scene (in device-independent units).
fn pixel(image: &Rgba32Image, scaling: f64, x: f64, y: f64) -> [u8; 4] {
    let (px, py) = ((x * scaling) as usize, (y * scaling) as usize);
    image.pixels[py * image.width + px]
}

/// How dark a pixel of the white scene is: 0 for white, 255 for black.
fn darkness(pixel: [u8; 4]) -> u8 {
    255 - pixel[0].min(pixel[1]).min(pixel[2])
}

/// The scene of the trace pointer: a white 120 x 120 canvas; `CursorShadow`, an element without
/// a background, at (30, 30) of 40 x 40, and over it `CursorPath`, a black triangle filling the
/// top left half of the same square. `scale` scales both through a render transform.
fn trace_pointer_scene(scale: f64) -> (Ref<Canvas>, Ref<Path>, Ref<Border>) {
    let scene = Canvas::new();
    scene.set_width(120.0);
    scene.set_height(120.0);
    scene.set_background(Some(Brushes::white()));
    let holder = Canvas::new();
    holder.set_width(120.0);
    holder.set_height(120.0);
    if scale != 1.0 {
        holder.set_render_transform(Some((&ScaleTransform::with_scale(scale, scale)).into()));
        holder.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
    }
    scene.children().add(holder.clone());

    let cursor_shadow = Border::new();
    cursor_shadow.set_width(40.0);
    cursor_shadow.set_height(40.0);
    Canvas::set_left(&cursor_shadow, 30.0 / scale);
    Canvas::set_top(&cursor_shadow, 30.0 / scale);
    holder.children().add(cursor_shadow.clone());

    let cursor_path = Path::new();
    cursor_path.set_data(Geometry::parse("M 0,0 L 40,0 L 0,40 Z").ok());
    cursor_path.set_fill(Some(Brushes::black()));
    cursor_path.set_width(40.0);
    cursor_path.set_height(40.0);
    Canvas::set_left(&cursor_path, 30.0 / scale);
    Canvas::set_top(&cursor_path, 30.0 / scale);
    holder.children().add(cursor_path.clone());
    (scene, cursor_path, cursor_shadow)
}

/// `AddTracePointerShadow`.
fn add_trace_pointer_shadow(cursor_path: &Path, cursor_shadow: &Border) {
    let visual = ElementCompositionPreview::get_element_visual(cursor_path).expect("the path has a visual");
    let compositor = visual.compositor().clone();
    let shadow = compositor.create_drop_shadow();
    shadow.set_blur_radius(6.0);
    shadow.set_opacity(0.33);
    shadow.set_offset(Vector3D::new(2.0, 2.0, 0.0));
    shadow.set_mask(Some(cursor_path.get_alpha_mask().expect("the path has an alpha mask")));
    let sprite = compositor.create_sprite_visual();
    sprite.set_size(Vector::new(40.0, 40.0));
    sprite.set_shadow(Some(shadow));
    ElementCompositionPreview::set_element_child_visual(cursor_shadow, Some((*sprite).clone()));
}

/// The shadow is cast by the triangle: it darkens the white beside the edges the offset moves it
/// past, and leaves the empty half of the square white, where the shadow of the rectangle would
/// be.
fn assert_follows_the_alpha(image: &Rgba32Image, scaling: f64, scale: f64, name: &str) {
    // In the scene the square is at (30, 30) of 40 * scale.
    let at = |x: f64, y: f64| pixel(image, scaling, 30.0 + x * scale, 30.0 + y * scale);
    // Inside the triangle: the black path.
    assert!(darkness(at(8.0, 8.0)) > 240, "{name}: the path is drawn: {:?}", at(8.0, 8.0));
    // Below the top edge's end and right of it, within the blur: the shadow of the triangle.
    let beside = at(41.5, 4.0);
    assert!(darkness(beside) > 12, "{name}: the shadow of the triangle is there: {beside:?}");
    // Below the left edge's end: the shadow of the triangle too.
    let below = at(4.0, 41.5);
    assert!(darkness(below) > 12, "{name}: the shadow of the triangle is there: {below:?}");
    // The empty half of the square, far from the slanted edge: white. A shadow of the rectangle
    // (33 % black, blurred by 6) would make it about 85 dark.
    let empty = at(36.0, 36.0);
    assert!(darkness(empty) < 6, "{name}: no shadow of the rectangle in the empty half: {empty:?}");
    // Outside the shadow altogether: white.
    let far = pixel(image, scaling, 5.0, 115.0);
    assert!(darkness(far) < 3, "{name}: the background is white: {far:?}");
}

#[test]
fn the_trace_pointer_shadow_follows_the_alpha_of_the_path_at_several_dpis() {
    let scope = begin_test();
    for scaling in [1.0, 1.5, 2.0] {
        let (scene, cursor_path, cursor_shadow) = trace_pointer_scene(1.0);
        let name = format!("trace_pointer_dpi_{}", (scaling * 96.0) as i32);
        let image = render(&scene.clone().upcast(), scaling, &name, || {
            add_trace_pointer_shadow(&cursor_path, &cursor_shadow);
        });
        assert_follows_the_alpha(&image, scaling, 1.0, &name);
    }
    end_test(scope);
}

#[test]
fn the_trace_pointer_shadow_follows_the_alpha_of_the_path_at_several_scales() {
    let scope = begin_test();
    for (scale, scaling) in [(1.5, 1.0), (2.0, 1.0), (2.0, 2.0)] {
        let (scene, cursor_path, cursor_shadow) = trace_pointer_scene(scale);
        let name = format!("trace_pointer_scale_{}_dpi_{}", (scale * 10.0) as i32, (scaling * 96.0) as i32);
        let image = render(&scene.clone().upcast(), scaling, &name, || {
            add_trace_pointer_shadow(&cursor_path, &cursor_shadow);
        });
        assert_follows_the_alpha(&image, scaling, scale, &name);
    }
    end_test(scope);
}

#[test]
fn the_mask_follows_a_change_of_the_path() {
    let scope = begin_test();
    let (scene, cursor_path, cursor_shadow) = trace_pointer_scene(1.0);
    // The mask is taken, then the path changes to the bottom right triangle before the frame: the
    // shadow is the one of the new shape.
    let image = render(&scene.clone().upcast(), 1.0, "trace_pointer_changed_path", || {
        add_trace_pointer_shadow(&cursor_path, &cursor_shadow);
        cursor_path.set_data(Geometry::parse("M 40,0 L 40,40 L 0,40 Z").ok());
    });
    let at = |x: f64, y: f64| pixel(&image, 1.0, 30.0 + x, 30.0 + y);
    assert!(darkness(at(36.0, 36.0)) > 240, "the new path is drawn: {:?}", at(36.0, 36.0));
    // The top left half is empty now and has no shadow: the shadow moved right and down.
    assert!(darkness(at(4.0, 4.0)) < 6, "no shadow over the empty half: {:?}", at(4.0, 4.0));
    assert!(darkness(at(41.5, 38.0)) > 12, "the shadow of the new path: {:?}", at(41.5, 38.0));
    end_test(scope);
}

#[test]
fn a_theme_shadow_is_drawn_around_its_caster() {
    let scope = begin_test();
    let scene = Canvas::new();
    scene.set_width(200.0);
    scene.set_height(140.0);
    scene.set_background(Some(Brushes::white()));
    let card = Border::new();
    card.set_width(100.0);
    card.set_height(60.0);
    card.set_background(Some(Brushes::white()));
    Canvas::set_left(&card, 50.0);
    Canvas::set_top(&card, 30.0);
    // A flyout: Translation.Z 32, a directional shadow blurred by 16 and moved down by 8.
    card.set_translation(Vector3D::new(0.0, 0.0, 32.0));
    let shadow: Ref<Shadow> = ThemeShadow::new().upcast();
    card.set_shadow(Some(shadow));
    scene.children().add(card);
    let image = render(&scene.clone().upcast(), 1.0, "theme_shadow_flyout", || {});
    // Below the card: the shadow; above it, less (moved down); far away: none.
    let below = darkness(pixel(&image, 1.0, 100.0, 96.0));
    let above = darkness(pixel(&image, 1.0, 100.0, 27.0));
    assert!(below > 4, "a shadow under the card: {below}");
    assert!(below > above, "the shadow is moved down: {below} > {above}");
    assert!(darkness(pixel(&image, 1.0, 100.0, 60.0)) < 3, "the card is drawn over its shadow");
    assert!(darkness(pixel(&image, 1.0, 5.0, 5.0)) < 2, "no shadow far from the card");
    end_test(scope);
}
