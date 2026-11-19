//! Builder methods for what the generator cannot see: values of classes that
//! are not registered properties.

use crate::builder::El;
use ferroui_base::{ferro_property, AttachedProperty, FerroProperty, ObjectType, Upcast};
use ferroui_controls::{ColumnDefinition, ColumnDefinitions, Grid, GridLength, RowDefinition, RowDefinitions};

/// The rows and columns of a grid, in the text form XAML takes
/// (`"Auto,*,2*,40"`).
///
/// The text is a value of the element ([`GridDefinitionsText`]): a component that runs again
/// with another text, or a text that follows state
/// (`.set(GridDefinitionsText::columns_property(), move || ..)`), gives the grid the new sizes.
pub trait GridDefinitions {
    /// `Grid.RowDefinitions`.
    fn row_definitions(self, rows: &str) -> Self;
    /// `Grid.ColumnDefinitions`.
    fn column_definitions(self, columns: &str) -> Self;
}

impl<T: ObjectType + Upcast<Grid>> GridDefinitions for El<T> {
    fn row_definitions(self, rows: &str) -> Self {
        lengths(rows, "row");
        self.set(GridDefinitionsText::rows_property(), rows)
    }

    fn column_definitions(self, columns: &str) -> Self {
        lengths(columns, "column");
        self.set(GridDefinitionsText::columns_property(), columns)
    }
}

/// The definitions of a grid in the text form, as values of the grid: what the builders of
/// [`GridDefinitions`] set. An addition of the declarative layer (no upstream counterpart).
///
/// A new text gives the grid its sizes. Where the grid has as many definitions as the text names,
/// each of the kind (pixels, auto, stars) the text gives it, the definitions the grid has take
/// the new sizes: the collection and its definitions stay, and nothing enters or leaves the tree
/// of the grid (docs/porting/desktop-performance.md, part (b) of #803). Otherwise the grid is
/// given a new collection, as `Grid::set_column_definitions` does. The description of the element
/// is a text and holds no definition, so no one but the grid knows the definitions apart; a
/// collection given to the grid directly (`Grid::set_column_definitions`) is always the one the
/// grid then has.
pub struct GridDefinitionsText;

ferroui_base::ferro_static_type!(GridDefinitionsText);

ferroui_base::ferro_properties! { impl GridDefinitionsText {
    ferro_property!(
        /// The row definitions of a grid in the text form (`"Auto,*,2*,40"`).
        pub fn rows_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<GridDefinitionsText, Grid, _>("Rows", None)
        }
    );

    ferro_property!(
        /// The column definitions of a grid in the text form (`"Auto,*,2*,40"`).
        pub fn columns_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<GridDefinitionsText, Grid, _>("Columns", None)
        }
    );
} }

impl GridDefinitionsText {
    fn static_constructor() {
        Self::rows_property().changed().add_class_handler::<Grid>(|grid, _| {
            let text = grid.get_value(Self::rows_property());
            let lengths = lengths(text.as_deref().unwrap_or(""), "row");
            let rows = grid.row_definitions();
            if !take_sizes(&rows.snapshot(), &lengths, RowDefinition::height, RowDefinition::set_height) {
                let rows = RowDefinitions::from_items(lengths.into_iter().map(RowDefinition::with_height));
                grid.set_row_definitions(rows);
            }
        })
        .detach();
        Self::columns_property().changed().add_class_handler::<Grid>(|grid, _| {
            let text = grid.get_value(Self::columns_property());
            let lengths = lengths(text.as_deref().unwrap_or(""), "column");
            let columns = grid.column_definitions();
            if !take_sizes(&columns.snapshot(), &lengths, ColumnDefinition::width, ColumnDefinition::set_width) {
                grid.set_column_definitions(ColumnDefinitions::from_items(
                    lengths.into_iter().map(ColumnDefinition::with_width),
                ));
            }
        })
        .detach();
    }
}

/// The sizes of a text of definitions.
fn lengths(text: &str, kind: &str) -> Vec<GridLength> {
    GridLength::parse_lengths(text).unwrap_or_else(|_| panic!("`{text}` are not {kind} definitions"))
}

/// Gives `definitions` the sizes `lengths` where they are as many and each of the kind of its
/// size; tells whether it did.
fn take_sizes<D>(
    definitions: &[Ref<D>],
    lengths: &[GridLength],
    size: fn(&D) -> GridLength,
    set_size: fn(&D, GridLength),
) -> bool
where
    D: ObjectType,
{
    let same_kinds = definitions.len() == lengths.len()
        && definitions
            .iter()
            .zip(lengths)
            .all(|(definition, length)| size(definition).grid_unit_type() == length.grid_unit_type());
    if !same_kinds {
        return false;
    }
    for (definition, length) in definitions.iter().zip(lengths) {
        if size(definition) != *length {
            set_size(definition, *length);
        }
    }
    true
}

use ferroui_base::Ref;
