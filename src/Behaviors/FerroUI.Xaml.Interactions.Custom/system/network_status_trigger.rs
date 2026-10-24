//! Port of `System/NetworkStatusTrigger.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTriggerOfT,
};
use std::cell::Cell;
use std::rc::Rc;

/// Defines the network status to listen for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NetworkStatus {
    /// The network is available (Online).
    Online,
    /// The network is not available (Offline).
    Offline,
    /// Any change in network status.
    #[default]
    Any,
}

ferro_markup_enum!(NetworkStatus { Online, Offline, Any });

impl std::fmt::Display for NetworkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Online => "Online",
            Self::Offline => "Offline",
            Self::Any => "Any",
        })
    }
}

/// A trigger that fires when the network status changes.
///
/// The availability of the network (a boolean) is the parameter of the actions.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference listens to the availability change event of
/// the network interfaces of .NET (`NetworkChange.NetworkAvailabilityChanged`) while the control
/// is in the visual tree. The framework has no network availability service, so the port does not
/// observe the system: an application that observes it reports a change with
/// [`on_network_availability_changed`](Self::on_network_availability_changed), which the trigger
/// handles while the control is in the visual tree, as the reference handles the event.
#[repr(C)]
pub struct NetworkStatusTrigger {
    base: StyledElementTriggerOfT,
    // Whether the trigger listens (between attached to and detached from the visual tree: the
    // subscription of the reference).
    is_listening: Cell<bool>,
}

ferro_class!(NetworkStatusTrigger: StyledElementTriggerOfT);
ferro_class_info!(NetworkStatusTrigger { new: NetworkStatusTrigger::new });
ferroui_base::ferro_impl_classes!(NetworkStatusTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl NetworkStatusTrigger {
        /// Identifies the `Status` property.
        pub fn status_property() -> StyledProperty<NetworkStatus> {
            FerroProperty::register::<NetworkStatusTrigger, _>("Status", NetworkStatus::Any)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for NetworkStatusTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);
            this.is_listening.set(true);
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);
            this.is_listening.set(false);
        }
    }
}

impl NetworkStatusTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE), is_listening: Cell::new(false) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the network status to listen for.
    pub fn status(&self) -> NetworkStatus {
        self.get_value(Self::status_property())
    }

    pub fn set_status(&self, value: NetworkStatus) {
        self.set_value(Self::status_property(), value)
    }

    /// Reports a change of the availability of the network (the handler of
    /// `NetworkChange.NetworkAvailabilityChanged` in the reference): unless the control is out of
    /// the visual tree, the actions run in a job of the dispatcher when the change matches
    /// [`status`](Self::status).
    pub fn on_network_availability_changed(&self, is_available: bool) {
        if !self.is_listening.get() {
            return;
        }

        let trigger: Ref<NetworkStatusTrigger> = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                let status = trigger.status();

                if status == NetworkStatus::Any
                    || (status == NetworkStatus::Online && is_available)
                    || (status == NetworkStatus::Offline && !is_available)
                {
                    let sender = trigger.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
                    let parameter: BoxedValue = Rc::new(is_available);
                    Interaction::execute_actions(sender.as_ref(), trigger.actions().as_ref(), Some(&parameter));
                }
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
