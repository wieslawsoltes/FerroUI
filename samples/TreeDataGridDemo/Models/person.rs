//! Port of `Models/Person.cs`.

use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A person of the people page, with the people who report to them (the children of its row).
pub struct Person {
    base: ViewModelBase,
    name: RefCell<Option<String>>,
    title: RefCell<Option<String>>,
    age: Cell<i32>,
    is_active: Cell<bool>,
    is_expanded: Cell<bool>,
    // `ObservableCollection<Person> Children`: a notifying list of the children as untyped values,
    // the items of the child rows of the source the grid generates from its `ItemsSource`.
    children: Rc<FerroList<BoxedValue>>,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for Person {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl Person {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            name: RefCell::new(None),
            title: RefCell::new(None),
            age: Cell::new(0),
            is_active: Cell::new(false),
            is_expanded: Cell::new(false),
            children: Rc::new(FerroList::new()),
        })
    }

    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.name, value, "Name");
    }

    pub fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    pub fn set_title(&self, value: Option<String>) {
        self.base.raise_and_set_if_changed(&self.title, value, "Title");
    }

    pub fn age(&self) -> i32 {
        self.age.get()
    }

    pub fn set_age(&self, value: i32) {
        self.base.raise_and_set_if_changed_cell(&self.age, value, "Age");
    }

    pub fn is_active(&self) -> bool {
        self.is_active.get()
    }

    pub fn set_is_active(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_active, value, "IsActive");
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        self.base.raise_and_set_if_changed_cell(&self.is_expanded, value, "IsExpanded");
    }

    pub fn has_children(&self) -> bool {
        self.children.count() > 0
    }

    /// The children, as the notifying list of untyped values the grid reads.
    pub fn children(&self) -> Rc<FerroList<BoxedValue>> {
        self.children.clone()
    }

    /// `Children = { child }`.
    pub fn add_child(&self, child: Rc<Person>) {
        self.children.add(child as BoxedValue);
    }
}

ferro_markup_type!(class Person {
    this: Rc<Person>,
    handles: [Person, Rc<Person>, Option<Rc<Person>>],
    constructors: [() => Person::new],
    properties: [
        Name: Option<String> {
            get: |this: &Rc<Person>| this.name(),
            set: |this: &Rc<Person>, value: Option<String>| this.set_name(value)
        },
        Title: Option<String> {
            get: |this: &Rc<Person>| this.title(),
            set: |this: &Rc<Person>, value: Option<String>| this.set_title(value)
        },
        Age: i32 {
            get: |this: &Rc<Person>| this.age(),
            set: |this: &Rc<Person>, value: i32| this.set_age(value)
        },
        IsActive: bool {
            get: |this: &Rc<Person>| this.is_active(),
            set: |this: &Rc<Person>, value: bool| this.set_is_active(value)
        },
        IsExpanded: bool {
            get: |this: &Rc<Person>| this.is_expanded(),
            set: |this: &Rc<Person>, value: bool| this.set_is_expanded(value)
        },
        HasChildren: bool { get: |this: &Rc<Person>| this.has_children() },
        Children: ItemsSource { get: |this: &Rc<Person>| ItemsSource::from(this.children()) },
    ],
    notify_property_changed: Person,
});
