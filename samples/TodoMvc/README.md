# TodoMVC

[TodoMVC](https://todomvc.com) written with the declarative Rust API of FerroUI (`ferroui-view`):
components that run again, with their state in hooks, and the router of single-window
applications. It is a sample of phase 6 of the API (`docs/declarative-ui.md`, section 14) and runs
in the WinUI 3 theme.

## What it shows

- **A model without user interface** (`model.rs`). A todo, the list of todos with its operations
  (add, toggle, mark all, rename, remove, clear the completed ones), its counts and its filters,
  with unit tests.
- **Components that run again** (`view.rs`). `App` keeps the list in a signal (`use_hook`) and the
  text of the new todo in another (`use_signal`); `TodoList`, `TodoItem` and `Footer` are given the
  signal and run again when what they read of it changes. The list is keyed by the identity of each
  todo, so the item of a todo that stays keeps its controls, and `TodoItem` is memoized: it runs
  again only when its todo changes.
- **Editing in place**. A double click on a title shows a text box with the title selected; Enter
  or leaving the box saves it, Escape cancels, and an empty title deletes the todo.
- **Routes as filters**. The filters All, Active and Completed are the routes `/`, `/active` and
  `/completed` of `router`; their toggle buttons call `use_router().navigate_and_reset(..)`.
- **The specification of TodoMVC**: a new todo on Enter (trimmed, an empty one ignored), the check
  box that marks every todo, `N items left`, `Clear completed` only when there are completed
  todos, and the footer and the check box hidden while the list is empty.

## Running it

```sh
cargo run -p todo-mvc
```

Take the picture of the window (it starts with a few todos, renders itself into
`<directory>/todo-mvc.png` and closes):

```sh
FERROUI_TODOMVC_SHOTS=<directory> cargo run -p todo-mvc
```

Run the tests (the model, and scenarios that mount the application on the test application of
`ferroui-controls` and drive it: add, complete, mark all, edit, delete, filter through the router,
clear the completed todos, and the counts):

```sh
cargo test -p todo-mvc
```
