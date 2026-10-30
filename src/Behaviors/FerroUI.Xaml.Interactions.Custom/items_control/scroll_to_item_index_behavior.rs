//! Port of `ItemsControl/ScrollToItemIndexBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::{IDisposable, IObservable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ItemsControl;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Scrolls the associated [`ItemsControl`] to a given item index.
///
/// While the items control is in the visual tree, every index the
/// [`item_index`](Self::item_index) observable produces is scrolled into view.
#[repr(C)]
pub struct ScrollToItemIndexBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ScrollToItemIndexBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ScrollToItemIndexBehavior { new: ScrollToItemIndexBehavior::new });
ferroui_base::ferro_impl_classes!(
    ScrollToItemIndexBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ScrollToItemIndexBehavior {
        /// Identifies the `ItemIndex` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the observable (`IObservable<int>` of the
        /// reference) is held as an untyped value, an `Rc<dyn IObservable<i32>>`, as the
        /// observable sources of the other behaviors of the port; another value is not an
        /// observable.
        pub fn item_index_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ScrollToItemIndexBehavior, _>("ItemIndex", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ScrollToItemIndexBehavior {
        /// Subscribes to the [`item_index`](Self::item_index) observable and scrolls to incoming
        /// indexes.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let behavior = this.to_ref().downgrade();

            let disposable = this.item_index().and_then(|item_index| {
                let observable = item_index.downcast_ref::<Rc<dyn IObservable<i32>>>()?;
                Some(observable.subscribe_fn(move |index: i32| {
                    if let Some(items_control) =
                        behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<ItemsControl>())
                    {
                        items_control.scroll_into_view(index);
                    }
                }))
            });

            disposable.unwrap_or_else(DisposableAction::empty)
        }
    }
}

impl ScrollToItemIndexBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<ItemsControl as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the observable that produces item indexes to scroll into view.
    pub fn item_index(&self) -> Option<BoxedValue> {
        self.get_value(Self::item_index_property())
    }

    pub fn set_item_index(&self, value: Option<BoxedValue>) {
        self.set_value(Self::item_index_property(), value)
    }
}
