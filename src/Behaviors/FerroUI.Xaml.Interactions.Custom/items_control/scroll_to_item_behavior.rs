//! Port of `ItemsControl/ScrollToItemBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::{IDisposable, IObservable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ItemsControl;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Scrolls the associated [`ItemsControl`] to a specific item.
///
/// While the items control is in the visual tree, every item the [`item`](Self::item) observable
/// produces is scrolled into view.
#[repr(C)]
pub struct ScrollToItemBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ScrollToItemBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ScrollToItemBehavior { new: ScrollToItemBehavior::new });
ferroui_base::ferro_impl_classes!(
    ScrollToItemBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ScrollToItemBehavior {
        /// Identifies the `Item` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the observable (`IObservable<object>` of the
        /// reference) is held as an untyped value: an `Rc<dyn IObservable<Option<BoxedValue>>>`
        /// or an `Rc<dyn IObservable<BoxedValue>>`, as the observable sources of the other
        /// behaviors of the port; another value is not an observable.
        pub fn item_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ScrollToItemBehavior, _>("Item", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ScrollToItemBehavior {
        /// Subscribes to the [`item`](Self::item) observable and scrolls to incoming values.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let behavior = this.to_ref().downgrade();
            let scroll_into_view = move |item: Option<BoxedValue>| {
                if let Some(items_control) =
                    behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<ItemsControl>())
                {
                    items_control.scroll_into_view_item(&item);
                }
            };

            let disposable = this.item().and_then(|item| {
                if let Some(observable) = item.downcast_ref::<Rc<dyn IObservable<Option<BoxedValue>>>>() {
                    return Some(observable.subscribe_fn(scroll_into_view));
                }
                if let Some(observable) = item.downcast_ref::<Rc<dyn IObservable<BoxedValue>>>() {
                    return Some(observable.subscribe_fn(move |item: BoxedValue| scroll_into_view(Some(item))));
                }
                None
            });

            disposable.unwrap_or_else(DisposableAction::empty)
        }
    }
}

impl ScrollToItemBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<ItemsControl as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the observable that produces items to scroll into view.
    pub fn item(&self) -> Option<BoxedValue> {
        self.get_value(Self::item_property())
    }

    pub fn set_item(&self, value: Option<BoxedValue>) {
        self.set_value(Self::item_property(), value)
    }
}
