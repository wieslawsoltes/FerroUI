use crate::i_headered::{register_headered, IHeadered};
use crate::primitives::HeaderedContentControl;
use crate::templates::IDataTemplate;
use crate::utils::debug_display::{append_optional_boxed_value, build_base_debug_display};
use crate::{AssignedBinding, ContentControl, GridLength, GridUnitType, TableView};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StyledElement, StyledElementImpl,
    StyledProperty, StyledPropertyMetadata, StyledPropertyOptions, Visual, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines a column in a [`TableView`].
#[repr(C)]
pub struct TableViewColumn {
    base: StyledElement,
    /// The table view the column belongs to. The column does not own it (it
    /// is its logical parent), so the reference is weak.
    table_view: RefCell<Option<WeakRef<TableView>>>,
    actual_width: Cell<f64>,
    can_user_effectively_resize: Cell<bool>,
}

ferro_class!(TableViewColumn: StyledElement);
ferro_class_info!(TableViewColumn {
    new: TableViewColumn::new,
    markup: {
        property_attributes: [
            CellTemplate: [InheritDataTypeFromItems("ItemsSource", AncestorType = type(Ref<TableView>))],
            Binding: [InheritDataTypeFromItems("ItemsSource", AncestorType = type(Ref<TableView>))],
        ],
    },
});
ferro_impl_classes!(TableViewColumn: StyledElementImpl);

impl FerroObjectImpl for TableViewColumn {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::width_property().as_property() || property == Self::is_visible_property().as_property()
        {
            if let Some(table_view) = this.table_view() {
                table_view.on_columns_size_changed();
            }
        }

        if property == Self::can_user_resize_property().as_property() {
            this.update_can_user_effectively_resize();
        } else {
            if Self::is_header_property(property) {
                if let Some(table_view) = this.table_view() {
                    table_view.refresh_column_headers(&this.to_ref());
                }
            }
            if Self::is_cell_property(property) {
                if let Some(table_view) = this.table_view() {
                    table_view.refresh_column_cells(&this.to_ref());
                }
            }
        }
    }
}

ferro_properties! {
    impl TableViewColumn {
        /// Defines the `IsVisible` property.
        pub fn is_visible_property() -> StyledProperty<bool> {
            Visual::is_visible_property().add_owner::<TableViewColumn>()
        }

        /// Defines the `HeaderTheme` property.
        pub fn header_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<TableViewColumn, _>("HeaderTheme", None)
        }

        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            HeaderedContentControl::header_template_property().add_owner::<TableViewColumn>()
        }

        /// Defines the `Header` property.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            HeaderedContentControl::header_property().add_owner::<TableViewColumn>()
        }

        /// Defines the `Width` property.
        pub fn width_property() -> StyledProperty<GridLength> {
            FerroProperty::register::<TableViewColumn, _>("Width", GridLength::new(1.0, GridUnitType::Star))
        }

        /// Defines the `CellTheme` property.
        pub fn cell_theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<TableViewColumn, _>("CellTheme", None)
        }

        /// Defines the `CellTemplate` property.
        pub fn cell_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TableViewColumn, _>("CellTemplate", None)
        }

        /// Defines the `Binding` property.
        pub fn binding_property() -> StyledProperty<Option<AssignedBinding>> {
            FerroProperty::register_with::<TableViewColumn, _>(
                "Binding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }

        /// Defines the `CanUserResize` property.
        pub fn can_user_resize_property() -> StyledProperty<Option<bool>> {
            FerroProperty::register::<TableViewColumn, _>("CanUserResize", None)
        }

        /// Defines the `HorizontalContentAlignment` property.
        pub fn horizontal_content_alignment_property() -> StyledProperty<HorizontalAlignment> {
            ContentControl::horizontal_content_alignment_property()
                .add_owner_with::<TableViewColumn>(StyledPropertyMetadata::new(Some(HorizontalAlignment::Left)))
        }

        /// Defines the `TableView` property.
        ///
        /// The column does not own its table view: the value is held weakly
        /// and read as a handle.
        pub fn table_view_property() -> DirectProperty<TableViewColumn, Option<Ref<TableView>>> {
            FerroProperty::register_direct::<TableViewColumn, _>("TableView", |o| o.table_view(), None, None)
        }

        /// Defines the `ActualWidth` property.
        pub fn actual_width_property() -> DirectProperty<TableViewColumn, f64> {
            FerroProperty::register_direct::<TableViewColumn, _>("ActualWidth", |o| o.actual_width(), None, 0.0)
        }

        /// Defines the `CanUserEffectivelyResize` property.
        pub fn can_user_effectively_resize_property() -> DirectProperty<TableViewColumn, bool> {
            FerroProperty::register_direct::<TableViewColumn, _>(
                "CanUserEffectivelyResize",
                |o| o.can_user_effectively_resize(),
                None,
                false,
            )
        }
    }
}

impl TableViewColumn {
    fn static_constructor() {
        register_headered::<TableViewColumn>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            table_view: RefCell::new(None),
            actual_width: Cell::new(f64::NAN),
            can_user_effectively_resize: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether the column is visible. The default is true.
    pub fn is_visible(&self) -> bool {
        self.get_value(Self::is_visible_property())
    }

    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value)
    }

    /// Gets or sets the theme to apply to the header. It must target
    /// [`TableViewColumnHeader`](crate::TableViewColumnHeader).
    pub fn header_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::header_theme_property())
    }

    pub fn set_header_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::header_theme_property(), value.into().0)
    }

    /// Gets or sets the data template used to display the header.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    /// Gets or sets the column header content.
    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    /// Gets or sets the column width. Supports pixel, star (*) and auto
    /// sizing.
    pub fn width(&self) -> GridLength {
        self.get_value(Self::width_property())
    }

    pub fn set_width(&self, value: GridLength) {
        self.set_value(Self::width_property(), value)
    }

    /// Gets or sets the theme to apply to the cells. It must target
    /// [`TableViewCell`](crate::TableViewCell).
    pub fn cell_theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::cell_theme_property())
    }

    pub fn set_cell_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::cell_theme_property(), value.into().0)
    }

    /// Gets or sets the data template used to display cell content. When
    /// set, the whole row data item is passed as the template's data
    /// context.
    ///
    /// This property takes priority over [`binding`](Self::binding).
    pub fn cell_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::cell_template_property())
    }

    pub fn set_cell_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::cell_template_property(), value)
    }

    /// Gets or sets a binding that retrieves the cell value from the row
    /// data item. Use this for simple property display, such as a binding to
    /// the `Name` property of the items.
    pub fn binding(&self) -> Option<AssignedBinding> {
        self.get_value(Self::binding_property())
    }

    pub fn set_binding(&self, value: Option<AssignedBinding>) {
        self.set_value(Self::binding_property(), value)
    }

    /// Gets or sets whether the column can be resized. Set to `None` to use
    /// the value of [`TableView::can_user_resize_columns`]. The default is
    /// `None`.
    pub fn can_user_resize(&self) -> Option<bool> {
        self.get_value(Self::can_user_resize_property())
    }

    pub fn set_can_user_resize(&self, value: Option<bool>) {
        self.set_value(Self::can_user_resize_property(), value)
    }

    /// Gets or sets the horizontal alignment of the content within a cell.
    pub fn horizontal_content_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_content_alignment_property())
    }

    pub fn set_horizontal_content_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_content_alignment_property(), value)
    }

    /// Gets the [`TableView`] associated with this column.
    pub fn table_view(&self) -> Option<Ref<TableView>> {
        self.table_view.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn set_table_view(&self, value: Option<Ref<TableView>>) {
        let old_value = self.table_view();
        if old_value == value {
            return;
        }

        *self.table_view.borrow_mut() = value.as_ref().map(Ref::downgrade);
        self.raise_direct_property_changed(Self::table_view_property(), &old_value, &value);
        self.update_can_user_effectively_resize();
    }

    /// Gets the actual width of the column, in device independent pixels.
    /// If the column hasn't yet been measured, returns NaN.
    pub fn actual_width(&self) -> f64 {
        self.actual_width.get()
    }

    pub(crate) fn set_actual_width(&self, value: f64) {
        // A NaN equals a NaN for the change detection of the reference.
        if value.is_nan() && self.actual_width.get().is_nan() {
            return;
        }

        self.set_and_raise_cell(Self::actual_width_property(), &self.actual_width, value);
    }

    /// Gets whether the column can be effectively resized. The value of
    /// this property depends on both [`can_user_resize`](Self::can_user_resize)
    /// and [`TableView::can_user_resize_columns`].
    pub fn can_user_effectively_resize(&self) -> bool {
        self.can_user_effectively_resize.get()
    }

    pub(crate) fn set_can_user_effectively_resize(&self, value: bool) {
        self.set_and_raise_cell(
            Self::can_user_effectively_resize_property(),
            &self.can_user_effectively_resize,
            value,
        );
    }

    fn is_cell_property(property: &'static FerroProperty) -> bool {
        property == Self::cell_theme_property().as_property()
            || property == Self::cell_template_property().as_property()
            || property == Self::binding_property().as_property()
            || property == Self::horizontal_content_alignment_property().as_property()
            || property == Self::is_visible_property().as_property()
    }

    fn is_header_property(property: &'static FerroProperty) -> bool {
        property == Self::header_theme_property().as_property()
            || property == Self::header_template_property().as_property()
            || property == Self::header_property().as_property()
            || property == Self::horizontal_content_alignment_property().as_property()
            || property == Self::is_visible_property().as_property()
    }

    pub(crate) fn update_can_user_effectively_resize(&self) {
        let value = self
            .can_user_resize()
            .or_else(|| self.table_view().map(|table_view| table_view.can_user_resize_columns()))
            .unwrap_or(true);
        self.set_can_user_effectively_resize(value);
    }

    /// The text that describes the column in diagnostics.
    pub(crate) fn debug_display(&self) -> String {
        let mut builder = String::new();
        self.build_debug_display(&mut builder, true);
        builder
    }

    /// Appends the text that describes the column in diagnostics: the name
    /// of its class, its name and, with the content, its header.
    pub(crate) fn build_debug_display(&self, builder: &mut String, include_content: bool) {
        build_base_debug_display(self, builder);

        if include_content {
            append_optional_boxed_value(builder, "Header", self.header().as_ref(), include_content);
        }
    }
}

impl IHeadered for TableViewColumn {
    fn header(&self) -> Option<BoxedValue> {
        TableViewColumn::header(self)
    }

    fn set_header(&self, value: Option<BoxedValue>) {
        TableViewColumn::set_header(self, value)
    }
}
