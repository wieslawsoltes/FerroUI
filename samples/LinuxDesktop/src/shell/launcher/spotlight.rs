//! Spotlight retains its editing host while the search field unfolds into results.
//! Result identifiers are catalog indices; the side pane resolves live windows separately.
use super::*;
use crate::services::ApplicationCatalog;
use crate::shell::Scene;

const PAGE_SIZE: usize = Results::List.capacity();

fn ranked(catalog: &ApplicationCatalog, query: &str) -> Vec<usize> {
    let mut indices = catalog.search_indices(query);
    let query = query.trim().to_lowercase();
    if !query.is_empty() {
        indices.sort_by_cached_key(|&index| {
            let name = catalog.applications()[index].name.to_lowercase();
            if name == query {
                0
            } else if name.starts_with(&query) {
                1
            } else if name.contains(&query) {
                2
            } else {
                3
            }
        });
    }
    indices
}

fn activate(shell: &Rc<Shell>, id: u32) {
    if let Some(session) = &shell.session {
        shell.dismiss();
        if !session.activate(id) {
            shell.open(Scene::Applications);
            shell.status.set("Could not activate this window".into());
        }
    }
}

fn open_app(shell: &Rc<Shell>, index: usize) {
    let target = shell.snapshot.with_untracked(|s| {
        s.windows
            .iter()
            .filter(|w| shell.catalog.window_index(&w.app_id) == Some(index))
            .max_by_key(|w| (w.active, w.last_activation))
            .map(|w| w.id)
    });
    if let Some(id) = target {
        activate(shell, id);
    } else {
        shell.launch(&shell.catalog.applications()[index]);
    }
}

fn focus_search(shell: &Shell) {
    if let Some(input) = shell
        .search_box
        .borrow()
        .as_ref()
        .and_then(|input| input.upgrade())
    {
        input.focus();
    }
}

fn result(
    shell: &Rc<Shell>,
    index: usize,
    select: impl Fn() + 'static,
    selected: Memo<Option<usize>>,
) -> Element {
    let app = &shell.catalog.applications()[index];
    let state = shell.clone();
    let open = shell.clone();
    let is_selected = memo(move |_| selected.get() == Some(index));
    let title = app.name.clone();
    let label = dynamic(move || {
        let text = shell_text(&title, 14., Ink::Primary);
        if is_selected.get() {
            text.foreground(resource("MacOSSelectionForegroundBrush"))
        } else {
            text
        }
    });
    grid()
        .child(
            border()
                .corner_radius(7.)
                .is_hit_test_visible(false)
                .background(resource("MacOSSelectionBrush"))
                .is_visible(move || is_selected.get()),
        )
        .child(
            action(&app.name, move || {
                select();
                focus_search(&state);
            })
            .on_double_tapped(move |event| {
                open_app(&open, index);
                event.set_handled(true);
            })
            .automation_properties_help_text(
                "Select to see details. Double-click or press Return to open.",
            )
            .horizontal_alignment(H::Stretch)
            .horizontal_content_alignment(H::Stretch)
            .height(42.)
            .padding((8., 3.))
            .corner_radius(7.)
            .background("Transparent")
            .border_thickness(0)
            .child(
                grid()
                    .column_definitions("34,*")
                    .child(app_icon(shell, app))
                    .child(border().grid_column(1).margin((8., 0.)).child(label)),
            ),
        )
        .into()
}

fn details(
    shell: &Rc<Shell>,
    selected: Memo<Option<usize>>,
    first_action: NodeRef<ferroui_controls::Button>,
) -> Element {
    let state = shell.clone();
    dynamic(move || -> Element {
        let Some(index) = selected.get() else {
            return Element::Empty;
        };
        let app = &state.catalog.applications()[index];
        let target = state.clone();
        let pin = state.clone();
        let pinned = state.favorites;
        let id = app.id.clone();
        let is_pinned = memo(move |_| pinned.with(|pins| pins.contains(&id)));
        let catalog = state.catalog.clone();
        let snapshot = state.snapshot;
        let windows = memo(move |_| {
            snapshot.with(|s| {
                let mut windows: Vec<_> = s
                    .windows
                    .iter()
                    .filter(|w| catalog.window_index(&w.app_id) == Some(index))
                    .map(|w| (w.id, w.title.clone(), w.last_activation))
                    .collect();
                windows.sort_by_key(|w| std::cmp::Reverse(w.2));
                windows
            })
        });
        let window_state = state.clone();
        let list = dynamic(move || -> Element {
            windows.with(|windows| {
                if windows.is_empty() {
                    return Element::Empty;
                }
                let mut list = stack_panel().spacing(3.0).child(
                    shell_text("Open windows", 11., Ink::Secondary)
                        .font_weight(FontWeight::SemiBold),
                );
                for (id, title, _) in windows {
                    let shell = window_state.clone();
                    let id = *id;
                    list = list.child(
                        action(title, move || activate(&shell, id))
                            .horizontal_content_alignment(H::Left)
                            .min_width(0.)
                            .padding((6., 2.))
                            .background("Transparent")
                            .border_thickness(0)
                            .child(
                                grid()
                                    .column_definitions("16,*")
                                    .child(glyph(Icon::Window, 13., colors().muted))
                                    .child(
                                        shell_text(title, 12., Ink::Primary)
                                            .grid_column(1)
                                            .margin((5., 0.)),
                                    ),
                            ),
                    );
                }
                list.into()
            })
        });
        let mut quick_actions = stack_panel().spacing(3.);
        if !app.actions.is_empty() {
            quick_actions = quick_actions.child(
                shell_text("Quick Actions", 11., Ink::Secondary).font_weight(FontWeight::SemiBold),
            );
        }
        for (ordinal, desktop_action) in app.actions.iter().enumerate() {
            let shell = state.clone();
            let id = desktop_action.id.clone();
            let command = action(&desktop_action.label, move || {
                shell.launch_entry(&shell.catalog.applications()[index], Some(id.clone()));
            })
            .horizontal_content_alignment(H::Left)
            .min_height(0.)
            .padding((6., 3.))
            .child(shell_text(&desktop_action.label, 12., Ink::Primary));
            quick_actions = quick_actions.child(if ordinal == 0 {
                command.node_ref(first_action)
            } else {
                command
            });
        }
        stack_panel()
            .spacing(7.0)
            .margin((12., 10.))
            .child(
                border()
                    .horizontal_alignment(H::Left)
                    .child(app_icon_sized(&state, app, 54.)),
            )
            .child(shell_text(&app.name, 19., Ink::Primary).font_weight(FontWeight::SemiBold))
            .child(shell_text(
                &format!("Application · {}", app.category),
                12.,
                Ink::Secondary,
            ))
            .child(
                shell_text(&app.description, 13., Ink::Primary)
                    .is_visible(!app.description.is_empty())
                    .text_wrapping(ferroui_base::media::TextWrapping::Wrap),
            )
            .child(shell_text(&app.id, 11., Ink::Secondary).tool_tip_tip(app.id.as_str()))
            .child(
                wrap_panel()
                    .child(
                        action("Open application", move || open_app(&target, index))
                            .height(28.)
                            .min_height(0.)
                            .padding((10., 1.))
                            .margin((0., 0., 5., 3.))
                            .child(shell_text("Open", 12., Ink::Primary)),
                    )
                    .child(
                        action("Change Dock pin", move || toggle_pin(&pin, index))
                            .height(28.)
                            .min_height(0.)
                            .padding((8., 1.))
                            .margin((0., 0., 0., 3.))
                            .automation_properties_name(move || {
                                if is_pinned.get() {
                                    "Remove from Dock"
                                } else {
                                    "Keep in Dock"
                                }
                            })
                            .child(shell_text("", 12., Ink::Primary).text(move || {
                                if is_pinned.get() {
                                    "Remove from Dock"
                                } else {
                                    "Keep in Dock"
                                }
                            })),
                    ),
            )
            .child(quick_actions)
            .child(list)
            .into()
    })
    .into()
}

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    let query = shell.query;
    let scene = shell.scene;
    let width = shell.metrics().width;
    let focused = signal(false);
    let browse_apps = signal(false);
    let pins_only = signal(false);
    let cursor = signal(0usize);
    let searching = memo(move |_| query.with(|q| !q.trim().is_empty()));
    let expanded = memo(move |_| searching.get() || browse_apps.get() || pins_only.get());
    let catalog = shell.catalog.clone();
    let pins = shell.favorites;
    let matches = memo(move |_| {
        if !expanded.get() {
            return Vec::new();
        }
        query.with(|q| {
            if pins_only.get() {
                pins.with(|pins| catalog.pinned_indices(pins, q))
            } else {
                ranked(&catalog, q)
            }
        })
    });
    let length = memo(move |_| matches.with(Vec::len));
    let position = memo(move |_| cursor.get().min(length.get().saturating_sub(1)));
    let selected = memo(move |_| matches.with(|items| items.get(position.get()).copied()));
    let page = memo(move |_| position.get() / PAGE_SIZE);
    let set_browse = move || {
        batch(|| {
            browse_apps.set(true);
            pins_only.set(false);
            cursor.set(0);
        })
    };
    let toggle_pins = move || {
        batch(|| {
            browse_apps.set(true);
            pins_only.set(!pins_only.get_untracked());
            cursor.set(0);
        })
    };
    effect(move || {
        if scene.get() != Scene::Applications {
            browse_apps.set_if_changed(false);
            pins_only.set_if_changed(false);
            cursor.set_if_changed(0);
        }
    });
    let first_action = NodeRef::<ferroui_controls::Button>::new();
    let key_state = shell.clone();
    let search = search_editor(shell, "Spotlight Search",
        "Type to search, use the arrows to select a result, and press Return to open. Command+1 browses applications.",
        24., focused, move || cursor.set_if_changed(0), move |event| {
            match (event.key, event.key_modifiers) {
                (Key::D1, KeyModifiers::META | KeyModifiers::CONTROL) => set_browse(),
                (Key::Down, KeyModifiers::NONE) if !expanded.get_untracked() => set_browse(),
                (key @ (Key::Up | Key::Down | Key::PageUp | Key::PageDown), KeyModifiers::NONE) => {
                    if let Some(i) = Results::List.navigate(key, position.get_untracked(), length.get_untracked(), true) {
                        cursor.set_if_changed(i);
                    } else { return; }
                }
                (Key::Tab, KeyModifiers::NONE) if selected.get_untracked()
                    .is_some_and(|i| !key_state.catalog.applications()[i].actions.is_empty()) => {
                    if !first_action.get_untracked().is_some_and(|button| button.focus()) { return; }
                }
                (Key::Return, KeyModifiers::NONE) => {
                    if let Some(i) = selected.get_untracked() { open_app(&key_state, i); } else { return; }
                }
                (Key::P, mods) if mods == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => toggle_pins(),
                (Key::P, KeyModifiers::CONTROL) => {
                    if let Some(i) = selected.get_untracked() { toggle_pin(&key_state, i); } else { return; }
                }
                (key @ (Key::Left | Key::Right), KeyModifiers::ALT) if pins_only.get_untracked() => {
                    if let Some(i) = selected.get_untracked() {
                        mutate_pin(&key_state, i, if key == Key::Left { Edit::Earlier } else { Edit::Later });
                    } else { return; }
                }
                _ => return,
            }
            event.set_handled(true);
        }).grid_column(1);
    let p = colors();
    let state = shell.clone();
    let browse = icon_action("Browse applications", Icon::Applications, move || {
        set_browse();
        focus_search(&state);
    })
    .tool_tip_tip("Applications (⌘1)")
    .width(32.)
    .height(32.)
    .padding(8.)
    .background("Transparent")
    .border_thickness(0)
    .grid_column(2);
    let state = shell.clone();
    let matches_list = dynamic(move || -> Element {
        let first = page.get() * PAGE_SIZE;
        let heading = if pins_only.get() {
            "Pinned applications"
        } else if searching.get() && first == 0 {
            "Top Hit"
        } else {
            "Applications"
        };
        let label = |s| {
            shell_text(s, 12., Ink::Secondary)
                .margin((8., 6., 0., 3.))
                .font_weight(FontWeight::SemiBold)
        };
        matches.with(|items| {
            if items.is_empty() {
                let message = state
                    .catalog
                    .unavailable_reason()
                    .unwrap_or(if pins_only.get() {
                        "No pinned applications"
                    } else {
                        "No matching applications"
                    });
                return shell_text(message, 14., Ink::Secondary).margin(12).into();
            }
            let mut rows = stack_panel().spacing(2.).child(label(heading));
            for (n, &index) in items.iter().skip(first).take(PAGE_SIZE).enumerate() {
                if n == 1 && heading == "Top Hit" {
                    rows = rows.child(label("Applications"));
                }
                rows = rows.child(result(
                    &state,
                    index,
                    move || cursor.set_if_changed(first + n),
                    selected,
                ));
            }
            rows.into()
        })
    });
    let recent = browse::recent(
        shell,
        Layout::MenuBarDock,
        memo(move |_| browse_apps.get() && !searching.get() && !pins_only.get() && page.get() == 0),
    );
    let narrow = memo(move |_| width.get() < 620.);
    let pane = details(shell, selected, first_action);
    let body = grid()
        .row_definitions("Auto,Auto")
        .column_definitions("3*,2*")
        .child(
            stack_panel()
                .grid_column_span(move || if narrow.get() { 2 } else { 1 })
                .child(recent)
                .child(matches_list),
        )
        .child(
            border()
                .grid_column(move || if narrow.get() { 0 } else { 1 })
                .grid_row(move || if narrow.get() { 1 } else { 0 })
                .grid_column_span(move || if narrow.get() { 2 } else { 1 })
                .border_brush(p.edge)
                .border_thickness(move || {
                    if narrow.get() {
                        (0., 1., 0., 0.)
                    } else {
                        (1., 0., 0., 0.)
                    }
                })
                .is_visible(move || selected.get().is_some())
                .child(pane),
        );
    let pages = memo(move |_| length.get().div_ceil(PAGE_SIZE).max(1));
    let prev = icon_action("Previous results", Icon::Back, move || {
        cursor.set(page.get_untracked().saturating_sub(1) * PAGE_SIZE)
    })
    .is_enabled(move || page.get() > 0);
    let next = icon_action("Next results", Icon::Forward, move || {
        cursor.set((page.get_untracked() + 1) * PAGE_SIZE)
    })
    .is_enabled(move || page.get() + 1 < pages.get());
    let pin_state = shell.clone();
    let chrome = grid()
        .row_definitions("Auto,*,Auto,Auto")
        .child(
            grid()
                .column_definitions("30,*,Auto")
                .margin((13., 7.))
                .child(glyph(Icon::Search, 24., p.muted))
                .child(search)
                .child(browse),
        )
        .child(
            scroll_viewer()
                .grid_row(1)
                .margin((8., 0., 8., 2.))
                .max_height(400.)
                .is_visible(move || expanded.get())
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                .child(body),
        )
        .child(
            grid()
                .grid_row(2)
                .column_definitions("*,Auto")
                .margin((12., 2., 12., 4.))
                .is_visible(move || expanded.get())
                .child(
                    action("Change application scope", move || {
                        toggle_pins();
                        focus_search(&pin_state);
                    })
                    .horizontal_alignment(H::Left)
                    .background("Transparent")
                    .border_thickness(0)
                    .min_height(0.)
                    .padding((6., 3.))
                    .child(shell_text("", 12., Ink::Secondary).text(move || {
                        if pins_only.get() {
                            "All applications"
                        } else {
                            "Pinned applications"
                        }
                    })),
                )
                .child(
                    row(4.)
                        .grid_column(1)
                        .is_visible(move || pages.get() > 1)
                        .child(prev)
                        .child(
                            shell_text("", 12., Ink::Secondary)
                                .text(move || format!("{} / {}", page.get() + 1, pages.get())),
                        )
                        .child(next),
                ),
        )
        .child(feedback(shell.status).grid_row(3));
    // Expanding results grows downwards from the field at a stable screen position.
    grid()
        .row_definitions("0.24*,0.76*")
        .is_visible(move || scene.get() == Scene::Applications)
        .child(
            crate::shell::desktop::popup_frame()
                .grid_row(1)
                .margin((10., 0., 10., 8.))
                .vertical_alignment(V::Top)
                .width(move || (width.get() - 24.).clamp(0., 660.))
                .padding(0.)
                .automation_properties_name("Spotlight")
                .child(chrome),
        )
        .into()
}
