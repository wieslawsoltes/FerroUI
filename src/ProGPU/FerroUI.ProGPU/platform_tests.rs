//! The render interface, the backend context and the render targets: frames are drawn through
//! the contracts of the base library and checked in the pixels that come back.
//!
//! Skipped with a message on a machine without a WebGPU adapter.

use crate::notes;
use crate::ProGpuPlatformRenderInterface;
use ferroui_base::media::imaging::BitmapInterpolationMode;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadows, Color, IBrush, PenLineCap, PenLineJoin, RenderOptions};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IGlyphRunImpl, ILockedFramebuffer, IPlatformRenderInterface,
    IPlatformRenderInterfaceContext, IRenderTarget, PixelFormat, RenderTargetSceneInfo,
};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use std::any::Any;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// A framebuffer surface in memory: `BGRA8888` premultiplied, rows longer than their pixels.
pub(crate) struct MemorySurface {
    state: Arc<Mutex<SurfaceMemory>>,
}

struct SurfaceMemory {
    size: PixelSize,
    row_bytes: i32,
    pixels: Vec<u8>,
    frames: u32,
}

impl MemorySurface {
    pub(crate) fn new(width: i32, height: i32) -> Arc<Self> {
        let surface = Arc::new(Self { state: Arc::new(Mutex::new(SurfaceMemory { size: PixelSize::new(0, 0), row_bytes: 0, pixels: Vec::new(), frames: 0 })) });
        surface.resize(width, height);
        surface
    }

    /// Changes the size of the frames that start from now; the pixels start transparent.
    pub(crate) fn resize(&self, width: i32, height: i32) {
        let mut memory = self.state.lock().unwrap();
        memory.size = PixelSize::new(width, height);
        memory.row_bytes = width * 4 + 12;
        memory.pixels = vec![0; (memory.row_bytes * height) as usize];
    }

    pub(crate) fn size(&self) -> PixelSize {
        self.state.lock().unwrap().size
    }

    /// The frames that were written.
    pub(crate) fn frames(&self) -> u32 {
        self.state.lock().unwrap().frames
    }

    /// The pixel as red, green, blue and alpha.
    pub(crate) fn rgba(&self, x: i32, y: i32) -> [u8; 4] {
        let memory = self.state.lock().unwrap();
        let offset = (y * memory.row_bytes + x * 4) as usize;
        [memory.pixels[offset + 2], memory.pixels[offset + 1], memory.pixels[offset], memory.pixels[offset + 3]]
    }
}

struct MemoryLock {
    state: Arc<Mutex<SurfaceMemory>>,
    size: PixelSize,
    row_bytes: i32,
}

impl ILockedFramebuffer for MemoryLock {
    fn address(&self) -> *mut u8 {
        self.state.lock().unwrap().pixels.as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.state.lock().unwrap().pixels);
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {
        self.state.lock().unwrap().frames += 1;
    }
}

impl IPlatformRenderSurface for MemorySurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for MemorySurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let state = self.state.clone();
        Rc::new(FuncFramebufferRenderTarget::with_scene_info(
            move |_| {
                let (size, row_bytes) = {
                    let memory = state.lock().unwrap();
                    (memory.size, memory.row_bytes)
                };
                let locked: Rc<dyn ILockedFramebuffer> = Rc::new(MemoryLock { state: state.clone(), size, row_bytes });
                (locked, FramebufferLockProperties { previous_frame_is_retained: false })
            },
            false,
        ))
    }
}

/// A backend context on the default adapter, or `None` (with a message) without one.
pub(crate) fn context() -> Option<Rc<dyn IPlatformRenderInterfaceContext>> {
    let context = crate::ProGpuContext::new(None);
    if !context.has_device() {
        eprintln!("skipped: no WebGPU adapter");
        return None;
    }
    Some(Rc::new(context))
}

fn scene_info(size: PixelSize) -> RenderTargetSceneInfo {
    RenderTargetSceneInfo::new(size, 1.0, CompositionTransparencyLevel::None)
}

fn solid(r: u8, g: u8, b: u8) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(Color::from_rgb(r, g, b))
}

fn fill(context: &mut dyn IDrawingContextImpl, brush: &ImmutableSolidColorBrush, rect: Rect) {
    context.draw_rectangle(Some(brush as &dyn IBrush), None, RoundedRect::from(rect), &BoxShadows::default());
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn near(expected: [u8; 4], actual: [u8; 4]) -> bool {
    expected.iter().zip(actual.iter()).all(|(a, b)| a.abs_diff(*b) <= 2)
}

#[test]
fn a_framebuffer_target_draws_shapes() {
    let Some(context) = context() else { return };
    let surface = MemorySurface::new(128, 96);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    assert!(target.platform_render_target_state().is_ready);

    let interface = ProGpuPlatformRenderInterface::new();
    let triangle = {
        let mut builder = interface.create_stream_geometry_builder(None);
        builder.begin_figure(Point::new(70.0, 90.0), true);
        builder.line_to(Point::new(95.0, 50.0), true);
        builder.line_to(Point::new(120.0, 90.0), true);
        builder.end_figure(true);
        builder.to_geometry()
    };

    let (mut frame, properties) = target.try_create_drawing_context(&scene_info(surface.size())).expect("a frame");
    assert!(!properties.previous_frame_is_retained, "the first frame has nothing before it");
    frame.clear(Color::from_rgb(255, 255, 255));
    // A rectangle.
    fill(&mut *frame, &solid(255, 0, 0), Rect::new(4.0, 4.0, 24.0, 16.0));
    // A rounded rectangle with a stroke.
    let pen = ImmutablePen::new(Some(Rc::new(solid(0, 0, 0))), 2.0, None, PenLineCap::Flat, PenLineJoin::Miter, 10.0);
    frame.draw_rectangle(
        Some(&solid(0, 0, 255) as &dyn IBrush),
        Some(&pen),
        RoundedRect::from_uniform_radii(Rect::new(40.0, 4.0, 40.0, 32.0), Vector::new(12.0, 12.0)),
        &BoxShadows::default(),
    );
    // A path.
    frame.draw_geometry(Some(&solid(0, 128, 0) as &dyn IBrush), None, &*triangle);
    // A clip.
    frame.push_clip(Rect::new(4.0, 44.0, 10.0, 40.0));
    fill(&mut *frame, &solid(255, 0, 0), Rect::new(0.0, 40.0, 40.0, 50.0));
    frame.pop_clip();
    // Opacity.
    frame.push_opacity(0.5, Some(Rect::new(20.0, 44.0, 20.0, 20.0)));
    fill(&mut *frame, &solid(0, 0, 0), Rect::new(20.0, 44.0, 20.0, 20.0));
    frame.pop_opacity();
    // A transform.
    frame.set_transform(Matrix::create_scale(2.0, 2.0) * Matrix::create_translation(44.0, 44.0));
    fill(&mut *frame, &solid(0, 0, 255), Rect::new(0.0, 0.0, 8.0, 8.0));
    frame.set_transform(Matrix::IDENTITY);
    frame.dispose();

    assert_eq!(1, surface.frames());
    assert_eq!(WHITE, surface.rgba(1, 1), "the cleared frame");
    assert_eq!(RED, surface.rgba(16, 12), "the rectangle");
    assert_eq!(WHITE, surface.rgba(30, 12), "beside the rectangle");
    assert_eq!(BLUE, surface.rgba(60, 20), "the rounded rectangle");
    assert_eq!(WHITE, surface.rgba(41, 5), "outside the rounded corner");
    assert_eq!([0, 0, 0, 255], surface.rgba(60, 4), "the stroke of the rounded rectangle");
    assert_eq!([0, 128, 0, 255], surface.rgba(95, 75), "the path");
    assert_eq!(WHITE, surface.rgba(72, 55), "beside the path");
    assert_eq!(RED, surface.rgba(8, 60), "inside the clip");
    assert_eq!(WHITE, surface.rgba(16, 60), "outside the clip");
    assert!(near([128, 128, 128, 255], surface.rgba(30, 54)), "black at half opacity over white: {:?}", surface.rgba(30, 54));
    assert_eq!(BLUE, surface.rgba(52, 52), "the scaled and translated square");
    assert_eq!(WHITE, surface.rgba(62, 62), "past the scaled square");
    target.dispose();
}

#[test]
fn a_framebuffer_target_keeps_the_frame_and_follows_the_surface() {
    let Some(context) = context() else { return };
    let surface = MemorySurface::new(64, 64);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    assert!(target.properties().retains_previous_frame_contents);

    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    fill(&mut *frame, &solid(255, 0, 0), Rect::new(0.0, 0.0, 32.0, 64.0));
    frame.dispose();

    // The second frame redraws a part: the clear and the draws stay inside the clip.
    let (mut frame, properties) = target.create_drawing_context(&scene_info(surface.size()));
    assert!(properties.previous_frame_is_retained);
    frame.push_clip(Rect::new(16.0, 16.0, 32.0, 32.0));
    frame.clear(Color::from_argb(0, 0, 0, 0));
    fill(&mut *frame, &solid(0, 0, 255), Rect::new(0.0, 0.0, 24.0, 64.0));
    frame.pop_clip();
    frame.dispose();

    assert_eq!(RED, surface.rgba(8, 8), "kept from the first frame");
    assert_eq!(WHITE, surface.rgba(56, 56), "kept from the first frame");
    assert_eq!(BLUE, surface.rgba(20, 30), "redrawn inside the clip");
    assert_eq!([0, 0, 0, 0], surface.rgba(40, 30), "cleared inside the clip");
    assert_eq!(RED, surface.rgba(20, 8), "above the clip");

    // A surface that changed its size gets a frame of the new size with nothing kept.
    surface.resize(96, 40);
    let (mut frame, properties) = target.create_drawing_context(&scene_info(surface.size()));
    assert!(!properties.previous_frame_is_retained, "a frame of another size keeps nothing");
    frame.clear(Color::from_rgb(255, 255, 255));
    fill(&mut *frame, &solid(0, 0, 255), Rect::new(80.0, 0.0, 16.0, 40.0));
    frame.dispose();
    assert_eq!(BLUE, surface.rgba(90, 20), "drawn at the new size");
    assert_eq!(WHITE, surface.rgba(8, 8), "nothing of the frame before");
    target.dispose();
}

#[test]
fn layers_are_drawn_into_and_sampled() {
    let Some(context) = context() else { return };
    let layer = context.create_offscreen_render_target(PixelSize::new(32, 32), Vector::new(1.0, 1.0), false);
    assert!(layer.can_blit());
    {
        let mut drawing = layer.create_drawing_context();
        drawing.clear(Color::from_rgb(255, 0, 0));
        fill(&mut *drawing, &solid(0, 0, 255), Rect::new(0.0, 0.0, 16.0, 16.0));
        drawing.dispose();
    }
    // A second frame of the layer draws over the first.
    {
        let mut drawing = layer.create_drawing_context();
        fill(&mut *drawing, &solid(0, 128, 0), Rect::new(16.0, 16.0, 16.0, 16.0));
        drawing.dispose();
    }
    assert!(!layer.is_corrupted());

    let surface = MemorySurface::new(96, 64);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    // The layer as it is, at a translation.
    frame.set_transform(Matrix::create_translation(8.0, 8.0));
    layer.blit(&mut *frame);
    // The layer scaled, at half opacity, from a part of it.
    frame.set_transform(Matrix::IDENTITY);
    frame.draw_bitmap(&*layer, 0.5, Rect::new(0.0, 0.0, 16.0, 16.0), Rect::new(56.0, 8.0, 32.0, 32.0));
    frame.dispose();

    assert_eq!(BLUE, surface.rgba(12, 12), "the first quarter of the layer");
    assert_eq!(RED, surface.rgba(30, 12), "the cleared part of the layer");
    assert_eq!([0, 128, 0, 255], surface.rgba(30, 30), "the second frame of the layer");
    assert_eq!(WHITE, surface.rgba(44, 12), "past the layer");
    assert_eq!(WHITE, surface.rgba(4, 4), "before the layer");
    assert!(near([128, 128, 255, 255], surface.rgba(72, 24)), "the blue quarter at half opacity: {:?}", surface.rgba(72, 24));

    // The snapshot of the layer has its pixels.
    let snapshot = layer.create_shared_snapshot();
    let readable = snapshot.as_readable_bitmap().expect("a readable snapshot");
    let locked = readable.lock();
    let mut first = [0u8; 4];
    locked.with_data(&mut |data| first.copy_from_slice(&data[..4]));
    locked.dispose();
    assert_eq!(BLUE, first);
    target.dispose();
}

#[test]
fn bitmaps_of_pixels_are_drawn() {
    let Some(context) = context() else { return };
    let interface = ProGpuPlatformRenderInterface::new();
    // Two by two: red, green; blue, half transparent white (not premultiplied).
    let pixels: [u8; 16] = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128];
    let bitmap = interface.load_bitmap_from_pixels(PixelFormat::RGBA8888, AlphaFormat::Unpremul, &pixels, PixelSize::new(2, 2), Vector::new(96.0, 96.0), 8);

    let surface = MemorySurface::new(64, 64);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(0, 0, 0));
    frame.push_render_options(RenderOptions { bitmap_interpolation_mode: BitmapInterpolationMode::None, ..RenderOptions::default() });
    frame.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 2.0, 2.0), Rect::new(16.0, 16.0, 32.0, 32.0));
    frame.pop_render_options();
    frame.dispose();

    assert_eq!(RED, surface.rgba(24, 24));
    assert_eq!([0, 255, 0, 255], surface.rgba(40, 24));
    assert_eq!(BLUE, surface.rgba(24, 40));
    assert!(near([128, 128, 128, 255], surface.rgba(40, 40)), "half transparent white over black: {:?}", surface.rgba(40, 40));
    assert_eq!([0, 0, 0, 255], surface.rgba(8, 8), "outside the bitmap");

    // The same bitmap, drawn to another size on the device.
    let resized = interface.resize_bitmap(&*bitmap, PixelSize::new(8, 8), BitmapInterpolationMode::None);
    assert_eq!(PixelSize::new(8, 8), resized.pixel_size());
    let locked = resized.as_readable_bitmap().expect("readable").lock();
    let mut corner = [0u8; 4];
    locked.with_data(&mut |data| corner.copy_from_slice(&data[..4]));
    locked.dispose();
    assert_eq!(RED, corner, "the first pixel of the resized bitmap");
    target.dispose();
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// A PNG of `width` x `height` RGBA pixels of eight bits, its rows unfiltered and its stream
/// stored, not compressed.
pub(crate) fn png(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    fn chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = png.len();
        png.extend_from_slice(kind);
        png.extend_from_slice(data);
        let crc = crc32(&png[start..]);
        png.extend_from_slice(&crc.to_be_bytes());
    }
    assert_eq!(pixels.len(), width as usize * height as usize * 4);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &header);
    let mut rows = Vec::new();
    for row in pixels.chunks(width as usize * 4) {
        rows.push(0);
        rows.extend_from_slice(row);
    }
    let mut stream = vec![0x78, 0x01, 0x01];
    stream.extend_from_slice(&(rows.len() as u16).to_le_bytes());
    stream.extend_from_slice(&(!(rows.len() as u16)).to_le_bytes());
    stream.extend_from_slice(&rows);
    stream.extend_from_slice(&adler32(&rows).to_be_bytes());
    chunk(&mut png, b"IDAT", &stream);
    chunk(&mut png, b"IEND", &[]);
    png
}

fn first_pixel(bitmap: &dyn IBitmapImpl) -> [u8; 4] {
    let locked = bitmap.as_readable_bitmap().expect("readable").lock();
    let mut pixel = [0u8; 4];
    locked.with_data(&mut |data| pixel.copy_from_slice(&data[..4]));
    locked.dispose();
    pixel
}

#[test]
fn image_files_are_decoded_and_drawn() {
    let Some(context) = context() else { return };
    let interface = ProGpuPlatformRenderInterface::new();
    // Four by two: a red and a blue half above, green and half transparent white below.
    let mut pixels = Vec::new();
    for color in [[255u8, 0, 0, 255], [255, 0, 0, 255], [0, 0, 255, 255], [0, 0, 255, 255], [0, 255, 0, 255], [0, 255, 0, 255], [255, 255, 255, 128], [255, 255, 255, 128]] {
        pixels.extend_from_slice(&color);
    }
    let file = png(4, 2, &pixels);

    // From a stream: the pixels of the file, not premultiplied, at 96 dpi.
    let bitmap = interface.load_bitmap(&mut std::io::Cursor::new(file.clone())).expect("a PNG is decoded");
    assert_eq!(PixelSize::new(4, 2), bitmap.pixel_size());
    assert_eq!(Vector::new(96.0, 96.0), bitmap.dpi());
    let readable = bitmap.as_readable_bitmap().expect("readable");
    assert_eq!(Some(PixelFormat::RGBA8888), readable.format());
    assert_eq!(Some(AlphaFormat::Unpremul), readable.alpha_format());
    let locked = readable.lock();
    locked.with_data(&mut |data| assert_eq!(&pixels[..], &data[..32]));
    locked.dispose();

    let surface = MemorySurface::new(64, 32);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(0, 0, 0));
    frame.push_render_options(RenderOptions { bitmap_interpolation_mode: BitmapInterpolationMode::None, ..RenderOptions::default() });
    frame.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 4.0, 2.0), Rect::new(0.0, 0.0, 64.0, 32.0));
    frame.pop_render_options();
    frame.dispose();
    assert_eq!(RED, surface.rgba(8, 8));
    assert_eq!(BLUE, surface.rgba(56, 8));
    assert_eq!([0, 255, 0, 255], surface.rgba(8, 24));
    assert!(near([128, 128, 128, 255], surface.rgba(56, 24)), "half transparent white over black: {:?}", surface.rgba(56, 24));
    target.dispose();

    // From a file, and as a bitmap that can be written to.
    let path = std::env::temp_dir().join(format!("ferroui-progpu-image-{}.png", std::process::id()));
    std::fs::write(&path, &file).expect("the file is written");
    let name = path.to_str().expect("a path of text");
    assert_eq!(PixelSize::new(4, 2), interface.load_bitmap_from_file(name).expect("decoded").pixel_size());
    assert_eq!(PixelSize::new(4, 2), interface.load_writeable_bitmap_from_file(name).expect("decoded").pixel_size());
    let _ = std::fs::remove_file(&path);
    assert_eq!(PixelSize::new(4, 2), interface.load_writeable_bitmap(&mut std::io::Cursor::new(file.clone())).expect("decoded").pixel_size());

    // To a width or a height: the other side keeps the proportion.
    let wide = interface.load_bitmap_to_width(&mut std::io::Cursor::new(file.clone()), 16, BitmapInterpolationMode::None).expect("decoded");
    assert_eq!(PixelSize::new(16, 8), wide.pixel_size());
    assert_eq!(RED, first_pixel(&*wide));
    let tall = interface.load_bitmap_to_height(&mut std::io::Cursor::new(file.clone()), 6, BitmapInterpolationMode::None).expect("decoded");
    assert_eq!(PixelSize::new(12, 6), tall.pixel_size());
    let same = interface.load_writeable_bitmap_to_width(&mut std::io::Cursor::new(file.clone()), 4, BitmapInterpolationMode::None).expect("decoded");
    assert_eq!(PixelSize::new(4, 2), same.pixel_size());
    assert_eq!(PixelSize::new(2, 1), interface.load_writeable_bitmap_to_height(&mut std::io::Cursor::new(file.clone()), 1, BitmapInterpolationMode::None).expect("decoded").pixel_size());
    assert!(interface.load_bitmap_to_width(&mut std::io::Cursor::new(file.clone()), 0, BitmapInterpolationMode::None).is_err());

    // A damaged file is invalid data; a file of another format is not supported.
    let mut damaged = file.clone();
    let cut = damaged.len() - 20;
    damaged.truncate(cut);
    assert_eq!(std::io::ErrorKind::InvalidData, interface.load_bitmap(&mut std::io::Cursor::new(damaged)).err().expect("refused").kind());
    let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0];
    assert_eq!(std::io::ErrorKind::Unsupported, interface.load_bitmap(&mut std::io::Cursor::new(jpeg)).err().expect("refused").kind());
}

/// A frame on a white surface of 96 pixels, drawn by `draw`.
fn clipped_frame(context: &Rc<dyn IPlatformRenderInterfaceContext>, draw: impl FnOnce(&mut dyn IDrawingContextImpl)) -> Arc<MemorySurface> {
    let surface = MemorySurface::new(96, 96);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    draw(&mut *frame);
    frame.dispose();
    target.dispose();
    surface
}

#[test]
fn rotated_and_nested_clips_restrict_shapes_images_and_paths() {
    let Some(context) = context() else { return };
    let interface = ProGpuPlatformRenderInterface::new();
    const GREEN: [u8; 4] = [0, 160, 0, 255];
    let green = solid(0, 160, 0);
    let whole = Rect::new(0.0, 0.0, 96.0, 96.0);
    let bitmap = interface.load_bitmap_from_pixels(PixelFormat::RGBA8888, AlphaFormat::Unpremul, &GREEN.repeat(4), PixelSize::new(2, 2), Vector::new(96.0, 96.0), 8);
    let ellipse = interface.create_ellipse_geometry(Rect::new(-40.0, -40.0, 176.0, 176.0));
    let quarter_turn_half = Matrix::create_rotation_at(std::f64::consts::FRAC_PI_4, Point::new(48.0, 48.0));

    // A square turned by 45 degrees about the middle of the target clips to a diamond that
    // reaches 28 pixels from the middle; what is drawn is drawn without the rotation.
    let draws: [(&str, Box<dyn Fn(&mut dyn IDrawingContextImpl)>); 3] = [
        ("a rectangle", Box::new(|frame| fill(frame, &green, whole))),
        ("a bitmap", Box::new(|frame| frame.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 2.0, 2.0), whole))),
        ("a path", Box::new(|frame| frame.draw_geometry(Some(&green as &dyn IBrush), None, &*ellipse))),
    ];
    for (what, draw) in &draws {
        let surface = clipped_frame(&context, |frame| {
            frame.set_transform(quarter_turn_half);
            frame.push_clip(Rect::new(28.0, 28.0, 40.0, 40.0));
            frame.set_transform(Matrix::IDENTITY);
            draw(frame);
            frame.pop_clip();
        });
        assert_eq!(GREEN, surface.rgba(48, 48), "{what}: the middle of the diamond");
        assert_eq!(GREEN, surface.rgba(48, 24), "{what}: under the top corner of the diamond");
        assert_eq!(GREEN, surface.rgba(70, 48), "{what}: before the right corner of the diamond");
        assert_eq!(WHITE, surface.rgba(30, 30), "{what}: in the square that was not turned, outside the diamond");
        assert_eq!(WHITE, surface.rgba(66, 66), "{what}: in the square that was not turned, outside the diamond");
        assert_eq!(WHITE, surface.rgba(48, 16), "{what}: above the diamond");
        assert_eq!(WHITE, surface.rgba(4, 4), "{what}: far outside");
    }

    // Five clips in each other: rounded, turned, rounded, turned, rounded. The first four are
    // one chain of masks, the fifth a masked layer.
    let surface = clipped_frame(&context, |frame| {
        frame.push_clip_rounded(RoundedRect::from_uniform_radii(Rect::new(8.0, 8.0, 80.0, 80.0), Vector::new(20.0, 20.0)));
        frame.set_transform(quarter_turn_half);
        frame.push_clip(Rect::new(18.0, 18.0, 60.0, 60.0));
        frame.set_transform(Matrix::IDENTITY);
        frame.push_clip_rounded(RoundedRect::from_uniform_radii(Rect::new(8.0, 30.0, 80.0, 36.0), Vector::new(10.0, 10.0)));
        frame.set_transform(quarter_turn_half);
        frame.push_clip(Rect::new(28.0, 28.0, 40.0, 40.0));
        frame.set_transform(Matrix::IDENTITY);
        frame.push_clip_rounded(RoundedRect::from_uniform_radii(Rect::new(40.0, 40.0, 30.0, 30.0), Vector::new(4.0, 4.0)));
        fill(frame, &green, whole);
        frame.pop_clip();
        frame.pop_clip();
        frame.pop_clip();
        frame.pop_clip();
        // Under the first clip alone again.
        fill(frame, &solid(0, 0, 255), Rect::new(0.0, 44.0, 20.0, 8.0));
        frame.pop_clip();
        fill(frame, &solid(255, 0, 0), Rect::new(0.0, 0.0, 6.0, 6.0));
    });
    assert_eq!(GREEN, surface.rgba(48, 48), "inside the five clips");
    assert_eq!(GREEN, surface.rgba(44, 60), "inside the five clips");
    assert_eq!(GREEN, surface.rgba(68, 48), "inside the five clips");
    assert_eq!(WHITE, surface.rgba(36, 48), "left of the fifth clip");
    assert_eq!(WHITE, surface.rgba(68, 63), "outside the fourth clip, the small diamond");
    assert_eq!(WHITE, surface.rgba(48, 68), "below the third clip, the band");
    assert_eq!(BLUE, surface.rgba(12, 48), "the first clip alone, after the others are popped");
    assert_eq!(WHITE, surface.rgba(4, 48), "left of the first clip");
    assert_eq!(RED, surface.rgba(3, 3), "no clip is left");
}

#[test]
fn a_frame_of_many_rotated_clips_needs_no_mask_textures() {
    let Some(context) = context() else { return };
    // A mask with a texture has the size of the target: at this size the engine refuses the
    // frame after a few of them. Rectangle clips under a rotation have none.
    let surface = MemorySurface::new(2048, 2048);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    let green = solid(0, 160, 0);
    for index in 0..256 {
        let (column, row) = (f64::from(index % 16), f64::from(index / 16));
        let cell = Rect::new(column * 128.0 + 24.0, row * 128.0 + 24.0, 80.0, 80.0);
        frame.set_transform(Matrix::create_rotation_at(0.1 + 0.01 * f64::from(index), cell.center()));
        frame.push_clip(cell);
        frame.push_clip(cell.deflate(8.0));
        fill(&mut *frame, &green, cell);
        frame.pop_clip();
        frame.pop_clip();
    }
    frame.dispose();
    target.dispose();
    assert_eq!([0, 160, 0, 255], surface.rgba(64, 64), "the first cell");
    assert_eq!([0, 160, 0, 255], surface.rgba(15 * 128 + 64, 15 * 128 + 64), "the last cell");
    assert_eq!(WHITE, surface.rgba(4, 4), "between the cells");
}

#[test]
fn shapes_without_an_area_draw_and_clip_to_nothing() {
    let Some(context) = context() else { return };
    let interface = ProGpuPlatformRenderInterface::new();
    let green = solid(0, 160, 0);
    let flat = interface.create_rectangle_geometry(Rect::new(10.0, 10.0, 40.0, 0.0));
    let surface = clipped_frame(&context, |frame| {
        // Neither fails the frame: the fill covers nothing, the clips leave nothing.
        frame.draw_geometry(Some(&green as &dyn IBrush), None, &*flat);
        frame.push_geometry_clip(&*flat);
        fill(frame, &green, Rect::new(0.0, 0.0, 96.0, 96.0));
        frame.pop_geometry_clip();
        frame.set_transform(Matrix::create_rotation(0.3));
        frame.push_clip(Rect::new(10.0, 10.0, 0.0, 40.0));
        fill(frame, &green, Rect::new(0.0, 0.0, 96.0, 96.0));
        frame.pop_clip();
        frame.set_transform(Matrix::IDENTITY);
        fill(frame, &green, Rect::new(60.0, 60.0, 20.0, 20.0));
    });
    assert_eq!(WHITE, surface.rgba(30, 10), "the flat rectangle");
    assert_eq!(WHITE, surface.rgba(30, 40), "under the clips without an area");
    assert_eq!([0, 160, 0, 255], surface.rgba(70, 70), "what is drawn after them");
}

#[test]
fn render_target_bitmaps_keep_what_was_drawn() {
    if context().is_none() {
        return;
    }
    let interface = ProGpuPlatformRenderInterface::new();
    let bitmap = interface.create_render_target_bitmap(PixelSize::new(16, 16), Vector::new(96.0, 96.0));
    {
        let mut drawing = bitmap.create_drawing_context();
        drawing.clear(Color::from_rgb(255, 255, 255));
        fill(&mut *drawing, &solid(255, 0, 0), Rect::new(0.0, 0.0, 8.0, 16.0));
        drawing.dispose();
    }
    {
        let mut drawing = bitmap.create_drawing_context();
        fill(&mut *drawing, &solid(0, 0, 255), Rect::new(0.0, 8.0, 16.0, 8.0));
        drawing.dispose();
    }
    let locked = bitmap.as_readable_bitmap().expect("readable").lock();
    let row_bytes = locked.row_bytes() as usize;
    let mut read = Vec::new();
    locked.with_data(&mut |data| read = data.to_vec());
    locked.dispose();
    let at = |x: usize, y: usize| [read[y * row_bytes + x * 4], read[y * row_bytes + x * 4 + 1], read[y * row_bytes + x * 4 + 2], read[y * row_bytes + x * 4 + 3]];
    assert_eq!(RED, at(2, 2), "the first frame");
    assert_eq!(WHITE, at(12, 2), "the first frame");
    assert_eq!(BLUE, at(2, 12), "the second frame over the first");
    assert!(bitmap.version() >= 2);
}

/// A thread that ends while a thread-local of it holds the device of its bitmaps (the device is
/// made after that thread-local is first used, so the thread-local outlives what the WebGPU
/// implementation keeps for the thread) leaves the device alone instead of releasing it from a
/// destructor of the thread.
#[test]
fn a_thread_ends_with_the_device_of_its_bitmaps() {
    if context().is_none() {
        return;
    }
    let version = std::thread::spawn(|| {
        let interface = ProGpuPlatformRenderInterface::new();
        let bitmap = interface.create_render_target_bitmap(PixelSize::new(8, 8), Vector::new(96.0, 96.0));
        let mut drawing = bitmap.create_drawing_context();
        drawing.clear(Color::from_rgb(255, 0, 0));
        drawing.dispose();
        bitmap.version()
    })
    .join()
    .expect("the thread ended");
    assert!(version >= 1, "the bitmap was drawn");
}

struct ForeignBitmap;

impl IBitmapImpl for ForeignBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        PixelSize::new(4, 4)
    }

    fn version(&self) -> i32 {
        1
    }

    fn save(&self, _stream: &mut dyn std::io::Write, _options: &ferroui_base::media::imaging::BitmapEncoderOptions) -> std::io::Result<()> {
        Ok(())
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ForeignGlyphRun;

impl IGlyphRunImpl for ForeignGlyphRun {
    fn font_rendering_em_size(&self) -> f64 {
        12.0
    }

    fn baseline_origin(&self) -> Point {
        Point::new(0.0, 0.0)
    }

    fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, 10.0, 10.0)
    }

    fn get_intersections(&self, _lower_limit: f32, _upper_limit: f32) -> Vec<f32> {
        Vec::new()
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn what_the_backend_lacks_is_drawn_as_nothing() {
    let Some(context) = context() else { return };
    let interface = ProGpuPlatformRenderInterface::new();
    let surface = MemorySurface::new(32, 32);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let (mut frame, _) = target.create_drawing_context(&scene_info(surface.size()));
    frame.clear(Color::from_rgb(255, 255, 255));
    // Text, a bitmap of another backend, a bitmap under a mask: nothing, and the frame goes on.
    frame.draw_glyph_run(Some(&solid(0, 0, 0) as &dyn IBrush), &ForeignGlyphRun);
    frame.draw_bitmap(&ForeignBitmap, 1.0, Rect::new(0.0, 0.0, 4.0, 4.0), Rect::new(0.0, 0.0, 32.0, 32.0));
    frame.draw_bitmap_with_mask(&ForeignBitmap, &solid(0, 0, 0), Rect::new(0.0, 0.0, 32.0, 32.0), Rect::new(0.0, 0.0, 32.0, 32.0));
    fill(&mut *frame, &solid(255, 0, 0), Rect::new(16.0, 16.0, 16.0, 16.0));
    frame.dispose();
    assert_eq!(WHITE, surface.rgba(4, 4));
    assert_eq!(RED, surface.rgba(24, 24), "what follows is drawn");
    assert!(notes::said("text-run") && notes::said("bitmap-kind") && notes::said("bitmap-mask"));

    // An image the image module does not read: an error, not a panic.
    let error = interface.load_bitmap(&mut std::io::Cursor::new(vec![0u8; 16])).err().expect("no codec");
    assert_eq!(std::io::ErrorKind::Unsupported, error.kind());
    assert!(interface.load_bitmap_from_file("missing.png").is_err());
    assert!(notes::said("image-codec"));

    // Pixels of a format the backend does not keep: a transparent bitmap of the size.
    let odd = interface.load_bitmap_from_pixels(PixelFormat::RGB565, AlphaFormat::Opaque, &[0u8; 8], PixelSize::new(2, 2), Vector::new(96.0, 96.0), 4);
    assert_eq!(PixelSize::new(2, 2), odd.pixel_size());

    // A context without surfaces it draws to gives a target that is never ready.
    let none = context.create_render_target(&[]);
    assert!(!none.platform_render_target_state().is_ready);
    assert!(none.try_create_drawing_context(&scene_info(PixelSize::new(8, 8))).is_err());
    target.dispose();
}

/// What is moved to another thread in the test below: the objects are used by one thread at a
/// time, as the compositor uses them.
struct Moved<T>(T);

// SAFETY: the test hands the value from one thread to the next and never shares it.
unsafe impl<T> Send for Moved<T> {}

#[test]
fn a_context_draws_on_the_thread_it_is_moved_to() {
    let Some(context) = context() else { return };
    let surface = MemorySurface::new(32, 32);
    let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    let draw = |target: &Rc<dyn IRenderTarget>, size: PixelSize, color: Color| {
        let (mut frame, _) = target.create_drawing_context(&scene_info(size));
        frame.clear(color);
        frame.dispose();
    };
    draw(&target, surface.size(), Color::from_rgb(255, 0, 0));
    assert_eq!(RED, surface.rgba(8, 8));

    let moved = Moved((context, target));
    let size = surface.size();
    let moved = std::thread::spawn(move || {
        let moved = moved;
        let (_, target) = &moved.0;
        let (mut frame, _) = target.create_drawing_context(&scene_info(size));
        frame.clear(Color::from_rgb(0, 0, 255));
        frame.dispose();
        moved
    })
    .join()
    .expect("the frame of the other thread");
    assert_eq!(BLUE, surface.rgba(8, 8), "drawn on the other thread");

    // And back on the first.
    let (_, target) = &moved.0;
    draw(target, surface.size(), Color::from_rgb(255, 0, 0));
    assert_eq!(RED, surface.rgba(8, 8));
    target.dispose();
}

/// The scope of the locator of a test: disposed when the test ends.
struct LocatorScope(Rc<dyn ferroui_base::reactive::IDisposable>);

impl Drop for LocatorScope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

#[test]
fn the_compositor_renders_controls_into_a_framebuffer() {
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::Brushes;
    use ferroui_base::{CornerRadius, FerroLocator, FerroObject, Size, Thickness};
    use ferroui_controls::embedding::EmbeddableControlRoot;
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::shapes::Ellipse;
    use ferroui_controls::templates::FuncControlTemplate;
    use ferroui_controls::testing::{CompositorTestServices, MockImplKind, MockWindowImpl, TestServices};
    use ferroui_controls::{Border, ContentControl, Control};

    if context().is_none() {
        return;
    }
    let _scope = LocatorScope(FerroLocator::enter_scope());
    let interface: Rc<dyn IPlatformRenderInterface> = Rc::new(ProGpuPlatformRenderInterface::new());
    let services = CompositorTestServices::start(TestServices::mock_platform_render_interface().with_render_interface(interface));

    let surface = MemorySurface::new(200, 120);
    let platform_impl = MockWindowImpl::bare(MockImplKind::TopLevel);
    platform_impl.client_size.set(Size::new(200.0, 120.0));
    platform_impl.setup_surfaces(vec![surface.clone() as Arc<dyn IPlatformRenderSurface>]);
    services.setup(&platform_impl);

    let top_level = EmbeddableControlRoot::with_impl(platform_impl);
    top_level.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        scope.register("PART_ContentPresenter", presenter.clone().upcast::<FerroObject>());
        presenter.upcast()
    })));
    top_level.prepare();
    top_level.start_rendering();
    services.run_jobs();

    // A rounded border with a border line, and an ellipse in its middle: no text anywhere.
    let ellipse = Ellipse::new();
    ellipse.set_width(40.0);
    ellipse.set_height(40.0);
    ellipse.set_fill(Some(Brushes::red()));
    ellipse.set_horizontal_alignment(HorizontalAlignment::Center);
    ellipse.set_vertical_alignment(VerticalAlignment::Center);
    let border = Border::new();
    border.set_width(160.0);
    border.set_height(80.0);
    border.set_background(Some(Brushes::blue()));
    border.set_border_brush(Some(Brushes::black()));
    border.set_border_thickness(Thickness::uniform(4.0));
    border.set_corner_radius(CornerRadius::uniform(20.0));
    border.set_horizontal_alignment(HorizontalAlignment::Center);
    border.set_vertical_alignment(VerticalAlignment::Center);
    border.set_child(&ellipse);
    let content: &Control = &border;
    top_level.set_content(Some(Control::boxed(content.to_ref())));
    services.run_jobs();

    assert!(surface.frames() >= 1, "the compositor rendered a frame");
    assert_eq!(RED, surface.rgba(100, 60), "the ellipse");
    assert_eq!(BLUE, surface.rgba(40, 60), "the background of the border");
    assert_eq!([0, 0, 0, 255], surface.rgba(100, 22), "the line of the border");
    assert_eq!([0, 0, 0, 0], surface.rgba(22, 22), "outside the rounded corner");
    assert_eq!([0, 0, 0, 0], surface.rgba(5, 5), "outside the border");

    // A change redraws what changed over the frame that was kept.
    let frames = surface.frames();
    ellipse.set_fill(Some(Brushes::white()));
    services.run_jobs();
    assert!(surface.frames() > frames, "the change rendered a frame");
    assert_eq!(WHITE, surface.rgba(100, 60), "the ellipse after the change");
    assert_eq!(BLUE, surface.rgba(40, 60), "what did not change");
    assert_eq!([0, 0, 0, 255], surface.rgba(100, 22), "what did not change");

    top_level.stop_rendering();
    top_level.dispose();
}

#[cfg(target_os = "macos")]
mod metal {
    use super::*;
    use ferroui_base::platform::IPlatformGraphics;
    use ferroui_metal::offscreen::{OffscreenFramePresentation, OffscreenMetalGraphics, OffscreenMetalSurface};

    /// The pixel of a frame of a Metal surface (premultiplied `BGRA`) as red, green, blue, alpha.
    fn rgba(frame: &(PixelSize, Vec<u8>), x: i32, y: i32) -> [u8; 4] {
        let offset = ((y * frame.0.width + x) * 4) as usize;
        [frame.1[offset + 2], frame.1[offset + 1], frame.1[offset], frame.1[offset + 3]]
    }

    #[test]
    fn a_metal_surface_gets_whole_frames_of_its_size() {
        let Some(graphics) = OffscreenMetalGraphics::try_new() else {
            eprintln!("skipped: no Metal device");
            return;
        };
        let interface = ProGpuPlatformRenderInterface::new();
        // The device of the context is on the Metal device and the command queue of the platform.
        let context = interface.create_backend_context(Some(graphics.create_context()));
        let surface = OffscreenMetalSurface::new(PixelSize::new(64, 48), 1.0, OffscreenFramePresentation::Kept, None);
        let surfaces = [surface.clone() as Arc<dyn IPlatformRenderSurface>];
        assert!(context.is_ready_to_create_render_target(&surfaces));
        let target = context.create_render_target(&surfaces);
        assert!(target.platform_render_target_state().is_ready);

        // A layer of the context: rendered by the engine of the offscreen targets, sampled by
        // the engine of the window on the same device.
        let layer = context.create_offscreen_render_target(PixelSize::new(16, 16), Vector::new(1.0, 1.0), false);
        {
            let mut drawing = layer.create_drawing_context();
            drawing.clear(Color::from_rgb(0, 0, 255));
            drawing.dispose();
        }

        let (mut frame, properties) = target.try_create_drawing_context(&scene_info(PixelSize::new(64, 48))).expect("a frame");
        assert!(!properties.previous_frame_is_retained, "a drawable holds nothing of the frame before");
        frame.clear(Color::from_rgb(255, 255, 255));
        fill(&mut *frame, &solid(255, 0, 0), Rect::new(0.0, 0.0, 32.0, 48.0));
        frame.set_transform(Matrix::create_translation(40.0, 8.0));
        layer.blit(&mut *frame);
        frame.dispose();

        assert_eq!(1, surface.frames());
        let first = surface.last_frame().expect("the frame that was presented");
        assert_eq!(PixelSize::new(64, 48), first.0);
        assert_eq!(RED, rgba(&first, 8, 8), "the rectangle");
        assert_eq!(WHITE, rgba(&first, 36, 40), "the cleared frame");
        assert_eq!(BLUE, rgba(&first, 48, 16), "the layer");
        assert_eq!(WHITE, rgba(&first, 60, 30), "past the layer");

        // The surface changes its size: the next frame is of the new size, drawn whole, with
        // nothing of the frame before and nothing stretched.
        surface.resize(PixelSize::new(96, 32), 1.0);
        let (mut frame, _) = target.try_create_drawing_context(&scene_info(PixelSize::new(96, 32))).expect("a frame");
        frame.clear(Color::from_argb(0, 0, 0, 0));
        fill(&mut *frame, &solid(0, 0, 255), Rect::new(80.0, 0.0, 16.0, 32.0));
        let half = ImmutableSolidColorBrush::new(Color::from_argb(128, 255, 0, 0));
        fill(&mut *frame, &half, Rect::new(0.0, 0.0, 16.0, 16.0));
        frame.dispose();

        assert_eq!(2, surface.frames());
        let second = surface.last_frame().expect("the frame that was presented");
        assert_eq!(PixelSize::new(96, 32), second.0);
        assert_eq!(BLUE, rgba(&second, 90, 16), "drawn at the new size");
        assert_eq!([0, 0, 0, 0], rgba(&second, 40, 24), "nothing of the frame before");
        assert!(near([128, 0, 0, 128], rgba(&second, 8, 8)), "premultiplied: {:?}", rgba(&second, 8, 8));

        target.dispose();
        assert!(!target.platform_render_target_state().is_ready);
        assert!(target.try_create_drawing_context(&scene_info(PixelSize::new(96, 32))).is_err());
        context.dispose();
    }

    /// A frame of many translucent groups is many render passes (the content of each group, then
    /// the target again), and the engine keeps a command buffer for each of them until it
    /// submits the frame: on the queue of the platform, which has to allow as many at once. A
    /// queue with the default capacity of 64 blocks the frame for ever.
    #[test]
    fn a_frame_of_many_translucent_groups_is_drawn_on_the_queue_of_the_platform() {
        let Some(graphics) = OffscreenMetalGraphics::try_new() else {
            eprintln!("skipped: no Metal device");
            return;
        };
        let interface = ProGpuPlatformRenderInterface::new();
        let context = interface.create_backend_context(Some(graphics.create_context()));
        let size = PixelSize::new(320, 320);
        let surface = OffscreenMetalSurface::new(size, 1.0, OffscreenFramePresentation::Kept, None);
        let target = context.create_render_target(&[surface.clone() as Arc<dyn IPlatformRenderSurface>]);
        let (mut frame, _) = target.try_create_drawing_context(&scene_info(size)).expect("a frame");
        frame.clear(Color::from_rgb(255, 255, 255));
        // 256 groups at half opacity, each a blue square under a red one that covers its
        // right half: inside a group the red replaces the blue, so no blue shows through it.
        for index in 0..256 {
            let (x, y) = (f64::from(index % 16) * 20.0, f64::from(index / 16) * 20.0);
            frame.push_opacity(0.5, Some(Rect::new(x, y, 16.0, 16.0)));
            fill(&mut *frame, &solid(0, 0, 255), Rect::new(x, y, 16.0, 16.0));
            fill(&mut *frame, &solid(255, 0, 0), Rect::new(x + 8.0, y, 8.0, 16.0));
            frame.pop_opacity();
        }
        frame.dispose();

        let drawn = surface.last_frame().expect("the frame that was presented");
        for index in [0, 17, 128, 255] {
            let (x, y) = ((index % 16) * 20, (index / 16) * 20);
            assert!(near([128, 128, 255, 255], rgba(&drawn, x + 4, y + 8)), "group {index}, blue: {:?}", rgba(&drawn, x + 4, y + 8));
            assert!(near([255, 128, 128, 255], rgba(&drawn, x + 12, y + 8)), "group {index}, red: {:?}", rgba(&drawn, x + 12, y + 8));
            assert_eq!(WHITE, rgba(&drawn, x + 18, y + 18), "between the groups");
        }
        target.dispose();
        context.dispose();
    }
}
