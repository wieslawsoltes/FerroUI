//! The smoke run of the sample (`--smoke`): what can be checked of the controls of WinUI beside
//! the content of the framework without a person in front of the window. The checks are listed
//! in docs/porting/winui.md, section 7.6; each prints a line, and the process exits with 0 only
//! if every check passed.

use crate::app::App;
use crate::main_window::MainWindow;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime;
use ferroui_controls::Application;
use ferroui_winui::XamlApplication;
use std::cell::Cell;
use std::io::Write;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use winui_embed_sample::EmbeddedView;

ferroui_base::ferro_thread_local! {
    static FAILED: Cell<bool> = const { Cell::new(false) };
}

/// Whether this is a smoke run (`--smoke`).
static SMOKE: AtomicBool = AtomicBool::new(false);
/// The last `[step]` line, which the watchdog of the end of the process names.
static LAST_STEP: Mutex<String> = Mutex::new(String::new());
/// How long the process has to end once the window is closed.
const END_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the panel has to load and to get its swap chain, and the walk of the worker probe
/// to come back.
const LOAD_TIMEOUT: Duration = Duration::from_secs(60);
/// How many frames the content has to render after the panel has loaded.
const FRAMES: u32 = 30;

/// `ElementTheme.Dark` and `ElementTheme.Default` of WinUI.
const ELEMENT_THEME_DARK: i32 = 2;
const ELEMENT_THEME_DEFAULT: i32 = 0;

/// Prints a check; a failed check fails the run.
pub fn report(ok: bool, what: &str) {
    println!("{} {what}", if ok { "[ ok ]" } else { "[FAIL]" });
    if !ok {
        FAILED.with(|failed| failed.set(true));
    }
}

fn info(what: &str) {
    println!("[info] {what}");
}

/// Whether every check passed.
pub fn passed() -> bool {
    !FAILED.with(Cell::get)
}

/// Marks the run as a smoke run: the steps of the end of the application are printed too.
pub fn set_smoke() {
    SMOKE.store(true, Ordering::SeqCst);
}

/// A step of the end of the application (the window closed, the loop ended, the Windows App SDK
/// released, `main` returning), printed in a smoke run.
pub fn end_step(what: &str) {
    if SMOKE.load(Ordering::SeqCst) {
        step(what);
    }
}

/// A line before a call of the system, so that a run that ends in the system's frames names the
/// call it was in.
fn step(what: &str) {
    if let Ok(mut last) = LAST_STEP.lock() {
        last.clear();
        last.push_str(what);
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "[step] {what}");
    let _ = out.flush();
}

/// From the close of the window on, a thread watches that the process ends: if it still runs
/// after [`END_TIMEOUT`], the run fails with the last step it reached, and the process is ended
/// (as in the smoke run of ControlGallery.WinUI; docs/porting/winui.md, section 7.3).
fn watch_the_end() {
    let _ = std::thread::Builder::new().name("smoke end watchdog".to_owned()).spawn(|| {
        std::thread::sleep(END_TIMEOUT);
        let last = LAST_STEP.lock().map(|last| last.clone()).unwrap_or_default();
        let mut out = std::io::stdout().lock();
        let _ = writeln!(
            out,
            "[FAIL] the process did not end within {} s after the window was closed; the last step: {last}",
            END_TIMEOUT.as_secs()
        );
        let _ = out.flush();
        // SAFETY: ends this process.
        unsafe {
            windows_sys::Win32::System::Threading::TerminateProcess(
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
                97,
            )
        };
    });
}

/// What the run waits for.
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    /// The panel to load, its swap chain and the frames of the content.
    Loading,
    /// The walk of the worker probe to come back to the UI thread.
    Worker,
}

/// Starts the checks once the main window is activated; the run closes the window and ends the
/// application when they are done. The timer holds the window weakly: `App` drops it when it is
/// closed.
pub fn start(window: Rc<MainWindow>) {
    report(App::lifetime().main_view().is_some(), "the embedded view is the main view of the single view lifetime");
    let diagnostics = window.diagnostics();
    report(diagnostics.contains("] ctor: "), "the constructor of the window probed the automation peer of the panel");
    info(&format!("the probe of the constructor: {}", line_of(&diagnostics, "] ctor: ")));

    let window: Weak<MainWindow> = Rc::downgrade(&window);
    let started = Instant::now();
    let frames = Rc::new(Cell::new(0u32));
    let counting = Rc::new(Cell::new(false));
    let phase = Rc::new(Cell::new(Phase::Loading));
    let worker_started = Rc::new(Cell::new(Instant::now()));
    let finished = Rc::new(Cell::new(false));
    DispatcherTimer::run(
        move || {
            if finished.get() {
                return false;
            }
            let Some(window) = window.upgrade() else {
                report(false, "the main window is gone before the checks ended");
                finished.set(true);
                return false;
            };
            match phase.get() {
                Phase::Loading => {
                    let panel = window.ferro_panel();
                    let root = panel.get_embedded_root_for_automation();
                    let loaded = root.is_some();
                    let attached = panel.is_swap_chain_attached();
                    if let (Some(root), false) = (&root, counting.get()) {
                        counting.set(true);
                        let size = root.client_size();
                        report(size.width > 0.0 && size.height > 0.0, &format!("the panel loaded: the content has the client size {size:?}"));
                        count_frames(root.clone().upcast(), frames.clone());
                    }
                    let elapsed = started.elapsed();
                    if loaded && attached && frames.get() >= FRAMES {
                        report(true, &format!("the swap chain is set on the panel ({} ms after activation)", elapsed.as_millis()));
                        report(true, &format!("{} frames rendered", frames.get()));
                        check_controls(&window);
                        step("Probe deep on worker thread");
                        window.probe_worker();
                        worker_started.set(Instant::now());
                        phase.set(Phase::Worker);
                        return true;
                    }
                    if elapsed > LOAD_TIMEOUT {
                        report(loaded, "the panel loaded");
                        report(attached, "the swap chain is set on the panel");
                        report(frames.get() >= FRAMES, &format!("{} of {FRAMES} frames rendered", frames.get()));
                        finished.set(true);
                        finish(&window);
                        return false;
                    }
                    true
                }
                Phase::Worker => {
                    let diagnostics = window.diagnostics();
                    let returned = diagnostics.contains("Worker-thread walk");
                    if returned || worker_started.get().elapsed() > LOAD_TIMEOUT {
                        report(returned, "the walk of the worker probe came back to the UI thread");
                        if returned {
                            info(&format!("the walk on the worker thread: {}", after(&diagnostics, "Worker-thread walk")));
                        }
                        finished.set(true);
                        finish(&window);
                        return false;
                    }
                    true
                }
            }
        },
        Duration::from_millis(250),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
}

/// Counts the frames of the content: each asks for the next.
fn count_frames(top_level: ferroui_base::Ref<ferroui_controls::TopLevel>, frames: Rc<Cell<u32>>) {
    let next = top_level.clone();
    top_level.request_animation_frame(move |_| {
        frames.set(frames.get() + 1);
        if frames.get() < FRAMES * 4 {
            count_frames(next, frames);
        }
    });
}

/// The line of the diagnostics that holds `marker`.
fn line_of(diagnostics: &str, marker: &str) -> String {
    diagnostics.lines().find(|line| line.contains(marker)).unwrap_or_default().to_string()
}

/// The diagnostics from `marker` on.
fn after(diagnostics: &str, marker: &str) -> String {
    diagnostics.find(marker).map(|index| diagnostics[index..].trim_end().to_string()).unwrap_or_default()
}

/// The controls of both sides: the handlers the window adds to the controls of WinUI, the theme
/// of both, the probes of the automation peer, and a control of the embedded view.
fn check_controls(window: &MainWindow) {
    let diagnostics = window.diagnostics();
    report(diagnostics.contains("] Loading: "), "the Loading event of the panel probed the peer");
    let loaded = line_of(&diagnostics, "] Loaded: ");
    report(
        loaded.contains("class=FerroSwapChainPanel") && loaded.contains("childCount=1"),
        &format!("the Loaded event of the panel probed the peer with the proxy of the content: {loaded}"),
    );

    step("WinUiSlider.Value = 60");
    let _ = window.win_ui_slider().set_value(60.0);
    let slider = window.win_ui_slider_value();
    report(slider == "Slider: 60", &format!("the slider of WinUI shows its value through ValueChanged: {slider:?}"));

    step("WinUiThemeCombo.SelectedIndex = 1");
    let _ = window.win_ui_theme_combo().set_selected_index(1);
    let dark = Application::current().and_then(|application| application.actual_theme_variant()) == Some(ThemeVariant::dark());
    report(
        window.requested_theme() == ELEMENT_THEME_DARK && dark,
        &format!(
            "the theme combo box of WinUI sets the dark theme on both sides (WinUI {}, the framework {})",
            window.requested_theme(),
            if dark { "dark" } else { "not dark" }
        ),
    );
    step("WinUiThemeCombo.SelectedIndex = 2");
    let _ = window.win_ui_theme_combo().set_selected_index(2);
    report(window.requested_theme() == ELEMENT_THEME_DEFAULT, "the theme combo box of WinUI sets the system theme back");

    step("Probe peer + log children");
    let probed = window.describe_peer();
    report(probed.contains("childCount=1"), &format!("the probe of the peer: {probed}"));
    step("Probe deep (walk full tree)");
    let walked = window.deep_walk();
    let deep = after(&walked, "Deep walk:");
    report(deep.contains("[FerroSwapChainPanel]") && deep.contains("[EmbeddableControlRoot]"), "the deep walk reaches the peers of the content");
    info(&format!("the deep walk: {} lines", deep.lines().count()));

    step("the slider of the embedded view: Value = 40");
    let view = App::lifetime().main_view().and_then(|view| view.cast::<EmbeddedView>());
    match view {
        Some(view) => {
            view.av_slider().set_range_value(40.0);
            let text = view.av_slider_value().text().unwrap_or_default();
            report(text == "Slider: 40", &format!("the slider of the embedded view shows its value: {text:?}"));
        }
        None => report(false, "the main view is the embedded view"),
    }
}

/// Closes the window; `App` drops it and ends the application when it is closed.
fn finish(window: &MainWindow) {
    watch_the_end();
    step("Window.Close");
    let close = window.window().close();
    report(close.is_ok(), "the window closes");
    if close.is_err() {
        XamlApplication.exit();
    }
}
