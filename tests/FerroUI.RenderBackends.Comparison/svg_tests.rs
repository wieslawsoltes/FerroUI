//! An SVG image through the drawing context of each backend: the sample of the gallery, as
//! vectors, against the raster path of Skia.

use crate::{render, Backend, Pixels};
use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Rect};
use ferroui_svg::SvgDocument;

const SAMPLE: &[u8] = include_bytes!("../../samples/WinUIGallery/Assets/SampleMedia/MirrorPCConsent.svg");

/// The error allowed between two backends: that of the render tests of the backends.
const ALLOWED_ERROR: f64 = 0.022;

/// A luminance mask with a dark shape over a light one: the hole shows that the luminance of the
/// composed content is read, not the alpha of each shape.
const LUMINANCE_MASK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <rect width="64" height="64" fill="#ffffff"/><rect x="16" y="16" width="32" height="32" fill="#000000"/>
    </mask></defs><rect width="64" height="64" fill="#ff0000" mask="url(#m)"/></svg>"##;

const ALPHA_MASK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <mask id="m" style="mask-type: alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <rect width="32" height="64" fill="#000000"/>
    </mask></defs><rect width="64" height="64" fill="#00ff00" mask="url(#m)"/></svg>"##;

/// Tiles of 16 units with their left half red, from x = 8.
const TILED_PATTERN: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <pattern id="p" patternUnits="userSpaceOnUse" x="8" width="16" height="16"><rect width="8" height="16" fill="#ff0000"/></pattern>
    </defs><rect width="64" height="64" fill="url(#p)"/></svg>"##;

fn draw(backend: &Backend) -> Pixels {
    draw_document(backend, SAMPLE, PixelSize::new(656, 328))
}

fn draw_document(backend: &Backend, bytes: &[u8], size: PixelSize) -> Pixels {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(backend.interface.clone());
    let document = SvgDocument::parse(bytes).unwrap();
    let image = document.to_image();
    let source = Rect::new(0.0, 0.0, document.size().width, document.size().height);
    let target = Rect::new(0.0, 0.0, f64::from(size.width), f64::from(size.height));
    let pixels = render(backend, size, &|_, context| {
        let mut core = PlatformDrawingContext::borrowed(context);
        let mut drawing = DrawingContext::new(&mut core);
        image.draw(&mut drawing, source, target);
    });
    scope.dispose();
    pixels
}

/// The pixel at `(x, y)` of an image 64 pixels wide.
fn at(pixels: &Pixels, x: usize, y: usize) -> [u8; 4] {
    let start = (y * 64 + x) * 4;
    [pixels.rgba[start], pixels.rgba[start + 1], pixels.rgba[start + 2], pixels.rgba[start + 3]]
}

fn assert_near(pixels: &Pixels, name: &str, (x, y): (usize, usize), expected: [u8; 4]) {
    let actual = at(pixels, x, y);
    assert!(actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 3), "{name} at ({x}, {y}): {actual:?}, not {expected:?}");
}

fn backends() -> Vec<Backend> {
    let mut backends = vec![Backend::skia()];
    backends.extend(Backend::vello_modes().into_iter().map(Backend::vello));
    backends
}

/// The root mean square of the differences, in 0..1.
fn error(left: &Pixels, right: &Pixels) -> f64 {
    let sum: f64 = left.rgba.iter().zip(&right.rgba).map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2)).sum();
    (sum / left.rgba.len() as f64).sqrt() / 255.0
}

#[test]
fn an_svg_image_draws_alike_on_skia_and_every_vello_mode() {
    let reference = draw(&Backend::skia());
    assert!(reference.rgba.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 656 * 328 / 2, "the image is drawn");
    for mode in Backend::vello_modes() {
        let backend = Backend::vello(mode);
        let difference = error(&reference, &draw(&backend));
        println!("{}: error {difference:.4}", backend.name);
        assert!(difference <= ALLOWED_ERROR, "{}: error {difference}", backend.name);
    }
}

#[test]
fn a_luminance_mask_cuts_its_hole_on_skia_and_every_vello_mode() {
    for backend in backends() {
        let pixels = draw_document(&backend, LUMINANCE_MASK, PixelSize::new(64, 64));
        assert_near(&pixels, backend.name, (8, 8), [255, 0, 0, 255]);
        assert_near(&pixels, backend.name, (32, 32), [0, 0, 0, 0]);
    }
}

#[test]
fn an_alpha_mask_reads_its_alpha_on_skia_and_every_vello_mode() {
    for backend in backends() {
        let pixels = draw_document(&backend, ALPHA_MASK, PixelSize::new(64, 64));
        assert_near(&pixels, backend.name, (16, 32), [0, 255, 0, 255]);
        assert_near(&pixels, backend.name, (48, 32), [0, 0, 0, 0]);
    }
}

#[test]
fn a_pattern_repeats_its_tile_on_skia_and_every_vello_mode() {
    for backend in backends() {
        let pixels = draw_document(&backend, TILED_PATTERN, PixelSize::new(64, 64));
        for x in [12, 28, 44] {
            assert_near(&pixels, backend.name, (x, 32), [255, 0, 0, 255]);
        }
        for x in [20, 36, 52] {
            assert_near(&pixels, backend.name, (x, 32), [0, 0, 0, 0]);
        }
    }
}
