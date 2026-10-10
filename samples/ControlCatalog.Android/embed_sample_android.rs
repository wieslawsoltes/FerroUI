//! Port of `EmbedSample.Android.cs`: the native control demo of Android.
//! The first sample is a button of the system that counts its clicks; the
//! second is a web view.

use control_catalog::pages::INativeDemoControl;
use ferroui_android::interop::java::{
    call_object, call_void, new_object, new_string, JavaClass, JavaObject, JavaRef, JavaValue,
};
use ferroui_android::interop::listeners::set_on_click_listener;
use ferroui_android::{AndroidViewControlHandle, FerroAndroidApplication};
use ferroui_controls::platform::IPlatformHandle;
use std::cell::Cell;
use std::rc::Rc;

pub struct EmbedSampleAndroid;

fn set_text(button: &dyn JavaRef, text: &str) {
    let text = new_string(text);
    call_void(button, "setText", "(Ljava/lang/CharSequence;)V", &[JavaValue::Object(Some(&text))]);
}

impl INativeDemoControl for EmbedSampleAndroid {
    fn create_control(
        &self,
        is_second: bool,
        parent: Rc<dyn IPlatformHandle>,
        _create_default: &dyn Fn() -> Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformHandle> {
        let parent_context: JavaObject = parent
            .as_any()
            .downcast_ref::<AndroidViewControlHandle>()
            .and_then(AndroidViewControlHandle::view)
            .and_then(|view| call_object(&view, "getContext", "()Landroid/content/Context;", &[]))
            .map(|context| context.to_global())
            .unwrap_or_else(FerroAndroidApplication::context);

        if is_second {
            let web_view = new_object(
                &JavaClass::find("android/webkit/WebView"),
                "(Landroid/content/Context;)V",
                &[JavaValue::Object(Some(&parent_context))],
            )
            .to_global();
            call_void(&web_view, "loadUrl", "(Ljava/lang/String;)V", &[JavaValue::String("https://www.android.com/")]);

            Rc::new(AndroidViewControlHandle::new(web_view))
        } else {
            let button = new_object(
                &JavaClass::find("android/widget/Button"),
                "(Landroid/content/Context;)V",
                &[JavaValue::Object(Some(&parent_context))],
            )
            .to_global();
            set_text(&button, "Hello world");
            let click_count = Cell::new(0);
            set_on_click_listener(&button, {
                let button = button.clone();
                Rc::new(move || {
                    click_count.set(click_count.get() + 1);
                    set_text(&button, &format!("Click count {}", click_count.get()));
                })
            });

            Rc::new(AndroidViewControlHandle::new(button))
        }
    }
}
