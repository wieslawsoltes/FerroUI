//! The scenarios of the tasks, which the tests of both ports run on their
//! user interface: what a user does and what the task must show afterwards.
//! A scenario mounts the element of a task, finds its controls by `Name`
//! (each port names them the same), and drives them as a user would (typing
//! sets the text of a field, a click raises `Click`).

use crate::cells::cell_ref;
use crate::state::{Drawer, Sheet};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{BoxedValue, ObjectType, Ref, StyledElement, Upcast};
use ferroui_controls::primitives::{RangeBase, SelectingItemsControl};
use ferroui_controls::{Button, Canvas, Control, ItemsControl, TextBlock, TextBox};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_view::{signal, Element, Owner, Signal};

/// The mounted user interface of a task, disposed when it is dropped.
pub struct Ui {
    root: Ref<Control>,
    owner: Option<Owner>,
    _app: UnitTestApplicationScope,
}

impl Ui {
    /// Mounts an element under an owner of its own.
    pub fn mount(make: impl FnOnce() -> Element) -> Ui {
        Self::mount_with(|| (make(), ())).0
    }

    /// Mounts what `make` builds, under an owner made first (so that `make`
    /// can make signals the test keeps).
    pub fn mount_with<R>(make: impl FnOnce() -> (Element, R)) -> (Ui, R) {
        // The dispatcher of the thread, which controls post their work to.
        let app = UnitTestApplication::start(TestServices::mock_threading_interface());
        let owner = Owner::new_root();
        let (root, kept) = owner.with(|| {
            let (element, kept) = make();
            (element.mount(), kept)
        });
        (Ui { root, owner: Some(owner), _app: app }, kept)
    }

    /// The control named `name`, of class `T`.
    ///
    /// # Panics
    /// Panics when there is none.
    pub fn find<T: ObjectType>(&self, name: &str) -> Ref<T> {
        fn search(element: &Ref<StyledElement>, name: &str) -> Option<Ref<StyledElement>> {
            if element.name().as_deref() == Some(name) {
                return Some(element.clone());
            }
            element.logical_children().to_vec().iter().find_map(|child| search(child, name))
        }
        let root: Ref<StyledElement> = self.root.clone().upcast();
        search(&root, name)
            .and_then(|found| found.cast::<T>())
            .unwrap_or_else(|| panic!("no {} named `{name}`", std::any::type_name::<T>()))
    }

    /// Clicks the button named `name`.
    pub fn click(&self, name: &str) {
        let button = self.find::<Button>(name);
        assert!(button.is_effectively_enabled(), "`{name}` is enabled");
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    }

    /// Whether the control named `name` is enabled.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.find::<Control>(name).is_enabled()
    }

    /// Types `text` into the field named `name`.
    pub fn type_text(&self, name: &str, text: &str) {
        self.find::<TextBox>(name).set_text(Some(text));
    }

    /// The text of the field or text block named `name`.
    pub fn text(&self, name: &str) -> String {
        let control = self.find::<Control>(name);
        if let Some(text_box) = control.cast::<TextBox>() {
            return text_box.text().unwrap_or_default();
        }
        control.cast::<TextBlock>().and_then(|text| text.text()).unwrap_or_default()
    }

    /// The number of items of the items control named `name`.
    pub fn item_count(&self, name: &str) -> usize {
        self.find::<ItemsControl>(name).items().count()
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            owner.dispose();
        }
    }
}

/// Counter: each click counts one more.
pub fn counter(task: impl FnOnce() -> Element) {
    let ui = Ui::mount(task);
    assert_eq!(ui.text("Count"), "0");
    ui.click("Increment");
    ui.click("Increment");
    assert_eq!(ui.text("Count"), "2");
}

/// Temperature converter: either field sets the other; text that is no
/// number sets nothing.
pub fn temperature(task: impl FnOnce() -> Element) {
    let ui = Ui::mount(task);
    ui.type_text("Celsius", "100");
    assert_eq!(ui.text("Fahrenheit"), "212");
    ui.type_text("Fahrenheit", "5");
    assert_eq!(ui.text("Celsius"), "-15");
    ui.type_text("Celsius", "abc");
    assert_eq!(ui.text("Fahrenheit"), "5", "text that is no number converts nothing");
    ui.type_text("Celsius", "1.5");
    assert_eq!(ui.text("Celsius"), "1.5", "a field the user typed into is not converted back");
    assert_eq!(ui.text("Fahrenheit"), "35");
}

/// Flight booker: the return date for a return flight only, invalid dates
/// marked, booking only with valid dates in order.
pub fn flight(task: impl FnOnce() -> Element) {
    let ui = Ui::mount(task);
    assert!(ui.is_enabled("Book"));
    assert!(!ui.is_enabled("Return"), "a one-way flight has no return date");

    ui.find::<SelectingItemsControl>("Kind").set_selected_index(1);
    assert!(ui.is_enabled("Return"));
    ui.type_text("Return", "26.03.2014");
    assert!(!ui.is_enabled("Book"), "the return date is before the start date");

    ui.type_text("Start", "31.02.2014");
    assert!(ui.find::<TextBox>("Start").background().is_some(), "an invalid date is marked");
    ui.type_text("Start", "20.03.2014");
    assert!(ui.find::<TextBox>("Start").background().is_none());
    assert!(ui.is_enabled("Book"));
    ui.click("Book");
    assert_eq!(ui.text("Message"), "You have booked a return flight on 20.03.2014, returning on 26.03.2014.");
}

/// Timer: the gauge is the elapsed time over the duration, the slider sets
/// the duration, reset starts again.
pub fn timer(task: impl FnOnce(Signal<f64>, Signal<f64>) -> Element) {
    let (ui, (elapsed, duration)) = Ui::mount_with(|| {
        let (elapsed, duration) = (signal(7.5_f64), signal(15.0_f64));
        (task(elapsed, duration), (elapsed, duration))
    });
    let gauge = || ui.find::<RangeBase>("Gauge").value();
    assert_eq!(gauge(), 0.5);
    assert_eq!(ui.text("Seconds"), "7.5 s");
    ui.find::<RangeBase>("Duration").set_value(RangeBase::value_property(), 30.0);
    assert_eq!(duration.get_untracked(), 30.0);
    assert_eq!(gauge(), 0.25);
    ui.click("Reset");
    assert_eq!(elapsed.get_untracked(), 0.0);
    assert_eq!(ui.text("Seconds"), "0.0 s");
}

/// CRUD: filter, select, update, delete, create.
pub fn crud(task: impl FnOnce() -> Element) {
    let ui = Ui::mount(task);
    assert_eq!(ui.item_count("People"), 3);
    assert!(!ui.is_enabled("Update") && !ui.is_enabled("Delete"));

    ui.type_text("Prefix", "M");
    assert_eq!(ui.item_count("People"), 1);
    ui.find::<SelectingItemsControl>("People").set_selected_index(0);
    assert_eq!((ui.text("Name"), ui.text("Surname")), (String::from("Max"), String::from("Mustermann")));

    ui.type_text("Surname", "Muster");
    ui.click("Update");
    let items = ui.find::<ItemsControl>("People").items();
    assert_eq!(items.count(), 1);
    let label = items.get_at(0).and_then(|item: BoxedValue| Control::from_boxed(&item));
    let label = label.and_then(|item| item.cast::<ferroui_controls::ContentControl>()).and_then(|item| item.content());
    assert_eq!(label.and_then(|label| label.downcast_ref::<String>().cloned()).as_deref(), Some("Muster, Max"));

    ui.click("Delete");
    assert_eq!(ui.item_count("People"), 0);
    ui.type_text("Prefix", "");
    assert_eq!(ui.item_count("People"), 2);

    ui.type_text("Name", "Anna");
    ui.type_text("Surname", "Zimmer");
    ui.click("Create");
    assert_eq!(ui.item_count("People"), 3);
}

/// Circle drawer: circles are added, undone and redone, and an adjusted
/// diameter is kept as one change.
pub fn circles(task: impl FnOnce(Drawer) -> Element) {
    let (ui, drawer) = Ui::mount_with(|| {
        let drawer = Drawer::new();
        (task(drawer), drawer)
    });
    let circles = || ui.find::<Canvas>("Canvas").children().count();
    assert!(!ui.is_enabled("Undo"));
    drawer.add(40.0, 40.0);
    drawer.add(100.0, 60.0);
    assert_eq!(circles(), 2);
    ui.click("Undo");
    assert_eq!(circles(), 1);
    ui.click("Redo");
    assert_eq!(circles(), 2);

    let first = drawer.history.with_untracked(|history| history.current()[0]);
    drawer.adjusting.set(Some((first.id, 80.0)));
    assert_eq!(ui.find::<Canvas>("Canvas").children().get(0).width(), 80.0, "the adjusted diameter shows");
    drawer.commit_adjustment();
    ui.click("Undo");
    assert_eq!(drawer.history.with_untracked(|history| history.current()[0].diameter), 30.0);
    ui.click("Redo");
    assert_eq!(drawer.history.with_untracked(|history| history.current()[0].diameter), 80.0);

    drawer.hovered.set(Some(first.id));
    assert!(ui.find::<Canvas>("Canvas").children().get(0).cast::<ferroui_controls::shapes::Shape>().and_then(|s| s.fill()).is_some());
}

/// Cells: formulas compute from the cells they name and follow them; a
/// cycle is an error until it is broken.
pub fn cells(task: impl FnOnce(Sheet) -> Element) {
    let (ui, sheet) = Ui::mount_with(|| {
        let sheet = Sheet::new(crate::cells::COLUMNS, crate::cells::ROWS);
        (task(sheet.clone()), sheet)
    });
    let cell = |name: &str| cell_ref(name).expect("a cell");
    let shown = |name: &str| {
        let border = ui.find::<ferroui_controls::Border>(name);
        border.child().and_then(|child| child.cast::<TextBlock>()).and_then(|text| text.text()).unwrap_or_default()
    };
    sheet.content(cell("A0")).set(String::from("1"));
    sheet.content(cell("A1")).set(String::from("=A0 + 1"));
    sheet.content(cell("A2")).set(String::from("=sum(A0:A1) * 2"));
    assert_eq!(shown("A2"), "6");
    sheet.content(cell("A0")).set(String::from("10"));
    assert_eq!(shown("A2"), "42");
    sheet.content(cell("A0")).set(String::from("=A2"));
    assert_eq!(shown("A2"), "#cycle");
    sheet.content(cell("A0")).set(String::from("2"));
    assert_eq!(shown("A2"), "10", "a broken cycle is computed again");

    sheet.editing.set(Some(cell("B3")));
    let editor = ui.find::<ferroui_controls::Border>("B3").child().and_then(|child| child.cast::<TextBox>());
    let editor = editor.expect("the edited cell is a text field");
    editor.set_text(Some("=A0 * 3"));
    sheet.editing.set(None);
    assert_eq!(shown("B3"), "6");
}
