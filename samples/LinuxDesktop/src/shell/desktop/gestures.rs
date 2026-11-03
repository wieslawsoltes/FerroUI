//! Android home and bottom-edge gestures. Only drags begun on the relevant
//! shell surface participate. Native windows keep their own pointer streams.
use super::*;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

fn swipe_target() -> El<Border> {
    border().background("Transparent").with(|area| {
        let swipe = SwipeGestureRecognizer::new();
        swipe.set_can_vertically_swipe(true);
        swipe.set_is_mouse_enabled(true);
        swipe.set_threshold(40.0);
        area.gesture_recognizers().add(swipe);
    })
}

pub(super) fn home_wallpaper(state: &Rc<Shell>) -> El<Border> {
    let shell = state.clone();
    let previous = Cell::new(None);
    swipe_target().on_swipe_gesture(move |event| {
        let dy = event.delta().y;
        if dy != 0.0 && previous.get() != Some(event.id()) {
            previous.set(Some(event.id()));
            // Positive deltas move toward the top edge.
            shell.open(if dy > 0.0 {
                Scene::Applications
            } else {
                Scene::Notifications
            });
            event.set_handled(true);
        }
    })
}

fn go_home(shell: &Shell) {
    if let Some(session) = &shell.session {
        let ids: Vec<_> = shell.snapshot.with_untracked(|s| {
            s.windows
                .iter()
                .filter(|w| {
                    w.workspace.is_none_or(|i| i == s.current_workspace)
                        && !w.minimized
                        && w.can_minimize
                })
                .map(|w| w.id)
                .collect()
        });
        for id in ids {
            session.minimize(id);
        }
    }
}

pub(in super::super) fn drawer_handle(state: &Rc<Shell>) -> El<Border> {
    let shell = state.clone();
    swipe_target().on_swipe_gesture(move |event| {
        if event.delta().y < 0.0 && shell.scene.get_untracked() == Scene::Applications {
            shell.cancel();
            event.set_handled(true);
        }
    })
}

#[derive(Clone, Copy)]
struct Drag {
    pointer: i32,
    origin: ferroui_base::Point,
    last: ferroui_base::Point,
    moved: Instant,
    opened_recents: bool,
}

impl Drag {
    fn lifted(self, position: ferroui_base::Point) -> bool {
        let lift = self.origin.y - position.y;
        lift >= 40.0 && lift > (position.x - self.origin.x).abs()
    }
}

pub(super) fn navigation_handle(state: &Rc<Shell>) -> El<Border> {
    const HOLD: Duration = Duration::from_millis(300);
    // A quick release and a cancelled capture have different consequences:
    // only release may minimize client windows. Track the initiating pointer
    // through the entire grab, including motion outside a layer-shell panel.
    let target = NodeRef::<Border>::new();
    let drag = Rc::new(Cell::new(None::<Drag>));
    let timer = Rc::new(RefCell::new(None::<Subscription>));
    let press = drag.clone();
    let movement = drag.clone();
    let release = drag.clone();
    let disposed_drag = drag.clone();
    let disposed_timer = timer.clone();
    on_cleanup(move || {
        disposed_drag.set(None);
        disposed_timer.borrow_mut().take();
    });
    let start_timer = timer.clone();
    let stop_timer = timer.clone();
    let shell = Rc::downgrade(state);
    let released_shell = state.clone();
    border()
        .background("Transparent")
        .node_ref(target)
        .on_pointer_pressed(move |e| {
            if press.get().is_some() || !e.get_current_point(None).properties.is_left_button_pressed
            {
                return;
            }
            let Some(control) = target.get_untracked() else {
                return;
            };
            let origin = e.get_position(None);
            e.pointer()
                .capture(Some(&control.upcast::<ferroui_base::input::InputElement>()));
            press.set(Some(Drag {
                pointer: e.pointer().id(),
                origin,
                last: origin,
                moved: Instant::now(),
                opened_recents: false,
            }));
            let motion = press.clone();
            let shell = shell.clone();
            start_timer.replace(Some(DispatcherTimer::run(
                move || {
                    let Some(mut drag) = motion.get() else {
                        return false;
                    };
                    if !drag.lifted(drag.last) || drag.moved.elapsed() < HOLD {
                        return true;
                    }
                    drag.opened_recents = true;
                    motion.set(Some(drag));
                    if let Some(shell) = shell.upgrade() {
                        shell.open(Scene::Workspaces);
                    }
                    false
                },
                Duration::from_millis(50),
                DispatcherPriority::INPUT,
            )));
            e.set_handled(true);
        })
        .on_pointer_moved(move |e| {
            let Some(mut drag) = movement.get().filter(|d| d.pointer == e.pointer().id()) else {
                return;
            };
            let point = e.get_position(None);
            if (point.x - drag.last.x)
                .abs()
                .max((point.y - drag.last.y).abs())
                >= 1.0
            {
                drag.moved = Instant::now();
                drag.last = point;
                movement.set(Some(drag));
            }
            e.set_handled(true);
        })
        .on_pointer_released(move |e| {
            let Some(drag) = release.get().filter(|d| d.pointer == e.pointer().id()) else {
                return;
            };
            let lifted = drag.lifted(e.get_position(None));
            release.set(None);
            stop_timer.borrow_mut().take();
            e.pointer().capture(None);
            if lifted
                && !drag.opened_recents
                && released_shell.scene.get_untracked() == Scene::Desktop
            {
                go_home(&released_shell);
            }
            e.set_handled(true);
        })
        .on_pointer_capture_lost(move |e| {
            if drag.get().is_some_and(|d| d.pointer == e.pointer().id()) {
                drag.set(None);
                timer.borrow_mut().take();
            }
        })
}
