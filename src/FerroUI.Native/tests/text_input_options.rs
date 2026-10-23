//! Main-thread AppKit protocol test. Uses the actual view of a FerroUI window,
//! not a second text field or a synthetic options object. No keyboard event is
//! sent outside this process and no native prediction service is enabled.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("text_input_options: macOS only, skipped");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions};
    use ferroui_base::input::{InputScope, InputScopeNameValue};
    use ferroui_base::platform::IMacOSTopLevelPlatformHandle;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::presenters::ContentPresenter;
    use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use ferroui_controls::{
        AppBuilder, Application, Border, Button, ContentControl, Control, RichEditBox, StackPanel,
        Window,
    };
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_native::{
        FerroNativePlatformExtensions, FerroNativePlatformOptions, FerroNativeRenderingMode,
        MacOSTopLevelHandle,
    };
    use ferroui_skia::SkiaApplicationExtensions;
    use std::ffi::{c_char, c_void, CStr};
    use std::rc::Rc;

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Range {
        location: usize,
        length: usize,
    }

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn objc_getProtocol(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> Id;
        fn objc_autoreleasePoolPop(pool: Id);
        fn dlsym(handle: Id, name: *const c_char) -> Id;
    }

    // Each helper uses the exact C ABI of the named Objective-C selector.
    // All receivers and returned objects live in run()'s autorelease pool.
    fn send_id(object: Id, selector: &CStr) -> Id {
        unsafe {
            let send: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(object, sel_registerName(selector.as_ptr()))
        }
    }

    fn send_range(object: Id, selector: &CStr) -> Range {
        unsafe {
            let send: unsafe extern "C" fn(Id, Sel) -> Range =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(object, sel_registerName(selector.as_ptr()))
        }
    }

    fn send_void(object: Id, selector: &CStr) {
        unsafe {
            let send: unsafe extern "C" fn(Id, Sel) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(object, sel_registerName(selector.as_ptr()));
        }
    }

    fn conforms(object: Id, protocol: &CStr) -> bool {
        unsafe {
            let protocol = objc_getProtocol(protocol.as_ptr());
            if protocol.is_null() {
                return false;
            }
            let send: unsafe extern "C" fn(Id, Sel, Id) -> bool =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(
                object,
                sel_registerName(c"conformsToProtocol:".as_ptr()),
                protocol,
            )
        }
    }

    /// Public NSString globals may be absent on a supported pre-14 system.
    /// Resolving them at runtime tests that version's documented fallback.
    fn content_constant(name: &CStr) -> Id {
        unsafe {
            let symbol = dlsym(-2_isize as Id, name.as_ptr()); // Darwin RTLD_DEFAULT
            if symbol.is_null() {
                std::ptr::null_mut()
            } else {
                *symbol.cast::<Id>()
            }
        }
    }

    fn assert_content(view: Id, name: Option<&CStr>) {
        let actual = send_id(view, c"contentType");
        let expected = name.map_or(std::ptr::null_mut(), content_constant);
        if expected.is_null() {
            assert!(actual.is_null(), "native semantic hint: {name:?}");
        } else {
            assert!(!actual.is_null(), "native semantic hint: {name:?}");
            // NSString copy semantics do not require pointer identity.
            let equal = unsafe {
                let send: unsafe extern "C" fn(Id, Sel, Id) -> bool =
                    std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
                send(
                    actual,
                    sel_registerName(c"isEqualToString:".as_ptr()),
                    expected,
                )
            };
            assert!(equal, "native semantic hint: {name:?}");
        }
    }

    fn set_marked_text(view: Id) {
        unsafe {
            let string: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let text = string(
                objc_getClass(c"NSString".as_ptr()),
                sel_registerName(c"stringWithUTF8String:".as_ptr()),
                c"preview".as_ptr(),
            );
            let mark: unsafe extern "C" fn(Id, Sel, Id, Range, Range) =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            mark(
                view,
                sel_registerName(c"setMarkedText:selectedRange:replacementRange:".as_ptr()),
                text,
                Range {
                    location: 7,
                    length: 0,
                },
                Range {
                    location: isize::MAX as usize,
                    length: 0,
                },
            );
        }
    }

    fn check_view() {
        let _builder = AppBuilder::configure::<Application>()
            .use_harfbuzz()
            .use_ferro_native()
            .use_skia()
            .with(Rc::new(FerroNativePlatformOptions {
                rendering_mode: vec![FerroNativeRenderingMode::Software],
                ..Default::default()
            }))
            .setup_without_starting();
        let owner = RichEditBox::new();
        owner.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
            |_, scope| {
                let host = Border::new();
                host.set_name(Some("ContentElement".to_owned()));
                host.register_in_name_scope(&**scope).upcast()
            },
        )));
        owner.set_height(80.0);
        owner.set_text("abcd").unwrap();
        owner.set_is_text_prediction_enabled(true);
        let other = Button::new();
        other.set_focusable(true);
        other.set_height(24.0);
        let panel = StackPanel::new();
        panel.children().add(owner.clone());
        panel.children().add(other.clone());
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
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(owner.focus());
        let handle = window.try_get_platform_handle().unwrap();
        let handle = handle
            .as_any()
            .downcast_ref::<MacOSTopLevelHandle>()
            .unwrap();
        let view = handle.ns_view() as Id;
        assert!(conforms(view, c"NSTextInputClient"));
        // This protocol exists on macOS 11+, including both current CI hosts.
        if !unsafe { objc_getProtocol(c"NSTextContent".as_ptr()) }.is_null() {
            assert!(conforms(view, c"NSTextContent"));
        }

        let supported = [
            (InputScopeNameValue::Url, c"NSTextContentTypeURL"),
            (
                InputScopeNameValue::EmailSmtpAddress,
                c"NSTextContentTypeEmailAddress",
            ),
            (
                InputScopeNameValue::EmailNameOrAddress,
                c"NSTextContentTypeEmailAddress",
            ),
            (
                InputScopeNameValue::PersonalFullName,
                c"NSTextContentTypeName",
            ),
            (
                InputScopeNameValue::TelephoneNumber,
                c"NSTextContentTypeTelephoneNumber",
            ),
        ];
        for name in InputScopeNameValue::ALL {
            owner.set_input_scope(Some(InputScope::with_name_value(name)));
            assert_content(
                view,
                supported
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| *value),
            );
        }

        let scope = InputScope::with_name_value(InputScopeNameValue::EmailSmtpAddress);
        owner.set_input_scope(Some(scope.clone()));
        scope
            .names()
            .get(0)
            .set_name_value(InputScopeNameValue::PersonalFullName);
        assert_content(view, Some(c"NSTextContentTypeName"));
        owner.select(1, 3);
        owner.editor().clear_history();
        set_marked_text(view);
        assert!(owner.editor().is_composing());
        let preview = owner.document().snapshot();
        let marked = send_range(view, c"markedRange");
        assert_eq!(marked.length, 7);
        let context = send_id(view, c"inputContext");
        assert!(!context.is_null());
        owner.set_input_scope(Some(InputScope::with_name_value(InputScopeNameValue::Url)));
        assert_content(view, Some(c"NSTextContentTypeURL"));
        assert_eq!(send_range(view, c"markedRange"), marked);
        assert_eq!(send_id(view, c"inputContext"), context);
        assert_eq!(owner.document().snapshot(), preview);
        assert!(!owner.can_undo());

        TextInputOptions::set_is_sensitive(&owner, true);
        assert_content(view, None);
        assert_eq!(send_range(view, c"markedRange"), marked);
        TextInputOptions::set_is_sensitive(&owner, false);
        assert_content(view, Some(c"NSTextContentTypeURL"));
        TextInputOptions::set_content_type(&owner, TextInputContentType::Password);
        assert_content(view, None);
        TextInputOptions::set_content_type(&owner, TextInputContentType::Normal);
        assert_content(view, Some(c"NSTextContentTypeURL"));
        owner.set_is_text_prediction_enabled(false);
        assert_content(view, None);
        owner.set_is_text_prediction_enabled(true);
        assert_content(view, Some(c"NSTextContentTypeURL"));
        send_void(view, c"unmarkText");
        assert!(!owner.editor().is_composing());

        assert!(other.focus());
        assert_content(view, None);
        owner.set_input_scope(Some(InputScope::with_name_value(
            InputScopeNameValue::TelephoneNumber,
        )));
        assert_content(view, None);
        assert!(owner.focus());
        assert_content(view, Some(c"NSTextContentTypeTelephoneNumber"));
        window.close();
        Dispatcher::ui_thread().run_jobs(None);
        println!("text_input_options: 43 mappings, live hints, privacy, focus and composition checks passed");
    }

    pub fn run() {
        // SAFETY: this dedicated harness runs AppKit on the main thread; the
        // pool encloses every Objective-C value used by the test.
        let pool = unsafe { objc_autoreleasePoolPush() };
        check_view();
        unsafe { objc_autoreleasePoolPop(pool) };
    }
}
