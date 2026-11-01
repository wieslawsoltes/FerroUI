//! A retained FerroUI view. Platform services are injected by both the login session
//! and the windowed host. The workspace model always comes from the window manager.
pub mod appearance;
mod chrome;
mod desktop;
use appearance::{Appearance, ColorMode, ThemeController, WinUISettings};
mod launcher;
mod notifications;
mod quick_settings;
mod wallpaper;
pub mod wayland;
mod workspaces;

use self::chrome::*;
use crate::services::{ApplicationCatalog, ApplicationIcons, DesktopApplication};
use crate::session::{Session, Snapshot};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::FontWeight;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::Grid;
use ferroui_system::threading::tasks;
use ferroui_view::prelude::*;
use std::rc::Rc;
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scene {
    Desktop,
    Applications,
    Workspaces,
    Appearance,
    Notifications,
    QuickSettings,
    Calendar,
}

struct Shell {
    scene: Signal<Scene>,
    appearance: Signal<Appearance>,
    themes: Rc<ThemeController>,
    favorites: Signal<Vec<String>>,
    pin_editing: Signal<bool>,
    pin_limit: Signal<usize>,
    viewport_width: Signal<f64>,
    notifications: notifications::Notifications,
    settings: quick_settings::Settings,
    query: Signal<String>,
    page: Signal<usize>,
    catalog: Rc<ApplicationCatalog>,
    icons: ApplicationIcons,
    session: Option<Rc<Session>>,
    snapshot: Signal<Snapshot>,
    status: Signal<String>,
    launching: Signal<bool>,
    owner: Owner,
    clicked_output: std::cell::RefCell<Option<String>>,
    desktop_focus: NodeRef<ferroui_controls::Button>,
    shortcut_origin: std::cell::Cell<Option<crate::session::ShortcutOrigin>>,
    // First use mounts the launcher. Keep its template, layout and search input until the theme changes.
    cached_launcher: Signal<Option<Appearance>>,
    search_box: std::cell::RefCell<Option<ferroui_base::WeakRef<ferroui_controls::TextBox>>>,
}

#[derive(Clone, Copy)]
struct OutputMetrics {
    width: Signal<f64>,
    pins: Signal<usize>,
}

impl Shell {
    fn metrics(&self) -> OutputMetrics {
        ferroui_view::use_context::<OutputMetrics>().unwrap_or(OutputMetrics {
            width: self.viewport_width,
            pins: self.pin_limit,
        })
    }

    fn open(&self, target: Scene) {
        self.status.set_if_changed(String::new());
        self.show(target);
        if let Some(session) = &self.session {
            session.reveal_desktop();
        }
    }

    fn show(&self, target: Scene) {
        if target == Scene::Applications {
            batch(|| {
                self.cached_launcher
                    .set_if_changed(Some(self.appearance.get_untracked()));
                self.scene.set_if_changed(target);
            });
            if let Some(input) = self
                .search_box
                .borrow()
                .as_ref()
                .and_then(|input| input.upgrade())
            {
                input.focus();
            }
        } else {
            self.scene.set_if_changed(target);
        }
    }

    fn select_appearance(&self, appearance: Appearance) {
        self.finish_appearance(self.themes.select(appearance));
    }

    fn select_windows(&self, settings: WinUISettings) {
        self.finish_appearance(self.themes.set_windows(settings));
    }

    fn finish_appearance(&self, result: std::io::Result<()>) {
        let applied = self.themes.current();
        let decorations = apply_session_appearance(self.session.as_deref(), applied);
        if self.appearance.get_untracked() != applied {
            batch(|| {
                self.appearance.set(applied);
                self.cached_launcher
                    .set((self.scene.get_untracked() == Scene::Applications).then_some(applied));
            });
        }
        self.status.set(
            [
                result
                    .err()
                    .map(|e| format!("Could not save appearance. {e}")),
                decorations
                    .err()
                    .map(|e| format!("Could not update window decorations. {e}")),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" "),
        );
    }

    fn cancel(&self) {
        let origin = self.shortcut_origin.take();
        self.dismiss();
        if let (Some(session), Some(origin)) = (&self.session, origin) {
            session.restore_shortcut_origin(origin);
        }
    }

    fn dismiss(&self) {
        self.shortcut_origin.set(None);
        self.scene.set_if_changed(Scene::Desktop);
        if let Some(session) = &self.session {
            session.restore_windows();
        }
        if let Some(host) = self.session.as_ref().and_then(|s| s.compositor()) {
            host.focus_client();
        } else if let Some(control) = self.desktop_focus.get_untracked() {
            control.focus();
        }
    }

    fn launch(self: &Rc<Self>, application: &DesktopApplication) {
        if self.launching.get_untracked() {
            return;
        }
        self.launching.set(true);
        self.status.set(format!("Opening {}…", application.name));
        let application = application.clone();
        let display = self
            .session
            .as_ref()
            .and_then(|s| s.compositor())
            .map(|c| c.display());
        let weak = Rc::downgrade(self);
        let _task = tasks::spawn_local(async move {
            let result = tasks::run(move || {
                Ok(match display {
                    Some(name) => application.launch_on_display(&name)?,
                    None => application.launch()?,
                })
            })
            .await;
            if let Some(shell) = weak.upgrade().filter(|shell| shell.owner.is_alive()) {
                shell.launching.set(false);
                match result {
                    Ok(_) => {
                        shell.dismiss();
                        shell.query.set_if_changed(String::new());
                        shell.page.set_if_changed(0);
                    }
                    Err(error) => shell
                        .status
                        .set(format!("Could not open application. {error}")),
                }
            }
            Ok(())
        });
    }
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

fn clock() -> Signal<OffsetDateTime> {
    let initial = now();
    let value = signal(initial);
    // Sync to the next minute boundary; keep the wall clock and date accurate after suspend.
    let owner = Owner::current().expect("clock is mounted in an owner");
    let repeating = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = repeating.clone();
    let start = DispatcherTimer::run_once(
        move || {
            value.set(now());
            let timer = DispatcherTimer::run(
                move || {
                    if !owner.is_alive() {
                        return false;
                    }
                    value.set(now());
                    true
                },
                Duration::from_secs(60),
                DispatcherPriority::BACKGROUND,
            );
            *slot.borrow_mut() = Some(timer);
        },
        Duration::from_secs(60 - u64::from(initial.second())),
        DispatcherPriority::BACKGROUND,
    );
    ferroui_view::on_cleanup(move || {
        start.dispose();
        if let Some(timer) = repeating.borrow_mut().take() {
            timer.dispose();
        }
    });
    value
}

fn time_text(t: OffsetDateTime) -> String {
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn workspace_name(snapshot: &Snapshot) -> String {
    snapshot
        .workspace_names
        .get(snapshot.current_workspace)
        .cloned()
        .unwrap_or_else(|| format!("Workspace {}", snapshot.current_workspace.saturating_add(1)))
}

fn apply_session_appearance(
    session: Option<&Session>,
    appearance: Appearance,
) -> std::io::Result<()> {
    if let Some(session) = session {
        let (top, bottom) = appearance.insets();
        session.set_panel_insets(top, bottom);
        session.set_window_appearance(appearance)?;
    }
    Ok(())
}

fn create(
    initial: Scene,
    catalog: Rc<ApplicationCatalog>,
    session: Option<Rc<Session>>,
    themes: Rc<ThemeController>,
    managed: bool,
) -> Rc<Shell> {
    let appearance = signal(themes.current());
    ferroui_view::provide_context(appearance);
    let initial_status = apply_session_appearance(session.as_deref(), themes.current())
        .err()
        .map(|error| format!("Could not update window decorations. {error}"))
        .unwrap_or_default();
    let favorites = crate::services::favorites::load();
    let initial_status = [
        Some(initial_status),
        favorites
            .as_ref()
            .err()
            .map(|e| format!("Could not load pinned apps. {e}")),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" ");
    if !initial_status.is_empty() {
        eprintln!("{initial_status}");
    }
    let pin_limit = signal(4);
    let viewport_width = signal(1280.0);
    let state = Rc::new(Shell {
        scene: signal(initial),
        favorites: signal(favorites.unwrap_or_default()),
        pin_editing: signal(false),
        pin_limit,
        viewport_width,
        notifications: notifications::Notifications::new(managed),
        settings: quick_settings::Settings::new(),
        appearance,
        themes,
        query: signal(String::new()),
        page: signal(0),
        catalog,
        icons: ApplicationIcons::default(),
        snapshot: signal(session.as_ref().map(|s| s.snapshot()).unwrap_or_default()),
        session,
        status: signal(initial_status),
        launching: signal(false),
        clicked_output: Default::default(),
        owner: Owner::current().expect("desktop view must be mounted in a FerroUI owner"),
        desktop_focus: NodeRef::new(),
        shortcut_origin: Default::default(),
        cached_launcher: signal(
            (initial == Scene::Applications).then_some(appearance.get_untracked()),
        ),
        search_box: Default::default(),
    });

    if state
        .session
        .as_ref()
        .and_then(|s| s.compositor())
        .is_none()
    {
        notifications::attach(&state);
    }
    if let Some(session) = &state.session {
        let weak = Rc::downgrade(&state);
        if managed {
            let current = state.scene;
            let session = session.clone();
            effect(move || {
                session.publish_scene(match current.get() {
                    Scene::Desktop => "desktop",
                    Scene::Applications => "applications",
                    Scene::Workspaces => "workspaces",
                    Scene::QuickSettings => "settings",
                    Scene::Notifications => "notifications",
                    Scene::Calendar => "calendar",
                    Scene::Appearance => "appearance",
                })
            });
        }
        session.on_command(move |command| {
            use crate::session::commands::Command;
            let Some(state) = weak.upgrade().filter(|shell| shell.owner.is_alive()) else {
                return;
            };
            let scene = match command {
                Command::Applications => Scene::Applications,
                Command::Workspaces => Scene::Workspaces,
                Command::QuickSettings => Scene::QuickSettings,
                Command::Notifications => Scene::Notifications,
            };
            if state.scene.get_untracked() == scene {
                state.cancel();
            } else {
                if state.shortcut_origin.get().is_none() {
                    state
                        .shortcut_origin
                        .set(state.session.as_ref().and_then(|s| s.shortcut_origin()));
                }
                state.open(scene);
            }
        });
        let weak = Rc::downgrade(&state);
        session.subscribe(move || {
            if let Some(state) = weak.upgrade().filter(|s| s.owner.is_alive()) {
                if let Some(session) = &state.session {
                    state.snapshot.set(session.snapshot());
                }
            }
        });
        let weak = Rc::downgrade(&state);
        session.on_hidden({
            let shell = Rc::downgrade(&state);
            move || {
                if let Some(state) = shell.upgrade().filter(|s| s.owner.is_alive()) {
                    state.shortcut_origin.set(None);
                    state.scene.set_if_changed(Scene::Desktop);
                    if let Some(control) = state.desktop_focus.get_untracked() {
                        control.focus();
                    }
                }
            }
        });
        session.on_reveal(move || {
            if let Some(state) = weak.upgrade().filter(|s| s.owner.is_alive()) {
                if state.scene.get_untracked() == Scene::Desktop {
                    state.show(Scene::Applications);
                }
            }
        });
    }

    state
}

pub fn view(
    initial: Scene,
    catalog: Rc<ApplicationCatalog>,
    session: Option<Rc<Session>>,
    themes: Rc<ThemeController>,
    managed: bool,
) -> Element {
    let state = create(initial, catalog, session, themes, managed);
    root(state, true)
}

fn shortcut(shell: &Rc<Shell>, event: &ferroui_base::input::KeyEventArgs) {
    match (event.key, event.key_modifiers) {
        (Key::Escape, _) if shell.scene.get_untracked() != Scene::Desktop => {
            shell.cancel();
            event.set_handled(true);
        }
        (Key::Space, KeyModifiers::CONTROL) | (Key::K, KeyModifiers::CONTROL) => {
            if shell.scene.get_untracked() == Scene::Applications {
                shell.dismiss()
            } else {
                shell.open(Scene::Applications)
            }
            event.set_handled(true);
        }
        (key, modifiers)
            if shell.scene.get_untracked() == Scene::Appearance
                && modifiers.is_empty()
                && (Key::D1 as i32..=Key::D9 as i32).contains(&(key as i32)) =>
        {
            let index = (key as i32 - Key::D1 as i32) as usize;
            if let Some(appearance) = Appearance::ALL.get(index).copied() {
                shell.select_appearance(appearance);
                event.set_handled(true);
            }
        }
        (key, modifiers)
            if shell.scene.get_untracked() == Scene::Appearance
                && modifiers.is_empty()
                && matches!(key, Key::L | Key::D | Key::H | Key::C) =>
        {
            if let Some(mut settings) = shell.appearance.get_untracked().winui() {
                match key {
                    Key::L => settings.mode = ColorMode::Light,
                    Key::D => settings.mode = ColorMode::Dark,
                    Key::H => settings.mode = ColorMode::HighContrast,
                    _ => settings = settings.toggle_density(),
                }
                shell.select_windows(settings);
                event.set_handled(true);
            }
        }
        (Key::C, modifiers) if modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            shell.open(Scene::Calendar);
            event.set_handled(true);
        }
        (key, mods)
            if shell.scene.get_untracked() == Scene::QuickSettings
                && quick_settings::handle_key(&shell, key, mods) =>
        {
            event.set_handled(true);
        }
        (Key::N, KeyModifiers::CONTROL) => {
            shell.open(Scene::Notifications);
            event.set_handled(true);
        }
        (Key::Q, modifiers) if modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            shell.open(Scene::QuickSettings);
            event.set_handled(true);
        }
        (Key::OemComma, KeyModifiers::CONTROL) => {
            shell.open(Scene::Appearance);
            event.set_handled(true);
        }
        (Key::W, KeyModifiers::CONTROL) => {
            shell.open(Scene::Workspaces);
            event.set_handled(true);
        }
        (key, modifiers)
            if modifiers == (KeyModifiers::CONTROL | KeyModifiers::ALT)
                && (Key::D1 as i32..=Key::D9 as i32).contains(&(key as i32)) =>
        {
            let index = (key as i32 - Key::D1 as i32) as usize;
            if let Some(session) = shell.session.as_ref().filter(|_| {
                shell
                    .snapshot
                    .with_untracked(|s| index < s.workspace_names.len())
            }) {
                session.switch_workspace(index);
                event.set_handled(true);
            }
        }
        _ => {}
    }
}

fn composed_shortcut(shell: &Rc<Shell>, event: &ferroui_base::input::KeyEventArgs) {
    let Some(session) = shell.session.as_ref() else {
        return;
    };
    let Some(host) = session.compositor() else {
        return;
    };
    let key = event.key;
    let mods = event.key_modifiers;
    let active = || {
        shell
            .snapshot
            .with_untracked(|s| s.windows.iter().find(|w| w.active).map(|w| w.id))
    };
    if mods == KeyModifiers::META {
        let target = match key {
            Key::Space => Some(Scene::Applications),
            Key::A => Some(Scene::QuickSettings),
            Key::N => Some(Scene::Notifications),
            Key::Tab => Some(Scene::Workspaces),
            _ => None,
        };
        if let Some(target) = target {
            if shell.scene.get_untracked() == target {
                shell.cancel()
            } else {
                shell.open(target)
            }
            event.set_handled(true);
            return;
        }
    }
    if (Key::D1 as i32..=Key::D9 as i32).contains(&(key as i32)) {
        let index = (key as i32 - Key::D1 as i32) as usize;
        let switched = if mods == KeyModifiers::META {
            session.switch_workspace(index)
        } else if mods == (KeyModifiers::META | KeyModifiers::SHIFT) {
            active().is_some_and(|id| session.move_to_workspace(id, index))
        } else {
            false
        };
        if switched {
            event.set_handled(true);
            return;
        }
    }
    let handled = match (key, mods) {
        (Key::F, KeyModifiers::META) => active().is_some_and(|id| session.toggle_fullscreen(id)),
        (Key::Up, KeyModifiers::META) => active().is_some_and(|id| session.toggle_maximized(id)),
        (Key::Down, KeyModifiers::META) => active().is_some_and(|id| session.minimize(id)),
        (Key::Left | Key::Right, KeyModifiers::META) => active().is_some_and(|id| {
            session.snap(
                id,
                if key == Key::Left {
                    crate::session::SnapSide::Left
                } else {
                    crate::session::SnapSide::Right
                },
            )
        }),
        (Key::F4, KeyModifiers::ALT) => active().is_some_and(|id| session.close(id)),
        _ => false,
    };
    if handled {
        event.set_handled(true);
    } else if shell.scene.get_untracked() != Scene::Desktop || !host.client_has_focus() {
        shortcut(shell, event);
    }
}

fn root(state: Rc<Shell>, include_desktop: bool) -> Element {
    let appearance = state.appearance;
    let viewport_width = state.viewport_width;
    let pin_limit = state.pin_limit;
    let current = state.scene;
    let compositor = state.session.as_ref().and_then(|s| s.compositor()).cloned();
    let composed = compositor.is_some();
    let banner = if composed {
        notifications::composed_banner(state.clone())
    } else {
        Element::Empty
    };
    let desktop = if include_desktop {
        let clients = compositor.as_ref().map_or(Element::Empty, |c| c.element());
        desktop::surface(&state, clock(), clients)
    } else {
        Element::Empty
    };
    let root_state = state.clone();
    let launcher_state = state.clone();
    let overlays = state.clone();
    let dismiss = state.clone();
    grid()
        .name("DesktopShell")
        .on_size_changed(move |event| {
            let width = event.new_size().width;
            if let Some(manager) = &compositor {
                manager.resize(width.round() as i32, event.new_size().height.round() as i32);
            }
            viewport_width.set_if_changed(width);
            pin_limit.set_if_changed(if width >= 1024.0 {
                4
            } else if width >= 800.0 {
                2
            } else {
                0
            });
        })
        .clip_to_bounds(true)
        .with({
            let shell = state.clone();
            move |grid| {
                if composed {
                    let shell = shell.clone();
                    grid.add_handler_with(
                        ferroui_base::input::InputElement::key_down_event(),
                        move |_, e| composed_shortcut(&shell, e),
                        ferroui_base::interactivity::RoutingStrategies::TUNNEL,
                        false,
                    );
                }
            }
        })
        .on_key_down(move |event| {
            if include_desktop && !composed {
                shortcut(&root_state, event);
            }
        })
        .child(desktop)
        .child(
            grid()
                .is_hit_test_visible(move || current.get() != Scene::Desktop)
                .keyboard_navigation_tab_navigation(
                    ferroui_base::input::KeyboardNavigationMode::Cycle,
                )
                .child(
                    // Keep the click target mounted. A transparent hit-test brush
                    // catches outside presses without changing desktop pixels.
                    border()
                        .background("Transparent")
                        .on_pointer_pressed(move |e| {
                            dismiss.dismiss();
                            e.set_handled(true);
                        }),
                )
                .child(
                    border()
                        .background("#65081420")
                        .is_hit_test_visible(false)
                        .is_visible(move || match current.get() {
                            Scene::Desktop => false,
                            Scene::Applications => {
                                appearance.get().layout() == appearance::Layout::BottomNavigation
                            }
                            _ => true,
                        }),
                )
                .child(dynamic(move || {
                    match launcher_state.cached_launcher.get() {
                        Some(_) => launcher::view(&launcher_state),
                        None => Element::Empty,
                    }
                }))
                .child(dynamic(move || match (current.get(), appearance.get()) {
                    (Scene::QuickSettings, _) => quick_settings::view(&overlays),
                    (Scene::Calendar, _) => quick_settings::calendar_view(&overlays),
                    (Scene::Notifications, _) => notifications::view(&overlays),
                    (Scene::Appearance, _) => desktop::picker(&overlays),
                    (Scene::Workspaces, _) => workspaces::view(&overlays),
                    _ => Element::Empty,
                })),
        )
        .child(banner)
        .into()
}

fn popup_header(title: &str, caption: &str, state: &Rc<Shell>) -> El<Grid> {
    let dismiss = state.clone();
    let focus = NodeRef::<ferroui_controls::Button>::new();
    let scene = state.scene;
    grid()
        .column_definitions("*,Auto")
        .child(
            stack_panel()
                .spacing(5.0)
                .child(eyebrow(caption))
                .child(shell_text(title, 28.0, Ink::Primary).font_weight(FontWeight::SemiBold)),
        )
        .child(
            icon_action("Close panel", Icon::Close, move || dismiss.dismiss())
                .node_ref(focus)
                .on_loaded(move |_| {
                    if matches!(
                        scene.get_untracked(),
                        Scene::Workspaces | Scene::Calendar | Scene::QuickSettings
                    ) {
                        if let Some(control) = focus.get_untracked() {
                            control.focus();
                        }
                    }
                })
                .grid_column(1)
                .vertical_alignment(V::Top),
        )
}

fn feedback(status: Signal<String>) -> El<Grid> {
    let is_windows = look().winui().is_some();
    grid()
        .is_visible(move || status.with(|s| !s.is_empty()))
        .child(dynamic(move || {
            if status.with(String::is_empty) {
                Element::Empty
            } else if is_windows {
                info_bar()
                    .is_open(true)
                    .is_closable(false)
                    .message(status)
                    .into()
            } else {
                shell_text("", 12.0, Ink::Accent)
                    .text(status)
                    .text_wrapping(ferroui_base::media::TextWrapping::Wrap)
                    .into()
            }
        }))
}
