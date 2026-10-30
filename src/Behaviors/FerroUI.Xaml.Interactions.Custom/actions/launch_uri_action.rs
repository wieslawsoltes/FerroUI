//! Port of `Actions/LaunchUriAction.cs`.

use super::visual_of;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Launches an absolute URI with the launcher of the top level of the sender.
#[repr(C)]
pub struct LaunchUriAction {
    base: StyledElementAction,
}

ferro_class!(LaunchUriAction: StyledElementAction);
ferro_class_info!(LaunchUriAction { new: LaunchUriAction::new });
ferroui_base::ferro_impl_classes!(LaunchUriAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl LaunchUriAction {
        /// Identifies the `Uri` property.
        pub fn uri_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<LaunchUriAction, _>("Uri", None)
        }
    }
}

impl StyledElementActionImpl for LaunchUriAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(uri_string) = this.uri().filter(|uri| !uri.is_empty()) else {
            return Some(Rc::new(false));
        };

        let Ok(uri) = Uri::absolute(&uri_string) else {
            return Some(Rc::new(false));
        };

        let visual = visual_of(sender);
        if let Some(top_level) = visual.and_then(|visual| TopLevel::get_top_level(Some(&*visual))) {
            // Fire and forget
            let launch = top_level.launcher().launch_uri_async(&uri);
            drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
                let _ = launch.await;
            }));
            return Some(Rc::new(true));
        }

        Some(Rc::new(false))
    }
}

impl LaunchUriAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the URI to launch. This is a property of the object model.
    pub fn uri(&self) -> Option<String> {
        self.get_value(Self::uri_property())
    }

    pub fn set_uri(&self, value: Option<String>) {
        self.set_value(Self::uri_property(), value)
    }
}
