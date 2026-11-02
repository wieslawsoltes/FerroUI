//! Native notification history and a passive top-level for banners. A banner has
//! its own native window so it stays visible above normal applications. No window is
//! allocated until the first notification arrives.
mod cards;
mod center;
mod x11_outline;

use super::appearance::{Appearance, Layout};
use super::chrome::*;
use super::{Scene, Shell};
use crate::services::notifications::{Notice, Service};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Ref};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{Control, Window, WindowBaseImpl, WindowDecorations};
use ferroui_view::prelude::*;
use ferroui_wayland::{
    layer_shell::{Anchors, LayerConfig, LayerWindow},
    wayland_top_level_factory::WaylandTopLevelFactory,
};
use ferroui_x11::xlib::{self, PropertyMode, XID};
use ferroui_x11::FerroX11Platform;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(super) struct Notifications {
    service: Rc<Service>,
    revision: Signal<u64>,
}

impl Notifications {
    pub(super) fn new(managed: bool) -> Self {
        let revision = signal(0_u64);
        let owner = Owner::current().expect("notification service belongs to the shell owner");
        let queued = Rc::new(Cell::new(false));
        let service = Service::new(managed, move || {
            // Coalesce bursts of CLI/desktop notifications. The D-Bus reply need
            // not wait for text shaping or for a notification window to map.
            if !queued.replace(true) {
                let queued = queued.clone();
                let owner = owner.clone();
                Dispatcher::ui_thread().post_local(
                    move || {
                        queued.set(false);
                        if owner.is_alive() {
                            revision.update(|v| *v = v.wrapping_add(1));
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
        });
        Self { service, revision }
    }
    pub(super) fn quiet(&self) -> bool {
        self.revision.get();
        self.service.quiet()
    }

    pub(super) fn toggle_quiet(&self) {
        self.service.toggle_quiet();
    }

    pub(super) fn count(&self) -> usize {
        self.revision.get();
        self.service.len()
    }
}

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    center::view(shell)
}

/// The compositor interprets layer margins within its usable area. X11 offers
/// the same area in physical pixels; the in-scene host reserves its own edges.
#[derive(Clone, Copy)]
struct BannerGeometry {
    layout: Layout,
    width: f64,
    height: f64,
}

impl BannerGeometry {
    const GAP: f64 = 12.;

    fn new(layout: Layout, actions: bool) -> Self {
        let (width, plain, actionable) = match layout {
            Layout::Taskbar => (364., 104., 140.),
            Layout::MenuBarDock => (354., 96., 132.),
            Layout::BottomNavigation => (414., 104., 144.),
        };
        Self {
            layout,
            width,
            height: if actions { actionable } else { plain },
        }
    }

    fn fit(mut self, width: f64, height: f64) -> Self {
        self.width = self.width.min((width - Self::GAP * 2.).max(1.));
        self.height = self.height.min((height - Self::GAP * 2.).max(1.));
        self
    }

    fn layer(self) -> LayerConfig {
        let gap = Self::GAP as i32;
        let mut config = LayerConfig::notification(self.width as u32, self.height as u32, gap, gap);
        match self.layout {
            Layout::Taskbar => {
                config.anchors = Anchors::BOTTOM | Anchors::RIGHT;
                config.margins = [0, gap, gap, 0];
            }
            Layout::MenuBarDock => {}
            Layout::BottomNavigation => {
                config.anchors = Anchors::TOP;
                config.margins = [gap, 0, 0, 0];
            }
        }
        config
    }

    fn position(self, work: ferroui_base::PixelRect, scale: f64) -> PixelPoint {
        let x = if self.layout == Layout::BottomNavigation {
            work.x + (work.width - (self.width * scale).round() as i32) / 2
        } else {
            work.right() - ((self.width + Self::GAP) * scale).round() as i32
        };
        let y = if self.layout == Layout::Taskbar {
            work.bottom() - ((self.height + Self::GAP) * scale).round() as i32
        } else {
            work.y + (Self::GAP * scale).round() as i32
        };
        PixelPoint::new(x, y)
    }
}

struct Banner {
    wayland: bool,
    output: Option<String>,
    window: Ref<Window>,
    content_owner: Option<Owner>,
}

impl Banner {
    fn new(
        wayland: bool,
        output: Option<String>,
        geometry: BannerGeometry,
    ) -> Result<Self, String> {
        let window = if wayland {
            let window = LayerWindow::new(
                "ferro-desktop-notification",
                output.as_deref(),
                geometry.layer(),
            )?;
            window
        } else {
            Window::new()
        };
        super::wayland::transparent(&window);
        window.set_title(Some("Ferro Desktop Notification".into()));
        window.set_window_decorations(WindowDecorations::None);
        window.set_show_in_taskbar(false);
        window.set_show_activated(false);
        window.set_can_resize(false);
        window.set_topmost(true);
        // The framework's X11 window type enum covers shells and ordinary clients.
        // Write EWMH's notification type before mapping this specialized top-level.
        if let (Some(platform), Some(handle)) = (
            FerroLocator::current().get_service::<FerroX11Platform>(),
            window.try_get_platform_handle(),
        ) {
            let info = platform.info();
            let atom =
                xlib::x_intern_atom(info.display(), "_NET_WM_WINDOW_TYPE_NOTIFICATION", false);
            xlib::x_change_property_longs(
                info.display(),
                handle.handle() as XID,
                info.atoms()._NET_WM_WINDOW_TYPE,
                info.atoms().ATOM,
                PropertyMode::Replace,
                &[atom],
            );
        }
        Ok(Self {
            wayland,
            output,
            window,
            content_owner: None,
        })
    }
    fn show(&mut self, shell: &Rc<Shell>, notice: &Notice) {
        let owner = Owner::new_root();
        let content = owner.with(|| {
            ferroui_view::provide_context(shell.appearance);
            if !self.wayland {
                x11_outline::attach(&self.window, f64::from(colors().radius));
            }
            shell_panel()
                .padding(4)
                .child(
                    scroll_viewer()
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(cards::card(shell, notice, true)),
                )
                .into_element()
                .mount()
        });
        self.window.set_content(Some(Control::boxed(content)));
        if let Some(old) = self.content_owner.replace(owner) {
            old.dispose();
        }
        let mut geometry = BannerGeometry::new(
            shell.appearance.get_untracked().layout(),
            !notice.actions.is_empty(),
        );
        if self.wayland {
            if let Ok(outputs) = LayerWindow::outputs() {
                if let Some(output) = outputs.iter().find(|o| o.name == self.output) {
                    geometry = geometry.fit(
                        output.logical_size.width.into(),
                        output.logical_size.height.into(),
                    );
                }
            }
            if let Err(e) = LayerWindow::configure(&self.window, geometry.layer()) {
                eprintln!("Wayland notification: {e}");
                return;
            }
        } else {
            let screens = self.window.screens();
            if let Some(screen) = screens
                .primary()
                .or_else(|| screens.all().into_iter().next())
            {
                let work = screen.working_area();
                let scale = screen.scaling().max(0.1);
                geometry = geometry.fit(
                    f64::from(work.width) / scale,
                    f64::from(work.height) / scale,
                );
                self.window.set_position(geometry.position(work, scale));
            }
        }
        self.window.set_width(geometry.width);
        self.window.set_height(geometry.height);
        WindowBaseImpl::show(&*self.window);
    }
}
impl Drop for Banner {
    fn drop(&mut self) {
        if let Some(owner) = self.content_owner.take() {
            owner.dispose();
        }
        self.window.close();
    }
}

/// In the FerroUI compositor the banner is part of the same output and can
/// appear above an external surface without creating another platform window.
pub(super) fn composed_banner(shell: Rc<Shell>) -> Element {
    dynamic(move || {
        shell.notifications.revision.get();
        let profile = shell.appearance.get();
        let (top, bottom) = profile.insets();
        if shell.scene.get() == Scene::Notifications {
            shell.notifications.service.hide_banner();
            return Element::Empty;
        }
        let Some(notice) = shell.notifications.service.banner() else {
            return Element::Empty;
        };
        let geometry = BannerGeometry::new(profile.layout(), !notice.actions.is_empty());
        let width = shell.metrics().width;
        let horizontal = if geometry.layout == Layout::BottomNavigation {
            H::Center
        } else {
            H::Right
        };
        let (vertical, margin) = if geometry.layout == Layout::Taskbar {
            (V::Bottom, (12., 0., 12., bottom + 12.))
        } else {
            (V::Top, (12., top + 12., 12., 0.))
        };
        shell_panel()
            .width(move || (width.get() - 24.).clamp(1., geometry.width))
            .max_height(geometry.height)
            .horizontal_alignment(horizontal)
            .vertical_alignment(vertical)
            .margin(margin)
            .child(
                scroll_viewer()
                    .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                    .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                    .child(cards::card(&shell, &notice, true)),
            )
            .into_element()
    })
    .into_element()
}

pub(super) fn attach(shell: &Rc<Shell>) {
    let banner = Rc::new(RefCell::new(None::<Banner>));
    let mounted = banner.clone();
    let last = RefCell::new(None::<(Notice, Appearance)>);
    let weak = Rc::downgrade(shell);
    effect(move || {
        let Some(shell) = weak.upgrade() else { return };
        shell.notifications.revision.get();
        let selected = shell.appearance.get();
        let active = if shell.scene.get() == Scene::Notifications {
            shell.notifications.service.hide_banner();
            None
        } else {
            shell.notifications.service.banner()
        };
        let desired = active.map(|n| (n, selected));
        if desired == *last.borrow() {
            return;
        }
        let mut mounted = mounted.borrow_mut();
        if let Some((notice, _)) = &desired {
            let wayland = FerroLocator::current()
                .get_service::<WaylandTopLevelFactory>()
                .is_some();
            let output = shell.session.as_ref().and_then(|s| s.focused_output());
            if mounted
                .as_ref()
                .is_some_and(|b| b.wayland && b.output != output)
            {
                mounted.take();
            }
            if mounted.is_none() {
                match Banner::new(
                    wayland,
                    output,
                    BannerGeometry::new(selected.layout(), !notice.actions.is_empty()),
                ) {
                    Ok(banner) => *mounted = Some(banner),
                    Err(e) => {
                        eprintln!("Desktop notification: {e}");
                        return;
                    }
                }
            }
            mounted.as_mut().unwrap().show(&shell, notice);
        } else if let Some(banner) = mounted.as_ref() {
            WindowBaseImpl::hide(&*banner.window);
        }
        last.replace(desired);
    });
    on_cleanup(move || {
        banner.take();
    });
}
