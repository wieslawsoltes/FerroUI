//! Native diagnostic sinks for selected Repeater item lifecycles.
//!
//! The source's debug index hooks select an item; the existing FerroUI logger
//! selects levels and output. Both sink lookup and delivery are deferred until
//! the host finishes its layout/change operation. Queued messages contain event
//! snapshots and weak sources, so diagnostics cannot prolong a host's lifetime.

use super::{ElementOwner, ItemsRepeater, RepeaterTestHooks};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};

pub(crate) fn element_lifecycle(
    repeater: &ItemsRepeater,
    index: i32,
    operation: &'static str,
    owner: ElementOwner,
) {
    if index < 0 || RepeaterTestHooks::get_log_item_index() != index {
        return;
    }
    let weak = repeater.to_ref().downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            let Some(repeater) = weak.upgrade() else {
                return;
            };
            // Both is_enabled and log are application callbacks. Defer the
            // lookup as well as delivery, retaining only the event snapshot.
            if let Some(logger) = Logger::try_get(LogEventLevel::Information, LogArea::CONTROL) {
                let owner = match owner {
                    ElementOwner::ElementFactory => "ElementFactory",
                    ElementOwner::Layout => "Layout",
                    ElementOwner::PinnedPool => "PinnedPool",
                    ElementOwner::UniqueIdResetPool => "UniqueIdResetPool",
                    ElementOwner::Animator => "Animator",
                };
                logger.log_with_values(
                    Some(&*repeater),
                    "Repeater item {Index}: {Operation}, owner {Owner}",
                    &[&index, &operation, &owner],
                );
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}

pub(crate) fn owner_arranged(repeater: &ItemsRepeater) {
    let Some(layout) = repeater.layout() else {
        return;
    };
    let index = layout.log_item_index();
    if index < 0 {
        return;
    }
    let Some(element) = repeater.try_get_element(index) else {
        return;
    };
    if !ItemsRepeater::get_virtualization_info(&element).is_held_by_layout() {
        return;
    }
    // Layout::arrange has returned, but the host still has its layout guard
    // and child/transition sweep. Neither sink callback may run until those
    // finish. Keep the geometry from this arrangement even if later work moves
    // or recycles the element before the diagnostic is delivered.
    let bounds = element.bounds();
    let desired = element.desired_size();
    let weak_layout = layout.downgrade();
    let weak_repeater = repeater.to_ref().downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            let (Some(_repeater), Some(layout)) = (weak_repeater.upgrade(), weak_layout.upgrade())
            else {
                return;
            };
            if let Some(logger) = Logger::try_get(LogEventLevel::Information, LogArea::LAYOUT) {
                logger.log_with_values(
                    Some(&*layout),
                    "Repeater arranged item {Index}: bounds {Bounds}, desired size {DesiredSize}",
                    &[&index, &bounds, &desired],
                );
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}
