//! Pure navigation state; event handlers only apply committed state to controls.

use crate::catalog;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavigationHistory {
    current: Option<&'static str>,
    back: Vec<&'static str>,
    forward: Vec<&'static str>,
}

impl NavigationHistory {
    pub fn current(&self) -> Option<&'static str> {
        self.current
    }
    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Rejects unknown IDs without mutating history. Re-selecting the current page
    /// is a no-op and preserves its forward history.
    pub fn navigate(&mut self, id: &str) -> bool {
        let Some(entry) = catalog::find(id) else {
            return false;
        };
        if self.current == Some(entry.id) {
            return false;
        }
        if let Some(current) = self.current.replace(entry.id) {
            self.back.push(current);
        }
        self.forward.clear();
        true
    }

    pub fn go_back(&mut self) -> bool {
        let Some(previous) = self.back.pop() else {
            return false;
        };
        if let Some(current) = self.current.replace(previous) {
            self.forward.push(current);
        }
        true
    }

    pub fn go_forward(&mut self) -> bool {
        let Some(next) = self.forward.pop() else {
            return false;
        };
        if let Some(current) = self.current.replace(next) {
            self.back.push(current);
        }
        true
    }
}
