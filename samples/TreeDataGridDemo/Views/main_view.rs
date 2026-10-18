//! The content of the main window of the reference demo (`MainWindow.xaml`: the tabs of the
//! pages, and `UpdateRealizedCount` of `MainWindow.xaml.cs`, which runs while the view is in a
//! visual tree), as a view that the main window and the ControlCatalog both show. Its data
//! context is a `MainWindowViewModel`.

use crate::markup::xaml_class;
use crate::view_models::MainWindowViewModel;
use crate::views::{update_realized_count, CoreDemoPage};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
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

        // The page of the core demo has the core view model of the main view model as its local
        // data context (`DataContext = viewModel` of the reference's core demo window): a binding
        // would lose it while the page is out of the tabs, and its grids would show their models
        // without the presentation options their columns need.
        let weak = this.to_ref().downgrade();
        this.property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
            if e.property() != StyledElement::data_context_property().as_property() {
                return;
            }
            if let Some(this) = weak.upgrade() {
                this.update_core_page();
            }
        });
        this.update_core_page();
        this
    }

    /// The page of the core demo.
    pub fn core_page(&self) -> Ref<CoreDemoPage> {
        self.get_control::<CoreDemoPage>("corePage")
    }

    fn update_core_page(&self) {
        let view_model = from_markup_value::<Rc<MainWindowViewModel>>(&self.data_context());
        let core = view_model.map(|view_model| view_model.core() as BoxedValue);
        let page = self.core_page();
        if page.data_context().is_some() || core.is_some() {
            page.set_data_context(core);
        }
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
