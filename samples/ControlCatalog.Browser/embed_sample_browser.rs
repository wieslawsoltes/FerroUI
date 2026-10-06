//! Port of `EmbedSample.Browser.cs`: the native control demo of the browser.
//! The first sample is the default native control of the platform (a `div`)
//! with a button of the page added by `wwwroot/embed.js`; the second is an
//! `iframe` showing a video.
//!
//! Differs from the original in how `embed.js` is loaded: the original
//! imports it on first use (`JSHost.ImportAsync`) and adds the button once
//! the import has completed; here the script is imported with the module
//! (the boundary has no run-time module import) and the button is added
//! while the control is created.

use control_catalog::pages::INativeDemoControl;
use ferroui_browser::JsObjectControlHandle;
use ferroui_controls::platform::IPlatformHandle;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

pub struct EmbedSampleWeb;

impl INativeDemoControl for EmbedSampleWeb {
    fn create_control(
        &self,
        is_second: bool,
        _parent: Rc<dyn IPlatformHandle>,
        create_default: &dyn Fn() -> Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformHandle> {
        if is_second {
            let iframe = embed_interop::create_element("iframe");
            embed_interop::set_property(&iframe, "src", "https://www.youtube.com/embed/kZCIporjJ70");

            Rc::new(JsObjectControlHandle::new(iframe))
        } else {
            let parent_container = create_default();
            let Some(parent_container_handle) = parent_container.as_any().downcast_ref::<JsObjectControlHandle>() else {
                panic!("Unable to cast the platform handle to JsObjectControlHandle.");
            };

            add_button(parent_container_handle.object());

            parent_container
        }
    }
}

fn add_button(parent: Option<JsValue>) {
    let Some(parent) = parent else {
        panic!("Cannot access a disposed object. Object name: 'JsObjectControlHandle'.");
    };
    embed_interop::add_app_button(&parent);
}

mod embed_interop {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["globalThis", "document"], js_name = createElement)]
        pub fn create_element(tag_name: &str) -> JsValue;

        // `JSObject.SetProperty` of the original.
        #[wasm_bindgen(js_namespace = Reflect, js_name = set)]
        fn reflect_set(target: &JsValue, property_key: &str, value: &str) -> bool;
    }

    #[wasm_bindgen(raw_module = "./embed.js")]
    extern "C" {
        #[wasm_bindgen(js_name = addAppButton)]
        pub fn add_app_button(parent_object: &JsValue);
    }

    /// Sets the property `name` of the object `target` to `value`.
    pub fn set_property(target: &JsValue, name: &str, value: &str) {
        reflect_set(target, name, value);
    }
}
