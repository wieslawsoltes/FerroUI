//! Port of `Diagnostics/Views/ControlDetailsView.xaml.cs`: the class of the document
//! `ControlDetailsView.xaml`.
//!
//! The property grid of the document is a TreeDataGrid, where the managed original has a
//! DataGrid (docs/porting/DEVIATIONS.md, Developer tools).

use crate::diagnostics::view_models::{ControlDetailsViewModel, SetterViewModelRef};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElementImpl, PointerPressedEventArgs, TappedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::StyledElementImplExt;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, UserControl};
use ferroui_controls_treedatagrid::TreeDataGrid;
use std::rc::Rc;

/// The details of the selected object of a tree page: the property grid, the layout explorer,
/// the value frames with their setters, and the pseudo-classes.
#[repr(C)]
pub struct ControlDetailsView {
    base: UserControl,
}

ferro_class!(ControlDetailsView: UserControl);
ferro_impl_classes!(
    ControlDetailsView: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
impl StyledElementImpl for ControlDetailsView {
    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        // Deviation (DEVIATIONS.md, Developer tools): the view is reused for the details of the next
        // selected control (the view locator recycles it); its property grid starts at the top, as
        // the new view of upstream does.
        if let Some(scroll) = this.find_control::<TreeDataGrid>("DataGrid").and_then(|grid| grid.scroll()) {
            scroll.set_offset(ferroui_base::Vector::default());
        }
    }
}

ferro_class_info!(ControlDetailsView {
    new: ControlDetailsView::new,
    markup: {
        methods: [
            fn PropertiesGrid_OnDoubleTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ControlDetailsView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<TappedEventArgs>() {
                        this.properties_grid_on_double_tapped(&sender, e)
                    }
                },
            fn PropertyNamePressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ControlDetailsView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.property_name_pressed(&sender, e)
                    }
                },
        ],
    },
});

/// The details view model of an untyped value (`DataContext as ControlDetailsViewModel`).
fn details_of(value: Option<&BoxedValue>) -> Option<Rc<ControlDetailsViewModel>> {
    let value = value?;
    if let Some(details) = value.downcast_ref::<Rc<ControlDetailsViewModel>>() {
        return Some(details.clone());
    }
    let any: Rc<dyn std::any::Any> = value.clone();
    any.downcast::<ControlDetailsViewModel>().ok()
}

impl ControlDetailsView {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/ControlDetailsView.xaml";

    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// # Panics
    /// When the document fails to load or lacks the grid (an exception of the constructor in the
    /// managed original).
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);

        // `_dataGrid = this.GetControl<DataGrid>("DataGrid")`: the grid is looked up when it is
        // used (`data_grid`), so that the view does not hold an element of its own tree.
        let _ = this.data_grid();

        this
    }

    fn initialize_component(this: &Ref<Self>) {
        crate::register_types();
        if let Err(error) = crate::compiled_diagnostics_views_control_details_view::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }

    /// The property grid (`_dataGrid`).
    fn data_grid(&self) -> Ref<TreeDataGrid> {
        self.get_control::<TreeDataGrid>("DataGrid")
    }

    fn properties_grid_on_double_tapped(&self, sender: &Option<BoxedValue>, _e: &TappedEventArgs) {
        let grid = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<TreeDataGrid>());
        if let Some(grid) = grid {
            if let Some(control_details) = details_of(grid.data_context().as_ref()) {
                control_details.navigate_to_selected_property();
            }
        }
    }

    pub fn property_name_pressed(&self, sender: &Option<BoxedValue>, _e: &PointerPressedEventArgs) {
        let Some(main_vm) = details_of(self.data_context().as_ref()) else {
            return;
        };

        let control = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Control>());
        let setter_vm = control
            .and_then(|control| control.data_context())
            .and_then(|data_context| SetterViewModelRef::from_boxed(&data_context));

        if let Some(setter_vm) = setter_vm {
            main_vm.select_property(setter_vm.property());

            if let Some(selected_property) = main_vm.selected_property() {
                // `_dataGrid.ScrollIntoView(mainVm.SelectedProperty, null)`: the row of the
                // property, brought into view by the presenter of the rows.
                let row_index = main_vm
                    .properties_view()
                    .map(|properties_view| properties_view.row_index_of(&selected_property))
                    .unwrap_or(-1);
                if row_index >= 0 {
                    if let Some(rows_presenter) = self.data_grid().rows_presenter() {
                        rows_presenter.bring_into_view(row_index, None);
                    }
                }
            }
        }
    }
}
