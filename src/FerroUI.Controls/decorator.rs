use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, VisualImpl,
};

/// Base class for controls which decorate a single child control.
#[repr(C)]
pub struct Decorator {
    base: Control,
}

ferro_class!(Decorator: Control);
ferroui_base::ferro_class_info!(Decorator { new: Decorator::new });
ferro_impl_classes!(Decorator: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for Decorator {}

impl LayoutableImpl for Decorator {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::measure_child(child.as_deref().map(|c| -> &Layoutable { c }), available_size, this.padding())
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let child = this.child();
        LayoutHelper::arrange_child(child.as_deref().map(|c| -> &Layoutable { c }), final_size, this.padding())
    }
}

ferroui_base::ferro_properties! { impl Decorator {
    ferro_property!(
        /// Defines the `Child` property.
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<Decorator, _>("Child", None)
        }
    );

    ferro_property!(
        /// Defines the `Padding` property.
        pub fn padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<Decorator, _>(
                "Padding",
                StyledPropertyOptions::new(Thickness::default())
                    .validate(|value| Layoutable::margin_property().validate_value().is_none_or(|validate| validate(value))),
            )
        }
    );
} }

impl Decorator {
    fn static_constructor() {
        Layoutable::affects_measure::<Decorator>(&[
            Self::child_property().as_property(),
            Self::padding_property().as_property(),
        ]);
        Self::child_property().changed().add_class_handler::<Decorator>(|x, e| x.child_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The decorated control.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.get_value(Self::child_property())
    }

    pub fn set_child(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::child_property(), value.into().0)
    }

    /// The padding to place around the child control.
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }

    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value)
    }

    /// Called when the `Child` property changes.
    fn child_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let (old_child, new_child) = e.get_old_and_new_value::<Option<Ref<Control>>>();

        if let Some(old_child) = old_child {
            old_child.set_parent(None);
            self.logical_children().clear();
            self.visual_children().remove(&old_child.upcast());
        }

        if let Some(new_child) = new_child {
            new_child.set_parent(self.to_ref());
            self.visual_children().add(new_child.clone().upcast());
            self.logical_children().add(new_child.upcast());
        }
    }
}
