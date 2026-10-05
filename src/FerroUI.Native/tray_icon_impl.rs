use crate::ferro_native_menu_exporter::FerroNativeMenuExporter;
use crate::frn_string::to_c_string;
use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::platform::{INativeMenuExporter, ITrayIconImpl, ITrayIconWithIsTemplateImpl, IWindowIconImpl};
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

/// A status bar (tray) icon.
pub struct TrayIconImpl {
    native: RefCell<Option<ComPtr<IFrnTrayIcon>>>,
    menu_exporter: Rc<FerroNativeMenuExporter>,
    on_clicked: RefCell<Option<Rc<dyn Fn()>>>,
}

impl TrayIconImpl {
    pub(crate) fn new(factory: &ComPtr<IFerroNativeFactory>) -> TrayIconImpl {
        let native = factory.create_tray_icon().check().expect("the native tray icon");

        TrayIconImpl {
            menu_exporter: FerroNativeMenuExporter::for_tray_icon(native.clone(), factory),
            native: RefCell::new(Some(native)),
            on_clicked: RefCell::new(None),
        }
    }

    #[track_caller]
    fn native(&self) -> ComPtr<IFrnTrayIcon> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: the native tray icon"),
        }
    }
}

impl IDisposable for TrayIconImpl {
    fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }
}

impl ITrayIconImpl for TrayIconImpl {
    fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
        match icon {
            // SAFETY: a null pointer with a zero length clears the icon.
            None => unsafe { self.native().set_icon(std::ptr::null_mut(), 0) }.check(),
            Some(icon) => {
                let mut image_data = Vec::new();
                if let Err(error) = icon.save(&mut image_data) {
                    panic!("Unable to save the tray icon: {error}");
                }

                // SAFETY: the native side copies the image data during the call.
                unsafe { self.native().set_icon(image_data.as_mut_ptr() as *mut c_void, image_data.len()) }.check();
            }
        }
    }

    fn set_tool_tip_text(&self, text: Option<&str>) {
        let text = text.map(to_c_string);
        self.native().set_tool_tip_text(text.as_deref()).check();
    }

    fn set_is_visible(&self, visible: bool) {
        self.native().set_is_visible(visible).check();
    }

    fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
        Some(self.menu_exporter.clone())
    }

    fn on_clicked(&self) -> Option<Rc<dyn Fn()>> {
        self.on_clicked.borrow().clone()
    }

    fn set_on_clicked(&self, value: Option<Rc<dyn Fn()>>) {
        let old = self.on_clicked.replace(value);
        drop(old);
    }

    fn as_tray_icon_with_is_template_impl(&self) -> Option<&dyn ITrayIconWithIsTemplateImpl> {
        Some(self)
    }
}

impl ITrayIconWithIsTemplateImpl for TrayIconImpl {
    fn set_is_template_icon(&self, is_template_icon: bool) {
        self.native().set_is_template_icon(is_template_icon).check();
    }
}
