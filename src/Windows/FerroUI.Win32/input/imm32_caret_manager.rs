//! The caret of the system that follows the text cursor (the port of
//! `Input/Imm32CaretManager.cs`, extended for assistive technology, #430).
//!
//! The framework draws its own caret. A caret of the system, owned by the
//! window that has the input method, shaped and placed as the text cursor and
//! moved with it, tells the tools that follow the caret where the text cursor
//! is: screen readers (Narrator) and magnifiers follow the WinEvents of the
//! caret object (`OBJID_CARET`: `EVENT_OBJECT_CREATE`, `EVENT_OBJECT_SHOW`,
//! and `EVENT_OBJECT_LOCATIONCHANGE`, which `SetCaretPos` raises when the
//! position changes) and read its rectangle (`GetGUIThreadInfo`), the text
//! cursor indicator of the system draws around it, and input methods that do
//! not take the candidate form place their windows by it. As a native edit
//! control does, the caret is shown, with a blank bitmap (see
//! `interop::unmanaged_methods::create_caret`), so the system draws nothing
//! over the frame of the framework.
//!
//! What is asked of the system is a pure function of the caret there is and
//! the one wanted ([`plan_caret`]), tested on every host; the manager keeps
//! the two and makes the calls.

use super::imm32_input_method::ImmSystem;
use std::cell::Cell;

/// A rectangle of the caret in pixels of the client area of the window that
/// owns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CaretRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl CaretRect {
    /// The rectangle between two corners in pixels, in either order (a
    /// mirrored window maps the left edge to the right), at least one pixel
    /// wide and high: the text cursor of the framework has no width.
    pub fn from_corners(x1: i32, y1: i32, x2: i32, y2: i32) -> CaretRect {
        CaretRect {
            x: x1.min(x2),
            y: y1.min(y2),
            width: x1.abs_diff(x2).clamp(1, i32::MAX as u32) as i32,
            height: y1.abs_diff(y2).clamp(1, i32::MAX as u32) as i32,
        }
    }
}

/// The caret of the system: the window that owns it and its rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SystemCaret {
    pub hwnd: isize,
    pub rect: CaretRect,
}

/// A call of the system the manager makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaretCall {
    /// `CreateCaret` with a blank bitmap of the size: it replaces the caret
    /// of the thread, whichever window owned it.
    Create { hwnd: isize, width: i32, height: i32 },
    /// `SetCaretPos`, in the client coordinates of the owner.
    Move { x: i32, y: i32 },
    /// `ShowCaret` of a caret just created, once it is in place.
    Show { hwnd: isize },
    /// `DestroyCaret`.
    Destroy,
}

/// The calls that turn the caret there is into the one wanted: none when
/// they are the same, a move when only the position differs, a new caret
/// (created, moved, then shown, so it never appears at the origin) when the
/// window or the size differs, and its destruction when none is wanted.
pub(crate) fn plan_caret(current: Option<SystemCaret>, wanted: Option<SystemCaret>) -> Vec<CaretCall> {
    let Some(wanted) = wanted else {
        return if current.is_some() { vec![CaretCall::Destroy] } else { Vec::new() };
    };
    let rect = wanted.rect;
    match current {
        Some(current)
            if current.hwnd == wanted.hwnd
                && current.rect.width == rect.width
                && current.rect.height == rect.height =>
        {
            if current.rect.x == rect.x && current.rect.y == rect.y {
                Vec::new()
            } else {
                vec![CaretCall::Move { x: rect.x, y: rect.y }]
            }
        }
        _ => vec![
            CaretCall::Create { hwnd: wanted.hwnd, width: rect.width, height: rect.height },
            CaretCall::Move { x: rect.x, y: rect.y },
            CaretCall::Show { hwnd: wanted.hwnd },
        ],
    }
}

/// Held through a shared reference: a call of the system may run a window
/// procedure that comes back to the input method, and a caret destroyed
/// meanwhile is not taken for created when the outer creation returns.
#[derive(Default)]
pub(crate) struct Imm32CaretManager {
    /// The window the input method serves a client in, which wants the
    /// caret; 0 for none.
    owner: Cell<isize>,
    /// The last rectangle of the text cursor, with the window it is in. It
    /// is kept while the caret is destroyed, so that a caret made again for
    /// the same window and client (a change of the keyboard language) is
    /// where the cursor is, and forgotten when the client changes.
    cursor: Cell<Option<SystemCaret>>,
    /// The caret of the system, as made.
    caret: Cell<Option<SystemCaret>>,
    generation: Cell<u64>,
}

impl Imm32CaretManager {
    /// The input method serves a client in a window: the caret is that
    /// window's. It is made once the rectangle of the text cursor in that
    /// window is known.
    pub fn try_create(&self, system: &dyn ImmSystem, hwnd: isize) {
        if hwnd == 0 {
            return;
        }
        self.owner.set(hwnd);
        self.apply(system);
    }

    /// The text cursor is at a rectangle of a window (the one the input
    /// method serves now, which takes the caret). Nothing while no caret is
    /// wanted; no call while the caret is already there.
    pub fn try_move(&self, system: &dyn ImmSystem, hwnd: isize, rect: CaretRect) {
        if hwnd == 0 || self.owner.get() == 0 {
            return;
        }
        self.owner.set(hwnd);
        self.cursor.set(Some(SystemCaret { hwnd, rect }));
        self.apply(system);
    }

    /// No client: no caret.
    pub fn try_destroy(&self, system: &dyn ImmSystem) {
        self.owner.set(0);
        self.apply(system);
    }

    /// Another client: the rectangle of the old one is not where the new
    /// one's caret goes. Asks nothing of the system; the next move does.
    pub fn forget_cursor(&self) {
        self.cursor.set(None);
    }

    /// The caret of the system as made, for the tests.
    #[cfg(test)]
    pub fn caret(&self) -> Option<SystemCaret> {
        self.caret.get()
    }

    fn wanted(&self) -> Option<SystemCaret> {
        let owner = self.owner.get();
        self.cursor.get().filter(|cursor| owner != 0 && cursor.hwnd == owner)
    }

    fn apply(&self, system: &dyn ImmSystem) {
        let wanted = self.wanted();
        let calls = plan_caret(self.caret.get(), wanted);
        if calls.is_empty() {
            return;
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        // Published before the calls: what a window procedure run from one of
        // them asks of the manager is planned against the caret wanted here.
        self.caret.set(wanted);
        for call in calls {
            match call {
                CaretCall::Create { hwnd, width, height } => {
                    let created = system.create_caret(hwnd, width, height);
                    if self.generation.get() != generation {
                        return;
                    }
                    if !created {
                        // Asked for again by the next creation, not by every move.
                        self.caret.set(None);
                        self.owner.set(0);
                        return;
                    }
                }
                CaretCall::Move { x, y } => system.set_caret_pos(x, y),
                CaretCall::Show { hwnd } => system.show_caret(hwnd),
                CaretCall::Destroy => system.destroy_caret(),
            }
            if self.generation.get() != generation {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the input method.
    use super::super::imm32_input_method::tests::{Call, FakeSystem};
    use super::*;

    const RECT: CaretRect = CaretRect { x: 30, y: 40, width: 1, height: 16 };

    fn caret(hwnd: isize, x: i32, y: i32, width: i32, height: i32) -> Option<SystemCaret> {
        Some(SystemCaret { hwnd, rect: CaretRect { x, y, width, height } })
    }

    #[test]
    fn the_rectangle_of_a_caret_is_between_its_corners_and_at_least_a_pixel() {
        assert_eq!(CaretRect { x: 10, y: 20, width: 1, height: 16 }, CaretRect::from_corners(10, 20, 10, 36));
        assert_eq!(CaretRect { x: 10, y: 20, width: 2, height: 16 }, CaretRect::from_corners(12, 20, 10, 36));
        assert_eq!(CaretRect { x: -5, y: -3, width: 1, height: 1 }, CaretRect::from_corners(-5, -3, -5, -3));
    }

    #[test]
    fn the_plan_makes_a_caret_moves_it_and_destroys_it_only_on_a_change() {
        // None wanted, none there.
        assert!(plan_caret(None, None).is_empty());
        // A new caret is created, placed, then shown.
        assert_eq!(
            vec![
                CaretCall::Create { hwnd: 7, width: 1, height: 16 },
                CaretCall::Move { x: 30, y: 40 },
                CaretCall::Show { hwnd: 7 }
            ],
            plan_caret(None, caret(7, 30, 40, 1, 16))
        );
        // The same caret: nothing.
        assert!(plan_caret(caret(7, 30, 40, 1, 16), caret(7, 30, 40, 1, 16)).is_empty());
        // Another position: a move only.
        assert_eq!(vec![CaretCall::Move { x: 31, y: 40 }], plan_caret(caret(7, 30, 40, 1, 16), caret(7, 31, 40, 1, 16)));
        // Another height (a larger font) or another window: a new caret.
        assert_eq!(
            vec![
                CaretCall::Create { hwnd: 7, width: 1, height: 20 },
                CaretCall::Move { x: 30, y: 40 },
                CaretCall::Show { hwnd: 7 }
            ],
            plan_caret(caret(7, 30, 40, 1, 16), caret(7, 30, 40, 1, 20))
        );
        assert_eq!(
            vec![
                CaretCall::Create { hwnd: 8, width: 1, height: 16 },
                CaretCall::Move { x: 30, y: 40 },
                CaretCall::Show { hwnd: 8 }
            ],
            plan_caret(caret(7, 30, 40, 1, 16), caret(8, 30, 40, 1, 16))
        );
        // None wanted: the caret there is destroyed.
        assert_eq!(vec![CaretCall::Destroy], plan_caret(caret(7, 30, 40, 1, 16), None));
    }

    #[test]
    fn the_caret_is_made_where_the_cursor_is_moved_on_a_change_and_destroyed_once() {
        let system = FakeSystem::new();
        let manager = Imm32CaretManager::default();

        // No caret wanted yet: nothing to move or destroy.
        manager.try_move(&*system, 7, RECT);
        manager.try_destroy(&*system);
        assert!(system.take_calls().is_empty());

        // Wanted, but where the cursor is is not known yet: nothing.
        manager.try_create(&*system, 7);
        assert!(system.take_calls().is_empty());

        manager.try_move(&*system, 7, RECT);
        manager.try_move(&*system, 7, RECT);
        manager.try_create(&*system, 7);
        manager.try_move(&*system, 7, CaretRect { x: 31, ..RECT });
        manager.try_destroy(&*system);
        manager.try_destroy(&*system);

        assert_eq!(
            vec![
                Call::CreateCaret(7, 1, 16),
                Call::SetCaretPos(30, 40),
                Call::ShowCaret(7),
                Call::SetCaretPos(31, 40),
                Call::DestroyCaret
            ],
            system.take_calls()
        );
        assert_eq!(None, manager.caret());
    }

    #[test]
    fn a_caret_made_again_for_the_same_window_is_where_the_cursor_was_unless_the_client_changed() {
        let system = FakeSystem::new();
        let manager = Imm32CaretManager::default();
        manager.try_create(&*system, 7);
        manager.try_move(&*system, 7, RECT);
        system.take_calls();

        // A change of the language destroys and makes the caret again.
        manager.try_destroy(&*system);
        manager.try_create(&*system, 7);
        assert_eq!(
            vec![Call::DestroyCaret, Call::CreateCaret(7, 1, 16), Call::SetCaretPos(30, 40), Call::ShowCaret(7)],
            system.take_calls()
        );

        // Another client: no caret until its cursor is known.
        manager.forget_cursor();
        manager.try_destroy(&*system);
        manager.try_create(&*system, 7);
        assert_eq!(vec![Call::DestroyCaret], system.take_calls());
        manager.try_move(&*system, 7, CaretRect { y: 60, ..RECT });
        assert_eq!(vec![Call::CreateCaret(7, 1, 16), Call::SetCaretPos(30, 60), Call::ShowCaret(7)], system.take_calls());
    }

    #[test]
    fn the_caret_goes_to_the_window_that_takes_the_input_method() {
        let system = FakeSystem::new();
        let manager = Imm32CaretManager::default();
        manager.try_create(&*system, 7);
        manager.try_move(&*system, 7, RECT);
        system.take_calls();

        // The rectangle of the old window is not used for the new one.
        manager.try_create(&*system, 8);
        assert_eq!(vec![Call::DestroyCaret], system.take_calls());
        manager.try_move(&*system, 8, RECT);
        assert_eq!(vec![Call::CreateCaret(8, 1, 16), Call::SetCaretPos(30, 40), Call::ShowCaret(8)], system.take_calls());

        // A cursor of another window takes the caret there at once (the
        // creation replaces the caret of the thread).
        manager.try_move(&*system, 9, RECT);
        assert_eq!(vec![Call::CreateCaret(9, 1, 16), Call::SetCaretPos(30, 40), Call::ShowCaret(9)], system.take_calls());
        assert_eq!(Some(SystemCaret { hwnd: 9, rect: RECT }), manager.caret());
    }

    #[test]
    fn a_caret_the_system_refuses_is_asked_for_again_by_the_next_creation() {
        let system = FakeSystem::new();
        system.refuse_caret.set(true);
        let manager = Imm32CaretManager::default();

        manager.try_create(&*system, 7);
        manager.try_move(&*system, 7, RECT);
        // Not asked for again by every move.
        manager.try_move(&*system, 7, CaretRect { x: 1, ..RECT });
        system.refuse_caret.set(false);
        manager.try_create(&*system, 7);
        manager.try_move(&*system, 7, CaretRect { x: 3, y: 4, ..RECT });

        assert_eq!(
            vec![
                Call::CreateCaret(7, 1, 16),
                Call::CreateCaret(7, 1, 16),
                Call::SetCaretPos(30, 40),
                Call::ShowCaret(7),
                Call::SetCaretPos(3, 4)
            ],
            system.take_calls()
        );
    }

    #[test]
    fn a_caret_destroyed_from_inside_its_creation_is_not_moved_or_shown() {
        let system = FakeSystem::new();
        let manager = std::rc::Rc::new(Imm32CaretManager::default());
        manager.try_create(&*system, 7);
        let inner = manager.clone();
        let inner_system = system.clone();
        *system.next_hook.borrow_mut() = Some((
            |call| matches!(call, Call::CreateCaret(..)),
            Box::new(move || inner.try_destroy(&*inner_system)),
        ));

        manager.try_move(&*system, 7, RECT);

        assert_eq!(vec![Call::CreateCaret(7, 1, 16), Call::DestroyCaret], system.take_calls());
        assert_eq!(None, manager.caret());
        manager.try_destroy(&*system);
        assert!(system.take_calls().is_empty());
    }
}
