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

fn draw(backend: &Backend) -> Pixels {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(backend.interface.clone());
    let document = SvgDocument::parse(SAMPLE).unwrap();
    let image = document.to_image();
    let pixels = render(backend, PixelSize::new(656, 328), &|_, context| {
        let mut core = PlatformDrawingContext::borrowed(context);
        let mut drawing = DrawingContext::new(&mut core);
        image.draw(&mut drawing, Rect::new(0.0, 0.0, 328.0, 164.0), Rect::new(0.0, 0.0, 656.0, 328.0));
    });
    scope.dispose();
    pixels
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
