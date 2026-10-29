//! Port of `Behaviors/AutoScrollToBottomBehavior.cs`.

use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{
    Control, ItemsChangedEventArgs, ItemsChangedHandler, ItemsControl, ItemsSource, ItemsSourceView,
    ScrollChangedEventArgs, ScrollViewer,
};
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The collection whose changes the behavior follows (the `INotifyCollectionChanged` of the
/// reference), with the token of its handler.
enum ItemsSubscription {
    /// An items source that notifies of its changes.
    Source(ItemsSource, u64),
    /// The items of the items control.
    Items(Rc<ItemsSourceView>, u64),
}

impl ItemsSubscription {
    fn unsubscribe(self) {
        match self {
            Self::Source(source, token) => source.list().remove_collection_changed(token),
            Self::Items(view, token) => view.remove_collection_changed(token),
        }
    }
}

/// A behavior that automatically scrolls to the bottom of a [`ScrollViewer`] or [`ItemsControl`]
/// when new items are added.
///
/// The scrolling follows the content while the viewer is at its bottom; once the user scrolls
/// away from the bottom it stops, until the user scrolls back.
#[repr(C)]
pub struct AutoScrollToBottomBehavior {
    base: StyledElementBehaviorOfT,
    // The scroll viewer and the token of its scroll changed handler.
    scroll_viewer: RefCell<Option<(Ref<ScrollViewer>, RoutedEventHandlerToken)>>,
    items: RefCell<Option<ItemsSubscription>>,
    auto_scroll: Cell<bool>,
}

ferro_class!(AutoScrollToBottomBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AutoScrollToBottomBehavior { new: AutoScrollToBottomBehavior::new });
ferroui_base::ferro_impl_classes!(AutoScrollToBottomBehavior: StyledElementImpl);

ferro_properties! {
    impl AutoScrollToBottomBehavior {
        /// Identifies the `ItemsSource` property.
        pub fn items_source_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<AutoScrollToBottomBehavior, _>("ItemsSource", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AutoScrollToBottomBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            let associated_object = this.associated_object_as::<Control>();
            if let Some(scroll_viewer) = associated_object.clone().and_then(|control| control.cast::<ScrollViewer>()) {
                this.set_scroll_viewer(Some(scroll_viewer));
            } else if let Some(items_control) = associated_object.and_then(|control| control.cast::<ItemsControl>()) {
                // Try to find ScrollViewer immediately.
                let scroll_viewer = items_control.find_descendant_of_type::<ScrollViewer>(false);
                if scroll_viewer.is_some() {
                    this.set_scroll_viewer(scroll_viewer);
                } else {
                    // Try later.
                    let behavior = this.to_ref().downgrade();
                    Dispatcher::ui_thread().post_local(
                        move || {
                            let Some(behavior) = behavior.upgrade() else {
                                return;
                            };
                            let is_associated = behavior
                                .associated_object_as::<ItemsControl>()
                                .is_some_and(|associated_object| associated_object.ptr_eq(&items_control));
                            if is_associated {
                                let scroll_viewer = items_control.find_descendant_of_type::<ScrollViewer>(false);
                                if scroll_viewer.is_some() {
                                    behavior.set_scroll_viewer(scroll_viewer);
                                }
                            }
                        },
                        DispatcherPriority::DEFAULT,
                    );
                }
            }

            this.update_items_source(this.items_source());
        }

        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);

            this.set_scroll_viewer(None);

            let items = this.items.borrow_mut().take();
            if let Some(items) = items {
                items.unsubscribe();
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for AutoScrollToBottomBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::items_source_property().as_property() {
                this.update_items_source(change.get_new_value::<Option<BoxedValue>>());
            }
        }
    }
}

impl AutoScrollToBottomBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            scroll_viewer: RefCell::new(None),
            items: RefCell::new(None),
            auto_scroll: Cell::new(true),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the items source to monitor for changes.
    pub fn items_source(&self) -> Option<BoxedValue> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, value: Option<BoxedValue>) {
        self.set_value(Self::items_source_property(), value)
    }

    /// Replaces the scroll viewer the behavior follows, moving its scroll changed handler.
    fn set_scroll_viewer(&self, scroll_viewer: Option<Ref<ScrollViewer>>) {
        let old = self.scroll_viewer.borrow_mut().take();
        if let Some((old, token)) = old {
            old.remove_handler(ScrollViewer::scroll_changed_event(), token);
        }

        if let Some(scroll_viewer) = scroll_viewer {
            let behavior = self.to_ref().downgrade();
            let token = scroll_viewer.scroll_changed(move |_sender: &Interactive, e: &ScrollChangedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_scroll_changed(e);
                }
            });
            *self.scroll_viewer.borrow_mut() = Some((scroll_viewer, token));
        }
    }

    fn current_scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        self.scroll_viewer.borrow().as_ref().map(|(scroll_viewer, _)| scroll_viewer.clone())
    }

    fn update_items_source(&self, items_source: Option<BoxedValue>) {
        let items = self.items.borrow_mut().take();
        if let Some(items) = items {
            items.unsubscribe();
        }

        let behavior = self.to_ref().downgrade();
        let handler: Rc<ItemsChangedHandler> = Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
            if let Some(behavior) = behavior.upgrade() {
                behavior.on_collection_changed(e);
            }
        });

        if let Some(items_source) = items_source {
            // Deviation (DEVIATIONS.md, Behaviors): the reference follows any value that
            // implements `INotifyCollectionChanged`; the port follows a value that converts to an
            // items source of the framework (the collections an items control accepts) and
            // notifies of its changes.
            let source = ValueTypes::try_convert(Some(&items_source), ValueType::of::<ItemsSource>())
                .flatten()
                .and_then(|value| value.downcast_ref::<ItemsSource>().cloned());
            if let Some(source) = source.filter(ItemsSource::is_notifying) {
                if let Some(token) = source.list().add_collection_changed(handler) {
                    *self.items.borrow_mut() = Some(ItemsSubscription::Source(source, token));
                }
            }
        } else if let Some(items_control) = self.associated_object_as::<ItemsControl>() {
            // Fallback to ItemsControl.Items if ItemsSource is not set.
            let view = items_control.items().view().clone();
            let token = view.add_collection_changed(handler);
            *self.items.borrow_mut() = Some(ItemsSubscription::Items(view, token));
        }
    }

    fn on_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        if e.action == NotifyCollectionChangedAction::Add && self.auto_scroll.get() {
            self.scroll_to_bottom();
        }
    }

    fn on_scroll_changed(&self, e: &ScrollChangedEventArgs) {
        let Some(scroll_viewer) = self.current_scroll_viewer() else {
            return;
        };

        if e.extent_delta().y == 0.0 {
            // User scroll.
            let offset = scroll_viewer.offset();
            let extent = scroll_viewer.extent();
            let viewport = scroll_viewer.viewport();
            self.auto_scroll.set(offset.y >= extent.height - viewport.height - 1.0);
        } else {
            // Content changed.
            if self.auto_scroll.get() {
                self.scroll_to_bottom();
            }
        }
    }

    fn scroll_to_bottom(&self) {
        if self.current_scroll_viewer().is_none() {
            if let Some(items_control) = self.associated_object_as::<ItemsControl>() {
                let scroll_viewer = items_control.find_descendant_of_type::<ScrollViewer>(false);
                if scroll_viewer.is_some() {
                    self.set_scroll_viewer(scroll_viewer);
                }
            }
        }

        if self.current_scroll_viewer().is_some() {
            let behavior = self.to_ref().downgrade();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(scroll_viewer) = behavior.upgrade().and_then(|behavior| behavior.current_scroll_viewer())
                    {
                        scroll_viewer.scroll_to_end();
                    }
                },
                DispatcherPriority::DEFAULT,
            );
        }
    }
}
