//! Native notification history and a passive top-level for banners. A banner has
//! its own X window so it stays visible above normal applications. No window is
//! allocated until the first notification arrives.
use super::appearance::Appearance;
use super::chrome::*;
use super::{popup_header, Scene, Shell};
use crate::services::notifications::{Notice, Service};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Ref};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{Control, Window, WindowBaseImpl, WindowDecorations};
use ferroui_view::prelude::*;
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
    pub(super) fn count(&self) -> usize {
        self.revision.get();
        self.service.len()
    }
}

fn card(notice: &Notice, service: &Rc<Service>, banner: bool) -> Element {
    let id = notice.id;
    let close = service.clone();
    let app = if notice.app.is_empty() {
        "Application"
    } else {
        &notice.app
    };
    let mut details = stack_panel()
        .spacing(4.0)
        .child(
            grid()
                .column_definitions("*,Auto")
                .child(shell_text(
                    app,
                    11.0,
                    if notice.critical {
                        Ink::Accent
                    } else {
                        Ink::Secondary
                    },
                ))
                .child(
                    icon_action("Dismiss notification", Icon::Close, move || {
                        close.dismiss(id)
                    })
                    .width(28)
                    .height(28)
                    .grid_column(1),
                ),
        )
        .child(
            shell_text(&notice.title, 15.0, Ink::Primary)
                .font_weight(FontWeight::SemiBold)
                .text_wrapping(TextWrapping::Wrap)
                .max_lines(if banner { 2 } else { 4 }),
        );
    if !notice.body.is_empty() {
        details = details.child(
            shell_text(&notice.body, 13.0, Ink::Primary)
                .text_wrapping(TextWrapping::Wrap)
                .max_lines(if banner { 2 } else { 12 }),
        );
    }
    if !notice.actions.is_empty() {
        let mut buttons = wrap_panel();
        for (key, title) in &notice.actions {
            let key = key.clone();
            let label = if title.is_empty() {
                if key == "default" {
                    "Open"
                } else {
                    &key
                }
            } else {
                title
            }
            .to_owned();
            let service = service.clone();
            buttons = buttons.child(
                action(&label, move || service.invoke(id, &key))
                    .margin((0.0, 3.0, 6.0, 0.0))
                    .padding((10.0, 3.0))
                    .child(shell_text(&label, 12.0, Ink::Primary)),
            );
        }
        details = details.child(buttons);
    }
    border()
        .padding((12.0, 8.0))
        .background(colors().tile)
        .corner_radius(colors().radius)
        .child(details)
        .into()
}

pub(super) fn view(shell: &Rc<Shell>) -> Element {
    const PAGE: usize = 8;
    let page = signal(0usize);
    let notices = shell.notifications.service.clone();
    let revision = shell.notifications.revision;
    let settings = notices.clone();
    let clear = notices.clone();
    let rows = dynamic(move || -> Element {
        revision.get();
        let count = notices.len();
        if count == 0 {
            let status = notices.status();
            return stack_panel()
                .spacing(6)
                .margin((0.0, 15.0))
                .child(shell_text("You're all caught up", 19.0, Ink::Primary))
                .child(
                    shell_text(
                        if status.is_empty() {
                            "New notifications will appear here."
                        } else {
                            &status
                        },
                        13.0,
                        Ink::Secondary,
                    )
                    .text_wrapping(TextWrapping::Wrap),
                )
                .into();
        }
        let last = (count - 1) / PAGE;
        let index = page.get().min(last);
        let mut list = stack_panel().spacing(8);
        for item in notices.page(index * PAGE, PAGE) {
            list = list.child(card(&item, &notices, false));
        }
        if last > 0 {
            list = list.child(
                row(5.0)
                    .child(
                        action("Previous notifications", move || {
                            page.set(index.saturating_sub(1))
                        })
                        .is_enabled(index != 0)
                        .child(shell_text("Previous", 12.0, Ink::Primary)),
                    )
                    .child(shell_text(
                        &format!("{} / {}", index + 1, last + 1),
                        12.0,
                        Ink::Secondary,
                    ))
                    .child(
                        action("More notifications", move || page.set(index + 1))
                            .is_enabled(index < last)
                            .child(shell_text("Next", 12.0, Ink::Primary)),
                    ),
            );
        }
        list.into()
    });
    let quiet = shell.notifications.service.clone();
    let quiet_button = NodeRef::<ferroui_controls::Button>::new();
    let count = shell.notifications.service.clone();
    super::desktop::popup_frame()
        .max_width(470)
        .max_height(640)
        .padding(18)
        .child(
            grid()
                .row_definitions("Auto,Auto,*")
                .child(popup_header("Notifications", "ACTIVITY", shell))
                .child(
                    row(8.0)
                        .grid_row(1)
                        .margin((0.0, 8.0))
                        .child(
                            action("Toggle Do Not Disturb", move || settings.toggle_quiet())
                                .node_ref(quiet_button)
                                .on_loaded(move |_| {
                                    if let Some(button) = quiet_button.get_untracked() {
                                        button.focus();
                                    }
                                })
                                .child(shell_text("", 12.0, Ink::Primary).text(move || {
                                    revision.get();
                                    if quiet.quiet() {
                                        "Do Not Disturb · On"
                                    } else {
                                        "Do Not Disturb · Off"
                                    }
                                    .to_owned()
                                })),
                        )
                        .child(
                            action("Clear all notifications", move || clear.clear())
                                .is_enabled(move || {
                                    revision.get();
                                    count.len() > 0
                                })
                                .child(shell_text("Clear all", 12.0, Ink::Primary)),
                        ),
                )
                .child(
                    scroll_viewer()
                        .grid_row(2)
                        .max_height(420)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(rows),
                ),
        )
        .into()
}

struct Banner {
    window: Ref<Window>,
    content_owner: Option<Owner>,
}

impl Banner {
    fn new() -> Self {
        let window = Window::new();
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
        Self {
            window,
            content_owner: None,
        }
    }
    fn show(&mut self, shell: &Rc<Shell>, notice: &Notice) {
        let owner = Owner::new_root();
        let content = owner.with(|| {
            ferroui_view::provide_context(shell.appearance);
            shell_panel()
                .padding(4)
                .child(
                    scroll_viewer()
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(card(notice, &shell.notifications.service, true)),
                )
                .into_element()
                .mount()
        });
        self.window.set_content(Some(Control::boxed(content)));
        if let Some(old) = self.content_owner.replace(owner) {
            old.dispose();
        }
        let mut width: f64 = 356.0;
        let height = if notice.actions.is_empty() {
            110.0
        } else {
            148.0
        };
        let screens = self.window.screens();
        if let Some(screen) = screens
            .primary()
            .or_else(|| screens.all().into_iter().next())
        {
            let work = screen.working_area();
            let scale = screen.scaling().max(0.1);
            width = width.min((f64::from(work.width) / scale - 24.0).max(1.0));
            self.window.set_position(PixelPoint::new(
                work.right() - ((width + 12.0) * scale).round() as i32,
                work.y + (12.0 * scale).round() as i32,
            ));
        }
        self.window.set_width(width);
        self.window.set_height(height);
        WindowBaseImpl::show(&*self.window);
    }
}
impl Drop for Banner {
    fn drop(&mut self) {
        self.window.close();
        if let Some(owner) = self.content_owner.take() {
            owner.dispose();
        }
    }
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
            mounted.get_or_insert_with(Banner::new).show(&shell, notice);
        } else if let Some(banner) = mounted.as_ref() {
            WindowBaseImpl::hide(&*banner.window);
        }
        last.replace(desired);
    });
    on_cleanup(move || {
        banner.take();
    });
}
