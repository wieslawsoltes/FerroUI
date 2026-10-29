//! Not from upstream: the page of the port's InkToolbar (docs/porting/winui-inking.md), the class of
//! `Pages/InkToolbarPage.xaml`. It targets the canvas of the page and shows the active tool.
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, TextBlock};
use ferroui_controls_inking::{CoreInputDeviceTypes, InkCanvas, InkToolbar};

#[repr(C)]
pub struct InkToolbarPage {
    base: ContentPage,
}
content_page_class!(InkToolbarPage);
ferro_class_info!(InkToolbarPage {
    new: InkToolbarPage::new
});
xaml_class!(InkToolbarPage, "/Pages/InkToolbarPage.xaml");

impl InkToolbarPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let canvas = this
            .find_control::<InkCanvas>("InkSurface")
            .expect("InkToolbarPage InkSurface");
        canvas.ink_presenter().set_input_device_types(
            CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE | CoreInputDeviceTypes::TOUCH,
        );
        let toolbar = this
            .find_control::<InkToolbar>("Toolbar")
            .expect("InkToolbarPage Toolbar");
        toolbar.set_target_ink_canvas(Some(canvas));
        let weak = this.downgrade();
        toolbar.active_tool_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.show_active_tool();
            }
        });
        // The toolbar selects its first pen when it is loaded, before the label can follow it.
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.show_active_tool();
            }
        });
        this
    }

    fn show_active_tool(&self) {
        let Some(toolbar) = self.find_control::<InkToolbar>("Toolbar") else { return };
        let tool = toolbar
            .active_tool()
            .map(|tool| format!("{:?}", tool.tool_kind()))
            .unwrap_or_else(|| "none".into());
        if let Some(label) = self.find_control::<TextBlock>("ActiveTool") {
            label.set_text(Some(&format!("Active tool: {tool}")));
        }
    }
}
