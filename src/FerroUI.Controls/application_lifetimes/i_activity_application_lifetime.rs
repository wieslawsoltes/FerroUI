use super::IApplicationLifetime;
use crate::Control;
use ferroui_base::Ref;
use std::rc::Rc;

/// The lifetime of an application whose views are created per activity: the
/// platform asks the factory for a main view whenever it creates one.
pub trait IActivityApplicationLifetime: IApplicationLifetime {
    /// The factory that creates the main view of an activity.
    fn main_view_factory(&self) -> Option<Rc<dyn Fn() -> Ref<Control>>>;

    /// Sets the factory that creates the main view of an activity.
    fn set_main_view_factory(&self, value: Option<Rc<dyn Fn() -> Ref<Control>>>);
}

impl dyn IActivityApplicationLifetime {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
