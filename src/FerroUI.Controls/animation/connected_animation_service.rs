use super::ConnectedAnimation;
use crate::TopLevel;
use ferroui_base::animation::easings::Easing;
use ferroui_base::animation::TimeSpan;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectImpl, Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// The service of each top level (the conditional weak table of the
    /// reference): an entry goes away with its top level.
    static PER_VIEW: RefCell<Vec<(WeakRef<TopLevel>, Ref<ConnectedAnimationService>)>> =
        const { RefCell::new(Vec::new()) };
}

/// Coordinates connected animations across views. Each [`TopLevel`] window
/// has its own independent instance so animations cannot bleed across
/// windows.
///
/// Typical usage:
///
/// 1. On the source view, call [`prepare_to_animate`](Self::prepare_to_animate)
///    to capture the element.
/// 2. Navigate to the destination view.
/// 3. On the destination view, call [`get_animation`](Self::get_animation)
///    then [`try_start`](ConnectedAnimation::try_start) on the returned
///    animation to run the animation.
#[repr(C)]
pub struct ConnectedAnimationService {
    base: FerroObject,
    animations: RefCell<HashMap<String, Rc<ConnectedAnimation>>>,
    default_duration: Cell<TimeSpan>,
    default_easing_function: RefCell<Option<Easing>>,
}

ferro_class!(ConnectedAnimationService: FerroObject);
ferro_class_info!(ConnectedAnimationService {});

ferroui_base::ferro_impl_classes!(ConnectedAnimationService: FerroObjectImpl);

impl ConnectedAnimationService {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub(crate) fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            animations: RefCell::new(HashMap::new()),
            default_duration: Cell::new(TimeSpan::from_milliseconds(300.0)),
            default_easing_function: RefCell::new(None),
        }
    }

    /// Initializes a new service.
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the [`ConnectedAnimationService`] for the specified `top_level`.
    /// Each top-level window has its own isolated instance.
    pub fn get_for_top_level(top_level: &TopLevel) -> Ref<ConnectedAnimationService> {
        PER_VIEW.with(|per_view| {
            let mut per_view = per_view.borrow_mut();
            per_view.retain(|(key, _)| key.upgrade().is_some());

            if let Some((_, service)) =
                per_view.iter().find(|(key, _)| key.upgrade().is_some_and(|key| std::ptr::eq(&*key, top_level)))
            {
                return service.clone();
            }

            let service = ConnectedAnimationService::new();
            per_view.push((top_level.to_ref().downgrade(), service.clone()));
            service
        })
    }

    /// Gets the default duration applied to all animations whose
    /// configuration does not specify one. Defaults to 300 ms.
    pub fn default_duration(&self) -> TimeSpan {
        self.default_duration.get()
    }

    /// Sets the default duration applied to all animations whose
    /// configuration does not specify one.
    pub fn set_default_duration(&self, value: TimeSpan) {
        self.default_duration.set(value);
    }

    /// Gets the default easing function applied when the active
    /// [`ConnectedAnimationConfiguration`](super::ConnectedAnimationConfiguration)
    /// does not specify one. When `None` a configuration-specific default
    /// is used.
    pub fn default_easing_function(&self) -> Option<Easing> {
        self.default_easing_function.borrow().clone()
    }

    /// Sets the default easing function.
    pub fn set_default_easing_function(&self, value: Option<Easing>) {
        *self.default_easing_function.borrow_mut() = value;
    }

    /// Captures `source` and registers a pending animation under `key`. Call
    /// this on the source view *before* navigating away.
    ///
    /// `key` is the unique string that pairs this call with the matching
    /// [`get_animation`](Self::get_animation) call on the destination view;
    /// `source` is the element to animate from. Returns the prepared
    /// [`ConnectedAnimation`].
    ///
    /// # Panics
    ///
    /// Panics if `key` is empty.
    pub fn prepare_to_animate(&self, key: &str, source: &Visual) -> Rc<ConnectedAnimation> {
        if key.is_empty() {
            panic!("The value cannot be an empty string. (Parameter 'key')");
        }

        // Replace any stale animation registered under the same key.
        let old = self.animations.borrow_mut().remove(key);
        if let Some(old) = old {
            old.dispose();
        }

        let animation = ConnectedAnimation::new(key, source, &self.to_ref());
        self.animations.borrow_mut().insert(key.to_owned(), animation.clone());
        animation
    }

    /// Retrieves a pending animation registered under `key`. Returns `None`
    /// if no animation exists or if it has already been consumed. Call this
    /// on the destination view *after* navigating, then call
    /// [`try_start`](ConnectedAnimation::try_start) on the returned
    /// animation.
    pub fn get_animation(&self, key: &str) -> Option<Rc<ConnectedAnimation>> {
        let animation = self.animations.borrow().get(key).cloned();
        animation.filter(|animation| !animation.is_consumed())
    }

    /// Removes the animation registered under `key`. The caller is
    /// responsible for disposing the animation separately.
    pub(crate) fn remove_animation(&self, key: &str) {
        let removed = self.animations.borrow_mut().remove(key);
        drop(removed);
    }
}
