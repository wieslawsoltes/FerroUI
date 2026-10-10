//! The input pane: the state of the keyboard of the system and the
//! rectangle it covers, from the notifications the keyboard posts before
//! it is shown and before it is hidden.

use ferroui_base::animation::easings::{IEasing, LinearEasing, SineEaseIn, SineEaseInOut, SineEaseOut};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Rect;
use ferroui_controls::platform::{IInputPane, InputPaneBase, InputPaneState, InputPaneStateEventArgs};
use std::rc::Rc;
use std::time::Duration;

/// The curve options of `UIViewAnimationOptions`.
pub mod view_animation_options {
    pub const CURVE_EASE_IN_OUT: usize = 0 << 16;
    pub const CURVE_EASE_IN: usize = 1 << 16;
    pub const CURVE_EASE_OUT: usize = 2 << 16;
    pub const CURVE_LINEAR: usize = 3 << 16;
}

/// The easing of the animation of the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardEasing {
    Linear,
    SineEaseIn,
    SineEaseOut,
    SineEaseInOut,
}

/// The easing of the animation curve of a keyboard notification. The
/// reference reads the curve of the notification as animation options and
/// asks for each of the curve options as a flag; "ease in out" is the
/// option without bits, which every value has, so it is the answer for
/// every curve a notification carries (the curves are numbers under the
/// bits of the options).
pub fn easing_of_curve(curve: usize) -> KeyboardEasing {
    use view_animation_options::{CURVE_EASE_IN, CURVE_EASE_OUT, CURVE_LINEAR};
    if curve & CURVE_LINEAR == CURVE_LINEAR {
        KeyboardEasing::Linear
    } else if curve & CURVE_EASE_IN == CURVE_EASE_IN {
        KeyboardEasing::SineEaseIn
    } else if curve & CURVE_EASE_OUT == CURVE_EASE_OUT {
        KeyboardEasing::SineEaseOut
    } else {
        KeyboardEasing::SineEaseInOut
    }
}

fn easing(kind: KeyboardEasing) -> Rc<dyn IEasing> {
    match kind {
        KeyboardEasing::Linear => Rc::new(LinearEasing::new()),
        KeyboardEasing::SineEaseIn => Rc::new(SineEaseIn::new()),
        KeyboardEasing::SineEaseOut => Rc::new(SineEaseOut::new()),
        KeyboardEasing::SineEaseInOut => Rc::new(SineEaseInOut::new()),
    }
}

/// The input pane of UIKit.
#[derive(Default)]
pub struct UIKitInputPane {
    base: InputPaneBase,
}

impl UIKitInputPane {
    /// The keyboard is about to be shown (`is_up`) or hidden: its frames
    /// at the start and at the end of the animation, in the coordinates
    /// of the screen, the duration of the animation in seconds and its
    /// curve.
    pub fn raise_event_from_notification(
        &self,
        is_up: bool,
        start_frame: Rect,
        end_frame: Rect,
        duration: f64,
        curve: usize,
    ) {
        let state = if is_up { InputPaneState::Open } else { InputPaneState::Closed };
        self.base.set_state(state);
        self.base.set_occluded_rect(end_frame);

        self.base.on_state_changed(InputPaneStateEventArgs::with_animation(
            state,
            Some(start_frame),
            end_frame,
            Duration::try_from_secs_f64(duration).unwrap_or_default(),
            Some(easing(easing_of_curve(curve))),
        ));
    }
}

impl IInputPane for UIKitInputPane {
    fn state(&self) -> InputPaneState {
        self.base.state()
    }

    fn occluded_rect(&self) -> Rect {
        self.base.occluded_rect()
    }

    fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.state_changed(handler)
    }
}

#[cfg(target_os = "ios")]
mod uikit {
    use super::UIKitInputPane;
    use crate::extensions::to_rect;
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{
        NSDictionary, NSNotification, NSNotificationCenter, NSNotificationName, NSNumber, NSOperationQueue,
        NSString, NSValue,
    };
    use objc2_ui_kit::{
        NSValueUIGeometryExtensions, UIKeyboardAnimationCurveUserInfoKey, UIKeyboardAnimationDurationUserInfoKey,
        UIKeyboardFrameBeginUserInfoKey, UIKeyboardFrameEndUserInfoKey, UIKeyboardWillHideNotification,
        UIKeyboardWillShowNotification,
    };
    use ferroui_base::Rect;
    use std::ptr::NonNull;
    use std::rc::Rc;

    thread_local! {
        static INSTANCE: Rc<UIKitInputPane> = UIKitInputPane::new();
    }

    fn value(user_info: &NSDictionary, key: &NSString) -> Option<Retained<AnyObject>> {
        let key: &AnyObject = key;
        user_info.objectForKey(key)
    }

    fn frame(user_info: &NSDictionary, key: &NSString) -> Rect {
        value(user_info, key)
            .and_then(|value| value.downcast::<NSValue>().ok())
            // SAFETY: the value of the frame keys of a keyboard
            // notification holds a rectangle.
            .map(|value| to_rect(unsafe { value.CGRectValue() }))
            .unwrap_or_default()
    }

    fn number(user_info: &NSDictionary, key: &NSString) -> Option<Retained<NSNumber>> {
        value(user_info, key).and_then(|value| value.downcast::<NSNumber>().ok())
    }

    impl UIKitInputPane {
        /// The input pane of the application.
        pub fn instance() -> Rc<UIKitInputPane> {
            INSTANCE.with(Rc::clone)
        }

        fn new() -> Rc<UIKitInputPane> {
            let this = Rc::new(UIKitInputPane::default());
            // SAFETY: the two names are constants of UIKit.
            let (will_show, will_hide) = unsafe { (UIKeyboardWillShowNotification, UIKeyboardWillHideNotification) };
            Self::observe(&this, will_show, true);
            Self::observe(&this, will_hide, false);
            this
        }

        fn observe(this: &Rc<UIKitInputPane>, name: &NSNotificationName, is_up: bool) {
            let weak = Rc::downgrade(this);
            let block = RcBlock::new(move |notification: NonNull<NSNotification>| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                // SAFETY: the notification is valid for the call of its
                // observer.
                let notification = unsafe { notification.as_ref() };
                let Some(user_info) = notification.userInfo() else {
                    return;
                };
                // SAFETY: the four keys are constants of UIKit.
                let (begin, end, duration, curve) = unsafe {
                    (
                        UIKeyboardFrameBeginUserInfoKey,
                        UIKeyboardFrameEndUserInfoKey,
                        UIKeyboardAnimationDurationUserInfoKey,
                        UIKeyboardAnimationCurveUserInfoKey,
                    )
                };
                this.raise_event_from_notification(
                    is_up,
                    frame(&user_info, begin),
                    frame(&user_info, end),
                    number(&user_info, duration).map_or(0.0, |duration| duration.doubleValue()),
                    number(&user_info, curve).map_or(0, |curve| curve.unsignedIntegerValue()),
                );
            });
            // SAFETY: the block takes the one argument of a notification
            // block and is run on the main queue, the thread the input
            // pane belongs to.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(name),
                    None,
                    Some(&NSOperationQueue::mainQueue()),
                    &block,
                )
            };
            // The input pane lives as long as the thread, and so does the
            // subscription (the reference does not remove it either).
            std::mem::forget(observer);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use std::cell::RefCell;

    #[test]
    fn every_curve_of_a_notification_is_ease_in_out() {
        // The curves of UIKit (ease in out, ease in, ease out, linear) and
        // the one the keyboard uses.
        for curve in [0, 1, 2, 3, 7] {
            assert_eq!(KeyboardEasing::SineEaseInOut, easing_of_curve(curve));
        }
        // The options the reference compares with.
        assert_eq!(KeyboardEasing::Linear, easing_of_curve(view_animation_options::CURVE_LINEAR));
        assert_eq!(KeyboardEasing::SineEaseIn, easing_of_curve(view_animation_options::CURVE_EASE_IN));
        assert_eq!(KeyboardEasing::SineEaseOut, easing_of_curve(view_animation_options::CURVE_EASE_OUT));
        assert_eq!(KeyboardEasing::SineEaseInOut, easing_of_curve(view_animation_options::CURVE_EASE_IN_OUT));
    }

    #[test]
    fn the_notifications_open_and_close_the_pane() {
        let _scope = Dispatcher::unit_test_scope();
        let pane = UIKitInputPane::default();
        assert_eq!(InputPaneState::Closed, pane.state());
        assert_eq!(Rect::default(), pane.occluded_rect());

        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let _subscription = pane.state_changed(Rc::new(move |e: &InputPaneStateEventArgs| {
            log.borrow_mut().push((e.new_state(), e.start_rect(), e.end_rect(), e.animation_duration(), e.easing().is_some()));
        }));

        let hidden = Rect::new(0.0, 874.0, 402.0, 336.0);
        let shown = Rect::new(0.0, 538.0, 402.0, 336.0);
        pane.raise_event_from_notification(true, hidden, shown, 0.25, 7);
        assert_eq!(InputPaneState::Open, pane.state());
        assert_eq!(shown, pane.occluded_rect());

        pane.raise_event_from_notification(false, shown, hidden, 0.25, 7);
        assert_eq!(InputPaneState::Closed, pane.state());
        assert_eq!(hidden, pane.occluded_rect());

        assert_eq!(
            vec![
                (InputPaneState::Open, Some(hidden), shown, Duration::from_millis(250), true),
                (InputPaneState::Closed, Some(shown), hidden, Duration::from_millis(250), true),
            ],
            *seen.borrow()
        );
    }

    #[test]
    fn a_duration_that_is_not_a_time_is_no_animation() {
        let _scope = Dispatcher::unit_test_scope();
        let pane = UIKitInputPane::default();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let _subscription =
            pane.state_changed(Rc::new(move |e: &InputPaneStateEventArgs| log.borrow_mut().push(e.animation_duration())));
        pane.raise_event_from_notification(true, Rect::default(), Rect::default(), f64::NAN, 0);
        pane.raise_event_from_notification(true, Rect::default(), Rect::default(), -1.0, 0);
        assert_eq!(vec![Duration::ZERO, Duration::ZERO], *seen.borrow());
    }
}
