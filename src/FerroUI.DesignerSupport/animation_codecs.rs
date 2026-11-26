use ferroui_designer_animation::{AnimationCodec, AnimationCodecRegistry};
use std::cell::RefCell;

ferroui_base::ferro_thread_local! {
    static CODECS: RefCell<AnimationCodecRegistry> = RefCell::new(AnimationCodecRegistry::default());
}

/// Register a project animation codec on the application's design UI thread.
/// Call this during application preview setup, before the first document loads.
pub fn register_preview_animation_codec(codec: Box<dyn AnimationCodec>) {
    CODECS.with(|codecs| codecs.borrow_mut().register(codec));
}

pub(crate) fn with_codecs<T>(read: impl FnOnce(&AnimationCodecRegistry) -> T) -> T {
    CODECS.with(|codecs| read(&codecs.borrow()))
}
