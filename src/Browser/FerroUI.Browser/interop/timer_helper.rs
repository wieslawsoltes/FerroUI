use ferroui_base::utilities::HandlerList;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./ferroui.js")]
extern "C" {
    /// Starts the perpetual animation frame loop of the page.
    #[wasm_bindgen(js_namespace = TimerHelper, js_name = runAnimationFrames)]
    pub fn run_animation_frames();
}

thread_local! {
    static ANIMATION_FRAME: HandlerList<dyn Fn(f64)> = HandlerList::new();
}

/// Subscribes to the animation frames; the argument is the timestamp of the
/// frame in milliseconds. Returns the token of the subscription.
pub fn add_animation_frame(handler: Rc<dyn Fn(f64)>) -> u64 {
    ANIMATION_FRAME.with(|handlers| handlers.add(handler))
}

/// Ends a subscription to the animation frames.
pub fn remove_animation_frame(token: u64) -> bool {
    ANIMATION_FRAME.with(|handlers| handlers.remove(token))
}

/// An animation frame of the page.
#[wasm_bindgen(js_name = TimerHelper_JsExportOnAnimationFrame)]
pub fn js_export_on_animation_frame(d: f64) {
    let handlers = ANIMATION_FRAME.with(|handlers| handlers.snapshot());
    for (_, handler) in handlers.iter() {
        handler(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn an_animation_frame_reaches_every_subscriber_until_it_unsubscribes() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let first = add_animation_frame({
            let seen = seen.clone();
            Rc::new(move |timestamp| seen.borrow_mut().push(("first", timestamp)))
        });
        let second = add_animation_frame({
            let seen = seen.clone();
            Rc::new(move |timestamp| seen.borrow_mut().push(("second", timestamp)))
        });

        js_export_on_animation_frame(16.5);
        assert!(remove_animation_frame(first));
        js_export_on_animation_frame(33.0);

        assert_eq!(vec![("first", 16.5), ("second", 16.5), ("second", 33.0)], *seen.borrow());
        assert!(remove_animation_frame(second));
        assert!(!remove_animation_frame(second));
    }
}
