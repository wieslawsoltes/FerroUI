//! Port of `Screen/ActiveScreenBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObjectExtensions,
    FerroObjectImpl, FerroProperty, Ref, Size, StaticType, StyledElementImpl,
};
use ferroui_controls::platform::Screen;
use ferroui_controls::{PixelPointEventArgs, TopLevel, Window};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that exposes the screen containing the associated [`TopLevel`].
#[repr(C)]
pub struct ActiveScreenBehavior {
    base: AttachedToVisualTreeBehavior,
    active_screen: RefCell<Option<Rc<Screen>>>,
}

ferro_class!(ActiveScreenBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ActiveScreenBehavior { new: ActiveScreenBehavior::new });
ferroui_base::ferro_impl_classes!(
    ActiveScreenBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ActiveScreenBehavior {
        /// Identifies the `ActiveScreen` property.
        pub fn active_screen_property() -> DirectProperty<ActiveScreenBehavior, Option<Rc<Screen>>> {
            FerroProperty::register_direct::<ActiveScreenBehavior, _>(
                "ActiveScreen",
                |behavior| behavior.active_screen(),
                None,
                None,
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ActiveScreenBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(top_level) = this.associated_object_as::<TopLevel>() else {
                return DisposableAction::empty();
            };

            this.update();

            let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();

            if let Some(window) = top_level.cast::<Window>() {
                let behavior = this.to_ref().downgrade();
                subscriptions.push(window.position_changed(move |_e: &PixelPointEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.update();
                    }
                }));
            }

            // Deviation (DEVIATIONS.md, Behaviors): the reference never ends its subscription to
            // the client size; here it ends with the other subscriptions when the top level
            // leaves the visual tree or the behavior is detached.
            let behavior = this.to_ref().downgrade();
            subscriptions.push(FerroObjectExtensions::get_observable(&*top_level as &ferroui_base::FerroObject, TopLevel::client_size_property()).subscribe_fn(
                move |_size: Size| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.update();
                    }
                },
            ));

            if let Some(screens) = top_level.screens() {
                let behavior = this.to_ref().downgrade();
                subscriptions.push(screens.changed(move || {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.update();
                    }
                }));
            }

            DisposableAction::create(move || {
                for subscription in &subscriptions {
                    subscription.dispose();
                }
            })
        }
    }
}

impl ActiveScreenBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeBehavior::construct(<TopLevel as StaticType>::TYPE),
            active_screen: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the active screen of the associated [`TopLevel`]. This is a property of the object
    /// model.
    pub fn active_screen(&self) -> Option<Rc<Screen>> {
        self.active_screen.borrow().clone()
    }

    fn set_active_screen(&self, value: Option<Rc<Screen>>) {
        self.set_and_raise(Self::active_screen_property(), &self.active_screen, value);
    }

    fn update(&self) {
        let active_screen = self
            .associated_object_as::<TopLevel>()
            .and_then(|top_level| top_level.screens().and_then(|screens| screens.screen_from_top_level(&top_level)));
        self.set_active_screen(active_screen);
    }
}
