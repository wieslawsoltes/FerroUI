//! Port of `Models/DragDropItem.cs`.
//!
//! The reference names its items with the name data set of the fake data library Bogus, whose
//! random sequence its application seeds with `new Random(0)`. The port has no such library:
//! [`Names`] gives full names the same way (a random gender, a first name of that gender and a
//! last name) from lists of common names, with its own generator seeded with 0, so the names
//! are the same on every run but not the reference's.

use crate::view_models::random::Random;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    // `private static Random _random = new Random(0)`.
    static RANDOM: RefCell<Random> = RefCell::new(Random::with_seed(0));
    // `Bogus.Randomizer.Seed = new System.Random(0)` (the application class of the reference).
    static NAMES: RefCell<Names> = RefCell::new(Names::new(Random::with_seed(0)));
}

/// An item of the drag and drop page: a name, whether its row may be dragged and whether rows
/// may be dropped into it, and random children created when they are first asked for.
pub struct DragDropItem {
    base: ViewModelBase,
    name: String,
    children: RefCell<Option<Rc<FerroList<Rc<DragDropItem>>>>>,
    allow_drag: Cell<bool>,
    allow_drop: Cell<bool>,
}

impl PartialEq for DragDropItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for DragDropItem {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl DragDropItem {
    pub fn new(name: String) -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            name,
            children: RefCell::new(None),
            allow_drag: Cell::new(true),
            allow_drop: Cell::new(true),
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn allow_drag(&self) -> bool {
        self.allow_drag.get()
    }

    pub fn set_allow_drag(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.allow_drag, value, "AllowDrag");
    }

    pub fn allow_drop(&self) -> bool {
        self.allow_drop.get()
    }

    pub fn set_allow_drop(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.allow_drop, value, "AllowDrop");
    }

    /// `Children`: created when first asked for.
    pub fn children(&self) -> Rc<FerroList<Rc<DragDropItem>>> {
        if let Some(children) = self.children.borrow().as_ref() {
            return children.clone();
        }
        // The borrow ends before the items are created.
        let children = Self::create_random_items();
        *self.children.borrow_mut() = Some(children.clone());
        children
    }

    /// `CreateRandomItems()`: up to nine items with random names.
    pub fn create_random_items() -> Rc<FerroList<Rc<DragDropItem>>> {
        let count = RANDOM.with(|random| random.borrow_mut().next_max(10));
        let items: Vec<_> = (0..count).map(|_| DragDropItem::new(NAMES.with(|names| names.borrow_mut().full_name()))).collect();
        Rc::new(FerroList::from_items(items))
    }
}

/// The gender of a generated name (`Bogus.DataSets.Name.Gender`).
#[derive(Clone, Copy)]
enum Gender {
    Male,
    Female,
}

/// The full names of the items (the stand-in of `Bogus.DataSets.Name`).
struct Names {
    random: Random,
}

impl Names {
    const MALE_FIRST_NAMES: &'static [&'static str] = &[
        "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph", "Thomas", "Charles", "Christopher",
        "Daniel", "Matthew", "Anthony", "Mark", "Donald", "Steven", "Paul", "Andrew", "Joshua", "Kenneth", "Kevin", "Brian",
        "George", "Timothy", "Ronald", "Edward", "Jason", "Jeffrey", "Ryan", "Jacob", "Gary", "Nicholas", "Eric", "Jonathan",
        "Stephen", "Larry", "Justin", "Scott", "Brandon",
    ];

    const FEMALE_FIRST_NAMES: &'static [&'static str] = &[
        "Mary", "Patricia", "Jennifer", "Linda", "Elizabeth", "Barbara", "Susan", "Jessica", "Sarah", "Karen", "Lisa",
        "Nancy", "Betty", "Margaret", "Sandra", "Ashley", "Kimberly", "Emily", "Donna", "Michelle", "Carol", "Amanda",
        "Dorothy", "Melissa", "Deborah", "Stephanie", "Rebecca", "Sharon", "Laura", "Cynthia", "Kathleen", "Amy", "Angela",
        "Shirley", "Anna", "Brenda", "Pamela", "Emma", "Nicole", "Helen",
    ];

    const LAST_NAMES: &'static [&'static str] = &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis", "Rodriguez", "Martinez", "Hernandez",
        "Lopez", "Gonzalez", "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Perez",
        "Thompson", "White", "Harris", "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson", "Walker", "Young", "Allen",
        "King", "Wright", "Scott", "Torres", "Nguyen", "Hill", "Flores", "Green", "Adams", "Nelson", "Baker", "Hall",
        "Rivera", "Campbell", "Mitchell", "Carter", "Roberts",
    ];

    fn new(random: Random) -> Self {
        Self { random }
    }

    fn element(&mut self, items: &[&'static str]) -> &'static str {
        items[self.random.next_max(items.len() as i32) as usize]
    }

    /// `FullName()`: a first name and a last name of a random gender.
    fn full_name(&mut self) -> String {
        let gender = if self.random.next_max(2) == 0 { Gender::Male } else { Gender::Female };
        let first_name = match gender {
            Gender::Male => self.element(Self::MALE_FIRST_NAMES),
            Gender::Female => self.element(Self::FEMALE_FIRST_NAMES),
        };
        let last_name = self.element(Self::LAST_NAMES);
        format!("{first_name} {last_name}")
    }
}

ferro_markup_type!(class DragDropItem {
    this: Rc<DragDropItem>,
    handles: [DragDropItem, Rc<DragDropItem>, Option<Rc<DragDropItem>>],
    properties: [
        Name: String { get: |this: &Rc<DragDropItem>| this.name() },
        AllowDrag: bool {
            get: |this: &Rc<DragDropItem>| this.allow_drag(),
            set: |this: &Rc<DragDropItem>, value: bool| this.set_allow_drag(value)
        },
        AllowDrop: bool {
            get: |this: &Rc<DragDropItem>| this.allow_drop(),
            set: |this: &Rc<DragDropItem>, value: bool| this.set_allow_drop(value)
        },
    ],
    notify_property_changed: DragDropItem,
});

#[cfg(test)]
mod tests {
    // Not ports: the reference has no tests of its items.
    use super::*;

    #[test]
    fn the_children_are_created_once() {
        let item = DragDropItem::new("An item".to_string());

        let children = item.children();
        assert!(Rc::ptr_eq(&children, &item.children()));
        assert!(children.count() < 10);
        assert!(children.to_vec().iter().all(|x| x.name().split(' ').count() == 2));
    }

    #[test]
    fn the_flags_notify_their_changes() {
        let item = DragDropItem::new("An item".to_string());
        let changes = Rc::new(RefCell::new(Vec::<String>::new()));
        {
            let changes = changes.clone();
            item.property_changed().add(Rc::new(move |name: &str| changes.borrow_mut().push(name.to_string())));
        }

        item.set_allow_drag(false);
        item.set_allow_drag(false);
        item.set_allow_drop(false);

        assert!(!item.allow_drag());
        assert!(!item.allow_drop());
        assert_eq!(vec!["AllowDrag".to_string(), "AllowDrop".to_string()], *changes.borrow());
    }
}
