//! Port of `Core/BindingBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingBase;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObject, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Applies a binding to a target property when the control is attached to the visual tree.
///
/// The binding is cleared when the control leaves the visual tree or the behavior is detached.
#[repr(C)]
pub struct BindingBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(BindingBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(BindingBehavior {
    new: BindingBehavior::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
            Binding: [AssignBinding],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    BindingBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl BindingBehavior {
        /// Gets or sets the property that will be bound.
        pub fn target_property_property() -> StyledProperty<Option<&'static FerroProperty>> {
            FerroProperty::register::<BindingBehavior, _>("TargetProperty", None)
        }

        /// Gets or sets the object that owns the `TargetProperty`.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own its target, so the
        /// value is an element reference.
        pub fn target_object_property() -> StyledProperty<Option<ElementRef<FerroObject>>> {
            ValueTypes::register_element_ref::<FerroObject>();
            FerroProperty::register::<BindingBehavior, _>("TargetObject", None)
        }

        /// Gets or sets the binding to apply.
        pub fn binding_property() -> StyledProperty<Option<Rc<dyn BindingBase>>> {
            FerroProperty::register_with::<BindingBehavior, _>(
                "Binding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for BindingBehavior {
        /// Applies the binding when the behavior is attached to the visual tree.
        ///
        /// Returns a disposable that clears the binding when disposed.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if let (Some(target_object), Some(target_property), Some(binding)) =
                (this.target_object(), this.target_property(), this.binding())
            {
                let expression = target_object.bind_binding(target_property, &*binding);
                return expression;
            }

            DisposableAction::empty()
        }
    }
}

impl BindingBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The property that will be bound. This is a property of the object model.
    pub fn target_property(&self) -> Option<&'static FerroProperty> {
        self.get_value(Self::target_property_property())
    }

    pub fn set_target_property(&self, value: Option<&'static FerroProperty>) {
        self.set_value(Self::target_property_property(), value)
    }

    /// The object that owns the [`target_property`](Self::target_property). This is a property
    /// of the object model; markup resolves a name given for it to the named element.
    pub fn target_object(&self) -> Option<Ref<FerroObject>> {
        ElementRef::resolve(&self.get_value(Self::target_object_property()))
    }

    pub fn set_target_object(&self, value: impl Into<Nullable<FerroObject>>) {
        self.set_value(Self::target_object_property(), ElementRef::from_nullable(value.into().0))
    }

    /// The binding to apply. This is a property of the object model, to which markup assigns a
    /// binding as it is.
    pub fn binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.get_value(Self::binding_property())
    }

    pub fn set_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        self.set_value(Self::binding_property(), value)
    }
}
