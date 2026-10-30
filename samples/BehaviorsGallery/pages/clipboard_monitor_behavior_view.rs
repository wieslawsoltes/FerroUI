//! Port of `Views/Pages/ClipboardMonitorBehaviorView` of the reference sample: the class of
//! `pages/ClipboardMonitorBehaviorView.xaml`, whose button pastes the text of the clipboard.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo, Visual};
use ferroui_controls::{Control, TextBox, TopLevel, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct ClipboardMonitorBehaviorView {
    base: UserControl,
}

user_control_class!(ClipboardMonitorBehaviorView);
ferro_class_info!(ClipboardMonitorBehaviorView {
    new: ClipboardMonitorBehaviorView::new,
    markup: {
        methods: [
            fn PasteButton_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ClipboardMonitorBehaviorView>, _sender: Option<BoxedValue>, _e: Rc<dyn IRoutedEventArgs>| {
                    ClipboardMonitorBehaviorView::paste_button_click(this)
                },
        ],
    },
});
xaml_class!(
    ClipboardMonitorBehaviorView,
    "/pages/ClipboardMonitorBehaviorView.xaml"
);

impl ClipboardMonitorBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ClipboardMonitorBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `PasteButton_Click` (`async void`): reads the text of the clipboard of the top level of
    /// the page, in a job of the dispatcher, and writes it to the output box.
    pub fn paste_button_click(this: &Ref<Self>) {
        let visual: &Visual = this;
        let Some(clipboard) = TopLevel::get_top_level(Some(visual)).and_then(|top_level| top_level.clipboard())
        else {
            return;
        };
        let this = this.clone();
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let text = clipboard.try_get_text_async().await.ok().flatten();
            if let Some(output_box) = this.find_control::<TextBox>("OutputBox") {
                output_box.set_text(text.as_deref());
            }
        }));
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ClipboardMonitorBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ClipboardMonitorBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ClipboardMonitorBehavior",
    reference: "Views/Pages/ClipboardMonitorBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ClipboardMonitorBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
