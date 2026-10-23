//! The caret of the system the input method follows (the port of
//! `Input/Imm32CaretManager.cs`).
//!
//! The framework draws its own caret; a caret of the system, two pixels
//! wide and never shown, tells the input method and the tools that follow
//! the caret (the emoji panel, the magnifier) where the text cursor is.

use super::imm32_input_method::ImmSystem;
use std::cell::Cell;

#[derive(Default)]
pub(crate) struct Imm32CaretManager {
    is_caret_created: Cell<bool>,
    generation: Cell<u64>,
}

impl Imm32CaretManager {
    pub fn try_create(&self, system: &dyn ImmSystem, hwnd: isize) {
        if hwnd != 0 && !self.is_caret_created.replace(true) {
            let generation = self.generation.get().wrapping_add(1);
            self.generation.set(generation);
            let created = system.create_caret(hwnd, 2, 2);
            if self.generation.get() == generation {
                self.is_caret_created.set(created);
            }
        }
    }

    pub fn try_move(&self, system: &dyn ImmSystem, x: i32, y: i32) {
        if self.is_caret_created.get() {
            system.set_caret_pos(x, y);
        }
    }

    pub fn try_destroy(&self, system: &dyn ImmSystem) {
        self.generation.set(self.generation.get().wrapping_add(1));
        if self.is_caret_created.replace(false) {
            system.destroy_caret();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the input method.
    use super::super::imm32_input_method::tests::{Call, FakeSystem};
    use super::*;

    #[test]
    fn the_caret_is_created_once_moved_while_it_exists_and_destroyed_once() {
        let system = FakeSystem::new();
        let caret = Imm32CaretManager::default();

        // No caret yet: nothing to move or destroy.
        caret.try_move(&*system, 1, 2);
        caret.try_destroy(&*system);
        assert!(system.take_calls().is_empty());

        caret.try_create(&*system, 7);
        caret.try_create(&*system, 7);
        caret.try_move(&*system, 30, 40);
        caret.try_destroy(&*system);
        caret.try_destroy(&*system);

        assert_eq!(
            vec![
                Call::CreateCaret(7, 2, 2),
                Call::SetCaretPos(30, 40),
                Call::DestroyCaret
            ],
            system.take_calls()
        );
    }

    #[test]
    fn a_caret_the_system_refuses_is_asked_for_again() {
        let system = FakeSystem::new();
        system.refuse_caret.set(true);
        let caret = Imm32CaretManager::default();

        caret.try_create(&*system, 7);
        caret.try_move(&*system, 1, 1);
        system.refuse_caret.set(false);
        caret.try_create(&*system, 7);
        caret.try_move(&*system, 3, 4);

        assert_eq!(
            vec![
                Call::CreateCaret(7, 2, 2),
                Call::CreateCaret(7, 2, 2),
                Call::SetCaretPos(3, 4)
            ],
            system.take_calls()
        );
    }
}
