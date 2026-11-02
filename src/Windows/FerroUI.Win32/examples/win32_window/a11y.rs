//! The accessibility phase of the example (`--a11y`): a window of controls
//! (a button, a button that is not enabled, a text box, a check box and a
//! slider), and a client of UI Automation on a second thread with its own
//! COM apartment that reads and drives them through the client interface of
//! the system, as a screen reader or a test tool does:
//!
//! - the element of the window from its handle, and the tree below it
//!   through the walker of the raw view, printed with control types, names,
//!   states and bounding rectangles;
//! - the invoke pattern of the button raises its click, the toggle pattern
//!   toggles the check box, the value pattern sets the text of the text
//!   box, the range value pattern sets the slider;
//! - a handler of focus changes hears of the slider getting the focus.
//!
//! Every check prints `[ ok ]` or `[FAIL]` with what it saw; the exit code
//! is 1 if any failed. What this cannot show is a screen reader reading the
//! window to a person.
//!
//! The window is one of the framework (an application, a top level, the
//! automation peers of the controls), unlike the window of the other phases
//! of the example, which is one of the windowing contract alone. The
//! renderer is the Vello backend in its CPU mode, as for those. The
//! controls have no theme: the window has the template of a top level of
//! the test theme of the controls, so that its content is in its tree, and
//! the controls below it have their sizes set and their names given as
//! automation names (a control without a template has no presenter to take
//! a name from). Nothing of them is drawn, and nothing of them needs to be:
//! the run reads what the peers say.

use ferroui_base::media::text_formatting::testing::TestTextShaperImpl;
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::top_level_template;
use ferroui_controls::{AppBuilder, Application, Button, CheckBox, Control, Slider, StackPanel, TextBox, Window};
use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};
use ferroui_win32::{Win32ApplicationExtensions, Win32PlatformOptions, Win32RenderingMode};
use std::process::ExitCode;
use ferroui_win32::automation::testing::{
    control_type_name, count_by_control_type, foreground_window, uninitialize_com, Element, ElementInfo, FocusLog, UiaClient,
    INVOKE_PATTERN, RANGE_VALUE_PATTERN, TOGGLE_PATTERN, VALUE_PATTERN,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const TITLE: &str = "FerroUI accessibility smoke";
const BUTTON: &str = "Invoke me";
const DISABLED_BUTTON: &str = "Not enabled";
const CHECK_BOX: &str = "Check me";
const TEXT: &str = "Hello";
const TYPED: &str = "typed by a client";

// The control types of the system the run looks for.
const BUTTON_TYPE: i32 = 50000;
const CHECK_BOX_TYPE: i32 = 50002;
const EDIT_TYPE: i32 = 50004;
const SLIDER_TYPE: i32 = 50015;
const WINDOW_TYPE: i32 = 50032;

/// The phase: brings the application up on the Windows backend with the
/// Vello backend, shows the window, and runs the message loop until the
/// client is done.
pub fn run() -> ExitCode {
    let builder = AppBuilder::configure::<Application>()
        .with(Rc::new(Win32PlatformOptions {
            rendering_mode: vec![Win32RenderingMode::Software],
            ..Win32PlatformOptions::default()
        }))
        .with(Rc::new(VelloOptions::with_rendering_mode(VelloRenderingMode::Cpu)))
        .use_win32()
        .use_vello()
        // An application needs a text shaper to be set up. The one of the unit tests of the
        // base library has no C or C++ sources (the example is checked for a Windows target on
        // any host); nothing asks it to shape: the controls of the window have no templates,
        // so no text is laid out.
        .use_text_shaping_subsystem(
            || {
                let shaper: Rc<dyn ITextShaperImpl> = Rc::new(TestTextShaperImpl::new());
                FerroLocator::current_mutable().bind::<dyn ITextShaperImpl>().to_constant(shaper);
            },
            "the text shaper of the unit tests",
        )
        .setup_without_starting();
    let _application = builder.instance();

    let done = CancellationTokenSource::new();
    DONE.with(|slot| *slot.borrow_mut() = Some(done.clone()));
    let window = create_main_window();
    window.show();
    Dispatcher::ui_thread().main_loop(&done.token());

    let failed = FAILED.with(|failed| failed.get());
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The controls of the window, read on the UI thread when the client is
/// done.
struct Ui {
    window: Ref<Window>,
    text_box: Ref<TextBox>,
    check_box: Ref<CheckBox>,
    slider: Ref<Slider>,
    clicks: Arc<AtomicU32>,
}

ferroui_base::ferro_thread_local! {
    static UI: RefCell<Option<Ui>> = const { RefCell::new(None) };
    /// Ends the message loop of the phase.
    static DONE: RefCell<Option<CancellationTokenSource>> = const { RefCell::new(None) };
    /// The number of checks that failed; one before the checks ran.
    static FAILED: Cell<usize> = const { Cell::new(1) };
}

/// The lines of the checks: whether each passed, and what it saw.
#[derive(Default)]
struct Report {
    lines: Vec<(bool, String)>,
    /// Lines that are no checks.
    infos: Vec<String>,
}

impl Report {
    /// A check: printed when it is made, so that a run that does not end shows how far it
    /// got, and kept for the count.
    fn check(&mut self, ok: bool, text: impl Into<String>) -> bool {
        let text = text.into();
        println!("{} {text}", if ok { "[ ok ]" } else { "[FAIL]" });
        self.lines.push((ok, text));
        ok
    }

    fn note(&mut self, text: impl Into<String>) {
        println!("  {}", text.into());
    }
}

fn create_main_window() -> Ref<Window> {
    let clicks = Arc::new(AtomicU32::new(0));

    let button = Button::new();
    button.set_content(Some(Rc::new(BUTTON.to_string())));
    AutomationProperties::set_name(&button, Some(BUTTON));
    button.set_width(200.0);
    button.set_height(32.0);
    let counter = clicks.clone();
    button.click(move |_, _| {
        counter.fetch_add(1, Ordering::SeqCst);
    });

    let disabled = Button::new();
    disabled.set_content(Some(Rc::new(DISABLED_BUTTON.to_string())));
    AutomationProperties::set_name(&disabled, Some(DISABLED_BUTTON));
    disabled.set_width(200.0);
    disabled.set_height(32.0);
    disabled.set_is_enabled(false);

    let text_box = TextBox::new();
    text_box.set_text(Some(TEXT));
    // A text box takes the focus through its presenter, a part of its template, which this one
    // does not have. The client of its input method failed without one (run 38098620662) until
    // its text view visual became optional, as upstream's is nullable; the text box stays not
    // focusable, which is then what the client has to read of it, so that the focus of the run,
    // as measured on the runners, is given to the slider.
    text_box.set_focusable(false);
    text_box.set_width(200.0);
    text_box.set_height(32.0);

    let check_box = CheckBox::new();
    check_box.set_content(Some(Rc::new(CHECK_BOX.to_string())));
    AutomationProperties::set_name(&check_box, Some(CHECK_BOX));
    check_box.set_width(200.0);
    check_box.set_height(32.0);

    let slider = Slider::new();
    slider.set_minimum(0.0);
    slider.set_maximum(100.0);
    slider.set_range_value(25.0);
    slider.set_width(200.0);
    slider.set_height(32.0);

    let panel = StackPanel::new();
    panel.set_spacing(8.0);
    panel.children().add(&button);
    panel.children().add(&disabled);
    panel.children().add(&text_box);
    panel.children().add(&check_box);
    panel.children().add(&slider);

    let window = Window::new();
    window.set_template(Some(top_level_template()));
    window.set_title(Some(TITLE.to_string()));
    window.set_width(480.0);
    window.set_height(360.0);
    window.set_content(Some(Control::boxed(&panel)));

    UI.with(|ui| {
        *ui.borrow_mut() =
            Some(Ui { window: window.clone(), text_box, check_box, slider, clicks: clicks.clone() });
    });

    let shown = window.downgrade();
    window.opened(move || {
        println!("-- accessibility: the window is open");
        // A window peer follows the focus of the keyboard device while its window is active.
        if let Some(shown) = shown.upgrade() {
            shown.activate();
        }
        // The client starts once the window was laid out and shown.
        let clicks = clicks.clone();
        let _timer = DispatcherTimer::run_once(
            move || start_client(clicks.clone()),
            Duration::from_millis(500),
            DispatcherPriority::NORMAL,
        )
        .detach();
    });
    window.closed(|| println!("-- accessibility: the window is closed"));
    window
}

fn start_client(clicks: Arc<AtomicU32>) {
    let handle = UI.with(|ui| {
        ui.borrow().as_ref().and_then(|ui| ui.window.try_get_platform_handle()).map_or(0, |handle| handle.handle())
    });
    println!("-- accessibility: a client of UI Automation on a second thread reads the window {handle:#x}");
    let dispatcher = Dispatcher::ui_thread();
    let spawned = std::thread::Builder::new().name("UI Automation client".to_owned()).spawn(move || {
        let mut report = Report::default();
        if let Err(error) = client(handle, &clicks, &mut report) {
            report.check(false, format!("the client ended early: {error}"));
        }
        uninitialize_com();
        dispatcher.post(move || finish(report), DispatcherPriority::NORMAL);
    });
    if let Err(error) = spawned {
        println!("[FAIL] the thread of the client did not start: {error}");
        end();
    }
}

/// Ends the message loop of the phase.
fn end() {
    if let Some(done) = DONE.with(|slot| slot.borrow_mut().take()) {
        done.cancel();
    }
}

fn describe(info: &ElementInfo) -> String {
    let [left, top, right, bottom] = info.bounding_rectangle;
    format!(
        "{}{} {:?} class {:?} enabled {} focusable {} at ({left},{top})-({right},{bottom})",
        "  ".repeat(info.depth),
        info.control_type_name(),
        info.name,
        info.class_name,
        info.is_enabled,
        info.is_keyboard_focusable
    )
}

fn inside(inner: [i32; 4], outer: [i32; 4]) -> bool {
    inner[2] > inner[0] && inner[3] > inner[1] && inner[0] >= outer[0] && inner[1] >= outer[1] && inner[2] <= outer[2] && inner[3] <= outer[3]
}

/// Waits until `done` answers true, for at most `timeout`.
fn wait_until(timeout: Duration, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// What the client does, on its thread.
fn client(handle: isize, clicks: &AtomicU32, report: &mut Report) -> Result<(), String> {
    let mut client = UiaClient::new()?;
    report.check(true, "CUIAutomation was created in a multithreaded apartment of the second thread");

    let root = client.element_from_handle(handle)?;
    let root_info = root.info(0)?;
    report.note(describe(&root_info));
    report.check(
        root_info.control_type == WINDOW_TYPE && root_info.name == TITLE,
        format!("the element of the window handle is {} {:?}", root_info.control_type_name(), root_info.name),
    );
    report.check(
        root_info.framework_id == "FerroUI",
        format!("the framework of the window is {:?}", root_info.framework_id),
    );

    let elements = client.descendants(&root)?;
    let mut infos = Vec::with_capacity(elements.len());
    for (depth, element) in &elements {
        let info = element.info(*depth)?;
        report.note(describe(&info));
        infos.push(info);
    }
    let counts = count_by_control_type(&infos);
    report.check(
        !infos.is_empty(),
        format!(
            "the walker of the raw view finds {} elements below the window: {}",
            infos.len(),
            counts.iter().map(|(name, count)| format!("{count} {name}")).collect::<Vec<_>>().join(", ")
        ),
    );

    let find = |control_type: i32, name: Option<&str>| -> Result<(&Element, &ElementInfo), String> {
        infos
            .iter()
            .position(|info| {
                info.control_type == control_type
                    && info.framework_id == "FerroUI"
                    && name.is_none_or(|name| info.name == name)
            })
            .map(|index| (&elements[index].1, &infos[index]))
            .ok_or_else(|| format!("the tree has no {} {}", control_type_name(control_type), name.unwrap_or("")))
    };

    let (button, button_info) = find(BUTTON_TYPE, Some(BUTTON))?;
    report.check(
        button_info.is_enabled && button_info.is_keyboard_focusable && inside(button_info.bounding_rectangle, root_info.bounding_rectangle),
        format!(
            "the button {BUTTON:?} is enabled ({}), focusable ({}) and lies inside the window: {:?} in {:?}",
            button_info.is_enabled, button_info.is_keyboard_focusable, button_info.bounding_rectangle, root_info.bounding_rectangle
        ),
    );
    let (_, disabled_info) = find(BUTTON_TYPE, Some(DISABLED_BUTTON))?;
    report.check(!disabled_info.is_enabled, format!("the button {DISABLED_BUTTON:?} is enabled: {}", disabled_info.is_enabled));
    let (edit, edit_info) = find(EDIT_TYPE, None)?;
    let (check_box, check_box_info) = find(CHECK_BOX_TYPE, Some(CHECK_BOX))?;
    let (slider, slider_info) = find(SLIDER_TYPE, None)?;
    for (what, info, focusable) in [("text box", edit_info, false), ("check box", check_box_info, true), ("slider", slider_info, true)] {
        report.check(
            info.is_enabled && info.is_keyboard_focusable == focusable && inside(info.bounding_rectangle, root_info.bounding_rectangle),
            format!(
                "the {what} is a {} that is enabled ({}), focusable ({}; made {focusable}) and inside the window: {:?}",
                info.control_type_name(),
                info.is_enabled,
                info.is_keyboard_focusable,
                info.bounding_rectangle
            ),
        );
    }

    // The patterns each element has, and has not.
    let patterns = |element: &Element| -> Result<[bool; 4], String> {
        Ok([
            element.has_pattern(INVOKE_PATTERN)?,
            element.has_pattern(TOGGLE_PATTERN)?,
            element.has_pattern(VALUE_PATTERN)?,
            element.has_pattern(RANGE_VALUE_PATTERN)?,
        ])
    };
    let (of_button, of_check_box, of_edit, of_slider) = (patterns(button)?, patterns(check_box)?, patterns(edit)?, patterns(slider)?);
    report.check(
        of_button == [true, false, false, false]
            && of_check_box == [false, true, false, false]
            && of_edit == [false, false, true, false]
            && of_slider[3]
            && !of_slider[0],
        format!(
            "the patterns (invoke, toggle, value, range value): button {of_button:?}, check box {of_check_box:?}, text box {of_edit:?}, slider {of_slider:?}"
        ),
    );

    // Invoke.
    button.invoke()?;
    let clicked = wait_until(Duration::from_secs(5), || clicks.load(Ordering::SeqCst) == 1);
    report.check(clicked, format!("IUIAutomationInvokePattern::Invoke raised the click of the button: {} click(s)", clicks.load(Ordering::SeqCst)));

    // Toggle.
    let before = check_box.toggle_state()?;
    check_box.toggle()?;
    let after = check_box.toggle_state()?;
    report.check(before == 0 && after == 1, format!("IUIAutomationTogglePattern::Toggle took the check box from state {before} to {after}"));

    // Value.
    let before = edit.value()?;
    edit.set_value(TYPED)?;
    let after = edit.value()?;
    report.check(before == TEXT && after == TYPED, format!("IUIAutomationValuePattern::SetValue took the text box from {before:?} to {after:?}"));

    // Range value.
    let before = slider.range_value()?;
    slider.set_range_value(60.0)?;
    let after = slider.range_value()?;
    report.check(before == 25.0 && after == 60.0, format!("IUIAutomationRangeValuePattern::SetValue took the slider from {before} to {after}"));

    // The focus.
    let log = FocusLog::new();
    let mut events: Vec<String> = Vec::new();
    client.add_focus_changed_event_handler(log.clone())?;
    // The slider may have the focus already: setting a value through a pattern gave it to the
    // element in run 38100265175 (the focused element of the system was the slider before
    // this point, and no event followed a focus that did not change). The focus goes to the
    // check box first, so that the slider getting it is a change.
    if let Err(error) = check_box.set_focus() {
        report.check(false, format!("IUIAutomationElement::SetFocus on the check box: {error}"));
    }
    std::thread::sleep(Duration::from_millis(300));
    if let Err(error) = slider.set_focus() {
        report.check(false, format!("IUIAutomationElement::SetFocus on the slider: {error}"));
    }
    // The events of the focus a client hears are those of the desktop. Where another window has
    // the foreground (a runner of the CI service shows a dialog of the system over everything),
    // whether the event of the slider arrives is not this run's to say: it is checked where
    // the window of the run is the foreground window, and reported as not checked elsewhere.
    // That a node raises the event is a test of the crate.
    let foreground = foreground_window();
    let ours = foreground == handle;
    let arrived = wait_until(Duration::from_secs(if ours { 10 } else { 3 }), || {
        events.extend(client.take_focus_events(&log));
        events.iter().any(|entry| entry.starts_with("Slider"))
    });
    if ours || arrived {
        report.check(
            arrived,
            format!("a focus changed event arrived for the slider after IUIAutomationElement::SetFocus: {events:?}"),
        );
    } else {
        report.infos.push(format!(
            "the focus changed event of the slider was not checked: the foreground window of the desktop is {foreground:#x}, not the window of the run ({handle:#x}); the handler heard {events:?}"
        ));
    }
    let has_focus = slider.has_keyboard_focus()?;
    report.check(has_focus, format!("the slider has the keyboard focus: {has_focus}"));
    let focused = match client.focused_element()? {
        Some(focused) => format!("{} {:?}", control_type_name(focused.control_type()?), focused.name()?),
        None => "nothing".to_owned(),
    };
    report.note(format!("the focused element of the system: {focused}"));

    // Everything of the client goes before COM of the thread does.
    drop(elements);
    drop(root);
    drop(client);
    Ok(())
}

/// On the UI thread, when the client is done: what the controls say, the
/// lines of the run, and the end of the phase.
fn finish(mut report: Report) {
    let Some(ui) = UI.with(|ui| ui.borrow_mut().take()) else { return };
    let text = ui.text_box.text();
    report.check(text.as_deref() == Some(TYPED), format!("the text of the text box is {text:?}"));
    let checked = ui.check_box.is_checked();
    report.check(checked == Some(true), format!("the check box is checked: {checked:?}"));
    let value = ui.slider.value();
    report.check(value == 60.0, format!("the value of the slider is {value}"));
    let clicks = ui.clicks.load(Ordering::SeqCst);
    report.check(clicks == 1, format!("the button was clicked {clicks} time(s)"));

    let failed = report.lines.iter().filter(|(ok, _)| !ok).count();
    for text in &report.infos {
        println!("[info] {text}");
    }
    println!("accessibility: {} checks, {failed} failed", report.lines.len());
    FAILED.with(|count| count.set(failed));
    ui.window.close();
    end();
}
