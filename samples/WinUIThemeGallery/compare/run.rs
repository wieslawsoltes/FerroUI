//! The run of the comparison: every page in every setting, the native page and the framework's
//! page shown at the same size, theme and flow direction, then the pictures.
//!
//! For each page and setting (`<page>-<setting>`, as the capture mode names its pictures) the
//! run writes into the output directory:
//!
//! - `<page>-<setting>-winui.png`: the native cell, cut out of a picture of the client area of
//!   the window (the window printing itself, `window_capture.rs`);
//! - `<page>-<setting>-ferroui.png`: the host of the framework's page drawn by the framework at
//!   the scale of the window (the frame of the framework, as the capture mode draws it);
//! - `<page>-<setting>-ferroui-window.png`, side by side only: the panel cut out of the same
//!   picture of the window, which shows that the panel presents what the framework draws;
//! - `<page>-<setting>-diff.png`: the pixels that differ in red over the native page in grey;
//!
//! and prints a line with the share of the pixels that differ. `compare.tsv` in the directory
//! has one line per comparison (`scripts/winui-theme/report.py` makes the report of it).
//!
//! A calibration run (`FERROUI_WINUI_COMPARE_TEXT_VARIANTS`, docs/porting/text-rendering.md,
//! section 10) compares every page and setting once per text setting of the list: before the
//! pages of a text setting are shown, the run sets the development override of the text
//! rendering of the process (`ferroui_skia::set_text_rendering_override`), and a text setting
//! with a `baseline` sets `TextBlock.LayoutRoundingPlacement` on each framework's page it shows
//! (over the value of the theme); it writes the pictures and `compare.tsv` of each text setting
//! into a directory of its name below the output directory, with `text-variants.tsv` listing them
//! (`scripts/text-quality.py <out> --variants` makes one table of them).

use crate::compare_window::CompareWindow;
use crate::window_capture;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_controls::{Control, TextBlock, TextLayoutRoundingPlacement};
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use winui_theme_gallery::capture::{self, Options, ProblemLog, Setting};
use winui_theme_gallery::comparison::{self, TextVariant, TEXT_VARIANTS_ENV};
use winui_theme_gallery::PageInfo;

/// Whether the run failed (the exit code of the process).
static FAILED: AtomicBool = AtomicBool::new(false);

/// Whether the run failed.
pub fn failed() -> bool {
    FAILED.load(Ordering::SeqCst)
}

/// How long the panel has to load and the window to be found.
const START_TIMEOUT: Duration = Duration::from_secs(60);
/// The difference of a colour channel up to which two pixels are the same, by default.
const DEFAULT_THRESHOLD: u8 = 24;
/// The header of `compare.tsv`.
const TSV_HEADER: &str = "page\tsetting\tpercent\tdiffering\twidth\theight\twinui\tferroui\tdiff\tferroui_window\tproblems";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The panel is to load and the window to be found.
    Start,
    /// The window is placed; the layout is to settle.
    Placed,
    /// The pages of the job are to be shown.
    Show,
    /// The pages are shown; the framework's states are to be forced.
    Force,
    /// The pictures are to be taken.
    Shoot,
    Done,
}

struct Run {
    window: Weak<CompareWindow>,
    out: PathBuf,
    options: Options,
    threshold: u8,
    /// The text settings of a calibration run, empty otherwise.
    variants: Vec<TextVariant>,
    /// The jobs: the text setting (an index into `variants`; `None` outside a calibration run),
    /// the page and the setting.
    jobs: Vec<(Option<usize>, &'static PageInfo, Setting)>,
    /// The text setting the overrides of the process are set to.
    applied_variant: Cell<Option<usize>>,
    /// The placement of the baseline the text setting sets on the pages (`None`: the theme's).
    baseline: Cell<Option<TextLayoutRoundingPlacement>>,
    next: Cell<usize>,
    phase: Cell<Phase>,
    since: Cell<Instant>,
    started: Instant,
    hwnd: Cell<isize>,
    side_by_side: Cell<bool>,
    /// The height of the pages: the height of the options, or less where the work area of the
    /// screen does not hold a page of that height (the bottom of each page is then left out of
    /// both pictures).
    page_height: Cell<f64>,
    /// Whether the native page of the job loaded.
    native_shown: Cell<bool>,
    page: RefCell<Option<Ref<Control>>>,
    log: Rc<ProblemLog>,
    /// The lines of `compare.tsv`, with the text setting of each.
    lines: RefCell<Vec<(Option<usize>, String)>>,
    failures: RefCell<Vec<String>>,
}

/// A picture of four bytes a pixel, blue first, as a PNG file at `scale`.
fn save_bgra(pixels: &[u8], size: (i32, i32), scale: f64, path: &Path) -> std::io::Result<()> {
    if size.0 < 1 || size.1 < 1 {
        return Err(std::io::Error::other(format!("the picture is empty ({} by {})", size.0, size.1)));
    }
    let bitmap = Bitmap::from_pixels(
        PixelFormat::BGRA8888,
        AlphaFormat::Opaque,
        pixels,
        PixelSize::new(size.0, size.1),
        Vector::new(96.0 * scale, 96.0 * scale),
        size.0 * 4,
    );
    let result = capture::save_png(&bitmap, path);
    bitmap.dispose();
    result
}

/// The rectangle `bounds` (device-independent pixels) in pixels at `scale`.
fn in_pixels(bounds: ferroui_winui::interop::Rect, scale: f64) -> (i32, i32, i32, i32) {
    let x = (f64::from(bounds.x) * scale).round() as i32;
    let y = (f64::from(bounds.y) * scale).round() as i32;
    let width = (f64::from(bounds.width) * scale).round() as i32;
    let height = (f64::from(bounds.height) * scale).round() as i32;
    (x, y, width, height)
}

/// The development override of the text rendering a text setting gives (those of the
/// environment of the process with the values of the setting), and the placement of the
/// baseline it sets on the pages (`None`: the theme's).
fn text_overrides(
    variant: &TextVariant,
) -> Result<(ferroui_skia::TextRenderingOverride, Option<TextLayoutRoundingPlacement>), String> {
    let mut rendering = ferroui_skia::startup_text_rendering_override();
    let mut baseline = None;
    for (key, value) in &variant.values {
        if key == "baseline" {
            baseline = comparison::parse_baseline(value)?;
        } else {
            rendering.set(key, value)?;
        }
    }
    Ok((rendering, baseline))
}

impl Run {
    fn job(&self) -> (&'static PageInfo, Setting) {
        let (_, page, setting) = self.jobs[self.next.get()];
        (page, setting)
    }

    fn job_variant(&self) -> Option<usize> {
        self.jobs[self.next.get()].0
    }

    /// The directory of the pictures of a text setting.
    fn directory(&self, variant: Option<usize>) -> PathBuf {
        match variant {
            Some(index) => self.out.join(&self.variants[index].name),
            None => self.out.clone(),
        }
    }

    fn name(&self) -> String {
        let (page, setting) = self.job();
        format!("{}-{}", page.id, setting.name())
    }

    /// The name of the job in the lines the run prints: with its text setting in a calibration
    /// run.
    fn label(&self) -> String {
        match self.job_variant() {
            Some(index) => format!("{}/{}", self.variants[index].name, self.name()),
            None => self.name(),
        }
    }

    /// Sets the overrides of the process to the text setting of the job, if it is another one.
    fn apply_text_variant(&self) {
        let Some(index) = self.job_variant() else {
            return;
        };
        if self.applied_variant.get() == Some(index) {
            return;
        }
        let variant = &self.variants[index];
        match text_overrides(variant) {
            Ok((rendering, baseline)) => {
                ferroui_skia::set_text_rendering_override(rendering);
                self.baseline.set(baseline);
                println!(
                    "compare: text setting {} ({}): the text rendering {rendering}, the baseline {}",
                    variant.name,
                    variant.specification(),
                    comparison::baseline_name(baseline)
                );
            }
            // Checked when the run started.
            Err(error) => self.fail(&format!("the text setting {}: {error}", variant.name)),
        }
        self.applied_variant.set(Some(index));
    }

    fn fail(&self, what: &str) {
        println!("compare: FAILED: {what}");
        self.failures.borrow_mut().push(what.to_string());
        FAILED.store(true, Ordering::SeqCst);
    }

    fn advance(&self) {
        self.next.set(self.next.get() + 1);
        self.phase.set(if self.next.get() < self.jobs.len() { Phase::Show } else { Phase::Done });
    }

    /// Writes `compare.tsv` of the text setting of the job with its lines so far.
    fn write_table(&self) {
        let variant = if self.next.get() < self.jobs.len() { self.job_variant() } else { None };
        self.write_table_of(variant);
    }

    fn write_table_of(&self, variant: Option<usize>) {
        let path = self.directory(variant).join("compare.tsv");
        let written = std::fs::File::create(&path).and_then(|mut file| {
            writeln!(file, "{TSV_HEADER}")?;
            for (_, line) in self.lines.borrow().iter().filter(|(of, _)| *of == variant) {
                writeln!(file, "{line}")?;
            }
            Ok(())
        });
        if let Err(error) = written {
            self.fail(&format!("{} could not be written: {error}", path.display()));
        }
    }

    /// Writes `text-variants.tsv` of a calibration run: a line per text setting with its
    /// directory, its specification and the overrides it gave.
    fn write_variants(&self) {
        if self.variants.is_empty() {
            return;
        }
        let path = self.out.join("text-variants.tsv");
        let written = std::fs::File::create(&path).and_then(|mut file| {
            writeln!(file, "name\ttext_rendering\tbaseline\tspecification")?;
            for variant in &self.variants {
                let (rendering, baseline) = match text_overrides(variant) {
                    Ok((rendering, baseline)) => (rendering.to_string(), comparison::baseline_name(baseline).to_string()),
                    Err(error) => (error, String::new()),
                };
                writeln!(file, "{}\t{rendering}\t{baseline}\t{}", variant.name, variant.specification())?;
            }
            Ok(())
        });
        if let Err(error) = written {
            self.fail(&format!("{} could not be written: {error}", path.display()));
        }
    }

    fn tick(&self) -> bool {
        let Some(window) = self.window.upgrade() else {
            self.fail("the window was closed before the run ended");
            return false;
        };
        match self.phase.get() {
            Phase::Start => self.start(&window),
            Phase::Placed => {
                if self.since.get().elapsed() >= self.options.settle {
                    self.phase.set(Phase::Show);
                }
                true
            }
            Phase::Show => {
                self.show(&window);
                true
            }
            Phase::Force => {
                if self.since.get().elapsed() >= self.options.settle {
                    capture::settle(window.host());
                    self.since.set(Instant::now());
                    self.phase.set(Phase::Shoot);
                }
                true
            }
            Phase::Shoot => {
                if self.since.get().elapsed() >= self.options.settle {
                    self.shoot(&window);
                    self.advance();
                }
                true
            }
            Phase::Done => {
                if self.variants.is_empty() {
                    self.write_table_of(None);
                } else {
                    for index in 0..self.variants.len() {
                        self.write_table_of(Some(index));
                    }
                    self.write_variants();
                    // The override of the environment again.
                    ferroui_skia::set_text_rendering_override(ferroui_skia::startup_text_rendering_override());
                    self.baseline.set(None);
                }
                let failures = self.failures.borrow();
                println!(
                    "compare: {} comparison(s) of {} written to {}, {} failure(s)",
                    self.lines.borrow().len(),
                    self.jobs.len(),
                    self.out.display(),
                    failures.len()
                );
                for failure in failures.iter() {
                    println!("    {failure}");
                }
                window.host().set_page(None);
                let _ = window.clear_native();
                if window.window().close().is_err() {
                    ferroui_winui::XamlApplication.exit();
                }
                false
            }
        }
    }

    /// Waits for the panel and the window, then sizes and places the window: side by side when
    /// two pages fit the work area of its screen, one over the other otherwise.
    fn start(&self, window: &CompareWindow) -> bool {
        let loaded = window.ferro_panel().get_embedded_root_for_automation().is_some();
        let hwnd = window_capture::find_window(window.title()).unwrap_or(0);
        if !loaded || hwnd == 0 {
            if self.started.elapsed() > START_TIMEOUT {
                self.fail(&format!(
                    "within {} s the panel did not load ({loaded}) or the window was not found ({})",
                    START_TIMEOUT.as_secs(),
                    hwnd != 0
                ));
                self.phase.set(Phase::Done);
            }
            return true;
        }
        self.hwnd.set(hwnd);
        let scale = window_capture::scale(hwnd);
        let width = (self.options.width * scale).ceil() as i32;
        let Some(work) = window_capture::work_area(hwnd) else {
            self.fail("the work area of the screen of the window could not be read");
            self.phase.set(Phase::Done);
            return true;
        };
        // The frame of the window takes some of the height of the work area.
        let frame_allowance = (48.0 * scale).ceil() as i32;
        // A page higher than the work area is compared at the height that fits, from its top.
        let fitting = f64::from(work.height() - frame_allowance) / scale;
        if self.options.height > fitting {
            let height = fitting.floor();
            if height < 200.0 || self.options.width * scale > f64::from(work.width()) {
                self.fail(&format!(
                    "a page of {} by {} at the scale {scale} does not fit the work area of the screen ({} by {} pixels): set FERROUI_WINUI_COMPARE_SIZE to a smaller size",
                    self.options.width,
                    self.options.height,
                    work.width(),
                    work.height()
                ));
                self.phase.set(Phase::Done);
                return true;
            }
            println!(
                "compare: the work area of the screen ({} by {} pixels) does not hold a page of {} by {} at the scale {scale}: the pages are compared at a height of {height}, from their top",
                work.width(),
                work.height(),
                self.options.width,
                self.options.height
            );
            self.page_height.set(height);
        }
        let height = (self.page_height.get() * scale).ceil() as i32;
        let fits = |columns: i32| columns * width <= work.width() && height + frame_allowance <= work.height();
        let side_by_side = fits(2);
        self.side_by_side.set(side_by_side);
        if let Err(error) = window.arrange(side_by_side, self.options.width, self.page_height.get()) {
            self.fail(&format!("the cells of the window could not be sized: {error}"));
            self.phase.set(Phase::Done);
            return true;
        }
        let columns = if side_by_side { 2 } else { 1 };
        match window_capture::place(hwnd, work.left, work.top, columns * width, height) {
            Ok((frame_width, frame_height)) => println!(
                "compare: the window is {frame_width} by {frame_height} pixels at the scale {scale}, the pages {}; {} job(s) into {}",
                if side_by_side { "side by side" } else { "one over the other (the screen is too small for both)" },
                self.jobs.len(),
                self.out.display()
            ),
            Err(error) => {
                self.fail(&format!("the window could not be placed: {error}"));
                self.phase.set(Phase::Done);
                return true;
            }
        }
        self.since.set(Instant::now());
        self.phase.set(Phase::Placed);
        true
    }

    /// Shows the native page and the framework's page of the job.
    fn show(&self, window: &CompareWindow) {
        self.apply_text_variant();
        let (page, setting) = self.job();
        let Some(markup) = comparison::native_markup(page, setting, self.options.width, self.page_height.get()) else {
            println!(
                "compare {} {}: skipped: WinUI has no requested theme for the variant {} (high contrast is a setting of the system)",
                page.id,
                setting.name(),
                setting.variant.name()
            );
            self.advance();
            return;
        };
        match window.show_native(&markup) {
            Ok(()) => self.native_shown.set(true),
            Err(error) => {
                self.native_shown.set(false);
                let _ = window.clear_native();
                self.fail(&format!("{}: the native page ({}) does not load in WinUI: {error}", self.label(), page.native_file));
            }
        }

        // The events of the page shown before do not count against this one.
        self.log.take();
        window.host().set_page(None);
        match capture::create_page(page) {
            Ok(control) => {
                // The placement of the baseline of the text setting, over the theme's: a local
                // value of the page, which its text blocks inherit.
                if let Some(placement) = self.baseline.get() {
                    TextBlock::set_layout_rounding_placement_on(&control, placement);
                }
                capture::show(window.host(), Some(control.clone()), setting);
                *self.page.borrow_mut() = Some(control);
            }
            Err(error) => {
                *self.page.borrow_mut() = None;
                self.fail(&format!("{}: the page of the framework does not load: {error}", self.label()));
            }
        }
        self.since.set(Instant::now());
        self.phase.set(Phase::Force);
    }

    /// Takes the pictures of the job, compares them and writes them.
    fn shoot(&self, window: &CompareWindow) {
        let file_name = self.name();
        // The name in the lines the run prints (with the text setting in a calibration run).
        let name = self.label();
        let directory = self.directory(self.job_variant());
        let hwnd = self.hwnd.get();
        let scale = window.rasterization_scale().unwrap_or_else(|| window_capture::scale(hwnd));
        let file = |suffix: &str| directory.join(format!("{file_name}-{suffix}.png"));

        // The framework's page, drawn by the framework.
        let mut problems = Vec::new();
        let mut ferro: Option<(Vec<u8>, (i32, i32))> = None;
        if let Some(page) = self.page.borrow().as_ref() {
            let host = window.host();
            host.update_layout();
            problems.extend(capture::missing_templates(page));
            problems.extend(self.log.take());
            let pixel_size =
                PixelSize::new((self.options.width * scale).round() as i32, (self.page_height.get() * scale).round() as i32);
            let bitmap = capture::render(&host.clone().upcast::<Visual>(), pixel_size, scale);
            let pixels = capture::bgra_pixels(&bitmap);
            let saved = capture::save_png(&bitmap, &file("ferroui"));
            bitmap.dispose();
            match saved {
                Ok(()) => ferro = Some((pixels, (pixel_size.width, pixel_size.height))),
                Err(error) => self.fail(&format!("{name}: the picture of the framework's page could not be written: {error}")),
            }
        }

        // The native page, and the panel side by side, cut out of a picture of the window.
        let mut native: Option<(Vec<u8>, (i32, i32))> = None;
        let mut ferro_window_file = String::new();
        match window_capture::capture_client_area(hwnd) {
            Err(error) => self.fail(&format!("{name}: the window could not be captured: {error}")),
            Ok(picture) => {
                let size = (picture.width, picture.height);
                if self.native_shown.get() {
                    match window.native_bounds() {
                        Err(error) => self.fail(&format!("{name}: the bounds of the native cell: {error}")),
                        Ok(bounds) => {
                            let (x, y, width, height) = in_pixels(bounds, scale);
                            let (pixels, cropped) = comparison::crop(&picture.pixels, size, x, y, width, height);
                            if window_capture::sampled_colors(&pixels, cropped.0, cropped.1) <= 1 {
                                problems.push(
                                    "the picture of the native page is of one colour: the capture of the window may not hold the content of WinUI"
                                        .to_string(),
                                );
                            }
                            match save_bgra(&pixels, cropped, scale, &file("winui")) {
                                Ok(()) => native = Some((pixels, cropped)),
                                Err(error) => self.fail(&format!("{name}: the picture of the native page could not be written: {error}")),
                            }
                        }
                    }
                }
                if self.side_by_side.get() {
                    if let Ok(bounds) = window.panel_bounds() {
                        let (x, y, width, height) = in_pixels(bounds, scale);
                        let (pixels, cropped) = comparison::crop(&picture.pixels, size, x, y, width, height);
                        if save_bgra(&pixels, cropped, scale, &file("ferroui-window")).is_ok() {
                            ferro_window_file = format!("{file_name}-ferroui-window.png");
                        }
                    }
                }
            }
        }

        if !problems.is_empty() {
            self.fail(&format!("{name}: {} problem(s) of the framework's page or of the capture", problems.len()));
            for problem in &problems {
                println!("    {problem}");
            }
        }

        let (Some((native_pixels, native_size)), Some((ferro_pixels, ferro_size))) = (native, ferro) else {
            println!("compare {name}: no comparison (a picture is missing)");
            return;
        };
        let difference = comparison::compare(&native_pixels, native_size, &ferro_pixels, ferro_size, self.threshold);
        if let Err(error) = save_bgra(&difference.image, (difference.width, difference.height), scale, &file("diff")) {
            self.fail(&format!("{name}: the picture of the difference could not be written: {error}"));
        }
        println!(
            "compare {name}: {:.2} % of the pixels differ ({} of {} by {} px)",
            difference.percent(),
            difference.differing,
            difference.width,
            difference.height
        );
        let (page, setting) = self.job();
        self.lines.borrow_mut().push((
            self.job_variant(),
            format!(
                "{}\t{}\t{:.4}\t{}\t{}\t{}\t{file_name}-winui.png\t{file_name}-ferroui.png\t{file_name}-diff.png\t{ferro_window_file}\t{}",
                page.id,
                setting.name(),
                difference.percent(),
                difference.differing,
                difference.width,
                difference.height,
                problems.len()
            ),
        ));
        self.write_table();
    }
}

/// Starts the run in `window` into `out`, once the application is launched; the run closes the
/// window when it has ended, which ends the application.
pub fn start(window: Rc<CompareWindow>, out: PathBuf) {
    let options = match Options::from_env("FERROUI_WINUI_COMPARE") {
        Ok(options) => options,
        Err(error) => {
            println!("compare: FAILED: {error}");
            FAILED.store(true, Ordering::SeqCst);
            let _ = window.window().close();
            return;
        }
    };
    let threshold = std::env::var("FERROUI_WINUI_COMPARE_THRESHOLD")
        .ok()
        .and_then(|value| value.trim().parse::<u8>().ok())
        .unwrap_or(DEFAULT_THRESHOLD);
    if std::env::var_os("FERROUI_WINUI_COMPARE_ACCENT").is_some() {
        println!("compare: FERROUI_WINUI_COMPARE_ACCENT is not used: the native pages have the accent colour of the system, and so has the theme");
    }
    if let Err(error) = std::fs::create_dir_all(&out) {
        println!("compare: FAILED: {} could not be created: {error}", out.display());
        FAILED.store(true, Ordering::SeqCst);
        let _ = window.window().close();
        return;
    }
    // The text settings of a calibration run, each checked before anything is shown.
    let variants = match comparison::parse_text_variants(&std::env::var(TEXT_VARIANTS_ENV).unwrap_or_default()) {
        Ok(variants) => variants,
        Err(error) => {
            println!("compare: FAILED: {TEXT_VARIANTS_ENV}: {error}");
            FAILED.store(true, Ordering::SeqCst);
            let _ = window.window().close();
            return;
        }
    };
    for variant in &variants {
        let directory = out.join(&variant.name);
        let checked = text_overrides(variant)
            .and_then(|_| std::fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display())));
        if let Err(error) = checked {
            println!("compare: FAILED: {TEXT_VARIANTS_ENV}: the text setting {}: {error}", variant.name);
            FAILED.store(true, Ordering::SeqCst);
            let _ = window.window().close();
            return;
        }
    }
    if !variants.is_empty() {
        println!(
            "compare: a calibration run of {} text setting(s): {}",
            variants.len(),
            variants.iter().map(TextVariant::specification).collect::<Vec<_>>().join("; ")
        );
    }
    let variant_indices: Vec<Option<usize>> =
        if variants.is_empty() { vec![None] } else { (0..variants.len()).map(Some).collect() };
    let (pages, settings) = (&options.pages, &options.settings);
    let jobs: Vec<(Option<usize>, &'static PageInfo, Setting)> = variant_indices
        .iter()
        .flat_map(|&variant| {
            pages.iter().flat_map(move |&page| settings.iter().map(move |&setting| (variant, page, setting)))
        })
        .collect();

    // A run that does not end is ended with the exit code 3.
    let timeout = options.timeout;
    let _ = std::thread::Builder::new().name("compare watchdog".to_owned()).spawn(move || {
        std::thread::sleep(timeout);
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "compare: FAILED: the run did not end within {} s", timeout.as_secs());
        let _ = out.flush();
        // SAFETY: ends this process.
        unsafe {
            windows_sys::Win32::System::Threading::TerminateProcess(windows_sys::Win32::System::Threading::GetCurrentProcess(), 3)
        };
    });

    let page_height = options.height;
    let run = Rc::new(Run {
        window: Rc::downgrade(&window),
        out,
        options,
        threshold,
        variants,
        jobs,
        applied_variant: Cell::new(None),
        baseline: Cell::new(None),
        next: Cell::new(0),
        phase: Cell::new(Phase::Start),
        since: Cell::new(Instant::now()),
        started: Instant::now(),
        hwnd: Cell::new(0),
        side_by_side: Cell::new(false),
        page_height: Cell::new(page_height),
        native_shown: Cell::new(false),
        page: RefCell::new(None),
        log: ProblemLog::install(),
        lines: RefCell::new(Vec::new()),
        failures: RefCell::new(Vec::new()),
    });
    let _timer =
        DispatcherTimer::run(move || run.tick(), Duration::from_millis(50), DispatcherPriority::BACKGROUND).detach();
}
