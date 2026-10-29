//! Port of `Network/NetworkInformationTrigger.cs`.

use crate::background;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref};
use ferroui_xaml_interactivity::{BehaviorImpl, BehaviorImplExt, Interaction, Trigger};
use std::cell::RefCell;
use std::net::UdpSocket;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// How often the trigger looks at the availability of the network.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// A trigger that listens to network availability changes: its actions run with the new
/// availability (`bool`) as the parameter.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference listens to the notification of the
/// operating system (`NetworkChange.NetworkAvailabilityChanged`); the port looks every two seconds,
/// on a background thread, whether the system has a route to the internet (a datagram socket can
/// be connected to a public address; nothing is sent). Behind the feature `network`.
#[repr(C)]
pub struct NetworkInformationTrigger {
    base: Trigger,
    monitor: RefCell<Option<(u64, Arc<AtomicBool>)>>,
}

ferro_class!(NetworkInformationTrigger: Trigger);
ferro_class_info!(NetworkInformationTrigger { new: NetworkInformationTrigger::new });
ferroui_base::ferro_impl_classes!(NetworkInformationTrigger: FerroObjectImpl);

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for NetworkInformationTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.start_monitor();
        }

        fn on_detaching(this: &Self) {
            this.stop_monitor();
            Self::parent_on_detaching(this);
        }
    }
}

/// Whether the system has a route to the internet.
fn is_network_available() -> bool {
    let connects = |local: &str, remote: &str| UdpSocket::bind(local).and_then(|socket| socket.connect(remote)).is_ok();
    connects("0.0.0.0:0", "8.8.8.8:53") || connects("[::]:0", "[2001:4860:4860::8888]:53")
}

impl NetworkInformationTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Trigger::construct(), monitor: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn start_monitor(&self) {
        if self.monitor.borrow().is_some() {
            return;
        }

        let id = background::register(self);
        let stop = Arc::new(AtomicBool::new(false));
        let dispatcher = Dispatcher::ui_thread();
        let stopped = stop.clone();
        let spawned = std::thread::Builder::new().name("NetworkInformationTrigger".to_string()).spawn(move || {
            let mut is_available = is_network_available();
            while !stopped.load(Ordering::Relaxed) {
                std::thread::sleep(POLL_INTERVAL);
                if stopped.load(Ordering::Relaxed) {
                    break;
                }

                let now_available = is_network_available();
                if now_available != is_available {
                    is_available = now_available;
                    background::post_to(&dispatcher, id, move |target| {
                        if let Some(trigger) = target.downcast_ref::<NetworkInformationTrigger>() {
                            trigger.on_network_availability_changed(now_available);
                        }
                    });
                }
            }
        });

        if spawned.is_err() {
            background::unregister(id);
            return;
        }

        *self.monitor.borrow_mut() = Some((id, stop));
    }

    fn stop_monitor(&self) {
        let monitor = self.monitor.borrow_mut().take();
        let Some((id, stop)) = monitor else {
            return;
        };

        stop.store(true, Ordering::Relaxed);
        background::unregister(id);
    }

    fn on_network_availability_changed(&self, is_available: bool) {
        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        let parameter: BoxedValue = Rc::new(is_available);
        Interaction::execute_actions(sender.as_ref(), Some(&self.actions()), Some(&parameter));
    }
}
