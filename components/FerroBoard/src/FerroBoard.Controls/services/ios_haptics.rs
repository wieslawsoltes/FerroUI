//! Native iOS equivalent of the pinned Uno vibration behavior, without managed services.
use super::HapticDevice;

const SHORT_FEEDBACK_SOUND: u32 = 1520;
const SYSTEM_VIBRATION_SOUND: u32 = 0x0000_0fff;

/// iOS uses the same two system-sound requests as the pinned Uno implementation.
/// Durations below 200 ms select its short pulse; all others select system vibration.
/// A true result means the void native API was invoked, not that hardware feedback
/// occurred. iOS can intentionally do nothing on hardware without vibration support.
pub struct IosHaptics {
    play: fn(u32),
}
impl IosHaptics {
    #[cfg(target_os = "ios")]
    pub fn new() -> Self {
        Self {
            play: play_system_sound,
        }
    }
}
#[cfg(target_os = "ios")]
impl Default for IosHaptics {
    fn default() -> Self {
        Self::new()
    }
}
impl HapticDevice for IosHaptics {
    fn vibrate(&self, duration_ms: i32) -> bool {
        let sound = if duration_ms < 200 {
            SHORT_FEEDBACK_SOUND
        } else {
            SYSTEM_VIBRATION_SOUND
        };
        (self.play)(sound);
        true
    }
}
#[cfg(target_os = "ios")]
fn play_system_sound(sound: u32) {
    #[link(name = "AudioToolbox", kind = "framework")]
    unsafe extern "C" {
        fn AudioServicesPlaySystemSound(sound: u32);
    }
    // SAFETY: public AudioToolbox ABI, a SystemSoundID (uint32_t) passed by value.
    // The IDs match the pinned source. This void call does not report hardware success.
    unsafe { AudioServicesPlaySystemSound(sound) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::KeyboardHapticService;
    use ferroboard_core::settings::FeedbackKind;
    use std::{cell::RefCell, rc::Rc};
    thread_local! { static PLAYED: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) }; }
    fn record(sound: u32) {
        PLAYED.with(|sounds| sounds.borrow_mut().push(sound));
    }

    #[test]
    fn threshold_and_all_feedback_kinds_match_the_pinned_ios_device() {
        PLAYED.with(|sounds| sounds.borrow_mut().clear());
        let service = KeyboardHapticService::new(Rc::new(IosHaptics { play: record }));
        let cases = [
            (FeedbackKind::KeyPress, -1, 1520),
            (FeedbackKind::KeyPress, 0, 1520),
            (FeedbackKind::KeyPress, 12, 1520),
            (FeedbackKind::KeyPress, 199, 1520),
            (FeedbackKind::KeyPress, 200, 4095),
            (FeedbackKind::KeyPress, 201, 4095),
            (FeedbackKind::LongPress, 99, 1520),
            (FeedbackKind::LongPress, 100, 4095),
            (FeedbackKind::LongPress, i32::MAX, 1520),
            (FeedbackKind::LongPress, i32::MIN, 1520),
            (FeedbackKind::KeyPressDelete, 200, 4095),
            (FeedbackKind::KeyPressSpace, 200, 4095),
            (FeedbackKind::KeyPressEnter, 200, 4095),
            (FeedbackKind::KeyPressFunction, 200, 4095),
            (FeedbackKind::KeyRepeat, 200, 4095),
            (FeedbackKind::GestureStep, 200, 4095),
            (FeedbackKind(999), 200, 4095),
        ];
        for (kind, duration, _) in cases {
            assert!(service.perform(kind, duration));
        }
        PLAYED.with(|sounds| assert_eq!(*sounds.borrow(), cases.map(|(_, _, sound)| sound)));
    }
}
