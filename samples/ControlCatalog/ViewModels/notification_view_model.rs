//! Port of `ViewModels/NotificationViewModel.cs`.

use ferroui_base::input::ICommand;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::notifications::{Notification, NotificationType, WindowNotificationManager};
use mini_mvvm::MiniCommand;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The view model of the notifications page, and the content of its custom
/// notification.
pub struct NotificationViewModel {
    notification_manager: RefCell<Option<Ref<WindowNotificationManager>>>,
    title: RefCell<Option<String>>,
    message: RefCell<Option<String>>,
    yes_command: Rc<MiniCommand>,
    no_command: Rc<MiniCommand>,
    show_custom_managed_notification_command: Rc<MiniCommand>,
    show_managed_notification_command: Rc<MiniCommand>,
}

impl PartialEq for NotificationViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl NotificationViewModel {
    pub fn new() -> Rc<NotificationViewModel> {
        Rc::new_cyclic(|this: &Weak<NotificationViewModel>| {
            let show_custom_managed_notification_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    if let Some(notification_manager) = this.notification_manager() {
                        let content = NotificationViewModel::new();
                        content.set_title(Some(String::from("Hey There!")));
                        content.set_message(Some(String::from(
                            "Did you know that FerroUI now supports Custom In-Window Notifications?",
                        )));
                        content.set_notification_manager(Some(notification_manager.clone()));
                        notification_manager.show_with(
                            content as BoxedValue,
                            NotificationType::Warning,
                            None,
                            None,
                            None,
                            None,
                        );
                    }
                })
            };

            let show_managed_notification_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    if let Some(notification_manager) = this.notification_manager() {
                        notification_manager.show(Notification::new(
                            Some(String::from("Welcome")),
                            Some(String::from("FerroUI now supports Notifications.")),
                            NotificationType::Information,
                            None,
                            None,
                            None,
                        ));
                    }
                })
            };

            let yes_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    if let Some(notification_manager) = this.notification_manager() {
                        notification_manager.show(Notification::new(
                            Some(String::from("FerroUI Notifications")),
                            Some(String::from("Start adding notifications to your app today.")),
                            NotificationType::Information,
                            None,
                            None,
                            None,
                        ));
                    }
                })
            };

            let no_command = {
                let this = this.clone();
                MiniCommand::create(move || {
                    let Some(this) = this.upgrade() else { return };
                    if let Some(notification_manager) = this.notification_manager() {
                        notification_manager.show(Notification::new(
                            Some(String::from("FerroUI Notifications")),
                            Some(String::from(
                                "Start adding notifications to your app today. To find out more visit...",
                            )),
                            NotificationType::Information,
                            None,
                            None,
                            None,
                        ));
                    }
                })
            };

            Self {
                notification_manager: RefCell::new(None),
                title: RefCell::new(None),
                message: RefCell::new(None),
                yes_command,
                no_command,
                show_custom_managed_notification_command,
                show_managed_notification_command,
            }
        })
    }

    pub fn notification_manager(&self) -> Option<Ref<WindowNotificationManager>> {
        self.notification_manager.borrow().clone()
    }

    pub fn set_notification_manager(&self, value: Option<Ref<WindowNotificationManager>>) {
        *self.notification_manager.borrow_mut() = value;
    }

    pub fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    pub fn set_title(&self, value: Option<String>) {
        *self.title.borrow_mut() = value;
    }

    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }

    pub fn set_message(&self, value: Option<String>) {
        *self.message.borrow_mut() = value;
    }

    pub fn yes_command(&self) -> Rc<MiniCommand> {
        self.yes_command.clone()
    }

    pub fn no_command(&self) -> Rc<MiniCommand> {
        self.no_command.clone()
    }

    pub fn show_custom_managed_notification_command(&self) -> Rc<MiniCommand> {
        self.show_custom_managed_notification_command.clone()
    }

    pub fn show_managed_notification_command(&self) -> Rc<MiniCommand> {
        self.show_managed_notification_command.clone()
    }
}

ferro_markup_type!(class NotificationViewModel {
    this: Rc<NotificationViewModel>,
    handles: [NotificationViewModel, Rc<NotificationViewModel>, Option<Rc<NotificationViewModel>>],
    constructors: [() => NotificationViewModel::new],
    properties: [
        NotificationManager: Option<Ref<WindowNotificationManager>> {
            get: |this: &Rc<NotificationViewModel>| this.notification_manager(),
            set: |this: &Rc<NotificationViewModel>, value: Option<Ref<WindowNotificationManager>>| {
                this.set_notification_manager(value)
            }
        },
        Title: Option<String> {
            get: |this: &Rc<NotificationViewModel>| this.title(),
            set: |this: &Rc<NotificationViewModel>, value: Option<String>| this.set_title(value)
        },
        Message: Option<String> {
            get: |this: &Rc<NotificationViewModel>| this.message(),
            set: |this: &Rc<NotificationViewModel>, value: Option<String>| this.set_message(value)
        },
        YesCommand: Rc<dyn ICommand> { get: |this: &Rc<NotificationViewModel>| this.yes_command().as_command() },
        NoCommand: Rc<dyn ICommand> { get: |this: &Rc<NotificationViewModel>| this.no_command().as_command() },
        ShowCustomManagedNotificationCommand: Rc<dyn ICommand> {
            get: |this: &Rc<NotificationViewModel>| this.show_custom_managed_notification_command().as_command()
        },
        ShowManagedNotificationCommand: Rc<dyn ICommand> {
            get: |this: &Rc<NotificationViewModel>| this.show_managed_notification_command().as_command()
        },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_new_view_model_has_no_manager_title_or_message() {
        let view_model = NotificationViewModel::new();
        assert!(view_model.notification_manager().is_none());
        assert_eq!(None, view_model.title());
        assert_eq!(None, view_model.message());
    }

    #[test]
    fn the_commands_do_nothing_without_a_manager() {
        let view_model = NotificationViewModel::new();
        view_model.show_managed_notification_command().as_command().execute(None);
        view_model.show_custom_managed_notification_command().as_command().execute(None);
        view_model.yes_command().as_command().execute(None);
        view_model.no_command().as_command().execute(None);
    }

    #[test]
    fn title_and_message_are_stored() {
        let view_model = NotificationViewModel::new();
        view_model.set_title(Some(String::from("Title")));
        view_model.set_message(Some(String::from("Message")));
        assert_eq!(Some(String::from("Title")), view_model.title());
        assert_eq!(Some(String::from("Message")), view_model.message());
    }
}
