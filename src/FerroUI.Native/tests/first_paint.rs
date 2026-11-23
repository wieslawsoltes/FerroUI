//! Main-thread AppKit test of the paints of a window that is shown: one paint draws it, and a
//! change of its size paints it again (section 9.23 of `docs/porting/desktop-performance.md`).
//!
//! The Metal render target of a window is made by its first paint. Setting it on the view once
//! asked for the display of the layer that paint was about to draw, so every window was painted
//! twice when shown.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("first_paint: macOS only, skipped");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::media::Brushes;
    use ferroui_base::platform::IMacOSTopLevelPlatformHandle;
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use ferroui_controls::{AppBuilder, Application, Border, ContentControl, Control, Window};
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_native::{
        FerroNativePlatformExtensions, FerroNativePlatformOptions, FerroNativeRenderingMode, MacOSTopLevelHandle,
    };
    use ferroui_skia::SkiaApplicationExtensions;
    use std::cell::Cell;
    use std::ffi::{c_char, c_void, CStr};
    use std::rc::Rc;

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> Id;
        fn objc_autoreleasePoolPop(pool: Id);
    }

    // Each helper uses the exact C ABI of the named Objective-C selector.
    fn send_id(object: Id, selector: &CStr) -> Id {
        unsafe {
            let send: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(object, sel_registerName(selector.as_ptr()))
        }
    }

    fn is_kind_of(object: Id, class: &CStr) -> bool {
        unsafe {
            let class = objc_getClass(class.as_ptr());
            if class.is_null() {
                return false;
            }
            let send: unsafe extern "C" fn(Id, Sel, Id) -> bool =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(object, sel_registerName(c"isKindOfClass:".as_ptr()), class)
        }
    }

    /// Runs the main run loop for `seconds`: the jobs of the dispatcher, the display of the views
    /// and the transactions of Core Animation, as in a running application.
    fn run_main_loop(seconds: f64) {
        unsafe {
            let date_with: unsafe extern "C" fn(Id, Sel, f64) -> Id =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let until = date_with(
                objc_getClass(c"NSDate".as_ptr()),
                sel_registerName(c"dateWithTimeIntervalSinceNow:".as_ptr()),
                seconds,
            );
            let run_loop = send_id(objc_getClass(c"NSRunLoop".as_ptr()), c"mainRunLoop");
            let run_until: unsafe extern "C" fn(Id, Sel, Id) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            run_until(run_loop, sel_registerName(c"runUntilDate:".as_ptr()), until);
        }
    }

    fn check_paints() {
        let _builder = AppBuilder::configure::<Application>()
            .use_harfbuzz()
            .use_ferro_native()
            .use_skia()
            .with(Rc::new(FerroNativePlatformOptions {
                rendering_mode: vec![FerroNativeRenderingMode::Metal, FerroNativeRenderingMode::Software],
                ..Default::default()
            }))
            .setup_without_starting();
        let content = Border::new();
        content.set_background(Some(Brushes::cornflower_blue()));
        let window = Window::new();
        window.set_template(Some(FuncControlTemplate::for_type::<Window>(|_, scope| {
            let presenter = ContentPresenter::new();
            presenter.set_name(Some("PART_ContentPresenter".to_owned()));
            let content = ContentControl::content_property().as_property();
            presenter.bind_binding(content, &TemplateBinding::new(content));
            presenter.register_in_name_scope(&**scope).upcast()
        })));
        window.set_width(300.0);
        window.set_height(180.0);
        window.set_content(Some(Control::boxed(content)));

        // Count the paints the platform asks of the top level.
        let implementation = window.platform_impl().expect("the window has its platform implementation");
        let paint = implementation.paint().expect("the top level paints its window");
        let paints = Rc::new(Cell::new(0_u32));
        implementation.set_paint(Some(Rc::new({
            let paints = paints.clone();
            move |rect| {
                paints.set(paints.get() + 1);
                paint(rect);
            }
        })));

        window.show();
        run_main_loop(0.5);

        let handle = window.try_get_platform_handle().unwrap();
        let view = handle.as_any().downcast_ref::<MacOSTopLevelHandle>().unwrap().ns_view() as Id;
        let metal = is_kind_of(send_id(view, c"layer"), c"CAMetalLayer");
        let shown = paints.get();
        if shown == 0 {
            // A session without a display shows no window, so nothing is ever painted.
            println!("first_paint: the window was not displayed, skipped");
            window.close();
            return;
        }
        if metal {
            assert_eq!(shown, 1, "a window that is shown is painted once");
        }
        // Without Metal (a host without a GPU) the render target is an IOSurface, whose windows
        // are asked to display as upstream's.

        // A change of the size is painted.
        window.set_width(420.0);
        window.set_height(260.0);
        run_main_loop(0.5);
        let resized = paints.get();
        assert!(resized > shown, "a window whose size changed is painted again ({shown} paints, then {resized})");

        // Nothing changes: nothing is painted.
        run_main_loop(0.3);
        assert_eq!(paints.get(), resized, "a window in which nothing changes is not painted");

        window.close();
        run_main_loop(0.1);
        println!(
            "first_paint: {shown} paint when shown ({}), {} for a changed size, none while idle",
            if metal { "Metal" } else { "no Metal" },
            resized - shown
        );
    }

    pub fn run() {
        // SAFETY: this dedicated harness runs AppKit on the main thread; the pool encloses every
        // Objective-C value used by the test.
        let pool = unsafe { objc_autoreleasePoolPush() };
        check_paints();
        unsafe { objc_autoreleasePoolPop(pool) };
    }
}
