//! The state of the circle drawer and of the cells, which both ports share:
//! signals and memos of `ferroui-view`, independent of the user interface
//! that shows them.

use crate::cells::{parse, CellRef, Value};
use crate::{Circle, History};
use ferroui_view::{memo, signal, Memo, Signal};
use std::cell::RefCell;
use std::rc::Rc;

/// The state of the drawer, shared by its handlers.
#[derive(Clone, Copy)]
pub struct Drawer {
    /// The circles, with undo and redo.
    pub history: Signal<History<Vec<Circle>>>,
    /// The circle under the pointer.
    pub hovered: Signal<Option<u32>>,
    /// The circle whose diameter is being adjusted, and the diameter.
    pub adjusting: Signal<Option<(u32, f64)>>,
    next_id: Signal<u32>,
}

impl Drawer {
    /// An empty drawer.
    pub fn new() -> Self {
        Self { history: signal(History::new(Vec::new())), hovered: signal(None), adjusting: signal(None), next_id: signal(0) }
    }

    /// The circles as they are shown: an adjusted diameter shows before it
    /// is kept.
    pub fn circles(&self) -> Vec<Circle> {
        let adjusting = self.adjusting.get();
        self.history.with(|history| {
            history
                .current()
                .iter()
                .map(|circle| match adjusting {
                    Some((id, diameter)) if id == circle.id => Circle { diameter, ..*circle },
                    _ => *circle,
                })
                .collect()
        })
    }

    /// Adds a circle centered at a point.
    pub fn add(&self, x: f64, y: f64) {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        let mut circles = self.history.with_untracked(|history| history.current().clone());
        circles.push(Circle { id, x, y, diameter: 30.0 });
        self.history.update(|history| history.push(circles));
    }

    /// Keeps the adjusted diameter as a change that can be undone.
    pub fn commit_adjustment(&self) {
        if let Some((id, diameter)) = self.adjusting.get_untracked() {
            let mut circles = self.history.with_untracked(|history| history.current().clone());
            if let Some(circle) = circles.iter_mut().find(|circle| circle.id == id) {
                if circle.diameter != diameter {
                    circle.diameter = diameter;
                    self.history.update(|history| history.push(circles));
                }
            }
            self.adjusting.set(None);
        }
    }
}

impl Drawer {
    /// A drawer with a few circles, to show.
    pub fn with_example() -> Self {
        let drawer = Self::new();
        for (x, y) in [(90.0, 80.0), (160.0, 120.0), (240.0, 70.0), (300.0, 160.0)] {
            drawer.add(x, y);
        }
        drawer
    }
}

impl Default for Drawer {
    fn default() -> Self {
        Self::new()
    }
}


/// The cells of a sheet: what each holds and its value.
#[derive(Clone)]
pub struct Sheet {
    columns: usize,
    rows: usize,
    contents: Rc<Vec<Signal<String>>>,
    values: Rc<RefCell<Vec<Memo<Value>>>>,
    /// The cell being edited.
    pub editing: Signal<Option<CellRef>>,
}

ferroui_base::ferro_thread_local! {
    /// The cells being computed, for the formulas that name themselves.
    static COMPUTING: RefCell<Vec<CellRef>> = const { RefCell::new(Vec::new()) };
}

impl Sheet {
    /// A sheet of `columns` by `rows` empty cells.
    pub fn new(columns: usize, rows: usize) -> Self {
        let contents: Rc<Vec<Signal<String>>> = Rc::new((0..columns * rows).map(|_| signal(String::new())).collect());
        let sheet = Sheet { columns, rows, contents, values: Rc::new(RefCell::new(Vec::new())), editing: signal(None) };
        let memos = (0..columns * rows)
            .map(|index| {
                let sheet = sheet.clone();
                let cell = CellRef { column: index % columns, row: index / columns };
                memo(move |_| {
                    COMPUTING.with(|computing| computing.borrow_mut().push(cell));
                    let value = parse(&sheet.content(cell).get()).evaluate(&|other| sheet.value_of(other));
                    COMPUTING.with(|computing| computing.borrow_mut().pop());
                    value
                })
            })
            .collect();
        *sheet.values.borrow_mut() = memos;
        sheet
    }

    /// A sheet of the size of the task with a few numbers and formulas, to
    /// show.
    pub fn with_example() -> Self {
        let sheet = Self::new(crate::cells::COLUMNS, crate::cells::ROWS);
        let examples = [
            ("A0", "Item"), ("B0", "Price"), ("C0", "Count"), ("D0", "Total"),
            ("A1", "Apples"), ("B1", "1.2"), ("C1", "10"), ("D1", "=B1 * C1"),
            ("A2", "Pears"), ("B2", "0.9"), ("C2", "12"), ("D2", "=B2 * C2"),
            ("A3", "Plums"), ("B3", "2.5"), ("C3", "4"), ("D3", "=B3 * C3"),
            ("C4", "Sum"), ("D4", "=sum(D1:D3)"), ("C5", "Average"), ("D5", "=avg(D1:D3)"),
        ];
        for (name, text) in examples {
            if let Some(cell) = crate::cells::cell_ref(name) {
                sheet.content(cell).set(String::from(text));
            }
        }
        sheet
    }

    fn index(&self, cell: CellRef) -> Option<usize> {
        (cell.column < self.columns && cell.row < self.rows).then_some(cell.row * self.columns + cell.column)
    }

    /// What a cell holds.
    ///
    /// # Panics
    /// Panics for a cell outside the sheet.
    pub fn content(&self, cell: CellRef) -> Signal<String> {
        self.contents[self.index(cell).unwrap_or_else(|| panic!("{cell} is not a cell of the sheet"))]
    }

    /// The value of a cell; a cell that names itself, directly or not, is an
    /// error until the cycle is broken.
    pub fn value_of(&self, cell: CellRef) -> Value {
        let Some(index) = self.index(cell) else { return Value::Error(format!("{cell} is not a cell of the sheet")) };
        if COMPUTING.with(|computing| computing.borrow().contains(&cell)) {
            // Follow what the cell holds, so that the cycle is computed again
            // once it is broken.
            self.contents[index].with(|_| ());
            return Value::Error(String::from("cycle"));
        }
        let value = self.values.borrow()[index];
        value.get()
    }
}

