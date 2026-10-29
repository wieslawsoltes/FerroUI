//! Port of `Views/Pages/WindowDragMoveBehaviorView` of the reference sample: the class of
//! `pages/WindowDragMoveBehaviorView.xaml`, whose button opens the window demo
//! (`Views/Windows/WindowDemo`).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::views::windows::window_demo::WindowDemo;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl, Window};
use std::rc::Rc;

#[repr(C)]
pub struct WindowDragMoveBehaviorView {
    base: UserControl,
}

user_control_class!(WindowDragMoveBehaviorView);
ferro_class_info!(WindowDragMoveBehaviorView {
    new: WindowDragMoveBehaviorView::new,
    markup: {
        methods: [
            fn OpenWindowDemo_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowDragMoveBehaviorView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.open_window_demo_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(
    WindowDragMoveBehaviorView,
    "/pages/WindowDragMoveBehaviorView.xaml"
);

impl WindowDragMoveBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `WindowDragMoveBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `OpenWindowDemo_Click`: shows the window demo, as a dialog of the window of the page when
    /// there is one.
    pub fn open_window_demo_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let window = WindowDemo::new();
        if let Some(owner) = self.visual_root().and_then(|root| root.cast::<Window>()) {
            let _ = window.show_dialog(&owner);
        } else {
            window.show();
        }
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    WindowDragMoveBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`), with the window it opens.
pub(crate) const TYPES: &[&TypeInfo] = &[WindowDragMoveBehaviorView::TYPE, WindowDemo::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "WindowDragMoveBehavior",
    reference: "Views/Pages/WindowDragMoveBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[
        &WindowDragMoveBehaviorView::XAML_CLASS,
        &WindowDemo::XAML_CLASS,
    ],
    markup_types: &[],
    register_value_types: no_value_types,
};
