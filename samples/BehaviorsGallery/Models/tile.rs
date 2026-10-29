//! Port of `Models/Tile.cs`.

use ferroui_base::ferro_markup_type;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// A tile of the grid of the draggable page: a title, its cell, its spans and its background.
#[derive(Default)]
pub struct Tile {
    title: RefCell<Option<String>>,
    column: Cell<i32>,
    row: Cell<i32>,
    column_span: Cell<i32>,
    row_span: Cell<i32>,
    background: RefCell<Option<String>>,
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Tile {
    /// `new Tile { Title = .., Column = .., Row = .., ColumnSpan = .., RowSpan = .., Background = .. }`.
    pub fn new(
        title: &str,
        column: i32,
        row: i32,
        column_span: i32,
        row_span: i32,
        background: &str,
    ) -> Rc<Self> {
        Rc::new(Self {
            title: RefCell::new(Some(title.to_string())),
            column: Cell::new(column),
            row: Cell::new(row),
            column_span: Cell::new(column_span),
            row_span: Cell::new(row_span),
            background: RefCell::new(Some(background.to_string())),
        })
    }

    /// `Title`.
    pub fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    /// Sets `Title`.
    pub fn set_title(&self, value: Option<String>) {
        *self.title.borrow_mut() = value;
    }

    /// `Column`.
    pub fn column(&self) -> i32 {
        self.column.get()
    }

    /// Sets `Column`.
    pub fn set_column(&self, value: i32) {
        self.column.set(value);
    }

    /// `Row`.
    pub fn row(&self) -> i32 {
        self.row.get()
    }

    /// Sets `Row`.
    pub fn set_row(&self, value: i32) {
        self.row.set(value);
    }

    /// `ColumnSpan`.
    pub fn column_span(&self) -> i32 {
        self.column_span.get()
    }

    /// Sets `ColumnSpan`.
    pub fn set_column_span(&self, value: i32) {
        self.column_span.set(value);
    }

    /// `RowSpan`.
    pub fn row_span(&self) -> i32 {
        self.row_span.get()
    }

    /// Sets `RowSpan`.
    pub fn set_row_span(&self, value: i32) {
        self.row_span.set(value);
    }

    /// `Background`.
    pub fn background(&self) -> Option<String> {
        self.background.borrow().clone()
    }

    /// Sets `Background`.
    pub fn set_background(&self, value: Option<String>) {
        *self.background.borrow_mut() = value;
    }
}

impl fmt::Display for Tile {
    /// `ToString()`: the title.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title.borrow().as_deref().unwrap_or_default())
    }
}

type This = Rc<Tile>;

ferro_markup_type!(class Tile {
    this: Rc<Tile>,
    handles: [Tile, Rc<Tile>, Option<Rc<Tile>>],
    constructors: [() => || Rc::new(Tile::default())],
    properties: [
        Title: Option<String> { get: |this: &This| this.title(), set: |this: &This, value: Option<String>| this.set_title(value) },
        Column: i32 { get: |this: &This| this.column(), set: |this: &This, value: i32| this.set_column(value) },
        Row: i32 { get: |this: &This| this.row(), set: |this: &This, value: i32| this.set_row(value) },
        ColumnSpan: i32 { get: |this: &This| this.column_span(), set: |this: &This, value: i32| this.set_column_span(value) },
        RowSpan: i32 { get: |this: &This| this.row_span(), set: |this: &This, value: i32| this.set_row_span(value) },
        Background: Option<String> {
            get: |this: &This| this.background(),
            set: |this: &This, value: Option<String>| this.set_background(value)
        },
    ],
});
