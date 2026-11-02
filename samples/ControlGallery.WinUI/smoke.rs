//! The smoke run of the sample (`--smoke`): what can be checked of the
//! integration without a person in front of the window. The checks are
//! listed in docs/porting/winui.md, section 7.2; each prints a line, and the
//! process exits with 0 only if every check passed.

use crate::app::App;
use crate::main_window::MainWindow;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime;
use ferroui_microcom::{ComPtr, HResult};
use ferroui_win32::win_rt::{HStringInterop, NativeWinRTMethods};
use ferroui_winui::interop::{AutomationControlType, IAutomationPeer, IFrameworkElementAutomationPeerStatics};
use ferroui_winui::XamlApplication;
use std::cell::Cell;
use std::io::Write;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

ferroui_base::ferro_thread_local! {
    static FAILED: Cell<bool> = const { Cell::new(false) };
}

/// Whether this is a smoke run (`--smoke`).
static SMOKE: AtomicBool = AtomicBool::new(false);
/// The last `[step]` line, which the watchdog of the end of the process
/// names.
static LAST_STEP: Mutex<String> = Mutex::new(String::new());
/// How long the process has to end once the window is closed.
const END_TIMEOUT: Duration = Duration::from_secs(60);

/// How long the panel has to load and to get its swap chain.
const LOAD_TIMEOUT: Duration = Duration::from_secs(60);
/// How many frames the content has to render after the panel has loaded.
const FRAMES: u32 = 30;

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

fn take_string(handle: isize) -> String {
    HStringInterop::from_handle(handle, true).value().unwrap_or_default()
}

/// Starts the checks once the main window is activated; the run closes the
/// window and ends the application when they are done. The timer holds the
/// window weakly: `App` drops it when it is closed.
pub fn start(window: Rc<MainWindow>) {
    report(App::lifetime().main_view().is_some(), "the catalog set the main view of the single view lifetime");
    report(true, "the main window with the swap chain panel is created and activated");

    let window: Weak<MainWindow> = Rc::downgrade(&window);
    let started = Instant::now();
    let frames = Rc::new(Cell::new(0u32));
    let counting = Rc::new(Cell::new(false));
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
                check_automation(&window);
                finished.set(true);
                finish(&window);
                return false;
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

/// Marks the run as a smoke run: the steps of the end of the application
/// are printed too.
pub fn set_smoke() {
    SMOKE.store(true, Ordering::SeqCst);
}

/// A step of the end of the application (the window closed, the loop ended,
/// the Windows App SDK released, `main` returning), printed in a smoke run.
pub fn end_step(what: &str) {
    if SMOKE.load(Ordering::SeqCst) {
        step(what);
    }
}

/// From the close of the window on, a thread watches that the process ends:
/// if it still runs after [`END_TIMEOUT`], the run fails with the last step
/// it reached, and the process is ended. A process that stayed after the
/// smoke checks had passed held the step of CI until the limit of the step
/// (runs 38741696373 and 38769434562); this names where. It cannot see a
/// process that hangs inside `ExitProcess` (the system ends this thread
/// first): the step of CI ends such a process itself and says so.
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

/// A line before a call of the system, so that a run that ends in the
/// system's frames names the call it was in.
fn step(what: &str) {
    if let Ok(mut last) = LAST_STEP.lock() {
        last.clear();
        last.push_str(what);
    }
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "[step] {what}");
    let _ = out.flush();
}

/// The automation tree XAML builds of the panel: the peer the panel creates
/// (`OnCreateAutomationPeer`, overridden by the composed panel), its class
/// name and control type (overrides of the peer), and the peer of the
/// content beneath it (the proxy of the embedded root, with the content's
/// peers as its children). Each call of the system is announced by a
/// `[step]` line; the automation objects of the integration add `[trace]`
/// lines of what XAML asks of them (`FERROUI_WINUI_TRACE`, which `main` sets
/// for a smoke run).
fn check_automation(window: &MainWindow) {
    let result = (|| -> Result<(), HResult> {
        step("RoGetActivationFactory(FrameworkElementAutomationPeer)");
        let statics = NativeWinRTMethods::create_activation_factory::<IFrameworkElementAutomationPeerStatics>(
            "Microsoft.UI.Xaml.Automation.Peers.FrameworkElementAutomationPeer",
        )?;
        let element = window.ferro_panel().as_ui_element();
        step("FrameworkElementAutomationPeer.CreatePeerForElement(the panel)");
        let peer: ComPtr<IAutomationPeer> = statics.create_peer_for_element(Some(&element))?.ok_or(HResult::POINTER)?;
        step("the peer of the panel: GetClassName");
        let class_name = take_string(peer.get_class_name()?);
        report(class_name == "FerroSwapChainPanel", &format!("the automation peer of the panel has the class name {class_name:?}"));
        step("the peer of the panel: GetAutomationControlType");
        let control_type = peer.get_automation_control_type()?;
        report(control_type == AutomationControlType::Pane, &format!("the automation peer of the panel is a {control_type:?}"));

        step("the peer of the panel: GetChildren");
        let children = peer.get_children()?.ok_or(HResult::POINTER)?;
        step("the children of the panel: Size");
        let count = children.size()?;
        report(count >= 1, &format!("the automation peer of the panel has {count} children"));
        if count >= 1 {
            step("the children of the panel: GetAt(the last)");
            let root = children.get_at(count - 1)?.ok_or(HResult::POINTER)?;
            step("the peer of the content: GetClassName");
            let root_class = take_string(root.get_class_name()?);
            step("the peer of the content: GetChildren");
            let root_children = root.get_children()?.ok_or(HResult::POINTER)?;
            step("the children of the content: Size");
            let root_count = root_children.size()?;
            report(root_count >= 1, &format!("the peer of the content ({root_class:?}) has {root_count} children"));
            step("the peer of the content: IsControlElement");
            info(&format!("the peer of the content is a control element: {}", root.is_control_element()? != 0));
            if root_count >= 1 {
                step("the children of the content: GetAt(0)");
                let first = root_children.get_at(0)?.ok_or(HResult::POINTER)?;
                step("the first child of the content: GetClassName");
                info(&format!("the first child of the content is a {:?}", take_string(first.get_class_name()?)));
            }
            step("releasing the peers of the content");
        }
        step("releasing the peer of the panel");
        Ok(())
    })();
    step("the automation check returned");
    if let Err(error) = result {
        report(false, &format!("the automation tree of the panel: {error}"));
    }
}

/// Closes the window; `App` drops it and ends the application when it is
/// closed.
fn finish(window: &MainWindow) {
    watch_the_end();
    step("Window.Close");
    let close = window.window().close();
    report(close.is_ok(), "the window closes");
    if close.is_err() {
        XamlApplication.exit();
    }
}
