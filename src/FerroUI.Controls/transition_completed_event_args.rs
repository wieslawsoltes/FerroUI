use crate::TransitioningContentControl;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{ferro_routed_event_args, BoxedValue};

/// Represents the event arguments for the `TransitionCompleted` event of the
/// [`TransitioningContentControl`].
#[derive(Clone)]
pub struct TransitionCompletedEventArgs {
    base: RoutedEventArgs,
    from: Option<BoxedValue>,
    to: Option<BoxedValue>,
    has_run_to_completion: bool,
}

ferro_routed_event_args!(TransitionCompletedEventArgs: RoutedEventArgs);

impl TransitionCompletedEventArgs {
    /// Creates the args: `from` is the content that was transitioned from,
    /// `to` the content that was transitioned to, and `has_run_to_completion`
    /// tells whether the transition ran to completion.
    pub fn new(from: Option<BoxedValue>, to: Option<BoxedValue>, has_run_to_completion: bool) -> Self {
        Self {
            base: RoutedEventArgs::with_event(TransitioningContentControl::transition_completed_event()),
            from,
            to,
            has_run_to_completion,
        }
    }

    /// The content that was transitioned from.
    #[inline]
    pub fn from(&self) -> Option<&BoxedValue> {
        self.from.as_ref()
    }

    /// The content that was transitioned to.
    #[inline]
    pub fn to(&self) -> Option<&BoxedValue> {
        self.to.as_ref()
    }

    /// Whether the transition ran to completion. If false, the transition
    /// may have completed instantly or been cancelled.
    #[inline]
    pub fn has_run_to_completion(&self) -> bool {
        self.has_run_to_completion
    }
}
