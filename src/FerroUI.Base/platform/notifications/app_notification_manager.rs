//! The manager of the notifications of the application (`AppNotificationManager`) and the
//! contract of the platforms (`IAppNotificationManagerImpl`).

use super::app_notification::{parse_arguments, AppNotification};
use crate::reactive::{Disposable, Subscription};
use crate::{FerroLocator, LocatorExtensions};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

/// Why a notification was not shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppNotificationError {
    /// The platform has no notification service, or the application did not register one.
    NotSupported,
    /// The user or a policy does not allow the application's notifications.
    Disabled,
    /// The notification service of the platform refused the notification.
    Failed(String),
}

impl fmt::Display for AppNotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSupported => f.write_str("notifications are not supported"),
            Self::Disabled => f.write_str("notifications are disabled for the application"),
            Self::Failed(reason) => write!(f, "the notification was not shown: {reason}"),
        }
    }
}

impl std::error::Error for AppNotificationError {}

/// What the application is given when the user clicks a notification or one of its buttons
/// (`AppNotificationActivatedEventArgs`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppNotificationActivatedEventArgs {
    argument: String,
    user_input: BTreeMap<String, String>,
}

impl AppNotificationActivatedEventArgs {
    /// The arguments string of what was clicked (the notification's or the button's) and what the
    /// user entered in the notification's inputs, by the ids of the inputs.
    pub fn new(argument: impl Into<String>, user_input: BTreeMap<String, String>) -> Self {
        Self { argument: argument.into(), user_input }
    }

    /// The arguments string of what was clicked (`Argument`).
    pub fn argument(&self) -> &str {
        &self.argument
    }

    /// The arguments of what was clicked, unescaped, in their order (`Arguments`).
    pub fn arguments(&self) -> Vec<(String, String)> {
        parse_arguments(&self.argument)
    }

    /// What the user entered in the inputs, by their ids (`UserInput`).
    pub fn user_input(&self) -> &BTreeMap<String, String> {
        &self.user_input
    }
}

/// The notification service of a platform.
///
/// Every method is called on the UI thread. The manager gives a notification its id before it
/// calls [`show`](Self::show); a platform that has ids of its own maps them. A platform reports a
/// click on a notification with [`AppNotificationManager::raise_notification_invoked`], on the UI
/// thread.
pub trait IAppNotificationManagerImpl {
    /// Whether the platform can show notifications for this application: `false`, for example,
    /// for an application without the bundle or the identity the platform's service needs.
    fn is_supported(&self) -> bool;

    /// Asks the user to allow the application's notifications where the platform asks, and
    /// starts reporting clicks (`Register`).
    fn register(&self);

    /// Stops reporting clicks (`Unregister`).
    fn unregister(&self) {}

    /// Shows `notification`, or replaces the shown notification with its tag and group.
    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError>;

    /// Removes the shown notification with `id`.
    fn remove_by_id(&self, id: u32);

    /// Removes the shown notifications with `tag`, in `group` when it is given.
    fn remove_by_tag(&self, tag: &str, group: Option<&str>);

    /// Removes the shown notifications of `group`.
    fn remove_by_group(&self, group: &str);

    /// Removes every shown notification of the application.
    fn remove_all(&self);
}

/// The notification service the application registered, or `None` (the platform has none, or
/// the application did not ask for it).
pub fn app_notification_manager_impl() -> Option<Rc<dyn IAppNotificationManagerImpl>> {
    FerroLocator::current().get_service::<dyn IAppNotificationManagerImpl>()
}

/// Registers `service` as the notification service of the application.
pub fn register_app_notification_manager_impl(service: Rc<dyn IAppNotificationManagerImpl>) {
    FerroLocator::current_mutable()
        .bind::<dyn IAppNotificationManagerImpl>()
        .to_constant(service);
}

type InvokedHandler = Rc<dyn Fn(&AppNotificationActivatedEventArgs)>;

crate::ferro_thread_local! {
    /// The id the next notification is given.
    static NEXT_ID: Cell<u32> = const { Cell::new(1) };
    /// The handlers of `NotificationInvoked`, with the keys of their subscriptions.
    static INVOKED: RefCell<Vec<(u64, InvokedHandler)>> = const { RefCell::new(Vec::new()) };
    static NEXT_HANDLER: Cell<u64> = const { Cell::new(0) };
}

/// The notifications of the application (`AppNotificationManager.Default`): shows them through
/// the notification service the application registered, and reports clicks on them.
///
/// ```no_run
/// use ferroui_base::platform::notifications::{AppNotificationBuilder, AppNotificationManager};
///
/// if AppNotificationManager::is_supported() {
///     let manager = AppNotificationManager::default();
///     manager.register();
///     let mut notification = AppNotificationBuilder::new()
///         .add_text("Download finished")
///         .add_text("report.pdf is in your Downloads folder.")
///         .add_argument("action", "open")
///         .build_notification();
///     manager.show(&mut notification).ok();
/// }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AppNotificationManager;

impl AppNotificationManager {
    /// Whether notifications can be shown (`IsSupported`): the application registered the
    /// notification service of a platform that can show them.
    pub fn is_supported() -> bool {
        app_notification_manager_impl().is_some_and(|service| service.is_supported())
    }

    /// Asks the user to allow the notifications where the platform asks, and starts reporting
    /// clicks (`Register`). Nothing happens when notifications are not supported.
    pub fn register(&self) {
        if let Some(service) = supported() {
            service.register();
        }
    }

    /// Stops reporting clicks (`Unregister`).
    pub fn unregister(&self) {
        if let Some(service) = supported() {
            service.unregister();
        }
    }

    /// Shows `notification` and gives it its [`id`](AppNotification::id) (`Show`).
    pub fn show(&self, notification: &mut AppNotification) -> Result<(), AppNotificationError> {
        let service = supported().ok_or(AppNotificationError::NotSupported)?;
        if notification.id == 0 {
            notification.id = NEXT_ID.with(|next| {
                let id = next.get();
                next.set(id.checked_add(1).unwrap_or(1));
                id
            });
        }
        service.show(notification)
    }

    /// Removes the shown notification with `id` (`RemoveByIdAsync`).
    pub fn remove_by_id(&self, id: u32) {
        if let Some(service) = supported() {
            service.remove_by_id(id);
        }
    }

    /// Removes the shown notifications with `tag` (`RemoveByTagAsync`).
    pub fn remove_by_tag(&self, tag: &str) {
        if let Some(service) = supported() {
            service.remove_by_tag(tag, None);
        }
    }

    /// Removes the shown notifications with `tag` in `group` (`RemoveByTagAndGroupAsync`).
    pub fn remove_by_tag_and_group(&self, tag: &str, group: &str) {
        if let Some(service) = supported() {
            service.remove_by_tag(tag, Some(group));
        }
    }

    /// Removes the shown notifications of `group` (`RemoveByGroupAsync`).
    pub fn remove_by_group(&self, group: &str) {
        if let Some(service) = supported() {
            service.remove_by_group(group);
        }
    }

    /// Removes every shown notification of the application (`RemoveAllAsync`).
    pub fn remove_all(&self) {
        if let Some(service) = supported() {
            service.remove_all();
        }
    }

    /// Calls `handler` on the UI thread when the user clicks a notification or one of its
    /// buttons (`NotificationInvoked`), until the subscription is dropped.
    pub fn notification_invoked(
        &self,
        handler: impl Fn(&AppNotificationActivatedEventArgs) + 'static,
    ) -> Subscription {
        let key = NEXT_HANDLER.with(|next| {
            let key = next.get();
            next.set(key + 1);
            key
        });
        INVOKED.with(|handlers| handlers.borrow_mut().push((key, Rc::new(handler))));
        Subscription::new(Disposable::create(move || {
            // The thread can be ending when the last subscription is dropped.
            let _ = INVOKED.try_with(|handlers| handlers.borrow_mut().retain(|(k, _)| *k != key));
        }))
    }

    /// Calls the handlers of [`notification_invoked`](Self::notification_invoked) with `args`:
    /// what a platform calls, on the UI thread, when the user clicks a notification.
    pub fn raise_notification_invoked(args: &AppNotificationActivatedEventArgs) {
        // The handlers are copied out, so that a handler can subscribe or unsubscribe.
        let handlers: Vec<InvokedHandler> =
            INVOKED.with(|handlers| handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect());
        for handler in handlers {
            handler(args);
        }
    }
}

fn supported() -> Option<Rc<dyn IAppNotificationManagerImpl>> {
    app_notification_manager_impl().filter(|service| service.is_supported())
}
