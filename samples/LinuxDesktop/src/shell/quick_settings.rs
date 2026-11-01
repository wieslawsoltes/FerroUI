//! Quick settings are loaded only while the sheet is visible. Mutations are
//! serialized and followed by a read of the actual device state.
use super::chrome::*;
use super::{feedback, popup_header, Shell};
use crate::services::quick_settings::{self, Change, Snapshot};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
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
    pending_level: std::cell::Cell<Option<u16>>,
}

impl Settings {
    pub(super) fn new() -> Self {
        Self {
            values: signal(None),
            busy: signal(false),
            error: signal(String::new()),
            pending_level: Default::default(),
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
            if let Some(value) = settings.pending_level.take() {
                apply_volume(&state, value);
            }
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

fn apply_volume(shell: &Rc<Shell>, percent: u16) {
    if shell.settings.busy.get_untracked() {
        shell.settings.pending_level.set(Some(percent));
        return;
    }
    let audio = shell
        .settings
        .values
        .with_untracked(|s| s.as_ref().and_then(|s| s.audio));
    if let Some(audio) = audio.filter(|a| a.percent != percent) {
        request(shell, Change::SetLevel(audio.mixer, percent));
    }
}

/// Commit an interactive edit on release so a pointer drag never floods the audio server.
/// The slider stays mounted and keeps keyboard focus when the device state is read back.
fn volume_slider(shell: &Rc<Shell>) -> Element {
    let values = shell.settings.values;
    let state = shell.clone();
    slider()
        .name("OutputVolume")
        .automation_properties_name("Output volume")
        .minimum(0.0)
        .maximum(100.0)
        .small_change(1.0)
        .large_change(10.0)
        .value(move || {
            values.with(|s| {
                s.as_ref()
                    .and_then(|s| s.audio)
                    .map_or(0.0, |a| f64::from(a.percent.min(100)))
            })
        })
        .is_visible(move || values.with(|s| s.as_ref().is_some_and(|s| s.audio.is_some())))
        .with(move |slider| {
            let commit = {
                let weak = slider.downgrade();
                let state = Rc::downgrade(&state);
                move || {
                    if let (Some(slider), Some(state)) = (weak.upgrade(), state.upgrade()) {
                        apply_volume(&state, slider.value().round() as u16);
                    }
                }
            };
            let pointer_up = ferroui_base::input::InputElement::pointer_released_event();
            let key_up = ferroui_base::input::InputElement::key_up_event();
            let commit_pointer = commit.clone();
            let mouse = slider.add_handler_with(
                pointer_up,
                move |_, _| commit_pointer(),
                RoutingStrategies::BUBBLE,
                true,
            );
            let keyboard = slider.add_handler_with(
                key_up,
                move |_, _| commit(),
                RoutingStrategies::BUBBLE,
                true,
            );
            let weak = slider.downgrade();
            on_cleanup(move || {
                if let Some(slider) = weak.upgrade() {
                    slider.remove_handler(pointer_up, mouse);
                    slider.remove_handler(key_up, keyboard);
                }
            });
        })
        .into()
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

fn audio_row(state: &Rc<Shell>, values: &Snapshot) -> Element {
    let mut rows = stack_panel();
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
    rows.into()
}

fn device_rows(state: &Rc<Shell>, values: &Snapshot) -> Element {
    let mut rows = stack_panel().spacing(7.0);
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
    let audio_state = shell.clone();
    let busy = shell.settings.busy;
    let values = shell.settings.values;
    super::desktop::system_frame(shell, 440.0)
        .padding(18)
        .child(
            stack_panel()
                .spacing(12.0)
                .child(popup_header("Quick settings", "SYSTEM CONTROLS", shell))
                .child(dynamic(move || {
                    values.with(|s| {
                        s.as_ref()
                            .map_or(Element::Empty, |snapshot| audio_row(&audio_state, snapshot))
                    })
                }))
                .child(volume_slider(shell))
                .child(scroll_viewer().max_height(300).child(dynamic(move || {
                    values.with(|snapshot| match snapshot {
                        Some(snapshot) => device_rows(&state, snapshot),
                        None => {
                            let label = shell_text("Reading device settings…", 13.0, Ink::Secondary);
                            if look().winui().is_some() {
                                row(10.0).child(progress_ring().is_active(busy).width(20.0).height(20.0))
                                    .child(label).into()
                            } else {
                                label.into()
                            }
                        },
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
    super::desktop::system_frame(shell, 390.0)
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
