//! Port of the code-behind of `Views/FindDisplayedRowIndexPage` of the reference demo: the class of
//! the document `Views/FindDisplayedRowIndexPage.xaml`, the page "Find Displayed Row" of the main
//! window. A country selected in the list of all the models of the source is found among the rows
//! the grid displays, after the filter and the sort of the source, and brought into view.

use crate::internal_tree_data_grid_source_extensions::TreeDataGridSourceExtensions;
use crate::markup::xaml_class;
use crate::models::Country;
use crate::view_models::CountriesPageViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    StyledElementImplExt, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, ItemsSource, ListBox, SelectionChangedEventArgs, TextBlock, UserControl,
};
use ferroui_controls_treedatagrid::TreeDataGrid;
use ferroui_treedatagrid_core::models::{IRows, RowsChangedArgs, RowsChangedHandler};
use ferroui_treedatagrid_core::{FlatTreeDataGridSource, ITreeDataGridSource};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The source the page follows, with the rows and the subscriptions it holds on them.
struct AttachedSource {
    source: Rc<FlatTreeDataGridSource<Country>>,
    rows: Rc<dyn IRows>,
    rows_changed_token: u64,
    sorted_token: u64,
}

#[repr(C)]
pub struct FindDisplayedRowIndexPage {
    base: UserControl,
    source: RefCell<Option<AttachedSource>>,
    is_attached_to_visual_tree: Cell<bool>,
}

ferro_class!(FindDisplayedRowIndexPage: UserControl);
ferro_impl_classes!(
    FindDisplayedRowIndexPage: FerroObjectImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(FindDisplayedRowIndexPage {
    new: FindDisplayedRowIndexPage::new,
    markup: {
        methods: [
            fn ClearSortClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FindDisplayedRowIndexPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.clear_sort_click(&sender, e.as_routed_event_args())
                },
            fn CountrySelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FindDisplayedRowIndexPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.country_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(FindDisplayedRowIndexPage, "/Views/FindDisplayedRowIndexPage.xaml");

impl VisualImpl for FindDisplayedRowIndexPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.is_attached_to_visual_tree.set(true);
        this.attach_to_source();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.is_attached_to_visual_tree.set(false);
        this.detach_from_source();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl StyledElementImpl for FindDisplayedRowIndexPage {
    fn on_data_context_changed(this: &Self) {
        this.detach_from_source();
        Self::parent_on_data_context_changed(this);
        this.update_country_list();

        if this.is_attached_to_visual_tree.get() {
            this.attach_to_source();
        }
    }
}

impl FindDisplayedRowIndexPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), source: RefCell::new(None), is_attached_to_visual_tree: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.update_country_list();
        this
    }

    fn view_model(&self) -> Option<Rc<CountriesPageViewModel>> {
        from_markup_value::<Rc<CountriesPageViewModel>>(&self.data_context())
    }

    pub fn clear_sort_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(vm) = self.view_model() {
            vm.clear_sort();
        }
    }

    pub fn country_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        self.queue_bring_selected_country_into_view();
    }

    /// `ItemsSource="{CompiledBinding Source.Items}"` of the list of the countries: the models of
    /// the source of the view model, all of them, whatever the filter and the sort of its rows.
    /// The view models of the pages that show this view never change the models of their
    /// sources, so the list shows the models the source has when the data context is set.
    fn update_country_list(&self) {
        let Some(country_list) = self.find_control::<ListBox>("displayedRowCountryList") else {
            return;
        };
        let items = self.view_model().map(|vm| ItemsSource::from_values(vm.source().items().to_vec()));
        country_list.set_items_source(items);
    }

    fn attach_to_source(&self) {
        self.detach_from_source();

        let Some(source) = self.view_model().map(|vm| vm.source()) else {
            return;
        };

        let rows = ITreeDataGridSource::rows(&*source);
        let page = self.to_ref().downgrade();
        let rows_changed_token = rows.add_collection_changed(Rc::new(move |_e: &RowsChangedArgs<'_>| {
            if let Some(page) = page.upgrade() {
                page.rows_collection_changed();
            }
        }) as Rc<RowsChangedHandler>);
        let page = self.to_ref().downgrade();
        let sorted_token = ITreeDataGridSource::add_sorted(
            &*source,
            Rc::new(move || {
                if let Some(page) = page.upgrade() {
                    page.source_sorted();
                }
            }),
        );

        *self.source.borrow_mut() = Some(AttachedSource { source, rows, rows_changed_token, sorted_token });
    }

    fn detach_from_source(&self) {
        // Taken out first: the subscriptions are removed without a borrow of the field held.
        let attached = self.source.borrow_mut().take();
        if let Some(attached) = attached {
            attached.rows.remove_collection_changed(attached.rows_changed_token);
            ITreeDataGridSource::remove_sorted(&*attached.source, attached.sorted_token);
        }
    }

    fn rows_collection_changed(&self) {
        self.queue_bring_selected_country_into_view();
    }

    fn source_sorted(&self) {
        self.queue_bring_selected_country_into_view();
    }

    fn queue_bring_selected_country_into_view(&self) {
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(move || this.bring_selected_country_into_view(), DispatcherPriority::LOADED);
    }

    fn bring_selected_country_into_view(&self) {
        let country_list = self.find_control::<ListBox>("displayedRowCountryList");
        let grid = self.find_control::<TreeDataGrid>("displayedRowGrid");
        let status = self.find_control::<TextBlock>("displayedRowStatus");
        let selected = country_list.and_then(|country_list| country_list.selected_item());
        let country = from_markup_value::<Rc<Country>>(&selected);
        let source = self.source.borrow().as_ref().map(|attached| attached.source.clone());

        let (Some(source), Some(grid), Some(status)) = (source, grid, status) else {
            return;
        };

        let row_index = source.find_displayed_row_index(country.as_ref().map(|country| country.clone() as BoxedValue).as_ref());

        match country {
            None => status.set_text(Some("Select a country to find its displayed row.")),
            Some(country) if row_index < 0 => {
                let name = country.name().unwrap_or_default();
                status.set_text(Some(&format!("{name} is not displayed by the current filter.")));
            }
            Some(country) => {
                let name = country.name().unwrap_or_default();
                status.set_text(Some(&format!("{name} is displayed at zero-based row index {row_index}.")));
                if let Some(rows_presenter) = grid.rows_presenter() {
                    rows_presenter.bring_into_view(row_index, None);
                }
            }
        }
    }
}
