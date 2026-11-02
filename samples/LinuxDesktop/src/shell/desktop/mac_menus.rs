//! The menu bar belongs to the desktop. Window commands operate through the
//! session backend; an application's own menus remain inside its client.
use super::*;
use crate::session::{ClientWindow, Session, SnapSide};
use ferroui_controls::{MenuItem, MenuItemToggleType};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuHost {
    InScene,
    Native,
    OpaqueNative,
}

impl MenuHost {
    pub(super) fn current() -> Self {
        use ferroui_base::{FerroLocator, LocatorExtensions};
        if use_context::<super::super::ManagedDesktopWindow>().is_some() {
            let platform = FerroLocator::current().get_service::<ferroui_x11::FerroX11Platform>();
            if platform.is_some_and(|x| !x.globals().is_composition_enabled()) {
                Self::OpaqueNative
            } else {
                Self::Native
            }
        } else if use_context::<DesktopBackdrop>().is_some() {
            Self::InScene
        } else {
            // A Wayland panel is a narrow layer surface. Its native popup
            // can extend beyond the edge reservation.
            Self::Native
        }
    }
}

pub(super) fn opaque_background(color: &str) -> Rc<dyn ferroui_base::media::IBrush> {
    use ferroui_base::media::{immutable::ImmutableSolidColorBrush, Color};
    Rc::new(ImmutableSolidColorBrush::new(
        Color::parse(color).expect("desktop surface color"),
    ))
}

fn submenu() -> El<MenuItem> {
    let host = MenuHost::current();
    let surface = colors().surface;
    menu_item().on_template_applied(move |event| {
        let Some(popup) = event
            .name_scope()
            .find_as::<ferroui_controls::primitives::Popup>("PART_Popup")
        else {
            return;
        };
        popup.set_should_use_overlay_layer(host == MenuHost::InScene);
        if host == MenuHost::OpaqueNative {
            super::super::x11_outline::track_popup(&popup, 10.0);
            // Without an X compositor, transparent pixels around the menu
            // become the native window's clear color. Remove the shadow
            // inset and paint a continuous surface to the popup edges.
            if let Some(frame) = popup.child().and_then(|c| c.downcast::<Border>().ok()) {
                frame.set_margin(Default::default());
                frame.set_box_shadow(Default::default());
                frame.set_corner_radius(Default::default());
                frame.set_background(Some(opaque_background(surface)));
            }
        }
    })
}

fn scene_item(shell: &Rc<Shell>, label: &str, scene: Scene) -> El<MenuItem> {
    let shell = shell.clone();
    menu_item()
        .header(label)
        .on_click(move |_| shell.open(scene))
}

fn window_command(
    shell: &Rc<Shell>,
    front: Memo<Option<ClientWindow>>,
    label: &'static str,
    allowed: fn(&ClientWindow) -> bool,
    run: fn(&Session, u32) -> bool,
) -> El<MenuItem> {
    let session = shell.session.clone();
    menu_item()
        .header(label)
        .is_enabled(move || front.with(|w| w.as_ref().is_some_and(allowed)))
        .on_click(move |_| {
            if let (Some(session), Some(w)) = (&session, front.get_untracked().filter(allowed)) {
                run(session, w.id);
            }
        })
}

pub(super) fn bar(shell: &Rc<Shell>) -> Element {
    let snapshot = shell.snapshot;
    let width = shell.metrics().width;
    // A native menu popup can temporarily receive focus from the client. Keep
    // that client's identity for the menu while it is still on this desktop.
    let previous = std::cell::Cell::new(None);
    let front = memo(move |_| {
        snapshot.with(|model| {
            let visible = |w: &&ClientWindow| {
                !w.minimized && w.workspace.is_none_or(|i| i == model.current_workspace)
            };
            let w = model
                .windows
                .iter()
                .filter(visible)
                .find(|w| w.active)
                .or_else(|| {
                    model
                        .windows
                        .iter()
                        .filter(visible)
                        .find(|w| Some(w.id) == previous.get())
                });
            previous.set(w.map(|w| w.id));
            w.cloned()
        })
    });
    let catalog = shell.catalog.clone();
    let name = memo(move |_| {
        front.with(|w| {
            let Some(w) = w else {
                return "Desktop".to_owned();
            };
            catalog
                .window_index(&w.app_id)
                .map(|index| catalog.applications()[index].name.as_str())
                .or_else(|| w.app_id.rsplit('.').next().filter(|s| !s.is_empty()))
                .unwrap_or(&w.title)
                .to_owned()
        })
    });
    let system = submenu()
        .header(glyph(Icon::Mark, 16.0, colors().bar_ink))
        .automation_properties_name("Ferro Desktop menu")
        .child(scene_item(shell, "System Settings…", Scene::Appearance))
        .child(scene_item(shell, "Control Center", Scene::QuickSettings))
        .child(separator())
        .child(scene_item(shell, "Spotlight…", Scene::Applications))
        .child(scene_item(shell, "Mission Control", Scene::Workspaces));
    let hide_session = shell.session.clone();
    let app = submenu()
        .header(
            shell_text("", 13.0, colors().bar_ink)
                .font_weight(FontWeight::Bold)
                .max_width(move || if width.get() < 480.0 { 82.0 } else { 145.0 })
                .text(move || name.get()),
        )
        .automation_properties_name(move || format!("{} menu", name.get()))
        .child(scene_item(
            shell,
            "Search Applications…",
            Scene::Applications,
        ))
        .child(scene_item(shell, "Mission Control", Scene::Workspaces))
        .child(separator())
        .child(
            menu_item()
                .header(move || format!("Hide {}", name.get()))
                .is_enabled(move || front.with(|w| w.as_ref().is_some_and(|w| w.can_minimize)))
                .on_click(move |_| {
                    if let (Some(session), Some(w)) = (&hide_session, front.get_untracked()) {
                        let group = snapshot.with_untracked(|s| {
                            s.windows
                                .iter()
                                .filter(|c| {
                                    c.can_minimize
                                        && !c.minimized
                                        && c.workspace.is_none_or(|i| i == s.current_workspace)
                                })
                                .filter(|c| {
                                    c.id == w.id || (!w.app_id.is_empty() && w.app_id == c.app_id)
                                })
                                .map(|c| c.id)
                                .collect::<Vec<_>>()
                        });
                        for id in group {
                            session.minimize(id);
                        }
                    }
                }),
        );
    let tile = submenu()
        .header("Move & Resize")
        .is_enabled(move || front.with(|w| w.as_ref().is_some_and(|w| w.can_snap && !w.fullscreen)))
        .child(window_command(
            shell,
            front,
            "Left Half",
            |w| w.can_snap && !w.fullscreen,
            |s, id| s.snap(id, SnapSide::Left),
        ))
        .child(window_command(
            shell,
            front,
            "Right Half",
            |w| w.can_snap && !w.fullscreen,
            |s, id| s.snap(id, SnapSide::Right),
        ));
    let session = shell.session.clone();
    let windows = submenu()
        .header("Window")
        .is_visible(move || width.get() >= 470.0)
        .child(window_command(
            shell,
            front,
            "Minimize",
            |w| w.can_minimize,
            Session::minimize,
        ))
        .child(
            window_command(
                shell,
                front,
                "Zoom",
                |w| w.can_maximize && !w.fullscreen,
                Session::toggle_maximized,
            )
            .header(move || {
                if front.with(|w| w.as_ref().is_some_and(|w| w.maximized)) {
                    "Restore Size"
                } else {
                    "Zoom"
                }
            }),
        )
        .child(tile)
        .child(
            window_command(
                shell,
                front,
                "Enter Full Screen",
                |w| w.can_fullscreen,
                Session::toggle_fullscreen,
            )
            .header(move || {
                if front.with(|w| w.as_ref().is_some_and(|w| w.fullscreen)) {
                    "Exit Full Screen"
                } else {
                    "Enter Full Screen"
                }
            }),
        )
        .child(separator())
        .child(scene_item(shell, "Mission Control", Scene::Workspaces))
        .child(for_each(
            move || {
                snapshot.with(|s| {
                    s.windows
                        .iter()
                        .filter(|w| w.workspace.is_none_or(|i| i == s.current_workspace))
                        .map(|w| (w.id, w.title.clone()))
                        .collect::<Vec<_>>()
                })
            },
            |(id, _)| *id as usize,
            move |(id, title)| {
                let session = session.clone();
                menu_item()
                    .header(if title.is_empty() {
                        "Untitled Window".to_owned()
                    } else {
                        title
                    })
                    .toggle_type(MenuItemToggleType::Radio)
                    .is_checked(move || front.with(|w| w.as_ref().is_some_and(|w| w.id == id)))
                    .on_click(move |_| {
                        if let Some(s) = &session {
                            s.activate(id);
                        }
                    })
            },
        ));
    menu()
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Center)
        .automation_properties_name("Desktop menu bar")
        .child(system)
        .child(app)
        .child(windows)
        .into()
}
