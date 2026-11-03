//! The Windows All apps presentation and the app/category items it can select.
use super::*;
use crate::services::preferences;
use std::{fs, io};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StartView {
    Grid,
    List,
    Categories,
}

impl StartView {
    pub(super) const ALL: [Self; 3] = [Self::Grid, Self::List, Self::Categories];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::List => "List",
            Self::Categories => "Category",
        }
    }

    pub(super) fn preferred() -> Self {
        preferences::config_path("start-view")
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|s| Self::ALL.into_iter().find(|v| v.label() == s.trim()))
            .unwrap_or(Self::Grid)
    }

    pub(super) fn save(self) -> io::Result<()> {
        preferences::replace(
            &preferences::config_path("start-view")?,
            self.label().as_bytes(),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Entry {
    App(usize),
    Category(&'static str),
}

impl Entry {
    pub(super) fn app(self) -> Option<usize> {
        match self {
            Self::App(index) => Some(index),
            Self::Category(_) => None,
        }
    }
}

pub(super) fn category_tile(
    shell: &Shell,
    label: &'static str,
    selected: Memo<Option<Entry>>,
    open: impl Fn() + 'static,
) -> Element {
    let apps = &shell.catalog.groups()[label];
    let p = colors();
    let mosaic = uniform_grid()
        .rows(2)
        .columns(2)
        .width(70.0)
        .height(70.0)
        .horizontal_alignment(H::Center)
        .children(
            apps.iter()
                .take(4)
                .map(|&i| app_icon(shell, &shell.catalog.applications()[i]))
                .collect::<Vec<_>>(),
        );
    let count = apps.len();
    action(&format!("{label}, {count} applications"), open)
        .automation_properties_help_text("Open this app category")
        .tool_tip_tip(label)
        .margin(3)
        .padding((5.0, 8.0))
        .horizontal_alignment(H::Stretch)
        .horizontal_content_alignment(H::Stretch)
        .background(p.tile)
        .border_thickness(1)
        .border_brush(move || {
            if selected.get() == Some(Entry::Category(label)) {
                p.accent
            } else {
                p.edge
            }
        })
        .child(
            stack_panel().spacing(5.0).child(mosaic).child(
                shell_text(label, 13.0, Ink::Primary)
                    .font_weight(FontWeight::SemiBold)
                    .horizontal_alignment(H::Center),
            ),
        )
        .into()
}

/// Recent means a successful launch in this shell session. No document paths
/// or external application activity are collected.
pub(super) fn recent(shell: &Rc<Shell>, layout: Layout, visible: Memo<bool>) -> Element {
    let state = shell.clone();
    dynamic(move || -> Element {
        if !visible.get() {
            return Element::Empty;
        }
        state.recent_apps.with(|recent| {
            if recent.is_empty() {
                return Element::Empty;
            }
            let width = state.metrics().width.get();
            let columns = if layout == Layout::MenuBarDock {
                if width < 450.0 {
                    1
                } else {
                    2
                }
            } else {
                (((width - 72.0).max(0.0) / 88.0) as usize).clamp(1, 4)
            };
            let title = match layout {
                Layout::Taskbar => "Recent",
                Layout::MenuBarDock => "Recent applications",
                Layout::BottomNavigation => "Recently opened",
            };
            let clear = state.clone();
            let header = grid()
                .column_definitions("*,Auto")
                .child(
                    shell_text(
                        title,
                        if layout == Layout::MenuBarDock {
                            12.0
                        } else {
                            15.0
                        },
                        Ink::Primary,
                    )
                    .font_weight(FontWeight::SemiBold),
                )
                .child(
                    action("Clear recent applications", move || {
                        clear.recent_apps.set(Vec::new())
                    })
                    .grid_column(1)
                    .height(27.0)
                    .min_height(0.0)
                    .padding((7.0, 0.0))
                    .background("Transparent")
                    .border_thickness(0)
                    .child(shell_text("Clear", 12.0, Ink::Secondary)),
                );
            let mut items = uniform_grid().columns(columns as i32);
            for &i in recent.iter().take(columns) {
                let app = &state.catalog.applications()[i];
                let shell = state.clone();
                let icon = app_icon(&state, app);
                let title = shell_text(&app.name, 12.0, Ink::Primary);
                let content: Element = if layout == Layout::MenuBarDock {
                    grid()
                        .column_definitions("32,*")
                        .child(icon)
                        .child(title.grid_column(1).margin((7.0, 0.0)))
                        .into()
                } else {
                    stack_panel()
                        .spacing(4.0)
                        .child(icon)
                        .child(title.horizontal_alignment(H::Center))
                        .into()
                };
                items = items.child(
                    action(&format!("Reopen {}", app.name), move || {
                        shell.launch(&shell.catalog.applications()[i])
                    })
                    .tool_tip_tip(app.name.as_str())
                    .background("Transparent")
                    .border_thickness(0)
                    .horizontal_alignment(H::Stretch)
                    .padding((3.0, 4.0))
                    .child(content),
                );
            }
            stack_panel()
                .spacing(3.0)
                .margin((5.0, 7.0, 5.0, 4.0))
                .child(header)
                .child(items)
                .into()
        })
    })
    .into()
}

pub(super) fn alphabetic_list(
    shell: &Rc<Shell>,
    indices: impl Iterator<Item = usize>,
    selected: Memo<Option<usize>>,
) -> Element {
    let mut list = stack_panel().spacing(2.0);
    let mut previous = None;
    for index in indices {
        let app = &shell.catalog.applications()[index];
        let initial = app
            .name
            .trim_start()
            .chars()
            .next()
            .filter(|c| c.is_alphabetic())
            .and_then(|c| c.to_uppercase().next())
            .unwrap_or('#');
        if previous != Some(initial) {
            list = list.child(
                shell_text(&initial.to_string(), 15.0, Ink::Accent)
                    .font_weight(FontWeight::SemiBold)
                    .margin((12.0, 5.0, 0.0, 2.0)),
            );
            previous = Some(initial);
        }
        let state = shell.clone();
        let menu = pin_menu(shell, index);
        let p = colors();
        list = list.child(
            action(&format!("Launch {}", app.name), move || {
                state.launch(&state.catalog.applications()[index])
            })
            .height(41.0)
            .padding((10.0, 2.0))
            .horizontal_alignment(H::Stretch)
            .horizontal_content_alignment(H::Stretch)
            .with(move |b| b.set_context_menu(Some(menu.clone())))
            .with(reveal_selection(index, selected))
            .background(move || {
                if selected.get() == Some(index) {
                    p.selected
                } else {
                    "Transparent"
                }
            })
            .border_thickness(0)
            .child(row(10.0).child(app_icon(shell, app)).child(shell_text(
                &app.name,
                14.0,
                Ink::Primary,
            ))),
        );
    }
    list.into()
}
