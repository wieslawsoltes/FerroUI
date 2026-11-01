//! Workspace navigation and client actions are requests to the window manager.
//! Property notifications drive the authoritative workspace and client list.
use super::chrome::*;
use super::{feedback, popup_header, Shell};
use crate::session::ClientWindow;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    let snapshot = shell.snapshot;
    let picker_shell = shell.clone();
    let window_shell = shell.clone();
    let tabs = dynamic(move || {
        snapshot.with(|model| {
            let mut choices = wrap_panel();
            for (index, name) in model.workspace_names.iter().enumerate() {
                let selected = index == model.current_workspace;
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
                    .child(label(
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
            choices
        })
    });
    let clients = dynamic(move || -> Element {
        snapshot.with(|model| {
            let windows: Vec<Element> = model
                .windows
                .iter()
                .filter(|window| {
                    window
                        .workspace
                        .is_none_or(|index| index == model.current_workspace)
                })
                .map(|client| {
                    client_row(
                        &window_shell,
                        client,
                        model.current_workspace,
                        model.workspace_names.len(),
                    )
                })
                .collect();
            if windows.is_empty() {
                let text = if window_shell.session.is_none() {
                    "Window management is unavailable in this host."
                } else {
                    "Applications opened here will appear in this workspace."
                };
                return border()
                    .padding(18)
                    .corner_radius(10)
                    .background(colors().tile)
                    .child(
                        stack_panel()
                            .spacing(6.0)
                            .child(
                                label("No windows", 17.0, Ink::Primary)
                                    .font_weight(FontWeight::Medium),
                            )
                            .child(
                                label(text, 12.0, Ink::Secondary)
                                    .text_wrapping(ferroui_base::media::TextWrapping::Wrap),
                            ),
                    )
                    .into();
            }
            stack_panel().spacing(5.0).children(windows).into()
        })
    });
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

fn client_row(shell: &Rc<Shell>, client: &ClientWindow, workspace: usize, count: usize) -> Element {
    let id = client.id;
    let next = (workspace + 1) % count.max(1);
    let state = shell.clone();
    let title = if client.title.is_empty() {
        "Untitled window"
    } else {
        &client.title
    };
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
                    .child(label(title, 14.0, Ink::Primary).font_weight(FontWeight::Medium))
                    .child(label(
                        if client.minimized {
                            "Minimized"
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
    let command = |name: &str, icon, operation: fn(&crate::session::Session, u32) -> bool| {
        let state = shell.clone();
        let label = name.to_owned();
        icon_action(name, icon, move || {
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
    let destination = state
        .snapshot
        .with_untracked(|s| s.workspace_names.get(next).cloned())
        .unwrap_or_default();
    let move_window = icon_action(&format!("Move to {destination}"), Icon::Move, move || {
        if state
            .session
            .as_ref()
            .is_some_and(|session| !session.move_to_workspace(id, next))
        {
            state.status.set("Could not move window".into());
        }
    })
    .is_visible(count > 1);
    border()
        .background(colors().tile)
        .corner_radius(9)
        .padding(4)
        .child(
            grid().column_definitions("*,Auto").child(restore).child(
                row(3.0)
                    .grid_column(1)
                    .child(move_window)
                    .child(
                        command(
                            "Minimize window",
                            Icon::Minimize,
                            crate::session::Session::minimize,
                        )
                        .is_enabled(!client.minimized),
                    )
                    .child(command(
                        "Close window",
                        Icon::Close,
                        crate::session::Session::close,
                    )),
            ),
        )
        .into()
}
