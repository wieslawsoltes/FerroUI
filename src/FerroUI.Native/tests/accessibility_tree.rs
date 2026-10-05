//! Not from upstream: walks the accessibility tree of a real window the way
//! an assistive application sees it, through the accessibility protocol of
//! AppKit, and drives controls through it.
//!
//! The window holds a stack panel with a button and a check box. The test
//! reads the identifier of the window, finds both controls among the
//! accessibility children of the view of the window, checks their roles,
//! titles and identifiers and that their parent chain leads back to the
//! view, presses the button and toggles the check box.
//!
//! AppKit only creates windows on the main thread, so this test has its own
//! `main` (`harness = false`). It needs the window server of a logged-in
//! session, which the macOS runners of the continuous integration have; it
//! needs no accessibility permission, as everything happens in-process.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("accessibility_tree: macOS only, skipped");
}

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    macos::run()
}

#[cfg(target_os = "macos")]
mod macos {
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::platform::IMacOSTopLevelPlatformHandle;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::Ref;
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use ferroui_controls::{AppBuilder, Application, Button, CheckBox, ContentControl, Control, StackPanel, Window};
    use ferroui_native::{
        FerroNativePlatformExtensions, FerroNativePlatformOptions, FerroNativeRenderingMode, MacOSTopLevelHandle,
    };
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use std::cell::Cell;
    use std::process::ExitCode;
    use std::rc::Rc;

    /// The Objective-C runtime calls the walk needs.
    mod objc {
        use std::ffi::{c_char, c_void, CStr, CString};

        pub type Id = *mut c_void;
        pub type Sel = *mut c_void;

        #[link(name = "objc")]
        extern "C" {
            fn sel_registerName(name: *const c_char) -> Sel;
            fn objc_msgSend();
            fn objc_autoreleasePoolPush() -> *mut c_void;
            fn objc_autoreleasePoolPop(pool: *mut c_void);
        }

        pub fn sel(name: &str) -> Sel {
            let name = CString::new(name).unwrap();
            // SAFETY: a NUL-terminated selector name.
            unsafe { sel_registerName(name.as_ptr()) }
        }

        // SAFETY (all `send*`): `objc_msgSend` is called through a pointer of
        // the exact signature of the method it dispatches to, on an object
        // that responds to the selector (checked by the callers).
        pub fn send_id(obj: Id, name: &str) -> Id {
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel) -> Id =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                f(obj, sel(name))
            }
        }

        pub fn send_bool(obj: Id, name: &str) -> bool {
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel) -> bool =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                f(obj, sel(name))
            }
        }

        pub fn send_usize(obj: Id, name: &str) -> usize {
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel) -> usize =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                f(obj, sel(name))
            }
        }

        pub fn send_i32(obj: Id, name: &str) -> i32 {
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel) -> i32 =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                f(obj, sel(name))
            }
        }

        pub fn object_at_index(array: Id, index: usize) -> Id {
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel, usize) -> Id =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                f(array, sel("objectAtIndex:"), index)
            }
        }

        pub fn responds_to(obj: Id, name: &str) -> bool {
            if obj.is_null() {
                return false;
            }
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel, Sel) -> bool =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                f(obj, sel("respondsToSelector:"), sel(name))
            }
        }

        /// The content of an `NSString`, `None` for nil.
        pub fn string(obj: Id) -> Option<String> {
            if obj.is_null() {
                return None;
            }
            let utf8 = send_id(obj, "UTF8String") as *const c_char;
            if utf8.is_null() {
                return None;
            }
            // SAFETY: `UTF8String` returns a NUL-terminated buffer that lives
            // at least as long as the enclosing autorelease pool.
            Some(unsafe { CStr::from_ptr(utf8) }.to_string_lossy().into_owned())
        }

        /// A string attribute of an accessibility object.
        pub fn string_attribute(obj: Id, name: &str) -> Option<String> {
            if responds_to(obj, name) {
                string(send_id(obj, name))
            } else {
                None
            }
        }

        /// The accessibility children of an object.
        pub fn children(obj: Id) -> Vec<Id> {
            if !responds_to(obj, "accessibilityChildren") {
                return Vec::new();
            }
            let array = send_id(obj, "accessibilityChildren");
            if array.is_null() {
                return Vec::new();
            }
            (0..send_usize(array, "count")).map(|i| object_at_index(array, i)).collect()
        }

        /// Runs `body` inside an autorelease pool.
        pub fn autorelease_pool<R>(body: impl FnOnce() -> R) -> R {
            // SAFETY: push and pop are paired on this thread.
            let pool = unsafe { objc_autoreleasePoolPush() };
            let result = body();
            unsafe { objc_autoreleasePoolPop(pool) };
            result
        }
    }

    use objc::Id;

    /// What the walk saw of one accessibility object.
    struct Element {
        object: Id,
        role: Option<String>,
        title: Option<String>,
        identifier: Option<String>,
    }

    fn walk(object: Id, depth: usize, out: &mut Vec<Element>) {
        if object.is_null() || depth > 16 || out.len() > 500 {
            return;
        }
        out.push(Element {
            object,
            role: objc::string_attribute(object, "accessibilityRole"),
            title: objc::string_attribute(object, "accessibilityTitle"),
            identifier: objc::string_attribute(object, "accessibilityIdentifier"),
        });
        for child in objc::children(object) {
            walk(child, depth + 1, out);
        }
    }

    fn find<'a>(elements: &'a [Element], role: &str, title: &str) -> Option<&'a Element> {
        elements.iter().find(|e| e.role.as_deref() == Some(role) && e.title.as_deref() == Some(title))
    }

    /// Whether the accessibility parent chain of `object` reaches `target`.
    fn parent_chain_reaches(mut object: Id, target: Id) -> bool {
        for _ in 0..32 {
            if !objc::responds_to(object, "accessibilityParent") {
                return false;
            }
            object = objc::send_id(object, "accessibilityParent");
            if object == target {
                return true;
            }
            if object.is_null() {
                return false;
            }
        }
        false
    }

    /// The template of a window without a theme: a content presenter.
    fn window_template() -> Rc<FuncControlTemplate> {
        FuncControlTemplate::for_type::<Window>(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".to_string()));
            let property = ContentControl::content_property().as_property();
            presenter.bind_binding(property, &TemplateBinding::new(property));
            presenter.register_in_name_scope(&**scope).upcast()
        })
    }

    struct Checks {
        failures: Cell<u32>,
    }

    impl Checks {
        fn check(&self, ok: bool, what: &str) {
            if ok {
                println!("ok: {what}");
            } else {
                println!("FAILED: {what}");
                self.failures.set(self.failures.get() + 1);
            }
        }
    }

    pub fn run() -> ExitCode {
        let options = FerroNativePlatformOptions {
            rendering_mode: vec![FerroNativeRenderingMode::Software],
            ..Default::default()
        };
        let _builder = AppBuilder::configure::<Application>()
            .use_harfbuzz()
            .use_ferro_native()
            .use_skia()
            .with(Rc::new(options))
            .setup_without_starting();

        let checks = Checks { failures: Cell::new(0) };

        let button = Button::new();
        AutomationProperties::set_name(&button, Some("OK"));
        AutomationProperties::set_automation_id(&button, Some("ok-button"));
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        button.click(move |_, _| count.set(count.get() + 1));

        let check_box = CheckBox::new();
        AutomationProperties::set_name(&check_box, Some("Remember"));

        let panel = StackPanel::new();
        panel.children().add(button.clone());
        panel.children().add(check_box.clone());

        let window: Ref<Window> = Window::new();
        window.set_template(Some(window_template()));
        AutomationProperties::set_automation_id(&window, Some("main-window"));
        window.set_width(320.0);
        window.set_height(200.0);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);

        let handle = window.try_get_platform_handle().expect("the platform handle of the window");
        let handle = handle.as_any().downcast_ref::<MacOSTopLevelHandle>().expect("a handle of this backend");
        let ns_window = handle.ns_window() as Id;
        let ns_view = handle.ns_view() as Id;
        checks.check(!ns_window.is_null() && !ns_view.is_null(), "the window has a native window and view");

        objc::autorelease_pool(|| {
            checks.check(
                objc::string_attribute(ns_window, "accessibilityIdentifier").as_deref() == Some("main-window"),
                "the native window reports the automation id of the window",
            );

            let mut elements = Vec::new();
            for child in objc::children(ns_view) {
                walk(child, 0, &mut elements);
            }
            for e in &elements {
                println!(
                    "element: role {:?}, title {:?}, identifier {:?}",
                    e.role.as_deref().unwrap_or(""),
                    e.title.as_deref().unwrap_or(""),
                    e.identifier.as_deref().unwrap_or("")
                );
            }

            let button_element = find(&elements, "AXButton", "OK");
            checks.check(button_element.is_some(), "the button is in the tree as AXButton titled OK");
            checks.check(
                button_element.is_some_and(|e| e.identifier.as_deref() == Some("ok-button")),
                "the button reports its automation id",
            );
            checks.check(
                button_element.is_some_and(|e| parent_chain_reaches(e.object, ns_view)),
                "the parent chain of the button leads to the view of the window",
            );
            if let Some(e) = button_element {
                let can_press = objc::responds_to(e.object, "accessibilityPerformPress");
                checks.check(can_press, "the button can be pressed");
                if can_press {
                    objc::send_bool(e.object, "accessibilityPerformPress");
                }
            }
            checks.check(clicks.get() == 1, "pressing the button element clicks the button");

            let check_box_element = find(&elements, "AXCheckBox", "Remember");
            checks.check(check_box_element.is_some(), "the check box is in the tree as AXCheckBox titled Remember");
            if let Some(e) = check_box_element {
                let usable = objc::responds_to(e.object, "accessibilityValue")
                    && objc::responds_to(e.object, "accessibilityPerformPress");
                checks.check(usable, "the check box reports a value and can be pressed");
                if usable {
                    // `None` when the element reports no value object.
                    let value = |object: Id| {
                        let value = objc::send_id(object, "accessibilityValue");
                        objc::responds_to(value, "intValue").then(|| objc::send_i32(value, "intValue"))
                    };
                    checks.check(value(e.object) == Some(0), "the check box reports that it is unchecked");
                    objc::send_bool(e.object, "accessibilityPerformPress");
                    checks.check(value(e.object) == Some(1), "pressing the check box element checks it");
                }
            }
            checks.check(check_box.is_checked() == Some(true), "the check box is checked");
        });

        window.close();
        Dispatcher::ui_thread().run_jobs(None);

        if checks.failures.get() == 0 {
            println!("accessibility_tree: all checks passed");
            ExitCode::SUCCESS
        } else {
            println!("accessibility_tree: {} check(s) failed", checks.failures.get());
            ExitCode::FAILURE
        }
    }
}
