//! Persist before iOS suspends the application, retaining the live keyboard session.
use ferroui_base::reactive::IDisposable;
use ferroui_ios::IFerroAppDelegate;
use std::rc::Rc;

pub struct BackgroundPersistence {
    subscription: Rc<dyn IDisposable>,
}
impl BackgroundPersistence {
    pub fn new(delegate: &dyn IFerroAppDelegate, save: Rc<dyn Fn()>) -> Self {
        Self {
            subscription: delegate.deactivated(Rc::new(move |_| save())),
        }
    }
}
impl Drop for BackgroundPersistence {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::application_lifetimes::{ActivatedEventArgs, ActivationKind};
    use ferroui_ios::ferro_app_delegate::AppDelegateEvents;
    use std::cell::{Cell, RefCell};

    #[test]
    fn background_saves_repeat_until_native_owner_retires() {
        let events = AppDelegateEvents::new();
        let saves = Rc::new(Cell::new(0));
        let persistence = BackgroundPersistence::new(events.as_ref(), {
            let saves = saves.clone();
            Rc::new(move || saves.set(saves.get() + 1))
        });
        events.on_activated(ActivatedEventArgs::new(ActivationKind::Background));
        assert_eq!(saves.get(), 0);
        for _ in 0..2 {
            events.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background));
        }
        assert_eq!(saves.get(), 2);
        drop(persistence);
        events.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background));
        assert_eq!(saves.get(), 2);
    }

    #[test]
    fn save_can_retire_its_subscription_synchronously() {
        let events = AppDelegateEvents::new();
        let owner = Rc::new(RefCell::new(None));
        let weak = Rc::downgrade(&owner);
        let saves = Rc::new(Cell::new(0));
        let persistence = BackgroundPersistence::new(events.as_ref(), {
            let saves = saves.clone();
            Rc::new(move || {
                saves.set(saves.get() + 1);
                if let Some(owner) = weak.upgrade() {
                    let previous: Option<BackgroundPersistence> = owner.take();
                    drop(previous);
                }
            })
        });
        owner.replace(Some(persistence));
        events.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background));
        events.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background));
        assert_eq!(saves.get(), 1);
        assert!(owner.borrow().is_none());
    }
}
