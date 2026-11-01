//! Quick settings are loaded only while the sheet is visible. Mutations are
//! serialized and followed by a read of the actual device state.
use super::chrome::*;
use super::{feedback, popup_header, Shell};
use crate::services::quick_settings::{self, Change, Snapshot};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
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
}

impl Settings {
    pub(super) fn new() -> Self {
        Self {
            values: signal(None),
            busy: signal(false),
            error: signal(String::new()),
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

fn section(icon: Icon, title: &str, detail: &str, controls: Element) -> El<Grid> {
    grid()
        .column_definitions("28,*,Auto")
        .margin((0.0, 6.0))
        .child(
            border()
                .vertical_alignment(V::Center)
                .child(glyph(icon, 17.0, colors().accent)),
        )
        .child(
            stack_panel()
                .grid_column(1)
                .spacing(1)
                .margin((4.0, 0.0, 10.0, 0.0))
                .child(shell_text(title, 14.0, Ink::Primary))
                .child(shell_text(detail, 12.0, Ink::Secondary)),
        )
        .child(
            border()
                .grid_column(2)
                .vertical_alignment(V::Center)
                .child(controls),
        )
}

fn control(
    shell: &Rc<Shell>,
    title: &str,
    content: &str,
    apply: impl Fn() -> Change + 'static,
) -> El<Button> {
    let state = shell.clone();
    let busy = shell.settings.busy;
    action(title, move || request(&state, apply()))
        .padding((9.0, 5.0))
        .min_width(34)
        .is_enabled(move || !busy.get())
        .child(shell_text(content, 13.0, Ink::Primary))
}

fn device_rows(state: &Rc<Shell>, values: &Snapshot) -> Element {
    let mut rows = stack_panel().spacing(7.0);
    if let Some(audio) = values.audio {
        let detail = if audio.muted {
            format!("Muted · {}%", audio.percent)
        } else {
            format!("{}%", audio.percent)
        };
        rows = rows.child(section(
            Icon::Sound,
            "Sound output",
            &detail,
            row(4.0)
                .child(control(state, "Decrease volume", "−", move || {
                    Change::Volume(audio.mixer, false)
                }))
                .child(control(
                    state,
                    if audio.muted {
                        "Unmute output"
                    } else {
                        "Mute output"
                    },
                    if audio.muted { "Unmute" } else { "Mute" },
                    move || Change::Mute(audio.mixer, !audio.muted),
                ))
                .child(control(state, "Increase volume", "+", move || {
                    Change::Volume(audio.mixer, true)
                }))
                .into(),
        ));
    } else {
        rows = rows.child(section(
            Icon::Sound,
            "Sound output",
            "No default output detected",
            Element::Empty,
        ));
    }
    if let Some(wifi) = &values.wifi {
        let enabled = wifi.enabled;
        let detail = if !wifi.hardware_enabled {
            "Radio disabled by hardware".to_owned()
        } else if wifi.connection.is_empty() {
            "No active network".to_owned()
        } else {
            format!("Active connection · {}", wifi.connection)
        };
        let toggle = if wifi.hardware_enabled {
            control(
                state,
                "Toggle Wi-Fi",
                if enabled { "Turn off" } else { "Turn on" },
                move || Change::Wifi(!enabled),
            )
            .into()
        } else {
            Element::Empty
        };
        rows = rows.child(section(Icon::Network, "Wi-Fi", &detail, toggle));
    } else {
        rows = rows.child(section(
            Icon::Network,
            "Network",
            "NetworkManager unavailable",
            Element::Empty,
        ));
    }
    for light in &values.backlights {
        let down = light.device.clone();
        let up = light.device.clone();
        let actions = if values.login_available {
            row(4.0)
                .child(control(state, "Lower brightness", "−", move || {
                    Change::Brightness(down.clone(), false)
                }))
                .child(control(state, "Raise brightness", "+", move || {
                    Change::Brightness(up.clone(), true)
                }))
                .into()
        } else {
            Element::Empty
        };
        rows = rows.child(section(
            Icon::Brightness,
            "Display brightness",
            &format!("{} · {}%", light.device, light.percent),
            actions,
        ));
    }
    if let Some(status) = &values.battery {
        rows = rows.child(section(Icon::Battery, "Battery", status, Element::Empty));
    }
    rows.into()
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
    ferroui_view::on_cleanup(move || timer.dispose());
    let state = shell.clone();
    let refresh = shell.clone();
    let busy = shell.settings.busy;
    let values = shell.settings.values;
    super::desktop::popup_frame()
        .max_width(440)
        .padding(18)
        .child(
            stack_panel()
                .spacing(12.0)
                .child(popup_header("Quick settings", "SYSTEM CONTROLS", shell))
                .child(scroll_viewer().max_height(300).child(dynamic(move || {
                    values.with(|snapshot| match snapshot {
                        Some(snapshot) => device_rows(&state, snapshot),
                        None => shell_text("Reading device settings…", 13.0, Ink::Secondary).into(),
                    })
                })))
                .child(feedback(shell.settings.error))
                .child(
                    action("Refresh device settings", move || {
                        request(&refresh, Change::Refresh)
                    })
                    .horizontal_alignment(H::Right)
                    .is_enabled(move || !busy.get())
                    .child(shell_text("Refresh", 12.0, Ink::Primary)),
                ),
        )
        .into()
}

pub(super) fn calendar_view(shell: &Rc<Shell>) -> Element {
    super::desktop::popup_frame()
        .max_width(390)
        .padding(16)
        .child(
            stack_panel()
                .spacing(9.0)
                .child(popup_header("Calendar", "DATE AND TIME", shell))
                .child(
                    calendar()
                        .name("DesktopCalendar")
                        .horizontal_alignment(H::Center),
                ),
        )
        .into()
}
