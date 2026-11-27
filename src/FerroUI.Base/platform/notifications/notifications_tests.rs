//! The notifications of the application over a test double of the platform's service: nothing
//! reaches the notification center of the system.

use super::*;
use crate::utilities::{DateTime, DateTimeKind};
use crate::FerroLocator;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A platform service that records what it is asked to do.
#[derive(Default)]
struct RecordingService {
    supported: Cell<bool>,
    registered: Cell<u32>,
    shown: RefCell<Vec<AppNotification>>,
    removed: RefCell<Vec<String>>,
}

impl IAppNotificationManagerImpl for RecordingService {
    fn is_supported(&self) -> bool {
        self.supported.get()
    }

    fn register(&self) {
        self.registered.set(self.registered.get() + 1);
    }

    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError> {
        self.shown.borrow_mut().push(notification.clone());
        Ok(())
    }

    fn remove_by_id(&self, id: u32) {
        self.removed.borrow_mut().push(format!("id {id}"));
    }

    fn remove_by_tag(&self, tag: &str, group: Option<&str>) {
        self.removed.borrow_mut().push(format!("tag {tag} {group:?}"));
    }

    fn remove_by_group(&self, group: &str) {
        self.removed.borrow_mut().push(format!("group {group}"));
    }

    fn remove_all(&self) {
        self.removed.borrow_mut().push("all".into());
    }
}

/// Runs `test` with `service` registered in a scope of the locator.
fn with_service(supported: bool, test: impl FnOnce(&RecordingService)) {
    let scope = FerroLocator::enter_scope();
    let service = Rc::new(RecordingService::default());
    service.supported.set(supported);
    register_app_notification_manager_impl(service.clone());
    test(&service);
    scope.dispose();
}

#[test]
fn without_a_service_notifications_are_not_supported() {
    let scope = FerroLocator::enter_scope();
    assert!(app_notification_manager_impl().is_none());
    assert!(!AppNotificationManager::is_supported());
    let mut notification = AppNotificationBuilder::new().add_text("Title").build_notification();
    assert_eq!(
        AppNotificationManager::default().show(&mut notification),
        Err(AppNotificationError::NotSupported)
    );
    assert_eq!(notification.id(), 0);
    scope.dispose();
}

#[test]
fn a_service_that_cannot_show_notifications_is_not_used() {
    with_service(false, |service| {
        let manager = AppNotificationManager::default();
        assert!(!AppNotificationManager::is_supported());
        manager.register();
        let mut notification = AppNotificationBuilder::new().add_text("Title").build_notification();
        assert_eq!(manager.show(&mut notification), Err(AppNotificationError::NotSupported));
        manager.remove_all();
        assert_eq!(service.registered.get(), 0);
        assert!(service.shown.borrow().is_empty());
        assert!(service.removed.borrow().is_empty());
    });
}

#[test]
fn show_gives_each_notification_an_id_and_passes_it_to_the_service() {
    with_service(true, |service| {
        let manager = AppNotificationManager::default();
        assert!(AppNotificationManager::is_supported());
        manager.register();
        assert_eq!(service.registered.get(), 1);
        let mut first = AppNotificationBuilder::new().add_text("First").build_notification();
        let mut second = AppNotificationBuilder::new().add_text("Second").build_notification();
        manager.show(&mut first).unwrap();
        manager.show(&mut second).unwrap();
        assert_ne!(first.id(), 0);
        assert_ne!(first.id(), second.id());
        // A notification shown again keeps its id (it replaces itself).
        let id = first.id();
        manager.show(&mut first).unwrap();
        assert_eq!(first.id(), id);
        let shown = service.shown.borrow();
        assert_eq!(shown.len(), 3);
        assert_eq!(shown[0].title(), "First");
        assert_eq!(shown[0].id(), id);
    });
}

#[test]
fn remove_calls_reach_the_service() {
    with_service(true, |service| {
        let manager = AppNotificationManager::default();
        manager.remove_by_id(7);
        manager.remove_by_tag("download");
        manager.remove_by_tag_and_group("download", "files");
        manager.remove_by_group("files");
        manager.remove_all();
        assert_eq!(
            *service.removed.borrow(),
            [
                "id 7",
                "tag download None",
                "tag download Some(\"files\")",
                "group files",
                "all"
            ]
        );
    });
}

#[test]
fn notification_invoked_reaches_the_handlers_until_they_are_dropped() {
    let manager = AppNotificationManager::default();
    let received = Rc::new(RefCell::new(Vec::new()));
    let sink = received.clone();
    let subscription = manager.notification_invoked(move |args| {
        sink.borrow_mut().push((args.argument().to_string(), args.user_input().clone()));
    });
    let mut input = BTreeMap::new();
    input.insert("comment".to_string(), "Nice".to_string());
    AppNotificationManager::raise_notification_invoked(&AppNotificationActivatedEventArgs::new(
        "action=submit_survey",
        input.clone(),
    ));
    drop(subscription);
    AppNotificationManager::raise_notification_invoked(&AppNotificationActivatedEventArgs::default());
    assert_eq!(*received.borrow(), [("action=submit_survey".to_string(), input)]);
}

#[test]
fn arguments_are_escaped_and_parsed_back() {
    let button = AppNotificationButton::new("Open")
        .add_argument("action", "open;file=a%b")
        .add_argument("flag", "");
    assert_eq!(button.argument_string(), "action=open%3Bfile%3Da%25b;flag");
    let args = AppNotificationActivatedEventArgs::new(button.argument_string(), BTreeMap::new());
    assert_eq!(
        args.arguments(),
        [("action".to_string(), "open;file=a%b".to_string()), ("flag".to_string(), String::new())]
    );
}

#[test]
fn the_builder_keeps_what_the_notification_shows() {
    let notification = AppNotificationBuilder::new()
        .add_text("Survey")
        .add_text("Please select your satisfaction level")
        .add_text("and leave a comment.")
        .set_attribution_text("WinUI gallery")
        .set_app_logo_override("file:///logo.png", AppNotificationImageCrop::Circle)
        .set_audio_event(AppNotificationSoundEvent::IM)
        .add_combo_box(
            AppNotificationComboBox::new("satisfaction")
                .add_item("1", "Bad")
                .add_item("2", "Good")
                .set_selected_item("2"),
        )
        .add_text_box("comment", "Leave a comment here...", "")
        .add_button(AppNotificationButton::new("Submit").add_argument("action", "submit_survey"))
        .set_tag("survey")
        .set_group("feedback")
        .build_notification();
    assert_eq!(notification.title(), "Survey");
    assert_eq!(notification.body(), "Please select your satisfaction level\nand leave a comment.");
    assert_eq!(notification.attribution_text(), Some("WinUI gallery"));
    assert_eq!(
        notification.app_logo_override(),
        Some(("file:///logo.png", AppNotificationImageCrop::Circle))
    );
    assert_eq!(notification.audio_event(), Some(AppNotificationSoundEvent::IM));
    assert!(notification.has_inputs());
    assert_eq!(notification.combo_boxes()[0].selected_item(), Some("2"));
    assert_eq!(notification.text_boxes()[0].placeholder_text(), "Leave a comment here...");
    assert_eq!(notification.buttons()[0].argument_string(), "action=submit_survey");
    assert_eq!((notification.tag(), notification.group()), ("survey", "feedback"));
    assert!(!AppNotificationBuilder::new().add_text("Plain").build_notification().has_inputs());
}

#[test]
fn the_payload_is_the_xml_of_a_windows_notification() {
    let time = DateTime::new_with_time(2026, 11, 27, 10, 30, 5).specify_kind(DateTimeKind::Utc);
    let notification = AppNotificationBuilder::new()
        .add_text("Harbor <Scene>")
        .add_text("Boats & \"water\"")
        .set_attribution_text("Gallery")
        .set_app_logo_override("https://example.com/logo.png", AppNotificationImageCrop::Circle)
        // Windows cannot load this one: it is left out.
        .set_hero_image("ferres://Gallery/Assets/hero.jpg")
        .set_time_stamp(time)
        .add_argument("page", "harbor")
        .add_progress_bar(
            AppNotificationProgressBar::new()
                .set_title("Demo")
                .set_value(0.6)
                .set_value_string_override("60%")
                .set_status("In progress..."),
        )
        .add_combo_box(AppNotificationComboBox::new("level").add_item("1", "Bad").set_selected_item("1"))
        .add_text_box("comment", "Comment", "")
        .add_button(AppNotificationButton::new("Submit").add_argument("action", "submit").set_input_id("comment"))
        .set_audio_event(AppNotificationSoundEvent::Alarm)
        .set_scenario(AppNotificationScenario::Reminder)
        .build_notification();
    assert_eq!(
        notification.payload(),
        concat!(
            "<toast launch=\"page=harbor\" scenario=\"reminder\" displayTimestamp=\"2026-11-27T10:30:05Z\">",
            "<visual><binding template=\"ToastGeneric\">",
            "<text>Harbor &lt;Scene&gt;</text><text>Boats &amp; &quot;water&quot;</text>",
            "<text placement=\"attribution\">Gallery</text>",
            "<image placement=\"appLogoOverride\" src=\"https://example.com/logo.png\" hint-crop=\"circle\"/>",
            "<progress title=\"Demo\" value=\"0.6\" valueStringOverride=\"60%\" status=\"In progress...\"/>",
            "</binding></visual><actions>",
            "<input type=\"text\" id=\"comment\" placeHolderContent=\"Comment\"/>",
            "<input type=\"selection\" id=\"level\" defaultInput=\"1\"><selection id=\"1\" content=\"Bad\"/></input>",
            "<action content=\"Submit\" arguments=\"action=submit\" hint-inputId=\"comment\"/>",
            "</actions><audio src=\"ms-winsoundevent:Notification.Looping.Alarm\"/></toast>"
        )
    );
    let muted = AppNotificationBuilder::new().add_text("Quiet").mute_audio().build_notification();
    assert_eq!(
        muted.payload(),
        "<toast><visual><binding template=\"ToastGeneric\"><text>Quiet</text></binding></visual>\
         <audio silent=\"true\"/></toast>"
    );
}

#[test]
fn the_progress_value_is_clamped() {
    assert_eq!(AppNotificationProgressBar::new().set_value(1.5).value(), 1.0);
    assert_eq!(AppNotificationProgressBar::new().set_value(f64::NAN).value(), 0.0);
}
