//! Port of `Focus/FocusTrapBehavior.cs`.

use crate::part_a::FocusNavigationHelper;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationDirection};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledProperty, Visual,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::Cell;

/// Keeps the keyboard focus inside the associated control: the Tab key (and Shift+Tab) moves the
/// focus from its last (first) tab stop back to its first (last) one, and into it from outside.
#[repr(C)]
pub struct FocusTrapBehavior {
    base: BehaviorOfT,
    // The handler added to the associated control (the reference removes its delegate).
    key_down_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(FocusTrapBehavior: BehaviorOfT);
ferro_class_info!(FocusTrapBehavior { new: FocusTrapBehavior::new });
ferroui_base::ferro_impl_classes!(FocusTrapBehavior: FerroObjectImpl);

ferro_properties! {
    impl FocusTrapBehavior {
        /// Identifies the `IsActive` property.
        pub fn is_active_property() -> StyledProperty<bool> {
            FerroProperty::register::<FocusTrapBehavior, _>("IsActive", true)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for FocusTrapBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                let handler = associated_object.add_handler(
                    InputElement::key_down_event(),
                    move |_sender: &Interactive, e: &KeyEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_key_down(e);
                        }
                    },
                );
                this.key_down_handler.set(Some(handler));
            }
        }

        fn on_detaching(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<Control>() {
                if let Some(handler) = this.key_down_handler.take() {
                    associated_object.remove_handler(InputElement::key_down_event(), handler);
                }
            }

            Self::parent_on_detaching(this);
        }
    }
}

impl FocusTrapBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: BehaviorOfT::construct(<Control as StaticType>::TYPE), key_down_handler: Cell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether the focus trap is active. This is a property of the object model.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }

    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value)
    }

    fn on_key_down(&self, e: &KeyEventArgs) {
        let Some(associated_object) = self.associated_object_as::<Control>().filter(|_| self.is_active()) else {
            return;
        };

        if e.key == Key::Tab {
            let direction = if e.key_modifiers.contains(KeyModifiers::SHIFT) {
                NavigationDirection::Previous
            } else {
                NavigationDirection::Next
            };

            let associated_visual: &Visual = &associated_object;
            // The focus manager of a top level is never missing here.
            let Some(top_level) = TopLevel::get_top_level(Some(associated_visual)) else {
                return;
            };
            let focus_manager = top_level.focus_manager();

            let current_focus = focus_manager.get_focused_element().and_then(|element| element.cast::<Control>());

            let Some(current_focus) =
                current_focus.filter(|current_focus| associated_object.is_visual_ancestor_of(current_focus))
            else {
                if let Some(boundary) = self.get_boundary_element(direction) {
                    boundary.focus();
                    e.set_handled(true);
                }
                return;
            };

            let top_level_visual: &Visual = &top_level;
            let current_element: &InputElement = &current_focus;
            let next = FocusNavigationHelper::find_adjacent(top_level_visual, Some(current_element), direction, false);

            if !next.is_some_and(|next| associated_object.is_visual_ancestor_of(&next)) {
                if let Some(wrap_target) = self.get_boundary_element(direction) {
                    wrap_target.focus();
                    e.set_handled(true);
                }
            }
        }
    }

    fn get_boundary_element(&self, direction: NavigationDirection) -> Option<Ref<Control>> {
        let associated_object = self.associated_object_as::<Control>()?;
        FocusNavigationHelper::find_boundary(&associated_object, direction).and_then(|element| element.cast::<Control>())
    }
}
