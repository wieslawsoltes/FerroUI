//! Focused SDK-only compile-failure reproducer: CLR model callbacks can cross threads.
//! No FerroRibbon source or dependency is used by this program.
use ferroui_system::events::Event;
#[test]
fn model_events_need_a_thread_safe_notification_bridge() {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<Event<str>>();
}
#[test]
fn model_events_need_nullable_utf16_property_names() {
    let event = Event::<str>::new();
    event.raise(None::<&str>);
}
