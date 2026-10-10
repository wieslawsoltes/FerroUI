//! The feedback of the platform: the sound of a click, and the taps of
//! the haptic engine.

use ferroui_controls::platform::{FeedbackAction, FeedbackType};

/// The strength of a tap of the haptic engine (`UIImpactFeedbackStyle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum ImpactStyle {
    Light = 0,
    Medium = 1,
}

/// The identifier of the system sound of a click.
pub const DEFAULT_SOUND: u32 = 1104;

/// What a feedback of a type consists of: whether the sound of a click is
/// played, and the tap of the haptic engine, if one.
pub fn feedback_plan(feedback: FeedbackAction, type_: FeedbackType) -> (bool, Option<ImpactStyle>) {
    let play_sound = matches!(type_, FeedbackType::Sound | FeedbackType::Auto);
    let vibrate = matches!(type_, FeedbackType::Haptic | FeedbackType::Auto);

    let sound = feedback == FeedbackAction::click() && play_sound;
    let impact = if vibrate { feedback_to_impact_style(feedback) } else { None };

    (sound, impact)
}

fn feedback_to_impact_style(feedback: FeedbackAction) -> Option<ImpactStyle> {
    if feedback == FeedbackAction::click() {
        Some(ImpactStyle::Light)
    } else if feedback == FeedbackAction::hold() {
        Some(ImpactStyle::Medium)
    } else {
        None
    }
}

#[cfg(target_os = "ios")]
pub use uikit::IosPlatformFeedback;

#[cfg(target_os = "ios")]
mod uikit {
    use super::{feedback_plan, ImpactStyle, DEFAULT_SOUND};
    use crate::ferro_view::FerroView;
    use ferroui_controls::platform::{FeedbackAction, FeedbackType, IPlatformFeedback};
    use objc2::rc::Weak;
    use objc2::{available, MainThreadMarker, MainThreadOnly};
    use objc2_ui_kit::{UIImpactFeedbackGenerator, UIImpactFeedbackStyle};

    const _: () = {
        assert!(ImpactStyle::Light as isize == UIImpactFeedbackStyle::Light.0);
        assert!(ImpactStyle::Medium as isize == UIImpactFeedbackStyle::Medium.0);
    };

    #[link(name = "AudioToolbox", kind = "framework")]
    extern "C" {
        /// `void AudioServicesPlaySystemSound(SystemSoundID inSystemSoundID)`
        /// of `AudioToolbox/AudioServices.h`.
        fn AudioServicesPlaySystemSound(in_system_sound_id: u32);
    }

    /// The feedback of a view.
    pub struct IosPlatformFeedback {
        view: Weak<FerroView>,
    }

    impl IosPlatformFeedback {
        /// Creates the feedback of `view`.
        pub fn new(view: Weak<FerroView>) -> Self {
            Self { view }
        }
    }

    impl IPlatformFeedback for IosPlatformFeedback {
        fn perform(&self, feedback: FeedbackAction, type_: FeedbackType) -> bool {
            let mut performed_feedback = false;
            let (play_sound, impact) = feedback_plan(feedback, type_);

            if play_sound {
                // SAFETY: the function takes an identifier by value; an
                // identifier the system does not know plays nothing.
                unsafe { AudioServicesPlaySystemSound(DEFAULT_SOUND) };
                performed_feedback = true;
            }

            if let Some(impact) = impact {
                let style = UIImpactFeedbackStyle(impact as isize);
                let view = self.view.load();
                let generator = match &view {
                    // A generator of a view exists since iOS 17.5.
                    Some(view) if available!(ios = 17.5) => {
                        Some(UIImpactFeedbackGenerator::feedbackGeneratorWithStyle_forView(style, view))
                    }
                    // Deprecated since iOS 17.5 for the generator of a
                    // view, which the branch above takes there.
                    #[allow(deprecated)]
                    Some(view) => Some(UIImpactFeedbackGenerator::initWithStyle(view.mtm().alloc(), style)),
                    // The view is gone; the feedback of a top-level is
                    // asked for on the main thread.
                    #[allow(deprecated)]
                    None => MainThreadMarker::new()
                        .map(|mtm| UIImpactFeedbackGenerator::initWithStyle(mtm.alloc(), style)),
                };
                if let Some(generator) = generator {
                    generator.impactOccurred();
                    performed_feedback = true;
                }
            }

            performed_feedback
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn a_click_sounds_and_taps_lightly() {
        assert_eq!((true, Some(ImpactStyle::Light)), feedback_plan(FeedbackAction::click(), FeedbackType::Auto));
        assert_eq!((true, None), feedback_plan(FeedbackAction::click(), FeedbackType::Sound));
        assert_eq!((false, Some(ImpactStyle::Light)), feedback_plan(FeedbackAction::click(), FeedbackType::Haptic));
        assert_eq!((false, None), feedback_plan(FeedbackAction::click(), FeedbackType::None));
    }

    #[test]
    fn holding_taps_and_has_no_sound() {
        assert_eq!((false, Some(ImpactStyle::Medium)), feedback_plan(FeedbackAction::hold(), FeedbackType::Auto));
        assert_eq!((false, None), feedback_plan(FeedbackAction::hold(), FeedbackType::Sound));
        assert_eq!((false, Some(ImpactStyle::Medium)), feedback_plan(FeedbackAction::hold(), FeedbackType::Haptic));
        assert_eq!((false, None), feedback_plan(FeedbackAction::hold(), FeedbackType::None));
    }

    #[test]
    fn the_sound_is_the_one_of_the_reference() {
        assert_eq!(1104, DEFAULT_SOUND);
    }
}
