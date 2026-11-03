use super::*;
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
struct Group {
    key: String,
    items: Vec<Notice>,
}

// Retain the order of each source's newest item. Android automatically groups
// only sources with at least four messages; other items stay in time order.
fn groups(items: Vec<Notice>, layout: Layout) -> Vec<Rc<Group>> {
    let mut counts = HashMap::new();
    if layout == Layout::BottomNavigation {
        for item in &items {
            *counts.entry(item.sender.identity()).or_insert(0usize) += 1;
        }
    }
    let keys: Vec<_> = items
        .iter()
        .map(|item| {
            let (desktop, id) = item.sender.identity();
            if layout == Layout::BottomNavigation && counts[&(desktop, id)] < 4 {
                format!("notice/{}", item.id)
            } else {
                format!("{}/{id}", if desktop { "desktop" } else { "sender" })
            }
        })
        .collect();
    let mut indices = HashMap::new();
    let mut groups: Vec<Group> = Vec::new();
    for (item, key) in items.into_iter().zip(keys) {
        let index = *indices.entry(key.clone()).or_insert_with(|| {
            groups.push(Group {
                key,
                items: Vec::new(),
            });
            groups.len() - 1
        });
        groups[index].items.push(item);
    }
    groups.into_iter().map(Rc::new).collect()
}

fn group_view(shell: &Rc<Shell>, groups: Memo<Vec<Rc<Group>>>, key: String) -> Element {
    // The source owns its controls. A new delivery or replacement can update
    // its cards without remounting the focused disclosure or losing expansion.
    let group =
        memo(move |_| groups.with(|all| all.iter().find(|group| group.key == key).cloned()));
    let count = memo(move |_| group.with(|g| g.as_ref().map_or(0, |g| g.items.len())));
    let state = shell.clone();
    let app = memo(move |_| {
        group.with(|g| {
            g.as_ref().map_or_else(String::new, |g| {
                cards::sender_name(&state, &g.items[0].sender).to_owned()
            })
        })
    });
    let layout = look().layout();
    let limit = if layout == Layout::Taskbar { 3 } else { 1 };
    let opened = signal(false);
    let service = shell.notifications.service.clone();
    let header = grid()
        .column_definitions("*,Auto")
        .is_visible(move || count.get() > 1)
        .child(
            shell_text("", 13., Ink::Primary)
                .font_weight(FontWeight::SemiBold)
                .text(move || app.get())
                .is_visible(move || count.get() <= limit),
        )
        .child(
            action("Show or hide older notifications", move || {
                opened.update(|v| *v = !*v)
            })
            .automation_properties_name(move || {
                format!(
                    "Expand or collapse {} notifications from {}",
                    count.get(),
                    app.get()
                )
            })
            .automation_properties_item_status(move || {
                if opened.get() {
                    "Expanded"
                } else {
                    "Collapsed"
                }
                .to_owned()
            })
            .is_visible(move || count.get() > limit)
            .horizontal_alignment(H::Stretch)
            .horizontal_content_alignment(H::Left)
            .background("Transparent")
            .border_thickness(0)
            .padding((4., 2.))
            .child(
                shell_text("", 13., Ink::Primary)
                    .font_weight(FontWeight::SemiBold)
                    .text(move || {
                        let app = app.get();
                        if opened.get() {
                            format!("{app}  ·  Show fewer")
                        } else {
                            format!("{app}  ·  {} notifications", count.get())
                        }
                    }),
            ),
        )
        .child(
            icon_action("Clear sender's notifications", Icon::Close, move || {
                if let Some(g) = group.get_untracked() {
                    service.clear(Some(&g.items[0].sender));
                }
            })
            .automation_properties_name(move || format!("Clear all from {}", app.get()))
            .grid_column(1)
            .width(26)
            .height(26)
            .padding(6)
            .background("Transparent")
            .border_thickness(0),
        );
    let state = shell.clone();
    let rows = for_each(
        move || {
            group.with(|g| {
                let limit = if opened.get() { usize::MAX } else { limit };
                g.iter()
                    .flat_map(|g| g.items.iter().take(limit))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        },
        |n| n.id,
        move |item| cards::card(&state, &item, false),
    );
    let stack = stack_panel()
        .spacing(if layout == Layout::MenuBarDock {
            2.
        } else {
            4.
        })
        .child(header)
        .child(rows);
    if layout == Layout::MenuBarDock {
        // The revealed edges distinguish a collapsed stack from one alert.
        stack
            .child(show(
                move || count.get() > 1 && !opened.get(),
                || {
                    stack_panel()
                        .child(
                            border()
                                .height(3.)
                                .margin((6., 0., 6., 0.))
                                .corner_radius(2)
                                .background(colors().edge),
                        )
                        .child(
                            border()
                                .height(2.)
                                .margin((13., 0., 13., 0.))
                                .corner_radius(2)
                                .background(colors().tile),
                        )
                },
            ))
            .into()
    } else {
        stack.into()
    }
}

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    const PAGE: usize = 8;
    let layout = look().layout();
    let page = signal(0usize);
    let notifications = shell.notifications.service.clone();
    let revision = shell.notifications.revision;
    let grouped = memo(move |_| {
        revision.get();
        groups(notifications.history(), layout)
    });
    let state = shell.clone();
    let dismiss = shell.clone();
    let quiet = shell.clone();
    let quiet_text = shell.clone();
    let focus = NodeRef::<ferroui_controls::Button>::new();
    let title = match layout {
        Layout::Taskbar => "Notifications",
        Layout::MenuBarDock => "Notification Center",
        Layout::BottomNavigation => "Notifications",
    };
    let now = super::super::now();
    let head = grid()
        .column_definitions("*,Auto")
        .child(
            stack_panel()
                .spacing(2.)
                .child(shell_text(
                    &format!("{}, {} {}", now.weekday(), now.day(), now.month()),
                    12.,
                    Ink::Secondary,
                ))
                .child(
                    shell_text(
                        title,
                        if layout == Layout::BottomNavigation {
                            26.
                        } else {
                            21.
                        },
                        Ink::Primary,
                    )
                    .font_weight(FontWeight::SemiBold),
                ),
        )
        .child(
            icon_action("Close notifications", Icon::Close, move || {
                dismiss.dismiss()
            })
            .grid_column(1)
            .vertical_alignment(V::Top),
        );
    let quiet_button = action("Toggle Do Not Disturb", move || {
        quiet.notifications.toggle_quiet()
    })
    .node_ref(focus)
    .on_loaded(move |_| {
        if let Some(button) = focus.get_untracked() {
            button.focus();
        }
    })
    .child(shell_text("", 12., Ink::Primary).text(move || {
        let on = quiet_text.notifications.quiet();
        match (layout, on) {
            (Layout::MenuBarDock, true) => "Focus · Do Not Disturb",
            (Layout::MenuBarDock, false) => "Focus · Off",
            (_, true) => "Do Not Disturb · On",
            (_, false) => "Do Not Disturb · Off",
        }
        .to_owned()
    }));
    let clear_button = || {
        let clear = shell.clone();
        let count = shell.clone();
        action("Clear all notifications", move || {
            clear.notifications.service.clear(None)
        })
        .is_enabled(move || count.notifications.count() > 0)
        .child(shell_text("Clear all", 12., Ink::Primary))
    };
    let mut tools = wrap_panel().child(quiet_button.margin((0., 0., 6., 3.)));
    if layout == Layout::BottomNavigation {
        let target = shell.clone();
        tools = tools.child(
            action("Open Quick Settings", move || {
                target.open(Scene::QuickSettings)
            })
            .child(shell_text("Quick settings", 12., Ink::Primary)),
        );
    } else {
        tools = tools.child(clear_button());
    }
    let status = shell.clone();
    let messages = stack_panel()
        .spacing(if layout == Layout::MenuBarDock {
            6.
        } else {
            8.
        })
        .child(show(
            move || grouped.with(Vec::is_empty),
            move || {
                let msg = status.notifications.service.status();
                stack_panel()
                    .spacing(6.)
                    .margin((6., 20.))
                    .child(
                        shell_text("No notifications", 17., Ink::Primary)
                            .font_weight(FontWeight::SemiBold),
                    )
                    .child(
                        shell_text(
                            if msg.is_empty() {
                                "New notifications will appear here."
                            } else {
                                &msg
                            },
                            13.,
                            Ink::Secondary,
                        )
                        .text_wrapping(TextWrapping::Wrap),
                    )
            },
        ))
        .child(for_each(
            move || {
                grouped.with(|g| {
                    let first = page.get().min(g.len().saturating_sub(1) / PAGE) * PAGE;
                    g.iter()
                        .skip(first)
                        .take(PAGE)
                        .map(|g| g.key.clone())
                        .collect::<Vec<_>>()
                })
            },
            |key| key.clone(),
            move |key| group_view(&state, grouped, key),
        ));
    let pager = dynamic(move || -> Element {
        let total = grouped.with(|v| v.len().div_ceil(PAGE));
        if total < 2 {
            return Element::Empty;
        }
        let i = page.get().min(total - 1);
        row(6.)
            .child(
                icon_action("Previous notifications", Icon::Back, move || {
                    page.set(i.saturating_sub(1))
                })
                .is_enabled(i != 0),
            )
            .child(shell_text(
                &format!("{} / {total}", i + 1),
                12.,
                Ink::Secondary,
            ))
            .child(
                icon_action("More notifications", Icon::Forward, move || page.set(i + 1))
                    .is_enabled(i + 1 < total),
            )
            .into()
    });
    let frame = super::super::desktop::system_frame(
        shell,
        match layout {
            Layout::MenuBarDock => 376.,
            Layout::Taskbar => 422.,
            Layout::BottomNavigation => 490.,
        },
    );
    let mut footer = stack_panel().child(pager);
    let frame = if layout == Layout::BottomNavigation {
        footer = footer.child(clear_button().horizontal_alignment(H::Right));
        frame
            .vertical_alignment(V::Top)
            .margin((8., look().insets().0 + 6., 8., 12.))
    } else {
        frame
    };
    frame
        .max_height(640.)
        .padding(if layout == Layout::BottomNavigation {
            14
        } else {
            16
        })
        .automation_properties_name(title)
        .child(
            grid()
                .row_definitions("Auto,Auto,*,Auto")
                .child(head)
                .child(tools.grid_row(1).margin((0., 8., 0., 5.)))
                .child(
                    scroll_viewer()
                        .grid_row(2)
                        .max_height(425.)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(messages),
                )
                .child(footer.grid_row(3)),
        )
        .into()
}
