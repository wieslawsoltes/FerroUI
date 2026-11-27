use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, Thumb};
use crate::utils::debug_display::{append_optional_boxed_value, build_base_debug_display};
use crate::{
    ContentControl, ContentControlImpl, ControlImpl, GridLength, GridUnitType, TableViewColumn, TableViewSortDirection, ToolTip,
};
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, MouseButton, PointerCaptureLostEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, VectorEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroProperty, Ref, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};

const PART_RESIZER: &str = "PART_Resizer";

/// Represents the header of a [`TableViewColumn`].
#[repr(C)]
pub struct TableViewColumnHeader {
    base: ContentControl,
    column: RefCell<Option<Ref<TableViewColumn>>>,
    /// The resizer of the template and the handler of its drags.
    resizer: RefCell<Option<(Ref<Thumb>, RoutedEventHandlerToken)>>,
    /// Whether the left button was pressed on a header the user can sort by (WinUI's sorting).
    is_sort_pressed: Cell<bool>,
    /// Whether the tool tip is the one of the column (`HeaderToolTip`).
    has_column_tool_tip: Cell<bool>,
}

ferro_class!(TableViewColumnHeader: ContentControl);
ferro_class_info!(TableViewColumnHeader {
    new: TableViewColumnHeader::new,
    markup: {
        attributes: [
            TemplatePart("PART_Resizer", type(Ref<Thumb>)),
            PseudoClasses(":sortable", ":sortascending", ":sortdescending", ":pressed"),
        ],
    },
});
ferro_impl_classes!(
    TableViewColumnHeader: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    ContentControlImpl
);

impl InputElementImpl for TableViewColumnHeader {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        // A press on the resizer is handled by its thumb.
        let sortable = this.column().is_some_and(|column| column.can_user_effectively_sort());
        if !e.handled() && sortable && e.get_current_point(Some(this)).properties.is_left_button_pressed {
            this.is_sort_pressed.set(true);
            this.pseudo_classes().set(":pressed", true);
            e.set_handled(true);
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if this.is_sort_pressed.replace(false) && e.initial_press_mouse_button() == MouseButton::Left {
            this.pseudo_classes().set(":pressed", false);
            e.set_handled(true);

            let position = e.get_position(Some(this));
            let bounds = this.bounds();
            let is_over = position.x >= 0.0 && position.y >= 0.0 && position.x <= bounds.width && position.y <= bounds.height;
            if is_over {
                if let Some(column) = this.column() {
                    if let Some(table_view) = column.table_view() {
                        table_view.on_column_header_clicked(&column);
                    }
                }
            }
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);

        if this.is_sort_pressed.replace(false) {
            this.pseudo_classes().set(":pressed", false);
        }
    }
}

impl TemplatedControlImpl for TableViewColumnHeader {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if let Some((resizer, token)) = this.resizer.take() {
            resizer.remove_handler(Thumb::drag_delta_event(), token);
        }

        if let Some(resizer) = e.name_scope().find_as::<Thumb>(PART_RESIZER) {
            let weak = this.to_ref().downgrade();
            let token = resizer.drag_delta(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_resizer_drag_delta(e);
                }
            });
            *this.resizer.borrow_mut() = Some((resizer, token));
        }
    }
}

ferro_properties! {
    impl TableViewColumnHeader {
        /// Identifies the `Column` property.
        pub fn column_property() -> DirectProperty<TableViewColumnHeader, Option<Ref<TableViewColumn>>> {
            FerroProperty::register_direct::<TableViewColumnHeader, _>("Column", |o| o.column(), None, None)
        }
    }
}

impl TableViewColumnHeader {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            column: RefCell::new(None),
            resizer: RefCell::new(None),
            is_sort_pressed: Cell::new(false),
            has_column_tool_tip: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the column associated with this header.
    pub fn column(&self) -> Option<Ref<TableViewColumn>> {
        self.column.borrow().clone()
    }

    pub(crate) fn set_column(&self, value: Option<Ref<TableViewColumn>>) {
        let old_value = self.column();
        if !self.set_and_raise(Self::column_property(), &self.column, value.clone()) {
            return;
        }

        if old_value.is_some() {
            self.clear_properties();
        }

        if let Some(value) = value {
            self.set_properties(&value);
        }
    }

    fn on_resizer_drag_delta(&self, e: &VectorEventArgs) {
        let Some(column) = self.column().filter(|column| column.can_user_effectively_resize()) else { return };

        let actual_width = column.actual_width();
        if actual_width.is_nan() {
            return;
        }

        let resizer = self.resizer.borrow().as_ref().map(|(resizer, _)| resizer.clone());
        let min_width = resizer.map_or(0.0, |resizer| resizer.bounds().width);
        let new_width = max(min_width, actual_width + e.vector.x);
        column.set_width(GridLength::new(new_width, GridUnitType::Pixel));
    }

    /// Appends the text that describes the header in diagnostics: the name
    /// of its class, its name and, with the content, its content and the
    /// header of its column.
    pub(crate) fn build_debug_display(&self, builder: &mut String, include_content: bool) {
        build_base_debug_display(self, builder);

        if include_content {
            // DEBUG-DISPLAY-SEAM: the content is the part of the description
            // the content control adds in the reference; it has no debug
            // display of its own here.
            append_optional_boxed_value(builder, "Content", self.content().as_ref(), include_content);

            let header = self.column().and_then(|column| column.header());
            append_optional_boxed_value(builder, "Column", header.as_ref(), include_content);
        }
    }

    fn clear_properties(&self) {
        self.update_sort_pseudo_classes(None);
        if self.has_column_tool_tip.replace(false) {
            self.clear_value(ToolTip::tip_property());
        }
        self.clear_value(Visual::is_visible_property());
        self.clear_value(StyledElement::theme_property());
        self.clear_value(ContentControl::horizontal_content_alignment_property());
        self.clear_value(ContentControl::content_template_property());
        self.clear_value(ContentControl::content_property());
    }

    fn set_properties(&self, column: &TableViewColumn) {
        self.set_value(Visual::is_visible_property(), column.is_visible());

        match column.header_theme() {
            Some(header_theme) => self.set_value(StyledElement::theme_property(), Some(header_theme)),
            None => self.clear_value(StyledElement::theme_property()),
        }

        self.set_value(ContentControl::horizontal_content_alignment_property(), column.horizontal_content_alignment());

        match column.header_template() {
            Some(header_template) => self.set_value(ContentControl::content_template_property(), Some(header_template)),
            None => self.clear_value(ContentControl::content_template_property()),
        }

        match column.header() {
            Some(header) => self.set_value(ContentControl::content_property(), Some(header)),
            None => self.clear_value(ContentControl::content_property()),
        }

        self.update_sort_pseudo_classes(Some(column));

        match column.header_tool_tip() {
            Some(tip) => {
                self.set_value(ToolTip::tip_property(), Some(tip));
                self.has_column_tool_tip.set(true);
            }
            None if self.has_column_tool_tip.replace(false) => self.clear_value(ToolTip::tip_property()),
            None => {}
        }
    }

    /// The pseudo-classes of WinUI's sorting: `:sortable` for a column the user can sort by,
    /// `:sortascending` and `:sortdescending` for the direction the rows are sorted by it in.
    fn update_sort_pseudo_classes(&self, column: Option<&TableViewColumn>) {
        let sortable = column.is_some_and(TableViewColumn::can_user_effectively_sort);
        let direction = column.map_or(TableViewSortDirection::None, TableViewColumn::sort_direction);
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(":sortable", sortable);
        pseudo_classes.set(":sortascending", direction == TableViewSortDirection::Ascending);
        pseudo_classes.set(":sortdescending", direction == TableViewSortDirection::Descending);
    }

    pub(crate) fn refresh(&self) {
        if let Some(column) = self.column() {
            self.set_properties(&column);
        }
    }
}
