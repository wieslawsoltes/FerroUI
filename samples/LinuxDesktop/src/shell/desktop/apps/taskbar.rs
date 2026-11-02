//! The taskbar keeps the anchor alive while the window manager changes focus.
//! Preview brushes live only for the open flyout and sample the original
//! client surfaces in the same compositor.
use super::*;
use crate::shell::desktop::popup_host::{opaque_background, PopupHost};
use crate::shell::workspaces::preview;
use ferroui_base::input::{
    Key, KeyModifiers, KeyboardNavigationHandler, KeyboardNavigationMode, NavigationDirection,
    PointerType,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_controls::{Control, Flyout, FlyoutPresenter, FlyoutShowMode, PlacementMode};
use std::{cell::RefCell, time::Duration};

pub(super) fn buttons(shell: &Rc<Shell>, items: Memo<Vec<Entry>>) -> Element {
    let shell = shell.clone();
    for_each(
        move || items.with(|all| all.iter().map(Entry::key).collect::<Vec<_>>()),
        |id| id.clone(),
        move |id| {
            let entry =
                memo(move |_| items.with(|all| all.iter().find(|e| e.key() == id).cloned()));
            group_button(&shell, entry)
        },
    )
}

fn group_button(shell: &Rc<Shell>, entry: Memo<Option<Entry>>) -> Element {
    let Some(initial) = entry.get_untracked() else {
        return Element::Empty;
    };
    let description = memo(move |_| {
        entry.with(|e| match e {
            Some(e) if e.windows.len() > 1 => format!("{} · {} windows", e.label, e.windows.len()),
            Some(e) => e.label.clone(),
            None => String::new(),
        })
    });
    let count = memo(move |_| entry.with(|e| e.as_ref().map_or(0, |e| e.windows.len())));
    let active = memo(move |_| {
        entry.with(|e| {
            e.as_ref()
                .is_some_and(|e| e.windows.iter().any(|w| w.active))
        })
    });
    let anchor = NodeRef::<Button>::new();
    let flyout = Flyout::new();
    flyout.set_placement(PlacementMode::Top);
    let host = PopupHost::current();
    host.configure(&flyout.popup(), 8.0);
    let composed = shell
        .session
        .as_ref()
        .is_some_and(|s| s.compositor().is_some());
    let width = shell.metrics().width;
    let showing = signal(false);
    let weak = flyout.downgrade();
    let surface = colors().surface;
    let opening = flyout.opening(move |_| {
        if let Some(presenter) = weak
            .upgrade()
            .and_then(|f| f.popup().child()?.downcast::<FlyoutPresenter>().ok())
        {
            presenter.set_focusable(false);
            presenter.set_max_width((width.get_untracked() - 24.0).max(160.0));
            presenter.set_padding(ferroui_base::Thickness::uniform(5.0));
            if host != PopupHost::InScene {
                presenter.set_background(Some(opaque_background(surface)));
            }
            if host == PopupHost::OpaqueNative {
                presenter.set_margin(Default::default());
            }
        }
        showing.set(true);
    });
    let closed = flyout.closed(move || showing.set(false));
    let state = shell.clone();
    let weak = flyout.downgrade();
    let content = grid()
        .keyboard_navigation_tab_navigation(KeyboardNavigationMode::Cycle)
        .child(show(
            move || showing.get(),
            move || {
                let tiles =
                    memo(move |_| composed && count.get() as f64 * 180.0 <= width.get() - 64.0);
                let session = state.session.clone();
                let snapshot = state.snapshot;
                let flyout = weak.clone();
                let windows = for_each(
                    move || {
                        entry.with(|e| {
                            e.iter()
                                .flat_map(|e| e.windows.iter().map(|w| w.id))
                                .collect::<Vec<_>>()
                        })
                    },
                    |id| *id as usize,
                    move |id| {
                        let first = entry.with(|e| {
                            e.as_ref()
                                .and_then(|e| e.windows.first())
                                .is_some_and(|w| w.id == id)
                        });
                        window_card(id, first, snapshot, session.clone(), tiles, flyout.clone())
                    },
                );
                stack_panel()
                    .spacing(3.0)
                    .orientation(move || {
                        if tiles.get() {
                            ferroui_base::layout::Orientation::Horizontal
                        } else {
                            ferroui_base::layout::Orientation::Vertical
                        }
                    })
                    .child(windows)
            },
        ))
        .into_element()
        .mount();
    flyout.set_content(Some(Control::boxed(content)));

    let hover = Rc::new(RefCell::new(None));
    let cancel = hover.clone();
    let dispose = flyout.clone();
    on_cleanup(move || {
        cancel.borrow_mut().take();
        dispose.set_is_open(false);
        drop((opening, closed));
    });
    let scene = shell.scene;
    let weak = flyout.downgrade();
    effect(move || {
        if (count.get() == 0 || scene.get() != Scene::Desktop) && showing.get() {
            if let Some(f) = weak.upgrade() {
                f.set_is_open(false);
            }
        }
    });

    let invoke = flyout.clone();
    let state = shell.clone();
    let pending = hover.clone();
    let weak = flyout.downgrade();
    let p = colors();
    button()
        .node_ref(anchor)
        .automation_properties_name(move || description.get())
        .tool_tip_tip(move || {
            (count.get() == 0 || !composed && count.get() == 1)
                .then(|| Rc::new(description.get()) as ferroui_base::BoxedValue)
        })
        .width(40.)
        .min_width(40.)
        .height(40.)
        .padding(0)
        .corner_radius(8)
        .border_thickness(0)
        .background(move || if active.get() { p.tile } else { "Transparent" })
        .on_pointer_entered(move |e| {
            // Independent native popup windows can grab the compositor seat.
            // Only open passively when the clients and taskbar share a scene.
            if !composed || e.pointer().type_() != PointerType::Mouse || count.get_untracked() == 0
            {
                return;
            }
            let flyout = weak.clone();
            pending.replace(Some(DispatcherTimer::run_once(
                move || {
                    let (Some(f), Some(b)) = (flyout.upgrade(), anchor.get_untracked()) else {
                        return;
                    };
                    if !b.is_pointer_over()
                        || f.is_open()
                        || scene.get_untracked() != Scene::Desktop
                    {
                        return;
                    }
                    f.set_show_mode(FlyoutShowMode::TransientWithDismissOnPointerMoveAway);
                    f.popup().set_is_light_dismiss_enabled(false);
                    f.show_at_with(&b, false);
                },
                Duration::from_millis(420),
                DispatcherPriority::INPUT,
            )));
        })
        .on_pointer_exited(move |_| {
            hover.borrow_mut().take();
        })
        .on_click(move |_| {
            let Some(current) = entry.get_untracked() else {
                return;
            };
            if invoke.is_open() {
                let sticky = invoke.show_mode() == FlyoutShowMode::Standard;
                invoke.set_is_open(false);
                if sticky {
                    return;
                }
            }
            if current.windows.len() <= 1 {
                current.invoke(&state, Layout::Taskbar);
            } else if let Some(b) = anchor.get_untracked() {
                invoke.set_show_mode(FlyoutShowMode::Standard);
                invoke.popup().set_is_light_dismiss_enabled(true);
                invoke.show_at_with(&b, false);
            }
        })
        .child(
            grid()
                .row_definitions("*,5")
                .child(app_art(shell, &initial))
                .child(
                    border()
                        .grid_row(1)
                        .is_visible(move || count.get() != 0)
                        .width(move || if active.get() { 16.0 } else { 6.0 })
                        .height(3)
                        .horizontal_alignment(H::Center)
                        .vertical_alignment(V::Center)
                        .corner_radius(2)
                        .background(move || if active.get() { p.accent } else { p.muted }),
                ),
        )
        .into()
}

fn window_card(
    id: u32,
    first: bool,
    snapshot: Signal<Snapshot>,
    session: Option<Rc<crate::session::Session>>,
    tiles: Memo<bool>,
    flyout: ferroui_base::WeakRef<Flyout>,
) -> Element {
    let title = memo(move |_| {
        snapshot.with(|s| {
            s.windows
                .iter()
                .find(|w| w.id == id)
                .map(|w| w.title.as_str())
                .filter(|t| !t.trim().is_empty())
                .unwrap_or("Untitled window")
                .to_owned()
        })
    });
    let can_close =
        memo(move |_| snapshot.with(|s| s.windows.iter().any(|w| w.id == id && w.can_close)));
    let source = session
        .as_ref()
        .and_then(|s| s.compositor())
        .and_then(|c| c.window_visual(id));
    let target = NodeRef::<Button>::new();
    let hovered = signal(false);
    let focused = signal(false);
    let focus_flyout = flyout.clone();
    let activate = session.clone();
    let close = session.clone();
    let select = button()
        .node_ref(target)
        .on_loaded(move |_| {
            if !first {
                return;
            }
            let Some(button) = target.get_untracked() else {
                return;
            };
            let weak = button.downgrade();
            let flyout = focus_flyout.clone();
            // A native popup can map after Flyout's synchronous focus pass.
            // Give a keyboard-opened group its first actual window button.
            Dispatcher::ui_thread().post_local(
                move || {
                    if flyout
                        .upgrade()
                        .is_some_and(|f| f.is_open() && f.show_mode() == FlyoutShowMode::Standard)
                    {
                        if let Some(button) = weak.upgrade() {
                            button.focus();
                        }
                    }
                },
                DispatcherPriority::INPUT,
            );
        })
        .automation_properties_name(move || title.get())
        .tool_tip_tip(move || title.get())
        .padding((7.0, 5.0))
        .horizontal_content_alignment(H::Stretch)
        .background("Transparent")
        .border_thickness(0)
        .on_got_focus(move |_| focused.set_if_changed(true))
        .on_lost_focus(move |_| focused.set_if_changed(false))
        .on_click(move |_| {
            if let Some(f) = flyout.upgrade() {
                f.set_is_open(false);
            }
            if let Some(session) = &activate {
                if session.activate(id) {
                    if let Some(host) = session.compositor() {
                        host.focus_client();
                    }
                }
            }
        })
        .on_key_down(move |e| {
            if e.key_modifiers != KeyModifiers::NONE {
                return;
            }
            if e.key == Key::Delete && can_close.get_untracked() {
                if let Some(s) = &close {
                    s.close(id);
                }
                e.set_handled(true);
                return;
            }
            let direction = match e.key {
                Key::Left | Key::Up => NavigationDirection::Previous,
                Key::Right | Key::Down => NavigationDirection::Next,
                _ => return,
            };
            if let Some(next) = target
                .get_untracked()
                .and_then(|b| KeyboardNavigationHandler::get_next(&b.upcast(), direction))
            {
                next.focus();
                e.set_handled(true);
            }
        })
        .child(
            stack_panel()
                .spacing(4.0)
                .child(
                    grid()
                        .column_definitions("18,*,24")
                        .height(22)
                        .child(glyph(Icon::Window, 14.0, colors().muted))
                        .child(
                            shell_text("", 12.0, Ink::Primary)
                                .grid_column(1)
                                .margin((4.0, 0.0))
                                .text(move || title.get()),
                        ),
                )
                .child(show(
                    move || tiles.get(),
                    move || {
                        source
                            .clone()
                            .map(|v| preview(v, 88.0))
                            .unwrap_or(Element::Empty)
                    },
                )),
        );
    let dismiss = button()
        .automation_properties_name(move || format!("Close {}", title.get()))
        .tool_tip_tip("Close window")
        .is_tab_stop(false)
        .background("Transparent")
        .border_thickness(0)
        .width(26)
        .height(25)
        .min_width(0)
        .min_height(0)
        .padding(0)
        .margin((0.0, 3.0, 4.0, 0.0))
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Top)
        .is_visible(move || can_close.get() && (hovered.get() || focused.get()))
        .child(glyph(Icon::Close, 12.0, colors().muted))
        .on_click(move |_| {
            if let Some(s) = &session {
                s.close(id);
            }
        });
    grid()
        .width(move || if tiles.get() { 176.0 } else { 238.0 })
        .on_pointer_entered(move |_| hovered.set_if_changed(true))
        .on_pointer_exited(move |_| hovered.set_if_changed(false))
        .child(select)
        .child(dismiss)
        .into()
}
