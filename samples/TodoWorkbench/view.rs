//! The editable RSX layout and its stateful components.

use ferroui_base::input::Key;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use todo_mvc::model::{Filter, Todo, Todos};

#[component(rerender)]
pub(crate) fn TodoPage(initial: Todos) -> Element {
    let tasks = use_signal(move || initial);
    let draft = use_signal(String::new);
    let filter = use_signal(|| Filter::All);
    let current = filter.get();
    let (visible, active, completed) = tasks.with(|list| {
        (
            list.visible(current),
            list.active_count(),
            list.completed_count(),
        )
    });
    let empty = visible.is_empty();
    let add_task = move || {
        let entry = draft.get_untracked();
        if !entry.trim().is_empty() {
            batch(|| {
                tasks.update(|list| {
                    list.add(&entry);
                });
                draft.set(String::new());
            });
        }
    };

    rsx! {
        Border { name: "TodoSurface", padding: (32.0, 24.0), background: "#F2F7F5",
            ScrollViewer {
                StackPanel { name: "Dashboard", spacing: 16.0, max_width: 560.0,
                    StackPanel { name: "Heading", spacing: 4.0,
                        TextBlock { name: "Title", text: "Today's tasks", font_size: 36.0,
                            font_weight: FontWeight::SemiBold, foreground: "#16443D" }
                        TextBlock { name: "Summary", text: format!("{active} open · {completed} complete"),
                            font_size: 15.0, foreground: "#4F6964" }
                        TextBlock { text: "Press Enter to add a task." }
                    }
                    Grid { name: "EntryRow", column_definitions: "*,Auto",
                        TextBox { name: "NewTask", placeholder_text: "What needs doing?",
                            text: draft.two_way(), margin: (0.0, 0.0, 8.0, 0.0),
                            on_key_down: move |event| {
                                if event.key == Key::Return {
                                    event.set_handled(true);
                                    add_task();
                                }
                            } }
                        Button { name: "AddTask", Grid.column: 1, on_click: move |_| add_task(), "Add task" }
                    }
                    StackPanel { name: "Filters", orientation: Orientation::Horizontal, spacing: 8.0,
                        for choice in Filter::ALL {
                            ToggleButton { name: format!("Filter{}", choice.label()), is_checked: choice == current,
                                on_click: move |event| {
                                    filter.set(choice);
                                    if let Some(button) = event.source().and_then(|source| source.cast::<ToggleButton>()) {
                                        button.set_is_checked(Some(true));
                                    }
                                },
                                {choice.label()}
                            }
                        }
                    }
                    Border { name: "TaskCard", padding: (14.0, 10.0), corner_radius: 10.0, background: "#FFFFFF",
                        StackPanel { name: "Tasks", spacing: 4.0,
                            if empty {
                                TextBlock { text: "No tasks in this view.", margin: (8.0, 14.0), foreground: "#4F6964" }
                            }
                            for task in visible {
                                TodoRow { key: task.id, task, tasks }
                            }
                        }
                    }
                    Button { name: "ClearCompleted", is_enabled: completed > 0,
                        horizontal_alignment: HorizontalAlignment::Right,
                        on_click: move |_| tasks.update(Todos::clear_completed), "Clear completed" }
                }
            }
        }
    }
}

#[component(rerender)]
fn TodoRow(task: Todo, tasks: Signal<Todos>) -> Element {
    let Todo {
        id,
        title,
        completed,
    } = task;
    rsx! {
        Grid { name: format!("Task-{id}"), column_definitions: "*,Auto", min_height: 44.0,
            CheckBox { name: format!("Done-{id}"), is_checked: completed,
                on_click: move |_| tasks.update(|list| list.toggle(id)), {title} }
            Button { name: format!("Remove-{id}"), Grid.column: 1, vertical_alignment: VerticalAlignment::Center,
                on_click: move |_| tasks.update(|list| list.remove(id)), "Remove" }
        }
    }
}
