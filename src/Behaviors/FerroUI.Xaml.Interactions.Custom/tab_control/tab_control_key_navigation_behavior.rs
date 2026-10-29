//! Port of `TabControl/TabControlKeyNavigationBehavior.cs`.

use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::Orientation;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::TabControl;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Enables keyboard navigation for a [`TabControl`] using arrow keys.
#[repr(C)]
pub struct TabControlKeyNavigationBehavior {
    base: StyledElementBehaviorOfT,
    // The handler added to the associated object (the reference removes its delegate).
    key_down_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(TabControlKeyNavigationBehavior: StyledElementBehaviorOfT);
ferro_class_info!(TabControlKeyNavigationBehavior { new: TabControlKeyNavigationBehavior::new });
ferroui_base::ferro_impl_classes!(TabControlKeyNavigationBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl TabControlKeyNavigationBehavior {
        /// Identifies the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<TabControlKeyNavigationBehavior, _>("Orientation", Orientation::Horizontal)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TabControlKeyNavigationBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            if let Some(associated_object) = this.associated_object_as::<TabControl>() {
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler_with(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_key_down(e);
                        }
                    },
                    RoutingStrategies::TUNNEL,
                    false,
                );
                this.key_down_handler.set(Some(token));
            }
        }

        fn on_detaching(this: &Self) {
            if let Some(token) = this.key_down_handler.take() {
                if let Some(associated_object) = this.associated_object_as::<TabControl>() {
                    associated_object.remove_handler(InputElement::key_down_event(), token);
                }
            }

            Self::parent_on_detaching(this);
        }
    }
}

impl TabControlKeyNavigationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<TabControl as StaticType>::TYPE), key_down_handler: Cell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the orientation used for navigation. This is a property of the object model.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    fn on_key_down(&self, e: &KeyEventArgs) {
        if !self.is_enabled() {
            return;
        }
        let Some(associated_object) = self.associated_object_as::<TabControl>() else {
            return;
        };

        let (next_key, previous_key) = match self.orientation() {
            Orientation::Horizontal => (Key::Right, Key::Left),
            Orientation::Vertical => (Key::Down, Key::Up),
        };

        if e.key == next_key {
            let index = (associated_object.selected_index() + 1).min(associated_object.item_count() - 1);
            associated_object.set_selected_index(index);
            e.set_handled(true);
        } else if e.key == previous_key {
            let index = (associated_object.selected_index() - 1).max(0);
            associated_object.set_selected_index(index);
            e.set_handled(true);
        }
    }
}
