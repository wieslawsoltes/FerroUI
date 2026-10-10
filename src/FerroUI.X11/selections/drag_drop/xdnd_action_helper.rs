//! The actions of the protocol and the effects of the framework (the port
//! of `XdndActionHelper.cs`).

use crate::x11_atoms::X11Atoms;
use crate::xlib::Atom;
use ferroui_base::input::DragDropEffects;

/// The three action atoms of the protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct XdndActions {
    pub copy: Atom,
    pub move_: Atom,
    pub link: Atom,
}

impl XdndActions {
    pub fn new(atoms: &X11Atoms) -> Self {
        Self { copy: atoms.XdndActionCopy, move_: atoms.XdndActionMove, link: atoms.XdndActionLink }
    }

    pub fn action_to_effects(&self, action: Atom) -> DragDropEffects {
        if action == self.copy {
            return DragDropEffects::COPY;
        }
        if action == self.move_ {
            return DragDropEffects::MOVE;
        }
        if action == self.link {
            return DragDropEffects::LINK;
        }
        DragDropEffects::NONE
    }

    pub fn effects_to_action(&self, effects: DragDropEffects) -> Atom {
        if effects.contains(DragDropEffects::COPY) {
            return self.copy;
        }
        if effects.contains(DragDropEffects::MOVE) {
            return self.move_;
        }
        if effects.contains(DragDropEffects::LINK) {
            return self.link;
        }
        0
    }
}

pub fn action_to_effects(action: Atom, atoms: &X11Atoms) -> DragDropEffects {
    XdndActions::new(atoms).action_to_effects(action)
}

pub fn effects_to_action(effects: DragDropEffects, atoms: &X11Atoms) -> Atom {
    XdndActions::new(atoms).effects_to_action(effects)
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    const ACTIONS: XdndActions = XdndActions { copy: 101, move_: 102, link: 103 };

    #[test]
    fn an_action_is_one_effect() {
        assert_eq!(ACTIONS.action_to_effects(101), DragDropEffects::COPY);
        assert_eq!(ACTIONS.action_to_effects(102), DragDropEffects::MOVE);
        assert_eq!(ACTIONS.action_to_effects(103), DragDropEffects::LINK);
        // No action, and an action the framework has no effect for (ask, private).
        assert_eq!(ACTIONS.action_to_effects(0), DragDropEffects::NONE);
        assert_eq!(ACTIONS.action_to_effects(999), DragDropEffects::NONE);
    }

    #[test]
    fn of_several_effects_the_action_is_copy_before_move_before_link() {
        assert_eq!(ACTIONS.effects_to_action(DragDropEffects::NONE), 0);
        assert_eq!(ACTIONS.effects_to_action(DragDropEffects::COPY), 101);
        assert_eq!(ACTIONS.effects_to_action(DragDropEffects::MOVE), 102);
        assert_eq!(ACTIONS.effects_to_action(DragDropEffects::LINK), 103);
        assert_eq!(ACTIONS.effects_to_action(DragDropEffects::MOVE | DragDropEffects::LINK), 102);
        assert_eq!(
            ACTIONS.effects_to_action(DragDropEffects::COPY | DragDropEffects::MOVE | DragDropEffects::LINK),
            101
        );
    }
}
