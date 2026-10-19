//! `RxSuspension` (`ReactiveUI.Shared/RxSuspension.cs`).

use crate::{ISuspensionHost, SuspensionHost};
use std::cell::RefCell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The published suspension host, if any (`_suspensionHost`).
    static SUSPENSION_HOST: RefCell<Option<Rc<dyn ISuspensionHost>>> = const { RefCell::new(None) };
}

/// The suspension host of the application, for the events of the life of its process, such as
/// its suspension and resumption on a mobile device (`RxSuspension`).
///
/// Unless the builder of the application published a host, the first read creates a default
/// [`SuspensionHost`]; the first host published wins.
// Deviation (DEVIATIONS.md, ReactiveUI): the host is per thread, as the objects of the crate are
// (docs/porting/reactive.md, section 5.1).
pub struct RxSuspension;

impl RxSuspension {
    /// The suspension host (`SuspensionHost`): the published one, or a default one created now.
    pub fn suspension_host() -> Rc<dyn ISuspensionHost> {
        if let Some(host) = SUSPENSION_HOST.with(|host| host.borrow().clone()) {
            return host;
        }
        Self::initialize_default_suspension_host()
    }

    /// Publishes `suspension_host` when no host is published yet (`InitializeSuspensionHost`),
    /// which the builder of the application calls.
    pub fn initialize_suspension_host(suspension_host: Rc<dyn ISuspensionHost>) {
        SUSPENSION_HOST.with(|host| {
            let mut host = host.borrow_mut();
            if host.is_none() {
                *host = Some(suspension_host);
            }
        });
    }

    /// Forgets the published host (`ResetForTesting`). For tests only.
    #[doc(hidden)]
    pub fn reset_for_testing() {
        SUSPENSION_HOST.with(|host| *host.borrow_mut() = None);
    }

    /// Publishes a new default host if none is published yet (`InitializeDefaultSuspensionHost`).
    fn initialize_default_suspension_host() -> Rc<dyn ISuspensionHost> {
        let created: Rc<dyn ISuspensionHost> = SuspensionHost::new();
        SUSPENSION_HOST.with(|host| host.borrow_mut().get_or_insert(created).clone())
    }
}
