//! Workspace navigation and client actions are requests to the window manager.
//! Property notifications drive the authoritative workspace and client list.
use super::chrome::*;
use super::{feedback, popup_header, Shell};
use crate::session::{ClientWindow, Session, SnapSide};
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_base::media::FontWeight;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    let snapshot = shell.snapshot;
    let picker_shell = shell.clone();
    let window_shell = shell.clone();
    // Keep keyboard focus on workspace buttons when an unrelated client changes its title.
    let session = shell.session.clone();
    let tab_state = memo(move |_| {
        snapshot.with(|model| {
            let removable = session.as_ref().is_some_and(|s| s.can_remove_empty_last_workspace(model));
            (
                model.workspace_names.clone(),
                model.current_workspace,
                removable,
            )
        })
    });
    let tabs = dynamic(move || {
        tab_state.with(|(names, current, removable)| {
            let mut choices = wrap_panel();
            for (index, name) in names.iter().enumerate() {
                let selected = index == *current;
                let state = picker_shell.clone();
                let title = if name.trim().is_empty() {
                    format!("Workspace {}", index + 1)
                } else {
                    name.clone()
                };
                choices = choices.child(
                    action(&format!("Switch to {title}"), move || {
                        if let Some(session) = &state.session {
                            if !session.switch_workspace(index) {
                                state.status.set("Could not switch workspace".into());
                            }
                        }
                    })
                    .background(if selected {
                        colors().selected
                    } else {
                        colors().tile
                    })
                    .margin((0.0, 0.0, 5.0, 5.0))
                    .max_width(154.0)
                    .child(shell_text(
                        &title,
                        12.0,
                        if selected {
                            colors().on_selected
                        } else {
                            colors().text
                        },
                    )),
                );
            }
            let state = picker_shell.clone();
            let add = action("Add workspace", move || {
                if !state.session.as_ref().is_some_and(|s| s.add_workspace()) {
                    state.status.set("Could not add workspace".into());
                }
            })
            .margin((0.0, 0.0, 5.0, 5.0))
            .is_enabled(picker_shell.session.is_some() && names.len() < 256)
            .child(shell_text("+", 14.0, Ink::Primary));
            let state = picker_shell.clone();
            let remove = action("Remove empty last workspace", move || {
                if !state
                    .session
                    .as_ref()
                    .is_some_and(|s| s.remove_empty_last_workspace())
                {
                    state.status.set(
                        "The last workspace is in use or the manager declined the request".into(),
                    );
                }
            })
            .margin((0.0, 0.0, 5.0, 5.0))
            .is_enabled(*removable)
            .child(shell_text("−", 14.0, Ink::Primary));
            choices.child(add).child(remove)
        })
    });
    let expanded = signal(None::<u32>);
    let empty_shell = shell.clone();
    let clients = stack_panel()
        .spacing(5.0)
        .child(show(
            move || {
                snapshot.with(|model| {
                    !model
                        .windows
                        .iter()
                        .any(|w| w.workspace.is_none_or(|ws| ws == model.current_workspace))
                })
            },
            move || {
                let text = if empty_shell.session.is_none() {
                    "Window management is unavailable in this host."
                } else {
                    "Applications opened here will appear in this workspace."
                };
                border()
                    .padding(18)
                    .corner_radius(10)
                    .background(colors().tile)
                    .child(
                        stack_panel()
                            .spacing(6.0)
                            .child(
                                shell_text("No windows", 17.0, Ink::Primary)
                                    .font_weight(FontWeight::Medium),
                            )
                            .child(
                                shell_text(text, 12.0, Ink::Secondary)
                                    .text_wrapping(ferroui_base::media::TextWrapping::Wrap),
                            ),
                    )
            },
        ))
        .child(for_each(
            move || {
                snapshot.with(|model| {
                    let mut windows: Vec<_> = model
                        .windows
                        .iter()
                        .filter(|w| w.workspace.is_none_or(|ws| ws == model.current_workspace))
                        .cloned()
                        .collect();
                    // Stable sorting preserves mapping order among windows not yet activated.
                    windows.sort_by_key(|w| std::cmp::Reverse(w.last_activation));
                    windows
                })
            },
            |w| w.id as usize,
            move |client| client_row(&window_shell, client, expanded),
        ));
    super::desktop::popup_frame()
        .max_width(600.0)
        .max_height(690.0)
        .padding(22)
        .child(
            grid()
                .row_definitions("Auto,Auto,Auto,*,Auto")
                .child(popup_header("Workspaces", "WINDOW OVERVIEW", shell))
                .child(
                    border()
                        .grid_row(1)
                        .margin((0.0, 16.0, 0.0, 8.0))
                        .max_height(100.0)
                        .child(
                            scroll_viewer()
                                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                                .child(tabs),
                        ),
                )
                .child(eyebrow("WINDOWS").grid_row(2).margin((0.0, 3.0, 0.0, 9.0)))
                .child(
                    scroll_viewer()
                        .grid_row(3)
                        .max_height(350.0)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(clients),
                )
                .child(feedback(shell.status).grid_row(4)),
        )
        .into()
}

fn client_row(shell: &Rc<Shell>, client: ClientWindow, expanded: Signal<Option<u32>>) -> Element {
    let id = client.id;
    let state = shell.clone();
    let title = client.title.as_str();
    let restore = action(title, move || {
        if let Some(session) = &state.session {
            state.dismiss();
            if !session.activate(id) {
                state.open(super::Scene::Workspaces);
                state.status.set("Could not activate window".into());
            }
        }
    })
    .horizontal_alignment(H::Stretch)
    .horizontal_content_alignment(H::Stretch)
    .background("Transparent")
    .child(
        grid()
            .column_definitions("20,*")
            .child(glyph(Icon::Window, 16.0, colors().accent))
            .child(
                stack_panel()
                    .grid_column(1)
                    .margin((8.0, 0.0, 0.0, 0.0))
                    .child(shell_text(title, 14.0, Ink::Primary).font_weight(FontWeight::Medium))
                    .child(shell_text(
                        if client.minimized {
                            "Minimized"
                        } else if client.fullscreen {
                            "Fullscreen"
                        } else if client.maximized {
                            "Maximized"
                        } else if client.active {
                            "Focused"
                        } else {
                            "Open"
                        },
                        11.0,
                        Ink::Secondary,
                    )),
            ),
    );
    let command = |name: &str, icon, operation: fn(&Session, u32) -> bool| {
        let state = shell.clone();
        let label = name.to_owned();
        icon_action(&format!("{name}: {title}"), icon, move || {
            if state
                .session
                .as_ref()
                .is_some_and(|session| !operation(session, id))
            {
                state.status.set(format!("{label} request failed"));
            }
        })
    };
    let state = shell.clone();
    let row_actions = row(2.0)
        .child(
            command("Minimize", Icon::Minimize, Session::minimize)
                .is_enabled(client.can_minimize && !client.minimized),
        )
        .child(
            command(
                if client.maximized {
                    "Restore size"
                } else {
                    "Maximize"
                },
                if client.maximized {
                    Icon::RestoreWindow
                } else {
                    Icon::Maximize
                },
                Session::toggle_maximized,
            )
            .is_enabled(client.can_maximize && !client.fullscreen),
        )
        .child(icon_action(
            &format!("Arrange or move {title}"),
            Icon::More,
            move || {
                expanded.set(if expanded.get_untracked() == Some(id) {
                    None
                } else {
                    Some(id)
                });
            },
        ))
        .child(command("Close", Icon::Close, Session::close).is_enabled(client.can_close));
    border()
        .background(colors().tile)
        .corner_radius(9)
        .padding(4)
        .child(
            stack_panel()
                .child(
                    grid()
                        .column_definitions("*,Auto")
                        .child(restore)
                        .child(row_actions.grid_column(1)),
                )
                .child(show(
                    move || expanded.get() == Some(id),
                    move || arrange_actions(&state, &client, expanded),
                )),
        )
        .into()
}

fn arrange_actions(
    shell: &Rc<Shell>,
    client: &ClientWindow,
    expanded: Signal<Option<u32>>,
) -> Element {
    let id = client.id;
    let mut sides = wrap_panel();
    for (label, side) in [
        ("Snap left", SnapSide::Left),
        ("Snap right", SnapSide::Right),
    ] {
        let state = shell.clone();
        sides = sides.child(
            action(label, move || {
                if let Some(session) = &state.session {
                    if session.snap(id, side) {
                        state.dismiss();
                        session.activate(id);
                    } else {
                        state.status.set("Could not arrange this window".into());
                    }
                }
            })
            .margin((0.0, 0.0, 5.0, 0.0))
            .is_enabled(client.can_snap && !client.fullscreen)
            .child(shell_text(label, 12.0, Ink::Primary)),
        );
    }
    for (label, allowed, operation) in [
        (
            if client.fullscreen {
                "Exit fullscreen"
            } else {
                "Fullscreen"
            },
            client.can_fullscreen,
            Session::toggle_fullscreen as fn(&Session, u32) -> bool,
        ),
        (
            if client.above {
                "Disable always on top"
            } else {
                "Always on top"
            },
            client.can_raise,
            Session::toggle_above as fn(&Session, u32) -> bool,
        ),
    ] {
        let state = shell.clone();
        sides = sides.child(
            action(label, move || {
                if state.session.as_ref().is_some_and(|s| operation(s, id)) {
                    expanded.set(None);
                } else {
                    state.status.set(format!("{label} request failed"));
                }
            })
            .margin((0.0, 0.0, 5.0, 0.0))
            .is_enabled(allowed)
            .child(shell_text(label, 12.0, Ink::Primary)),
        );
    }
    let mut actions = stack_panel()
        .spacing(5.0)
        .margin((8.0, 6.0, 8.0, 8.0))
        .child(sides);
    if client.can_move_workspace {
        let state = shell.clone();
        let snapshot = shell.snapshot;
        let current = client.workspace;
        actions = actions
            .child(eyebrow("MOVE TO WORKSPACE"))
            .child(dynamic(move || {
                snapshot.with(|model| {
                    let mut choices = wrap_panel();
                    for (index, name) in model.workspace_names.iter().enumerate() {
                        let shell = state.clone();
                        choices = choices.child(
                            action(&format!("Move window to {name}"), move || {
                                if shell
                                    .session
                                    .as_ref()
                                    .is_some_and(|session| session.move_to_workspace(id, index))
                                {
                                    expanded.set(None);
                                } else {
                                    shell.status.set("Could not move window".into());
                                }
                            })
                            .margin((0.0, 0.0, 5.0, 3.0))
                            .is_enabled(Some(index) != current)
                            .child(shell_text(
                                name,
                                12.0,
                                Ink::Primary,
                            )),
                        );
                    }
                    choices
                })
            }));
    }
    actions.into()
}
