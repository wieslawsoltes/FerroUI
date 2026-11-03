//! Real radio and notification commands shared by the three native tile layouts.
use super::*;
use crate::shell::appearance::ColorMode;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tile {
    Wifi,
    Bluetooth,
    Quiet,
    Dark,
}

impl Tile {
    fn label(self, layout: Layout) -> &'static str {
        match (self, layout) {
            (Self::Wifi, Layout::BottomNavigation) => "Internet",
            (Self::Wifi, _) => "Wi-Fi",
            (Self::Bluetooth, _) => "Bluetooth",
            (Self::Quiet, Layout::MenuBarDock) => "Focus",
            (Self::Quiet, _) => "Do not disturb",
            (Self::Dark, Layout::BottomNavigation) => "Dark theme",
            (Self::Dark, _) => "Dark mode",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::Wifi => Icon::Network,
            Self::Bluetooth => Icon::Bluetooth,
            Self::Quiet => Icon::Moon,
            Self::Dark => Icon::Appearance,
        }
    }
    fn active(self, shell: &Shell) -> bool {
        match self {
            Self::Wifi => shell.settings.values.with(|s| {
                s.as_ref()
                    .and_then(|s| s.wifi.as_ref())
                    .is_some_and(|w| w.enabled && w.hardware_enabled)
            }),
            Self::Bluetooth => shell.settings.values.with(|s| {
                s.as_ref()
                    .and_then(|s| s.bluetooth.as_ref())
                    .is_some_and(|b| b.powered)
            }),
            Self::Quiet => shell.notifications.quiet(),
            Self::Dark => shell.appearance.get().color_mode() == Some(ColorMode::Dark),
        }
    }
    fn detail(self, shell: &Shell) -> String {
        if self == Self::Wifi {
            return shell.settings.values.with(|s| {
                let Some(wifi) = s.as_ref().and_then(|s| s.wifi.as_ref()) else {
                    return "Unavailable".to_owned();
                };
                if !wifi.hardware_enabled {
                    "Hardware switch off".to_owned()
                } else if !wifi.enabled {
                    "Off".to_owned()
                } else if wifi.connection.is_empty() {
                    "Not connected".to_owned()
                } else {
                    wifi.connection.clone()
                }
            });
        }
        if self == Self::Bluetooth
            && shell
                .settings
                .values
                .with(|s| s.as_ref().is_none_or(|s| s.bluetooth.is_none()))
        {
            return "Unavailable".into();
        }
        if self.active(shell) { "On" } else { "Off" }.to_owned()
    }
    fn enabled(self, shell: &Shell) -> bool {
        match self {
            Self::Wifi => {
                !shell.settings.busy.get()
                    && shell.settings.values.with(|s| {
                        s.as_ref()
                            .and_then(|s| s.wifi.as_ref())
                            .is_some_and(|w| w.hardware_enabled)
                    })
            }
            Self::Bluetooth => {
                !shell.settings.busy.get()
                    && shell
                        .settings
                        .values
                        .with(|s| s.as_ref().is_some_and(|s| s.bluetooth.is_some()))
            }
            Self::Quiet => true,
            Self::Dark => matches!(
                shell.appearance.get().color_mode(),
                Some(ColorMode::Dark | ColorMode::Light)
            ),
        }
    }
    fn activate(self, shell: &Rc<Shell>) {
        match self {
            Self::Wifi => {
                handle_key(shell, Key::W, KeyModifiers::NONE);
            }
            Self::Bluetooth => {
                handle_key(shell, Key::B, KeyModifiers::NONE);
            }
            Self::Quiet => shell.notifications.toggle_quiet(),
            Self::Dark => match shell.appearance.get_untracked().color_mode() {
                Some(ColorMode::Light) => shell.select_mode(ColorMode::Dark),
                Some(ColorMode::Dark) => shell.select_mode(ColorMode::Light),
                _ => {}
            },
        }
    }
}

fn tile(shell: &Rc<Shell>, kind: Tile, layout: Layout) -> Element {
    let p = colors();
    let title = kind.label(layout);
    let state = shell.clone();
    let selected = memo(move |_| kind.active(&state));
    let state = shell.clone();
    let detail = memo(move |_| kind.detail(&state));
    let state = shell.clone();
    let action = action(title, move || kind.activate(&state));
    let state = shell.clone();
    let action = action
        .automation_properties_item_status(move || detail.get())
        .is_enabled(move || kind.enabled(&state))
        .horizontal_alignment(H::Stretch)
        .horizontal_content_alignment(H::Stretch);
    let (active_fill, active_ink) = match layout {
        Layout::Taskbar if look().color_mode() == Some(ColorMode::Light) => (p.accent, "#FFFFFF"),
        Layout::Taskbar if look().color_mode() == Some(ColorMode::Dark) => (p.accent, "#00293B"),
        Layout::MenuBarDock => ("#007AFF", "#FFFFFF"),
        _ => (p.selected, p.on_selected),
    };
    let icon = move || {
        glyph(
            kind.icon(),
            19.,
            if selected.get() { active_ink } else { p.text },
        )
    };
    match layout {
        Layout::Taskbar => stack_panel()
            .spacing(4.)
            .margin((3., 0.))
            .child(
                action
                    .height(42.)
                    .corner_radius(5)
                    .padding(0)
                    .background(move || if selected.get() { active_fill } else { p.tile })
                    .child(dynamic(icon)),
            )
            .child(shell_text(title, 12., Ink::Primary).horizontal_alignment(H::Center))
            .into(),
        Layout::MenuBarDock => {
            let radio = matches!(kind, Tile::Wifi | Tile::Bluetooth);
            let badge = border()
                .width(32.)
                .height(32.)
                .corner_radius(16)
                .background(move || {
                    if selected.get() {
                        active_fill
                    } else {
                        p.surface
                    }
                })
                .child(dynamic(icon));
            let labels = stack_panel()
                .spacing(2.)
                .child(shell_text(title, 13., Ink::Primary).font_weight(FontWeight::SemiBold))
                .child(shell_text("", 11., Ink::Secondary).text(move || detail.get()));
            let contents = grid()
                .column_definitions("32,*")
                .child(badge)
                .child(labels.grid_column(1).margin((8., 0., 0., 0.)));
            let action = if radio {
                action.theme(resource("MacOSBorderlessButtonStyle"))
            } else {
                action
            };
            action
                .corner_radius(14)
                .padding(10)
                .height(if radio { 59. } else { 67. })
                .background(if radio { "Transparent" } else { p.tile })
                .child(contents)
                .into()
        }
        Layout::BottomNavigation => {
            let ink = move || if selected.get() { active_ink } else { p.text };
            action
                .corner_radius(19)
                .height(62.)
                .padding((12., 5.))
                .background(move || if selected.get() { active_fill } else { p.tile })
                .child(
                    grid()
                        .column_definitions("24,*")
                        .child(dynamic(icon))
                        .child(
                            stack_panel()
                                .grid_column(1)
                                .spacing(1.)
                                .margin((8., 0., 0., 0.))
                                .child(
                                    shell_text(title, 13., p.text)
                                        .foreground(ink)
                                        .font_weight(FontWeight::SemiBold),
                                )
                                .child(
                                    shell_text("", 11., p.muted)
                                        .text(move || detail.get())
                                        .foreground(ink),
                                ),
                        ),
                )
                .into()
        }
    }
}

pub(super) fn mode_button(shell: &Rc<Shell>) -> El<Button> {
    let toggle = shell.clone();
    let state = shell.clone();
    let enabled = shell.clone();
    compact_action("Toggle dark mode", Icon::Appearance, move || {
        Tile::Dark.activate(&toggle)
    })
    .automation_properties_item_status(move || Tile::Dark.detail(&state))
    .is_enabled(move || Tile::Dark.enabled(&enabled))
}

pub(super) fn view(shell: &Rc<Shell>, layout: Layout) -> Element {
    let wifi = tile(shell, Tile::Wifi, layout);
    let bluetooth = tile(shell, Tile::Bluetooth, layout);
    let quiet = tile(shell, Tile::Quiet, layout);
    match layout {
        Layout::Taskbar => uniform_grid()
            .columns(3)
            .child(wifi)
            .child(bluetooth)
            .child(quiet)
            .into(),
        Layout::MenuBarDock => grid()
            .column_definitions("*,8,*")
            .child(
                border()
                    .corner_radius(14)
                    .padding(1)
                    .background(colors().tile)
                    .child(
                        stack_panel()
                            .spacing(4.)
                            .vertical_alignment(V::Center)
                            .child(wifi)
                            .child(bluetooth),
                    ),
            )
            .child(
                stack_panel()
                    .grid_column(2)
                    .spacing(6.)
                    .child(quiet)
                    .child(tile(shell, Tile::Dark, layout)),
            )
            .into(),
        Layout::BottomNavigation => grid()
            .column_definitions("*,8,*")
            .row_definitions("Auto,8,Auto")
            .child(wifi)
            .child(border().grid_column(2).child(bluetooth))
            .child(border().grid_row(2).child(quiet))
            .child(
                border()
                    .grid_row(2)
                    .grid_column(2)
                    .child(tile(shell, Tile::Dark, layout)),
            )
            .into(),
    }
}
