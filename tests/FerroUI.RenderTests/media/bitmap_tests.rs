//! Port of upstream's `Media/BitmapTests.cs`.
//!
//! The enumeration of the pixel formats and the constructor of a pixel
//! format from it are internal upstream and visible to its test assembly;
//! the port has them public and hidden from the documentation.
//!
//! Upstream has one directory for the expected images and for its outputs,
//! and finds the pixel data (`PixelFormats/Lenna`) relative to it. The port
//! reads the pixel data and the expected images relative to the directory of
//! the expected images and writes its outputs to the output directory.
//!
//! Where upstream hands a bitmap the address of pinned memory, the port
//! hands it the slice.

use crate::test_base::TestBase;
use ferroui_base::media::imaging::{Bitmap, PngBitmapEncoderOptions, RenderTargetBitmap, WriteableBitmap};
use ferroui_base::media::{BoxShadows, Brushes, Colors, IBrush};
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{
    AlphaFormat, ILockedFramebuffer, IPlatformRenderInterface, PixelFormat, PixelFormatEnum, RenderTargetSceneInfo,
};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelRect, PixelSize, Rect, Vector};
use std::any::Any;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn base() -> TestBase {
    let t = TestBase::new(r"Media\Bitmap");
    // `Directory.CreateDirectory(OutputPath)`.
    t.output_path();
    t
}

fn save(bitmap: &Bitmap, path: &Path) {
    bitmap
        .save_to_file(path.to_str().expect("the path is text"), &PngBitmapEncoderOptions::default().into())
        .expect("the image is saved");
}

#[derive(Clone)]
struct Framebuffer {
    memory: Arc<Mutex<Option<Vec<u8>>>>,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    size: PixelSize,
    row_bytes: i32,
}

impl Framebuffer {
    fn new(fmt: PixelFormat, alpha_format: AlphaFormat, size: PixelSize) -> Framebuffer {
        let bpp = if fmt == PixelFormat::RGB565 { 2 } else { 4 };
        let row_bytes = bpp * size.width;
        Framebuffer {
            memory: Arc::new(Mutex::new(Some(vec![0u8; (size.height * row_bytes) as usize]))),
            dpi: Vector::new(96.0, 96.0),
            format: fmt,
            alpha_format,
            size,
            row_bytes,
        }
    }

    fn deallocate(&self) {
        self.memory.lock().unwrap().take();
    }
}

impl ILockedFramebuffer for Framebuffer {
    fn address(&self) -> *mut u8 {
        self.memory.lock().unwrap().as_mut().map_or(std::ptr::null_mut(), |memory| memory.as_mut_ptr())
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        if let Some(memory) = self.memory.lock().unwrap().as_mut() {
            access(memory);
        }
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn format(&self) -> PixelFormat {
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    fn dispose(&self) {
        //no-op
    }
}

impl IPlatformRenderSurface for Framebuffer {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for Framebuffer {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let this = self.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || Rc::new(this.clone()) as Rc<dyn ILockedFramebuffer>))
    }
}

fn framebuffer_render_results_should_be_usable_as_bitmap(fmte: PixelFormatEnum) {
    let t = base();
    let fmt = PixelFormat::new(fmte);
    let test_name = format!("FramebufferRenderResultsShouldBeUsableAsBitmap_{fmt}");
    let fb = Framebuffer::new(fmt, AlphaFormat::Premul, PixelSize::new(80, 80));
    let r = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
    {
        let cpu_context = r.create_backend_context(None);
        let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(fb.clone());
        let target = cpu_context.create_render_target(&[surface]);
        let (mut ctx, _) = target.create_drawing_context(&RenderTargetSceneInfo::new(
            fb.size,
            1.0,
            CompositionTransparencyLevel::None,
        ));
        ctx.clear(Colors::TRANSPARENT);
        ctx.push_opacity(0.8, Some(Rect::new(0.0, 0.0, 80.0, 80.0)));
        ctx.draw_rectangle(
            Some(&*Brushes::chartreuse()),
            None,
            Rect::new(0.0, 0.0, 20.0, 100.0).into(),
            &BoxShadows::default(),
        );
        ctx.draw_rectangle(Some(&*Brushes::crimson()), None, Rect::new(20.0, 0.0, 20.0, 100.0).into(), &BoxShadows::default());
        ctx.draw_rectangle(Some(&*Brushes::gold()), None, Rect::new(40.0, 0.0, 20.0, 100.0).into(), &BoxShadows::default());
        ctx.pop_opacity();
        ctx.dispose();
        target.dispose();
        cpu_context.dispose();
    }

    let bmp = {
        let memory = fb.memory.lock().unwrap();
        Bitmap::from_pixels(
            fmt,
            AlphaFormat::Premul,
            memory.as_ref().expect("the framebuffer is allocated"),
            fb.size,
            Vector::new(96.0, 96.0),
            fb.row_bytes,
        )
    };
    fb.deallocate();
    {
        let rtb = RenderTargetBitmap::with_dpi(PixelSize::new(100, 100), Vector::new(96.0, 96.0));
        {
            let mut ctx = rtb.create_drawing_context();
            let blue: Rc<dyn IBrush> = Brushes::blue();
            let pink: Rc<dyn IBrush> = Brushes::pink();
            ctx.draw_rectangle(Some(&blue), None, Rect::new(0.0, 0.0, 100.0, 100.0), 0.0, 0.0, &BoxShadows::default());
            ctx.draw_rectangle(Some(&pink), None, Rect::new(0.0, 20.0, 100.0, 10.0), 0.0, 0.0, &BoxShadows::default());

            let rc = Rect::new(0.0, 0.0, 60.0, 60.0);
            ctx.draw_bitmap(bmp.platform_impl(), 1.0, rc, rc);
            ctx.dispose();
        }
        save(&rtb, &t.output_path().join(format!("{test_name}.out.png")));
        rtb.dispose();
    }
    t.compare_images_no_renderer(&test_name, None);
}

#[test]
fn framebuffer_render_results_should_be_usable_as_bitmap_rgba8888() {
    framebuffer_render_results_should_be_usable_as_bitmap(PixelFormatEnum::Rgba8888);
}

#[test]
fn framebuffer_render_results_should_be_usable_as_bitmap_bgra8888() {
    framebuffer_render_results_should_be_usable_as_bitmap(PixelFormatEnum::Bgra8888);
}

#[test]
fn framebuffer_render_results_should_be_usable_as_bitmap_rgb565() {
    framebuffer_render_results_should_be_usable_as_bitmap(PixelFormatEnum::Rgb565);
}

fn writeable_bitmap_should_be_usable(fmte: PixelFormatEnum) {
    let t = base();
    let fmt = PixelFormat::new(fmte);
    let writeable_bitmap = WriteableBitmap::new(PixelSize::new(256, 256), Vector::new(96.0, 96.0), Some(fmt), None);

    let mut data = vec![0i32; 256 * 256];
    for y in 0..256usize {
        for x in 0..256usize {
            data[y * 256 + x] = ((x + (y << 8)) as u32 | 0xFF000000u32) as i32;
        }
    }

    {
        let l = writeable_bitmap.lock();
        let row_bytes = l.row_bytes() as usize;
        l.with_data(&mut |dest| {
            for r in 0..256usize {
                let row = &mut dest[r * row_bytes..r * row_bytes + 256 * 4];
                for (pixel, value) in row.chunks_exact_mut(4).zip(&data[r * 256..r * 256 + 256]) {
                    pixel.copy_from_slice(&value.to_ne_bytes());
                }
            }
        });
        l.dispose();
    }

    let name = format!("WriteableBitmapShouldBeUsable_{fmt}");

    save(&writeable_bitmap, &t.output_path().join(format!("{name}.out.png")));
    t.compare_images_no_renderer(&name, None);
}

#[test]
fn writeable_bitmap_should_be_usable_bgra8888() {
    writeable_bitmap_should_be_usable(PixelFormatEnum::Bgra8888);
}

#[test]
fn writeable_bitmap_should_be_usable_rgba8888() {
    writeable_bitmap_should_be_usable(PixelFormatEnum::Rgba8888);
}

struct RawHeader {
    width: i32,
    height: i32,
    stride: i32,
}

/// The bitmap of a step of the transcoder test: upstream holds it as a
/// `Bitmap` and calls its virtual members.
enum StepBitmap {
    Writeable(WriteableBitmap),
    Normal(Bitmap),
}

impl StepBitmap {
    fn copy_pixels(&self, source_rect: PixelRect, buffer: &mut [u8], stride: i32) {
        match self {
            StepBitmap::Writeable(bitmap) => bitmap.copy_pixels(source_rect, buffer, stride),
            StepBitmap::Normal(bitmap) => bitmap.copy_pixels(source_rect, buffer, stride),
        }
    }

    fn bitmap(&self) -> &Bitmap {
        match self {
            StepBitmap::Writeable(bitmap) => bitmap,
            StepBitmap::Normal(bitmap) => bitmap,
        }
    }
}

impl Drop for StepBitmap {
    fn drop(&mut self) {
        self.bitmap().dispose();
    }
}

fn bitmaps_should_support_transcoders_lenna(format: PixelFormatEnum, alpha_format: AlphaFormat) {
    let t = base();
    let relative_files_dir = "../../../PixelFormats/Lenna";
    let files_dir = t.expected_path().join(relative_files_dir);

    let mut format_name = format!("{format:?}");
    if alpha_format == AlphaFormat::Premul {
        format_name = format!("P{}", format_name.to_lowercase());
    }

    let bits_data = std::fs::read(files_dir.join(format!("{format_name}.bits"))).expect("the pixel data is read");
    let header_size = std::mem::size_of::<RawHeader>();
    let field = |index: usize| i32::from_ne_bytes(bits_data[index * 4..index * 4 + 4].try_into().unwrap());
    let header = RawHeader { width: field(0), height: field(1), stride: field(2) };
    let data = &bits_data[header_size..];

    let size = PixelSize::new(header.width, header.height);
    let stride = header.stride;

    let mut expected_name = format!("{relative_files_dir}/{format_name}");
    if !t.expected_path().join(format!("{expected_name}.expected.png")).exists() {
        expected_name = format!("{relative_files_dir}/Default");
    }

    let names = ["_Writeable", "_WriteableInitialized", "_Normal"];

    for step in [0usize, 1, 2] {
        let test_name = format!("BitmapsShouldSupportTranscoders_Lenna_{format_name}{}", names[step]);

        let path = t.output_path().join(format!("{test_name}.out.png"));
        {
            let b = if step == 0 {
                let bmp = WriteableBitmap::new(
                    size,
                    Vector::new(96.0, 96.0),
                    Some(PixelFormat::new(format)),
                    Some(alpha_format),
                );

                {
                    let l = bmp.lock();
                    let min_stride = ((l.size().width * l.format().bits_per_pixel() as i32 + 7) / 8) as usize;
                    let row_bytes = l.row_bytes() as usize;
                    l.with_data(&mut |dest| {
                        for y in 0..size.height as usize {
                            dest[y * row_bytes..y * row_bytes + min_stride]
                                .copy_from_slice(&data[y * stride as usize..y * stride as usize + min_stride]);
                        }
                    });
                    l.dispose();
                }

                StepBitmap::Writeable(bmp)
            } else if step == 1 {
                StepBitmap::Writeable(WriteableBitmap::from_pixels(
                    PixelFormat::new(format),
                    alpha_format,
                    data,
                    size,
                    Vector::new(96.0, 96.0),
                    stride,
                ))
            } else {
                StepBitmap::Normal(Bitmap::from_pixels(
                    PixelFormat::new(format),
                    alpha_format,
                    data,
                    size,
                    Vector::new(96.0, 96.0),
                    stride,
                ))
            };

            if step < 2 {
                let mut copy_to = vec![0u8; data.len()];
                b.copy_pixels(PixelRect::default(), &mut copy_to, stride);
                assert_eq!(data, &copy_to[..]);
            }

            save(b.bitmap(), &path);
            t.compare_images_no_renderer(&test_name, Some(&expected_name));
        }
    }
}

#[test]
fn bitmaps_should_support_transcoders_lenna_black_white() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::BlackWhite, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_gray2() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Gray2, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_gray4() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Gray4, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_gray8() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Gray8, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_gray16() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Gray16, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_rgb24() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Rgb24, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_bgr24() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Bgr24, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_gray32_float() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Gray32Float, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_rgba64() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Rgba64, AlphaFormat::Unpremul);
}

#[test]
fn bitmaps_should_support_transcoders_lenna_rgba64_premul() {
    bitmaps_should_support_transcoders_lenna(PixelFormatEnum::Rgba64, AlphaFormat::Premul);
}

/// The bytes of `new Random().NextBytes`: a generator seeded from the clock,
/// as upstream's is not seeded by the test.
fn next_bytes(data: &mut [u8]) {
    let mut state = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0x9E37_79B9_7F4A_7C15, |elapsed| elapsed.as_nanos() as u64)
        | 1;
    for value in data.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *value = (state >> 32) as u8;
    }
}

#[test]
fn copy_pixels_should_work_for_non_transcoded_bitmaps() {
    let _t = base();
    let stride = 32 * 4;
    let mut data = vec![0u8; 32 * stride];
    next_bytes(&mut data);
    for c in 0..data.len() {
        if data[c] == 0 {
            data[c] = 1;
        }
    }

    let bmp = Bitmap::from_pixels(
        PixelFormat::BGRA8888,
        AlphaFormat::Unpremul,
        &data,
        PixelSize::new(32, 32),
        Vector::new(96.0, 96.0),
        32 * 4,
    );

    let mut copy_to = vec![0u8; data.len()];
    bmp.copy_pixels(PixelRect::default(), &mut copy_to, stride as i32);
    assert_eq!(data, copy_to);
}

#[test]
fn should_copy_pixels_with_source_rect() {
    let _t = base();
    let size = 80;
    let part_size = 20;
    let bitmap = RenderTargetBitmap::new(PixelSize::new(size, size));

    {
        let mut context = bitmap.create_drawing_context();
        let black: Rc<dyn IBrush> = Brushes::black();
        let white: Rc<dyn IBrush> = Brushes::white();
        context.fill_rectangle(
            &black,
            Rect::new(0.0, 0.0, bitmap.pixel_size().width as f64, bitmap.pixel_size().height as f64),
            0.0,
        );
        context.fill_rectangle(
            &white,
            Rect::new(part_size as f64, part_size as f64, part_size as f64, part_size as f64),
            0.0,
        );
        context.dispose();
    }

    let bpp = (bitmap.format().expect("the bitmap has a format").bits_per_pixel() / 8) as i32;
    let mut buffer = vec![0u8; (part_size * part_size * bpp) as usize];

    bitmap.copy_pixels(PixelRect::new(part_size, part_size, part_size, part_size), &mut buffer, part_size * bpp);

    for t in &buffer {
        assert_eq!(u8::MAX, *t);
    }
}
