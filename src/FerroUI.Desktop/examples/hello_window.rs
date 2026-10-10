//! The smallest desktop application: an application class whose main window
//! holds a border with a text block and a button, started through the
//! application builder with the classic desktop lifetime.
//!
//! ```text
//! cargo run -p ferroui-desktop --example hello_window
//! FERROUI_SMOKE_EXIT_MS=2000 cargo run -p ferroui-desktop --example hello_window
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n`
//! milliseconds, which ends the main loop; the process exits with the exit
//! code the lifetime returns.
//!
//! For runs without a person on Windows: `FERROUI_SMOKE_RENDERING=software`
//! or `angle` asks for one rendering mode without a fallback,
//! `FERROUI_SMOKE_RENDER_ON_UI_THREAD=1` renders on the UI thread,
//! `FERROUI_SMOKE_LOG=1` writes the warnings and errors the framework logs
//! to the error stream, and a fatal exception of the system is reported
//! with its code before the process ends.
//!
//! The window content is painted by the compositing renderer through the
//! compositor the platform registers.

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::Color;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, CornerRadius, FerroObjectImpl,
    Ref, Thickness,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Button, Control, NativeMenu, NativeMenuItem,
    NewApplication, StackPanel, TextBlock, TrayIcon, TrayIcons, Window,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self { base: Application::construct() })
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_name(Some("FerroUI hello window".to_string()));
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) =
            lifetime.as_ref().and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            let window = create_main_window();
            if std::env::var_os("FERROUI_SMOKE_MENU").is_some() {
                smoke_menu(&window);
            }
            desktop.set_main_window(Some(window));
        }

        if std::env::var_os("FERROUI_SMOKE_TRAY").is_some() {
            smoke_tray(this);
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

fn create_main_window() -> Ref<Window> {
    let text = TextBlock::new();
    text.set_text(Some("Hello from FerroUI"));
    text.set_font_size(24.0);

    let button = Button::new();
    button.set_content(Some(Rc::new("Click me".to_string())));
    button.click(|_, _| println!("Button clicked"));

    let panel = StackPanel::new();
    panel.set_spacing(12.0);
    panel.set_horizontal_alignment(HorizontalAlignment::Center);
    panel.set_vertical_alignment(VerticalAlignment::Center);
    panel.children().add(text);
    panel.children().add(button);

    let border = Border::new();
    border.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(0xee, 0xf2, 0xff)))));
    border.set_corner_radius(CornerRadius::uniform(12.0));
    border.set_padding(Thickness::uniform(24.0));
    border.set_margin(Thickness::uniform(16.0));
    border.set_child(panel);

    let window = Window::new();
    window.set_title(Some("FerroUI hello window".to_string()));
    window.set_width(640.0);
    window.set_height(400.0);
    window.set_content(Some(Control::boxed(border)));

    window.opened(|| println!("Window opened"));
    window.closed(|| println!("Window closed"));

    if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|v| v.parse::<u64>().ok()) {
        println!("Will close the main window after {ms} ms");
        let window = window.clone();
        // The timer stops itself after its only tick.
        let _timer = DispatcherTimer::run_once(
            move || {
                println!("Timer fired: closing the main window");
                window.close();
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }

    window
}

fn logging_item(header: &str) -> Ref<NativeMenuItem> {
    let item = NativeMenuItem::with_header(header);
    let name = header.to_string();
    // The subscription lives as long as the item.
    let _ = item.click(move |_| println!("Menu item clicked: {name}"));
    item
}

/// Gives the window a native menu and, once it is up, inspects and drives
/// the system's main menu.
fn smoke_menu(window: &Ref<Window>) {
    let menu = NativeMenu::new();
    menu.add(logging_item("Smoke _A"));
    menu.add(logging_item("Smoke B"));
    let more = NativeMenuItem::with_header("More");
    let sub_menu = NativeMenu::new();
    sub_menu.add(logging_item("Nested"));
    more.set_menu(Some(sub_menu));
    menu.add(more);
    NativeMenu::set_menu(window, Some(menu));
    println!("Native menu set on the window");

    let _timer = DispatcherTimer::run_once(
        || {
            #[cfg(target_os = "macos")]
            main_menu_probe::dump_and_click(&["Smoke A", "Smoke B"], ("More", "Nested"));
        },
        Duration::from_millis(800),
        DispatcherPriority::NORMAL,
    );
}

/// Creates a tray icon with a menu and disposes it a little later.
fn smoke_tray(application: &Application) {
    let menu = NativeMenu::new();
    menu.add(logging_item("Tray item"));

    let icon = TrayIcon::new();
    icon.set_tool_tip_text(Some("FerroUI hello window".to_string()));
    icon.set_menu(Some(menu));
    let _ = icon.clicked(|_| println!("Tray icon clicked"));

    let icons = TrayIcons::new();
    icons.add(icon.clone());
    TrayIcon::set_icons(application, Some(icons));
    println!("Tray icon created (native menu exporter: {})", icon.native_menu_exporter().is_some());

    // A second item added after the export goes through the queued reset.
    if let Some(menu) = icon.menu() {
        menu.add(logging_item("Tray item added later"));
    }

    let _timer = DispatcherTimer::run_once(
        move || {
            icon.dispose();
            println!("Tray icon disposed");
        },
        Duration::from_millis(1200),
        DispatcherPriority::NORMAL,
    );
}

/// Reads the main menu AppKit holds for this application and performs item
/// actions, all in-process. The native interface of the backend has no
/// query for this, so the probe talks to AppKit directly.
#[cfg(target_os = "macos")]
mod main_menu_probe {
    use std::ffi::{c_char, c_void, CStr};

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_getClass(name: *const c_char) -> Id;
        fn objc_msgSend();
    }

    fn sel(name: &CStr) -> Sel {
        // SAFETY: `name` is NUL-terminated.
        unsafe { sel_registerName(name.as_ptr()) }
    }

    // SAFETY (all `send_*`): `objc_msgSend` is called through a pointer
    // with the C signature of the method being sent; receivers are live
    // objects or null (messages to nil return zero).
    unsafe fn send_id(receiver: Id, selector: Sel) -> Id {
        let f: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, selector)
    }

    unsafe fn send_isize(receiver: Id, selector: Sel) -> isize {
        let f: unsafe extern "C" fn(Id, Sel) -> isize = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, selector)
    }

    unsafe fn send_bool(receiver: Id, selector: Sel) -> bool {
        let f: unsafe extern "C" fn(Id, Sel) -> bool = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, selector)
    }

    unsafe fn send_index(receiver: Id, selector: Sel, index: isize) -> Id {
        let f: unsafe extern "C" fn(Id, Sel, isize) -> Id =
            std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        f(receiver, selector, index)
    }

    unsafe fn title(item: Id) -> String {
        if send_bool(item, sel(c"isSeparatorItem")) {
            return "-".to_string();
        }
        let ns_string = send_id(item, sel(c"title"));
        let utf8 = send_id(ns_string, sel(c"UTF8String")) as *const c_char;
        if utf8.is_null() {
            String::new()
        } else {
            CStr::from_ptr(utf8).to_string_lossy().into_owned()
        }
    }

    unsafe fn items(menu: Id) -> Vec<(String, Id)> {
        let count = send_isize(menu, sel(c"numberOfItems"));
        (0..count)
            .map(|index| {
                let item = send_index(menu, sel(c"itemAtIndex:"), index);
                (title(item), item)
            })
            .collect()
    }

    unsafe fn perform(menu: Id, wanted: &str) {
        match items(menu).iter().position(|(title, _)| title == wanted) {
            Some(index) => {
                println!("Performing the action of {wanted:?}");
                send_index(menu, sel(c"performActionForItemAtIndex:"), index as isize);
            }
            None => println!("Main menu has no item {wanted:?}"),
        }
    }

    /// Prints the menu bar and its submenus, then performs the actions of
    /// the named menu bar items and of one nested item.
    pub fn dump_and_click(top_level: &[&str], nested: (&str, &str)) {
        // SAFETY: plain AppKit calls on the main thread (see `send_*`).
        unsafe {
            let app = send_id(objc_getClass(c"NSApplication".as_ptr()), sel(c"sharedApplication"));
            let main_menu = send_id(app, sel(c"mainMenu"));
            if main_menu.is_null() {
                println!("Main menu: none");
                return;
            }

            let menu_bar = items(main_menu);
            for (index, (title, item)) in menu_bar.iter().enumerate() {
                let sub_menu = send_id(*item, sel(c"submenu"));
                let children: Vec<String> = items(sub_menu).into_iter().map(|(title, _)| title).collect();
                println!("Main menu [{index}] {title:?} -> {children:?}");
            }

            for wanted in top_level {
                perform(main_menu, wanted);
            }

            if let Some((_, parent)) = menu_bar.iter().find(|(title, _)| title == nested.0) {
                perform(send_id(*parent, sel(c"submenu")), nested.1);
            } else {
                println!("Main menu has no item {:?}", nested.0);
            }
        }
    }
}

/// On Windows, a fatal exception of the system (an access violation in a
/// library, for one) ends the process without a word, where a panic says
/// what failed. This prints the code and the address of such an exception
/// before the system goes on to end the process, so a run without a person
/// leaves something to read.
#[cfg(all(windows, not(target_arch = "x86")))]
mod fatal_exceptions {
    use std::ffi::c_void;

    #[repr(C)]
    struct ExceptionRecord {
        code: u32,
        flags: u32,
        record: *mut ExceptionRecord,
        address: *mut c_void,
    }

    #[repr(C)]
    struct ExceptionPointers {
        record: *mut ExceptionRecord,
        context: *mut c_void,
    }

    #[link(name = "kernel32", kind = "raw-dylib")]
    extern "system" {
        fn AddVectoredExceptionHandler(
            first: u32,
            handler: unsafe extern "system" fn(info: *mut ExceptionPointers) -> i32,
        ) -> *mut c_void;
    }

    /// `EXCEPTION_CONTINUE_SEARCH`: the exception goes on to the handlers
    /// it would have reached without this one.
    const CONTINUE_SEARCH: i32 = 0;

    unsafe extern "system" fn handler(info: *mut ExceptionPointers) -> i32 {
        // SAFETY: the system passes the record of the exception that is
        // being dispatched, valid for the length of this call.
        let (code, address) = unsafe {
            let record = (*info).record;
            ((*record).code, (*record).address)
        };
        // The errors of the system (the severity bits 11); the exceptions a
        // language runtime throws and catches, and debug output, are not.
        if code >= 0xC000_0000 {
            let thread = std::thread::current();
            eprintln!(
                "Fatal exception {code:#010x} at {address:p} on thread {:?} ({:?})",
                thread.name().unwrap_or("unnamed"),
                thread.id()
            );
        }
        CONTINUE_SEARCH
    }

    pub fn install() {
        // SAFETY: the handler is a function of this program with the
        // signature the system calls, and it stays for the life of the
        // process.
        unsafe {
            AddVectoredExceptionHandler(1, handler);
        }
    }
}

/// The options of a smoke run on Windows, from the environment:
/// `FERROUI_SMOKE_RENDERING=software|angle` asks for that rendering mode
/// alone (no fallback, so a mode that does not work fails the run), and
/// `FERROUI_SMOKE_RENDER_ON_UI_THREAD=1` renders on the UI thread.
#[cfg(windows)]
fn smoke_platform_options(builder: AppBuilder) -> AppBuilder {
    use ferroui_win32::{Win32PlatformOptions, Win32RenderingMode};

    let rendering = std::env::var("FERROUI_SMOKE_RENDERING").ok();
    let on_ui_thread = std::env::var_os("FERROUI_SMOKE_RENDER_ON_UI_THREAD").is_some();
    if rendering.is_none() && !on_ui_thread {
        return builder;
    }
    let mut options = Win32PlatformOptions { should_render_on_ui_thread: on_ui_thread, ..Default::default() };
    match rendering.as_deref() {
        Some("software") => options.rendering_mode = vec![Win32RenderingMode::Software],
        Some("angle") => {
            options.rendering_mode = vec![Win32RenderingMode::AngleEgl];
            options.composition_mode = vec![smoke_composition_mode()];
        }
        Some(other) => println!("FERROUI_SMOKE_RENDERING: unknown mode {other:?} (software, angle); the default order is used"),
        None => {}
    }
    println!(
        "Rendering modes: {:?}; composition modes: {:?}; render on the UI thread: {on_ui_thread}",
        options.rendering_mode, options.composition_mode
    );
    builder.with(Rc::new(options))
}

/// The composition mode of a smoke run through ANGLE, without a fallback:
/// `FERROUI_SMOKE_COMPOSITION=redirection|dcomp`; the redirection surface
/// of the window when the variable is not set.
#[cfg(windows)]
fn smoke_composition_mode() -> ferroui_win32::Win32CompositionMode {
    use ferroui_win32::Win32CompositionMode;

    match std::env::var("FERROUI_SMOKE_COMPOSITION").ok().as_deref() {
        None | Some("redirection") => Win32CompositionMode::RedirectionSurface,
        Some("dcomp") => Win32CompositionMode::DirectComposition,
        Some(other) => panic!("FERROUI_SMOKE_COMPOSITION: unknown mode {other:?} (redirection, dcomp)"),
    }
}

#[cfg(not(windows))]
fn smoke_platform_options(builder: AppBuilder) -> AppBuilder {
    builder
}

fn main() -> std::process::ExitCode {
    #[cfg(all(windows, not(target_arch = "x86")))]
    fatal_exceptions::install();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut builder = smoke_platform_options(AppBuilder::configure::<App>());
    // `FERROUI_SMOKE_LOG=1`: what the framework logs at the level of
    // warnings and above, to the error stream.
    if std::env::var_os("FERROUI_SMOKE_LOG").is_some() {
        builder = builder.log_to_text_writer(std::io::stderr(), ferroui_base::logging::LogEventLevel::Warning, &[]);
    }
    let exit_code = builder.use_platform_detect().start_with_classic_desktop_lifetime(&args);
    println!("start_with_classic_desktop_lifetime returned {exit_code}");
    std::process::ExitCode::from(exit_code as u8)
}
