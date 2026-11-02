//! Quick settings are loaded only while the sheet is visible. Mutations are
//! serialized and followed by a read of the actual device state.
mod tiles;

use super::appearance::Layout;
use super::chrome::*;
use super::{feedback, Scene, Shell};
use crate::services::quick_settings::{self, Change, Snapshot};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{Button, Grid};
use ferroui_system::threading::tasks;
use ferroui_view::prelude::*;
use std::rc::Rc;
use std::time::Duration;

pub(super) struct Settings {
    values: Signal<Option<Snapshot>>,
    busy: Signal<bool>,
    error: Signal<String>,
    pending_levels: std::cell::RefCell<Vec<(Level, u8)>>,
}

impl Settings {
    pub(super) fn new() -> Self {
        Self {
            values: signal(None),
            busy: signal(false),
            error: signal(String::new()),
            pending_levels: Default::default(),
        }
    }
}

fn request(shell: &Rc<Shell>, change: Change) {
    let settings = &shell.settings;
    if settings.busy.get_untracked() {
        return;
    }
    settings.busy.set(true);
    let explicit = !matches!(change, Change::Refresh);
    let weak = Rc::downgrade(shell);
    let _task = tasks::spawn_local(async move {
        let result = tasks::run(move || Ok(quick_settings::update(change))).await;
        if let Some(state) = weak.upgrade().filter(|state| state.owner.is_alive()) {
            let settings = &state.settings;
            batch(|| {
                match result {
                    Ok((snapshot, error)) => {
                        settings.values.set_if_changed(Some(snapshot));
                        if explicit || error.is_some() {
                            settings.error.set_if_changed(error.unwrap_or_default());
                        }
                    }
                    Err(error) => {
                        settings.error.set(error.to_string());
                    }
                }
                settings.busy.set(false);
            });
            drain_levels(&state);
        }
        Ok(())
    });
}

pub(super) fn handle_key(shell: &Rc<Shell>, key: Key, modifiers: KeyModifiers) -> bool {
    let action = shell.settings.values.with_untracked(|values| {
        let devices = values.as_ref()?;
        match (key, modifiers) {
            (Key::M, m) if m.is_empty() => devices.audio.map(|a| Change::Mute(a.mixer, !a.muted)),
            (Key::OemPlus | Key::Add, m) if m.is_empty() || m == KeyModifiers::SHIFT => {
                devices.audio.map(|a| Change::Volume(a.mixer, true))
            }
            (Key::OemMinus | Key::Subtract, m) if m.is_empty() => {
                devices.audio.map(|a| Change::Volume(a.mixer, false))
            }
            (Key::B, m) if m.is_empty() => devices
                .bluetooth
                .as_ref()
                .map(|b| Change::Bluetooth(b.path.clone(), !b.powered)),
            (Key::W, m) if m.is_empty() => devices
                .wifi
                .as_ref()
                .filter(|w| w.hardware_enabled)
                .map(|w| Change::Wifi(!w.enabled)),
            _ => None,
        }
    });
    if let Some(action) = action {
        request(shell, action);
        true
    } else {
        false
    }
}

#[derive(Clone, Eq, PartialEq)]
enum Level {
    Volume,
    Backlight(String),
}

impl Level {
    fn percent(&self, snapshot: &Snapshot) -> Option<u8> {
        match self {
            Self::Volume => snapshot.audio.map(|a| a.percent.min(100) as u8),
            Self::Backlight(device) => snapshot
                .backlights
                .iter()
                .find(|light| &light.device == device)
                .map(|light| light.percent),
        }
    }

    fn change(&self, values: &Snapshot, percent: u8) -> Option<Change> {
        if self.percent(values)? == percent {
            return None;
        }
        match self {
            Self::Volume => values
                .audio
                .map(|a| Change::SetLevel(a.mixer, u16::from(percent))),
            Self::Backlight(device) if values.login_available => {
                Some(Change::SetBrightness(device.clone(), percent))
            }
            _ => None,
        }
    }
}

// A refresh, audio write or another display's write can be outstanding when
// the thumb is released. Coalesce the latest edit per physical destination.
fn submit_level(shell: &Rc<Shell>, target: &Level, percent: u8) {
    {
        let mut pending = shell.settings.pending_levels.borrow_mut();
        pending.retain(|(other, _)| other != target);
        pending.push((target.clone(), percent));
    }
    drain_levels(shell);
}

fn drain_levels(shell: &Rc<Shell>) {
    while !shell.settings.busy.get_untracked() {
        let next = {
            let mut pending = shell.settings.pending_levels.borrow_mut();
            if pending.is_empty() {
                return;
            }
            pending.remove(0)
        };
        let change = shell
            .settings
            .values
            .with_untracked(|s| next.0.change(s.as_ref()?, next.1));
        if let Some(change) = change {
            request(shell, change);
        }
    }
}

// Keep the Slider object alive through readback. Release commits pointer,
// touch, and keyboard edits without sending every intermediate thumb value.
fn level_slider(shell: &Rc<Shell>, target: Level, label: &str) -> Element {
    let values = shell.settings.values;
    let read = target.clone();
    let enabled = target.clone();
    let state = Rc::downgrade(shell);
    slider()
        .name(if target == Level::Volume {
            "OutputVolume"
        } else {
            "DisplayBrightness"
        })
        .automation_properties_name(label)
        .minimum(if target == Level::Volume { 0.0 } else { 1.0 })
        .maximum(100.0)
        .small_change(1.0)
        .large_change(10.0)
        .value(move || {
            values.with(|s| {
                s.as_ref()
                    .and_then(|s| read.percent(s))
                    .map_or(0., f64::from)
            })
        })
        .is_enabled(move || {
            values.with(|s| {
                s.as_ref().is_some_and(|s| {
                    enabled.percent(s).is_some() && (enabled == Level::Volume || s.login_available)
                })
            })
        })
        .with(move |slider| {
            let weak = slider.downgrade();
            let state = state.clone();
            let target = target.clone();
            let commit = move || {
                if let (Some(slider), Some(state)) = (weak.upgrade(), state.upgrade()) {
                    submit_level(&state, &target, slider.value().round() as u8);
                }
            };
            let pointer_up = ferroui_base::input::InputElement::pointer_released_event();
            let key_up = ferroui_base::input::InputElement::key_up_event();
            let pointer_commit = commit.clone();
            let mouse = slider.add_handler_with(
                pointer_up,
                move |_, _| pointer_commit(),
                RoutingStrategies::BUBBLE,
                true,
            );
            let keyboard = slider.add_handler_with(
                key_up,
                move |_, _| commit(),
                RoutingStrategies::BUBBLE,
                true,
            );
            let slider = slider.downgrade();
            on_cleanup(move || {
                if let Some(slider) = slider.upgrade() {
                    slider.remove_handler(pointer_up, mouse);
                    slider.remove_handler(key_up, keyboard);
                }
            });
        })
        .into()
}

fn level_group(shell: &Rc<Shell>, target: Level, layout: Layout) -> Element {
    let volume = target == Level::Volume;
    let values = shell.settings.values;
    let target_read = target.clone();
    let detail = move || {
        values.with(|s| {
            let Some(s) = s else {
                return "Reading…".to_owned();
            };
            if volume && s.audio.is_some_and(|a| a.muted) {
                return "Muted".to_owned();
            }
            target_read
                .percent(s)
                .map_or_else(|| "Unavailable".into(), |v| format!("{v}%"))
        })
    };
    let title = match &target {
        Level::Volume => "Sound".to_owned(),
        Level::Backlight(device) => {
            let multiple =
                values.with_untracked(|s| s.as_ref().is_some_and(|s| s.backlights.len() > 1));
            if multiple {
                format!("Brightness · {device}")
            } else {
                "Brightness".to_owned()
            }
        }
    };
    let row_icon: Element = if volume {
        let state = shell.clone();
        let enabled = shell.settings.busy;
        action("Mute or unmute output", move || {
            handle_key(&state, Key::M, KeyModifiers::NONE);
        })
        .automation_properties_item_status(move || {
            values.with(|s| {
                if s.as_ref().and_then(|s| s.audio).is_some_and(|a| a.muted) {
                    "Muted"
                } else {
                    "Unmuted"
                }
            })
        })
        .is_enabled(move || {
            !enabled.get() && values.with(|s| s.as_ref().is_some_and(|s| s.audio.is_some()))
        })
        .min_width(0.)
        .min_height(0.)
        .width(34.)
        .height(34.)
        .padding(6.)
        .border_thickness(0)
        .background("Transparent")
        .child(dynamic(move || {
            let muted = values.with(|s| s.as_ref().and_then(|s| s.audio).is_some_and(|a| a.muted));
            glyph(
                if muted { Icon::SoundOff } else { Icon::Sound },
                18.,
                colors().text,
            )
        }))
        .into()
    } else {
        glyph(Icon::Brightness, 18., colors().text)
    };
    let heading = grid()
        .column_definitions("*,Auto")
        .child(shell_text(&title, 13., Ink::Primary).font_weight(FontWeight::SemiBold))
        .child(
            shell_text("", 12., Ink::Secondary)
                .grid_column(1)
                .text(detail),
        );
    let line = grid().column_definitions("36,*").child(row_icon).child(
        border()
            .grid_column(1)
            .vertical_alignment(V::Center)
            .child(level_slider(shell, target, &title)),
    );
    let body = stack_panel().spacing(2.).child(heading).child(line);
    match layout {
        Layout::MenuBarDock => border()
            .corner_radius(14)
            .padding((12., 8.))
            .background(colors().tile)
            .child(body)
            .into(),
        Layout::BottomNavigation => border().padding((6., 2.)).child(body).into(),
        Layout::Taskbar => body.into(),
    }
}

fn backlights(shell: &Rc<Shell>, layout: Layout) -> Element {
    let values = shell.settings.values;
    let state = shell.clone();
    stack_panel()
        .spacing(8.)
        .child(for_each(
            move || {
                values.with(|s| {
                    s.as_ref().map_or_else(Vec::new, |v| {
                        v.backlights
                            .iter()
                            .map(|b| b.device.clone())
                            .collect::<Vec<_>>()
                    })
                })
            },
            |id| id.clone(),
            move |id| level_group(&state, Level::Backlight(id), layout),
        ))
        .into()
}

fn compact_action(label: &str, icon: Icon, activate: impl Fn() + 'static) -> El<Button> {
    let button = icon_action(label, icon, activate)
        .min_width(0.)
        .min_height(0.);
    match look().layout() {
        Layout::MenuBarDock => button.theme(resource("MacOSBorderlessButtonStyle")),
        Layout::BottomNavigation => button.theme(resource("MaterialIconButton")),
        Layout::Taskbar => button,
    }
}

// Opening establishes the keyboard focus scope even when there is no audio,
// radio or battery. All controls retain normal tab order and automation names.
fn header(shell: &Rc<Shell>, text: Element) -> El<Grid> {
    let dismiss = shell.clone();
    let focus = NodeRef::<Button>::new();
    grid().column_definitions("*,Auto").child(text).child(
        compact_action("Close panel", Icon::Close, move || dismiss.dismiss())
            .grid_column(1)
            .node_ref(focus)
            .on_loaded(move |_| {
                if let Some(button) = focus.get_untracked() {
                    button.focus();
                }
            }),
    )
}

fn battery(shell: &Rc<Shell>) -> Element {
    let values = shell.settings.values;
    row(6.)
        .is_visible(move || values.with(|s| s.as_ref().is_some_and(|s| s.battery.is_some())))
        .child(glyph(Icon::Battery, 16., colors().text))
        .child(shell_text("", 12., Ink::Secondary).text(move || {
            values.with(|s| {
                s.as_ref()
                    .and_then(|v| v.battery.clone())
                    .unwrap_or_default()
            })
        }))
        .into()
}

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    request(shell, Change::Refresh);
    let weak = Rc::downgrade(shell);
    let timer = DispatcherTimer::run(
        move || {
            if let Some(shell) = weak.upgrade().filter(|s| s.owner.is_alive()) {
                request(&shell, Change::Refresh);
                return true;
            }
            false
        },
        Duration::from_secs(10),
        DispatcherPriority::BACKGROUND,
    );
    on_cleanup(move || timer.dispose());
    let layout = look().layout();
    let (title, width) = match layout {
        Layout::Taskbar => ("Quick settings", 370.),
        Layout::MenuBarDock => ("Control Center", 328.),
        Layout::BottomNavigation => ("Quick Settings", 436.),
    };
    let mut frame = super::desktop::system_frame(shell, width);
    if layout == Layout::BottomNavigation {
        frame = frame
            .vertical_alignment(V::Top)
            .horizontal_alignment(H::Right)
            .margin((8., look().insets().0 + 4., 8., 8.));
    }
    let mut body = stack_panel().spacing(if layout == Layout::Taskbar { 12. } else { 8. });
    let lights = backlights(shell, layout);
    if layout == Layout::BottomNavigation {
        body = body.child(lights).child(tiles::view(shell, layout));
    } else {
        body = body.child(tiles::view(shell, layout)).child(lights);
    }
    let refresh = shell.clone();
    let busy = shell.settings.busy;
    let body = body
        .child(level_group(shell, Level::Volume, layout))
        .child(feedback(shell.settings.error));
    let settings = shell.clone();
    let mut tools = row(4.);
    if layout == Layout::Taskbar {
        tools = tools.child(tiles::mode_button(shell));
    }
    let tools = tools
        .child(
            compact_action("Refresh device settings", Icon::Refresh, move || {
                request(&refresh, Change::Refresh)
            })
            .is_enabled(move || !busy.get()),
        )
        .child(compact_action(
            "Desktop appearance",
            Icon::Settings,
            move || settings.show(Scene::Appearance),
        ));
    let heading: Element = if layout == Layout::BottomNavigation {
        let clock = super::clock();
        stack_panel()
            .child(shell_text("", 26., Ink::Primary).text(move || super::time_text(clock.get())))
            .child(shell_text("", 12., Ink::Secondary).text(move || {
                let date = clock.get();
                format!("{}, {} {}", date.weekday(), date.month(), date.day())
            }))
            .into()
    } else {
        shell_text(title, 20., Ink::Primary)
            .font_weight(FontWeight::SemiBold)
            .into()
    };
    frame
        .automation_properties_name(title)
        .padding(14)
        .child(
            grid()
                .row_definitions("Auto,*,Auto")
                .child(header(shell, heading))
                .child(
                    scroll_viewer()
                        .grid_row(1)
                        .max_height(420.)
                        .margin((0., 10., 0., 10.))
                        .child(body),
                )
                .child(
                    grid()
                        .grid_row(2)
                        .column_definitions("*,Auto")
                        .child(battery(shell))
                        .child(tools.grid_column(1)),
                ),
        )
        .into()
}

pub(super) fn calendar_view(shell: &Rc<Shell>) -> Element {
    let date = super::now();
    let title = format!("{}, {} {}", date.weekday(), date.month(), date.day());
    super::desktop::system_frame(shell, 350.)
        .padding(14)
        .child(
            stack_panel()
                .spacing(9.)
                .child(header(
                    shell,
                    stack_panel()
                        .spacing(2.)
                        .child(
                            shell_text("Calendar", 20., Ink::Primary)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(shell_text(&title, 12., Ink::Secondary))
                        .into(),
                ))
                .child(
                    calendar()
                        .name("DesktopCalendar")
                        .horizontal_alignment(H::Center),
                ),
        )
        .into()
}
