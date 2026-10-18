//! Port of the code-behind of `MainWindow` of the reference's core demo (`TreeDataGridCoreDemo`):
//! the class of the document `Views/CoreDemoPage.xaml`, the content of the main window of the core
//! demo as a view of its own that the main window of the demo and the ControlCatalog both show.
//! Every grid of the page shows a source of the framework-neutral core through the control's
//! `Model`, never through `Source`. Its data context is a `CoreDemoViewModel`.

use crate::markup::{user_control_class, xaml_class};
use crate::models::{Country, DragDropItem};
use crate::view_models::CoreDemoViewModel;
use ferroui_base::input::DragDropEffects;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{SelectionChangedEventArgs, TextBox, UserControl};
use ferroui_controls_treedatagrid::{
    TreeDataGrid, TreeDataGridRowDragEventArgs, TreeDataGridRowDragStartedEventArgs, TreeDataGridRowDropPosition,
};
use ferroui_treedatagrid_core::ITreeDataGridSource;
use std::rc::Rc;

#[repr(C)]
pub struct CoreDemoPage {
    base: UserControl,
}

user_control_class!(CoreDemoPage);
ferro_class_info!(CoreDemoPage {
    new: CoreDemoPage::new,
    markup: {
        methods: [
            fn ApplyCountryFilter(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.apply_country_filter(&sender, e.as_routed_event_args())
                },
            fn ClearCountrySort(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.clear_country_sort(&sender, e.as_routed_event_args())
                },
            fn AddCountry(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_country(&sender, e.as_routed_event_args())
                },
            fn RemoveCountry(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.remove_country(&sender, e.as_routed_event_args())
                },
            fn FindCountry(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.find_country(&sender, e)
                    }
                },
            fn BringCountryIntoView(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.bring_country_into_view(&sender, e)
                    }
                },
            fn DragStarted(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TreeDataGridRowDragStartedEventArgs>() {
                        this.drag_started(&sender, e)
                    }
                },
            fn DragOver(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CoreDemoPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TreeDataGridRowDragEventArgs>() {
                        this.drag_over(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(CoreDemoPage, "/Views/CoreDemoPage.xaml");

/// The first country among the items a selection added (`e.AddedItems.OfType<Country>().FirstOrDefault()`).
fn first_added_country(e: &SelectionChangedEventArgs) -> Option<Rc<Country>> {
    e.added_items().iter().find_map(from_markup_value::<Rc<Country>>)
}

impl CoreDemoPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// The page; its data context is set by the element that shows it.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// The page with `view_model` as its data context (`MainWindow(CoreDemoViewModel)`).
    pub fn with_view_model(view_model: Rc<CoreDemoViewModel>) -> Ref<Self> {
        let this = Self::new();
        this.set_data_context(Some(view_model as BoxedValue));
        this
    }

    fn view_model(&self) -> Option<Rc<CoreDemoViewModel>> {
        from_markup_value::<Rc<CoreDemoViewModel>>(&self.data_context())
    }

    pub fn apply_country_filter(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(vm) = self.view_model() else {
            return;
        };
        let text = self.find_control::<TextBox>("CountryFilter").and_then(|text_box| text_box.text());
        vm.apply_country_filter(text.as_deref());
    }

    pub fn clear_country_sort(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(vm) = self.view_model() {
            ITreeDataGridSource::clear_sort(&*vm.countries_source());
        }
    }

    pub fn add_country(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(vm) = self.view_model() else {
            return;
        };
        vm.add_country();
        let grid = self.find_control::<TreeDataGrid>("CountriesGrid");
        Dispatcher::ui_thread().post_local(
            move || {
                let last = ITreeDataGridSource::rows(&*vm.countries_source()).count() as i32 - 1;
                if last >= 0 {
                    if let Some(rows_presenter) = grid.and_then(|grid| grid.rows_presenter()) {
                        rows_presenter.bring_into_view(last, None);
                    }
                }
            },
            DispatcherPriority::LOADED,
        );
    }

    pub fn remove_country(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(vm) = self.view_model() {
            vm.remove_selected_country();
        }
    }

    pub fn find_country(&self, _sender: &Option<BoxedValue>, e: &SelectionChangedEventArgs) {
        let Some(country) = first_added_country(e) else {
            return;
        };
        let Some(vm) = self.view_model() else {
            return;
        };

        let rows = ITreeDataGridSource::rows(&*vm.find_source());
        let mut index = -1;
        for i in 0..rows.count() {
            let model = from_markup_value::<Rc<Country>>(&rows.get(i).model());
            if model.is_some_and(|model| Rc::ptr_eq(&model, &country)) {
                index = i as i32;
                break;
            }
        }

        let name = country.name().unwrap_or_default();
        vm.set_find_status(if index >= 0 {
            format!("Model index {name}: displayed Core row {index}")
        } else {
            format!("{name} is filtered out")
        });
        if index >= 0 {
            if let Some(rows_presenter) = self.find_control::<TreeDataGrid>("FindGrid").and_then(|grid| grid.rows_presenter()) {
                rows_presenter.bring_into_view(index, None);
            }
        }
    }

    pub fn bring_country_into_view(&self, _sender: &Option<BoxedValue>, e: &SelectionChangedEventArgs) {
        let Some(country) = first_added_country(e) else {
            return;
        };
        let Some(vm) = self.view_model() else {
            return;
        };

        if let Some(index) = vm.variable_country_items().index_of(&country) {
            let this = self.to_ref();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(rows_presenter) =
                        this.find_control::<TreeDataGrid>("VariableGrid").and_then(|grid| grid.rows_presenter())
                    {
                        rows_presenter.bring_into_view(index as i32, None);
                    }
                },
                DispatcherPriority::LOADED,
            );
        }
    }

    pub fn drag_started(&self, _sender: &Option<BoxedValue>, e: &TreeDataGridRowDragStartedEventArgs) {
        let mut models = e.models().iter().filter_map(from_markup_value::<Rc<DragDropItem>>);
        if models.any(|x| !x.allow_drag()) {
            e.set_allowed_effects(DragDropEffects::NONE);
        }
    }

    pub fn drag_over(&self, _sender: &Option<BoxedValue>, e: &TreeDataGridRowDragEventArgs) {
        if e.position() == TreeDataGridRowDropPosition::Inside {
            let item = e.target_row().and_then(|row| from_markup_value::<Rc<DragDropItem>>(&row.model()));
            if item.is_some_and(|item| !item.allow_drop()) {
                e.inner().set_drag_effects(DragDropEffects::NONE);
            }
        }
    }
}
