//! Port of `Diagnostics/Views/MainView.xaml.cs`: the class of the document
//! `MainView.xaml`, with the tabs of the extension pages (an addition of the
//! port).

use crate::diagnostics::view_models::MainViewModel;
use ferroui_base::controls::NameScopeExtensions;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, StyledElementImplExt, VisualImpl,
};
use ferroui_markup::data::Binding;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControl, TabControl, TabItem};
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

/// The main view of the developer tools: the menu, the tabs of the pages,
/// the content of the selected tab and the status bar.
#[repr(C)]
pub struct MainView {
    base: UserControl,
    /// Addition of the port: the tabs of the extension pages of the view
    /// model, after the tabs of the document.
    extension_tabs: RefCell<Vec<Ref<TabItem>>>,
}

ferro_class!(MainView: UserControl);
ferro_impl_classes!(
    MainView: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(MainView { new: MainView::new });

impl StyledElementImpl for MainView {
    /// Addition of the port: the tabs of the extension pages follow the view
    /// model.
    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);
        this.update_extension_tabs();
    }
}

impl MainView {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/MainView.xaml";

    pub fn construct() -> Self {
        Self { base: UserControl::construct(), extension_tabs: RefCell::new(Vec::new()) }
    }

    /// Creates the view and populates it from its document
    /// (`InitializeComponent()`: the compiled markup of the document, which
    /// the build of the crate generates).
    ///
    /// # Panics
    /// Panics when the document fails to load (an exception of the
    /// constructor in the original).
    pub fn new() -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);
        this
    }

    /// `InitializeComponent()`.
    fn initialize_component(this: &Ref<Self>) {
        if let Err(error) = crate::compiled_diagnostics_views_main_view::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }

    /// Addition of the port: replaces the tabs of the extension pages with
    /// a tab for each extension page of the view model, in order, after the
    /// tabs of the document (the tab of the page at index `i` is the tab
    /// `MainViewModel::FIRST_EXTENSION_TAB + i`).
    fn update_extension_tabs(&self) {
        let Some(tab_control) = NameScopeExtensions::find_name_scope(self)
            .and_then(|name_scope| name_scope.find_as::<TabControl>("tabControl"))
        else {
            // The document is not populated yet.
            return;
        };
        let items = tab_control.items();

        let old_tabs: Vec<Ref<TabItem>> = self.extension_tabs.borrow_mut().drain(..).collect();
        for tab in old_tabs {
            items.remove(&Some(Control::boxed(tab)));
        }

        let Some(view_model) = from_markup_value::<Rc<MainViewModel>>(&self.data_context()) else {
            return;
        };

        let mut tabs = Vec::new();
        for page in view_model.extension_pages() {
            let tab = TabItem::new();
            tab.set_header(into_markup_value(page.title().to_string()));
            // The content of the view model, as the tabs of the document.
            let _ = tab.bind_binding(ContentControl::content_property().as_property(), &*Binding::with_path("Content"));
            items.add(Some(Control::boxed(tab.clone())));
            tabs.push(tab);
        }
        *self.extension_tabs.borrow_mut() = tabs;
    }
}
