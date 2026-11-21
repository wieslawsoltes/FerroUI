//! Typed declarations for the nonvisual columns of a TableView.
use ferroui_base::{
    Ref, collections::FerroList, data::ReflectionBinding, layout::HorizontalAlignment,
    styling::ControlTheme,
};
use ferroui_controls::{AssignedBinding, GridLength, TableViewColumn, templates::IDataTemplate};
use std::rc::Rc;

/// Builds a single native column. Assign the result through [`table_columns`].
#[must_use]
pub struct TableViewColumnBuilder(Ref<TableViewColumn>);

/// Start a column with the native defaults for width, visibility and resizing.
pub fn table_column() -> TableViewColumnBuilder {
    TableViewColumnBuilder(TableViewColumn::new())
}

/// Build an ordered collection for `TableViewProps::columns` or the `columns` RSX field.
pub fn table_columns(
    columns: impl IntoIterator<Item = TableViewColumnBuilder>,
) -> FerroList<Ref<TableViewColumn>> {
    FerroList::from_items(columns.into_iter().map(TableViewColumnBuilder::build))
}

impl TableViewColumnBuilder {
    pub fn header(self, text: impl Into<String>) -> Self {
        self.0.set_header(Some(Rc::new(text.into())));
        self
    }

    pub fn width(self, width: GridLength) -> Self {
        self.0.set_width(width);
        self
    }

    pub fn is_visible(self, visible: bool) -> Self {
        self.0.set_is_visible(visible);
        self
    }

    pub fn can_user_resize(self, resize: bool) -> Self {
        self.0.set_can_user_resize(Some(resize));
        self
    }

    pub fn horizontal_content_alignment(self, alignment: HorizontalAlignment) -> Self {
        self.0.set_horizontal_content_alignment(alignment);
        self
    }

    /// Use a reflection binding to read a property of the row object.
    pub fn binding_path(self, path: impl AsRef<str>) -> Self {
        self.binding(AssignedBinding::new(ReflectionBinding::new(path.as_ref())))
    }

    pub fn binding(self, binding: AssignedBinding) -> Self {
        self.0.set_binding(Some(binding));
        self
    }

    pub fn cell_template(self, template: Rc<dyn IDataTemplate>) -> Self {
        self.0.set_cell_template(Some(template));
        self
    }

    pub fn header_template(self, template: Rc<dyn IDataTemplate>) -> Self {
        self.0.set_header_template(Some(template));
        self
    }

    pub fn cell_theme(self, theme: Ref<ControlTheme>) -> Self {
        self.0.set_cell_theme(theme);
        self
    }

    pub fn header_theme(self, theme: Ref<ControlTheme>) -> Self {
        self.0.set_header_theme(theme);
        self
    }

    pub fn build(self) -> Ref<TableViewColumn> {
        self.0
    }
}
