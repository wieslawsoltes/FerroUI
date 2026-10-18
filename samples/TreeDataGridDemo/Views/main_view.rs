//! The content of the main window of the reference demo (`MainWindow.axaml`: the tabs of the
//! pages, and `UpdateRealizedCount` of `MainWindow.axaml.cs`, which runs while the view is in a
//! visual tree), as a view that the main window and the ControlCatalog both show. Its data
//! context is a `MainWindowViewModel`.

use crate::markup::xaml_class;
use crate::views::update_realized_count;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, TabControl, TabItem, UserControl};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct MainView {
    base: UserControl,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(MainView: UserControl);
ferro_impl_classes!(
    MainView: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(MainView { new: MainView::new });
xaml_class!(MainView, "/Views/MainView.xaml");

impl VisualImpl for MainView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let view = this.to_ref().downgrade();
        let timer = DispatcherTimer::run(
            move || {
                let Some(view) = view.upgrade() else {
                    return false;
                };
                view.update_realized_count();
                true
            },
            Duration::from_millis(500),
            DispatcherPriority::BACKGROUND,
        );
        if let Some(previous) = this.timer.replace(Some(timer)) {
            previous.dispose();
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        if let Some(timer) = this.timer.take() {
            timer.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl MainView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), timer: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// The tabs of the pages.
    pub fn tabs(&self) -> Ref<TabControl> {
        self.get_control::<TabControl>("tabs")
    }

    fn update_realized_count(&self) {
        let Some(tab_item) = from_markup_value::<Ref<TabItem>>(&self.tabs().selected_item()) else {
            return;
        };
        let Some(content) = from_markup_value::<Ref<Control>>(&tab_item.content()) else {
            return;
        };
        update_realized_count(&content);
    }
}
