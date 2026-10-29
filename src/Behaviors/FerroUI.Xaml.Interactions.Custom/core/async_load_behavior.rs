//! Port of `Core/AsyncLoadBehavior.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::member_lookup::find_methods;
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, BehaviorOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// Behavior that calls an asynchronous method when the associated control is loaded.
///
/// The method is found by name on [`target_object`](Self::target_object), or on the data context
/// of the control, among the methods its markup metadata declares (the reflection of the
/// reference), and is called in a job of the dispatcher once the control is attached to the
/// visual tree.
#[repr(C)]
pub struct AsyncLoadBehavior {
    base: BehaviorOfT,
    // The subscription of the attached to visual tree event (the reference removes its delegate).
    loaded_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(AsyncLoadBehavior: BehaviorOfT);
ferro_class_info!(AsyncLoadBehavior {
    new: AsyncLoadBehavior::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AsyncLoadBehavior: FerroObjectImpl);

ferro_properties! {
    impl AsyncLoadBehavior {
        /// Identifies the `MethodName` property.
        pub fn method_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<AsyncLoadBehavior, _>("MethodName", None)
        }

        /// Identifies the `TargetObject` property.
        pub fn target_object_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<AsyncLoadBehavior, _>("TargetObject", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for AsyncLoadBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            if let Some(associated_object) = this.associated_object_as::<Control>() {
                let behavior = this.to_ref().downgrade();
                let subscription = associated_object.attached_to_visual_tree(move |_e| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.on_loaded();
                    }
                });
                *this.loaded_subscription.borrow_mut() = Some(subscription);
            }
        }

        fn on_detaching(this: &Self) {
            this.remove_loaded_handler();
            Self::parent_on_detaching(this);
        }
    }
}

impl AsyncLoadBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: BehaviorOfT::construct(<Control as StaticType>::TYPE), loaded_subscription: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the method to invoke on load.
    pub fn method_name(&self) -> Option<String> {
        self.get_value(Self::method_name_property())
    }

    pub fn set_method_name(&self, value: Option<String>) {
        self.set_value(Self::method_name_property(), value)
    }

    /// Gets or sets the object that exposes the method of interest. If null the data context is
    /// used. Markup resolves a name given for it to the named element.
    pub fn target_object(&self) -> Option<BoxedValue> {
        self.get_value(Self::target_object_property())
    }

    pub fn set_target_object(&self, value: Option<BoxedValue>) {
        self.set_value(Self::target_object_property(), value)
    }

    fn remove_loaded_handler(&self) {
        let subscription = self.loaded_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn on_loaded(&self) {
        self.remove_loaded_handler();

        let behavior: Ref<AsyncLoadBehavior> = self.to_ref();
        Dispatcher::ui_thread().post_local(move || behavior.invoke_async(), DispatcherPriority::DEFAULT);
    }

    fn invoke_async(&self) {
        let Some(associated_object) = self.associated_object_as::<Control>() else {
            return;
        };
        let Some(method_name) = self.method_name().filter(|name| !name.is_empty()) else {
            return;
        };

        let Some(target) = self.target_object().or_else(|| associated_object.data_context()) else {
            return;
        };

        // `GetRuntimeMethod(MethodName, Type.EmptyTypes)`: the method without parameters.
        let Some(method_info) = find_methods(&*target, &method_name).into_iter().find(|method| method.parameters.is_empty())
        else {
            return;
        };

        let arguments = if method_info.is_static { Vec::new() } else { vec![Some(target.clone())] };
        // Deviation (DEVIATIONS.md, Behaviors): the result of the method is not awaited (the
        // reference awaits a returned `Task`); a task the method starts runs on its own. A failure
        // of the call is dropped, as the reference drops the faulted task of its dispatcher job.
        let _ = (method_info.invoke)(&arguments);
    }
}
