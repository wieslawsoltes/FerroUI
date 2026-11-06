use ferroboard_core::input::IKeyboardScheduler;
use ferroboard_core::input::ScheduledAction;
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
pub struct ManualScheduler {
    self_ref: Weak<Self>,
    now: Cell<i64>,
    next: Cell<u64>,
    entries: RefCell<Vec<Entry>>,
}
struct Entry {
    id: u64,
    due: i64,
    action: Rc<dyn Fn()>,
}
struct Token {
    owner: Weak<ManualScheduler>,
    id: u64,
}
impl Token {
    fn cancel(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner
                .entries
                .borrow_mut()
                .retain(|entry| entry.id != self.id);
        }
    }
}
impl ManualScheduler {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            now: Cell::new(0),
            next: Cell::new(0),
            entries: RefCell::new(Vec::new()),
        })
    }
    pub fn now(&self) -> i64 {
        self.now.get()
    }
    pub fn advance(&self, ticks: i64) {
        let target = self
            .now
            .get()
            .checked_add(ticks)
            .expect("SignedDuration addition overflow");
        loop {
            let index = {
                let entries = self.entries.borrow();
                let mut index = None;
                let mut due = 0;
                for (i, entry) in entries.iter().enumerate() {
                    if entry.due <= target && (index.is_none() || entry.due < due) {
                        index = Some(i);
                        due = entry.due;
                    }
                }
                index
            };
            let Some(index) = index else {
                break;
            };
            let entry = self.entries.borrow_mut().remove(index);
            self.now.set(entry.due);
            (entry.action)();
        }
        self.now.set(target);
    }
}
impl IKeyboardScheduler for ManualScheduler {
    fn schedule(&self, delay: Duration, action: Rc<dyn Fn()>) -> ScheduledAction {
        let id = self.next.get();
        self.next.set(id + 1);
        let due = self
            .now
            .get()
            .checked_add(i64::try_from(delay.as_nanos() / 100).unwrap())
            .expect("SignedDuration addition overflow");
        self.entries.borrow_mut().push(Entry { id, due, action });
        let token = Token {
            owner: self.self_ref.clone(),
            id,
        };
        ScheduledAction::new(move || token.cancel())
    }
}
