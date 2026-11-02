//! Sender identity, notification content and client-provided actions. Cards use
//! real buttons for every operation, including the optional default action.
use super::*;
use crate::services::notifications::Sender;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::media::{IImage, Stretch};
use time::OffsetDateTime;

pub(super) fn sender_name<'a>(shell: &'a Shell, sender: &'a Sender) -> &'a str {
    if !sender.name.is_empty() {
        return &sender.name;
    }
    shell
        .catalog
        .window_index(&sender.desktop_entry)
        .map(|i| shell.catalog.applications()[i].name.as_str())
        .unwrap_or("Application")
}

pub(super) fn sender_mark(shell: &Shell, sender: &Sender, size: f64) -> Element {
    let bitmap = shell.icons.get_icon(&sender.icon).or_else(|| {
        let id = shell.catalog.window_index(&sender.desktop_entry)?;
        shell.icons.get(&shell.catalog.applications()[id])
    });
    if let Some(bitmap) = bitmap {
        let source: Rc<dyn IImage> = bitmap;
        return image()
            .source(source)
            .stretch(Stretch::Uniform)
            .width(size)
            .height(size)
            .is_hit_test_visible(false)
            .into();
    }
    // Preserve a recognizable sender in installations without an icon theme.
    let initial = sender_name(shell, sender)
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "•".into());
    border()
        .width(size)
        .height(size)
        .corner_radius(if look().layout() == Layout::MenuBarDock {
            size * 0.24
        } else {
            size / 2.
        })
        .background(colors().selected)
        .child(
            shell_text(&initial, size * 0.52, colors().on_selected)
                .horizontal_alignment(H::Center)
                .font_weight(FontWeight::SemiBold),
        )
        .is_hit_test_visible(false)
        .into()
}

fn timestamp(notice: &Notice) -> String {
    let now = super::super::now();
    let received = OffsetDateTime::from(notice.received).to_offset(now.offset());
    if received.date() == now.date() {
        super::super::time_text(received)
    } else {
        format!("{} {}", received.day(), received.month())
    }
}

fn message(shell: &Rc<Shell>, notice: &Notice, banner: bool, lines: i32) -> Element {
    let mut text = stack_panel().spacing(2).child(
        shell_text(&notice.title, 14., Ink::Primary)
            .font_weight(FontWeight::SemiBold)
            .text_wrapping(TextWrapping::Wrap)
            .max_lines(2),
    );
    if !notice.body.is_empty() {
        text = text.child(
            shell_text(&notice.body, 13., Ink::Primary)
                .text_wrapping(TextWrapping::Wrap)
                .max_lines(lines),
        );
    }
    if notice.actions.iter().any(|(key, _)| key == "default") {
        let id = notice.id;
        let service = shell.notifications.service.clone();
        let available = shell.clone();
        return action(&format!("Open {}", notice.title), move || {
            service.invoke(id, "default")
        })
        .background("Transparent")
        .border_thickness(0)
        .padding(0)
        .focusable(!banner)
        .horizontal_content_alignment(H::Stretch)
        .horizontal_alignment(H::Stretch)
        .is_enabled(move || {
            available.notifications.revision.get();
            available.notifications.service.accepts_actions()
        })
        .child(text)
        .into();
    }
    text.into()
}

fn action_bar(shell: &Rc<Shell>, notice: &Notice, banner: bool) -> Element {
    let id = notice.id;
    let mut bar = wrap_panel();
    for (key, title) in notice.actions.iter().filter(|(key, _)| key != "default") {
        let label = if title.is_empty() { key } else { title };
        let key = key.clone();
        let invoke = shell.notifications.service.clone();
        let available = shell.clone();
        let button = action(label, move || invoke.invoke(id, &key))
            .focusable(!banner)
            .is_enabled(move || {
                available.notifications.revision.get();
                available.notifications.service.accepts_actions()
            })
            .padding((10., 3.))
            .margin((0., 3., 5., 0.));
        let button = if look().layout() == Layout::BottomNavigation {
            button
                .class("text")
                .background("Transparent")
                .border_thickness(0)
        } else {
            button
        };
        bar = bar.child(button.child(shell_text(label, 12., Ink::Primary)));
    }
    bar.into()
}

pub(super) fn card(shell: &Rc<Shell>, notice: &Notice, banner: bool) -> Element {
    let layout = look().layout();
    let id = notice.id;
    let dismiss = shell.notifications.service.clone();
    let close_name = format!("Dismiss {}", notice.title);
    let close = move || {
        let dismiss = dismiss.clone();
        icon_action(&close_name, Icon::Close, move || dismiss.dismiss(id))
            .focusable(!banner)
            .width(26)
            .height(26)
            .padding(6)
            .background("Transparent")
            .border_thickness(0)
    };
    let name = sender_name(shell, &notice.sender);
    let time = timestamp(notice);
    let compact = layout == Layout::BottomNavigation && !banner;
    let expanded = signal(!compact);
    let mut head = grid()
        .column_definitions("*,Auto,Auto")
        .child(
            shell_text(
                name,
                12.,
                if notice.critical {
                    Ink::Accent
                } else {
                    Ink::Secondary
                },
            )
            .font_weight(FontWeight::SemiBold),
        )
        .child(
            shell_text(&time, 11., Ink::Secondary)
                .grid_column(1)
                .margin((4., 0.)),
        );
    if compact {
        let toggle = action(&format!("Expand or collapse {}", notice.title), move || {
            expanded.update(|v| *v = !*v);
        })
        .width(30)
        .height(28)
        .padding(3)
        .corner_radius(14)
        .child(shell_text("", 15., Ink::Primary).text(move || {
            if expanded.get() {
                "−".to_owned()
            } else {
                "+".to_owned()
            }
        }));
        head = head.child(toggle.grid_column(2));
    } else {
        head = head.child(close().grid_column(2));
    }
    let content = if compact {
        let item = notice.clone();
        let state = shell.clone();
        dynamic(move || -> Element {
            let open = expanded.get();
            let mut body =
                stack_panel().child(message(&state, &item, false, if open { 12 } else { 1 }));
            if open {
                body = body.child(
                    grid()
                        .column_definitions("*,Auto")
                        .child(action_bar(&state, &item, false))
                        .child(close().grid_column(1)),
                );
            }
            body.into()
        })
        .into_element()
    } else {
        stack_panel()
            .spacing(3)
            .child(message(shell, notice, banner, if banner { 2 } else { 12 }))
            .child(action_bar(shell, notice, banner))
            .into_element()
    };
    let inner: Element = match layout {
        Layout::Taskbar => stack_panel()
            .spacing(4.)
            .child(
                grid()
                    .column_definitions("Auto,*")
                    .child(sender_mark(shell, &notice.sender, 20.))
                    .child(head.grid_column(1).margin((8., 0., 0., 0.))),
            )
            .child(content)
            .into(),
        Layout::MenuBarDock | Layout::BottomNavigation => {
            let size = if layout == Layout::MenuBarDock {
                35.
            } else {
                24.
            };
            grid()
                .column_definitions("Auto,*")
                .child(
                    border()
                        .vertical_alignment(V::Top)
                        .margin((0., 5., 10., 0.))
                        .child(sender_mark(shell, &notice.sender, size)),
                )
                .child(
                    stack_panel()
                        .grid_column(1)
                        .spacing(3)
                        .child(head)
                        .child(content),
                )
                .into()
        }
    };
    let frame = border()
        .padding((12., 8.))
        .corner_radius(match layout {
            Layout::Taskbar => 6,
            Layout::MenuBarDock => 15,
            Layout::BottomNavigation => 20,
        })
        .background(if banner { "Transparent" } else { colors().tile })
        .child(inner);
    if layout == Layout::BottomNavigation {
        let service = shell.notifications.service.clone();
        frame
            .with(|area| {
                let swipe = SwipeGestureRecognizer::new();
                swipe.set_can_horizontally_swipe(true);
                swipe.set_is_mouse_enabled(true);
                swipe.set_threshold(60.);
                area.gesture_recognizers().add(swipe);
            })
            .on_swipe_gesture(move |event| {
                if event.delta().x != 0. {
                    service.dismiss(id);
                    event.set_handled(true);
                }
            })
            .into()
    } else {
        frame.into()
    }
}
