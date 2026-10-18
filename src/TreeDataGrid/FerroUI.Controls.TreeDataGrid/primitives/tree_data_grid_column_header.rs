//! `Primitives/TreeDataGridColumnHeader.cs`: the header of a column: its header content, sort
//! direction and the resizer that sets the column's width.

use super::INaturalWidthMeasureCache;
use crate::models::{ColumnRef, IColumns};
use crate::TreeDataGrid;
#[allow(unused_imports)]
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::input::{InputElement, InputElementImpl, VectorEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledElement, StyledElementImpl, Thickness, VisualImpl,
    WeakRef,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::metadata::PseudoClassesAttribute;
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, Thumb};
use ferroui_controls::{Button, ButtonImpl, ContentControlImpl, ControlImpl, GridLength, GridUnitType};
use ferroui_treedatagrid_core::models::ListSortDirection;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The header of a column of a [`TreeDataGrid`].
#[repr(C)]
pub struct TreeDataGridColumnHeader {
    base: Button,
    can_user_resize: Cell<bool>,
    columns: RefCell<Option<Rc<dyn IColumns>>>,
    header: RefCell<Option<BoxedValue>>,
    /// The column the header is realized for, and the token of the handler of its property
    /// changes.
    model: RefCell<Option<(ColumnRef, u64)>>,
    /// The column of the data context, when the data context is a column, and the token of the
    /// handler of its property changes.
    data_context_model: RefCell<Option<(ColumnRef, u64)>>,
    sort_direction: Cell<Option<ListSortDirection>>,
    /// The control the header is in the template of, and the subscription to its property changes.
    owner: RefCell<Option<(WeakRef<TreeDataGrid>, Rc<dyn IDisposable>)>>,
    /// The resizer of the template, and the tokens of its handlers.
    resizer: RefCell<Option<(Ref<Thumb>, RoutedEventHandlerToken, RoutedEventHandlerToken)>>,
    column_index: Cell<i32>,
    natural_desired_size: Cell<Option<Size>>,
}

ferro_class!(TreeDataGridColumnHeader: Button);
ferroui_base::ferro_class_info!(TreeDataGridColumnHeader { new: TreeDataGridColumnHeader::new });
ferroui_base::ferro_impl_classes!(
    TreeDataGridColumnHeader: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ContentControlImpl,
    ButtonImpl
);

ferroui_base::ferro_properties! { impl TreeDataGridColumnHeader {
    ferro_property!(
        /// Defines the `CanUserResize` property.
        pub fn can_user_resize_property() -> DirectProperty<TreeDataGridColumnHeader, bool> {
            FerroProperty::register_direct::<TreeDataGridColumnHeader, _>(
                "CanUserResize",
                |o| o.can_user_resize(),
                None,
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `Header` property.
        pub fn header_property() -> DirectProperty<TreeDataGridColumnHeader, Option<BoxedValue>> {
            FerroProperty::register_direct::<TreeDataGridColumnHeader, _>("Header", |o| o.header(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `SortDirection` property.
        pub fn sort_direction_property() -> DirectProperty<TreeDataGridColumnHeader, Option<ListSortDirection>> {
            FerroProperty::register_direct::<TreeDataGridColumnHeader, _>(
                "SortDirection",
                |o| o.sort_direction(),
                None,
                None,
            )
        }
    );
} }

impl FerroObjectImpl for TreeDataGridColumnHeader {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::can_user_resize_property().as_property() {
            this.pseudo_classes().set(":resizable", change.get_new_value::<bool>());
        } else if change.property() == StyledElement::data_context_property().as_property() {
            let new_model = change
                .get_new_value::<Option<BoxedValue>>()
                .and_then(|value| value.downcast_ref::<ColumnRef>().cloned());

            let old_subscription = this.data_context_model.borrow_mut().take();
            if let Some((old_model, token)) = old_subscription {
                old_model.property_changed().remove(token);
            }

            if let Some(new_model) = new_model {
                let token = this.subscribe_to_model(&new_model);
                *this.data_context_model.borrow_mut() = Some((new_model, token));
            }

            this.update_properties_from_model();
        } else if change.property() == StyledElement::parent_property().as_property() {
            let old_owner = this.owner.borrow_mut().take();
            if let Some((_, subscription)) = old_owner {
                subscription.dispose();
            }

            let owner = change
                .get_new_value::<Option<Ref<StyledElement>>>()
                .and_then(|parent| parent.templated_parent())
                .and_then(|templated_parent| templated_parent.cast::<TreeDataGrid>());

            if let Some(owner) = owner {
                let weak = this.to_ref().downgrade();
                let subscription = owner.property_changed(move |e| {
                    if let Some(this) = weak.upgrade() {
                        this.on_owner_property_changed(e);
                    }
                });
                *this.owner.borrow_mut() = Some((owner.downgrade(), subscription));
            }

            this.update_properties_from_model();
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl LayoutableImpl for TreeDataGridColumnHeader {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let result = Self::parent_measure_override(this, available_size);

        // HACKFIX for #83 of the reference. Seems that cells are getting truncated at times due to
        // DPI scaling; a pixel is added to the header size.
        result.inflate(Thickness::new(1.0, 0.0, 1.0, 0.0))
    }

    fn on_measure_invalidated(this: &Self) {
        this.natural_desired_size.set(None);
        Self::parent_on_measure_invalidated(this);
    }
}

impl ControlImpl for TreeDataGridColumnHeader {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        crate::automation::peers::TreeDataGridColumnHeaderAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for TreeDataGridColumnHeader {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let previous = this.resizer.borrow_mut().take();
        if let Some((resizer, drag_delta, double_tapped)) = previous {
            resizer.remove_handler(Thumb::drag_delta_event(), drag_delta);
            resizer.remove_handler(InputElement::double_tapped_event(), double_tapped);
        }

        if let Some(resizer) = e.name_scope().find_as::<Thumb>("PART_Resizer") {
            let weak = this.to_ref().downgrade();
            let drag_delta = resizer.add_handler(Thumb::drag_delta_event(), move |_, e: &VectorEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.resizer_drag_delta(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let double_tapped = resizer.add_handler(InputElement::double_tapped_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.resizer_double_tapped();
                }
            });
            *this.resizer.borrow_mut() = Some((resizer, drag_delta, double_tapped));
        }
    }
}

impl INaturalWidthMeasureCache for TreeDataGridColumnHeader {
    fn natural_desired_size(&self) -> Option<Size> {
        self.natural_desired_size.get()
    }

    fn set_natural_desired_size(&self, value: Option<Size>) {
        self.natural_desired_size.set(value);
    }
}

impl TreeDataGridColumnHeader {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":resizable"]);

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
            can_user_resize: Cell::new(false),
            columns: RefCell::new(None),
            header: RefCell::new(None),
            model: RefCell::new(None),
            data_context_model: RefCell::new(None),
            sort_direction: Cell::new(None),
            owner: RefCell::new(None),
            resizer: RefCell::new(None),
            column_index: Cell::new(0),
            natural_desired_size: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the user can resize the column.
    pub fn can_user_resize(&self) -> bool {
        self.can_user_resize.get()
    }

    fn set_can_user_resize(&self, value: bool) {
        self.set_and_raise_cell(Self::can_user_resize_property(), &self.can_user_resize, value);
    }

    /// The index of the column.
    pub fn column_index(&self) -> i32 {
        self.column_index.get()
    }

    /// The header of the column.
    pub fn header(&self) -> Option<BoxedValue> {
        self.header.borrow().clone()
    }

    fn set_header(&self, value: Option<BoxedValue>) {
        self.set_and_raise(Self::header_property(), &self.header, value);
    }

    /// The sort direction of the column, if it is sorted.
    pub fn sort_direction(&self) -> Option<ListSortDirection> {
        self.sort_direction.get()
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.set_and_raise_cell(Self::sort_direction_property(), &self.sort_direction, value);
    }

    /// The column the header is realized for.
    pub fn model(&self) -> Option<ColumnRef> {
        self.model.borrow().as_ref().map(|(model, _)| model.clone())
    }

    /// Realizes the header for the column at `column_index` of `columns`.
    pub fn realize(&self, columns: &Rc<dyn IColumns>, column_index: i32) {
        if self.model.borrow().is_some() {
            panic!("Column header is already realized.");
        }

        *self.columns.borrow_mut() = Some(columns.clone());
        let model = columns.get(column_index as usize);
        let token = self.subscribe_to_model(&model);
        *self.model.borrow_mut() = Some((model, token));
        self.column_index.set(column_index);
        self.natural_desired_size.set(None);
        self.update_properties_from_model();
    }

    /// The index of the column changed.
    pub fn update_column_index(&self, column_index: i32) {
        self.column_index.set(column_index);
    }

    /// Unrealizes the header.
    pub fn unrealize(&self) {
        let model = self.model.borrow_mut().take();
        if let Some((model, token)) = model {
            model.property_changed().remove(token);
        }

        *self.columns.borrow_mut() = None;
        self.column_index.set(-1);
        self.natural_desired_size.set(None);
        self.update_properties_from_model();
    }

    fn subscribe_to_model(&self, model: &ColumnRef) -> u64 {
        let weak = self.to_ref().downgrade();
        model.property_changed().add(Rc::new(move |property_name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_model_property_changed(property_name);
            }
        }))
    }

    fn owner(&self) -> Option<Ref<TreeDataGrid>> {
        self.owner.borrow().as_ref().and_then(|(owner, _)| owner.upgrade())
    }

    fn resizer_double_tapped(&self) {
        let columns = self.columns.borrow().clone();
        if let Some(columns) = columns {
            columns.set_column_width(self.column_index.get(), GridLength::AUTO);
        }
    }

    fn on_model_property_changed(&self, property_name: &str) {
        if property_name == "CanUserResize" || property_name == "Header" || property_name == "SortDirection" {
            self.update_properties_from_model();
        }
    }

    fn on_owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(owner) = self.owner() else { return };

        if e.property() == TreeDataGrid::can_user_resize_columns_property().as_property() {
            let model_value = self.model().and_then(|model| model.can_user_resize());
            self.set_can_user_resize(model_value.unwrap_or_else(|| owner.can_user_resize_columns()));
        }
    }

    fn resizer_drag_delta(&self, e: &VectorEventArgs) {
        let columns = self.columns.borrow().clone();
        let (Some(columns), Some(model)) = (columns, self.model()) else { return };

        if MathUtilities::is_zero(e.vector.x) {
            return;
        }

        let pixel_width = if model.width().is_absolute() { model.width().value() } else { self.bounds().width };

        if pixel_width.is_nan() || pixel_width.is_infinite() || pixel_width + e.vector.x < 0.0 {
            return;
        }

        let width = GridLength::new(pixel_width + e.vector.x, GridUnitType::Pixel);
        columns.set_column_width(self.column_index.get(), width);
    }

    fn update_properties_from_model(&self) {
        let model = self.model();
        let can_user_resize = model
            .as_ref()
            .and_then(|model| model.can_user_resize())
            .or_else(|| self.owner().map(|owner| owner.can_user_resize_columns()))
            .unwrap_or(false);
        self.set_can_user_resize(can_user_resize);
        self.set_header(model.as_ref().and_then(|model| model.header()));
        self.set_sort_direction(model.as_ref().and_then(|model| model.sort_direction()));
    }
}
