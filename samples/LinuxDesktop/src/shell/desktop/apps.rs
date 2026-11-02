//! Pinned and running apps share one slot. Only the small set of visible
//! buttons is mounted; the overview still lists every individual window.
use super::*;
use crate::services::ApplicationCatalog;
use crate::session::Snapshot;

#[derive(Clone, Eq, PartialEq)]
struct Window {
    id: u32,
    active: bool,
    can_minimize: bool,
}

#[derive(Clone, Eq, PartialEq)]
struct Entry {
    app: Option<usize>,
    label: String,
    native_id: String,
    windows: Vec<Window>,
}

impl Entry {
    fn key(&self) -> i64 {
        self.app
            .map_or_else(|| -1 - i64::from(self.windows[0].id), |index| index as i64)
    }

    fn invoke(&self, shell: &Rc<Shell>, layout: Layout) {
        if self.windows.is_empty() {
            if let Some(index) = self.app {
                shell.launch(&shell.catalog.applications()[index]);
            }
        } else if let Some(session) = &shell.session {
            let active = self.windows.iter().position(|w| w.active);
            let next = active.map_or(0, |i| (i + 1) % self.windows.len());
            let target = &self.windows[next];
            if layout == Layout::Taskbar && target.active && target.can_minimize {
                session.minimize(target.id);
            } else {
                session.activate(target.id);
            }
        }
    }
}

/// Space occupied by the built-in commands, tray, margins and separators,
/// measured in the same logical pixels as the containing panel. A centered
/// Windows taskbar needs to leave tray-sized space on both sides.
fn slot_capacity(layout: Layout, width: f64) -> usize {
    let (fixed, pitch) = match layout {
        Layout::Taskbar if width >= 900.0 => (590.0, 45.0),
        Layout::Taskbar => (325.0, 45.0),
        Layout::MenuBarDock => (192.0, 51.0),
        Layout::BottomNavigation => (170.0, 52.0),
    };
    (((width - fixed) / pitch) as usize).min(7)
}

fn entries(
    catalog: &ApplicationCatalog,
    pins: &[String],
    model: &Snapshot,
    slots: usize,
) -> Vec<Entry> {
    let mut items: Vec<_> = pins
        .iter()
        .filter_map(|id| catalog.index_of(id))
        .take(slots.min(4))
        .map(|app| Entry {
            app: Some(app),
            native_id: String::new(),
            label: catalog.applications()[app].name.clone(),
            windows: Vec::new(),
        })
        .collect();
    let limit = slots.min(items.len() + 3);
    for w in model
        .windows
        .iter()
        .filter(|w| w.workspace.is_none_or(|i| i == model.current_workspace))
    {
        let app = catalog.window_index(&w.app_id);
        let slot = items.iter().position(|item| match app {
            Some(i) => item.app == Some(i),
            None => !w.app_id.is_empty() && item.native_id == w.app_id,
        });
        let item = if let Some(i) = slot {
            &mut items[i]
        } else if items.len() < limit {
            let label = app
                .map(|i| catalog.applications()[i].name.as_str())
                .or_else(|| (!w.title.trim().is_empty()).then_some(w.title.as_str()))
                .unwrap_or("Window");
            items.push(Entry {
                app,
                native_id: w.app_id.clone(),
                label: label.to_owned(),
                windows: Vec::new(),
            });
            items.last_mut().unwrap()
        } else {
            continue;
        };
        item.windows.push(Window {
            id: w.id,
            active: w.active,
            can_minimize: w.can_minimize,
        });
    }
    items
}

pub(super) fn strip(shell: &Rc<Shell>, layout: Layout) -> Element {
    let pins = shell.favorites;
    let model = shell.snapshot;
    let display = shell.metrics();
    let catalog = shell.catalog.clone();
    let shell = shell.clone();
    let capacity = memo(move |_| slot_capacity(layout, display.width.get()));
    let items = memo(move |_| {
        pins.with(|pins| model.with(|model| entries(&catalog, pins, model, capacity.get())))
    });
    let buttons = for_each(
        move || items.get(),
        Entry::key,
        move |item| {
            let running = !item.windows.is_empty();
            let active = item.windows.iter().any(|w| w.active);
            let label = match item.windows.len() {
                0 => format!("Launch pinned {}", item.label),
                1 => item.label.clone(),
                n => format!("{} · {n} windows", item.label),
            };
            let icon = item
                .app
                .map(|i| super::super::launcher::app_icon(&shell, &shell.catalog.applications()[i]))
                .unwrap_or_else(|| {
                    super::super::launcher::fallback_icon(
                        if item.native_id.is_empty() {
                            &item.label
                        } else {
                            &item.native_id
                        },
                        &item.label,
                    )
                });
            let state = shell.clone();
            let width = match layout {
                Layout::Taskbar => 40,
                Layout::MenuBarDock => 44,
                Layout::BottomNavigation => 48,
            };
            let (marker_width, marker_height) = match layout {
                Layout::Taskbar if active => (16, 3),
                Layout::MenuBarDock => (4, 4),
                _ => (6, 3),
            };
            action(&label, move || item.invoke(&state, layout))
                .width(width)
                .min_width(width)
                .height(if layout == Layout::Taskbar { 40 } else { 48 })
                .padding(0)
                .corner_radius(8)
                .border_thickness(0)
                .background(if layout == Layout::Taskbar && active {
                    colors().tile
                } else {
                    "Transparent"
                })
                .child(
                    grid().row_definitions("*,5").child(icon).child(
                        border()
                            .grid_row(1)
                            .is_visible(running)
                            .width(marker_width)
                            .height(marker_height)
                            .horizontal_alignment(H::Center)
                            .vertical_alignment(V::Center)
                            .corner_radius(2)
                            .background(if layout == Layout::MenuBarDock {
                                colors().bar_ink
                            } else if active {
                                colors().accent
                            } else {
                                colors().muted
                            }),
                    ),
                )
        },
    );
    if layout == Layout::MenuBarDock {
        row(7.0)
            .child(
                border()
                    .width(1)
                    .height(31)
                    .background(colors().bar_edge)
                    .is_visible(move || items.with(|items| !items.is_empty())),
            )
            .child(buttons)
            .into()
    } else {
        buttons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_capacity_respects_chrome_and_retains_small_screen_pins() {
        assert_eq!(slot_capacity(Layout::Taskbar, 320.0), 0);
        assert_eq!(slot_capacity(Layout::MenuBarDock, 320.0), 2);
        assert_eq!(slot_capacity(Layout::BottomNavigation, 375.0), 3);
        for layout in [
            Layout::Taskbar,
            Layout::MenuBarDock,
            Layout::BottomNavigation,
        ] {
            assert_eq!(slot_capacity(layout, 0.0), 0);
            assert_eq!(slot_capacity(layout, 1440.0), 7);
        }
    }
}
