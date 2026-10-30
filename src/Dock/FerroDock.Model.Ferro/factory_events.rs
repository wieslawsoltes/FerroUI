// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source factory-event adapters for native metadata declarations.
use ferrodock_model::core::{
    events::{
        ActiveDockableChangedEventArgs, DockableActivatedEventArgs, DockableAddedEventArgs,
        DockableClosedEventArgs, DockableClosingEventArgs, DockableDeactivatedEventArgs,
        DockableDockedEventArgs, DockableHiddenEventArgs, DockableInitEventArgs,
        DockableMovedEventArgs, DockablePinnedEventArgs, DockableRemovedEventArgs,
        DockableRestoredEventArgs, DockableSwappedEventArgs, DockableUndockedEventArgs,
        DockableUnpinnedEventArgs, FocusedDockableChangedEventArgs,
        GlobalDockTrackingChangedEventArgs, WindowActivatedEventArgs, WindowAddedEventArgs,
        WindowClosedEventArgs, WindowClosingEventArgs, WindowDeactivatedEventArgs,
        WindowMoveDragBeginEventArgs, WindowMoveDragEndEventArgs, WindowMoveDragEventArgs,
        WindowOpenedEventArgs, WindowRemovedEventArgs,
    },
    IFactory,
};
use ferroui_base::{metadata::MarkupDelegate, BoxedValue};
use ferroui_system::IDisposable;
use std::rc::Rc;
/// Subscribes to the original ActiveDockableChanged event.
#[doc(hidden)]
pub fn active_dockable_changed(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.active_dockable_changed(Rc::new(move |args: &ActiveDockableChangedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original FocusedDockableChanged event.
#[doc(hidden)]
pub fn focused_dockable_changed(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.focused_dockable_changed(Rc::new(move |args: &FocusedDockableChangedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original GlobalDockTrackingChanged event.
#[doc(hidden)]
pub fn global_dock_tracking_changed(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.global_dock_tracking_changed(Rc::new(
        move |args: &GlobalDockTrackingChangedEventArgs| {
            let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
            handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
        },
    ))
}
/// Subscribes to the original DockableInit event.
#[doc(hidden)]
pub fn dockable_init(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_init(Rc::new(move |args: &DockableInitEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableAdded event.
#[doc(hidden)]
pub fn dockable_added(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_added(Rc::new(move |args: &DockableAddedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableRemoved event.
#[doc(hidden)]
pub fn dockable_removed(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_removed(Rc::new(move |args: &DockableRemovedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableClosing event.
#[doc(hidden)]
pub fn dockable_closing(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_closing(Rc::new(move |args: &DockableClosingEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableClosed event.
#[doc(hidden)]
pub fn dockable_closed(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_closed(Rc::new(move |args: &DockableClosedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableMoved event.
#[doc(hidden)]
pub fn dockable_moved(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_moved(Rc::new(move |args: &DockableMovedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableDocked event.
#[doc(hidden)]
pub fn dockable_docked(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_docked(Rc::new(move |args: &DockableDockedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableUndocked event.
#[doc(hidden)]
pub fn dockable_undocked(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_undocked(Rc::new(move |args: &DockableUndockedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableSwapped event.
#[doc(hidden)]
pub fn dockable_swapped(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_swapped(Rc::new(move |args: &DockableSwappedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockablePinned event.
#[doc(hidden)]
pub fn dockable_pinned(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_pinned(Rc::new(move |args: &DockablePinnedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableUnpinned event.
#[doc(hidden)]
pub fn dockable_unpinned(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_unpinned(Rc::new(move |args: &DockableUnpinnedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableHidden event.
#[doc(hidden)]
pub fn dockable_hidden(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_hidden(Rc::new(move |args: &DockableHiddenEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableRestored event.
#[doc(hidden)]
pub fn dockable_restored(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_restored(Rc::new(move |args: &DockableRestoredEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowOpened event.
#[doc(hidden)]
pub fn window_opened(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_opened(Rc::new(move |args: &WindowOpenedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowClosing event.
#[doc(hidden)]
pub fn window_closing(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_closing(Rc::new(move |args: &WindowClosingEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowClosed event.
#[doc(hidden)]
pub fn window_closed(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_closed(Rc::new(move |args: &WindowClosedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowAdded event.
#[doc(hidden)]
pub fn window_added(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_added(Rc::new(move |args: &WindowAddedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowRemoved event.
#[doc(hidden)]
pub fn window_removed(factory: &Rc<dyn IFactory>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_removed(Rc::new(move |args: &WindowRemovedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowMoveDragBegin event.
#[doc(hidden)]
pub fn window_move_drag_begin(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_move_drag_begin(Rc::new(move |args: &WindowMoveDragBeginEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowMoveDrag event.
#[doc(hidden)]
pub fn window_move_drag(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_move_drag(Rc::new(move |args: &WindowMoveDragEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowMoveDragEnd event.
#[doc(hidden)]
pub fn window_move_drag_end(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_move_drag_end(Rc::new(move |args: &WindowMoveDragEndEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowActivated event.
#[doc(hidden)]
pub fn window_activated(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_activated(Rc::new(move |args: &WindowActivatedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableActivated event.
#[doc(hidden)]
pub fn dockable_activated(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_activated(Rc::new(move |args: &DockableActivatedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original WindowDeactivated event.
#[doc(hidden)]
pub fn window_deactivated(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.window_deactivated(Rc::new(move |args: &WindowDeactivatedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
/// Subscribes to the original DockableDeactivated event.
#[doc(hidden)]
pub fn dockable_deactivated(
    factory: &Rc<dyn IFactory>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = Rc::downgrade(factory);
    factory.dockable_deactivated(Rc::new(move |args: &DockableDeactivatedEventArgs| {
        let sender = sender.upgrade().map(|value| Rc::new(value) as BoxedValue);
        handler.invoke(&[sender, Some(Rc::new(args.clone()))]);
    }))
}
