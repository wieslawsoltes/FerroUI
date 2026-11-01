//! Keyboard-first search with a bounded number of mounted results. Changing the
//! query leaves the text box, its selection and its input-method context intact.
use super::appearance::Layout;
use super::chrome::*;
use super::{feedback, popup_header, Shell};
use crate::services::DesktopApplication;
use ferroui_base::input::Key;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;

const PAGE_SIZE: usize = 6;

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    let query = shell.query;
    let page = shell.page;
    let catalog = shell.catalog.clone();
    let matches = memo(move |_| query.with(|q| catalog.search_indices(q).collect::<Vec<_>>()));

    let pages = memo(move |_| matches.with(|items| items.len().div_ceil(PAGE_SIZE).max(1)));
    let field = NodeRef::<ferroui_controls::TextBox>::new();
    let enter = shell.clone();
    let result_shell = shell.clone();
    let catalogue_status = shell.catalog.unavailable_reason().map(str::to_owned);
    let launching = shell.launching;
    let results = dynamic(move || -> Element {
        matches.with(|items| {
            if items.is_empty() {
                let (heading, message) = if let Some(error) = &catalogue_status {
                    ("Application catalog unavailable", error.as_str())
                } else if query.with(|q| q.trim().is_empty()) {
                    (
                        "No installed applications",
                        "Install an application with an XDG desktop entry to see it here.",
                    )
                } else {
                    (
                        "No results",
                        "Try another name, description or desktop file ID.",
                    )
                };
                return stack_panel()
                    .spacing(7.0)
                    .margin((8.0, 16.0))
                    .child(label(heading, 16.0, Ink::Primary))
                    .child(
                        label(message, 12.0, Ink::Secondary)
                            .text_wrapping(ferroui_base::media::TextWrapping::Wrap),
                    )
                    .into();
            }
            let offset = page.get().min(pages.get() - 1) * PAGE_SIZE;
            let items = items.iter().skip(offset).take(PAGE_SIZE).copied();
            if look().layout() == Layout::Taskbar {
                let mut tiles = grid()
                    .column_definitions("*,*,*")
                    .row_definitions("Auto,Auto");
                for (position, index) in items.enumerate() {
                    let shell = result_shell.clone();
                    let application = &result_shell.catalog.applications()[index];
                    let name = &application.name;
                    tiles = tiles.child(
                        action(&format!("Launch {name}"), move || {
                            shell.launch(&shell.catalog.applications()[index])
                        })
                        .grid_row((position / 3) as i32)
                        .grid_column((position % 3) as i32)
                        .margin(2)
                        .padding((5.0, 7.0))
                        .height(78.0)
                        .horizontal_content_alignment(H::Stretch)
                        .child(
                            stack_panel()
                                .spacing(5.0)
                                .child(monogram(application))
                                .child(
                                    label(name, 12.0, Ink::Primary).horizontal_alignment(H::Center),
                                ),
                        ),
                    );
                }
                tiles.into()
            } else {
                let buttons: Vec<Element> = items
                    .map(|index| result_button(&result_shell, index))
                    .collect();
                stack_panel().spacing(4.0).children(buttons).into()
            }
        })
    });
    let search = text_box()
        .name("ApplicationSearch")
        .automation_properties_name("Search installed applications")
        .placeholder_text("Search by name or description")
        .text(query.two_way())
        .on_changed(ferroui_controls::TextBox::text_property(), move |_| {
            page.set_if_changed(0);
        })
        .node_ref(field)
        .on_loaded(move |_| {
            if let Some(input) = field.get_untracked() {
                input.focus();
            }
        })
        .on_key_down(move |e| {
            if e.key == Key::Return {
                let index =
                    matches.with(|items| items.get(page.get_untracked() * PAGE_SIZE).copied());
                if let Some(index) = index {
                    enter.launch(&enter.catalog.applications()[index]);
                }
                e.set_handled(true);
            }
        })
        .grid_column(1)
        .foreground(colors().text)
        .background("Transparent")
        .border_thickness(0)
        .padding((8.0, 6.0))
        .vertical_alignment(V::Center);
    let previous = icon_action("Previous results", Icon::Back, move || {
        page.update(|value| *value = value.saturating_sub(1))
    })
    .is_enabled(move || page.get() > 0);
    let next = icon_action("Next results", Icon::Forward, move || {
        if page.get_untracked() + 1 < pages.get_untracked() {
            page.update(|value| *value += 1);
        }
    })
    .is_enabled(move || page.get() + 1 < pages.get());

    let (title, caption) = match look().layout() {
        Layout::Taskbar => ("Start", "APPLICATIONS"),
        Layout::MenuBarDock => ("Search applications", "SPOTLIGHT"),
        Layout::BottomNavigation => ("All apps", "APP DRAWER"),
    };
    super::desktop::popup_frame()
        .max_width(560.0)
        .max_height(690.0)
        .padding(22.0)
        .child(
            grid()
                .row_definitions("Auto,Auto,*,Auto,Auto")
                .child(popup_header(title, caption, shell))
                .child(
                    border()
                        .grid_row(1)
                        .margin((0.0, 14.0, 0.0, 10.0))
                        .padding((12.0, 3.0))
                        .corner_radius(9)
                        .border_brush(colors().edge)
                        .border_thickness(1)
                        .background(colors().tile)
                        .child(
                            grid()
                                .column_definitions("Auto,*")
                                .child(glyph(Icon::Search, 16.0, colors().accent))
                                .child(search),
                        ),
                )
                .child(
                    scroll_viewer()
                        .grid_row(2)
                        .max_height(350.0)
                        .is_enabled(move || !launching.get())
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(results),
                )
                .child(
                    grid()
                        .grid_row(3)
                        .column_definitions("*,Auto")
                        .margin((0.0, 10.0, 0.0, 0.0))
                        .child(label("", 12.0, Ink::Secondary).text(move || {
                            matches.with(|items| match items.len() {
                                1 => "1 application".to_owned(),
                                n => format!("{n} applications"),
                            })
                        }))
                        .child(
                            row(7.0)
                                .grid_column(1)
                                .is_visible(move || pages.get() > 1)
                                .child(previous)
                                .child(
                                    label("", 12.0, Ink::Primary).text(move || {
                                        format!("{} / {}", page.get() + 1, pages.get())
                                    }),
                                )
                                .child(next),
                        ),
                )
                .child(feedback(shell.status).grid_row(4)),
        )
        .into()
}

fn result_button(shell: &Rc<Shell>, index: usize) -> Element {
    let application = &shell.catalog.applications()[index];
    let state = shell.clone();
    let subtitle = if application.description.is_empty() {
        application
            .id
            .strip_suffix(".desktop")
            .unwrap_or(&application.id)
    } else {
        &application.description
    };
    action(&format!("Launch {}", application.name), move || {
        state.launch(&state.catalog.applications()[index])
    })
    .tool_tip_tip(application.name.as_str())
    .horizontal_alignment(H::Stretch)
    .horizontal_content_alignment(H::Stretch)
    .padding((10.0, 6.0))
    .height(49.0)
    .background(colors().tile)
    .child(
        grid()
            .column_definitions("32,*,18")
            .child(monogram(application))
            .child(
                stack_panel()
                    .grid_column(1)
                    .margin((12.0, 0.0))
                    .vertical_alignment(V::Center)
                    .child(
                        label(&application.name, 14.0, Ink::Primary)
                            .font_weight(FontWeight::SemiBold),
                    )
                    .child(label(subtitle, 11.0, Ink::Secondary)),
            )
            .child(
                border()
                    .grid_column(2)
                    .child(glyph(Icon::Forward, 10.0, colors().muted)),
            ),
    )
    .into()
}

fn monogram(application: &DesktopApplication) -> Element {
    const COLORS: [(&str, &str); 5] = [
        ("#264943", "#A9E5CA"),
        ("#35394F", "#CAC8F5"),
        ("#413F33", "#EECE9F"),
        ("#2B4154", "#A6D5EF"),
        ("#443947", "#EBC1DA"),
    ];
    let hash = application.id.bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u32::from(byte))
    });
    let (bg, fg) = COLORS[hash as usize % COLORS.len()];
    let initial = application
        .name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    border()
        .width(32.0)
        .height(32.0)
        .corner_radius(8)
        .background(bg)
        .child(
            label(&initial, 16.0, fg)
                .font_weight(FontWeight::SemiBold)
                .horizontal_alignment(H::Center),
        )
        .into()
}
