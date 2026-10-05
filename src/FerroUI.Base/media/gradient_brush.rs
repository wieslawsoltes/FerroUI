use crate::media::media_collection::track_item_property_changed;
use crate::media::immutable::ImmutableGradientStop;
use crate::media::{Brush, BrushImpl, BrushImplExt, GradientSpreadMethod, GradientStops, IImmutableBrush};
use crate::reactive::IDisposable;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::Compositor;
use crate::{
    ferro_class, ferro_property, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Base class for brushes that draw with a gradient.
#[repr(C)]
pub struct GradientBrush {
    base: Brush,
    gradient_stops_collection_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    gradient_stops_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    GradientBrush: Brush, virtuals GradientBrushImpl: BrushImpl {
        /// Creates an immutable clone of the brush.
        fn to_immutable(this) -> Rc<dyn IImmutableBrush>;
    }
}

impl FerroObjectImpl for GradientBrush {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_gradient_stops(GradientStops::new());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::gradient_stops_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<GradientStops>>();

            if old_value.is_some() {
                if let Some(subscription) = this.gradient_stops_collection_subscription.take() {
                    subscription.dispose();
                }
                if let Some(subscription) = this.gradient_stops_subscription.take() {
                    subscription.dispose();
                }
            }

            if let Some(new_value) = new_value {
                let weak = this.to_ref().downgrade();
                let collection_subscription = new_value.collection_changed({
                    let weak = weak.clone();
                    move |_| {
                        if let Some(this) = weak.upgrade() {
                            this.register_for_serialization();
                            this.raise_invalidated();
                        }
                    }
                });
                *this.gradient_stops_collection_subscription.borrow_mut() = Some(collection_subscription);

                let subscription = track_item_property_changed(&new_value, move || {
                    if let Some(this) = weak.upgrade() {
                        this.register_for_serialization();
                        this.raise_invalidated();
                    }
                });
                *this.gradient_stops_subscription.borrow_mut() = Some(subscription);
            }
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl BrushImpl for GradientBrush {
    fn serialize_changes(this: &Self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        Self::parent_serialize_changes(this, c, writer);
        writer.write(this.spread_method());
        let gradient_stops = this.gradient_stops();
        writer.write(gradient_stops.len() as i32);
        for stop in gradient_stops.iter() {
            // TODO: Technically it allocates, so it would be better to sync stops individually
            writer.write_value(Some(ImmutableGradientStop::new(stop.offset(), stop.color())));
        }
    }
}

impl GradientBrushImpl for GradientBrush {
    fn to_immutable(_this: &Self) -> Rc<dyn IImmutableBrush> {
        panic!("GradientBrush is abstract: 'to_immutable' must be implemented by the deriving class")
    }
}

crate::ferro_properties! { impl GradientBrush {
    ferro_property!(pub fn spread_method_property() -> StyledProperty<GradientSpreadMethod> {
        FerroProperty::register::<GradientBrush, _>("SpreadMethod", GradientSpreadMethod::Pad)
    });

    ferro_property!(pub fn gradient_stops_property() -> StyledProperty<Option<GradientStops>> {
        FerroProperty::register::<GradientBrush, _>("GradientStops", None)
    });
} }

impl GradientBrush {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Brush::construct(),
            gradient_stops_collection_subscription: RefCell::new(None),
            gradient_stops_subscription: RefCell::new(None),
        }
    }

    /// The brush's spread method that defines how to draw a gradient that
    /// doesn't fill the bounds of the destination control.
    pub fn spread_method(&self) -> GradientSpreadMethod {
        self.get_value(Self::spread_method_property())
    }

    pub fn set_spread_method(&self, value: GradientSpreadMethod) {
        self.set_value(Self::spread_method_property(), value)
    }

    /// The brush's gradient stops. Panics if the property has been cleared.
    pub fn gradient_stops(&self) -> GradientStops {
        self.get_value(Self::gradient_stops_property()).expect("GradientStops is not set")
    }

    pub fn set_gradient_stops(&self, value: GradientStops) {
        self.set_value(Self::gradient_stops_property(), Some(value))
    }
}
