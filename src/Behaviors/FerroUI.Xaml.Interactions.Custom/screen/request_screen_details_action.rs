//! Port of `Screen/RequestScreenDetailsAction.cs`.

use crate::ScreensRef;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that requests extended screen information from the screens of a top level
/// ([`Screens::request_screen_details`](ferroui_controls::Screens::request_screen_details)).
///
/// The request runs in a job of the dispatcher; the action returns `true` when the sender is a
/// visual.
#[repr(C)]
pub struct RequestScreenDetailsAction {
    base: StyledElementAction,
}

ferro_class!(RequestScreenDetailsAction: StyledElementAction);
ferro_class_info!(RequestScreenDetailsAction {
    new: RequestScreenDetailsAction::new,
    markup: {
        property_attributes: [
            Screens: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RequestScreenDetailsAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl RequestScreenDetailsAction {
        /// Identifies the `Screens` property.
        pub fn screens_property() -> StyledProperty<Option<ScreensRef>> {
            FerroProperty::register::<RequestScreenDetailsAction, _>("Screens", None)
        }
    }
}

impl StyledElementActionImpl for RequestScreenDetailsAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(visual) = sender.and_then(object_of).and_then(|sender| sender.cast::<Visual>()) else {
            return Some(Rc::new(false));
        };

        let action: Ref<RequestScreenDetailsAction> = this.to_ref();
        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            action.request_async(&visual).await;
        }));

        Some(Rc::new(true))
    }
}

impl RequestScreenDetailsAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the screens used to request screen details. This is a property of the object
    /// model. If not set, the screens of the top level of the sender are used.
    pub fn screens(&self) -> Option<ScreensRef> {
        self.get_value(Self::screens_property())
    }

    pub fn set_screens(&self, value: Option<ScreensRef>) {
        self.set_value(Self::screens_property(), value)
    }

    async fn request_async(&self, visual: &Visual) {
        if !self.is_enabled() {
            return;
        }

        let screens = self
            .screens()
            .map(|screens| screens.0)
            .or_else(|| TopLevel::get_top_level(Some(visual)).and_then(|top_level| top_level.screens()));
        if let Some(screens) = screens {
            // The result is ignored, as the reference ignores the failures of the request.
            let _ = screens.request_screen_details().await;
        }
    }
}
