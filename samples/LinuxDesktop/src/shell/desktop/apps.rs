//! Pinned and running apps share one slot. Only the small set of visible
//! buttons is mounted; the overview still lists every individual window.
mod taskbar;

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
    pinned: bool,
    windows: Vec<Window>,
}

impl Entry {
    fn key(&self) -> ferroui_view::Key {
        if let Some(app) = self.app {
            app.into()
        } else if !self.native_id.is_empty() {
            self.native_id.as_str().into()
        } else {
            (-1 - i64::from(self.windows[0].id)).into()
        }
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
                activate_window(session, target.id);
            }
        }
    }
}

fn activate_window(session: &crate::session::Session, id: u32) {
    if session.activate(id) {
        if let Some(host) = session.compositor() {
            host.focus_client();
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
    if slots == 0 {
        return Vec::new();
    }
    let mut items: Vec<_> = pins
        .iter()
        .filter_map(|id| catalog.index_of(id))
        .take(slots.min(4))
        .map(|app| Entry {
            app: Some(app),
            native_id: String::new(),
            pinned: true,
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
                pinned: false,
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

fn app_art(shell: &Rc<Shell>, item: &Entry) -> Element {
    if let Some(i) = item.app {
        super::super::launcher::app_icon(shell, &shell.catalog.applications()[i])
    } else {
        let id = if item.native_id.is_empty() {
            &item.label
        } else {
            &item.native_id
        };
        super::super::launcher::fallback_icon(id, &item.label)
    }
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
    if layout == Layout::Taskbar {
        return taskbar::buttons(&shell, items);
    }
    let buttons = for_each(
        move || {
            items.with(|items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let separator = layout == Layout::MenuBarDock
                            && !entry.pinned
                            && (i == 0 || items[i - 1].pinned);
                        (separator, entry.clone())
                    })
                    .collect::<Vec<_>>()
            })
        },
        |(_, entry)| entry.key(),
        move |(separator, item)| {
            let running = !item.windows.is_empty();
            let active = item.windows.iter().any(|w| w.active);
            let label = match item.windows.len() {
                0 => format!("Launch pinned {}", item.label),
                1 => item.label.clone(),
                n => format!("{} · {n} windows", item.label),
            };
            let icon = app_art(&shell, &item);
            let state = shell.clone();
            let width = match layout {
                Layout::Taskbar => unreachable!(),
                Layout::MenuBarDock => 44,
                Layout::BottomNavigation => 48,
            };
            let (marker_width, marker_height) = match layout {
                Layout::MenuBarDock => (4, 4),
                _ => (6, 3),
            };
            let dock_menu = (layout == Layout::MenuBarDock).then(|| dock_menu(&shell, &item));
            let button = action(&label, move || item.invoke(&state, layout))
                .width(width)
                .min_width(width)
                .height(48)
                .padding(0)
                .corner_radius(8)
                .border_thickness(0)
                .background("Transparent")
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
                );
            if let Some(menu) = dock_menu {
                let button = button
                    .tool_tip_placement(ferroui_controls::PlacementMode::Top)
                    .with(move |b| b.set_context_flyout(&menu));
                if separator {
                    row(7.0)
                        .child(border().width(1).height(31).background(colors().bar_edge))
                        .child(button)
                        .into_element()
                } else {
                    button.into_element()
                }
            } else {
                button.into_element()
            }
        },
    );
    buttons
}

fn dock_menu(shell: &Rc<Shell>, entry: &Entry) -> ferroui_base::Ref<ferroui_controls::MenuFlyout> {
    use ferroui_controls::{
        Control, MenuFlyout, MenuItem, MenuItemToggleType, PlacementMode, Separator,
    };

    fn item(label: &str, invoke: impl Fn() + 'static) -> ferroui_base::Ref<MenuItem> {
        let item = MenuItem::new();
        item.set_header(Some(Rc::new(label.to_owned())));
        item.add_handler(MenuItem::click_event(), move |_, _| invoke());
        item
    }

    let menu = MenuFlyout::new();
    super::popup_host::configure_menu(&menu, 10.0);
    menu.set_placement(PlacementMode::Top);
    let app = entry.app;
    let native_id = entry.native_id.clone();
    let window_id = entry.windows.first().map(|w| w.id);
    let shell = Rc::downgrade(shell);
    let weak = menu.downgrade();
    let opening = menu.opening(move |_| {
        let (Some(menu), Some(shell)) = (weak.upgrade(), shell.upgrade()) else {
            return;
        };
        let items = menu.items();
        items.clear();
        let windows = shell.snapshot.with_untracked(|s| {
            s.windows
                .iter()
                .filter(|w| w.workspace.is_none_or(|i| i == s.current_workspace))
                .filter(|w| match app {
                    Some(id) => shell.catalog.window_index(&w.app_id) == Some(id),
                    None if !native_id.is_empty() => w.app_id == native_id,
                    None => Some(w.id) == window_id,
                })
                .cloned()
                .collect::<Vec<_>>()
        });
        for w in &windows {
            let id = w.id;
            let session = shell.session.clone();
            let title = if w.title.is_empty() {
                "Untitled Window"
            } else {
                w.title.as_str()
            };
            let action = item(title, move || {
                if let Some(s) = &session {
                    activate_window(s, id);
                }
            });
            action.set_toggle_type(MenuItemToggleType::Radio);
            action.set_is_checked(w.active);
            items.add(Some(Control::boxed(action)));
        }
        if let Some(index) = app {
            if !windows.is_empty() {
                items.add(Some(Control::boxed(Separator::new())));
            } else {
                let state = shell.clone();
                items.add(Some(Control::boxed(item("Open", move || {
                    state.launch(&state.catalog.applications()[index])
                }))));
            }
            let state = shell.clone();
            let pin = item("Keep in Dock", move || {
                super::super::launcher::toggle_pin(&state, index)
            });
            pin.set_toggle_type(MenuItemToggleType::CheckBox);
            pin.set_is_checked(
                shell
                    .favorites
                    .with_untracked(|ids| ids.contains(&shell.catalog.applications()[index].id)),
            );
            pin.set_is_enabled(!shell.pin_editing.get_untracked());
            items.add(Some(Control::boxed(pin)));
        }
    });
    on_cleanup(move || drop(opening));
    menu
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
