/// Performs audible or haptic feedback for user interactions.
pub trait IPlatformFeedback {
    /// Performs the feedback; returns whether it was performed.
    fn perform(&self, feedback: FeedbackAction, type_: FeedbackType) -> bool;
}

/// The kind of feedback an element gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FeedbackType {
    #[default]
    None = 0,
    Auto = 1,
    Sound = 2,
    Haptic = 3,
}

/// An interaction that feedback can be given for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FeedbackAction {
    key: &'static str,
}

impl FeedbackAction {
    /// The key that identifies the action. For platform backends.
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// The feedback of a click.
    pub const fn click() -> Self {
        Self { key: "Click" }
    }

    /// The feedback of holding.
    pub const fn hold() -> Self {
        Self { key: "Hold" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_are_identified_by_their_key() {
        assert_eq!(FeedbackAction::click().key(), "Click");
        assert_eq!(FeedbackAction::hold().key(), "Hold");
        assert_eq!(FeedbackAction::click(), FeedbackAction::click());
        assert_ne!(FeedbackAction::click(), FeedbackAction::hold());
    }
}
