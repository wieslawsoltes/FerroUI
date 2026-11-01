//! A retained FerroUI view. Platform services are injected by both the login session
//! and the windowed host. The workspace model always comes from the window manager.
pub mod appearance;
mod chrome;
mod desktop;
use appearance::{Appearance, ThemeController};
mod launcher;
mod wallpaper;
mod workspaces;

use self::chrome::*;
use crate::services::{ApplicationCatalog, ApplicationIcons, DesktopApplication};
use crate::session::{Session, Snapshot};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::FontWeight;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{Grid, TextBlock};
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
}

struct Shell {
    scene: Signal<Scene>,
    appearance: Signal<Appearance>,
    themes: Rc<ThemeController>,
    query: Signal<String>,
    page: Signal<usize>,
    catalog: Rc<ApplicationCatalog>,
    icons: ApplicationIcons,
    session: Option<Rc<Session>>,
    snapshot: Signal<Snapshot>,
    status: Signal<String>,
    launching: Signal<bool>,
    owner: Owner,
    desktop_focus: NodeRef<ferroui_controls::Button>,
}

impl Shell {
    fn open(&self, target: Scene) {
        self.status.set_if_changed(String::new());
        self.scene.set_if_changed(target);
        if let Some(session) = &self.session {
            session.reveal_desktop();
        }
    }

    fn select_appearance(&self, appearance: Appearance) {
        let result = self.themes.select(appearance);
        let applied = self.themes.current();
        let decorations = apply_session_appearance(self.session.as_deref(), applied);
        self.appearance.set_if_changed(applied);
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

    fn dismiss(&self) {
        self.scene.set_if_changed(Scene::Desktop);
        if let Some(session) = &self.session {
            session.restore_windows();
        }
        if let Some(control) = self.desktop_focus.get_untracked() {
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
        let weak = Rc::downgrade(self);
        let _task = tasks::spawn_local(async move {
            let result = tasks::run(move || Ok(application.launch()?)).await;
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

pub fn view(
    initial: Scene,
    catalog: Rc<ApplicationCatalog>,
    session: Option<Rc<Session>>,
    themes: Rc<ThemeController>,
) -> Element {
    let appearance = signal(themes.current());
    ferroui_view::provide_context(appearance);
    let initial_status = apply_session_appearance(session.as_deref(), themes.current())
        .err()
        .map(|error| format!("Could not update window decorations. {error}"))
        .unwrap_or_default();
    if !initial_status.is_empty() {
        eprintln!("{initial_status}");
    }
    let state = Rc::new(Shell {
        scene: signal(initial),
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
        owner: Owner::current().expect("desktop view must be mounted in a FerroUI owner"),
        desktop_focus: NodeRef::new(),
    });

    if let Some(session) = &state.session {
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
                    state.scene.set(Scene::Applications);
                }
            }
        });
    }

    let current = state.scene;
    let clock = clock();
    let root_state = state.clone();
    let overlays = state.clone();
    grid()
        .name("DesktopShell")
        .clip_to_bounds(true)
        .on_key_down(move |event| match (event.key, event.key_modifiers) {
            (Key::Escape, _) if current.get_untracked() != Scene::Desktop => {
                root_state.dismiss();
                event.set_handled(true);
            }
            (Key::Space, KeyModifiers::CONTROL) | (Key::K, KeyModifiers::CONTROL) => {
                if current.get_untracked() == Scene::Applications {
                    root_state.dismiss()
                } else {
                    root_state.open(Scene::Applications)
                }
                event.set_handled(true);
            }
            (key, modifiers)
                if current.get_untracked() == Scene::Appearance
                    && modifiers.is_empty()
                    && (Key::D1 as i32..=Key::D9 as i32).contains(&(key as i32)) =>
            {
                let index = (key as i32 - Key::D1 as i32) as usize;
                if let Some(appearance) = Appearance::ALL.get(index).copied() {
                    root_state.select_appearance(appearance);
                    event.set_handled(true);
                }
            }
            (Key::OemComma, KeyModifiers::CONTROL) => {
                root_state.open(Scene::Appearance);
                event.set_handled(true);
            }
            (Key::W, KeyModifiers::CONTROL) => {
                root_state.open(Scene::Workspaces);
                event.set_handled(true);
            }
            (key, modifiers)
                if modifiers == (KeyModifiers::CONTROL | KeyModifiers::ALT)
                    && (Key::D1 as i32..=Key::D9 as i32).contains(&(key as i32)) =>
            {
                let index = (key as i32 - Key::D1 as i32) as usize;
                if let Some(session) = root_state.session.as_ref().filter(|_| {
                    root_state
                        .snapshot
                        .with_untracked(|s| index < s.workspace_names.len())
                }) {
                    session.switch_workspace(index);
                    event.set_handled(true);
                }
            }
            _ => {}
        })
        .child(dynamic(move || {
            desktop::surface(&state, clock, appearance.get())
        }))
        .child(dynamic(move || match (current.get(), appearance.get()) {
            (Scene::Desktop, _) => Element::Empty,
            (scene, _) => {
                let shell = overlays.clone();
                let content = match scene {
                    Scene::Applications => launcher::view(&shell),
                    Scene::Appearance => desktop::picker(&shell),
                    _ => workspaces::view(&shell),
                };
                grid()
                    .keyboard_navigation_tab_navigation(
                        ferroui_base::input::KeyboardNavigationMode::Cycle,
                    )
                    .child(
                        border()
                            .background("#65081420")
                            .on_pointer_pressed(move |e| {
                                shell.dismiss();
                                e.set_handled(true);
                            }),
                    )
                    .child(content)
                    .into()
            }
        }))
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
            icon_action("Close overview", Icon::Close, move || dismiss.dismiss())
                .node_ref(focus)
                .on_loaded(move |_| {
                    if scene.get_untracked() == Scene::Workspaces {
                        if let Some(control) = focus.get_untracked() {
                            control.focus();
                        }
                    }
                })
                .grid_column(1)
                .vertical_alignment(V::Top),
        )
}

fn feedback(status: Signal<String>) -> El<TextBlock> {
    shell_text("", 12.0, Ink::Accent)
        .text(status)
        .is_visible(move || status.with(|s| !s.is_empty()))
        .text_wrapping(ferroui_base::media::TextWrapping::Wrap)
}
