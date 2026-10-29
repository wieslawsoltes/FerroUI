//! Port of `ViewModel/ViewModelPropertyChangedTrigger.cs`.

use ferroui_base::data::model::ModelTypes;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTriggerOfT,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Triggers when the specified view model property changes.
///
/// Deviation (DEVIATIONS.md, Behaviors): the parameter of the actions is the name of the changed
/// property (the `PropertyChangedEventArgs` of the reference carry only that name), and the view
/// model is observed when its model type declares its property change notifications.
#[repr(C)]
pub struct ViewModelPropertyChangedTrigger {
    base: StyledElementTriggerOfT,
    // The observed view model and the token of the handler of its notifications.
    inpc: RefCell<Option<(BoxedValue, u64)>>,
}

ferro_class!(ViewModelPropertyChangedTrigger: StyledElementTriggerOfT);
ferro_class_info!(ViewModelPropertyChangedTrigger { new: ViewModelPropertyChangedTrigger::new });
ferroui_base::ferro_impl_classes!(ViewModelPropertyChangedTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ViewModelPropertyChangedTrigger {
        /// Identifies the `PropertyName` property.
        pub fn property_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ViewModelPropertyChangedTrigger, _>("PropertyName", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ViewModelPropertyChangedTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            let Some(data_context) = this.associated_object_as::<Control>().and_then(|control| control.data_context())
            else {
                return;
            };

            let Some(model) = ModelTypes::find(&*data_context) else {
                return;
            };
            let Some(inpc) = model.as_notify_property_changed(&*data_context) else {
                return;
            };

            let trigger = this.to_ref().downgrade();
            let token = inpc.property_changed().add(Rc::new(move |property_name: &str| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.view_model_property_changed(property_name);
                }
            }));
            *this.inpc.borrow_mut() = Some((data_context.clone(), token));
        }

        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);

            let inpc = this.inpc.borrow_mut().take();
            if let Some((data_context, token)) = inpc {
                if let Some(inpc) =
                    ModelTypes::find(&*data_context).and_then(|model| model.as_notify_property_changed(&*data_context))
                {
                    inpc.property_changed().remove(token);
                }
            }
        }
    }
}

impl ViewModelPropertyChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE), inpc: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the property to monitor. This is a property of the object model.
    pub fn property_name(&self) -> Option<String> {
        self.get_value(Self::property_name_property())
    }

    pub fn set_property_name(&self, value: Option<String>) {
        self.set_value(Self::property_name_property(), value)
    }

    fn view_model_property_changed(&self, property_name: &str) {
        if self.property_name().as_deref() != Some(property_name) {
            return;
        }

        let trigger: Ref<ViewModelPropertyChangedTrigger> = self.to_ref();
        let parameter: BoxedValue = Rc::new(property_name.to_string());
        Dispatcher::ui_thread().post_local(
            move || {
                let sender =
                    trigger.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
                Interaction::execute_actions(sender.as_ref(), trigger.actions().as_ref(), Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
