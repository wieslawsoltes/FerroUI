//! The feedback of the platform for an action of the user: a click sound,
//! a vibration.

use ferroui_controls::platform::{FeedbackAction, FeedbackType, IPlatformFeedback};

/// `AudioManager.FX_KEY_CLICK`.
pub(crate) const SOUND_EFFECT_KEY_CLICK: i32 = 0;
/// `HapticFeedbackConstants.CONTEXT_CLICK`.
pub(crate) const FEEDBACK_CONTEXT_CLICK: i32 = 6;
/// `HapticFeedbackConstants.LONG_PRESS`.
pub(crate) const FEEDBACK_LONG_PRESS: i32 = 0;

/// The view the feedback is performed on, and the audio manager of its
/// context.
pub(crate) trait IFeedbackView {
    /// `(view.Context?.GetSystemService(Context.AudioService) as AudioManager)?.PlaySoundEffect(effect)`.
    fn play_sound_effect(&self, effect: i32);
    /// `view.PerformHapticFeedback(feedback)`.
    fn perform_haptic_feedback(&self, feedback: i32);
}

pub(crate) struct AndroidPlatformFeedback {
    view: Box<dyn IFeedbackView>,
}

impl AndroidPlatformFeedback {
    pub fn new(view: Box<dyn IFeedbackView>) -> Self {
        Self { view }
    }
}

impl IPlatformFeedback for AndroidPlatformFeedback {
    fn perform(&self, feedback: FeedbackAction, type_: FeedbackType) -> bool {
        let play_sound = type_ != FeedbackType::Haptic;
        let vibrate = type_ != FeedbackType::Sound;

        if feedback == FeedbackAction::click() {
            if play_sound {
                self.view.play_sound_effect(SOUND_EFFECT_KEY_CLICK);

                return true;
            }
            if vibrate {
                self.view.perform_haptic_feedback(FEEDBACK_CONTEXT_CLICK);

                return true;
            }
        } else if feedback == FeedbackAction::hold() && vibrate {
            self.view.perform_haptic_feedback(FEEDBACK_LONG_PRESS);

            return true;
        }

        false
    }
}

#[cfg(target_os = "android")]
pub(crate) use imp::ViewFeedback;

#[cfg(target_os = "android")]
mod imp {
    use super::IFeedbackView;
    use crate::interop::java::{call_boolean, call_object, call_void, JavaObject, JavaValue};

    /// A view of the system.
    pub(crate) struct ViewFeedback {
        view: JavaObject,
    }

    impl ViewFeedback {
        pub fn new(view: JavaObject) -> Self {
            Self { view }
        }
    }

    impl IFeedbackView for ViewFeedback {
        fn play_sound_effect(&self, effect: i32) {
            let Some(context) = call_object(&self.view, "getContext", "()Landroid/content/Context;", &[]) else {
                return;
            };
            let audio_manager = call_object(
                &context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JavaValue::String("audio")],
            );
            if let Some(audio_manager) = audio_manager {
                call_void(&audio_manager, "playSoundEffect", "(I)V", &[JavaValue::Int(effect)]);
            }
        }

        fn perform_haptic_feedback(&self, feedback: i32) {
            call_boolean(&self.view, "performHapticFeedback", "(I)Z", &[JavaValue::Int(feedback)]);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the feedback.
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct View(Rc<RefCell<Vec<String>>>);

    impl IFeedbackView for View {
        fn play_sound_effect(&self, effect: i32) {
            self.0.borrow_mut().push(format!("sound {effect}"));
        }

        fn perform_haptic_feedback(&self, feedback: i32) {
            self.0.borrow_mut().push(format!("haptic {feedback}"));
        }
    }

    #[test]
    fn a_click_sounds_unless_it_is_haptic_and_a_hold_only_vibrates() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let feedback = AndroidPlatformFeedback::new(Box::new(View(calls.clone())));

        assert!(feedback.perform(FeedbackAction::click(), FeedbackType::Auto));
        assert!(feedback.perform(FeedbackAction::click(), FeedbackType::Sound));
        assert!(feedback.perform(FeedbackAction::click(), FeedbackType::Haptic));
        assert!(feedback.perform(FeedbackAction::hold(), FeedbackType::Auto));
        assert!(feedback.perform(FeedbackAction::hold(), FeedbackType::Haptic));
        assert!(!feedback.perform(FeedbackAction::hold(), FeedbackType::Sound));

        assert_eq!(*calls.borrow(), ["sound 0", "sound 0", "haptic 6", "haptic 0", "haptic 0"]);
    }
}
