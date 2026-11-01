//! Keyboard-first search with a bounded number of mounted results. Changing the
//! query leaves the text box, its selection and its input-method context intact.
use super::appearance::Layout;
use super::chrome::*;
use super::{feedback, popup_header, Shell};
use crate::services::{
    favorites::{self, Edit},
    DesktopApplication,
};
use ferroui_base::input::{Key, KeyModifiers};
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
    let pinned_only = signal(false);
    let favorites = shell.favorites;
    let matches = memo(move |_| {
        query.with(|q| {
            if pinned_only.get() {
                favorites.with(|pins| catalog.pinned_indices(pins, q))
            } else {
                catalog.search_indices(q)
            }
        })
    });

    let pages = memo(move |_| matches.with(|items| items.len().div_ceil(PAGE_SIZE).max(1)));
    let active_page = memo(move |_| page.get().min(pages.get() - 1));
    let field = NodeRef::<ferroui_controls::TextBox>::new();
    let enter = shell.clone();
    let shell_focus = shell.clone();
    let result_shell = shell.clone();
    let catalogue_status = shell.catalog.unavailable_reason().map(str::to_owned);
    let launching = shell.launching;
    let results = dynamic(move || -> Element {
        matches.with(|items| {
            if items.is_empty() {
                let (heading, message) = if let Some(error) = &catalogue_status {
                    ("Application catalog unavailable", error.as_str())
                } else if pinned_only.get() && query.with(|q| q.trim().is_empty()) {
                    ("No pinned applications", "Choose All apps, then pin an application to add it here and to the desktop.")
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
                    .child(shell_text(heading, 16.0, Ink::Primary))
                    .child(
                        shell_text(message, 12.0, Ink::Secondary)
                            .text_wrapping(ferroui_base::media::TextWrapping::Wrap),
                    )
                    .into();
            }
            let offset = active_page.get() * PAGE_SIZE;
            let items = items.iter().skip(offset).take(PAGE_SIZE).copied();
            if look().layout() == Layout::Taskbar && !pinned_only.get() {
                let mut tiles = grid()
                    .column_definitions("*,*,*")
                    .row_definitions("Auto,Auto");
                for (position, index) in items.enumerate() {
                    let shell = result_shell.clone();
                    let application = &result_shell.catalog.applications()[index];
                    let name = &application.name;
                    let pin = pin_button(&result_shell, index);
                    let launch = action(&format!("Launch {name}"), move || {
                            shell.launch(&shell.catalog.applications()[index])
                        })

                        .padding((5.0, 7.0))
                        .height(78.0)
                        .horizontal_alignment(H::Stretch)
                        .horizontal_content_alignment(H::Stretch)
                        .child(
                            stack_panel()
                                .spacing(5.0)
                                .child(app_icon(&result_shell, application))
                                .child(
                                    shell_text(name, 12.0, Ink::Primary)
                                        .horizontal_alignment(H::Center),
                                ),
                        );
                    tiles = tiles.child(grid()
                        .grid_row((position / 3) as i32)
                        .grid_column((position % 3) as i32)
                        .margin(2)
                        .child(launch)
                        .child(pin.horizontal_alignment(H::Right).vertical_alignment(V::Top)));
                }
                tiles.into()
            } else {
                let buttons: Vec<Element> = items
                    .map(|index| result_row(&result_shell, index, pinned_only.get()))
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
        .with(move |input| {
            shell_focus.search_box.replace(Some(input.downgrade()));
            // The native text box handles arrow movement during bubbling. Intercept
            // only the pinned-item reorder gesture before it moves the text caret.
            let state = shell_focus.clone();
            let event = ferroui_base::input::InputElement::key_down_event();
            let token = input.add_handler_with(
                event,
                move |_, e| {
                    if pinned_only.get_untracked()
                        && e.key_modifiers == KeyModifiers::ALT
                        && matches!(e.key, Key::Left | Key::Right)
                    {
                        if let Some(index) = matches.with(|items| {
                            items.get(active_page.get_untracked() * PAGE_SIZE).copied()
                        }) {
                            mutate_pin(
                                &state,
                                index,
                                if e.key == Key::Left {
                                    Edit::Earlier
                                } else {
                                    Edit::Later
                                },
                            );
                        }
                        e.set_handled(true);
                    }
                },
                ferroui_base::interactivity::RoutingStrategies::TUNNEL,
                false,
            );
            let weak = input.downgrade();
            on_cleanup(move || {
                if let Some(input) = weak.upgrade() {
                    input.remove_handler(event, token);
                }
            });
        })
        .node_ref(field)
        .on_loaded(move |_| {
            if let Some(input) = field.get_untracked() {
                input.focus();
            }
        })
        .on_key_down(move |e| {
            let index =
                matches.with(|items| items.get(active_page.get_untracked() * PAGE_SIZE).copied());
            if e.key == Key::P && e.key_modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) {
                batch(|| {
                    pinned_only.set(!pinned_only.get_untracked());
                    page.set_if_changed(0);
                });
                e.set_handled(true);
            } else if e.key == Key::P && e.key_modifiers == KeyModifiers::CONTROL {
                if let Some(index) = index {
                    toggle_pin(&enter, index);
                }
                e.set_handled(true);
            } else if e.key == Key::Return {
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
        page.set_if_changed(active_page.get_untracked().saturating_sub(1))
    })
    .is_enabled(move || active_page.get() > 0);
    let next = icon_action("Next results", Icon::Forward, move || {
        if active_page.get_untracked() + 1 < pages.get_untracked() {
            page.set_if_changed(active_page.get_untracked() + 1);
        }
    })
    .is_enabled(move || active_page.get() + 1 < pages.get());

    let (title, caption) = match look().layout() {
        Layout::Taskbar => ("Start", "APPLICATIONS"),
        Layout::MenuBarDock => ("Search applications", "SPOTLIGHT"),
        Layout::BottomNavigation => ("All apps", "APP DRAWER"),
    };
    let scene = shell.scene;
    super::desktop::popup_frame()
        .is_visible(move || scene.get() == super::Scene::Applications)
        .max_width(560.0)
        .max_height(690.0)
        .padding(22.0)
        .child(
            grid()
                .row_definitions("Auto,Auto,Auto,*,Auto,Auto")
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
                    row(6.0)
                        .grid_row(2)
                        .margin((0.0, 0.0, 0.0, 8.0))
                        .child(scope_button("All apps", false, pinned_only, page))
                        .child(scope_button("Pinned", true, pinned_only, page)),
                )
                .child(
                    scroll_viewer()
                        .grid_row(3)
                        .max_height(350.0)
                        .is_enabled(move || !launching.get())
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(results),
                )
                .child(
                    grid()
                        .grid_row(4)
                        .column_definitions("*,Auto")
                        .margin((0.0, 10.0, 0.0, 0.0))
                        .child(shell_text("", 12.0, Ink::Secondary).text(move || {
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
                                .child(shell_text("", 12.0, Ink::Primary).text(move || {
                                    format!("{} / {}", active_page.get() + 1, pages.get())
                                }))
                                .child(next),
                        ),
                )
                .child(feedback(shell.status).grid_row(5)),
        )
        .into()
}

fn scope_button(
    name: &'static str,
    pinned: bool,
    current: Signal<bool>,
    page: Signal<usize>,
) -> El<ferroui_controls::Button> {
    let p = colors();
    action(name, move || {
        batch(|| {
            current.set_if_changed(pinned);
            page.set_if_changed(0);
        })
    })
    .height(31.0)
    .padding((11.0, 3.0))
    .background(move || {
        if current.get() == pinned {
            p.selected
        } else {
            p.tile
        }
    })
    .child(shell_text(name, 12.0, Ink::Primary))
}

fn mutate_pin(shell: &Rc<Shell>, index: usize, edit: Edit) {
    if shell.pin_editing.get_untracked() {
        return;
    }
    let id = shell.catalog.applications()[index].id.clone();
    let weak = Rc::downgrade(shell);
    shell.pin_editing.set(true);
    let _task = ferroui_system::threading::tasks::spawn_local(async move {
        let result =
            ferroui_system::threading::tasks::run(move || Ok(favorites::update(&id, edit)?)).await;
        if let Some(shell) = weak.upgrade().filter(|shell| shell.owner.is_alive()) {
            shell.pin_editing.set(false);
            match result {
                Ok(pins) => shell.favorites.set_if_changed(pins),
                Err(error) => shell
                    .status
                    .set(format!("Could not update pinned applications. {error}")),
            }
        }
        Ok(())
    });
}

fn toggle_pin(shell: &Rc<Shell>, index: usize) {
    let id = &shell.catalog.applications()[index].id;
    let pinned = shell.favorites.with_untracked(|pins| pins.contains(id));
    mutate_pin(shell, index, if pinned { Edit::Unpin } else { Edit::Pin });
}

fn pin_button(shell: &Rc<Shell>, index: usize) -> El<ferroui_controls::Button> {
    let application = &shell.catalog.applications()[index];
    let pinned = shell.favorites.with(|ids| ids.contains(&application.id));
    let state = shell.clone();
    let busy = shell.pin_editing;
    icon_action(
        &format!(
            "{} {}",
            if pinned { "Unpin" } else { "Pin" },
            application.name
        ),
        if pinned { Icon::Unpin } else { Icon::Pin },
        move || toggle_pin(&state, index),
    )
    .width(29.0)
    .height(29.0)
    .padding(7)
    .background("Transparent")
    .border_thickness(0)
    .is_enabled(move || !busy.get())
}

fn result_row(shell: &Rc<Shell>, index: usize, pinned_only: bool) -> Element {
    let mut edits = row(1.0).vertical_alignment(V::Center);
    if pinned_only {
        let app = &shell.catalog.applications()[index];
        let (position, len) = shell
            .favorites
            .with(|pins| (pins.iter().position(|p| p == &app.id), pins.len()));
        for (title, icon, edit, enabled) in [
            (
                "Move earlier",
                Icon::Back,
                Edit::Earlier,
                position.is_some_and(|i| i > 0),
            ),
            (
                "Move later",
                Icon::Forward,
                Edit::Later,
                position.is_some_and(|i| i + 1 < len),
            ),
        ] {
            let state = shell.clone();
            let busy = shell.pin_editing;
            edits = edits.child(
                icon_action(&format!("{title}: {}", app.name), icon, move || {
                    mutate_pin(&state, index, edit)
                })
                .width(27.0)
                .height(29.0)
                .padding(6)
                .background("Transparent")
                .border_thickness(0)
                .is_enabled(move || enabled && !busy.get()),
            );
        }
    }
    grid()
        .column_definitions("*,Auto")
        .child(result_button(shell, index))
        .child(edits.grid_column(1).child(pin_button(shell, index)))
        .into()
}

/// A bounded group in the taskbar, Dock or home screen. The launcher exposes
/// the complete ordered collection even when space on a compact desktop is limited.
pub(super) fn pinned_shortcuts(shell: &Rc<Shell>) -> Element {
    let favorites = shell.favorites;
    let pin_limit = shell.metrics().pins;
    let material_home = look().layout() == Layout::BottomNavigation;
    let catalog = shell.catalog.clone();
    let state = shell.clone();
    for_each(
        move || {
            favorites.with(|ids| {
                ids.iter()
                    .filter_map(|id| catalog.index_of(id))
                    .take(if material_home { 4 } else { pin_limit.get() })
                    .collect::<Vec<_>>()
            })
        },
        |index| *index,
        move |index| {
            let shell = state.clone();
            let app = &state.catalog.applications()[index];
            action(&format!("Launch pinned {}", app.name), move || {
                shell.launch(&shell.catalog.applications()[index])
            })
            .width(36.0)
            .height(36.0)
            .padding(2)
            .background("Transparent")
            .border_thickness(0)
            .child(app_icon(&state, app))
        },
    )
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
            .child(app_icon(shell, application))
            .child(
                stack_panel()
                    .grid_column(1)
                    .margin((12.0, 0.0))
                    .vertical_alignment(V::Center)
                    .child(
                        shell_text(&application.name, 14.0, Ink::Primary)
                            .font_weight(FontWeight::SemiBold),
                    )
                    .child(shell_text(subtitle, 11.0, Ink::Secondary)),
            )
            .child(
                border()
                    .grid_column(2)
                    .child(glyph(Icon::Forward, 10.0, colors().muted)),
            ),
    )
    .into()
}

fn app_icon(shell: &Shell, application: &DesktopApplication) -> Element {
    if let Some(bitmap) = shell.icons.get(application) {
        let source: Rc<dyn ferroui_base::media::IImage> = bitmap;
        return image()
            .source(source)
            .width(32.0)
            .height(32.0)
            .stretch(ferroui_base::media::Stretch::Uniform)
            .is_hit_test_visible(false)
            .into();
    }

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
            shell_text(&initial, 16.0, fg)
                .font_weight(FontWeight::SemiBold)
                .horizontal_alignment(H::Center),
        )
        .into()
}
