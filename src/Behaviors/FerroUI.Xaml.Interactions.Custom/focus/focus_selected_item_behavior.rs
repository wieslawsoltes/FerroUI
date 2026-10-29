//! Port of `Focus/FocusSelectedItemBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl, Ref,
    StaticType, StyledElementImpl,
};
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::ItemsControl;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Focuses the container of the selected item of the associated items control each time the
/// selection changes.
#[repr(C)]
pub struct FocusSelectedItemBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(FocusSelectedItemBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(FocusSelectedItemBehavior { new: FocusSelectedItemBehavior::new });
ferroui_base::ferro_impl_classes!(
    FocusSelectedItemBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for FocusSelectedItemBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            // Deviation (DEVIATIONS.md, Behaviors): the reference observes the selected item
            // property of a selecting items control on any items control; the selected item is a
            // direct property of the selecting items control, so an items control that is not
            // one has no selection to observe and the behavior does nothing.
            let Some(selecting_items_control) = this.associated_object_as::<SelectingItemsControl>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let source: &FerroObject = &selecting_items_control;
            FerroObjectExtensions::get_observable(source, SelectingItemsControl::selected_item_property())
                .subscribe_fn(move |selected_item: Option<BoxedValue>| {
                    let Some(item) = selected_item else {
                        return;
                    };

                    let behavior = behavior.clone();
                    Dispatcher::ui_thread().post_local(
                        move || {
                            let Some(associated_object) =
                                behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<ItemsControl>())
                            else {
                                return;
                            };
                            if let Some(container) = associated_object.container_from_item(&Some(item)) {
                                container.focus();
                            }
                        },
                        DispatcherPriority::DEFAULT,
                    );
                })
        }
    }
}

impl FocusSelectedItemBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<ItemsControl as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
