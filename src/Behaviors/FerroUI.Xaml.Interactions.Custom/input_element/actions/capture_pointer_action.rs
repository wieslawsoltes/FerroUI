//! Port of `InputElement/Actions/CapturePointerAction.cs`.

use super::super::pointer_event_args_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};

/// Captures the pointer.
#[repr(C)]
pub struct CapturePointerAction {
    base: StyledElementAction,
}

ferro_class!(CapturePointerAction: StyledElementAction);
ferro_class_info!(CapturePointerAction {
    new: CapturePointerAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CapturePointerAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CapturePointerAction {
        /// Identifies the [`target_control`](CapturePointerAction::target_control) property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the
        /// value is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<CapturePointerAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for CapturePointerAction {
    /// Executes the action: the pointer of the pointer event args given as the parameter captures
    /// the target control, or the source of the event when no target is set.
    ///
    /// Returns null after executed.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let pointer_event_args = pointer_event_args_of(parameter)?;

        let input_element = pointer_event_args.source().and_then(|source| source.cast::<InputElement>())?;

        let control: Ref<InputElement> = match this.target_control() {
            Some(target_control) => target_control.upcast(),
            None => input_element,
        };

        pointer_event_args.pointer().capture(Some(&control));

        None
    }
}

impl CapturePointerAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target control. This is a property of the object model. Markup resolves a
    /// name given for it to the named element.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }
}
