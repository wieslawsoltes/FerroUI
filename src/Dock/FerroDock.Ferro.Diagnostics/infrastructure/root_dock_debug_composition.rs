//! Default library composition root; presentation has no UI dependency.
use crate::view_models::{IEventLogClock, RootDockDebugViewModel};
use ferroui_base::utilities::DateTime;
use std::rc::Rc;
struct LocalEventLogClock;
impl IEventLogClock for LocalEventLogClock {
    fn local_timestamp(&self) -> String {
        DateTime::now().to_string_with("HH:mm:ss.fff")
    }
}
pub(crate) fn create_presentation() -> Rc<RootDockDebugViewModel> {
    RootDockDebugViewModel::new(Rc::new(LocalEventLogClock))
}
