//! Keyboard-first search with a bounded number of mounted results. Changing the
//! query leaves the text box, its selection and its input-method context intact.
use super::appearance::Layout;
use super::chrome::*;
use super::{feedback, Shell};
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Results {
    List,
    Tiles { columns: usize, rows: usize },
}

impl Results {
    fn for_view(layout: Layout, width: f64, searching: bool, pinned: bool) -> Self {
        if pinned || layout == Layout::MenuBarDock || (layout == Layout::Taskbar && searching) {
            return Self::List;
        }
        let columns = ((width - 64.0) / 96.0) as usize;
        Self::Tiles {
            columns: columns.clamp(1, if layout == Layout::Taskbar { 6 } else { 5 }),
            rows: if layout == Layout::Taskbar { 2 } else { 3 },
        }
    }

    fn capacity(self) -> usize {
        match self {
            Self::List => 6,
            Self::Tiles { columns, rows } => columns * rows,
        }
    }
}

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    let layout = look().layout();
    let width = shell.metrics().width;
    let query = shell.query;
    let page = shell.page;
    let slot = signal(0usize);
    let catalog = shell.catalog.clone();
    let pinned_only = signal(false);
    let favorites = shell.favorites;
    let searching = memo(move |_| query.with(|q| !q.trim().is_empty()));
    let presentation =
        memo(move |_| Results::for_view(layout, width.get(), searching.get(), pinned_only.get()));
    let matches = memo(move |_| {
        query.with(|q| {
            if pinned_only.get() {
                favorites.with(|pins| catalog.pinned_indices(pins, q))
            } else {
                catalog.search_indices(q)
            }
        })
    });
    let count = memo(move |_| matches.with(Vec::len));
    let pages = memo(move |_| count.get().div_ceil(presentation.get().capacity()).max(1));
    let active_page = memo(move |_| page.get().min(pages.get() - 1));
    let offset = memo(move |_| active_page.get() * presentation.get().capacity());
    let selected = memo(move |_| {
        let position = offset.get() + slot.get().min(presentation.get().capacity() - 1);
        position.min(count.get().saturating_sub(1))
    });
    let selected_app = memo(move |_| matches.with(|items| items.get(selected.get()).copied()));
    let set_page = move |page_index| {
        batch(|| {
            page.set_if_changed(page_index);
            slot.set_if_changed(0);
        })
    };
    let toggle_scope = move || {
        batch(|| {
            pinned_only.set(!pinned_only.get_untracked());
            set_page(0);
        })
    };

    let state = shell.clone();
    let results = dynamic(move || -> Element {
        if count.get() == 0 {
            let (heading, message) = if let Some(error) = state.catalog.unavailable_reason() {
                ("Application catalog unavailable", error)
            } else if searching.get() {
                (
                    "No results",
                    "Try another name, description or desktop file ID.",
                )
            } else if pinned_only.get() {
                (
                    "No pinned applications",
                    "Choose All apps to pin an application.",
                )
            } else {
                (
                    "No installed applications",
                    "Install an application with an XDG desktop entry to see it here.",
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
        matches.with(|items| {
            let view = presentation.get();
            let items = items
                .iter()
                .skip(offset.get())
                .take(view.capacity())
                .copied();
            match view {
                Results::Tiles { columns, .. } => uniform_grid()
                    .columns(columns as i32)
                    .children(
                        items
                            .map(|index| result_tile(&state, index, selected_app, layout))
                            .collect::<Vec<_>>(),
                    )
                    .into(),
                Results::List => stack_panel()
                    .spacing(2.0)
                    .children(
                        items
                            .map(|index| result_row(&state, index, pinned_only.get(), selected_app))
                            .collect::<Vec<_>>(),
                    )
                    .into(),
            }
        })
    });

    let field = NodeRef::<ferroui_controls::TextBox>::new();
    let state = shell.clone();
    let search = text_box()
        .name("ApplicationSearch")
        .automation_properties_name("Search installed applications")
        .automation_properties_help_text("Use Up and Down to select an application. Press Enter to open it or Control+P to pin it.")
        .placeholder_text(match layout {
            Layout::Taskbar => "Search for apps",
            Layout::MenuBarDock => "Spotlight Search",
            Layout::BottomNavigation => "Search apps",
        })
        .text(query.two_way())
        .on_changed(ferroui_controls::TextBox::text_property(), move |_| set_page(0))
        .with(move |input| {
            let weak = input.downgrade();
            state.search_box.replace(Some(weak.clone()));
            let shell = state.clone();
            let event = ferroui_base::input::InputElement::key_down_event();
            // Route selection above TextBox's caret navigation. Leave composition,
            // horizontal editing and modified selection gestures to the text host.
            let token = input.add_handler_with(event, move |_, e| {
                if weak.upgrade().is_none_or(|input| input.is_composing_text()) {
                    return;
                }
                let capacity = presentation.get_untracked().capacity();
                let index = selected_app.get_untracked();
                match (e.key, e.key_modifiers) {
                    (Key::Down | Key::Up | Key::PageDown | Key::PageUp, KeyModifiers::NONE) => {
                        let current = selected.get_untracked();
                        let length = count.get_untracked();
                        if length == 0 { return; }
                        let next = match e.key {
                            Key::Down => current + 1,
                            Key::Up => current.saturating_sub(1),
                            Key::PageDown => current + capacity,
                            _ => current.saturating_sub(capacity),
                        }.min(length - 1);
                        batch(|| {
                            page.set_if_changed(next / capacity);
                            slot.set_if_changed(next % capacity);
                        });
                    }
                    (Key::Left | Key::Right, KeyModifiers::ALT) if pinned_only.get_untracked() => {
                        if let Some(index) = index {
                            mutate_pin(&shell, index, if e.key == Key::Left { Edit::Earlier } else { Edit::Later });
                        }
                    }
                    (Key::P, modifiers) if modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => toggle_scope(),
                    (Key::P, KeyModifiers::CONTROL) => {
                        if let Some(index) = index { toggle_pin(&shell, index); }
                    }
                    (Key::Return, KeyModifiers::NONE) => {
                        if let Some(index) = index { shell.launch(&shell.catalog.applications()[index]); }
                    }
                    _ => return,
                }
                e.set_handled(true);
            }, ferroui_base::interactivity::RoutingStrategies::TUNNEL, false);
            let weak = input.downgrade();
            on_cleanup(move || {
                if let Some(input) = weak.upgrade() { input.remove_handler(event, token); }
            });
        })
        .node_ref(field)
        .on_loaded(move |_| {
            if let Some(input) = field.get_untracked() { input.focus(); }
        })
        .grid_column(1)
        .foreground(colors().text)
        .background("Transparent")
        .border_thickness(0)
        .font_size(if layout == Layout::MenuBarDock { 23.0 } else { 15.0 })
        .padding((9.0, 5.0))
        .vertical_alignment(V::Center);

    let scope_name = move || {
        if pinned_only.get() {
            "All apps"
        } else {
            "Pinned"
        }
    };
    let section = move || match (pinned_only.get(), searching.get(), layout) {
        (true, _, _) => "Pinned applications",
        (_, true, _) => "Search results",
        (_, _, Layout::MenuBarDock) => "Applications",
        _ => "All apps",
    };
    let previous = icon_action("Previous results", Icon::Back, move || {
        set_page(active_page.get_untracked().saturating_sub(1));
    })
    .is_enabled(move || active_page.get() > 0);
    let next = icon_action("Next results", Icon::Forward, move || {
        set_page((active_page.get_untracked() + 1).min(pages.get_untracked() - 1));
    })
    .is_enabled(move || active_page.get() + 1 < pages.get());

    let state = shell.clone();
    let p = colors();
    let scene = shell.scene;
    let launching = shell.launching;
    let is_drawer = layout == Layout::BottomNavigation;
    let spotlight = layout == Layout::MenuBarDock;
    let content =
        grid()
            .row_definitions("Auto,Auto,Auto,Auto,*,Auto,Auto")
            .child(
                border()
                    .is_visible(is_drawer)
                    .width(36.0)
                    .height(4.0)
                    .corner_radius(2)
                    .horizontal_alignment(H::Center)
                    .margin((0.0, 0.0, 0.0, 14.0))
                    .background(p.muted),
            )
            .child(
                border()
                    .grid_row(1)
                    .background(if spotlight { "Transparent" } else { p.tile })
                    .border_brush(p.edge)
                    .border_thickness(if layout == Layout::Taskbar { 1 } else { 0 })
                    .corner_radius(if is_drawer { 26 } else { 8 })
                    .padding((12.0, 3.0))
                    .child(
                        grid()
                            .column_definitions("Auto,*,Auto")
                            .child(glyph(
                                Icon::Search,
                                if spotlight { 23.0 } else { 18.0 },
                                p.muted,
                            ))
                            .child(search)
                            .child(
                                icon_action("Close launcher", Icon::Close, move || state.cancel())
                                    .grid_column(2)
                                    .width(32.0)
                                    .height(32.0)
                                    .padding(8.0)
                                    .background("Transparent")
                                    .border_thickness(0),
                            ),
                    ),
            )
            .child(pinned_preview(
                shell,
                layout,
                searching,
                pinned_only,
                active_page,
            ))
            .child(
                grid()
                    .grid_row(3)
                    .column_definitions("*,Auto")
                    .margin((6.0, 9.0, 4.0, 5.0))
                    .child(
                        shell_text("", if spotlight { 12.0 } else { 16.0 }, Ink::Primary)
                            .text(section)
                            .font_weight(FontWeight::SemiBold),
                    )
                    .child(
                        action("Change application scope", toggle_scope)
                            .grid_column(1)
                            .automation_properties_name(scope_name)
                            .height(30.0)
                            .min_height(0.0)
                            .padding((10.0, 2.0))
                            .child(
                                row(6.0)
                                    .child(shell_text("", 12.0, Ink::Primary).text(scope_name))
                                    .child(glyph(Icon::Forward, 9.0, p.muted)),
                            ),
                    ),
            )
            .child(
                scroll_viewer()
                    .grid_row(4)
                    .max_height(345.0)
                    .is_enabled(move || !launching.get())
                    .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                    .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                    .child(results),
            )
            .child(
                grid()
                    .grid_row(5)
                    .column_definitions("*,Auto")
                    .margin((6.0, 7.0, 4.0, 0.0))
                    .child(
                        shell_text("", 12.0, Ink::Secondary).text(move || match count.get() {
                            1 => "1 application".to_owned(),
                            n => format!("{n} applications"),
                        }),
                    )
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
            .child(feedback(shell.status).grid_row(6));
    super::desktop::popup_frame()
        .automation_properties_name(match layout {
            Layout::Taskbar => "Start",
            Layout::MenuBarDock => "Spotlight",
            Layout::BottomNavigation => "App drawer",
        })
        .is_visible(move || scene.get() == super::Scene::Applications)
        .max_width(match layout {
            Layout::Taskbar => 650.0,
            Layout::MenuBarDock => 610.0,
            Layout::BottomNavigation => 540.0,
        })
        .max_height(690.0)
        .padding(if spotlight { 13.0 } else { 19.0 })
        .child(content)
        .into()
}

// Start keeps real user pins within reach above the alphabetic catalog.
// The scoped view exposes every pin, including entries outside this preview.
fn pinned_preview(
    shell: &Rc<Shell>,
    layout: Layout,
    searching: Memo<bool>,
    pinned_only: Signal<bool>,
    page: Memo<usize>,
) -> El<ferroui_controls::Border> {
    let frame = border().grid_row(2);
    if layout != Layout::Taskbar {
        return frame;
    }
    let pins = shell.favorites;
    let width = shell.metrics().width;
    let state = shell.clone();
    frame.child(dynamic(move || -> Element {
        if searching.get() || pinned_only.get() || page.get() != 0 {
            return Element::Empty;
        }
        let columns = ((width.get() - 64.0) / 96.0) as usize;
        let indices = pins.with(|ids| {
            ids.iter()
                .filter_map(|id| state.catalog.index_of(id))
                .take(columns.clamp(1, 6))
                .collect::<Vec<_>>()
        });
        if indices.is_empty() {
            return Element::Empty;
        }
        let mut preview = uniform_grid().columns(columns.clamp(1, 6) as i32);
        for index in indices {
            let app = &state.catalog.applications()[index];
            let launch = state.clone();
            preview = preview.child(
                action(&format!("Launch pinned {}", app.name), move || {
                    launch.launch(&launch.catalog.applications()[index]);
                })
                .height(67.0)
                .horizontal_alignment(H::Stretch)
                .background("Transparent")
                .border_thickness(0)
                .padding(2.0)
                .child(
                    stack_panel()
                        .spacing(2.0)
                        .child(app_icon(&state, app))
                        .child(
                            shell_text(&app.name, 12.0, Ink::Primary)
                                .horizontal_alignment(H::Center),
                        ),
                ),
            );
        }
        stack_panel()
            .margin((6.0, 13.0, 4.0, 0.0))
            .child(shell_text("Pinned", 16.0, Ink::Primary).font_weight(FontWeight::SemiBold))
            .child(preview)
            .into()
    }))
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

/// Grid tiles expose the desktop's native context gesture without a row of
/// pin buttons over the application artwork. The explicit pin button remains
/// available in every list, including search on Windows and pinned management.
fn result_tile(
    shell: &Rc<Shell>,
    index: usize,
    selected: Memo<Option<usize>>,
    layout: Layout,
) -> Element {
    let app = &shell.catalog.applications()[index];
    let state = shell.clone();
    let pinned = shell.favorites.with(|ids| ids.contains(&app.id));
    let pin = ferroui_controls::MenuItem::new();
    pin.set_header(Some(Rc::new(if pinned {
        "Unpin application".to_owned()
    } else {
        "Pin application".to_owned()
    })));
    let edit = shell.clone();
    pin.add_handler(ferroui_controls::MenuItem::click_event(), move |_, _| {
        toggle_pin(&edit, index)
    });
    let menu = ferroui_controls::ContextMenu::new();
    menu.items()
        .add(Some(ferroui_controls::Control::boxed(pin)));
    let p = colors();
    let material = layout == Layout::BottomNavigation;
    let artwork = if material {
        border()
            .width(46.0)
            .height(46.0)
            .corner_radius(23)
            .horizontal_alignment(H::Center)
            .background(p.tile)
            .child(app_icon(shell, app))
            .into()
    } else {
        app_icon(shell, app)
    };
    action(&format!("Launch {}", app.name), move || {
        state.launch(&state.catalog.applications()[index])
    })
    .context_menu(Some(menu))
    .automation_properties_help_text("Open the context menu to pin or unpin this application.")
    .height(if material { 88.0 } else { 76.0 })
    .horizontal_alignment(H::Stretch)
    .horizontal_content_alignment(H::Stretch)
    .margin(2.0)
    .padding((4.0, 3.0))
    .corner_radius(if material { 16 } else { 6 })
    .background("Transparent")
    .border_brush(move || {
        if selected.get() == Some(index) {
            p.accent
        } else {
            "Transparent"
        }
    })
    .border_thickness(1)
    .child(
        stack_panel()
            .spacing(5.0)
            .child(artwork)
            .child(shell_text(&app.name, 12.0, Ink::Primary).horizontal_alignment(H::Center)),
    )
    .into()
}

fn result_row(
    shell: &Rc<Shell>,
    index: usize,
    pinned_only: bool,
    selected: Memo<Option<usize>>,
) -> Element {
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
    let p = colors();
    let spotlight = look().layout() == Layout::MenuBarDock;
    border()
        .corner_radius(if spotlight { 7 } else { 6 })
        .border_thickness(1)
        .border_brush(move || {
            if selected.get() == Some(index) {
                p.accent
            } else {
                "Transparent"
            }
        })
        .background(move || {
            if selected.get() == Some(index) && spotlight {
                p.selected
            } else {
                "Transparent"
            }
        })
        .child(
            grid()
                .column_definitions("*,Auto")
                .child(result_button(shell, index))
                .child(edits.grid_column(1).child(pin_button(shell, index))),
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
    .background("Transparent")
    .border_thickness(0)
    .child(
        grid()
            .column_definitions("32,*")
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
            ),
    )
    .into()
}

pub(super) fn app_icon(shell: &Shell, application: &DesktopApplication) -> Element {
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

    fallback_icon(&application.id, &application.name)
}

pub(super) fn fallback_icon(id: &str, name: &str) -> Element {
    const COLORS: [(&str, &str); 5] = [
        ("#264943", "#A9E5CA"),
        ("#35394F", "#CAC8F5"),
        ("#413F33", "#EECE9F"),
        ("#2B4154", "#A6D5EF"),
        ("#443947", "#EBC1DA"),
    ];
    let hash = id.bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(u32::from(byte))
    });
    let (bg, fg) = COLORS[hash as usize % COLORS.len()];
    let initial = name
        .trim_start()
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
