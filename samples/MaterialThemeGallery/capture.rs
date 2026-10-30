//! The capture mode of the gallery and what the window and the comparison host share with it:
//! a page shown in a [`CaptureHost`] in a [`Setting`] (theme variant, density, flow direction),
//! the pointer-over and pressed states forced on the elements that ask for them, the checks of a
//! page (a control without a template, an error the bindings or the framework logged), and the
//! picture of the host drawn by the framework at a fixed size and scale.
//!
//! # The capture mode
//!
//! With `FERROUI_MATERIAL_GALLERY_CAPTURE=<directory>` the desktop entry point shows every page in
//! every setting, one after the other, in a host of a fixed size, and writes the picture of the
//! host as `<directory>/<page>-<setting>.png` (`buttons-light.png`, `buttons-dark-compact.png`,
//! `buttons-light-rtl.png`). One line is printed per picture; when every picture is written the
//! main window is closed, and the process exits with 1 if anything failed: a page that does not
//! load, a control without a template, an error a binding or the framework logged while the page
//! was shown, a picture that could not be written. Only the frame the framework draws is
//! written, never the screen.
//!
//! The options, all optional:
//!
//! | Variable | Default | What |
//! |---|---|---|
//! | `FERROUI_MATERIAL_GALLERY_PAGES` | every page | ids of pages, separated by commas |
//! | `FERROUI_MATERIAL_GALLERY_VARIANTS` | `Light,Dark` | `Light`, `Dark`, `HighContrast` |
//! | `FERROUI_MATERIAL_GALLERY_DENSITIES` | `normal,compact` | `normal`, `compact` |
//! | `FERROUI_MATERIAL_GALLERY_FLOWS` | `ltr` | `ltr`, `rtl` |
//! | `FERROUI_MATERIAL_GALLERY_SIZE` | `720x960` | the size of the host, in device-independent pixels |
//! | `FERROUI_MATERIAL_GALLERY_SCALE` | `1` | the scale of the pictures (2 is 192 DPI) |
//! | `FERROUI_MATERIAL_GALLERY_SETTLE_MS` | `300` | the wait after a page is shown and after its states are forced |
//! | `FERROUI_MATERIAL_GALLERY_ACCENT` | the platform's | an accent colour, `#RRGGBB` |
//! | `FERROUI_MATERIAL_GALLERY_TIMEOUT_S` | `900` | the run is ended with the exit code 3 after this |

use crate::capture_host::CaptureHost;
use crate::pages::{PageInfo, PAGES};
use crate::theme::{self, Density, Variant};
use crate::App;
use ferroui_base::logging::{ILogSink, LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::media::{Color, FlowDirection};
use ferroui_base::platform::PixelFormat;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{FerroObject, PixelRect, PixelSize, Ref, Vector, Visual};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Application, Canvas, Control, Window};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The class an element of a page carries to be shown in the pointer-over state.
pub const FORCE_POINTER_OVER: &str = "force-pointerover";
/// The class an element of a page carries to be shown in the pressed state (and pointer-over).
pub const FORCE_PRESSED: &str = "force-pressed";

/// Whether a capture failed (the exit code of the desktop entry point).
static FAILED: AtomicBool = AtomicBool::new(false);

/// Whether a capture of this process failed.
pub fn failed() -> bool {
    FAILED.load(Ordering::SeqCst)
}

/// The variant, the density and the flow direction of a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Setting {
    pub variant: Variant,
    pub density: Density,
    pub right_to_left: bool,
}

impl Setting {
    /// The name of the setting in the names of the pictures: the variant in lower case, then
    /// `-compact` and `-rtl` where they apply (`dark-compact-rtl`).
    pub fn name(&self) -> String {
        let mut name = self.variant.name().to_ascii_lowercase();
        if self.density == Density::Compact {
            name.push_str("-compact");
        }
        if self.right_to_left {
            name.push_str("-rtl");
        }
        name
    }

    pub fn flow_direction(&self) -> FlowDirection {
        if self.right_to_left {
            FlowDirection::RightToLeft
        } else {
            FlowDirection::LeftToRight
        }
    }
}

/// What a run of pictures shows, from the environment: the pages, the settings, the size and
/// scale of the pictures, the waits and the accent colour. `prefix` is the prefix of the names
/// of the variables (`FERROUI_MATERIAL_GALLERY` for the capture mode, `FERROUI_WINUI_COMPARE` for
/// the comparison host).
#[derive(Clone, Debug)]
pub struct Options {
    pub pages: Vec<&'static PageInfo>,
    pub settings: Vec<Setting>,
    /// The size of the host, in device-independent pixels.
    pub width: f64,
    pub height: f64,
    /// The scale of the pictures (1 is 96 DPI).
    pub scale: f64,
    pub settle: Duration,
    pub accent: Option<Color>,
    pub timeout: Duration,
}

/// The value of `<prefix>_<name>`, if it is set and not empty.
fn variable(prefix: &str, name: &str) -> Option<String> {
    std::env::var(format!("{prefix}_{name}")).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

/// The parts of a list separated by commas.
fn parts(list: &str) -> impl Iterator<Item = &str> {
    list.split(',').map(str::trim).filter(|part| !part.is_empty())
}

impl Options {
    /// The options of the environment; an option whose value cannot be read is an error.
    pub fn from_env(prefix: &str) -> Result<Options, String> {
        let pages = match variable(prefix, "PAGES") {
            None => PAGES.iter().collect(),
            Some(list) => parts(&list)
                .map(|id| PageInfo::find(id).ok_or_else(|| format!("{prefix}_PAGES: there is no page {id:?}")))
                .collect::<Result<Vec<_>, _>>()?,
        };
        let variants = match variable(prefix, "VARIANTS") {
            None => vec![Variant::Light, Variant::Dark],
            Some(list) => parts(&list)
                .map(|name| Variant::parse(name).ok_or_else(|| format!("{prefix}_VARIANTS: there is no variant {name:?}")))
                .collect::<Result<Vec<_>, _>>()?,
        };
        let densities = match variable(prefix, "DENSITIES") {
            None => Density::ALL.to_vec(),
            Some(list) => parts(&list)
                .map(|name| Density::parse(name).ok_or_else(|| format!("{prefix}_DENSITIES: there is no density {name:?}")))
                .collect::<Result<Vec<_>, _>>()?,
        };
        let flows = match variable(prefix, "FLOWS") {
            None => vec![false],
            Some(list) => parts(&list)
                .map(|name| match name.to_ascii_lowercase().as_str() {
                    "ltr" => Ok(false),
                    "rtl" => Ok(true),
                    _ => Err(format!("{prefix}_FLOWS: {name:?} is neither ltr nor rtl")),
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let mut settings = Vec::new();
        for &variant in &variants {
            for &density in &densities {
                for &right_to_left in &flows {
                    settings.push(Setting { variant, density, right_to_left });
                }
            }
        }
        let (width, height) = match variable(prefix, "SIZE") {
            None => (720.0, 960.0),
            Some(size) => {
                let (width, height) = size
                    .split_once(['x', 'X'])
                    .ok_or_else(|| format!("{prefix}_SIZE: {size:?} is not <width>x<height>"))?;
                let parse = |text: &str| text.trim().parse::<f64>().ok().filter(|value| *value >= 16.0);
                match (parse(width), parse(height)) {
                    (Some(width), Some(height)) => (width, height),
                    _ => return Err(format!("{prefix}_SIZE: {size:?} is not <width>x<height> of at least 16 each")),
                }
            }
        };
        let scale = match variable(prefix, "SCALE") {
            None => 1.0,
            Some(scale) => scale
                .parse::<f64>()
                .ok()
                .filter(|scale| (0.5..=4.0).contains(scale))
                .ok_or_else(|| format!("{prefix}_SCALE: {scale:?} is not a scale between 0.5 and 4"))?,
        };
        let settle = match variable(prefix, "SETTLE_MS") {
            None => Duration::from_millis(300),
            Some(ms) => {
                Duration::from_millis(ms.parse::<u64>().map_err(|_| format!("{prefix}_SETTLE_MS: {ms:?} is not a number"))?)
            }
        };
        let accent = match variable(prefix, "ACCENT") {
            None => None,
            Some(text) => Some(Color::try_parse(&text).ok_or_else(|| format!("{prefix}_ACCENT: {text:?} is not a colour"))?),
        };
        let timeout = match variable(prefix, "TIMEOUT_S") {
            None => Duration::from_secs(900),
            Some(s) => Duration::from_secs(s.parse::<u64>().map_err(|_| format!("{prefix}_TIMEOUT_S: {s:?} is not a number"))?),
        };
        if pages.is_empty() || settings.is_empty() {
            return Err(format!("{prefix}: nothing to show (no page or no setting)"));
        }
        Ok(Options { pages, settings, width, height, scale, settle, accent, timeout })
    }

    /// The size of the pictures, in pixels.
    pub fn pixel_size(&self) -> PixelSize {
        PixelSize::new((self.width * self.scale).round() as i32, (self.height * self.scale).round() as i32)
    }
}

/// Shows `page` in `host` in `setting`: the density of the theme, the theme variant and the
/// flow direction of the host, then the page.
pub fn show(host: &CaptureHost, page: Option<Ref<Control>>, setting: Setting) {
    if let Some(gallery_theme) = App::current_theme() {
        theme::set_density(&gallery_theme, setting.density);
    }
    host.set_variant(setting.variant);
    host.set_page_flow_direction(setting.flow_direction());
    host.set_page(page);
}

/// Lays the host out (which applies the templates of the page) and forces the pointer-over and
/// pressed states the elements of the page ask for. Returns the number of elements whose state
/// was forced.
pub fn settle(host: &CaptureHost) -> usize {
    host.update_layout();
    let forced = host.page().map_or(0, |page| force_states(&page));
    host.update_layout();
    forced
}

/// Forces the pointer-over state on the elements of `root` (itself included) that have the class
/// [`FORCE_POINTER_OVER`], and the pressed state (with pointer-over) on those with
/// [`FORCE_PRESSED`]: the pseudo-classes are set as the control sets them under a pointer. The
/// control resets them on the next pointer event that reaches it, and a button resets
/// `:pressed` when its template is applied again. Returns the number of elements.
pub fn force_states(root: &Ref<Control>) -> usize {
    let mut forced = 0;
    for visual in root.get_self_and_visual_descendants() {
        let classes = visual.classes();
        let pressed = classes.contains(FORCE_PRESSED);
        if pressed || classes.contains(FORCE_POINTER_OVER) {
            visual.pseudo_classes().set(":pointerover", true);
            if pressed {
                visual.pseudo_classes().set(":pressed", true);
            }
            forced += 1;
        }
    }
    forced
}

/// The templated controls of `root` (itself included) that are visible and have no template,
/// or whose template was not applied (it made no visual child): what a theme that has no control
/// theme for a control, or a broken one, leaves.
pub fn missing_templates(root: &Ref<Control>) -> Vec<String> {
    let mut missing = Vec::new();
    for visual in root.get_self_and_visual_descendants() {
        let Some(control) = visual.cast::<TemplatedControl>() else { continue };
        if !control.is_effectively_visible() {
            continue;
        }
        let name = control.get_type().name();
        if control.template().is_none() {
            missing.push(format!("{name} has no template"));
        } else if control.visual_children_count() == 0 {
            missing.push(format!("the template of {name} made no visual"));
        }
    }
    missing.sort();
    missing.dedup();
    missing
}

/// Creates the page; a panic of its constructor (a document that does not load) is the error.
pub fn create_page(page: &PageInfo) -> Result<Ref<Control>, String> {
    std::panic::catch_unwind(page.create).map_err(|payload| panic_text(payload.as_ref()))
}

/// The message of a panic.
pub fn panic_text(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "a panic without a message".to_string()
    }
}

/// Draws `visual` into a bitmap of `pixel_size` at `scale` (96 DPI times the scale), through the
/// render target of the framework: the frame the framework draws, not a capture of the screen.
pub fn render(visual: &Ref<Visual>, pixel_size: PixelSize, scale: f64) -> RenderTargetBitmap {
    let bitmap = RenderTargetBitmap::with_dpi(pixel_size, Vector::new(96.0 * scale, 96.0 * scale));
    bitmap.render(visual);
    bitmap
}

/// Writes a bitmap as a PNG file, creating its directory.
pub fn save_png(bitmap: &Bitmap, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    bitmap.save_to_file(&path.to_string_lossy(), &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
}

/// The pixels of a bitmap as four bytes a pixel, blue first, rows from the top without padding.
pub fn bgra_pixels(bitmap: &Bitmap) -> Vec<u8> {
    let size = bitmap.pixel_size();
    let stride = size.width * 4;
    let mut pixels = vec![0u8; (stride * size.height).max(0) as usize];
    bitmap.copy_pixels(PixelRect::default(), &mut pixels, stride);
    if bitmap.format() == Some(PixelFormat::RGBA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    pixels
}

/// The events of the log that fail a picture: an event of the bindings from the level warning
/// on, and an event of any area from the level error on. Installed as the log sink of the
/// thread; every event is passed on to the sink of the process, which prints it.
#[derive(Default)]
pub struct ProblemLog {
    lines: RefCell<Vec<String>>,
}

impl ProblemLog {
    /// Installs a log as the sink of this thread and returns it.
    pub fn install() -> Rc<ProblemLog> {
        let log = Rc::new(ProblemLog::default());
        let sink: Rc<dyn ILogSink> = log.clone();
        Logger::set_thread_sink(Some(sink));
        log
    }

    /// The events since the last call.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.lines.borrow_mut())
    }

    fn counts(level: LogEventLevel, area: &str) -> bool {
        (area == LogArea::BINDING && level >= LogEventLevel::Warning) || level >= LogEventLevel::Error
    }
}

/// `template` with its `{Placeholders}` replaced by the values, in order.
fn format_template(template: &str, values: &[&dyn Display]) -> String {
    let mut text = String::new();
    let mut values = values.iter();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(length) = rest[start..].find('}') else { break };
        text.push_str(&rest[..start]);
        match values.next() {
            Some(value) => text.push_str(&value.to_string()),
            None => text.push_str(&rest[start..start + length + 1]),
        }
        rest = &rest[start + length + 1..];
    }
    text.push_str(rest);
    text
}

impl ILogSink for ProblemLog {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        Self::counts(level, area) || Logger::sink().is_some_and(|sink| sink.is_enabled(level, area))
    }

    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn Any>, message_template: &str) {
        self.log_with_values(level, area, source, message_template, &[]);
    }

    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        source: Option<&dyn Any>,
        message_template: &str,
        property_values: &[&dyn Display],
    ) {
        if Self::counts(level, area) {
            let source_name = source
                .and_then(|source| source.downcast_ref::<FerroObject>())
                .map_or("(unknown)", |object| object.get_type().name());
            self.lines.borrow_mut().push(format!(
                "[{area} {level:?}] {source_name}: {}",
                format_template(message_template, property_values)
            ));
        }
        if let Some(sink) = Logger::sink() {
            if sink.is_enabled(level, area) {
                sink.log_with_values(level, area, source, message_template, property_values);
            }
        }
    }
}

/// The main window of the application with the classic desktop lifetime.
fn main_window() -> Option<Ref<Window>> {
    let lifetime = Application::current().and_then(|application| application.application_lifetime());
    lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).and_then(|l| l.main_window())
}

/// Ends the application with `exit_code`: the lifetime is shut down, which closes the main
/// window.
fn end(exit_code: i32) {
    let lifetime = Application::current().and_then(|application| application.application_lifetime());
    let desktop = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime());
    match desktop {
        Some(desktop) => {
            if !desktop.try_shutdown(exit_code) {
                if let Some(window) = desktop.main_window() {
                    window.close();
                }
            }
        }
        None => {
            if let Some(window) = main_window() {
                window.close();
            }
        }
    }
}

/// Where a run is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The main window and its host are not there yet.
    Start,
    /// The page of the job is to be shown.
    Show,
    /// The page is shown; its states are to be forced once it has settled.
    Force,
    /// The states are forced; the picture is to be taken once they have settled.
    Shoot,
    /// Every job is done.
    Done,
}

/// A run of the capture mode.
struct Run {
    directory: PathBuf,
    options: Options,
    jobs: Vec<(&'static PageInfo, Setting)>,
    next: Cell<usize>,
    phase: Cell<Phase>,
    since: Cell<Instant>,
    host: RefCell<Option<Ref<CaptureHost>>>,
    page: RefCell<Option<Ref<Control>>>,
    log: RefCell<Option<Rc<ProblemLog>>>,
    written: Cell<usize>,
    failures: RefCell<Vec<String>>,
}

impl Run {
    fn job(&self) -> (&'static PageInfo, Setting) {
        self.jobs[self.next.get()]
    }

    fn file(&self, page: &PageInfo, setting: Setting) -> PathBuf {
        self.directory.join(format!("{}-{}.png", page.id, setting.name()))
    }

    fn fail(&self, page: &PageInfo, setting: Setting, what: &str) {
        println!("capture {} {}: FAILED: {what}", page.id, setting.name());
        self.failures.borrow_mut().push(format!("{} {}: {what}", page.id, setting.name()));
        FAILED.store(true, Ordering::SeqCst);
    }

    /// Moves on to the next job, or to the end.
    fn advance(&self) {
        self.next.set(self.next.get() + 1);
        self.phase.set(if self.next.get() < self.jobs.len() { Phase::Show } else { Phase::Done });
    }

    /// One tick of the run; false when it has ended.
    fn tick(&self) -> bool {
        match self.phase.get() {
            Phase::Start => {
                let Some(window) = main_window() else { return true };
                let host = CaptureHost::new();
                host.set_width(self.options.width);
                host.set_height(self.options.height);
                let canvas = Canvas::new();
                canvas.children().add(host.clone());
                window.set_content(Some(Control::boxed(canvas)));
                if let Some(gallery_theme) = App::current_theme() {
                    theme::set_accent(&gallery_theme, self.options.accent);
                }
                *self.log.borrow_mut() = Some(ProblemLog::install());
                *self.host.borrow_mut() = Some(host);
                self.phase.set(Phase::Show);
                true
            }
            Phase::Show => {
                let host = self.host.borrow().clone().expect("the host of the run");
                let (page, setting) = self.job();
                // The events of the page shown before do not count against this one.
                if let Some(log) = self.log.borrow().as_ref() {
                    log.take();
                }
                host.set_page(None);
                match create_page(page) {
                    Ok(control) => {
                        show(&host, Some(control.clone()), setting);
                        *self.page.borrow_mut() = Some(control);
                        self.since.set(Instant::now());
                        self.phase.set(Phase::Force);
                    }
                    Err(error) => {
                        self.fail(page, setting, &format!("the page does not load: {error}"));
                        self.advance();
                    }
                }
                true
            }
            Phase::Force => {
                if self.since.get().elapsed() < self.options.settle {
                    return true;
                }
                let host = self.host.borrow().clone().expect("the host of the run");
                settle(&host);
                self.since.set(Instant::now());
                self.phase.set(Phase::Shoot);
                true
            }
            Phase::Shoot => {
                if self.since.get().elapsed() < self.options.settle {
                    return true;
                }
                let host = self.host.borrow().clone().expect("the host of the run");
                let (page, setting) = self.job();
                host.update_layout();
                let mut problems = Vec::new();
                if let Some(control) = self.page.borrow().as_ref() {
                    problems.extend(missing_templates(control));
                }
                if let Some(log) = self.log.borrow().as_ref() {
                    problems.extend(log.take());
                }
                let path = self.file(page, setting);
                let pixel_size = self.options.pixel_size();
                let bitmap = render(&host.clone().upcast::<Visual>(), pixel_size, self.options.scale);
                let saved = save_png(&bitmap, &path);
                bitmap.dispose();
                match saved {
                    Err(error) => self.fail(page, setting, &format!("{} could not be written: {error}", path.display())),
                    Ok(()) => {
                        self.written.set(self.written.get() + 1);
                        if problems.is_empty() {
                            println!(
                                "capture {} {}: {} ({} by {} px)",
                                page.id,
                                setting.name(),
                                path.display(),
                                pixel_size.width,
                                pixel_size.height
                            );
                        } else {
                            self.fail(page, setting, &format!("{} written, with {} problem(s):", path.display(), problems.len()));
                            for problem in &problems {
                                println!("    {problem}");
                            }
                        }
                    }
                }
                self.advance();
                true
            }
            Phase::Done => {
                let failures = self.failures.borrow();
                println!(
                    "capture: {} picture(s) of {} written to {}, {} failure(s)",
                    self.written.get(),
                    self.jobs.len(),
                    self.directory.display(),
                    failures.len()
                );
                for failure in failures.iter() {
                    println!("    {failure}");
                }
                end(if failures.is_empty() { 0 } else { 1 });
                false
            }
        }
    }
}

/// Starts the capture mode into `directory` (see the module). Called once the application is set
/// up (`AppBuilder::after_setup`); the run ends the application.
pub fn start(directory: PathBuf) {
    let options = match Options::from_env("FERROUI_MATERIAL_GALLERY") {
        Ok(options) => options,
        Err(error) => {
            println!("capture: FAILED: {error}");
            FAILED.store(true, Ordering::SeqCst);
            let _timer = DispatcherTimer::run_once(|| end(2), Duration::ZERO, DispatcherPriority::NORMAL);
            return;
        }
    };
    let jobs: Vec<(&'static PageInfo, Setting)> =
        options.pages.iter().flat_map(|&page| options.settings.iter().map(move |&setting| (page, setting))).collect();
    println!(
        "capture: {} page(s) in {} setting(s), {} by {} at scale {}, into {}",
        options.pages.len(),
        options.settings.len(),
        options.width,
        options.height,
        options.scale,
        directory.display()
    );

    // A run that does not end (a page whose constructor never returns, a window that is never
    // created) is ended with the exit code 3.
    let timeout = options.timeout;
    let _ = std::thread::Builder::new().name("capture watchdog".to_owned()).spawn(move || {
        std::thread::sleep(timeout);
        println!("capture: FAILED: the run did not end within {} s", timeout.as_secs());
        std::process::exit(3);
    });

    let run = Rc::new(Run {
        directory,
        options,
        jobs,
        next: Cell::new(0),
        phase: Cell::new(Phase::Start),
        since: Cell::new(Instant::now()),
        host: RefCell::new(None),
        page: RefCell::new(None),
        log: RefCell::new(None),
        written: Cell::new(0),
        failures: RefCell::new(Vec::new()),
    });
    let _timer = DispatcherTimer::run(move || run.tick(), Duration::from_millis(50), DispatcherPriority::NORMAL);
}
