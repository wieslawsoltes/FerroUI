//! `ViewContractObservableHelpers` (`Shared/ViewContractObservableHelpers.cs`).

use super::StartWithObservable;
use crate::IPlatformOperations;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_splat::{AppLocator, IFullLogger, ModeDetector};
use ferroui_system::IObservable;
use std::rc::Rc;

/// The streams of the view contract that the view hosts of the platforms share
/// (`ViewContractObservableHelpers`).
pub struct ViewContractObservableHelpers;

impl ViewContractObservableHelpers {
    /// The function that gives the orientation of the platform to a view host
    /// (`GetPlatformOrientation(IFullLogger)`): that of the registered [`IPlatformOperations`],
    /// or, with none registered, a function that gives `None` after `logger` reported the missing
    /// registration.
    pub fn get_platform_orientation(logger: &dyn IFullLogger) -> Rc<dyn Fn() -> Option<String>> {
        let Some(platform) = AppLocator::get_service::<dyn IPlatformOperations>() else {
            logger.error(
                "Couldn't find an IPlatformOperations implementation. Please make sure you have installed \
                 the latest version of the ReactiveUI packages for your platform. \
                 See https://reactiveui.net/docs/getting-started/installation for guidance.",
            );
            return Rc::new(|| None);
        };

        Rc::new(move || platform.get_orientation())
    }

    /// The stream of the view contract from the orientation of the platform and the changes of
    /// the size of the host (`Create(Func<string?>, IObservable<string?>)`): the current
    /// orientation, then the orientation after each change of size, without repeats. In a unit
    /// test runner it never notifies.
    pub fn create(
        platform_getter: &dyn Fn() -> Option<String>,
        size_changes: Rc<dyn IObservable<Option<String>>>,
    ) -> Rc<dyn IObservable<Option<String>>> {
        if ModeDetector::in_unit_test_runner() {
            return Observable::never();
        }

        let with_start: Rc<dyn IObservable<Option<String>>> = StartWithObservable::new(size_changes, platform_getter());
        with_start.distinct_until_changed()
    }
}
