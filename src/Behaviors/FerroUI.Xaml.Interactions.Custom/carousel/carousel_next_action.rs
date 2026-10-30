//! Port of `Carousel/CarouselNextAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Carousel;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Advances the target [`Carousel`] to the next page.
#[repr(C)]
pub struct CarouselNextAction {
    base: StyledElementAction,
}

ferro_class!(CarouselNextAction: StyledElementAction);
ferro_class_info!(CarouselNextAction {
    new: CarouselNextAction::new,
    markup: {
        property_attributes: [
            Carousel: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CarouselNextAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CarouselNextAction {
        /// Identifies the `Carousel` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the carousel, so the value
        /// is an element reference.
        pub fn carousel_property() -> StyledProperty<Option<ElementRef<Carousel>>> {
            ValueTypes::register_element_ref::<Carousel>();
            FerroProperty::register::<CarouselNextAction, _>("Carousel", None)
        }
    }
}

impl StyledElementActionImpl for CarouselNextAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let carousel =
            this.carousel().or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<Carousel>()));
        if let Some(carousel) = carousel {
            carousel.next();
        }

        None
    }
}

impl CarouselNextAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the carousel instance this action will operate on. This is a property of the
    /// object model. Markup resolves a name given for it to the named element.
    pub fn carousel(&self) -> Option<Ref<Carousel>> {
        ElementRef::resolve(&self.get_value(Self::carousel_property()))
    }

    pub fn set_carousel(&self, value: impl Into<Nullable<Carousel>>) {
        self.set_value(Self::carousel_property(), ElementRef::from_nullable(value.into().0))
    }
}
