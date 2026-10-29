//! Port of `Models/DragItem.cs`.

use ferroui_base::ferro_markup_type;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// An item of the canvas of the draggable page: a title at a position.
#[derive(Default)]
pub struct DragItem {
    title: RefCell<Option<String>>,
    x: Cell<f64>,
    y: Cell<f64>,
}

impl PartialEq for DragItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DragItem {
    /// `new DragItem { Title = title, X = x, Y = y }`.
    pub fn new(title: &str, x: f64, y: f64) -> Rc<Self> {
        Rc::new(Self {
            title: RefCell::new(Some(title.to_string())),
            x: Cell::new(x),
            y: Cell::new(y),
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

    /// `X`.
    pub fn x(&self) -> f64 {
        self.x.get()
    }

    /// Sets `X`.
    pub fn set_x(&self, value: f64) {
        self.x.set(value);
    }

    /// `Y`.
    pub fn y(&self) -> f64 {
        self.y.get()
    }

    /// Sets `Y`.
    pub fn set_y(&self, value: f64) {
        self.y.set(value);
    }
}

impl fmt::Display for DragItem {
    /// `ToString()`: the title.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.title.borrow().as_deref().unwrap_or_default())
    }
}

ferro_markup_type!(class DragItem {
    this: Rc<DragItem>,
    handles: [DragItem, Rc<DragItem>, Option<Rc<DragItem>>],
    constructors: [() => || Rc::new(DragItem::default())],
    properties: [
        Title: Option<String> {
            get: |this: &Rc<DragItem>| this.title(),
            set: |this: &Rc<DragItem>, value: Option<String>| this.set_title(value)
        },
        X: f64 { get: |this: &Rc<DragItem>| this.x(), set: |this: &Rc<DragItem>, value: f64| this.set_x(value) },
        Y: f64 { get: |this: &Rc<DragItem>| this.y(), set: |this: &Rc<DragItem>, value: f64| this.set_y(value) },
    ],
});
