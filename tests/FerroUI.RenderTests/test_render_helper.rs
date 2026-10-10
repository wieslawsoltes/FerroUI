//! Port of upstream's `TestRenderHelper.cs` of the render tests: renders a
//! control into a file through the immediate renderer or through the
//! compositor, and compares two images.
//!
//! Differences of the port, none of which changes what is drawn or how it
//! is compared:
//!
//! - Upstream's static constructor initializes the services of the process
//!   once. The services of the port belong to a thread and the tests of a
//!   test binary run on threads of their own, so `begin_test` initializes
//!   the thread it is called on.
//! - Upstream writes its output files next to the expected images, inside
//!   the repository. The port writes them under the build directory
//!   (`render-tests-output` beside the profile directory of the test
//!   binary), in the same relative layout.
//! - The Mesa GL and Vulkan outputs of upstream (`MesaSoftwareRenderer`)
//!   are compiled only where that software renderer exists and are off
//!   unless an environment variable asks for them; the port has no GL or
//!   Vulkan surface of that kind and renders the two outputs upstream
//!   renders by default.

use crate::manual_render_timer::ManualRenderTimer;
use crate::test_render_root::TestRenderRoot;
use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::{Bitmap, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::media::MediaContext;
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{
    register_manifest_resources, IAssetLoader, ICursorFactory, ICursorImpl, IPlatformRenderInterface, ITextShaperImpl,
    IWriteableBitmapImpl, StandardAssetLoader,
};
use ferroui_base::rendering::composition::{CompositingRenderer, Compositor, RenderSurfaces};
use ferroui_base::rendering::{IRenderer, RenderLoop};
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, PixelSize, Rect, Ref, Size, Vector};
use ferroui_controls::Control;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use std::any::Any;
use std::cell::Cell;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex, Once};

/// The assembly the assets of the tests are manifest resources of.
pub const ASSEMBLY: &str = "ferroui-render-tests";

/// The largest error two images may have and count as the same image.
pub const ALLOWED_ERROR: f64 = 0.022;

struct NullCursorFactory;

struct NullCursor;

impl ICursorImpl for NullCursor {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICursorFactory for NullCursorFactory {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(NullCursor)
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(NullCursor)
    }
}

/// The name of the render backend of this configuration.
pub fn backend_name() -> &'static str {
    if cfg!(feature = "vello") {
        "vello"
    } else {
        "skia"
    }
}

/// Upstream's static constructor.
fn initialize_thread() {
    thread_local! {
        static INITIALIZED: Cell<bool> = const { Cell::new(false) };
    }

    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| register_manifest_resources(ASSEMBLY, crate::assets::RESOURCES));

    if INITIALIZED.replace(true) {
        return;
    }

    #[cfg(not(feature = "vello"))]
    ferroui_skia::SkiaPlatform::initialize();
    #[cfg(feature = "vello")]
    ferroui_vello::VelloPlatform::initialize_with_options(ferroui_vello::VelloOptions::with_rendering_mode(
        ferroui_vello::VelloRenderingMode::Cpu,
    ));

    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(Rc::new(StandardAssetLoader::new(None)))
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()))
        .bind::<dyn ICursorFactory>()
        .to_constant(Rc::new(NullCursorFactory));
}

pub fn render_to_file(target: &Ref<Control>, path: &Path, immediate: bool, dpi: f64) {
    if immediate {
        ensure_output_directory(path);

        let pixel_size = PixelSize::new(target.width() as i32, target.height() as i32);
        let size = Size::new(target.width(), target.height());
        let dpi_vector = Vector::new(dpi, dpi);

        let bitmap = RenderTargetBitmap::with_dpi(pixel_size, dpi_vector);
        target.measure(size);
        target.arrange(Rect::from_size(size));
        bitmap.render(&target.clone().upcast());
        bitmap
            .save_to_file(path.to_str().expect("the path is text"), &PngBitmapEncoderOptions::default().into())
            .expect("the image is saved");
        bitmap.dispose();

        return;
    }

    render_composited_to_file(target, path, dpi);
}

pub fn render_composited_to_file(target: &Ref<Control>, path: &Path, dpi: f64) {
    ensure_output_directory(path);

    let factory = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
    let pixel_size = PixelSize::new(target.width() as i32, target.height() as i32);
    let dpi_vector = Vector::new(dpi, dpi);
    let scaling = dpi / 96.0;

    let timer = Arc::new(ManualRenderTimer::new());

    let compositor = Compositor::with_scheduler(
        RenderLoop::from_timer(timer),
        None,
        true,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );
    let writeable_bitmap = factory.create_writeable_bitmap(
        pixel_size,
        dpi_vector,
        factory.default_pixel_format(),
        factory.default_alpha_format(),
    );

    let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(BitmapFramebufferSurface { bitmap: writeable_bitmap.clone() });
    let surfaces: RenderSurfaces = Arc::new(move || vec![surface.clone()]);
    let root = TestRenderRoot::new(scaling);
    let renderer = CompositingRenderer::new(&root.source(), &compositor, surfaces);
    root.initialize(&renderer, target);
    renderer.start();
    Dispatcher::ui_thread().run_jobs(None);
    renderer.paint(Rect::from_size(root.bounds().size()));
    renderer.dispose();
    // Detach the control, so it can be rendered again with a different root
    root.set_child(None);
    root.release();

    let mut file_stream = std::fs::File::create(path).expect("the output file is created");
    writeable_bitmap.save(&mut file_stream, &PngBitmapEncoderOptions::default().into()).expect("the image is saved");
    file_stream.flush().expect("the image is written");
    writeable_bitmap.dispose();
}

fn ensure_output_directory(path: &Path) {
    let dir = path.parent().expect("the output file is in a directory");

    if !dir.exists() {
        std::fs::create_dir_all(dir).expect("the output directory is created");
    }
}

struct BitmapFramebufferSurface {
    bitmap: Arc<dyn IWriteableBitmapImpl>,
}

impl IPlatformRenderSurface for BitmapFramebufferSurface {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for BitmapFramebufferSurface {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let bitmap = self.bitmap.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || bitmap.lock()))
    }
}

/// Upstream resets the dispatcher of the process (`Dispatcher.ResetBeforeUnitTests`); the tests of the port run
/// on threads of their own, so the test gets the dispatcher of its thread for as long as the returned scope
/// lives, and `end_test` takes the scope back.
pub fn begin_test() -> UnitTestDispatcherScope {
    let scope = Dispatcher::unit_test_scope();
    initialize_thread();
    scope
}

pub fn end_test(scope: UnitTestDispatcherScope) {
    if Dispatcher::ui_thread().check_access() {
        Dispatcher::ui_thread().run_jobs(None);
    }
    // `Dispatcher.ResetForUnitTests`.
    drop(scope);
}

/// The directory of the test crate, which holds `TestFiles`. Upstream
/// walks up from the working directory to its `tests` directory.
pub fn get_tests_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The directory the outputs of the tests are written under: never inside
/// the repository. `FERROUI_RENDER_TESTS_OUTPUT` names it; otherwise it is
/// `render-tests-output` in the build directory the test binary is in.
pub fn get_output_directory() -> PathBuf {
    if let Some(directory) = std::env::var_os("FERROUI_RENDER_TESTS_OUTPUT") {
        return PathBuf::from(directory).join(backend_name());
    }

    let executable = std::env::current_exe().expect("the test binary has a path");
    // <build directory>/<profile>/deps/<binary>
    let build_directory = executable.ancestors().nth(3).expect("the test binary is in a build directory");
    build_directory.join("render-tests-output").join(backend_name())
}

/// Appends the measured error of a comparison to `results.tsv` of the
/// output directory (not from upstream): the name of the output file
/// relative to that directory, the error and whether it is within the
/// allowed error.
fn record_result(actual_path: &Path, error: Option<f64>) {
    static LOCK: Mutex<()> = Mutex::new(());

    let directory = get_output_directory();
    let name = actual_path.strip_prefix(&directory).unwrap_or(actual_path).display().to_string();
    let line = match error {
        Some(error) => format!("{name}\t{error:.6}\t{}\n", if error > ALLOWED_ERROR { "fail" } else { "pass" }),
        None => format!("{name}\t\tfail\n"),
    };

    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(directory.join("results.tsv")) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// An image as upstream loads it for the comparison: eight bits a channel,
/// red, green, blue and alpha, not premultiplied.
pub struct Rgba32Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

impl Rgba32Image {
    pub fn load(path: &Path) -> Rgba32Image {
        let file = std::fs::File::open(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let mut buffer = vec![0; reader.output_buffer_size().expect("the image has a size")];
        let info = reader.next_frame(&mut buffer).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let (width, height) = (info.width as usize, info.height as usize);
        let data = &buffer[..info.buffer_size()];

        let pixels: Vec<[u8; 4]> = match info.color_type {
            png::ColorType::Rgba => data.chunks_exact(4).map(|p| [p[0], p[1], p[2], p[3]]).collect(),
            png::ColorType::Rgb => data.chunks_exact(3).map(|p| [p[0], p[1], p[2], 255]).collect(),
            png::ColorType::Grayscale => data.iter().map(|p| [*p, *p, *p, 255]).collect(),
            png::ColorType::GrayscaleAlpha => data.chunks_exact(2).map(|p| [p[0], p[0], p[0], p[1]]).collect(),
            png::ColorType::Indexed => panic!("{}: the palette was not expanded", path.display()),
        };
        assert_eq!(width * height, pixels.len(), "{}", path.display());

        Rgba32Image { width, height, pixels }
    }

    fn get(&self, x: usize, y: usize) -> [u8; 4] {
        self.pixels[y * self.width + x]
    }
}

pub fn assert_compare_images(actual_path: &Path, expected_path: &Path) {
    let expected = Rgba32Image::load(expected_path);
    let actual = Rgba32Image::load(actual_path);
    let immediate_error = match compare_images(&actual, &expected) {
        Ok(error) => error,
        Err(message) => {
            record_result(actual_path, None);
            panic!("{}: {message}", actual_path.display());
        }
    };
    record_result(actual_path, Some(immediate_error));

    if immediate_error > 0.022 {
        panic!("{}: Error = {}", actual_path.display(), immediate_error);
    }
}

/// The comparison of `TestBase.CompareImages`: the error of one output
/// against the expected image, with the allowed error of the test base.
pub fn compare_output(actual_path: &Path, expected: &Rgba32Image, allowed_error: f64) -> Result<(), String> {
    let actual = Rgba32Image::load(actual_path);
    let error = match compare_images(&actual, expected) {
        Ok(error) => error,
        Err(message) => {
            record_result(actual_path, None);
            return Err(format!("{}: {message}", actual_path.display()));
        }
    };
    record_result(actual_path, Some(error));

    if error > allowed_error {
        return Err(format!("{}: Error = {}", actual_path.display(), error));
    }

    Ok(())
}

/// Calculates root mean square error for given two images.
/// Based roughly on ImageMagick implementation to ensure consistency.
pub fn compare_images(actual: &Rgba32Image, expected: &Rgba32Image) -> Result<f64, String> {
    if actual.width != expected.width || actual.height != expected.height {
        return Err(format!(
            "Images have different resolutions ({}x{} and {}x{} expected)",
            actual.width, actual.height, expected.width, expected.height
        ));
    }

    let quantity = actual.width * actual.height;
    let mut squares_error = 0.0f64;

    const SCALE: f64 = 1.0 / 255.0;

    for x in 0..actual.width {
        let mut local_error = 0.0f64;

        for y in 0..actual.height {
            let e = expected.get(x, y);
            let a = actual.get(x, y);
            let expected_alpha = e[3] as f64 * SCALE;
            let actual_alpha = a[3] as f64 * SCALE;

            let r = SCALE * (expected_alpha * e[0] as f64 - actual_alpha * a[0] as f64);
            let g = SCALE * (expected_alpha * e[1] as f64 - actual_alpha * a[1] as f64);
            let b = SCALE * (expected_alpha * e[2] as f64 - actual_alpha * a[2] as f64);
            let alpha = expected_alpha - actual_alpha;

            let error = r * r + g * g + b * b + alpha * alpha;

            local_error += error;
        }

        squares_error += local_error;
    }

    let mut mean_squares_error = squares_error / quantity as f64;

    const CHANNEL_COUNT: f64 = 4.0;

    mean_squares_error /= CHANNEL_COUNT;

    Ok(mean_squares_error.sqrt())
}
