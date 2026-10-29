//! Port of `Control/BoundsObserverBehavior.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroProperty, Rect, Ref, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Observes the bounds given to [`bounds`](Self::bounds) (usually bound to the bounds of a
/// control) and exposes their width and height.
#[repr(C)]
pub struct BoundsObserverBehavior {
    base: DisposingBehavior,
}

ferro_class!(BoundsObserverBehavior: DisposingBehavior);
ferro_class_info!(BoundsObserverBehavior { new: BoundsObserverBehavior::new });
ferroui_base::ferro_impl_classes!(BoundsObserverBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl BoundsObserverBehavior {
        /// Identifies the `Bounds` property.
        pub fn bounds_property() -> StyledProperty<Rect> {
            FerroProperty::register_with::<BoundsObserverBehavior, _>(
                "Bounds",
                StyledPropertyOptions::new(Rect::default()).default_binding_mode(BindingMode::OneWay),
            )
        }

        /// Identifies the `Width` property.
        pub fn width_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<BoundsObserverBehavior, _>(
                "Width",
                StyledPropertyOptions::new(0.0).default_binding_mode(BindingMode::TwoWay),
            )
        }

        /// Identifies the `Height` property.
        pub fn height_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<BoundsObserverBehavior, _>(
                "Height",
                StyledPropertyOptions::new(0.0).default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for BoundsObserverBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            if this.associated_object().is_some() {
                let behavior = this.to_ref().downgrade();
                let this_object: &FerroObject = this;
                return FerroObjectExtensions::get_observable(this_object, Self::bounds_property()).subscribe_fn(
                    move |bounds: Rect| {
                        if let Some(behavior) = behavior.upgrade() {
                            behavior.set_width(bounds.width);
                            behavior.set_height(bounds.height);
                        }
                    },
                );
            }

            DisposableAction::empty()
        }
    }
}

impl BoundsObserverBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the observed bounds. This is a property of the object model.
    pub fn bounds(&self) -> Rect {
        self.get_value(Self::bounds_property())
    }

    pub fn set_bounds(&self, value: Rect) {
        self.set_value(Self::bounds_property(), value)
    }

    /// Gets or sets the width of the observed bounds. This is a property of the object model,
    /// bound in both directions by default.
    pub fn width(&self) -> f64 {
        self.get_value(Self::width_property())
    }

    pub fn set_width(&self, value: f64) {
        self.set_value(Self::width_property(), value)
    }

    /// Gets or sets the height of the observed bounds. This is a property of the object model,
    /// bound in both directions by default.
    pub fn height(&self) -> f64 {
        self.get_value(Self::height_property())
    }

    pub fn set_height(&self, value: f64) {
        self.set_value(Self::height_property(), value)
    }
}
