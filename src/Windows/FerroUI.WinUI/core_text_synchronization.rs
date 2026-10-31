//! State acknowledged by CoreText is distinct from the latest client state.
//! A failed NotifyTextChanged must retain the server's old length for retries.
use ferroui_base::input::text_input::{TextInputMethodState, TextSelection};

#[derive(Clone, Debug)]
pub(super) struct Notification {
    generation: u64,
    revision: u64,
    state: TextInputMethodState,
    pub old_length: i32,
    pub new_length: i32,
    pub selection: TextSelection,
    pub text_changed: bool,
}

pub(super) struct CoreTextSynchronization {
    generation: u64,
    revision: u64,
    acknowledged_revision: u64,
    current: TextInputMethodState,
    acknowledged: Option<TextInputMethodState>,
}

impl Default for CoreTextSynchronization {
    fn default() -> Self {
        Self {
            generation: 0,
            revision: 0,
            acknowledged_revision: 0,
            current: TextInputMethodState {
                text: String::new(),
                selection: TextSelection::default(),
            },
            acknowledged: None,
        }
    }
}

impl CoreTextSynchronization {
    pub fn replace_client(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.revision = 0;
        self.acknowledged_revision = 0;
        self.acknowledged = None;
    }

    pub fn update(&mut self, state: TextInputMethodState) {
        if self.current != state {
            self.current = state;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn current(&self) -> &TextInputMethodState {
        &self.current
    }

    pub fn pending(&self) -> Option<Notification> {
        if self.acknowledged.as_ref() == Some(&self.current) {
            return None;
        }
        let text_changed = self
            .acknowledged
            .as_ref()
            .is_none_or(|old| old.text != self.current.text);
        Some(Notification {
            generation: self.generation,
            revision: self.revision,
            state: self.current.clone(),
            old_length: self
                .acknowledged
                .as_ref()
                .map_or(i32::MAX, |old| old.text.encode_utf16().count() as i32),
            new_length: self.current.text.encode_utf16().count() as i32,
            selection: self.current.selection,
            text_changed,
        })
    }

    /// Call only after a successful native notification or a successful native
    /// edit acknowledgement. Reentrant newer work and replacement clients win.
    pub fn acknowledge(&mut self, notification: &Notification) {
        if notification.generation == self.generation
            && notification.revision >= self.acknowledged_revision
        {
            self.acknowledged = Some(notification.state.clone());
            self.acknowledged_revision = notification.revision;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(text: &str, caret: i32) -> TextInputMethodState {
        TextInputMethodState {
            text: text.into(),
            selection: TextSelection::new(caret, caret),
        }
    }

    #[test]
    fn failed_text_notification_retries_from_last_acknowledged_length() {
        let mut sync = CoreTextSynchronization::default();
        sync.update(state("a😀b", 4));
        sync.acknowledge(&sync.pending().unwrap());
        sync.update(state("z", 1));
        let failed = sync.pending().unwrap();
        assert_eq!((failed.old_length, failed.new_length), (4, 1));
        // Native HRESULT failed: no acknowledgement is applied.
        sync.update(state("hello", 5));
        let retry = sync.pending().unwrap();
        assert_eq!((retry.old_length, retry.new_length), (4, 5));
        sync.acknowledge(&retry);
        assert!(sync.pending().is_none());
    }

    #[test]
    fn successful_old_notification_does_not_hide_reentrant_new_state() {
        let mut sync = CoreTextSynchronization::default();
        sync.update(state("old", 3));
        let initial = sync.pending().unwrap();
        sync.update(state("new state", 9));
        sync.acknowledge(&initial);
        let next = sync.pending().unwrap();
        assert_eq!((next.old_length, next.new_length), (3, 9));
        sync.acknowledge(&next);
        sync.acknowledge(&initial);
        assert!(sync.pending().is_none());
    }

    #[test]
    fn replacement_client_rejects_late_acknowledgement() {
        let mut sync = CoreTextSynchronization::default();
        sync.update(state("previous", 8));
        let previous = sync.pending().unwrap();
        sync.replace_client();
        sync.update(state("next", 4));
        sync.acknowledge(&previous);
        assert_eq!(sync.pending().unwrap().old_length, i32::MAX);
    }

    #[test]
    fn failed_selection_notification_remains_pending_and_text_failure_takes_precedence() {
        let mut sync = CoreTextSynchronization::default();
        sync.update(state("abc", 0));
        sync.acknowledge(&sync.pending().unwrap());
        sync.update(state("abc", 2));
        assert!(!sync.pending().unwrap().text_changed);
        assert!(sync.pending().is_some());
        sync.update(state("abcX", 4));
        assert!(sync.pending().unwrap().text_changed);
        assert_eq!(sync.pending().unwrap().old_length, 3);
    }
}
