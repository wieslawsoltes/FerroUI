//! The notifications of iOS through the UserNotifications framework (#1276).
//!
//! Not from upstream. `UNUserNotificationCenter` shows the notifications of
//! `AppNotificationManager`: the first text is the title, the attribution text the subtitle and
//! the other texts the body; the group is the thread of the notification; the buttons are the
//! actions of its category, and with a text box the first button is a text input action. The
//! arguments are kept in the notification's user info, so that a click reports them even after
//! the application was started again. Images are not attached (the framework moves an attached
//! file into its own store), and combo boxes and progress bars are not shown.
//!
//! The same service as the one of the macOS platform (`ferroui-native`), which the two platform
//! crates do not share a crate for.

use block2::RcBlock;
use ferroui_base::platform::notifications::{
    AppNotification, AppNotificationActivatedEventArgs, AppNotificationError, AppNotificationManager,
    IAppNotificationManagerImpl,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, NSObject};
use objc2::{class, define_class, msg_send, sel, AnyThread};
use objc2_foundation::{NSArray, NSBundle, NSDictionary, NSError, NSObjectProtocol, NSSet, NSString};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[link(name = "UserNotifications", kind = "framework")]
extern "C" {}

/// `UNAuthorizationOptionBadge | UNAuthorizationOptionSound | UNAuthorizationOptionAlert`.
const AUTHORIZATION_OPTIONS: usize = (1 << 0) | (1 << 1) | (1 << 2);
/// `UNNotificationPresentationOptionSound | List | Banner`: shown while the application is in
/// front, as when it is not.
const PRESENTATION_OPTIONS: usize = (1 << 1) | (1 << 3) | (1 << 4);
/// `UNNotificationActionOptionForeground`: an action brings the application to the front.
const ACTION_OPTION_FOREGROUND: usize = 1 << 2;
/// The action of a click on the notification itself.
const DEFAULT_ACTION: &str = "com.apple.UNNotificationDefaultActionIdentifier";
/// The key of the arguments of the notification in its user info.
const ARGUMENTS_KEY: &str = "ferroui.arguments";
/// The key of the id of the text box of the text input action.
const INPUT_KEY: &str = "ferroui.input";
/// The prefix of the identifier of the action of a button, and of its arguments' key.
const BUTTON_PREFIX: &str = "ferroui.button.";

define_class!(
    // SAFETY: `NSObject` has no requirements on a subclass that only adds the methods of the
    // delegate protocol of the notification center, and the class does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "FerroUIIosAppNotificationDelegate"]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    impl NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(&self, _center: &AnyObject, _notification: &AnyObject, handler: &block2::Block<dyn Fn(usize)>) {
            handler.call((PRESENTATION_OPTIONS,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(&self, _center: &AnyObject, response: &AnyObject, handler: &block2::Block<dyn Fn()>) {
            if let Some(args) = activation(response) {
                // The center may call its delegate on another thread: the handlers run on the UI
                // thread.
                Dispatcher::ui_thread().post(
                    move || AppNotificationManager::raise_notification_invoked(&args),
                    DispatcherPriority::DEFAULT,
                );
            }
            handler.call(());
        }
    }
);

impl NotificationDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        // SAFETY: `init` of `NSObject` on the object that was just allocated.
        unsafe { msg_send![super(this), init] }
    }
}

/// What the response to a notification reports: the arguments of what was clicked and the text
/// entered in its text input action.
fn activation(response: &AnyObject) -> Option<AppNotificationActivatedEventArgs> {
    // SAFETY: `response` is a `UNNotificationResponse`: its notification, request, content and
    // user info are objects, the user info a dictionary of the strings the manager put in it.
    unsafe {
        let action: Retained<NSString> = msg_send![response, actionIdentifier];
        let notification: Retained<AnyObject> = msg_send![response, notification];
        let request: Retained<AnyObject> = msg_send![&notification, request];
        let content: Retained<AnyObject> = msg_send![&request, content];
        let user_info: Retained<NSDictionary<AnyObject, AnyObject>> = msg_send![&content, userInfo];
        let value = |key: &str| -> Option<String> {
            let key = NSString::from_str(key);
            let value: Option<Retained<AnyObject>> = msg_send![&user_info, objectForKey: &*key];
            value?.downcast::<NSString>().ok().map(|value| value.to_string())
        };
        let action = action.to_string();
        let argument = if action == DEFAULT_ACTION {
            value(ARGUMENTS_KEY)?
        } else if action.starts_with(BUTTON_PREFIX) {
            value(&action)?
        } else {
            return None;
        };
        let mut user_input = BTreeMap::new();
        let has_text: bool = msg_send![response, respondsToSelector: sel!(userText)];
        if has_text {
            let text: Retained<NSString> = msg_send![response, userText];
            if let Some(input) = value(INPUT_KEY) {
                user_input.insert(input, text.to_string());
            }
        }
        Some(AppNotificationActivatedEventArgs::new(argument, user_input))
    }
}

/// A notification the center shows: its id, tag and group and the identifier of its request.
struct Entry {
    id: u32,
    tag: String,
    group: String,
    identifier: String,
}

/// The notification service of iOS: `UNUserNotificationCenter`. Registered by
/// `use_platform_app_notifications()` of the iOS platform.
pub struct IosAppNotificationManager {
    delegate: RefCell<Option<Retained<NotificationDelegate>>>,
    /// The categories of the buttons of the notifications shown, by their identifiers: the
    /// center replaces its categories as a whole.
    categories: RefCell<Vec<(String, Retained<AnyObject>)>>,
    entries: RefCell<Vec<Entry>>,
}

impl IosAppNotificationManager {
    /// The service of the notification center of the application.
    pub fn new() -> Self {
        Self { delegate: RefCell::new(None), categories: RefCell::new(Vec::new()), entries: RefCell::new(Vec::new()) }
    }

    /// Registers the notification center as the notification service of the application.
    pub fn register() {
        ferroui_base::platform::notifications::register_app_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn center() -> Retained<AnyObject> {
        // SAFETY: the class is in the framework the crate links; `is_supported` checked that the
        // application has the bundle the center needs.
        unsafe { msg_send![class!(UNUserNotificationCenter), currentNotificationCenter] }
    }

    /// The identifier of the category of the buttons of `notification`, registered with the
    /// center when it is new; `None` for a notification without buttons.
    fn category(&self, notification: &AppNotification) -> Option<String> {
        if notification.buttons().is_empty() {
            return None;
        }
        let text_box = notification.text_boxes().first();
        let mut identifier = String::from("ferroui.category");
        for button in notification.buttons() {
            identifier.push('.');
            identifier.push_str(&button.content().replace('.', "_"));
        }
        if let Some(text_box) = text_box {
            identifier.push_str(".input.");
            identifier.push_str(&text_box.placeholder_text().replace('.', "_"));
        }
        if self.categories.borrow().iter().any(|(known, _)| *known == identifier) {
            return Some(identifier);
        }
        // SAFETY: the classes are in the framework; the arguments are the strings, arrays and
        // options their constructors take.
        unsafe {
            let mut actions = Vec::new();
            for (index, button) in notification.buttons().iter().enumerate() {
                let action_id = NSString::from_str(&format!("{BUTTON_PREFIX}{index}"));
                let title = NSString::from_str(button.content());
                let action: Retained<AnyObject> = match (index, text_box) {
                    (0, Some(text_box)) => {
                        let placeholder = NSString::from_str(text_box.placeholder_text());
                        msg_send![
                            class!(UNTextInputNotificationAction),
                            actionWithIdentifier: &*action_id,
                            title: &*title,
                            options: ACTION_OPTION_FOREGROUND,
                            textInputButtonTitle: &*title,
                            textInputPlaceholder: &*placeholder
                        ]
                    }
                    _ => msg_send![
                        class!(UNNotificationAction),
                        actionWithIdentifier: &*action_id,
                        title: &*title,
                        options: ACTION_OPTION_FOREGROUND
                    ],
                };
                actions.push(action);
            }
            let actions = NSArray::from_retained_slice(&actions);
            let intents = NSArray::<NSString>::new();
            let category_id = NSString::from_str(&identifier);
            let category: Retained<AnyObject> = msg_send![
                class!(UNNotificationCategory),
                categoryWithIdentifier: &*category_id,
                actions: &*actions,
                intentIdentifiers: &*intents,
                options: 0usize
            ];
            self.categories.borrow_mut().push((identifier.clone(), category));
            let categories: Vec<Retained<AnyObject>> =
                self.categories.borrow().iter().map(|(_, category)| category.clone()).collect();
            let categories = NSSet::from_retained_slice(&categories);
            let _: () = msg_send![&Self::center(), setNotificationCategories: &*categories];
        }
        Some(identifier)
    }

    fn remove(&self, matches: impl Fn(&Entry) -> bool) {
        let mut identifiers = Vec::new();
        self.entries.borrow_mut().retain(|entry| {
            if matches(entry) {
                identifiers.push(NSString::from_str(&entry.identifier));
                false
            } else {
                true
            }
        });
        if identifiers.is_empty() {
            return;
        }
        let identifiers = NSArray::from_retained_slice(&identifiers);
        let center = Self::center();
        // SAFETY: both methods take an array of the identifiers of requests.
        unsafe {
            let _: () = msg_send![&center, removePendingNotificationRequestsWithIdentifiers: &*identifiers];
            let _: () = msg_send![&center, removeDeliveredNotificationsWithIdentifiers: &*identifiers];
        }
    }
}

impl Default for IosAppNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IAppNotificationManagerImpl for IosAppNotificationManager {
    fn is_supported(&self) -> bool {
        AnyClass::get(c"UNUserNotificationCenter").is_some()
            && NSBundle::mainBundle().bundleIdentifier().is_some()
    }

    fn register(&self) {
        let center = Self::center();
        let mut delegate = self.delegate.borrow_mut();
        let delegate = delegate.get_or_insert_with(NotificationDelegate::new);
        let completion = RcBlock::new(|_granted: Bool, _error: *mut NSError| {});
        // SAFETY: the center keeps its delegate weakly, the manager keeps it; the completion
        // handler is a block of the type the method takes.
        unsafe {
            let _: () = msg_send![&center, setDelegate: &**delegate];
            let _: () = msg_send![
                &center,
                requestAuthorizationWithOptions: AUTHORIZATION_OPTIONS,
                completionHandler: &*completion
            ];
        }
    }

    fn unregister(&self) {
        if let Some(_delegate) = self.delegate.borrow_mut().take() {
            // SAFETY: a nil delegate.
            unsafe {
                let _: () = msg_send![&Self::center(), setDelegate: Option::<&AnyObject>::None];
            }
        }
    }

    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError> {
        if !self.is_supported() {
            return Err(AppNotificationError::NotSupported);
        }
        // A notification with the tag and group of a shown one replaces it: the same identifier.
        let identifier = if notification.tag().is_empty() {
            format!("ferroui.id.{}", notification.id())
        } else {
            format!("ferroui.tag.{}.{}", notification.group(), notification.tag())
        };
        let category = self.category(notification);
        let mut keys = vec![NSString::from_str(ARGUMENTS_KEY)];
        let mut values = vec![NSString::from_str(&notification.argument_string())];
        for (index, button) in notification.buttons().iter().enumerate() {
            keys.push(NSString::from_str(&format!("{BUTTON_PREFIX}{index}")));
            values.push(NSString::from_str(&button.argument_string()));
        }
        if let Some(text_box) = notification.text_boxes().first() {
            keys.push(NSString::from_str(INPUT_KEY));
            values.push(NSString::from_str(text_box.id()));
        }
        let keys: Vec<&NSString> = keys.iter().map(|key| &**key).collect();
        let values: Vec<&NSString> = values.iter().map(|value| &**value).collect();
        let user_info = NSDictionary::from_slices(&keys, &values);
        let mut body = notification.body();
        if let Some(status) = notification.progress_bar().map(|progress| progress.status()).filter(|s| !s.is_empty()) {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(status);
        }
        // SAFETY: the classes are in the framework; the setters take the strings, the
        // dictionary and the sound the content keeps.
        unsafe {
            let content: Retained<AnyObject> = msg_send![class!(UNMutableNotificationContent), new];
            let _: () = msg_send![&content, setTitle: &*NSString::from_str(notification.title())];
            let _: () = msg_send![&content, setBody: &*NSString::from_str(&body)];
            if let Some(attribution) = notification.attribution_text() {
                let _: () = msg_send![&content, setSubtitle: &*NSString::from_str(attribution)];
            }
            if !notification.group().is_empty() {
                let _: () = msg_send![&content, setThreadIdentifier: &*NSString::from_str(notification.group())];
            }
            if let Some(category) = &category {
                let _: () = msg_send![&content, setCategoryIdentifier: &*NSString::from_str(category)];
            }
            let _: () = msg_send![&content, setUserInfo: &*user_info];
            if !notification.is_audio_muted() {
                let sound: Retained<AnyObject> = msg_send![class!(UNNotificationSound), defaultSound];
                let _: () = msg_send![&content, setSound: &*sound];
            }
            let request: Retained<AnyObject> = msg_send![
                class!(UNNotificationRequest),
                requestWithIdentifier: &*NSString::from_str(&identifier),
                content: &*content,
                trigger: Option::<&AnyObject>::None
            ];
            let _: () = msg_send![
                &Self::center(),
                addNotificationRequest: &*request,
                withCompletionHandler: Option::<&block2::Block<dyn Fn(*mut NSError)>>::None
            ];
        }
        let mut entries = self.entries.borrow_mut();
        entries.retain(|entry| entry.identifier != identifier && entry.id != notification.id());
        entries.push(Entry {
            id: notification.id(),
            tag: notification.tag().to_string(),
            group: notification.group().to_string(),
            identifier,
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
        // SAFETY: methods without arguments.
        unsafe {
            let center = Self::center();
            let _: () = msg_send![&center, removeAllPendingNotificationRequests];
            let _: () = msg_send![&center, removeAllDeliveredNotifications];
        }
    }
}
