//! Port of `TreeViewItem/ToggleIsExpandedOnDoubleTappedBehavior.cs`.

use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StaticType, StyledElementImpl, WeakRef};
use ferroui_controls::{Control, TreeViewItem};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::RefCell;

/// Toggles the `IsExpanded` property of the [`TreeViewItem`] parent of the associated control on
/// the `DoubleTapped` event.
#[repr(C)]
pub struct ToggleIsExpandedOnDoubleTappedBehavior {
    base: StyledElementBehaviorOfT,
    // The control the double tapped handler is attached to.
    double_tapped: RefCell<Option<(WeakRef<Control>, RoutedEventHandlerToken)>>,
}

ferro_class!(ToggleIsExpandedOnDoubleTappedBehavior: StyledElementBehaviorOfT);
ferro_class_info!(ToggleIsExpandedOnDoubleTappedBehavior { new: ToggleIsExpandedOnDoubleTappedBehavior::new });
ferroui_base::ferro_impl_classes!(ToggleIsExpandedOnDoubleTappedBehavior: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ToggleIsExpandedOnDoubleTappedBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            this.remove_double_tapped_handler();

            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                let token = associated_object.add_handler(
                    InputElement::double_tapped_event(),
                    move |_sender: &Interactive, _e: &TappedEventArgs| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.double_tapped();
                        }
                    },
                );
                *this.double_tapped.borrow_mut() = Some((associated_object.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_double_tapped_handler();
        }
    }
}

impl ToggleIsExpandedOnDoubleTappedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            double_tapped: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn remove_double_tapped_handler(&self) {
        let double_tapped = self.double_tapped.borrow_mut().take();
        if let Some((associated_object, token)) = double_tapped {
            if let Some(associated_object) = associated_object.upgrade() {
                associated_object.remove_handler(InputElement::double_tapped_event(), token);
            }
        }
    }

    fn double_tapped(&self) {
        let item = self
            .associated_object_as::<Control>()
            .and_then(|associated_object| associated_object.parent())
            .and_then(|parent| parent.cast::<TreeViewItem>());
        if let Some(item) = item {
            item.set_is_expanded(!item.is_expanded());
        }
    }
}
