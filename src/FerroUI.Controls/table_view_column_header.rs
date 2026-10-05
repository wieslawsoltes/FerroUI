use crate::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, Thumb};
use crate::utils::debug_display::{append_optional_boxed_value, build_base_debug_display};
use crate::{ContentControl, ContentControlImpl, ControlImpl, GridLength, GridUnitType, TableViewColumn};
use ferroui_base::input::{InputElementImpl, VectorEventArgs};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, DirectProperty, FerroObjectImpl,
    FerroProperty, Ref, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::RefCell;

const PART_RESIZER: &str = "PART_Resizer";

/// Represents the header of a [`TableViewColumn`].
#[repr(C)]
pub struct TableViewColumnHeader {
    base: ContentControl,
    column: RefCell<Option<Ref<TableViewColumn>>>,
    /// The resizer of the template and the handler of its drags.
    resizer: RefCell<Option<(Ref<Thumb>, RoutedEventHandlerToken)>>,
}

ferro_class!(TableViewColumnHeader: ContentControl);
ferro_class_info!(TableViewColumnHeader {
    new: TableViewColumnHeader::new,
    markup: {
        attributes: [TemplatePart("PART_Resizer", type(Ref<Thumb>))],
    },
});
ferro_impl_classes!(
    TableViewColumnHeader: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    ContentControlImpl
);

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
        Self { base: ContentControl::construct(), column: RefCell::new(None), resizer: RefCell::new(None) }
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
    }

    pub(crate) fn refresh(&self) {
        if let Some(column) = self.column() {
            self.set_properties(&column);
        }
    }
}
