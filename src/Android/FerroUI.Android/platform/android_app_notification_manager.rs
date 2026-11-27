//! The notifications of Android through the notification manager of the system (#1276).
//!
//! Not from upstream. `NotificationManager` posts the notifications of `AppNotificationManager`
//! to the channel of the application (`NotificationHelper.java`): the first text is the title,
//! the other texts and the attribution text the body, the buttons are the actions and the
//! progress bar is the progress of the notification. A notification with a tag is posted with
//! its group and tag, so that one replaces another. A click starts the launch activity of the
//! application with the arguments of what was clicked in an extra of its intent; reading that
//! extra (`NotificationInvoked`) is not part of the platform yet, and the inputs and images are
//! not shown. `register` asks for the permission to post notifications on Android 13 and later,
//! which the application declares in its manifest.

use crate::ferro_activity::FerroActivity;
use crate::ferro_android_application::FerroAndroidApplication;
use crate::interop::java::{call_static_boolean, call_static_void, new_string_array, JavaClass, JavaValue};
use ferroui_base::platform::notifications::{
    AppNotification, AppNotificationError, AppNotificationScenario, IAppNotificationManagerImpl,
};
use std::cell::RefCell;

/// The Java side of the notifications.
pub(crate) const NOTIFICATION_HELPER: &str = "org/ferroui/android/NotificationHelper";

/// The tag a notification without a tag of its own is posted with (by its id).
const DEFAULT_TAG: &str = "org.ferroui.notification";

/// A notification the system shows: its id, tag and group, and the tag and id it was posted with.
struct Entry {
    id: u32,
    tag: String,
    group: String,
    posted_tag: String,
    posted_id: i32,
}

/// The notification service of Android. Registered by `use_platform_app_notifications()` of the
/// Android platform.
#[derive(Default)]
pub struct AndroidAppNotificationManager {
    entries: RefCell<Vec<Entry>>,
}

impl AndroidAppNotificationManager {
    /// The service of the application.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the notification manager of the system as the notification service of the
    /// application.
    pub fn register() {
        ferroui_base::platform::notifications::register_app_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn helper() -> JavaClass {
        JavaClass::find(NOTIFICATION_HELPER)
    }

    fn remove(&self, matches: impl Fn(&Entry) -> bool) {
        let Some(context) = FerroAndroidApplication::try_context() else { return };
        self.entries.borrow_mut().retain(|entry| {
            if !matches(entry) {
                return true;
            }
            call_static_void(
                &Self::helper(),
                "cancel",
                "(Landroid/content/Context;Ljava/lang/String;I)V",
                &[
                    JavaValue::Object(Some(&context)),
                    JavaValue::String(&entry.posted_tag),
                    JavaValue::Int(entry.posted_id),
                ],
            );
            false
        });
    }
}

impl IAppNotificationManagerImpl for AndroidAppNotificationManager {
    fn is_supported(&self) -> bool {
        FerroAndroidApplication::try_context().is_some()
    }

    fn register(&self) {
        if let Some(activity) = FerroActivity::current_main_activity() {
            call_static_void(
                &Self::helper(),
                "requestPermission",
                "(Landroid/app/Activity;)V",
                &[JavaValue::Object(Some(activity.java()))],
            );
        }
    }

    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError> {
        let context = FerroAndroidApplication::try_context().ok_or(AppNotificationError::NotSupported)?;
        let helper = Self::helper();
        let context_argument = [JavaValue::Object(Some(&context))];
        let can_post = call_static_boolean(&helper, "canPost", "(Landroid/content/Context;)Z", &context_argument);
        if !can_post {
            return Err(AppNotificationError::Disabled);
        }
        let (posted_tag, posted_id) = if notification.tag().is_empty() {
            (DEFAULT_TAG.to_string(), notification.id() as i32)
        } else {
            (format!("{}\u{1f}{}", notification.group(), notification.tag()), 0)
        };
        let mut lines: Vec<&str> = notification.texts().iter().skip(1).map(String::as_str).collect();
        if let Some(attribution) = notification.attribution_text() {
            lines.push(attribution);
        }
        if let Some(status) = notification.progress_bar().map(|progress| progress.status()).filter(|s| !s.is_empty()) {
            lines.push(status);
        }
        let body = lines.join("\n");
        let buttons: Vec<String> = notification.buttons().iter().map(|button| button.content().to_string()).collect();
        let button_arguments: Vec<String> =
            notification.buttons().iter().map(|button| button.argument_string()).collect();
        let (buttons, button_arguments) = (new_string_array(&buttons), new_string_array(&button_arguments));
        let progress = notification.progress_bar().map_or(-1, |progress| (progress.value() * 100.0).round() as i32);
        let stays = notification.scenario() != AppNotificationScenario::Default || notification.has_inputs();
        let shown = call_static_boolean(
            &helper,
            "show",
            "(Landroid/content/Context;Ljava/lang/String;ILjava/lang/String;Ljava/lang/String;Ljava/lang/String;\
             [Ljava/lang/String;[Ljava/lang/String;IZZ)Z",
            &[
                JavaValue::Object(Some(&context)),
                JavaValue::String(&posted_tag),
                JavaValue::Int(posted_id),
                JavaValue::String(notification.title()),
                JavaValue::String(&body),
                JavaValue::String(&notification.argument_string()),
                JavaValue::Object(Some(&buttons)),
                JavaValue::Object(Some(&button_arguments)),
                JavaValue::Int(progress),
                JavaValue::Boolean(notification.is_audio_muted()),
                JavaValue::Boolean(stays),
            ],
        );
        if !shown {
            return Err(AppNotificationError::Failed("the notification manager refused the notification".into()));
        }
        let mut entries = self.entries.borrow_mut();
        entries.retain(|entry| !(entry.posted_tag == posted_tag && entry.posted_id == posted_id));
        entries.push(Entry {
            id: notification.id(),
            tag: notification.tag().to_string(),
            group: notification.group().to_string(),
            posted_tag,
            posted_id,
        });
        Ok(())
    }

    fn remove_by_id(&self, id: u32) {
        self.remove(|entry| entry.id == id);
    }

    fn remove_by_tag(&self, tag: &str, group: Option<&str>) {
        self.remove(|entry| entry.tag == tag && group.is_none_or(|group| entry.group == group));
    }

    fn remove_by_group(&self, group: &str) {
        self.remove(|entry| entry.group == group);
    }

    fn remove_all(&self) {
        self.entries.borrow_mut().clear();
        if let Some(context) = FerroAndroidApplication::try_context() {
            call_static_void(
                &Self::helper(),
                "cancelAll",
                "(Landroid/content/Context;)V",
                &[JavaValue::Object(Some(&context))],
            );
        }
    }
}
