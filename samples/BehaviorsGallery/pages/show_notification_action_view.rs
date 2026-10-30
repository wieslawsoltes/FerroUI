//! Port of `Views/Pages/ShowNotificationActionView` of the reference sample: the class of `pages/ShowNotificationActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::notifications::{INotification, Notification, NotificationType};
use ferroui_controls::{Control, UserControl};
use ferroui_xaml_interactions_custom::ShowNotificationAction;
use ferroui_xaml_interactivity::{Interaction, StyledElementTrigger};
use std::rc::Rc;

#[repr(C)]
pub struct ShowNotificationActionView {
    base: UserControl,
}

user_control_class!(ShowNotificationActionView);
ferro_class_info!(ShowNotificationActionView {
    new: ShowNotificationActionView::new
});
xaml_class!(
    ShowNotificationActionView,
    "/pages/ShowNotificationActionView.xaml"
);

impl ShowNotificationActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ShowNotificationActionView()`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the document of the reference declares the
    /// notification of the action (`<Notification Title=".." Message=".." />`), a type the
    /// framework does not declare to markup; the page sets it here.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        for behavior in Interaction::get_behaviors(&this).list().to_vec() {
            let Some(actions) = behavior
                .cast::<StyledElementTrigger>()
                .and_then(|trigger| trigger.actions())
            else {
                continue;
            };
            for action in actions.list().to_vec() {
                if let Some(action) = action.cast::<ShowNotificationAction>() {
                    let notification: Rc<dyn INotification> = Notification::new(
                        Some("Hello".to_string()),
                        Some("Custom notification".to_string()),
                        NotificationType::Information,
                        None,
                        None,
                        None,
                    );
                    action.set_notification(Some(notification));
                }
            }
        }
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ShowNotificationActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ShowNotificationActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ShowNotificationAction",
    reference: "Views/Pages/ShowNotificationActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ShowNotificationActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
