//! The user interface of TodoMVC: components that run again, with their
//! state in hooks, over the model of `model.rs`.
//!
//! - [`App`]: the header and the field of a new todo, and a router whose
//!   routes are the filters (`/`, `/active`, `/completed`).
//! - `TodoList`: the page of a filter: the check box that marks every todo,
//!   the todos the filter shows, keyed by their identity, and the footer.
//! - `TodoItem`: a todo: its check box, its title (a double click edits it)
//!   and the button that deletes it.
//! - `Footer`: the count of the todos left, the filters, and the button that
//!   clears the completed todos.
//!
//! The list is a signal of [`Todos`] that [`App`] keeps; the components below
//! it are given the signal and run again when what they read of it changes.

use crate::model::{items_left, Filter, Todo, Todos};
use ferroui_base::input::Key as KeyCode;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::{FontWeight, TextDecorations};
use ferroui_view::prelude::*;

/// The application: the header, the field of a new todo and the router of
/// the filters. `todos` is the list it shows (a new, empty one when it is
/// not given), `path` the route it starts on.
#[component(rerender)]
pub fn App(todos: Option<Signal<Todos>>, #[prop(default = String::from("/"))] path: String) -> Element {
    let todos = use_hook(move || todos.unwrap_or_else(|| signal(Todos::new())));
    let draft = use_signal(String::new);

    // A route per filter; each shows the list through that filter.
    let routes: Vec<RouteDef> = Filter::ALL
        .into_iter()
        .map(|filter| route(filter.path(), move |_| rsx! { TodoList { todos, filter } }))
        .collect();

    rsx! {
        Border { padding: (16.0, 24.0),
            StackPanel { max_width: 560.0, spacing: 8.0,
                TextBlock { name: "Header", text: "todos", font_size: 64.0, font_weight: FontWeight::Thin,
                    foreground: "#B83F45", horizontal_alignment: HorizontalAlignment::Center }
                TextBox { name: "NewTodo", placeholder_text: "What needs to be done?", font_size: 18.0,
                    text: draft.two_way(),
                    // Enter adds the todo (trimmed; an empty title adds nothing) and empties the field.
                    on_key_down: move |e| {
                        if e.key == KeyCode::Return {
                            e.set_handled(true);
                            let title = draft.get_untracked();
                            batch(|| {
                                todos.update(|todos| {
                                    todos.add(&title);
                                });
                                draft.set(String::new());
                            });
                        }
                    } }
                {router(&path, routes)}
                TextBlock { text: "Double-click a todo to edit it", font_size: 11.0, opacity: 0.6,
                    horizontal_alignment: HorizontalAlignment::Center, margin: (0.0, 16.0, 0.0, 0.0) }
            }
        }
    }
}

/// The page of a filter: the todos it shows, with the check box that marks
/// them all and the footer, both hidden while the list is empty.
#[component(rerender)]
fn TodoList(todos: Signal<Todos>, filter: Filter) -> Element {
    let (visible, all_completed, any) =
        todos.with(|list| (list.visible(filter), list.all_completed(), !list.is_empty()));

    rsx! {
        StackPanel { spacing: 8.0,
            if any {
                CheckBox { name: "ToggleAll", is_checked: all_completed,
                    on_click: move |_| todos.update(|list| list.set_all_completed(!all_completed)),
                    "Mark all as complete" }
            }
            StackPanel { name: "Todos", spacing: 2.0,
                for todo in visible {
                    TodoItem { key: todo.id, todo, todos }
                }
            }
            if any {
                Footer { todos, filter }
            }
        }
    }
}

/// A todo. A double click on its title edits it in a text box: Enter or
/// leaving the box saves the title (an empty title deletes the todo), Escape
/// cancels. Its parameters can be compared, so it runs again only when its
/// todo changes.
#[component(rerender)]
fn TodoItem(todo: Todo, todos: Signal<Todos>) -> Element {
    let editing = use_signal(|| false);
    let draft = use_signal(String::new);
    let editor = use_hook(NodeRef::<TextBox>::new);
    let id = todo.id;

    // Saves the title being edited; does nothing when the edit has ended
    // already (the box loses the focus when it is taken out).
    let save = move || {
        if editing.get_untracked() {
            let title = draft.get_untracked();
            batch(|| {
                editing.set(false);
                todos.update(|list| list.rename(id, &title));
            });
        }
    };
    let title = todo.title.clone();
    let completed = todo.completed;

    rsx! {
        Grid { name: format!("Todo-{id}"), column_definitions: "Auto,*,Auto", min_height: 40.0,
            // The check box of WinUI 3 is at least 120 wide; the title starts next to it here.
            CheckBox { name: format!("Done-{id}"), is_checked: completed, vertical_alignment: VerticalAlignment::Center,
                min_width: 0.0, margin: (0.0, 0.0, 8.0, 0.0),
                on_click: move |_| todos.update(|list| list.toggle(id)) }
            if editing.get() {
                TextBox { name: format!("Edit-{id}"), Grid.column: 1, vertical_alignment: VerticalAlignment::Center,
                    text: draft.two_way(), node_ref: editor,
                    // The box takes the focus, with its text selected, once it is shown.
                    on_loaded: move |_| {
                        if let Some(editor) = editor.get_untracked() {
                            let _ = editor.focus();
                            editor.select_all();
                        }
                    },
                    on_key_down: move |e| {
                        if e.key == KeyCode::Return {
                            e.set_handled(true);
                            save();
                        } else if e.key == KeyCode::Escape {
                            e.set_handled(true);
                            editing.set(false);
                        }
                    },
                    on_lost_focus: move |_| save() }
            } else {
                TextBlock { name: format!("Title-{id}"), Grid.column: 1, text: todo.title.clone(), font_size: 16.0,
                    vertical_alignment: VerticalAlignment::Center, background: "Transparent",
                    opacity: if completed { 0.5 } else { 1.0 },
                    text_decorations: if completed { Some(TextDecorations::strikethrough()) } else { None },
                    on_double_tapped: move |_| {
                        draft.set(title.clone());
                        editing.set(true);
                    } }
            }
            Button { name: format!("Delete-{id}"), Grid.column: 2, vertical_alignment: VerticalAlignment::Center,
                on_click: move |_| todos.update(|list| list.remove(id)), "Delete" }
        }
    }
}

/// The footer: the count of the todos left, the filters (the one shown is
/// checked), and the button that clears the completed todos, shown only when
/// there are some.
#[component(rerender)]
fn Footer(todos: Signal<Todos>, filter: Filter) -> Element {
    let router = use_router();
    let (left, completed) = todos.with(|list| (list.active_count(), list.completed_count()));

    rsx! {
        Grid { name: "Footer", column_definitions: "*,Auto,*", margin: (0.0, 8.0, 0.0, 0.0),
            TextBlock { name: "ItemsLeft", text: items_left(left), vertical_alignment: VerticalAlignment::Center }
            StackPanel { Grid.column: 1, orientation: Orientation::Horizontal, spacing: 4.0,
                {filter_link(Filter::All, filter, router)}
                {filter_link(Filter::Active, filter, router)}
                {filter_link(Filter::Completed, filter, router)}
            }
            if completed > 0 {
                Button { name: "ClearCompleted", Grid.column: 2, horizontal_alignment: HorizontalAlignment::Right,
                    on_click: move |_| todos.update(Todos::clear_completed), "Clear completed" }
            }
        }
    }
}

/// The toggle button of the filter `target`, checked when it is the filter
/// `shown`; a click shows its route.
fn filter_link(target: Filter, shown: Filter, router: Router) -> Element {
    let selected = target == shown;
    rsx! {
        ToggleButton { name: format!("Filter{}", target.label()), is_checked: selected,
            on_click: move |e| {
                if selected {
                    // A click on the filter shown unchecks its button; it stays checked.
                    if let Some(button) = e.source().and_then(|source| source.cast::<ToggleButton>()) {
                        button.set_is_checked(Some(true));
                    }
                } else {
                    router.navigate_and_reset(target.path());
                }
            },
            {target.label()}
        }
    }
}
