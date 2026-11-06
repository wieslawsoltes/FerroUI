//! Explicit native metadata cleanup for the selected add-only markup event contract.
use ferroboard_core::{
    editor::ITextInputTarget, engine::KeyboardEngine, layouts::KeyboardResources,
};
use ferroui_base::metadata::{MarkupDelegate, MarkupValue};
use ferroui_system::{IDisposable, Subscription};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
type Key = (u8, usize);
ferroui_system::ferro_thread_local! {static CLEANUP:RefCell<HashMap<Key,Vec<Subscription>>>=RefCell::default();}
fn retain(key: Key, token: Rc<dyn IDisposable>) {
    CLEANUP.with_borrow_mut(|m| m.entry(key).or_default().push(token.into()));
}
fn release(key: Key) {
    let tokens = CLEANUP.with_borrow_mut(|m| m.remove(&key).unwrap_or_default());
    for t in tokens {
        t.dispose();
    }
}
pub(crate) fn target_id(t: &Rc<dyn ITextInputTarget>) -> usize {
    Rc::as_ptr(t).cast::<()>() as usize
}
pub(crate) fn retain_target(t: &Rc<dyn ITextInputTarget>, token: Rc<dyn IDisposable>) {
    retain((0, target_id(t)), token);
}
pub(crate) fn retain_engine(e: &Rc<KeyboardEngine>, token: Rc<dyn IDisposable>) {
    retain((1, Rc::as_ptr(e) as usize), token);
}
pub(crate) fn retain_resources(resources: &Rc<KeyboardResources>, token: Rc<dyn IDisposable>) {
    retain((2, Rc::as_ptr(resources) as usize), token);
}
/// Port-only explicit cleanup for the selected add-only native event metadata contract.
pub fn release_resources_markup_subscriptions(resources: &Rc<KeyboardResources>) {
    release((2, Rc::as_ptr(resources) as usize));
}
/// Port-only lifetime bridge; this is not a fabricated source TextBufferTarget.Dispose method.
pub fn release_target_markup_subscriptions(target: &Rc<dyn ITextInputTarget>) {
    release((0, target_id(target)));
}
/// Source Engine.Dispose does not clear its public event handlers; cleanup is a separate native step.
pub fn release_engine_markup_subscriptions(engine: &Rc<KeyboardEngine>) {
    release((1, Rc::as_ptr(engine) as usize));
}
pub(crate) fn invoke(handler: &MarkupDelegate, args: &[MarkupValue]) {
    if let Err(error) = handler.try_invoke(args) {
        std::panic::panic_any(error);
    }
}
