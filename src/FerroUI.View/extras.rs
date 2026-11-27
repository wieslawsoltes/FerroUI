//! Builder methods for what the generator cannot see: values of classes that
//! are not registered properties.

use crate::builder::{El, IntoValue};
use ferroui_base::{ferro_property, AttachedProperty, FerroProperty, ObjectType, Upcast};
use ferroui_controls::{
    ColumnDefinition, ColumnDefinitions, Grid, GridLength, GridUnitType, RowDefinition, RowDefinitions,
};
use std::ops::Deref;
use std::rc::Rc;

/// A row or column that takes the size of its content: `Auto`.
#[inline]
pub const fn auto() -> GridLength {
    GridLength::AUTO
}

/// A row or column that takes `weight` shares of the space left: `2*` is `star(2.0)`.
///
/// # Panics
/// When `weight` is negative, NaN or infinite.
#[inline]
pub fn star(weight: f64) -> GridLength {
    GridLength::new(weight, GridUnitType::Star)
}

/// A row or column of a fixed size: `40` is `px(40.0)`.
///
/// # Panics
/// When `size` is negative, NaN or infinite.
#[inline]
pub fn px(size: f64) -> GridLength {
    GridLength::from_pixels(size)
}

/// The sizes of the rows or columns of a grid, shared by the elements that hold them and compared
/// by value.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLengths(Rc<[GridLength]>);

ferroui_base::styled_value!(GridLengths);

impl Deref for GridLengths {
    type Target = [GridLength];

    fn deref(&self) -> &[GridLength] {
        &self.0
    }
}

impl From<Vec<GridLength>> for GridLengths {
    fn from(lengths: Vec<GridLength>) -> Self {
        Self(lengths.into())
    }
}

impl From<&[GridLength]> for GridLengths {
    fn from(lengths: &[GridLength]) -> Self {
        Self(lengths.into())
    }
}

impl<const N: usize> From<[GridLength; N]> for GridLengths {
    fn from(lengths: [GridLength; N]) -> Self {
        Self(Rc::new(lengths))
    }
}

impl IntoValue<Option<GridLengths>> for Vec<GridLength> {
    fn into_value(self) -> Option<GridLengths> {
        Some(self.into())
    }
}

impl<const N: usize> IntoValue<Option<GridLengths>> for [GridLength; N] {
    fn into_value(self) -> Option<GridLengths> {
        Some(self.into())
    }
}

/// What [`GridDefinitions`] takes: typed lengths (`[auto(), star(1.0), px(40.0)]`), or the text
/// form of XAML (`"Auto,*,40"`). Only the text forms call the parser of lengths.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a list of row or column lengths",
    label = "the definitions of a grid go here",
    note = "write `[auto(), star(1.0), px(40.0)]`, or text such as `\"Auto,*,40\"`"
)]
pub trait IntoGridLengths {
    /// The lengths; `kind` (`row`, `column`) names them in the message of a text that does not
    /// parse.
    fn into_grid_lengths(self, kind: &str) -> GridLengths;
}

impl IntoGridLengths for GridLengths {
    fn into_grid_lengths(self, _kind: &str) -> GridLengths {
        self
    }
}

impl<const N: usize> IntoGridLengths for [GridLength; N] {
    fn into_grid_lengths(self, _kind: &str) -> GridLengths {
        self.into()
    }
}

impl IntoGridLengths for Vec<GridLength> {
    fn into_grid_lengths(self, _kind: &str) -> GridLengths {
        self.into()
    }
}

impl IntoGridLengths for &Vec<GridLength> {
    fn into_grid_lengths(self, _kind: &str) -> GridLengths {
        self.as_slice().into()
    }
}

impl IntoGridLengths for &[GridLength] {
    fn into_grid_lengths(self, _kind: &str) -> GridLengths {
        self.into()
    }
}

impl IntoGridLengths for &str {
    #[track_caller]
    fn into_grid_lengths(self, kind: &str) -> GridLengths {
        lengths(self, kind).into()
    }
}

impl IntoGridLengths for &String {
    #[track_caller]
    fn into_grid_lengths(self, kind: &str) -> GridLengths {
        lengths(self, kind).into()
    }
}

impl IntoGridLengths for String {
    #[track_caller]
    fn into_grid_lengths(self, kind: &str) -> GridLengths {
        lengths(&self, kind).into()
    }
}

/// The rows and columns of a grid: typed lengths (`[auto(), star(1.0), star(2.0), px(40.0)]`), or
/// the text form XAML takes (`"Auto,*,2*,40"`).
///
/// The lengths are a value of the element ([`GridDefinitionSizes`]): a component that runs again
/// with other lengths, or lengths that follow state
/// (`.set(GridDefinitionSizes::columns_property(), move || [star(weight.get()), auto()])`), give
/// the grid the new sizes. Text that follows state goes through
/// [`GridDefinitionsText`].
pub trait GridDefinitions {
    /// `Grid.RowDefinitions`.
    fn row_definitions(self, rows: impl IntoGridLengths) -> Self;
    /// `Grid.ColumnDefinitions`.
    fn column_definitions(self, columns: impl IntoGridLengths) -> Self;
}

impl<T: ObjectType + Upcast<Grid>> GridDefinitions for El<T> {
    #[track_caller]
    fn row_definitions(self, rows: impl IntoGridLengths) -> Self {
        let rows = rows.into_grid_lengths("row");
        self.set(GridDefinitionSizes::rows_property(), rows)
    }

    #[track_caller]
    fn column_definitions(self, columns: impl IntoGridLengths) -> Self {
        let columns = columns.into_grid_lengths("column");
        self.set(GridDefinitionSizes::columns_property(), columns)
    }
}

/// The definitions of a grid as typed lengths, as values of the grid: what the builders of
/// [`GridDefinitions`] set. An addition of the declarative layer (no upstream counterpart).
///
/// New lengths give the grid its sizes. Where the grid has as many definitions as there are
/// lengths, each of the kind (pixels, auto, stars) of its length, the definitions the grid has
/// take the new sizes: the collection and its definitions stay, and nothing enters or leaves the
/// tree of the grid (docs/porting/desktop-performance.md, part (b) of #803). Otherwise the grid is
/// given a new collection, as `Grid::set_column_definitions` does. The description of the element
/// holds lengths and no definition, so no one but the grid knows the definitions apart; a
/// collection given to the grid directly (`Grid::set_column_definitions`) is always the one the
/// grid then has.
pub struct GridDefinitionSizes;

ferroui_base::ferro_static_type!(GridDefinitionSizes);

ferroui_base::ferro_properties! { impl GridDefinitionSizes {
    ferro_property!(
        /// The sizes of the rows of a grid.
        pub fn rows_property() -> AttachedProperty<Option<GridLengths>> {
            FerroProperty::register_attached::<GridDefinitionSizes, Grid, _>("Rows", None)
        }
    );

    ferro_property!(
        /// The sizes of the columns of a grid.
        pub fn columns_property() -> AttachedProperty<Option<GridLengths>> {
            FerroProperty::register_attached::<GridDefinitionSizes, Grid, _>("Columns", None)
        }
    );
} }

impl GridDefinitionSizes {
    fn static_constructor() {
        // Lengths that are taken away leave the grid the lengths of its text, if it has one (an
        // element that went from typed lengths to text sets one and clears the other, in either
        // order).
        Self::rows_property().changed().add_class_handler::<Grid>(|grid, _| match grid.get_value(Self::rows_property()) {
            Some(lengths) => give_rows(grid, &lengths),
            None => give_rows(grid, &text_lengths(grid, GridDefinitionsText::rows_property(), "row")),
        })
        .detach();
        Self::columns_property().changed().add_class_handler::<Grid>(|grid, _| {
            match grid.get_value(Self::columns_property()) {
                Some(lengths) => give_columns(grid, &lengths),
                None => give_columns(grid, &text_lengths(grid, GridDefinitionsText::columns_property(), "column")),
            }
        })
        .detach();
    }
}

/// The definitions of a grid in the text form (`"Auto,*,2*,40"`), for text that follows state
/// (`.set(GridDefinitionsText::columns_property(), move || ..)`). A new text is parsed and gives
/// the grid its sizes as the lengths of [`GridDefinitionSizes`] do; the text is not copied into
/// them, so a text that follows state costs one parse and no other value of the grid. An addition
/// of the declarative layer (no upstream counterpart).
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
        // A text that is taken away leaves the grid its typed lengths, if it has them (an element
        // that went from text to typed lengths sets one and clears the other, in either order).
        Self::rows_property().changed().add_class_handler::<Grid>(|grid, _| {
            match grid.get_value(GridDefinitionSizes::rows_property()) {
                Some(lengths) if grid.get_value(Self::rows_property()).is_none() => give_rows(grid, &lengths),
                _ => give_rows(grid, &text_lengths(grid, Self::rows_property(), "row")),
            }
        })
        .detach();
        Self::columns_property().changed().add_class_handler::<Grid>(|grid, _| {
            match grid.get_value(GridDefinitionSizes::columns_property()) {
                Some(lengths) if grid.get_value(Self::columns_property()).is_none() => give_columns(grid, &lengths),
                _ => give_columns(grid, &text_lengths(grid, Self::columns_property(), "column")),
            }
        })
        .detach();
    }
}

/// The lengths of the text of a grid, none without one.
fn text_lengths(grid: &Grid, text: &'static AttachedProperty<Option<String>>, kind: &str) -> Vec<GridLength> {
    grid.get_value(text).map(|text| lengths(&text, kind)).unwrap_or_default()
}

/// Gives the rows of a grid these lengths: in place where they fit ([`take_sizes`]), otherwise a
/// new collection.
fn give_rows(grid: &Grid, lengths: &[GridLength]) {
    let rows = grid.row_definitions();
    if !take_sizes(&rows.snapshot(), lengths, RowDefinition::height, RowDefinition::set_height) {
        grid.set_row_definitions(RowDefinitions::from_items(lengths.iter().copied().map(RowDefinition::with_height)));
    }
}

/// Gives the columns of a grid these lengths, as [`give_rows`] does the rows.
fn give_columns(grid: &Grid, lengths: &[GridLength]) {
    let columns = grid.column_definitions();
    if !take_sizes(&columns.snapshot(), lengths, ColumnDefinition::width, ColumnDefinition::set_width) {
        grid.set_column_definitions(ColumnDefinitions::from_items(
            lengths.iter().copied().map(ColumnDefinition::with_width),
        ));
    }
}

/// The sizes of a text of definitions.
#[track_caller]
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
