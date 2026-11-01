//! Platform-specific desktop composition; scenes use the same session, catalog and input model.
use super::appearance::{Appearance, ColorMode, Layout, WinUISettings};
use super::chrome::*;
use super::{feedback, popup_header, time_text, wallpaper, workspace_name, Scene, Shell};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::{Border, Button};
use ferroui_themes_winui::DensityStyle;
use ferroui_view::prelude::*;
use std::rc::Rc;
use time::OffsetDateTime;

pub(super) fn background(
    state: &Rc<Shell>,
    clock: Signal<OffsetDateTime>,
    appearance: Appearance,
) -> Element {
    grid()
        .child(wallpaper::wallpaper(appearance))
        .child(match appearance.layout() {
            Layout::Taskbar => windows_icons(state),
            Layout::MenuBarDock => Element::Empty,
            Layout::BottomNavigation => material_home(state, clock),
        })
        .into()
}

pub(super) fn panel(
    state: &Rc<Shell>,
    clock: Signal<OffsetDateTime>,
    appearance: Appearance,
    top: bool,
) -> Element {
    match (appearance.layout(), top) {
        (Layout::Taskbar, false) => taskbar(state, clock),
        (Layout::MenuBarDock, true) => menubar(state, clock),
        (Layout::MenuBarDock, false) => dock(state),
        (Layout::BottomNavigation, false) => navigation(state),
        _ => Element::Empty,
    }
}

pub(super) fn surface(state: &Rc<Shell>, clock: Signal<OffsetDateTime>, clients: Element) -> Element {
    let scene = state.scene;
    let bg = state.clone();
    let edge = |top: bool| {
        let state = state.clone();
        dynamic(move || {
            let appearance = state.appearance.get();
            let (upper, lower) = appearance.insets();
            border().height(if top { upper } else { lower })
                .vertical_alignment(if top { V::Top } else { V::Bottom })
                .child(panel(&state, clock, appearance, top))
        })
    };
    // Keep the native client visual tree alive across appearance changes.
    grid()
        .is_hit_test_visible(move || scene.get() == Scene::Desktop)
        .child(dynamic(move || background(&bg, clock, bg.appearance.get())))
        .child(clients)
        .child(edge(true))
        .child(edge(false))
        .into()
}

fn command(state: &Rc<Shell>, target: Scene, name: &str) -> El<Button> {
    let state = state.clone();
    action(name, move || state.open(target))
}

/// Bind the desktop's initial focus to its primary command. Mounting an appearance
/// while a sheet is open must leave focus inside that sheet.
fn focus_home(
    state: &Rc<Shell>,
) -> impl Fn(&ferroui_base::interactivity::RoutedEventArgs) + 'static {
    let scene = state.scene;
    let button = state.desktop_focus;
    move |_| {
        if scene.get_untracked() == Scene::Desktop {
            if let Some(button) = button.get_untracked() {
                button.focus();
            }
        }
    }
}

/// The compact area shows the leading clients on this workspace; the overview
/// always contains the full list. Keyed entries survive unrelated X events.
fn running_clients(state: &Rc<Shell>) -> Element {
    let snapshot = state.snapshot;
    let state = state.clone();
    for_each(
        move || {
            snapshot.with(|s| {
                s.windows
                    .iter()
                    .filter(|w| w.workspace.is_none_or(|i| i == s.current_workspace))
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
            })
        },
        |window| window.id,
        move |window| {
            let shell = state.clone();
            let id = window.id;
            let title = if window.title.is_empty() {
                "Window"
            } else {
                &window.title
            };
            let initial = title
                .trim()
                .chars()
                .next()
                .unwrap_or('•')
                .to_uppercase()
                .to_string();
            action(title, move || {
                if let Some(session) = &shell.session {
                    session.activate(id);
                }
            })
            .width(34.0)
            .height(36.0)
            .padding(2)
            .corner_radius(7)
            .child(
                stack_panel()
                    .spacing(2.0)
                    .child(shell_text(&initial, 13.0, Ink::Primary).horizontal_alignment(H::Center))
                    .child(
                        border()
                            .width(if window.active { 13.0 } else { 5.0 })
                            .height(2.0)
                            .corner_radius(1)
                            .background(if window.active {
                                colors().accent
                            } else {
                                colors().muted
                            }),
                    ),
            )
        },
    )
}

fn date(t: OffsetDateTime) -> String {
    format!("{:02}/{:02}/{}", u8::from(t.month()), t.day(), t.year())
}

fn desktop_shortcut(state: &Rc<Shell>, scene: Scene, name: &str, icon: Icon) -> Element {
    command(state, scene, name)
        .width(85.0)
        .height(70.0)
        .background("Transparent")
        .border_thickness(0)
        .padding(2)
        .child(
            stack_panel()
                .spacing(5.0)
                .child(glyph(icon, 28.0, "#DFF3FF"))
                .child(shell_text(name, 12.0, "#FFFFFF").horizontal_alignment(H::Center)),
        )
        .into()
}

fn status_button(state: &Rc<Shell>, scene: Scene, label: &str, icon: Icon) -> El<Button> {
    command(state, scene, label)
        .class("text")
        .min_width(34)
        .height(32)
        .padding((7.0, 4.0))
        .background("Transparent")
        .border_thickness(0)
        .child(glyph(icon, 17.0, colors().bar_ink))
}

fn notification_button(state: &Rc<Shell>) -> Element {
    let counter = state.clone();
    let badge = if look().winui().is_some() {
        let value = state.clone();
        info_badge()
            .value(move || value.notifications.count().min(i32::MAX as usize) as i32)
            .automation_properties_name("Unread notifications")
            .is_visible(move || counter.notifications.count() != 0)
            .is_hit_test_visible(false)
            .vertical_alignment(V::Top)
            .horizontal_alignment(H::Right)
            .into_element()
    } else {
        border()
            .width(7)
            .height(7)
            .corner_radius(4)
            .background(colors().accent)
            .margin((0.0, 2.0, 3.0, 0.0))
            .vertical_alignment(V::Top)
            .horizontal_alignment(H::Right)
            .is_hit_test_visible(false)
            .is_visible(move || counter.notifications.count() != 0)
            .into_element()
    };
    grid()
        .child(status_button(
            state,
            Scene::Notifications,
            "Notification center",
            Icon::Bell,
        ))
        .child(badge)
        .into()
}

fn windows_icons(state: &Rc<Shell>) -> Element {
    stack_panel()
        .margin((10.0, 17.0))
        .spacing(5.0)
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Top)
        .child(desktop_shortcut(
            state,
            Scene::Applications,
            "Applications",
            Icon::Applications,
        ))
        .child(desktop_shortcut(
            state,
            Scene::Workspaces,
            "Task view",
            Icon::Workspaces,
        ))
        .into()
}

fn taskbar(state: &Rc<Shell>, clock: Signal<OffsetDateTime>) -> Element {
    let snapshot = state.snapshot;
    let width = state.metrics().width;
    let compact = memo(move |_| width.get() < 760.0);
    let start = command(state, Scene::Applications, "Open applications")
        .name("Start")
        .node_ref(state.desktop_focus)
        .on_loaded(focus_home(state))
        .width(42.0)
        .height(40.0)
        .background("Transparent")
        .border_thickness(0)
        .child(glyph(Icon::Applications, 23.0, colors().dock_tiles[0]));
    let search = command(state, Scene::Applications, "Search applications")
        .is_visible(move || !compact.get())
        .corner_radius(20)
        .height(34.0)
        .padding((14.0, 2.0))
        .child(
            row(9.0)
                .child(glyph(Icon::Search, 14.0, colors().muted))
                .child(shell_text("Search", 13.0, Ink::Secondary)),
        );
    let clock_labels = stack_panel()
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Center)
        .child(
            shell_text("", 12.0, Ink::Primary)
                .horizontal_alignment(H::Right)
                .text(move || time_text(clock.get())),
        )
        .child(
            shell_text("", 11.0, Ink::Secondary)
                .is_visible(move || !compact.get())
                .horizontal_alignment(H::Right)
                .text(move || date(clock.get())),
        );
    let tray = row(2.0)
        .horizontal_alignment(H::Right)
        .child(status_button(
            state,
            Scene::QuickSettings,
            "Quick settings",
            Icon::Settings,
        ))
        .child(notification_button(state))
        .child(
            command(state, Scene::Calendar, "Open calendar")
                .padding((6.0, 0.0))
                .background("Transparent")
                .border_thickness(0)
                .child(clock_labels),
        );
    let center = row(5.0)
        .horizontal_alignment(move || if compact.get() { H::Left } else { H::Center })
        .child(start)
        .child(search)
        .child(
            command(state, Scene::Workspaces, "Workspace overview")
                .width(40.0)
                .height(40.0)
                .background("Transparent")
                .border_thickness(0)
                .child(glyph(Icon::Workspaces, 19.0, colors().dock_tiles[1])),
        )
        .child(
            command(state, Scene::Appearance, "Appearance settings")
                .width(40.0)
                .height(40.0)
                .background("Transparent")
                .border_thickness(0)
                .child(glyph(Icon::Appearance, 19.0, colors().dock_tiles[2])),
        )
        .child(super::launcher::pinned_shortcuts(state))
        .child(
            row(3.0)
                .is_visible(move || !compact.get())
                .child(running_clients(state)),
        );
    let panel = if look().winui().is_some() {
        border()
            .background(resource("SolidBackgroundFillColorBaseBrush"))
            .border_brush(resource("CardStrokeColorDefaultBrush"))
    } else {
        border()
            .background(colors().bar)
            .border_brush(colors().bar_edge)
    };
    panel
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .child(
            grid()
                .margin((10.0, 0.0))
                .child(
                    shell_text("", 11.0, Ink::Secondary)
                        .is_visible(move || !compact.get())
                        .margin((5.0, 0.0))
                        .text(move || snapshot.with(workspace_name)),
                )
                .child(center)
                .child(
                    border()
                        .horizontal_alignment(H::Right)
                        .padding((0.0, 0.0, 5.0, 0.0))
                        .child(tray),
                ),
        )
        .into()
}

fn menubar(state: &Rc<Shell>, clock: Signal<OffsetDateTime>) -> Element {
    let width = state.metrics().width;
    let compact = memo(move |_| width.get() < 550.0);
    let menu = |scene, title: &'static str| {
        command(state, scene, title)
            .is_visible(move || !compact.get())
            .background("Transparent")
            .border_thickness(0)
            .padding((7.0, 0.0))
            .child(shell_text(title, 12.0, Ink::Primary))
    };

    border()
        .background(colors().bar)
        .child(
            grid()
                .column_definitions("*,Auto")
                .margin((12.0, 0.0))
                .child(
                    row(1.0)
                        .child(
                            command(state, Scene::Applications, "Open applications")
                                .node_ref(state.desktop_focus)
                                .on_loaded(focus_home(state))
                                .padding((4.0, 0.0))
                                .background("Transparent")
                                .border_thickness(0)
                                .child(
                                    row(6.0)
                                        .child(glyph(Icon::Mark, 15.0, colors().bar_ink))
                                        .child(
                                            shell_text("Ferro", 13.0, colors().bar_ink)
                                                .font_weight(FontWeight::Bold),
                                        ),
                                ),
                        )
                        .child(menu(Scene::Applications, "Applications"))
                        .child(menu(Scene::Workspaces, "Window"))
                        .child(menu(Scene::Appearance, "Appearance")),
                )
                .child(
                    row(2.0)
                        .grid_column(1)
                        .child(status_button(
                            state,
                            Scene::QuickSettings,
                            "Control center",
                            Icon::Settings,
                        ))
                        .child(notification_button(state))
                        .child(
                            command(state, Scene::Calendar, "Open calendar")
                                .height(28)
                                .padding((5.0, 0.0))
                                .border_thickness(0)
                                .background("Transparent")
                                .child(
                                    shell_text("", 12.0, colors().bar_ink)
                                        .text(move || time_text(clock.get())),
                                ),
                        ),
                ),
        )
        .into()
}

fn dock(state: &Rc<Shell>) -> Element {
    let width = state.metrics().width;
    let compact = memo(move |_| width.get() < 550.0);
    let dock_tile = |scene, title: &'static str, icon, fill: &'static str| {
        command(state, scene, title)
            .width(45.0)
            .height(45.0)
            .padding(7)
            .corner_radius(12)
            .background(fill)
            .border_thickness(1)
            .border_brush(colors().bar_edge)
            .child(glyph(icon, 22.0, "#FFFFFF"))
    };
    border()
        .vertical_alignment(V::Center)
        .horizontal_alignment(H::Center)
        .corner_radius(19)
        .border_brush(colors().bar_edge)
        .border_thickness(1)
        .padding(5)
        .background(colors().bar)
        .child(
            row(7.0)
                .child(dock_tile(
                    Scene::Applications,
                    "Applications",
                    Icon::Applications,
                    colors().dock_tiles[0],
                ))
                .child(dock_tile(
                    Scene::Workspaces,
                    "Mission Control",
                    Icon::Workspaces,
                    colors().dock_tiles[1],
                ))
                .child(dock_tile(
                    Scene::Appearance,
                    "Appearance settings",
                    Icon::Appearance,
                    colors().dock_tiles[2],
                ))
                .child(super::launcher::pinned_shortcuts(state))
                .child(
                    row(3.0)
                        .is_visible(move || !compact.get())
                        .child(running_clients(state)),
                ),
        )
        .into()
}

fn material_home(state: &Rc<Shell>, clock: Signal<OffsetDateTime>) -> Element {
    stack_panel()
        .margin((34.0, 32.0))
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Top)
        .child(
            command(state, Scene::Calendar, "Open calendar")
                .class("text")
                .padding(0)
                .horizontal_alignment(H::Left)
                .background("Transparent")
                .border_thickness(0)
                .child(shell_text("", 17.0, "#D4E7D9").text(move || {
                    let t = clock.get();
                    format!("{}, {} {}", t.weekday(), t.month(), t.day())
                })),
        )
        .child(
            shell_text("", 76.0, "#E1F9E7")
                .font_weight(FontWeight::Light)
                .letter_spacing(-3.0)
                .text(move || time_text(clock.get())),
        )
        .child(row(2.0).child(super::launcher::pinned_shortcuts(state)))
        .into()
}

fn navigation(state: &Rc<Shell>) -> Element {
    let scene = state.scene;
    let tab = |destination, name: &'static str, icon, col| {
        let shell = state.clone();
        action(name, move || {
            if destination == Scene::Desktop {
                shell.dismiss();
            } else {
                shell.open(destination);
            }
        })
        .class("text")
        .grid_column(col)
        .horizontal_alignment(H::Stretch)
        .height(54.0)
        .min_width(42.0)
        .padding((1.0, 0.0))
        .child(
            stack_panel()
                .spacing(2.0)
                .child(
                    grid()
                        .width(50.0)
                        .height(25.0)
                        .child(
                            border()
                                .corner_radius(14)
                                .background(colors().selected)
                                .is_visible(move || scene.get() == destination),
                        )
                        .child(glyph(icon, 16.0, colors().bar_ink)),
                )
                .child(
                    shell_text(name, 11.0, colors().bar_ink)
                        .horizontal_alignment(H::Center)
                        .font_weight(FontWeight::Medium),
                ),
        )
    };
    border()
        .background(colors().bar)
        .padding((12.0, 3.0))
        .child(
            grid()
                .row_definitions("*,6")
                .child(
                    grid()
                        .max_width(550.0)
                        .column_definitions("*,*,*,*,*,*")
                        .child(
                            tab(Scene::Desktop, "Home", Icon::Mark, 0)
                                .node_ref(state.desktop_focus)
                                .on_loaded(focus_home(state)),
                        )
                        .child(tab(Scene::Applications, "Apps", Icon::Applications, 1))
                        .child(tab(Scene::Workspaces, "Overview", Icon::Workspaces, 2))
                        .child(tab(Scene::Notifications, "Inbox", Icon::Bell, 3))
                        .child(tab(Scene::QuickSettings, "Controls", Icon::Settings, 4))
                        .child(tab(Scene::Appearance, "Style", Icon::Appearance, 5)),
                )
                .child(
                    border()
                        .grid_row(1)
                        .width(82.0)
                        .height(3.0)
                        .corner_radius(2)
                        .background(colors().bar_ink),
                ),
        )
        .into()
}

/// System flyouts open at the status area for the active desktop layout.
pub(super) fn system_frame(shell: &Shell, preferred_width: f64) -> El<Border> {
    let width = shell.metrics().width;
    let (top, bottom) = look().insets();
    match look().layout() {
        Layout::Taskbar => shell_panel()
            .horizontal_alignment(H::Right)
            .vertical_alignment(V::Bottom)
            .margin((10.0, 10.0, 10.0, bottom + 10.0)),
        Layout::MenuBarDock => shell_panel()
            .horizontal_alignment(H::Right)
            .vertical_alignment(V::Top)
            .margin((10.0, top + 8.0, 10.0, 10.0)),
        Layout::BottomNavigation => popup_frame(),
    }
    .width(move || (width.get() - 24.0).clamp(0.0, preferred_width))
}

pub(super) fn popup_frame() -> El<Border> {
    let (top, bottom) = look().insets();
    match look().layout() {
        Layout::Taskbar => {
            shell_panel()
                .vertical_alignment(V::Bottom)
                .margin((12.0, 10.0, 12.0, bottom + 9.0))
        }
        Layout::MenuBarDock => shell_panel().vertical_alignment(V::Center).margin((
            20.0,
            top + 5.0,
            20.0,
            bottom + 4.0,
        )),
        Layout::BottomNavigation => shell_panel()
            .vertical_alignment(V::Bottom)
            .margin((8.0, 12.0, 8.0, 7.0)),
    }
}

fn windows_settings(shell: &Rc<Shell>, settings: WinUISettings) -> Element {
    let mut modes = wrap_panel().automation_properties_name("Windows color mode");
    for mode in ColorMode::ALL {
        let state = shell.clone();
        modes = modes.child(
            radio_button()
                .group_name("WinUIColorMode")
                .automation_properties_name(mode.label())
                .content(mode.label())
                .margin((0.0, 0.0, 12.0, 0.0))
                .is_checked(Some(mode == settings.mode))
                .on_click(move |_| state.select_windows(WinUISettings { mode, ..settings })),
        );
    }
    let state = shell.clone();
    let compact = settings.density == DensityStyle::Compact;
    let toggle = toggle_switch()
        .automation_properties_name("Compact controls")
        .off_content("Off")
        .on_content("On")
        .is_checked(Some(compact))
        .on_is_checked_changed(move |e| {
            let Some(toggle) = e
                .source()
                .and_then(|source| source.downcast::<ferroui_controls::ToggleSwitch>().ok())
            else {
                return;
            };
            let checked = toggle.is_checked() == Some(true);
            if checked != compact {
                state.select_windows(settings.toggle_density());
            }
        });
    stack_panel()
        .spacing(4.0)
        .margin((0.0, 8.0, 0.0, 0.0))
        .child(eyebrow("WINDOWS PERSONALIZATION"))
        .child(shell_text("Color mode", 14.0, Ink::Primary).font_weight(FontWeight::SemiBold))
        .child(modes)
        .child(
            grid()
                .column_definitions("*,Auto")
                .margin((0.0, 6.0, 0.0, 0.0))
                .child(
                    stack_panel()
                        .vertical_alignment(V::Center)
                        .child(
                            shell_text("Compact controls", 14.0, Ink::Primary)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(shell_text(
                            "Use tighter control spacing and a shorter taskbar",
                            11.0,
                            Ink::Secondary,
                        )),
                )
                .child(toggle.grid_column(1)),
        )
        .into()
}

pub(super) fn picker(state: &Rc<Shell>) -> Element {
    let mut options = stack_panel().spacing(6.0);
    for (number, appearance) in Appearance::ALL.iter().copied().enumerate() {
        let shell = state.clone();
        let selected = appearance.id() == state.themes.current().id();
        let description = match appearance.layout() {
            Layout::Taskbar => "Centered taskbar and Start menu",
            Layout::MenuBarDock => "Menu bar, floating Dock and Spotlight-style search",
            Layout::BottomNavigation => "Touch navigation and bottom sheets",
        };
        let color = appearance.palette();
        let name = appearance.name();
        let hint = match (selected, number < 9) {
            (true, true) => format!("Active  ·  {}", number + 1),
            (true, false) => "Active".to_owned(),
            (false, true) => (number + 1).to_string(),
            (false, false) => String::new(),
        };
        let focus = NodeRef::<Button>::new();
        options = options.child(
            action(&format!("Use {name} appearance"), move || {
                shell.select_appearance(appearance);
            })
            .node_ref(focus)
            .on_loaded(move |_| {
                if selected {
                    if let Some(button) = focus.get_untracked() {
                        button.focus();
                    }
                }
            })
            .horizontal_alignment(H::Stretch)
            .horizontal_content_alignment(H::Stretch)
            .padding((12.0, 8.0))
            .child(
                grid()
                    .column_definitions("28,*,Auto")
                    .child(
                        border()
                            .width(26.0)
                            .height(26.0)
                            .corner_radius(7)
                            .background(color.surface)
                            .border_thickness(1)
                            .border_brush(color.edge)
                            .child(glyph(Icon::Appearance, 16.0, color.accent)),
                    )
                    .child(
                        stack_panel()
                            .grid_column(1)
                            .margin((10.0, 0.0))
                            .child(
                                shell_text(name, 15.0, Ink::Primary)
                                    .font_weight(FontWeight::SemiBold),
                            )
                            .child(shell_text(description, 11.0, Ink::Secondary)),
                    )
                    .child(
                        shell_text(&hint, 12.0, Ink::Accent)
                            .grid_column(2)
                            .horizontal_alignment(H::Right),
                    ),
            ),
        );
    }
    if let Some(settings) = state.themes.current().winui() {
        options = options.child(windows_settings(state, settings));
    }
    popup_frame()
        .max_width(530.0)
        .padding(20)
        .child(
            stack_panel()
                .spacing(14.0)
                .child(popup_header("Appearance", "PERSONALIZATION", state))
                .child(scroll_viewer().max_height(440.0).child(options))
                .child(feedback(state.status)),
        )
        .into()
}
