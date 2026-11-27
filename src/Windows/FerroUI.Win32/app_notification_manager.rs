//! The toast notifications of Windows (#1276).
//!
//! Not from upstream. `ToastNotificationManager` shows the notifications of
//! `AppNotificationManager` from their XML (`AppNotification::payload`): the texts, the images
//! Windows can load, the progress bar, the inputs, the buttons and the sound. A notification
//! has a tag and a group, so that it can be removed (a notification without a tag gets one from
//! its id). A click on the notification or on a button raises `NotificationInvoked` with its
//! arguments while the application runs (the `Activated` event of the toast); the values of the
//! inputs are not read, and an application that is not running is not started.
//!
//! An application that is not packaged needs an application user model id, which it sets with
//! `SetCurrentProcessExplicitAppUserModelID` and registers (a Start menu shortcut with the id, or
//! the `AppUserModelId` key of the registry): without one `is_supported` is `false`.

use crate::win32_platform::Win32Platform;
use crate::win_rt::{
    HStringInterop, IAgileObject, IAgileObjectImpl, IInspectable, IToastActivatedEventArgs, IToastActivatedHandler,
    IToastActivatedHandlerImpl, IToastNotification, IToastNotification2, IToastNotificationFactory,
    IToastNotificationManagerStatics, IToastNotificationManagerStatics2, IToastNotifier, IXmlDocument,
    IXmlDocumentIO, NativeWinRTMethods,
};
use ferroui_base::platform::notifications::{
    AppNotification, AppNotificationActivatedEventArgs, AppNotificationError, AppNotificationManager,
    IAppNotificationManagerImpl,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_microcom::{make_com_with, ComPtr, HResult, InterfaceEntry};
use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;

/// A notification Windows shows: its id, tag and group.
struct Entry {
    id: u32,
    tag: String,
    group: String,
}

/// The handler of `Activated` of every toast: reads the arguments on the thread Windows calls it
/// on and raises `NotificationInvoked` on the UI thread.
struct ActivatedHandler;

impl IToastActivatedHandlerImpl for ActivatedHandler {
    fn invoke(&self, _sender: Option<&IToastNotification>, args: Option<&IInspectable>) -> Result<(), HResult> {
        let Some(args) = args else { return Ok(()) };
        let args = ComPtr::from_ref(args).cast::<IToastActivatedEventArgs>()?;
        let argument = HStringInterop::from_handle(args.get_arguments()?, true).value().unwrap_or_default();
        let args = AppNotificationActivatedEventArgs::new(argument, BTreeMap::new());
        Dispatcher::ui_thread().post(
            move || AppNotificationManager::raise_notification_invoked(&args),
            DispatcherPriority::DEFAULT,
        );
        Ok(())
    }
}

impl IAgileObjectImpl for ActivatedHandler {}

/// The notification service of Windows: the toast notifications of the application. Registered
/// by `use_platform_app_notifications()` of the desktop platform.
pub struct Win32AppNotificationManager {
    app_user_model_id: Option<String>,
    notifier: OnceCell<Option<ComPtr<IToastNotifier>>>,
    activated: RefCell<Option<ComPtr<IToastActivatedHandler>>>,
    entries: RefCell<Vec<Entry>>,
}

impl Win32AppNotificationManager {
    /// The service of the application user model id of the process.
    pub fn new() -> Self {
        Self::with_app_user_model_id(current_app_user_model_id())
    }

    /// The service of the application with `app_user_model_id`.
    pub fn with_app_user_model_id(app_user_model_id: Option<String>) -> Self {
        Self {
            app_user_model_id,
            notifier: OnceCell::new(),
            activated: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
        }
    }

    /// Registers the toast notifications of the process as the notification service of the
    /// application.
    pub fn register() {
        ferroui_base::platform::notifications::register_app_notification_manager_impl(std::rc::Rc::new(Self::new()));
    }

    fn notifier(&self) -> Option<ComPtr<IToastNotifier>> {
        self.notifier
            .get_or_init(|| {
                if Win32Platform::windows_version().major < 10 {
                    return None;
                }
                let id = self.app_user_model_id.as_deref()?;
                let statics = NativeWinRTMethods::create_activation_factory::<IToastNotificationManagerStatics>(
                    "Windows.UI.Notifications.ToastNotificationManager",
                )
                .ok()?;
                let id = HStringInterop::new(Some(id)).ok()?;
                statics.create_toast_notifier_with_id(id.handle()).ok().flatten()
            })
            .clone()
    }

    fn toast(notification: &AppNotification) -> Result<ComPtr<IToastNotification>, HResult> {
        let document = NativeWinRTMethods::create_instance::<IXmlDocumentIO>("Windows.Data.Xml.Dom.XmlDocument")?;
        let xml = HStringInterop::new(Some(&notification.payload()))?;
        document.load_xml(xml.handle())?;
        let document = document.cast::<IXmlDocument>()?;
        let factory = NativeWinRTMethods::create_activation_factory::<IToastNotificationFactory>(
            "Windows.UI.Notifications.ToastNotification",
        )?;
        let toast = factory.create_toast_notification(Some(&document))?.ok_or(HResult::POINTER)?;
        let toast2 = toast.cast::<IToastNotification2>()?;
        let (tag, group) = Self::tag_and_group(notification);
        toast2.set_tag(HStringInterop::new(Some(&tag))?.handle())?;
        if !group.is_empty() {
            toast2.set_group(HStringInterop::new(Some(group))?.handle())?;
        }
        Ok(toast)
    }

    /// The tag Windows keeps a notification by: its own, or one made from its id.
    fn tag_and_group(notification: &AppNotification) -> (String, &str) {
        let tag = if notification.tag().is_empty() {
            format!("ferroui.{}", notification.id())
        } else {
            notification.tag().to_string()
        };
        (tag, notification.group())
    }

    fn remove(&self, matches: impl Fn(&Entry) -> bool) {
        let Some(id) = self.app_user_model_id.as_deref() else { return };
        let Some(history) = history() else { return };
        let Ok(application_id) = HStringInterop::new(Some(id)) else { return };
        self.entries.borrow_mut().retain(|entry| {
            if !matches(entry) {
                return true;
            }
            let (tag, group) = (HStringInterop::new(Some(&entry.tag)), HStringInterop::new(Some(&entry.group)));
            if let (Ok(tag), Ok(group)) = (tag, group) {
                let _ = history.remove_grouped_tag_with_id(tag.handle(), group.handle(), application_id.handle());
            }
            false
        });
    }
}

impl Default for Win32AppNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn history() -> Option<ComPtr<crate::win_rt::IToastNotificationHistory>> {
    let statics = NativeWinRTMethods::create_activation_factory::<IToastNotificationManagerStatics2>(
        "Windows.UI.Notifications.ToastNotificationManager",
    )
    .ok()?;
    statics.get_history().ok().flatten()
}

/// The application user model id the process set; `None` when it set none.
fn current_app_user_model_id() -> Option<String> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::GetCurrentProcessExplicitAppUserModelID;
    let mut id: *mut u16 = std::ptr::null_mut();
    // SAFETY: the function writes a string it allocated with the task allocator, which is freed
    // after it was copied.
    unsafe {
        if GetCurrentProcessExplicitAppUserModelID(&mut id) < 0 || id.is_null() {
            return None;
        }
        let length = (0..).take_while(|&index| *id.add(index) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(id, length));
        CoTaskMemFree(id.cast());
        (!text.is_empty()).then_some(text)
    }
}

impl IAppNotificationManagerImpl for Win32AppNotificationManager {
    fn is_supported(&self) -> bool {
        self.notifier().is_some()
    }

    fn register(&self) {
        let mut activated = self.activated.borrow_mut();
        if activated.is_none() {
            *activated = Some(make_com_with(ActivatedHandler, [InterfaceEntry::of::<IAgileObject>()]));
        }
    }

    fn unregister(&self) {
        self.activated.borrow_mut().take();
    }

    fn show(&self, notification: &AppNotification) -> Result<(), AppNotificationError> {
        let notifier = self.notifier().ok_or(AppNotificationError::NotSupported)?;
        let failed = |error: HResult| AppNotificationError::Failed(format!("{error:?}"));
        let toast = Self::toast(notification).map_err(failed)?;
        if let Some(handler) = self.activated.borrow().as_ref() {
            toast.add_activated(Some(handler)).map_err(failed)?;
        }
        notifier.show(Some(&toast)).map_err(failed)?;
        let (tag, group) = Self::tag_and_group(notification);
        let mut entries = self.entries.borrow_mut();
        entries.retain(|entry| !(entry.tag == tag && entry.group == group) && entry.id != notification.id());
        entries.push(Entry { id: notification.id(), tag, group: group.to_string() });
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
        let (Some(id), Some(history)) = (self.app_user_model_id.as_deref(), history()) else { return };
        if let Ok(application_id) = HStringInterop::new(Some(id)) {
            let _ = history.clear_with_id(application_id.handle());
        }
    }
}
