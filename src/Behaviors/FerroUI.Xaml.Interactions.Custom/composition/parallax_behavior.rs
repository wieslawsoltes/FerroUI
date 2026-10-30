//! Port of `Composition/ParallaxBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::{IDisposable, ObservableError, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectExtensions, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledProperty, Vector,
};
use ferroui_controls::{Control, ScrollViewer};
use ferroui_xaml_behaviors_animations::ParallaxAnimation;
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that moves the associated element at a different speed than the scrolling
/// container, creating a parallax effect.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference implements `IObserver<Vector>` and
/// subscribes itself to the offset of the scroll viewer; here the subscription is a closure that
/// calls [`on_next`](Self::on_next), and [`on_error`](Self::on_error) and
/// [`on_completed`](Self::on_completed) are plain members (the class does not implement the
/// observer contract, an object of the class model is not shared as an `Rc`).
#[repr(C)]
pub struct ParallaxBehavior {
    base: BehaviorOfT,
    scroll_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    animation: RefCell<Option<ParallaxAnimation>>,
}

ferro_class!(ParallaxBehavior: BehaviorOfT);
ferro_class_info!(ParallaxBehavior {
    new: ParallaxBehavior::new,
    markup: {
        property_attributes: [
            SourceScrollViewer: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ParallaxBehavior: FerroObjectImpl);

ferro_properties! {
    impl ParallaxBehavior {
        /// Identifies the `SourceScrollViewer` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the scroll viewer, so
        /// the value is an element reference.
        pub fn source_scroll_viewer_property() -> StyledProperty<Option<ElementRef<ScrollViewer>>> {
            ValueTypes::register_element_ref::<ScrollViewer>();
            FerroProperty::register::<ParallaxBehavior, _>("SourceScrollViewer", None)
        }

        /// Identifies the `ParallaxRatio` property.
        pub fn parallax_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxBehavior, _>("ParallaxRatio", 0.2)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for ParallaxBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            let associated_object = this.associated_object_as::<Control>();
            *this.animation.borrow_mut() = ParallaxAnimation::try_create(associated_object.as_deref());
            if this.source_scroll_viewer().is_none() {
                // Try to find parent ScrollViewer
                let mut parent = associated_object.as_ref().and_then(|associated_object| associated_object.parent());
                while let Some(element) = parent {
                    if let Some(scroll_viewer) = element.cast::<ScrollViewer>() {
                        this.set_source_scroll_viewer(scroll_viewer);
                        break;
                    }
                    parent = element.parent();
                }
            }

            if let Some(source_scroll_viewer) = this.source_scroll_viewer() {
                let behavior = this.to_ref().downgrade();
                let subscription = FerroObjectExtensions::get_observable(&*source_scroll_viewer as &ferroui_base::FerroObject, ScrollViewer::offset_property()).subscribe_fn(
                    move |value: Vector| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.on_next(value);
                        }
                    },
                );
                let previous = this.scroll_subscription.borrow_mut().replace(subscription);
                if let Some(previous) = previous {
                    previous.dispose();
                }
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            let subscription = this.scroll_subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            *this.animation.borrow_mut() = None;
        }
    }
}

impl ParallaxBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: BehaviorOfT::construct(<Control as StaticType>::TYPE),
            scroll_subscription: RefCell::new(None),
            animation: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the source scroll viewer. If not set, the behavior will attempt to find a
    /// parent scroll viewer. Markup resolves a name given for it to the named element.
    pub fn source_scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        ElementRef::resolve(&self.get_value(Self::source_scroll_viewer_property()))
    }

    pub fn set_source_scroll_viewer(&self, value: impl Into<Nullable<ScrollViewer>>) {
        self.set_value(Self::source_scroll_viewer_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the parallax ratio.
    /// 0.0 means no movement (static).
    /// 1.0 means moves with scroll (normal).
    /// Values between 0 and 1 create a "far away" depth effect.
    /// Negative values move in reverse.
    pub fn parallax_ratio(&self) -> f64 {
        self.get_value(Self::parallax_ratio_property())
    }

    pub fn set_parallax_ratio(&self, value: f64) {
        self.set_value(Self::parallax_ratio_property(), value)
    }

    /// Notifies the behavior that the offset observable completed (`IObserver.OnCompleted`).
    pub fn on_completed(&self) {}

    /// Notifies the behavior that the offset observable failed (`IObserver.OnError`).
    pub fn on_error(&self, _error: ObservableError) {}

    /// Applies the parallax offset for the scroll offset `value` (`IObserver.OnNext`).
    pub fn on_next(&self, value: Vector) {
        if self.animation.borrow().is_none() {
            let associated_object = self.associated_object_as::<Control>();
            *self.animation.borrow_mut() = ParallaxAnimation::try_create(associated_object.as_deref());
        }
        if let Some(animation) = &*self.animation.borrow() {
            animation.apply(value, self.parallax_ratio());
        }
    }
}
