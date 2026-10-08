//! Tests of the bitmap types against a mock rendering backend. Not from
//! upstream: the upstream bitmap tests are render tests that need a real
//! backend.

use super::{
    Bitmap, BitmapEncoderOptions, BitmapInterpolationMode, CroppedBitmap, IBitmap, JpegBitmapEncoderOptions,
    PngBitmapEncoderOptions, RenderTargetBitmap, WriteableBitmap,
};
use crate::media::{Color, FillRule, GeometryCombineMode, IBrush, IImage, IImageBrushSource, IPen, RenderOptions};
use crate::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IGeometryImpl, IGlyphRunImpl,
    ILockedFramebuffer, IPlatformGraphicsContext, IPlatformRenderInterface, IPlatformRenderInterfaceContext,
    IPlatformRenderInterfaceRegion, IReadableBitmapImpl, IRenderTargetBitmapImpl, IStreamGeometryImpl,
    IWriteableBitmapImpl, PixelFormat, PixelFormats,
};
use crate::{
    FerroLocator, Matrix, PixelRect, PixelSize, Point, Rect, RoundedRect, Size, Vector,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::Arc;

/// A bitmap held in memory; the backend of every bitmap kind in these tests.
struct MockBitmap {
    size: PixelSize,
    dpi: Vector,
    format: Option<PixelFormat>,
    alpha_format: AlphaFormat,
    row_bytes: i32,
    pixels: Rc<RefCell<Vec<u8>>>,
    disposed: Cell<bool>,
    locks: Rc<Cell<i32>>,
    log: Rc<RefCell<Vec<String>>>,
}

impl MockBitmap {
    fn new(size: PixelSize, dpi: Vector, format: Option<PixelFormat>, alpha_format: AlphaFormat) -> Self {
        let bits = format.map_or(32, |format| format.bits_per_pixel() as i32);
        // Rows are padded, to tell the row size from the minimal stride.
        let row_bytes = (size.width * bits + 7) / 8 + 3;
        Self {
            size,
            dpi,
            format,
            alpha_format,
            row_bytes,
            pixels: Rc::new(RefCell::new(vec![0; (row_bytes * size.height) as usize])),
            disposed: Cell::new(false),
            locks: Rc::new(Cell::new(0)),
            log: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn row(&self, y: i32, length: usize) -> Vec<u8> {
        let start = (self.row_bytes * y) as usize;
        self.pixels.borrow()[start..start + length].to_vec()
    }
}

// SAFETY: the tests create and use this mock on one thread; the impls only
// satisfy the thread-safety bound of the bitmap contracts.
unsafe impl Send for MockBitmap {}
unsafe impl Sync for MockBitmap {}

impl IBitmapImpl for MockBitmap {
    fn dpi(&self) -> Vector {
        self.dpi
    }
    fn pixel_size(&self) -> PixelSize {
        self.size
    }
    fn version(&self) -> i32 {
        1
    }
    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        let text = match options {
            BitmapEncoderOptions::Jpeg(options) => format!("jpeg {}", options.quality),
            BitmapEncoderOptions::Png(options) => format!("png {:?}", options.compression_level),
        };
        stream.write_all(text.as_bytes())
    }
    fn dispose(&self) {
        self.disposed.set(true);
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        self.format.map(|_| self as &dyn IReadableBitmapImpl)
    }
}

impl IReadableBitmapImpl for MockBitmap {
    fn format(&self) -> Option<PixelFormat> {
        self.format
    }
    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.alpha_format)
    }
    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        self.locks.set(self.locks.get() + 1);
        Rc::new(MockFramebuffer {
            size: self.size,
            dpi: self.dpi,
            format: self.format.expect("readable"),
            alpha_format: self.alpha_format,
            row_bytes: self.row_bytes,
            pixels: self.pixels.clone(),
            locks: self.locks.clone(),
            disposed: Cell::new(false),
        })
    }
}

impl IWriteableBitmapImpl for MockBitmap {}

impl IRenderTargetBitmapImpl for MockBitmap {
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(MockDrawingContext { log: self.log.clone() })
    }
}

struct MockFramebuffer {
    size: PixelSize,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    row_bytes: i32,
    pixels: Rc<RefCell<Vec<u8>>>,
    locks: Rc<Cell<i32>>,
    disposed: Cell<bool>,
}

impl ILockedFramebuffer for MockFramebuffer {
    fn address(&self) -> *mut u8 {
        self.pixels.borrow_mut().as_mut_ptr()
    }
    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.pixels.borrow_mut());
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
        if !self.disposed.replace(true) {
            self.locks.set(self.locks.get() - 1);
        }
    }
}

struct MockDrawingContext {
    log: Rc<RefCell<Vec<String>>>,
}

impl IDrawingContextImpl for MockDrawingContext {
    fn transform(&self) -> Matrix {
        Matrix::IDENTITY
    }
    fn set_transform(&mut self, _: Matrix) {}
    fn clear(&mut self, color: Color) {
        self.log.borrow_mut().push(format!("clear {color}"));
    }
    fn draw_bitmap(&mut self, _: &dyn IBitmapImpl, _: f64, _: Rect, _: Rect) {}
    fn draw_bitmap_with_mask(&mut self, _: &dyn IBitmapImpl, _: &dyn IBrush, _: Rect, _: Rect) {}
    fn draw_line(&mut self, _: Option<&dyn IPen>, _: Point, _: Point) {}
    fn draw_geometry(&mut self, _: Option<&dyn IBrush>, _: Option<&dyn IPen>, _: &dyn IGeometryImpl) {}
    fn draw_rectangle(&mut self, _: Option<&dyn IBrush>, _: Option<&dyn IPen>, _: RoundedRect, _: &crate::media::BoxShadows) {}
    fn draw_region(&mut self, _: Option<&dyn IBrush>, _: Option<&dyn IPen>, _: &dyn IPlatformRenderInterfaceRegion) {}
    fn draw_ellipse(&mut self, _: Option<&dyn IBrush>, _: Option<&dyn IPen>, _: Rect) {}
    fn draw_glyph_run(&mut self, _: Option<&dyn IBrush>, _: &dyn IGlyphRunImpl) {}
    fn create_layer(&mut self, _: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        unimplemented!()
    }
    fn push_clip(&mut self, _: Rect) {}
    fn push_clip_rounded(&mut self, _: RoundedRect) {}
    fn push_clip_region(&mut self, _: &dyn IPlatformRenderInterfaceRegion) {}
    fn pop_clip(&mut self) {}
    fn push_layer(&mut self, _: Rect) {}
    fn pop_layer(&mut self) {}
    fn push_opacity(&mut self, _: f64, _: Option<Rect>) {}
    fn pop_opacity(&mut self) {}
    fn push_opacity_mask(&mut self, _: &dyn IBrush, _: Rect) {}
    fn pop_opacity_mask(&mut self) {}
    fn push_geometry_clip(&mut self, _: &dyn IGeometryImpl) {}
    fn pop_geometry_clip(&mut self) {}
    fn push_render_options(&mut self, _: RenderOptions) {}
    fn pop_render_options(&mut self) {}
    fn push_text_options(&mut self, _: crate::media::TextOptions) {}
    fn pop_text_options(&mut self) {}
    fn get_feature(&mut self, _: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn dispose(&mut self) {}
}

/// A backend that supports the 32-bit color formats only and keeps every
/// bitmap it creates.
#[derive(Default)]
struct MockBackend {
    created: RefCell<Vec<std::sync::Arc<MockBitmap>>>,
}

impl MockBackend {
    fn create(&self, bitmap: MockBitmap) -> std::sync::Arc<MockBitmap> {
        let bitmap = std::sync::Arc::new(bitmap);
        self.created.borrow_mut().push(bitmap.clone());
        bitmap
    }

    fn last(&self) -> std::sync::Arc<MockBitmap> {
        self.created.borrow().last().expect("a bitmap was created").clone()
    }

    fn decode(&self, stream: &mut dyn Read, size: Option<PixelSize>) -> std::io::Result<std::sync::Arc<MockBitmap>> {
        // The "encoded" form is two bytes: width and height.
        let mut header = [0u8; 2];
        stream.read_exact(&mut header)?;
        let size = size.unwrap_or(PixelSize::new(header[0] as i32, header[1] as i32));
        Ok(self.create(MockBitmap::new(
            size,
            Vector::new(96.0, 96.0),
            Some(PixelFormats::RGBA8888),
            AlphaFormat::Premul,
        )))
    }
}

impl IPlatformRenderInterface for MockBackend {
    fn build_glyph_run_geometry(&self, _glyph_run: &crate::media::GlyphRun) -> Arc<dyn IGeometryImpl> {
        unimplemented!()
    }

    fn create_glyph_run(
        &self,
        _glyph_typeface: &Rc<crate::media::GlyphTypeface>,
        _font_rendering_em_size: f64,
        _glyph_infos: &[crate::media::text_formatting::GlyphInfo],
        _baseline_origin: Point,
    ) -> std::sync::Arc<dyn crate::platform::IGlyphRunImpl> {
        unimplemented!()
    }

    fn create_ellipse_geometry(&self, _: Rect) -> Arc<dyn IGeometryImpl> {
        unimplemented!()
    }
    fn create_line_geometry(&self, _: Point, _: Point) -> Arc<dyn IGeometryImpl> {
        unimplemented!()
    }
    fn create_rectangle_geometry(&self, _: Rect) -> Arc<dyn IGeometryImpl> {
        unimplemented!()
    }
    fn create_stream_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
        unimplemented!()
    }
    fn create_geometry_group(&self, _: FillRule, _: &[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl> {
        unimplemented!()
    }
    fn create_combined_geometry(
        &self,
        _: GeometryCombineMode,
        _: Arc<dyn IGeometryImpl>,
        _: Arc<dyn IGeometryImpl>,
    ) -> Arc<dyn IGeometryImpl> {
        unimplemented!()
    }
    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> std::sync::Arc<dyn IRenderTargetBitmapImpl> {
        self.create(MockBitmap::new(size, dpi, Some(PixelFormats::RGBA8888), AlphaFormat::Premul))
    }
    fn create_writeable_bitmap(
        &self,
        size: PixelSize,
        dpi: Vector,
        format: PixelFormat,
        alpha_format: AlphaFormat,
    ) -> std::sync::Arc<dyn IWriteableBitmapImpl> {
        self.create(MockBitmap::new(size, dpi, Some(format), alpha_format))
    }
    fn load_bitmap_from_file(&self, file_name: &str) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, file_name.to_owned()))
    }
    fn load_bitmap(&self, stream: &mut dyn Read) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        Ok(self.decode(stream, None)?)
    }
    fn load_writeable_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        _: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        Ok(self.decode(stream, Some(PixelSize::new(width, 1)))?)
    }
    fn load_writeable_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        _: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        Ok(self.decode(stream, Some(PixelSize::new(1, height)))?)
    }
    fn load_writeable_bitmap_from_file(&self, file_name: &str) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        Err(std::io::Error::new(std::io::ErrorKind::NotFound, file_name.to_owned()))
    }
    fn load_writeable_bitmap(&self, stream: &mut dyn Read) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        Ok(self.decode(stream, None)?)
    }
    fn load_bitmap_to_width(
        &self,
        stream: &mut dyn Read,
        width: i32,
        _: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        Ok(self.decode(stream, Some(PixelSize::new(width, 1)))?)
    }
    fn load_bitmap_to_height(
        &self,
        stream: &mut dyn Read,
        height: i32,
        _: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        Ok(self.decode(stream, Some(PixelSize::new(1, height)))?)
    }
    fn resize_bitmap(
        &self,
        bitmap_impl: &dyn IBitmapImpl,
        destination_size: PixelSize,
        _: BitmapInterpolationMode,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        self.create(MockBitmap::new(destination_size, bitmap_impl.dpi(), Some(PixelFormats::RGBA8888), AlphaFormat::Premul))
    }
    fn load_bitmap_from_pixels(
        &self,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        data: &[u8],
        size: PixelSize,
        dpi: Vector,
        stride: i32,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        let bitmap = MockBitmap::new(size, dpi, Some(format), alpha_format);
        let row = ((size.width * format.bits_per_pixel() as i32 + 7) / 8) as usize;
        for y in 0..size.height {
            let (src, dst) = ((stride * y) as usize, (bitmap.row_bytes * y) as usize);
            bitmap.pixels.borrow_mut()[dst..dst + row].copy_from_slice(&data[src..src + row]);
        }
        self.create(bitmap)
    }
    fn create_backend_context(
        &self,
        _: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        unimplemented!()
    }
    fn supports_individual_round_rects(&self) -> bool {
        false
    }
    fn default_alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }
    fn default_pixel_format(&self) -> PixelFormat {
        PixelFormats::BGRA8888
    }
    fn is_supported_bitmap_pixel_format(&self, format: PixelFormat) -> bool {
        format == PixelFormats::RGBA8888 || format == PixelFormats::BGRA8888
    }
    fn supports_regions(&self) -> bool {
        false
    }
    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        unimplemented!()
    }
}

fn with_backend(f: impl FnOnce(&MockBackend)) {
    let scope = FerroLocator::enter_scope();
    let backend = Rc::new(MockBackend::default());
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(backend.clone());
    f(&backend);
    scope.dispose();
}

const DPI_96: Vector = Vector::new(96.0, 96.0);

#[test]
fn bitmap_from_stream_reports_the_platform_bitmap() {
    with_backend(|backend| {
        let bitmap = Bitmap::from_stream(&mut &[4u8, 2][..]).unwrap();
        assert_eq!(PixelSize::new(4, 2), bitmap.pixel_size());
        assert_eq!(DPI_96, bitmap.dpi());
        assert_eq!(Size::new(4.0, 2.0), bitmap.size());
        assert_eq!(Some(PixelFormats::RGBA8888), bitmap.format());
        assert_eq!(Some(AlphaFormat::Premul), bitmap.alpha_format());

        let scaled = bitmap.create_scaled_bitmap(PixelSize::new(8, 4), BitmapInterpolationMode::HighQuality);
        assert_eq!(PixelSize::new(8, 4), scaled.pixel_size());

        let by_width = Bitmap::decode_to_width(&mut &[4u8, 2][..], 16, BitmapInterpolationMode::LowQuality).unwrap();
        assert_eq!(16, by_width.pixel_size().width);
        let by_height = Bitmap::decode_to_height(&mut &[4u8, 2][..], 9, BitmapInterpolationMode::LowQuality).unwrap();
        assert_eq!(9, by_height.pixel_size().height);

        assert!(Bitmap::from_stream(&mut &[1u8][..]).is_err());
        assert!(Bitmap::from_file("missing.png").is_err());
        assert_eq!(4, backend.created.borrow().len());
    });
}

#[test]
fn bitmap_size_follows_the_dpi() {
    with_backend(|_| {
        let bitmap = Bitmap::from_pixels(
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            &[0; 8 * 4 * 4],
            PixelSize::new(8, 4),
            Vector::new(192.0, 192.0),
            32,
        );
        assert_eq!(Size::new(4.0, 2.0), bitmap.size());
        let image: Rc<dyn IImage> = Rc::new(bitmap);
        assert_eq!(Size::new(4.0, 2.0), image.size());
        assert_eq!(PixelSize::new(8, 4), image.as_bitmap().unwrap().pixel_size());
        assert!(image.as_affects_render().is_none());
        assert!(image.as_object().is_none());
    });
}

#[test]
fn save_forwards_the_encoder_options() {
    with_backend(|_| {
        let bitmap = Bitmap::from_stream(&mut &[1u8, 1][..]).unwrap();
        let mut png = Vec::new();
        bitmap.save(&mut png, &PngBitmapEncoderOptions::DEFAULT.into()).unwrap();
        assert_eq!(b"png Optimal".to_vec(), png);
        let mut jpeg = Vec::new();
        IBitmap::save(&bitmap, &mut jpeg, &JpegBitmapEncoderOptions { quality: 80 }.into()).unwrap();
        assert_eq!(b"jpeg 80".to_vec(), jpeg);
    });
}

#[test]
fn disposing_the_last_bitmap_releases_the_platform_bitmap() {
    with_backend(|backend| {
        let bitmap = Bitmap::from_stream(&mut &[1u8, 1][..]).unwrap();
        let platform = backend.last();
        let shared = Bitmap::from_ref(bitmap.platform_impl());
        assert_eq!(2, bitmap.platform_impl().ref_count());
        assert!(IImageBrushSource::bitmap(&bitmap).is_some());
        assert!(bitmap.get_bitmap().is_some());

        bitmap.dispose();
        assert!(!platform.disposed.get());
        assert!(IImageBrushSource::bitmap(&bitmap).is_none());
        assert!(bitmap.get_bitmap().is_none());

        // Dropping a bitmap releases its reference like disposing does.
        drop(shared);
        assert!(platform.disposed.get());
    });
}

#[test]
fn copy_pixels_copies_the_requested_rows() {
    with_backend(|backend| {
        // 3x2 pixels, source rows 16 bytes apart.
        let mut data = vec![0u8; 32];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = i as u8;
        }
        let bitmap =
            Bitmap::from_pixels(PixelFormats::BGRA8888, AlphaFormat::Premul, &data, PixelSize::new(3, 2), DPI_96, 16);
        assert_eq!((0..12).collect::<Vec<u8>>(), backend.last().row(0, 12));
        assert_eq!((16..28).collect::<Vec<u8>>(), backend.last().row(1, 12));

        // The whole bitmap, into rows 14 bytes apart.
        let mut buffer = vec![0xffu8; 28];
        bitmap.copy_pixels(PixelRect::default(), &mut buffer, 14);
        assert_eq!((0..12).collect::<Vec<u8>>(), buffer[..12]);
        assert_eq!([0xff, 0xff], buffer[12..14]);
        assert_eq!((16..28).collect::<Vec<u8>>(), buffer[14..26]);

        // One pixel of the second row.
        let mut pixel = [0u8; 4];
        bitmap.copy_pixels(PixelRect::new(2, 1, 1, 1), &mut pixel, 4);
        assert_eq!([24, 25, 26, 27], pixel);
        assert_eq!(0, backend.last().locks.get());
    });
}

#[test]
fn copy_pixels_between_uses_one_blit_for_identical_layouts() {
    let source: Vec<u8> = (0..24).collect();
    let mut buffer = vec![0u8; 16];
    Bitmap::copy_pixels_between(PixelRect::new(0, 1, 2, 2), &source, 8, PixelFormats::RGBA8888, &mut buffer, 8);
    assert_eq!((8..24).collect::<Vec<u8>>(), buffer);
}

#[test]
#[should_panic(expected = "stride is out of range")]
fn copy_pixels_rejects_a_small_stride() {
    with_backend(|_| {
        let bitmap = Bitmap::from_stream(&mut &[4u8, 2][..]).unwrap();
        bitmap.copy_pixels(PixelRect::default(), &mut [0; 64], 15);
    });
}

#[test]
#[should_panic(expected = "buffer size is out of range")]
fn copy_pixels_rejects_a_small_buffer() {
    with_backend(|_| {
        let bitmap = Bitmap::from_stream(&mut &[4u8, 2][..]).unwrap();
        bitmap.copy_pixels(PixelRect::default(), &mut [0; 31], 16);
    });
}

#[test]
#[should_panic(expected = "source_rect is out of range")]
fn copy_pixels_rejects_a_rect_outside_the_bitmap() {
    with_backend(|_| {
        let bitmap = Bitmap::from_stream(&mut &[4u8, 2][..]).unwrap();
        bitmap.copy_pixels(PixelRect::new(3, 0, 2, 1), &mut [0; 64], 16);
    });
}

#[test]
fn bitmap_in_an_unsupported_format_is_transcoded() {
    with_backend(|backend| {
        // Two gray pixels.
        let bitmap =
            Bitmap::from_pixels(PixelFormats::GRAY8, AlphaFormat::Opaque, &[0x10, 0x80], PixelSize::new(2, 1), DPI_96, 2);
        let platform = backend.last();
        assert_eq!(Some(PixelFormats::RGBA8888), platform.format);
        assert_eq!(AlphaFormat::Opaque, platform.alpha_format);
        assert_eq!(vec![0x10, 0x10, 0x10, 0xff, 0x80, 0x80, 0x80, 0xff], platform.row(0, 8));
        assert_eq!(Some(PixelFormats::RGBA8888), bitmap.format());
    });
}

#[test]
#[should_panic(expected = "CopyPixels is not supported for transcoded bitmaps")]
fn copy_pixels_of_a_transcoded_bitmap_is_not_supported() {
    with_backend(|_| {
        let bitmap =
            Bitmap::from_pixels(PixelFormats::GRAY8, AlphaFormat::Opaque, &[0x10, 0x80], PixelSize::new(2, 1), DPI_96, 2);
        bitmap.copy_pixels(PixelRect::default(), &mut [0; 8], 8);
    });
}

#[test]
fn copy_pixels_to_framebuffer_transcodes_between_formats() {
    with_backend(|backend| {
        let source = Bitmap::from_pixels(
            PixelFormats::RGBA8888,
            AlphaFormat::Unpremul,
            &[1, 2, 3, 255, 4, 5, 6, 255],
            PixelSize::new(2, 1),
            DPI_96,
            8,
        );

        let target = WriteableBitmap::new(PixelSize::new(2, 1), DPI_96, Some(PixelFormats::BGRA8888), Some(AlphaFormat::Unpremul));
        let target_platform = backend.last();
        let fb = target.lock();
        source.copy_pixels_to_framebuffer(&*fb);
        fb.dispose();
        assert_eq!(vec![3, 2, 1, 255, 6, 5, 4, 255], target_platform.row(0, 8));

        // Same formats: a plain copy.
        let same = WriteableBitmap::new(PixelSize::new(2, 1), DPI_96, Some(PixelFormats::RGBA8888), Some(AlphaFormat::Unpremul));
        let same_platform = backend.last();
        let fb = same.lock();
        source.copy_pixels_to_framebuffer(&*fb);
        fb.dispose();
        assert_eq!(vec![1, 2, 3, 255, 4, 5, 6, 255], same_platform.row(0, 8));
        assert_eq!(0, same_platform.locks.get());
    });
}

#[test]
fn writeable_bitmap_uses_the_backend_defaults() {
    with_backend(|backend| {
        let bitmap = WriteableBitmap::new(PixelSize::new(2, 2), DPI_96, None, None);
        assert_eq!(Some(PixelFormats::BGRA8888), bitmap.format());
        assert_eq!(Some(AlphaFormat::Premul), bitmap.alpha_format());

        let fb = bitmap.lock();
        assert_eq!(PixelSize::new(2, 2), fb.size());
        assert_eq!(PixelFormats::BGRA8888, fb.format());
        fb.with_data(&mut |data| data[0] = 7);
        assert_eq!(1, backend.last().locks.get());
        fb.dispose();
        assert_eq!(0, backend.last().locks.get());
        assert_eq!(7, backend.last().pixels.borrow()[0]);
    });
}

#[test]
fn writeable_bitmap_from_pixels_copies_rows() {
    with_backend(|backend| {
        let data: Vec<u8> = (0..20).collect();
        let bitmap = WriteableBitmap::from_pixels(
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            &data,
            PixelSize::new(2, 2),
            DPI_96,
            10,
        );
        let platform = backend.last();
        assert_eq!((0..8).collect::<Vec<u8>>(), platform.row(0, 8));
        assert_eq!((10..18).collect::<Vec<u8>>(), platform.row(1, 8));

        let mut buffer = [0u8; 16];
        bitmap.copy_pixels(PixelRect::default(), &mut buffer, 8);
        assert_eq!((10..18).collect::<Vec<u8>>(), buffer[8..]);
    });
}

#[test]
fn writeable_bitmap_in_an_unsupported_format_keeps_its_own_pixels() {
    with_backend(|backend| {
        let bitmap = WriteableBitmap::new(PixelSize::new(2, 1), DPI_96, Some(PixelFormats::GRAY8), None);
        let platform = backend.last();
        // The platform bitmap is RGBA; the bitmap reports the requested format.
        assert_eq!(Some(PixelFormats::RGBA8888), platform.format);
        assert_eq!(AlphaFormat::Opaque, platform.alpha_format);
        assert_eq!(Some(PixelFormats::GRAY8), bitmap.format());
        assert_eq!(Some(PixelFormats::GRAY8), bitmap.as_bitmap().map(|_| bitmap.format()).unwrap());

        let fb = bitmap.lock();
        assert_eq!(PixelFormats::GRAY8, fb.format());
        assert_eq!(AlphaFormat::Opaque, fb.alpha_format());
        assert_eq!(4, fb.row_bytes());
        assert!(!fb.address().is_null());
        fb.with_data(&mut |data| data[..2].copy_from_slice(&[0x20, 0xf0]));
        // Nothing reaches the platform bitmap until the framebuffer is
        // disposed.
        assert_eq!(vec![0; 8], platform.row(0, 8));
        fb.dispose();
        fb.dispose();
        assert_eq!(vec![0x20, 0x20, 0x20, 0xff, 0xf0, 0xf0, 0xf0, 0xff], platform.row(0, 8));
        assert_eq!(0, platform.locks.get());

        // The pixels read back in the bitmap's own format.
        let mut buffer = [0u8; 2];
        bitmap.copy_pixels(PixelRect::default(), &mut buffer, 2);
        assert_eq!([0x20, 0xf0], buffer);
    });
}

#[test]
#[should_panic(expected = "Size should be >= (1,1)")]
fn writeable_bitmap_needs_a_size() {
    with_backend(|_| {
        WriteableBitmap::new(PixelSize::new(0, 1), DPI_96, None, None);
    });
}

#[test]
fn writeable_bitmap_decodes() {
    with_backend(|_| {
        let bitmap = WriteableBitmap::decode(&mut &[3u8, 5][..]).unwrap();
        assert_eq!(PixelSize::new(3, 5), bitmap.pixel_size());
        let bitmap = WriteableBitmap::decode_to_width(&mut &[3u8, 5][..], 6, BitmapInterpolationMode::HighQuality).unwrap();
        assert_eq!(6, bitmap.pixel_size().width);
        let bitmap = WriteableBitmap::decode_to_height(&mut &[3u8, 5][..], 7, BitmapInterpolationMode::HighQuality).unwrap();
        assert_eq!(7, bitmap.pixel_size().height);
    });
}

#[test]
fn render_target_bitmap_shares_one_platform_bitmap() {
    with_backend(|backend| {
        let bitmap = RenderTargetBitmap::new(PixelSize::new(10, 20));
        let platform = backend.last();
        assert_eq!(DPI_96, bitmap.dpi());
        assert_eq!(Size::new(10.0, 20.0), bitmap.size());
        // One reference as a render target, one as a bitmap.
        assert_eq!(2, bitmap.platform_impl().ref_count());
        assert_eq!(1, backend.created.borrow().len());

        let mut context = bitmap.create_platform_drawing_context(true);
        context.dispose();
        assert_eq!(vec![format!("clear {}", crate::media::Colors::TRANSPARENT)], *platform.log.borrow());
        let mut context = bitmap.create_platform_drawing_context(false);
        context.dispose();
        assert_eq!(1, platform.log.borrow().len());

        let scaled = RenderTargetBitmap::with_dpi(PixelSize::new(10, 20), Vector::new(192.0, 192.0));
        assert_eq!(Size::new(5.0, 10.0), scaled.size());

        bitmap.dispose();
        assert!(platform.disposed.get());
        assert!(IImageBrushSource::bitmap(&bitmap).is_none());
    });
}

#[test]
fn cropped_bitmap_reports_the_cropped_size() {
    with_backend(|_| {
        let source: Rc<dyn IImage> = Rc::new(Bitmap::from_pixels(
            PixelFormats::RGBA8888,
            AlphaFormat::Premul,
            &[0; 8 * 4 * 4],
            PixelSize::new(8, 4),
            Vector::new(192.0, 192.0),
            32,
        ));

        let target = CroppedBitmap::new();
        assert_eq!(Size::default(), target.size());

        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        target.invalidated(move || c.set(c.get() + 1));

        target.set_source(Some(source.clone()));
        assert_eq!(1, count.get());
        // An empty source rect shows the whole source.
        assert_eq!(Size::new(4.0, 2.0), target.size());

        target.set_source_rect(PixelRect::new(2, 0, 4, 2));
        assert_eq!(2, count.get());
        assert_eq!(Size::new(2.0, 1.0), target.size());

        // Clearing the source does not raise the notification.
        target.set_source(None);
        assert_eq!(2, count.get());
        assert_eq!(Size::default(), target.size());

        let target = CroppedBitmap::with_source(source.clone(), PixelRect::new(0, 0, 2, 2));
        let image: Rc<dyn IImage> = target.clone().into();
        assert_eq!(Size::new(1.0, 1.0), image.size());
        assert!(image.as_bitmap().is_none());
        assert!(image.as_affects_render().is_some());
        assert!(image.as_object().unwrap().is::<CroppedBitmap>());
        let other: Rc<dyn IImage> = (&target).into();
        assert!(*image == *other);
        assert!(*image != *source);

        // Disposing the cropped bitmap disposes its source.
        target.dispose();
        assert!(!source.as_bitmap().unwrap().platform_impl().is_alive());
    });
}

#[test]
#[should_panic(expected = "Only IBitmap supported as source")]
fn cropped_bitmap_needs_a_bitmap_source() {
    let drawing: Rc<dyn IImage> = crate::media::DrawingImage::new().into();
    CroppedBitmap::new().set_source(Some(drawing));
}
