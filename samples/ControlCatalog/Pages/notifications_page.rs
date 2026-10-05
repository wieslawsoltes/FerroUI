//! Port of `Pages/NotificationsPage.xaml.cs`: the class of the document
//! `Pages/NotificationsPage.xaml`.

use crate::markup::xaml_class;
use crate::view_models::NotificationViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::notifications::{Notification, WindowNotificationManager};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentPage, ControlImpl, PageImpl, TopLevel};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct NotificationsPage {
    base: ContentPage,
    view_model: RefCell<Option<Rc<NotificationViewModel>>>,
}

ferro_class!(NotificationsPage: ContentPage);
ferro_impl_classes!(
    NotificationsPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(NotificationsPage {
    new: NotificationsPage::new,
    markup: {
        methods: [
            fn ShowNotification(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NotificationsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.show_notification(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NotificationsPage, "/Pages/NotificationsPage.xaml");

impl VisualImpl for NotificationsPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let top_level = TopLevel::get_top_level(Some(this)).expect("the page is attached to a top level");
        let view_model = this.view_model.borrow().clone().expect("the view model of the page");
        view_model.set_notification_manager(Some(WindowNotificationManager::with_host(Some(&top_level))));
    }
}

impl NotificationsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), view_model: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let view_model = NotificationViewModel::new();
        *this.view_model.borrow_mut() = Some(view_model.clone());

        this.set_data_context(Some(view_model as BoxedValue));
        this
    }

    fn control_notifications(&self) -> Ref<WindowNotificationManager> {
        self.get_control::<WindowNotificationManager>("ControlNotifications")
    }

    fn show_notification(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let control_notifications = self.control_notifications();
        // The notification is shown by the manager: its action holds the manager weakly.
        let weak = control_notifications.downgrade();
        let on_click: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(control_notifications) = weak.upgrade() {
                control_notifications.show_content(Rc::new(String::from("Notification clicked")));
            }
        });
        control_notifications.show(
            Notification::empty()
                .with_on_click(Some(on_click))
                .with_title(Some(String::from("Title")))
                .with_message(Some(String::from("Message"))),
        );
    }
}
