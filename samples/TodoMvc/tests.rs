//! The scenarios of TodoMVC: the application is mounted on the test
//! application of `ferroui-controls` (a dispatcher of the thread), its
//! controls are found by `Name` and driven as a user would (typing sets the
//! text of a field, a click raises `Click`, a key raises `KeyDown`).

use crate::model::Todos;
use crate::view::app;
use ferroui_base::input::{
    FocusChangedEventArgs, IPointer, InputElement, Key as KeyCode, KeyEventArgs, KeyModifiers, Pointer, PointerEventArgs,
    PointerPointProperties, PointerType, TappedEventArgs,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Point, Ref, StyledElement};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Control, Panel, TextBlock, TextBox};
use ferroui_view::{signal, Element, IntoElement, Owner, Signal};
use std::rc::Rc;

/// The mounted application, disposed when it is dropped.
struct Ui {
    root: Ref<Control>,
    owner: Option<Owner>,
    _app: UnitTestApplicationScope,
}

impl Ui {
    /// Mounts the application on `path` with a list the test keeps.
    fn app_on(path: &str) -> (Ui, Signal<Todos>) {
        Self::mount_with(|| {
            let todos = signal(Todos::new());
            (app().todos(Some(todos)).path(path).into_element(), todos)
        })
    }

    /// Mounts the application on `/`.
    fn app() -> (Ui, Signal<Todos>) {
        Self::app_on("/")
    }

    /// Mounts what `make` builds, under an owner made first (so that `make`
    /// can make signals the test keeps).
    fn mount_with<R>(make: impl FnOnce() -> (Element, R)) -> (Ui, R) {
        let scope = UnitTestApplication::start(TestServices::mock_threading_interface());
        let owner = Owner::new_root();
        let (root, kept) = owner.with(|| {
            let (element, kept) = make();
            (element.mount(), kept)
        });
        let ui = Ui { root, owner: Some(owner), _app: scope };
        ui.run_jobs();
        (ui, kept)
    }

    /// Runs the work queued on the dispatcher.
    fn run_jobs(&self) {
        Dispatcher::ui_thread().run_jobs(None);
    }

    /// The control named `name`, of class `T`, if there is one.
    fn try_find<T: ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        fn search(element: &Ref<StyledElement>, name: &str) -> Option<Ref<StyledElement>> {
            if element.name().as_deref() == Some(name) {
                return Some(element.clone());
            }
            element.logical_children().to_vec().iter().find_map(|child| search(child, name))
        }
        let root: Ref<StyledElement> = self.root.clone().upcast();
        search(&root, name).and_then(|found| found.cast::<T>())
    }

    /// The control named `name`, of class `T`.
    ///
    /// # Panics
    /// Panics when there is none.
    fn find<T: ObjectType>(&self, name: &str) -> Ref<T> {
        self.try_find(name).unwrap_or_else(|| panic!("no {} named `{name}`", std::any::type_name::<T>()))
    }

    /// Whether a control is named `name`.
    fn has(&self, name: &str) -> bool {
        self.try_find::<Control>(name).is_some()
    }

    /// Clicks the button (or check box, or toggle button) named `name`.
    fn click(&self, name: &str) {
        let button = self.find::<Button>(name);
        assert!(button.is_effectively_enabled(), "`{name}` is enabled");
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        self.run_jobs();
    }

    /// Types `text` into the field named `name`.
    fn type_text(&self, name: &str, text: &str) {
        self.find::<TextBox>(name).set_text(Some(text));
        self.run_jobs();
    }

    /// Presses `key` in the control named `name`.
    fn press(&self, name: &str, key: KeyCode) {
        let target = self.find::<Control>(name);
        let mut args = KeyEventArgs::new();
        args.set_routed_event(Some(InputElement::key_down_event()));
        args.key = key;
        target.raise_event(&args);
        self.run_jobs();
    }

    /// Types `text` into the field named `name` and presses Enter.
    fn enter(&self, name: &str, text: &str) {
        self.type_text(name, text);
        self.press(name, KeyCode::Return);
    }

    /// Double-clicks the control named `name`.
    fn double_tap(&self, name: &str) {
        let target = self.find::<Control>(name);
        let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let released = PointerEventArgs::new(
            Some(InputElement::pointer_released_event()),
            &target,
            pointer,
            None,
            Point::new(4.0, 4.0),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
        );
        target.raise_event(&TappedEventArgs::new(Some(InputElement::double_tapped_event()), &released));
        self.run_jobs();
    }

    /// The control named `name` loses the keyboard focus.
    fn lose_focus(&self, name: &str) {
        let target = self.find::<Control>(name);
        target.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
        self.run_jobs();
    }

    /// The text of the field or text block named `name`.
    fn text(&self, name: &str) -> String {
        let control = self.find::<Control>(name);
        if let Some(text_box) = control.cast::<TextBox>() {
            return text_box.text().unwrap_or_default();
        }
        control.cast::<TextBlock>().and_then(|text| text.text()).unwrap_or_default()
    }

    /// The number of todos the list shows.
    fn shown(&self) -> usize {
        self.find::<Panel>("Todos").children().count()
    }

    /// Whether the toggle button or check box named `name` is checked.
    fn is_checked(&self, name: &str) -> bool {
        self.find::<ToggleButton>(name).is_checked() == Some(true)
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            owner.dispose();
        }
    }
}

/// The titles of the list, in order.
fn titles(todos: Signal<Todos>) -> Vec<String> {
    todos.with_untracked(|list| list.items().iter().map(|todo| todo.title.clone()).collect())
}

#[test]
fn enter_adds_a_trimmed_todo_and_an_empty_title_adds_nothing() {
    let (ui, todos) = Ui::app();
    assert_eq!(ui.text("Header"), "todos");
    assert_eq!(ui.shown(), 0);
    assert!(!ui.has("Footer") && !ui.has("ToggleAll"), "the footer and the check box of all todos are hidden");

    ui.enter("NewTodo", "  Buy milk  ");
    assert_eq!(titles(todos), ["Buy milk"]);
    assert_eq!(ui.text("NewTodo"), "", "the field is emptied");
    assert_eq!(ui.shown(), 1);
    assert_eq!(ui.text("Title-1"), "Buy milk");
    assert!(ui.has("Footer") && ui.has("ToggleAll"));
    assert_eq!(ui.text("ItemsLeft"), "1 item left");

    ui.enter("NewTodo", "   ");
    assert_eq!(ui.shown(), 1, "an empty title adds nothing");
    ui.type_text("NewTodo", "Walk the dog");
    assert_eq!(ui.shown(), 1, "typing alone adds nothing");
    ui.press("NewTodo", KeyCode::Return);
    assert_eq!(titles(todos), ["Buy milk", "Walk the dog"]);
    assert_eq!(ui.text("ItemsLeft"), "2 items left");
}

#[test]
fn a_todo_is_completed_counted_and_cleared() {
    let (ui, todos) = Ui::app();
    for title in ["a", "b", "c"] {
        ui.enter("NewTodo", title);
    }
    assert!(!ui.has("ClearCompleted"), "nothing to clear");

    ui.click("Done-2");
    assert!(ui.is_checked("Done-2"));
    assert_eq!(ui.text("ItemsLeft"), "2 items left");
    assert!(ui.has("ClearCompleted"));
    assert!(ui.find::<TextBlock>("Title-2").text_decorations().is_some(), "a completed todo is struck through");

    ui.click("Done-2");
    assert!(!ui.is_checked("Done-2"));
    assert_eq!(ui.text("ItemsLeft"), "3 items left");
    assert!(!ui.has("ClearCompleted"));

    ui.click("Done-1");
    ui.click("Done-3");
    ui.click("ClearCompleted");
    assert_eq!(titles(todos), ["b"]);
    assert_eq!(ui.shown(), 1);
    assert_eq!(ui.text("ItemsLeft"), "1 item left");
    assert!(!ui.has("ClearCompleted"));
}

#[test]
fn toggle_all_marks_every_todo_and_follows_the_todos() {
    let (ui, todos) = Ui::app();
    ui.enter("NewTodo", "a");
    ui.enter("NewTodo", "b");
    assert!(!ui.is_checked("ToggleAll"));

    ui.click("ToggleAll");
    assert!(todos.with_untracked(Todos::all_completed));
    assert!(ui.is_checked("ToggleAll") && ui.is_checked("Done-1") && ui.is_checked("Done-2"));
    assert_eq!(ui.text("ItemsLeft"), "0 items left");

    ui.click("ToggleAll");
    assert_eq!(todos.with_untracked(Todos::active_count), 2);
    assert!(!ui.is_checked("Done-1") && !ui.is_checked("Done-2"));

    // Completing every todo one by one checks it too.
    ui.click("Done-1");
    ui.click("Done-2");
    assert!(ui.is_checked("ToggleAll"));
}

#[test]
fn a_double_click_edits_a_todo() {
    let (ui, todos) = Ui::app();
    ui.enter("NewTodo", "Walk the dog");
    assert!(!ui.has("Edit-1"));

    // Enter saves, trimmed.
    ui.double_tap("Title-1");
    assert!(!ui.has("Title-1"));
    assert_eq!(ui.text("Edit-1"), "Walk the dog", "the box starts with the title");
    ui.enter("Edit-1", "  Walk the cat ");
    assert!(!ui.has("Edit-1"));
    assert_eq!(ui.text("Title-1"), "Walk the cat");

    // Escape cancels.
    ui.double_tap("Title-1");
    ui.type_text("Edit-1", "Something else");
    ui.press("Edit-1", KeyCode::Escape);
    assert!(!ui.has("Edit-1"));
    assert_eq!(titles(todos), ["Walk the cat"]);

    // Leaving the box saves.
    ui.double_tap("Title-1");
    ui.type_text("Edit-1", "Feed the cat");
    ui.lose_focus("Edit-1");
    assert!(!ui.has("Edit-1"));
    assert_eq!(ui.text("Title-1"), "Feed the cat");

    // An empty title deletes the todo.
    ui.double_tap("Title-1");
    ui.enter("Edit-1", "   ");
    assert!(todos.with_untracked(Todos::is_empty));
    assert_eq!(ui.shown(), 0);
    assert!(!ui.has("Footer"));
}

#[test]
fn the_delete_button_removes_its_todo_only() {
    let (ui, todos) = Ui::app();
    for title in ["a", "b", "c"] {
        ui.enter("NewTodo", title);
    }
    let kept = ui.find::<Control>("Todo-3");
    ui.click("Delete-2");
    assert_eq!(titles(todos), ["a", "c"]);
    assert_eq!(ui.shown(), 2);
    assert!(!ui.has("Todo-2"));
    assert!(kept.ptr_eq(&ui.find::<Control>("Todo-3")), "the item of a todo that stays keeps its controls");
    ui.click("Delete-1");
    ui.click("Delete-3");
    assert!(!ui.has("Footer") && !ui.has("ToggleAll"));
}

#[test]
fn the_filters_are_routes_of_the_router() {
    let (ui, _todos) = Ui::app();
    for title in ["a", "b", "c"] {
        ui.enter("NewTodo", title);
    }
    ui.click("Done-2");
    assert!(ui.is_checked("FilterAll") && !ui.is_checked("FilterActive"));
    assert_eq!(ui.shown(), 3);

    ui.click("FilterActive");
    assert_eq!(ui.shown(), 2);
    assert!(ui.has("Todo-1") && !ui.has("Todo-2") && ui.has("Todo-3"));
    assert!(ui.is_checked("FilterActive") && !ui.is_checked("FilterAll"));
    assert_eq!(ui.text("ItemsLeft"), "2 items left", "the count does not depend on the filter");

    // Completing a todo on the page of the active ones takes it off the page.
    ui.click("Done-1");
    assert_eq!(ui.shown(), 1);

    ui.click("FilterCompleted");
    assert_eq!(ui.shown(), 2);
    assert!(ui.has("Todo-1") && ui.has("Todo-2"));
    assert!(ui.is_checked("FilterCompleted"));

    // A click on the filter shown keeps it.
    ui.click("FilterCompleted");
    assert!(ui.is_checked("FilterCompleted"));
    assert_eq!(ui.shown(), 2);

    ui.click("FilterAll");
    assert_eq!(ui.shown(), 3);
    assert!(ui.is_checked("FilterAll") && !ui.is_checked("FilterCompleted"));
}

#[test]
fn the_application_starts_on_the_route_it_is_given() {
    let (ui, todos) = Ui::app_on("/completed");
    todos.update(|list| {
        let id = list.add("done").expect("a todo");
        list.toggle(id);
        list.add("not done");
    });
    ui.run_jobs();
    assert_eq!(ui.shown(), 1);
    assert!(ui.has("Todo-1") && !ui.has("Todo-2"));
    assert!(ui.is_checked("FilterCompleted"));
}
