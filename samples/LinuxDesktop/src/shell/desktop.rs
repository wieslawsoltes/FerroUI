//! Platform-specific desktop composition; scenes use the same session, catalog and input model.
use super::appearance::{Appearance, Layout};
use super::chrome::*;
use super::{feedback, popup_header, time_text, wallpaper, workspace_name, Scene, Shell};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::{Border, Button};
use ferroui_view::prelude::*;
use std::rc::Rc;
use time::OffsetDateTime;

pub(super) fn surface(
    state: &Rc<Shell>,
    clock: Signal<OffsetDateTime>,
    appearance: Appearance,
) -> Element {
    let scene = state.scene;
    let shell = match appearance.layout() {
        Layout::Taskbar => taskbar(state, clock),
        Layout::MenuBarDock => menubar_and_dock(state, clock),
        Layout::BottomNavigation => material_home(state, clock),
    };
    grid()
        .child(wallpaper::wallpaper(appearance))
        .child(
            grid()
                .is_enabled(move || scene.get() == Scene::Desktop)
                .child(shell),
        )
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
                    .child(label(&initial, 13.0, Ink::Primary).horizontal_alignment(H::Center))
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
                .child(label(name, 12.0, "#FFFFFF").horizontal_alignment(H::Center)),
        )
        .into()
}

fn taskbar(state: &Rc<Shell>, clock: Signal<OffsetDateTime>) -> Element {
    let snapshot = state.snapshot;
    let start = command(state, Scene::Applications, "Open applications")
        .name("Start")
        .node_ref(state.desktop_focus)
        .on_loaded(focus_home(state))
        .width(42.0)
        .height(40.0)
        .background("Transparent")
        .border_thickness(0)
        .child(glyph(Icon::Applications, 23.0, "#0078D4"));
    let search = command(state, Scene::Applications, "Search applications")
        .corner_radius(20)
        .height(34.0)
        .padding((14.0, 2.0))
        .child(
            row(9.0)
                .child(glyph(Icon::Search, 14.0, colors().muted))
                .child(label("Search", 13.0, Ink::Secondary)),
        );
    let tray = stack_panel()
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Center)
        .child(
            label("", 12.0, Ink::Primary)
                .horizontal_alignment(H::Right)
                .text(move || time_text(clock.get())),
        )
        .child(
            label("", 11.0, Ink::Secondary)
                .horizontal_alignment(H::Right)
                .text(move || date(clock.get())),
        );
    let center = row(5.0)
        .horizontal_alignment(H::Center)
        .child(start)
        .child(search)
        .child(
            command(state, Scene::Workspaces, "Workspace overview")
                .width(40.0)
                .height(40.0)
                .background("Transparent")
                .border_thickness(0)
                .child(glyph(Icon::Workspaces, 19.0, "#326AAA")),
        )
        .child(
            command(state, Scene::Appearance, "Appearance settings")
                .width(40.0)
                .height(40.0)
                .background("Transparent")
                .border_thickness(0)
                .child(glyph(Icon::Appearance, 19.0, "#465E78")),
        )
        .child(running_clients(state));
    grid()
        .row_definitions("*,52")
        .child(
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
                )),
        )
        .child(
            border()
                .grid_row(1)
                .background("#F5EAF0F7")
                .border_brush("#F2FFFFFF")
                .border_thickness((0.0, 1.0, 0.0, 0.0))
                .child(
                    grid()
                        .column_definitions("*,Auto,*")
                        .margin((10.0, 0.0))
                        .child(
                            label("", 11.0, Ink::Secondary)
                                .margin((5.0, 0.0))
                                .text(move || snapshot.with(workspace_name)),
                        )
                        .child(border().grid_column(1).child(center))
                        .child(
                            border()
                                .grid_column(2)
                                .padding((0.0, 0.0, 5.0, 0.0))
                                .child(tray),
                        ),
                ),
        )
        .into()
}

fn menubar_and_dock(state: &Rc<Shell>, clock: Signal<OffsetDateTime>) -> Element {
    let menu = |scene, title: &'static str| {
        command(state, scene, title)
            .background("Transparent")
            .border_thickness(0)
            .padding((7.0, 0.0))
            .child(label(title, 12.0, Ink::Primary))
    };
    let dock_tile = |scene, title: &'static str, icon, fill: &'static str| {
        command(state, scene, title)
            .width(45.0)
            .height(45.0)
            .padding(7)
            .corner_radius(12)
            .background(fill)
            .border_thickness(1)
            .border_brush("#60FFFFFF")
            .child(glyph(icon, 22.0, "#FFFFFF"))
    };
    grid()
        .row_definitions("34,*,64")
        .child(
            border().background("#E8383044").child(
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
                                        row(6.0).child(glyph(Icon::Mark, 15.0, "#FFFFFF")).child(
                                            label("Ferro", 13.0, "#FFFFFF")
                                                .font_weight(FontWeight::Bold),
                                        ),
                                    ),
                            )
                            .child(menu(Scene::Applications, "Applications"))
                            .child(menu(Scene::Workspaces, "Window"))
                            .child(menu(Scene::Appearance, "Appearance")),
                    )
                    .child(
                        label("", 12.0, "#FBF4FF")
                            .grid_column(1)
                            .text(move || time_text(clock.get())),
                    ),
            ),
        )
        .child(
            border()
                .grid_row(2)
                .vertical_alignment(V::Center)
                .horizontal_alignment(H::Center)
                .corner_radius(19)
                .border_brush("#70F7E9FF")
                .border_thickness(1)
                .padding(5)
                .background("#CA514B59")
                .child(
                    row(7.0)
                        .child(dock_tile(
                            Scene::Applications,
                            "Applications",
                            Icon::Applications,
                            "#635BA4",
                        ))
                        .child(dock_tile(
                            Scene::Workspaces,
                            "Mission Control",
                            Icon::Workspaces,
                            "#3185B1",
                        ))
                        .child(dock_tile(
                            Scene::Appearance,
                            "Appearance settings",
                            Icon::Appearance,
                            "#81617E",
                        ))
                        .child(running_clients(state)),
                ),
        )
        .into()
}

fn material_home(state: &Rc<Shell>, clock: Signal<OffsetDateTime>) -> Element {
    let tab = |scene, name: &'static str, icon| {
        command(state, scene, name)
            .height(54.0)
            .max_width(145.0)
            .padding((12.0, 2.0))
            .corner_radius(25)
            .child(
                stack_panel()
                    .spacing(2.0)
                    .child(glyph(icon, 17.0, colors().accent))
                    .child(
                        label(name, 11.0, Ink::Primary)
                            .font_weight(FontWeight::Medium)
                            .horizontal_alignment(H::Center),
                    ),
            )
    };
    grid()
        .row_definitions("*,72")
        .child(
            stack_panel()
                .margin((34.0, 32.0))
                .horizontal_alignment(H::Left)
                .vertical_alignment(V::Top)
                .child(label("", 17.0, "#D4E7D9").text(move || {
                    let t = clock.get();
                    format!("{}, {} {}", t.weekday(), t.month(), t.day())
                }))
                .child(
                    label("", 76.0, "#E1F9E7")
                        .font_weight(FontWeight::Light)
                        .letter_spacing(-3.0)
                        .text(move || time_text(clock.get())),
                ),
        )
        .child(
            border()
                .grid_row(1)
                .background("#F41D3027")
                .padding((12.0, 3.0))
                .child(
                    grid()
                        .row_definitions("*,6")
                        .child(
                            row(8.0)
                                .horizontal_alignment(H::Center)
                                .child(
                                    tab(Scene::Applications, "Apps", Icon::Applications)
                                        .node_ref(state.desktop_focus)
                                        .on_loaded(focus_home(state)),
                                )
                                .child(tab(Scene::Workspaces, "Overview", Icon::Workspaces))
                                .child(tab(Scene::Appearance, "Style", Icon::Appearance)),
                        )
                        .child(
                            border()
                                .grid_row(1)
                                .width(82.0)
                                .height(3.0)
                                .corner_radius(2)
                                .background("#BDD4C2"),
                        ),
                ),
        )
        .into()
}

pub(super) fn popup_frame() -> El<Border> {
    match look().layout() {
        Layout::Taskbar => panel()
            .vertical_alignment(V::Bottom)
            .margin((12.0, 10.0, 12.0, 61.0)),
        Layout::MenuBarDock => panel()
            .vertical_alignment(V::Center)
            .margin((20.0, 39.0, 20.0, 68.0)),
        Layout::BottomNavigation => panel()
            .vertical_alignment(V::Bottom)
            .margin((8.0, 12.0, 8.0, 7.0)),
    }
}

pub(super) fn picker(state: &Rc<Shell>) -> Element {
    let mut options = stack_panel().spacing(6.0);
    for (number, appearance) in Appearance::ALL.into_iter().enumerate() {
        let shell = state.clone();
        let selected = appearance == state.themes.current();
        let description = match appearance.layout() {
            Layout::Taskbar => "Centered taskbar and Start menu",
            Layout::MenuBarDock => "Menu bar, floating Dock and Spotlight-style search",
            Layout::BottomNavigation => "Touch navigation and bottom sheets",
        };
        let color = palette(appearance);
        let name = appearance.name();
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
                                label(name, 15.0, Ink::Primary).font_weight(FontWeight::SemiBold),
                            )
                            .child(label(description, 11.0, Ink::Secondary)),
                    )
                    .child(
                        label(
                            &if selected {
                                format!("Active  ·  {}", number + 1)
                            } else {
                                format!("{}", number + 1)
                            },
                            12.0,
                            Ink::Accent,
                        )
                        .grid_column(2)
                        .horizontal_alignment(H::Right),
                    ),
            ),
        );
    }
    popup_frame()
        .max_width(530.0)
        .padding(20)
        .child(
            stack_panel()
                .spacing(14.0)
                .child(popup_header("Appearance", "PERSONALIZATION", state))
                .child(options)
                .child(feedback(state.status)),
        )
        .into()
}
