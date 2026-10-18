//! The page "BringIntoView (Non-uniform rows)": the tab of that header of the main window of the
//! reference demo (`MainWindow`), with its handler `BringIntoViewNonUniformRows_SelectionChanged`
//! from the code-behind of the main window, as a view of its own that the main window and the
//! ControlCatalog both show. Its data context is a `CountriesPageViewModel` of rows of different heights: a country
//! selected in the list is brought into view in the grid.

use crate::internal_tree_data_grid_source_extensions::TreeDataGridSourceExtensions;
use crate::markup::xaml_class;
use crate::view_models::CountriesPageViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    StyledElementImplExt, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, ItemsSource, ListBox, SelectionChangedEventArgs, UserControl};
use ferroui_controls_treedatagrid::TreeDataGrid;
use std::rc::Rc;

#[repr(C)]
pub struct BringIntoViewNonUniformRowsPage {
    base: UserControl,
}

ferro_class!(BringIntoViewNonUniformRowsPage: UserControl);
ferro_impl_classes!(
    BringIntoViewNonUniformRowsPage: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(BringIntoViewNonUniformRowsPage {
    new: BringIntoViewNonUniformRowsPage::new,
    markup: {
        methods: [
            fn BringIntoViewNonUniformRows_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<BringIntoViewNonUniformRowsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.bring_into_view_non_uniform_rows_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(BringIntoViewNonUniformRowsPage, "/Views/BringIntoViewNonUniformRowsPage.xaml");

impl StyledElementImpl for BringIntoViewNonUniformRowsPage {
    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);
        this.update_country_list();
    }
}

impl BringIntoViewNonUniformRowsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
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

    /// `ItemsSource="{Binding BringIntoViewNonUniformRows.Source.Items}"` of the list: the models
    /// of the source of the view model, all of them, whatever the filter and the sort of its
    /// rows. The view model of the page never changes the models of its source, so the list
    /// shows the models the source has when the data context is set.
    fn update_country_list(&self) {
        let Some(list) = self.find_control::<ListBox>("bringIntoViewNonUniformRowsList") else {
            return;
        };
        let items = self.view_model().map(|vm| ItemsSource::from_values(vm.source().items().to_vec()));
        list.set_items_source(items);
    }

    pub fn bring_into_view_non_uniform_rows_selection_changed(&self, _sender: &Option<BoxedValue>, e: &SelectionChangedEventArgs) {
        let Some(tree_data_grid) = self.find_control::<TreeDataGrid>("bringIntoViewNonUniformRowsGrid") else {
            return;
        };

        if let Some(item) = e.added_items().first() {
            let source = tree_data_grid.source();
            let (Some(item), Some(source)) = (item, source) else {
                return;
            };
            let row_index = source.source().find_displayed_row_index(Some(item));

            Dispatcher::ui_thread().post_local(
                move || {
                    if row_index >= 0 {
                        if let Some(rows_presenter) = tree_data_grid.rows_presenter() {
                            rows_presenter.bring_into_view(row_index, None);
                        }
                    }
                },
                DispatcherPriority::LOADED,
            );
        }
    }
}
