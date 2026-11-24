//! The images in pixels: drawn through the drawing context of the Vello backend, in its CPU mode
//! and, where the machine has an adapter, its two GPU modes.

use crate::SvgDocument;
use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
use ferroui_base::platform::{IPlatformRenderInterface, IReadableBitmapImpl, PixelFormat};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Rect, Vector};
use ferroui_vello::{PlatformRenderInterface, VelloOptions, VelloRenderingMode};
use std::rc::Rc;

/// The error allowed between two modes: that of the render tests of the backends.
const ALLOWED_ERROR: f64 = 0.022;

struct Pixels {
    width: usize,
    rgba: Vec<u8>,
}

impl Pixels {
    fn at(&self, x: usize, y: usize) -> [u8; 4] {
        let start = (y * self.width + x) * 4;
        [self.rgba[start], self.rgba[start + 1], self.rgba[start + 2], self.rgba[start + 3]]
    }
}

fn read(bitmap: &dyn IReadableBitmapImpl) -> Pixels {
    let framebuffer = bitmap.lock();
    let size = framebuffer.size();
    let (width, height) = (size.width as usize, size.height as usize);
    let (row_bytes, format) = (framebuffer.row_bytes() as usize, framebuffer.format());
    let mut rgba = Vec::with_capacity(width * height * 4);
    framebuffer.with_data(&mut |data| {
        for y in 0..height {
            for pixel in data[y * row_bytes..y * row_bytes + width * 4].chunks_exact(4) {
                if format == PixelFormat::BGRA8888 {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                } else {
                    rgba.extend_from_slice(pixel);
                }
            }
        }
    });
    framebuffer.dispose();
    Pixels { width, rgba }
}

fn render(bytes: &[u8], mode: VelloRenderingMode, width: i32, height: i32) -> Pixels {
    let scope = FerroLocator::enter_scope();
    let interface: Rc<dyn IPlatformRenderInterface> =
        Rc::new(PlatformRenderInterface::new(VelloOptions::with_rendering_mode(mode)));
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(interface.clone());

    let document = SvgDocument::parse(bytes).unwrap();
    let image = document.to_image();
    let bitmap = interface.create_render_target_bitmap(PixelSize::new(width, height), Vector::new(96.0, 96.0));
    let mut target = bitmap.create_drawing_context();
    {
        let mut core = PlatformDrawingContext::borrowed(&mut *target);
        let mut context = DrawingContext::new(&mut core);
        let source = Rect::new(0.0, 0.0, document.size().width, document.size().height);
        image.draw(&mut context, source, Rect::new(0.0, 0.0, f64::from(width), f64::from(height)));
    }
    target.dispose();
    let pixels = read(&*bitmap);
    bitmap.dispose();
    scope.dispose();
    pixels
}

/// The rendering modes this machine can draw in.
fn modes() -> Vec<VelloRenderingMode> {
    [VelloRenderingMode::Cpu, VelloRenderingMode::Hybrid, VelloRenderingMode::Gpu]
        .into_iter()
        .filter(|mode| ferroui_vello::scene::try_create_scene_sink(*mode, 1, 1).is_ok())
        .collect()
}

fn cpu(body: &str) -> Pixels {
    let text = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">{body}</svg>"#);
    render(text.as_bytes(), VelloRenderingMode::Cpu, 64, 64)
}

fn near(actual: [u8; 4], expected: [u8; 4], tolerance: u8) -> bool {
    actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= tolerance)
}

#[test]
fn fill_stroke_and_transform_land_where_the_document_says() {
    let pixels = cpu(r##"<rect x="8" y="8" width="16" height="16" fill="#ff0000"/>
        <g transform="translate(32 0)"><rect x="8" y="8" width="16" height="16" fill="none" stroke="#0000ff" stroke-width="4"/></g>"##);
    assert_eq!(pixels.at(16, 16), [255, 0, 0, 255]);
    assert_eq!(pixels.at(4, 4), [0, 0, 0, 0]);
    // On the stroke of the second rectangle, and inside it.
    assert_eq!(pixels.at(40, 16), [0, 0, 255, 255]);
    assert_eq!(pixels.at(48, 16), [0, 0, 0, 0]);
}

#[test]
fn a_gradient_follows_its_units_and_its_transform() {
    // In the bounding box of the rectangle, from its left to its right.
    let pixels = cpu(r##"<defs><linearGradient id="g"><stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs>
        <rect x="16" y="0" width="32" height="64" fill="url(#g)"/>"##);
    assert!(pixels.at(17, 32)[0] < 24, "{:?}", pixels.at(17, 32));
    assert!(pixels.at(32, 32)[0].abs_diff(128) < 16, "{:?}", pixels.at(32, 32));
    assert!(pixels.at(46, 32)[0] > 230, "{:?}", pixels.at(46, 32));

    // In the user space, turned to run from top to bottom.
    let pixels = cpu(r##"<defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="64" y2="0" gradientTransform="matrix(0 1 1 0 0 0)">
        <stop offset="0" stop-color="#000"/><stop offset="1" stop-color="#fff"/></linearGradient></defs><rect width="64" height="64" fill="url(#g)"/>"##);
    assert!(pixels.at(32, 2)[0] < 24, "{:?}", pixels.at(32, 2));
    assert!(pixels.at(32, 61)[0] > 230, "{:?}", pixels.at(32, 61));
    assert!(pixels.at(2, 32)[0].abs_diff(pixels.at(61, 32)[0]) < 4);

    // A radial gradient with its centre scaled by the transform.
    let pixels = cpu(r##"<defs><radialGradient id="r" gradientUnits="userSpaceOnUse" cx="16" cy="16" r="16" gradientTransform="scale(2)">
        <stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></radialGradient></defs><rect width="64" height="64" fill="url(#r)"/>"##);
    assert!(pixels.at(32, 32)[0] > 230, "{:?}", pixels.at(32, 32));
    assert!(pixels.at(1, 32)[0] < 24, "{:?}", pixels.at(1, 32));
}

#[test]
fn a_clip_path_and_group_opacity_apply() {
    let pixels = cpu(r##"<defs><clipPath id="c"><rect x="0" y="0" width="32" height="64"/></clipPath></defs>
        <rect width="64" height="64" fill="#00ff00" clip-path="url(#c)"/>"##);
    assert_eq!(pixels.at(16, 32), [0, 255, 0, 255]);
    assert_eq!(pixels.at(48, 32), [0, 0, 0, 0]);

    // Two overlapping shapes in a group at half opacity: the overlap is not darker.
    let text = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><g opacity="0.5">
        <rect width="40" height="64" fill="#ff0000"/><rect x="20" width="44" height="64" fill="#ff0000"/></g></svg>"##;
    for mode in modes() {
        let pixels = render(text, mode, 64, 64);
        assert!(near(pixels.at(10, 32), pixels.at(30, 32), 2), "{mode:?}: {:?} {:?}", pixels.at(10, 32), pixels.at(30, 32));
        assert!(pixels.at(30, 32)[3].abs_diff(128) <= 2, "{mode:?}: {:?}", pixels.at(30, 32));
    }
}

#[test]
fn a_dash_array_is_in_user_units() {
    let pixels = cpu(r##"<line x1="0" y1="32" x2="64" y2="32" stroke="#ff0000" stroke-width="4" stroke-dasharray="8 8"/>"##);
    assert_eq!(pixels.at(4, 32), [255, 0, 0, 255]);
    assert_eq!(pixels.at(12, 32), [0, 0, 0, 0]);
    assert_eq!(pixels.at(20, 32), [255, 0, 0, 255]);
}

#[test]
fn the_image_is_sharp_at_a_large_scale() {
    // A viewBox of eight units drawn at 512 pixels: the edge between the halves stays one pixel
    // wide, as a vector does and a scaled bitmap would not.
    let text = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><rect width="4" height="8" fill="#ffffff"/></svg>"##;
    let pixels = render(text, VelloRenderingMode::Cpu, 512, 512);
    assert_eq!(pixels.at(255, 256), [255, 255, 255, 255]);
    assert_eq!(pixels.at(256, 256), [0, 0, 0, 0]);
}

/// The root mean square of the differences, in 0..1.
fn error(left: &Pixels, right: &Pixels) -> f64 {
    let sum: f64 = left.rgba.iter().zip(&right.rgba).map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2)).sum();
    (sum / left.rgba.len() as f64).sqrt() / 255.0
}

#[test]
fn the_sample_of_the_gallery_draws_alike_in_every_rendering_mode() {
    let bytes = include_bytes!("../../samples/WinUIGallery/Assets/SampleMedia/MirrorPCConsent.svg");
    let reference = render(bytes, VelloRenderingMode::Cpu, 656, 328);
    assert!(reference.rgba.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 656 * 328 / 2, "the image is drawn");
    for mode in modes().into_iter().filter(|mode| *mode != VelloRenderingMode::Cpu) {
        let drawn = render(bytes, mode, 656, 328);
        let difference = error(&reference, &drawn);
        println!("{mode:?}: error {difference:.4}");
        assert!(difference <= ALLOWED_ERROR, "{mode:?}: error {difference}");
    }
}
