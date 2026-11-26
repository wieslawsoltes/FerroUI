//! The route of SVG documents in the GPU modes (#1260): a mask or a pattern of a real document
//! reaches no scene of the CPU renderer, and lands where the document says.
//!
//! Skipped with a message on a machine without a graphics adapter.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::scene::{cpu_scenes_made, IVelloSceneSink};
use crate::vello_options::VelloRenderingMode;
use crate::{PlatformRenderInterface, VelloOptions};
use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
use ferroui_base::platform::{IDrawingContextImpl, IPlatformRenderInterface};
use ferroui_base::{FerroLocator, Rect, Vector};
use ferroui_svg::SvgDocument;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 64;

const LUMINANCE_MASK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <mask id="m" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <rect width="64" height="64" fill="#ffffff"/><rect x="16" y="16" width="32" height="32" fill="#000000"/>
    </mask></defs><rect width="64" height="64" fill="#ff0000" mask="url(#m)"/></svg>"##;

const ALPHA_MASK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <mask id="m" style="mask-type: alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <rect width="32" height="64" fill="#000000"/>
    </mask></defs><rect width="64" height="64" fill="#ff0000" mask="url(#m)"/></svg>"##;

const TILED_PATTERN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <pattern id="p" patternUnits="userSpaceOnUse" x="8" width="16" height="16"><rect width="8" height="16" fill="#ff0000"/></pattern>
    </defs><rect width="64" height="64" fill="url(#p)"/></svg>"##;

const STROKE_PATTERN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs>
    <pattern id="p" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="8" height="8" fill="#ff0000"/></pattern>
    </defs><line x1="0" y1="32" x2="64" y2="32" stroke="url(#p)" stroke-width="8"/></svg>"##;

/// A name, a document, the points that are red and the points that are empty.
type Case = (&'static str, &'static str, &'static [(usize, usize)], &'static [(usize, usize)]);

const CASES: &[Case] = &[
    ("luminance mask", LUMINANCE_MASK, &[(8, 8)], &[(32, 32)]),
    ("alpha mask", ALPHA_MASK, &[(16, 32)], &[(48, 32)]),
    ("tiled pattern", TILED_PATTERN, &[(12, 32), (28, 32), (44, 32)], &[(20, 32), (36, 32), (52, 32)]),
    ("pattern on a stroke", STROKE_PATTERN, &[(32, 32)], &[(32, 8)]),
];

fn device_modes(device: &Arc<VelloWgpuDevice>) -> Vec<VelloRenderingMode> {
    let mut modes = Vec::new();
    if cfg!(feature = "hybrid") {
        modes.push(VelloRenderingMode::Hybrid);
    }
    if cfg!(feature = "gpu") && device.supports_compute() {
        modes.push(VelloRenderingMode::Gpu);
    }
    modes
}

fn draw(device: &Arc<VelloWgpuDevice>, mode: VelloRenderingMode, text: &str) -> Vec<u8> {
    let document = SvgDocument::parse(text.as_bytes()).unwrap();
    assert_eq!(document.unsupported_features(), &[] as &[String]);
    let image = document.to_image();
    let sink: Box<dyn IVelloSceneSink> =
        crate::gpu::create_window_scene_sink(device, &[mode], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm);
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
    let target = pixels.clone();
    let mut drawing = DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut target.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![mode],
        },
        Vec::new(),
    );
    {
        let mut core = PlatformDrawingContext::borrowed(&mut drawing);
        let mut context = DrawingContext::new(&mut core);
        let area = Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE));
        image.draw(&mut context, area, area);
    }
    drawing.dispose();
    pixels.take()
}

fn at(pixels: &[u8], (x, y): (usize, usize)) -> [u8; 4] {
    let start = (y * usize::from(SIZE) + x) * 4;
    [pixels[start], pixels[start + 1], pixels[start + 2], pixels[start + 3]]
}

fn near(actual: [u8; 4], expected: [u8; 4]) -> bool {
    actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 3)
}

#[test]
fn no_mask_or_pattern_of_a_document_reaches_a_scene_of_the_cpu_renderer() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let mut failures = Vec::new();
    for mode in device_modes(&device) {
        let scope = FerroLocator::enter_scope();
        let interface: Rc<dyn IPlatformRenderInterface> =
            Rc::new(PlatformRenderInterface::new(VelloOptions::with_rendering_mode(mode)));
        FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(interface);
        for (name, text, red, empty) in CASES {
            let before = cpu_scenes_made();
            let pixels = draw(&device, mode, text);
            let made = cpu_scenes_made() - before;
            if made > 0 {
                failures.push(format!("{mode:?}, {name}: {made} CPU scenes were made"));
            }
            for (points, expected) in [(*red, [255, 0, 0, 255]), (*empty, [0, 0, 0, 0])] {
                for point in points {
                    let actual = at(&pixels, *point);
                    if !near(actual, expected) {
                        failures.push(format!("{mode:?}, {name} at {point:?}: {actual:?}, not {expected:?}"));
                    }
                }
            }
            println!("{mode:?}, {name}: {made} CPU scenes");
        }
        scope.dispose();
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    !crc
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let group = u32::from(chunk[0]) << 16 | u32::from(*chunk.get(1).unwrap_or(&0)) << 8 | u32::from(*chunk.get(2).unwrap_or(&0));
        for index in 0..4 {
            if index <= chunk.len() {
                text.push(ALPHABET[(group >> (18 - 6 * index) & 63) as usize] as char);
            } else {
                text.push('=');
            }
        }
    }
    text
}

/// A red PNG of 2 by 2 pixels, its data in a stored deflate block.
fn red_png() -> Vec<u8> {
    let mut raw = Vec::new();
    for _ in 0..2 {
        raw.push(0);
        raw.extend_from_slice(&[255, 0, 0, 255, 255, 0, 0, 255]);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &raw {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    let length = raw.len() as u16;
    let mut zlib = vec![0x78, 0x01, 0x01];
    zlib.extend_from_slice(&length.to_le_bytes());
    zlib.extend_from_slice(&(!length).to_le_bytes());
    zlib.extend_from_slice(&raw);
    zlib.extend_from_slice(&(b << 16 | a).to_be_bytes());
    let header = [0, 0, 0, 2, 0, 0, 0, 2, 8, 6, 0, 0, 0].to_vec();
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [(b"IHDR", header), (b"IDAT", zlib), (b"IEND", Vec::new())] {
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(&data);
        bytes.extend_from_slice(&body);
        bytes.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    bytes
}

#[test]
fn no_embedded_image_or_text_of_a_document_reaches_a_scene_of_the_cpu_renderer() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let image = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><image x="16" y="16" width="32" height="32" href="data:image/png;base64,{}"/></svg>"##,
        base64(&red_png())
    );
    let text = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><text x="4" y="44" font-size="40" font-weight="bold" fill="#ff0000">Hi</text></svg>"##;
    let mut failures = Vec::new();
    for mode in device_modes(&device) {
        crate::unit_tests::register_test_assets();
        let scope = FerroLocator::enter_scope();
        crate::VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(mode));
        ferroui_harfbuzz::HarfBuzzPlatform::initialize();
        for (name, document) in [("embedded image", image.as_str()), ("text", text)] {
            let before = cpu_scenes_made();
            let pixels = draw(&device, mode, document);
            let made = cpu_scenes_made() - before;
            if made > 0 {
                failures.push(format!("{mode:?}, {name}: {made} CPU scenes were made"));
            }
            let red = pixels.chunks_exact(4).filter(|pixel| near([pixel[0], pixel[1], pixel[2], pixel[3]], [255, 0, 0, 255])).count();
            if red == 0 {
                failures.push(format!("{mode:?}, {name}: nothing red was drawn"));
            }
            if name == "embedded image" && !near(at(&pixels, (32, 32)), [255, 0, 0, 255]) {
                failures.push(format!("{mode:?}, {name}: {:?} in the middle", at(&pixels, (32, 32))));
            }
            println!("{mode:?}, {name}: {made} CPU scenes, {red} red pixels");
        }
        scope.dispose();
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
