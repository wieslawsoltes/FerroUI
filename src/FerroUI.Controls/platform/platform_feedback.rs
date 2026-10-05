use super::{FeedbackAction, FeedbackType, IPlatformFeedback};
use crate::TopLevel;
use ferroui_base::input::InputElement;
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::{ferro_property, AttachedProperty, FerroProperty};

/// Defines the `FeedbackType` attached property: the kind of audible or
/// haptic feedback an input element gives for user interactions.
pub struct PlatformFeedback;

ferroui_base::ferro_static_type!(PlatformFeedback);

ferroui_base::ferro_properties! { impl PlatformFeedback {
    ferro_property!(
        /// Defines the `FeedbackType` attached property.
        pub fn feedback_type_property() -> AttachedProperty<FeedbackType> {
            FerroProperty::register_attached::<PlatformFeedback, InputElement, _>("FeedbackType", FeedbackType::None)
        }
    );
} }

impl PlatformFeedback {
    /// Sets the value of the attached `FeedbackType` property.
    pub fn set_feedback_type(control: &InputElement, feedback_type: FeedbackType) {
        control.set_value(Self::feedback_type_property(), feedback_type)
    }

    /// Gets the value of the attached `FeedbackType` property.
    pub fn get_feedback_type(control: &InputElement) -> FeedbackType {
        control.get_value(Self::feedback_type_property())
    }
}

/// Platform feedback on input elements.
pub trait PlatformFeedbackExtensions {
    /// Performs the specified [`FeedbackAction`] on this input element. The
    /// type of feedback to perform is defined by the `FeedbackType`
    /// attached property.
    ///
    /// Returns true if the platform performed the requested feedback; false
    /// otherwise.
    fn perform_feedback(&self, feedback_action: FeedbackAction) -> bool;
}

impl PlatformFeedbackExtensions for InputElement {
    fn perform_feedback(&self, feedback_action: FeedbackAction) -> bool {
        let feedback = PlatformFeedback::get_feedback_type(self);
        if feedback != FeedbackType::None {
            let platform_feedback = TopLevel::get_top_level(Some(self))
                .and_then(|top_level| top_level.platform_impl())
                .and_then(|platform_impl| {
                    let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
                    provider.try_get::<dyn IPlatformFeedback>()
                });
            if let Some(platform_feedback) = platform_feedback {
                return platform_feedback.perform(feedback_action, feedback);
            }
        }

        false
    }
}
