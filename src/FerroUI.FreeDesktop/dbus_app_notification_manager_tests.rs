//! The notification service over a double of the notification server, on a socket pair: no
//! notification reaches the desktop.

use super::*;
use crate::test_support::{expect_calls, log, pump_until, scope, Log, TestConnections};
use ferroui_base::platform::notifications::{
    AppNotificationBuilder, AppNotificationButton, AppNotificationProgressBar,
};
use ferroui_base::FerroLocator;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

const INTERFACE: &str = "org.freedesktop.Notifications";

/// A double of the notification server: logs the calls, gives ids from 100.
struct Server {
    next_id: Arc<AtomicU32>,
    log: Log,
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        _app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, zbus::OwnedVariant>,
        expire_timeout: i32,
    ) -> u32 {
        let mut hints: Vec<String> =
            hints.iter().map(|(key, value)| format!("{key}={}", crate::test_support::show(value))).collect();
        hints.sort();
        log(
            &self.log,
            format!(
                "Notify({app_name}, {replaces_id}, {summary}, {body}, [{}], [{}], {expire_timeout})",
                actions.join(","),
                hints.join(",")
            ),
        );
        if replaces_id != 0 {
            replaces_id
        } else {
            self.next_id.fetch_add(1, Ordering::SeqCst)
        }
    }

    fn close_notification(&self, id: u32) {
        log(&self.log, format!("CloseNotification({id})"));
    }
}

struct Fixture {
    connections: TestConnections,
    log: Log,
    manager: Rc<DBusAppNotificationManager>,
    _locator: Rc<dyn IDisposable>,
}

fn fixture() -> Fixture {
    let log: Log = Arc::default();
    let server = Server { next_id: Arc::new(AtomicU32::new(100)), log: log.clone() };
    let connections = TestConnections::new(PATH, move |builder| builder.serve_at(PATH, server));
    connections.start(SERVICE);
    let manager = DBusAppNotificationManager::with_connection(Some(connections.client.clone()), "Gallery");
    let locator = FerroLocator::enter_scope();
    ferroui_base::platform::notifications::register_app_notification_manager_impl(manager.clone());
    Fixture { connections, log, manager, _locator: locator }
}

/// Shows `notification` as the application does: through `AppNotificationManager`, which
/// gives it its id and calls the service the fixture registered.
fn shown(notification: &mut AppNotification) {
    AppNotificationManager::default().show(notification).unwrap();
}

#[test]
fn without_a_connection_notifications_are_not_supported() {
    let _scope = scope();
    let manager = DBusAppNotificationManager::with_connection(None, "Gallery");
    assert!(!manager.is_supported());
    let notification = AppNotificationBuilder::new().add_text("Title").build_notification();
    assert_eq!(manager.show(&notification), Err(AppNotificationError::NotSupported));
}

#[test]
fn a_notification_is_the_summary_body_actions_and_hints_of_notify() {
    let _scope = scope();
    let f = fixture();
    assert!(f.manager.is_supported());
    let mut notification = AppNotificationBuilder::new()
        .add_text("Survey")
        .add_text("Pick <one> & comment.")
        .set_attribution_text("Gallery")
        .set_audio_event(AppNotificationSoundEvent::IM)
        .add_text_box("comment", "Leave a comment here...", "")
        .add_button(AppNotificationButton::new("Submit").add_argument("action", "submit_survey"))
        .build_notification();
    shown(&mut notification);
    expect_calls(
        &f.connections,
        &f.log,
        &["Notify(Gallery, 0, Survey, Pick &lt;one&gt; &amp; comment.\nGallery, [default,,0,Submit], \
           [sound-name=message-new-instant,urgency=1], 0)"],
    );

    let mut progress = AppNotificationBuilder::new()
        .add_text("Progress")
        .add_progress_bar(AppNotificationProgressBar::new().set_value(0.6).set_status("In progress..."))
        .mute_audio()
        .build_notification();
    shown(&mut progress);
    expect_calls(
        &f.connections,
        &f.log,
        &["Notify(Gallery, 0, Progress, In progress..., [default,], [suppress-sound=true,urgency=1,value=60], -1)"],
    );
}

#[test]
fn a_notification_with_the_tag_of_a_shown_one_replaces_it_and_remove_closes_it() {
    let _scope = scope();
    let f = fixture();
    let mut first = AppNotificationBuilder::new().add_text("One").set_tag("download").build_notification();
    shown(&mut first);
    expect_calls(&f.connections, &f.log, &["Notify(Gallery, 0, One, , [default,], [urgency=1], -1)"]);
    let mut second = AppNotificationBuilder::new().add_text("Two").set_tag("download").build_notification();
    shown(&mut second);
    expect_calls(&f.connections, &f.log, &["Notify(Gallery, 100, Two, , [default,], [urgency=1], -1)"]);
    f.manager.remove_by_tag("download", None);
    expect_calls(&f.connections, &f.log, &["CloseNotification(100)"]);
    // Nothing is left to remove.
    f.manager.remove_all();
    f.connections.settle();
    assert!(f.log.lock().unwrap().is_empty());
}

#[test]
fn a_notification_removed_before_the_server_answered_is_closed_when_it_answers() {
    let _scope = scope();
    let f = fixture();
    let mut notification = AppNotificationBuilder::new().add_text("Gone").set_group("files").build_notification();
    shown(&mut notification);
    f.manager.remove_by_group("files");
    expect_calls(
        &f.connections,
        &f.log,
        &["Notify(Gallery, 0, Gone, , [default,], [urgency=1], -1)", "CloseNotification(100)"],
    );
}

#[test]
fn actions_of_the_server_raise_notification_invoked_with_the_arguments() {
    let _scope = scope();
    let f = fixture();
    f.manager.register();
    f.connections.settle();
    let mut notification = AppNotificationBuilder::new()
        .add_text("Survey")
        .add_argument("page", "survey")
        .add_button(AppNotificationButton::new("Submit").add_argument("action", "submit_survey"))
        .build_notification();
    shown(&mut notification);
    f.connections.settle();
    f.log.lock().unwrap().clear();

    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let _subscription = AppNotificationManager::default()
        .notification_invoked(move |args| sink.borrow_mut().push(args.argument().to_string()));
    f.connections.emit(PATH, INTERFACE, "ActionInvoked", &(100u32, "0"));
    pump_until(|| received.borrow().len() == 1);
    f.connections.emit(PATH, INTERFACE, "ActionInvoked", &(100u32, DEFAULT_ACTION));
    pump_until(|| received.borrow().len() == 2);
    // Another application's notification, and an action of none.
    f.connections.emit(PATH, INTERFACE, "ActionInvoked", &(7u32, DEFAULT_ACTION));
    f.connections.emit(PATH, INTERFACE, "ActionInvoked", &(100u32, "9"));
    // The server closed it: its actions are no longer reported.
    f.connections.emit(PATH, INTERFACE, "NotificationClosed", &(100u32, 2u32));
    f.connections.settle();
    f.connections.emit(PATH, INTERFACE, "ActionInvoked", &(100u32, DEFAULT_ACTION));
    f.connections.settle();
    assert_eq!(*received.borrow(), ["action=submit_survey", "page=survey"]);

    f.manager.unregister();
}

#[test]
fn an_image_is_a_hint_only_with_a_file_uri() {
    let with_file = AppNotificationBuilder::new()
        .add_text("Image")
        .set_hero_image("file:///tmp/hero.png")
        .build_notification();
    assert_eq!(NotifyRequest::of(&with_file, 0).image_path.as_deref(), Some("/tmp/hero.png"));
    let with_asset = AppNotificationBuilder::new()
        .add_text("Image")
        .set_hero_image("ferres://Gallery/Assets/hero.jpg")
        .build_notification();
    assert_eq!(NotifyRequest::of(&with_asset, 0).image_path, None);
    let urgent = AppNotificationBuilder::new()
        .add_text("Call")
        .set_scenario(AppNotificationScenario::IncomingCall)
        .build_notification();
    let request = NotifyRequest::of(&urgent, 0);
    assert_eq!((request.urgency, request.expire_timeout), (2, 0));
}
