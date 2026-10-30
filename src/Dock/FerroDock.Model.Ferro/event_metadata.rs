// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native metadata over the actual shared source event arguments and tracking state.
use ferrodock_model::{
    controls::IRootDock,
    core::{
        events::{
            ActiveDockableChangedEventArgs, DockableActivatedEventArgs, DockableAddedEventArgs,
            DockableClosedEventArgs, DockableClosingEventArgs, DockableDeactivatedEventArgs,
            DockableDockedEventArgs, DockableHiddenEventArgs, DockableInitEventArgs,
            DockableMovedEventArgs, DockableOwnerChangingEventArgs, DockablePinnedEventArgs,
            DockableRemovedEventArgs, DockableRestoredEventArgs, DockableSwappedEventArgs,
            DockableUndockedEventArgs, DockableUnpinnedEventArgs, FocusedDockableChangedEventArgs,
            GlobalDockTrackingChangedEventArgs, WindowActivatedEventArgs, WindowAddedEventArgs,
            WindowClosedEventArgs, WindowClosingEventArgs, WindowDeactivatedEventArgs,
            WindowMoveDragBeginEventArgs, WindowMoveDragEndEventArgs, WindowMoveDragEventArgs,
            WindowOpenedEventArgs, WindowRemovedEventArgs,
        },
        DockOperation, DockTrackingChangeReason, GlobalDockTrackingState, IDockWindow, IDockable,
        IHostWindow,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
    BoxedValue,
};
use std::rc::Rc;
/// Descriptor of the original ActiveDockableChangedEventArgs class.
#[doc(hidden)]
pub trait ActiveDockableChangedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn ActiveDockableChangedEventArgsMetadata as "ActiveDockableChangedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:ActiveDockableChangedEventArgs,handles:[ActiveDockableChangedEventArgs,Option<ActiveDockableChangedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => ActiveDockableChangedEventArgs::new,(Option<Rc<dyn IDockable>>,Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>) => ActiveDockableChangedEventArgs::with_context],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&ActiveDockableChangedEventArgs|this.dockable()},
RootDock:Option<Rc<dyn IRootDock>>{get:|this:&ActiveDockableChangedEventArgs|this.root_dock()},
Window:Option<Rc<dyn IDockWindow>>{get:|this:&ActiveDockableChangedEventArgs|this.window()},
HostWindow:Option<Rc<dyn IHostWindow>>{get:|this:&ActiveDockableChangedEventArgs|this.host_window()},
 ]});
/// Descriptor of the original DockableActivatedEventArgs class.
#[doc(hidden)]
pub trait DockableActivatedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableActivatedEventArgsMetadata as "DockableActivatedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableActivatedEventArgs,handles:[DockableActivatedEventArgs,Option<DockableActivatedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableActivatedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableActivatedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableAddedEventArgs class.
#[doc(hidden)]
pub trait DockableAddedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableAddedEventArgsMetadata as "DockableAddedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableAddedEventArgs,handles:[DockableAddedEventArgs,Option<DockableAddedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableAddedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableAddedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableClosedEventArgs class.
#[doc(hidden)]
pub trait DockableClosedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableClosedEventArgsMetadata as "DockableClosedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableClosedEventArgs,handles:[DockableClosedEventArgs,Option<DockableClosedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableClosedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableClosedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableClosingEventArgs class.
#[doc(hidden)]
pub trait DockableClosingEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableClosingEventArgsMetadata as "DockableClosingEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableClosingEventArgs,handles:[DockableClosingEventArgs,Option<DockableClosingEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableClosingEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableClosingEventArgs|this.dockable()},
Cancel:bool{get:|this:&DockableClosingEventArgs|this.cancel(),set:|this:&DockableClosingEventArgs,value:bool|this.set_cancel(value)},
 ]});
/// Descriptor of the original DockableDeactivatedEventArgs class.
#[doc(hidden)]
pub trait DockableDeactivatedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableDeactivatedEventArgsMetadata as "DockableDeactivatedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableDeactivatedEventArgs,handles:[DockableDeactivatedEventArgs,Option<DockableDeactivatedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableDeactivatedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableDeactivatedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableDockedEventArgs class.
#[doc(hidden)]
pub trait DockableDockedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableDockedEventArgsMetadata as "DockableDockedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableDockedEventArgs,handles:[DockableDockedEventArgs,Option<DockableDockedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>,DockOperation) => DockableDockedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableDockedEventArgs|this.dockable()},
Operation:DockOperation{get:|this:&DockableDockedEventArgs|this.operation()},
 ]});
/// Descriptor of the original DockableHiddenEventArgs class.
#[doc(hidden)]
pub trait DockableHiddenEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableHiddenEventArgsMetadata as "DockableHiddenEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableHiddenEventArgs,handles:[DockableHiddenEventArgs,Option<DockableHiddenEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableHiddenEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableHiddenEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableInitEventArgs class.
#[doc(hidden)]
pub trait DockableInitEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableInitEventArgsMetadata as "DockableInitEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableInitEventArgs,handles:[DockableInitEventArgs,Option<DockableInitEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableInitEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableInitEventArgs|this.dockable()},
Context:Option<BoxedValue>{get:|this:&DockableInitEventArgs|this.context().as_ref().map(ferrodock_runtime::from_source_value),set:|this:&DockableInitEventArgs,value:Option<BoxedValue>|this.set_context(value.map(ferrodock_runtime::to_source_value))},
 ]});
/// Descriptor of the original DockableMovedEventArgs class.
#[doc(hidden)]
pub trait DockableMovedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableMovedEventArgsMetadata as "DockableMovedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableMovedEventArgs,handles:[DockableMovedEventArgs,Option<DockableMovedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableMovedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableMovedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableOwnerChangingEventArgs class.
#[doc(hidden)]
pub trait DockableOwnerChangingEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableOwnerChangingEventArgsMetadata as "DockableOwnerChangingEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableOwnerChangingEventArgs,handles:[DockableOwnerChangingEventArgs,Option<DockableOwnerChangingEventArgs>],constructors:[(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => DockableOwnerChangingEventArgs::new],properties:[
OldOwner:Option<Rc<dyn IDockable>>{get:|this:&DockableOwnerChangingEventArgs|this.old_owner()},
NewOwner:Option<Rc<dyn IDockable>>{get:|this:&DockableOwnerChangingEventArgs|this.new_owner()},
 ]});
/// Descriptor of the original DockablePinnedEventArgs class.
#[doc(hidden)]
pub trait DockablePinnedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockablePinnedEventArgsMetadata as "DockablePinnedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockablePinnedEventArgs,handles:[DockablePinnedEventArgs,Option<DockablePinnedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockablePinnedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockablePinnedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableRemovedEventArgs class.
#[doc(hidden)]
pub trait DockableRemovedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableRemovedEventArgsMetadata as "DockableRemovedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableRemovedEventArgs,handles:[DockableRemovedEventArgs,Option<DockableRemovedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableRemovedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableRemovedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableRestoredEventArgs class.
#[doc(hidden)]
pub trait DockableRestoredEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableRestoredEventArgsMetadata as "DockableRestoredEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableRestoredEventArgs,handles:[DockableRestoredEventArgs,Option<DockableRestoredEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableRestoredEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableRestoredEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableSwappedEventArgs class.
#[doc(hidden)]
pub trait DockableSwappedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableSwappedEventArgsMetadata as "DockableSwappedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableSwappedEventArgs,handles:[DockableSwappedEventArgs,Option<DockableSwappedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableSwappedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableSwappedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original DockableUndockedEventArgs class.
#[doc(hidden)]
pub trait DockableUndockedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableUndockedEventArgsMetadata as "DockableUndockedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableUndockedEventArgs,handles:[DockableUndockedEventArgs,Option<DockableUndockedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>,DockOperation) => DockableUndockedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableUndockedEventArgs|this.dockable()},
Operation:DockOperation{get:|this:&DockableUndockedEventArgs|this.operation()},
 ]});
/// Descriptor of the original DockableUnpinnedEventArgs class.
#[doc(hidden)]
pub trait DockableUnpinnedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn DockableUnpinnedEventArgsMetadata as "DockableUnpinnedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:DockableUnpinnedEventArgs,handles:[DockableUnpinnedEventArgs,Option<DockableUnpinnedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => DockableUnpinnedEventArgs::new],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&DockableUnpinnedEventArgs|this.dockable()},
 ]});
/// Descriptor of the original FocusedDockableChangedEventArgs class.
#[doc(hidden)]
pub trait FocusedDockableChangedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn FocusedDockableChangedEventArgsMetadata as "FocusedDockableChangedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:FocusedDockableChangedEventArgs,handles:[FocusedDockableChangedEventArgs,Option<FocusedDockableChangedEventArgs>],constructors:[(Option<Rc<dyn IDockable>>) => FocusedDockableChangedEventArgs::new,(Option<Rc<dyn IDockable>>,Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>) => FocusedDockableChangedEventArgs::with_context],properties:[
Dockable:Option<Rc<dyn IDockable>>{get:|this:&FocusedDockableChangedEventArgs|this.dockable()},
RootDock:Option<Rc<dyn IRootDock>>{get:|this:&FocusedDockableChangedEventArgs|this.root_dock()},
Window:Option<Rc<dyn IDockWindow>>{get:|this:&FocusedDockableChangedEventArgs|this.window()},
HostWindow:Option<Rc<dyn IHostWindow>>{get:|this:&FocusedDockableChangedEventArgs|this.host_window()},
 ]});
/// Descriptor of the original GlobalDockTrackingChangedEventArgs class.
#[doc(hidden)]
pub trait GlobalDockTrackingChangedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn GlobalDockTrackingChangedEventArgsMetadata as "GlobalDockTrackingChangedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:GlobalDockTrackingChangedEventArgs,handles:[GlobalDockTrackingChangedEventArgs,Option<GlobalDockTrackingChangedEventArgs>],constructors:[(GlobalDockTrackingState,GlobalDockTrackingState,DockTrackingChangeReason) => GlobalDockTrackingChangedEventArgs::new],properties:[
Previous:GlobalDockTrackingState{get:|this:&GlobalDockTrackingChangedEventArgs|this.previous()},
Current:GlobalDockTrackingState{get:|this:&GlobalDockTrackingChangedEventArgs|this.current()},
Reason:DockTrackingChangeReason{get:|this:&GlobalDockTrackingChangedEventArgs|this.reason()},
 ]});
/// Descriptor of the original WindowActivatedEventArgs class.
#[doc(hidden)]
pub trait WindowActivatedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowActivatedEventArgsMetadata as "WindowActivatedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowActivatedEventArgs,handles:[WindowActivatedEventArgs,Option<WindowActivatedEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowActivatedEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowActivatedEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowAddedEventArgs class.
#[doc(hidden)]
pub trait WindowAddedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowAddedEventArgsMetadata as "WindowAddedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowAddedEventArgs,handles:[WindowAddedEventArgs,Option<WindowAddedEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowAddedEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowAddedEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowClosedEventArgs class.
#[doc(hidden)]
pub trait WindowClosedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowClosedEventArgsMetadata as "WindowClosedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowClosedEventArgs,handles:[WindowClosedEventArgs,Option<WindowClosedEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowClosedEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowClosedEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowClosingEventArgs class.
#[doc(hidden)]
pub trait WindowClosingEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowClosingEventArgsMetadata as "WindowClosingEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowClosingEventArgs,handles:[WindowClosingEventArgs,Option<WindowClosingEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowClosingEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowClosingEventArgs|this.window()},
Cancel:bool{get:|this:&WindowClosingEventArgs|this.cancel(),set:|this:&WindowClosingEventArgs,value:bool|this.set_cancel(value)},
 ]});
/// Descriptor of the original WindowDeactivatedEventArgs class.
#[doc(hidden)]
pub trait WindowDeactivatedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowDeactivatedEventArgsMetadata as "WindowDeactivatedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowDeactivatedEventArgs,handles:[WindowDeactivatedEventArgs,Option<WindowDeactivatedEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowDeactivatedEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowDeactivatedEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowMoveDragBeginEventArgs class.
#[doc(hidden)]
pub trait WindowMoveDragBeginEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowMoveDragBeginEventArgsMetadata as "WindowMoveDragBeginEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowMoveDragBeginEventArgs,handles:[WindowMoveDragBeginEventArgs,Option<WindowMoveDragBeginEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowMoveDragBeginEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowMoveDragBeginEventArgs|this.window()},
Cancel:bool{get:|this:&WindowMoveDragBeginEventArgs|this.cancel(),set:|this:&WindowMoveDragBeginEventArgs,value:bool|this.set_cancel(value)},
 ]});
/// Descriptor of the original WindowMoveDragEndEventArgs class.
#[doc(hidden)]
pub trait WindowMoveDragEndEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowMoveDragEndEventArgsMetadata as "WindowMoveDragEndEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowMoveDragEndEventArgs,handles:[WindowMoveDragEndEventArgs,Option<WindowMoveDragEndEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowMoveDragEndEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowMoveDragEndEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowMoveDragEventArgs class.
#[doc(hidden)]
pub trait WindowMoveDragEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowMoveDragEventArgsMetadata as "WindowMoveDragEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowMoveDragEventArgs,handles:[WindowMoveDragEventArgs,Option<WindowMoveDragEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowMoveDragEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowMoveDragEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowOpenedEventArgs class.
#[doc(hidden)]
pub trait WindowOpenedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowOpenedEventArgsMetadata as "WindowOpenedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowOpenedEventArgs,handles:[WindowOpenedEventArgs,Option<WindowOpenedEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowOpenedEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowOpenedEventArgs|this.window()},
 ]});
/// Descriptor of the original WindowRemovedEventArgs class.
#[doc(hidden)]
pub trait WindowRemovedEventArgsMetadata {}
ferroui_base::ferro_markup_type!(class dyn WindowRemovedEventArgsMetadata as "WindowRemovedEventArgs" {namespace:"FerroDock.Model.Core.Events",this:WindowRemovedEventArgs,handles:[WindowRemovedEventArgs,Option<WindowRemovedEventArgs>],constructors:[(Option<Rc<dyn IDockWindow>>) => WindowRemovedEventArgs::new],properties:[
Window:Option<Rc<dyn IDockWindow>>{get:|this:&WindowRemovedEventArgs|this.window()},
 ]});
/// Descriptor of the original weak tracking snapshot class.
#[doc(hidden)]
pub trait GlobalDockTrackingStateMetadata {}
ferroui_base::ferro_markup_type!(class dyn GlobalDockTrackingStateMetadata as "GlobalDockTrackingState"{
 namespace:"FerroDock.Model.Core",this:GlobalDockTrackingState,handles:[GlobalDockTrackingState,Option<GlobalDockTrackingState>],
 constructors:[(Option<Rc<dyn IDockable>>,Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>)=>GlobalDockTrackingState::new],
 static_properties:[Empty:GlobalDockTrackingState{get:GlobalDockTrackingState::empty}],
 properties:[Dockable:Option<Rc<dyn IDockable>>{get:GlobalDockTrackingState::dockable},RootDock:Option<Rc<dyn IRootDock>>{get:GlobalDockTrackingState::root_dock},Window:Option<Rc<dyn IDockWindow>>{get:GlobalDockTrackingState::window},HostWindow:Option<Rc<dyn IHostWindow>>{get:GlobalDockTrackingState::host_window}],
});
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(<dyn ActiveDockableChangedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<ActiveDockableChangedEventArgs>();
    ValueTypes::register_reference_identity::<ActiveDockableChangedEventArgs>(
        ActiveDockableChangedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableActivatedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableActivatedEventArgs>();
    ValueTypes::register_reference_identity::<DockableActivatedEventArgs>(
        DockableActivatedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableAddedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableAddedEventArgs>();
    ValueTypes::register_reference_identity::<DockableAddedEventArgs>(
        DockableAddedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableClosedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableClosedEventArgs>();
    ValueTypes::register_reference_identity::<DockableClosedEventArgs>(
        DockableClosedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableClosingEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableClosingEventArgs>();
    ValueTypes::register_reference_identity::<DockableClosingEventArgs>(
        DockableClosingEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableDeactivatedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableDeactivatedEventArgs>();
    ValueTypes::register_reference_identity::<DockableDeactivatedEventArgs>(
        DockableDeactivatedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableDockedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableDockedEventArgs>();
    ValueTypes::register_reference_identity::<DockableDockedEventArgs>(
        DockableDockedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableHiddenEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableHiddenEventArgs>();
    ValueTypes::register_reference_identity::<DockableHiddenEventArgs>(
        DockableHiddenEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableInitEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableInitEventArgs>();
    ValueTypes::register_reference_identity::<DockableInitEventArgs>(
        DockableInitEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableMovedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableMovedEventArgs>();
    ValueTypes::register_reference_identity::<DockableMovedEventArgs>(
        DockableMovedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableOwnerChangingEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableOwnerChangingEventArgs>();
    ValueTypes::register_reference_identity::<DockableOwnerChangingEventArgs>(
        DockableOwnerChangingEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockablePinnedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockablePinnedEventArgs>();
    ValueTypes::register_reference_identity::<DockablePinnedEventArgs>(
        DockablePinnedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableRemovedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableRemovedEventArgs>();
    ValueTypes::register_reference_identity::<DockableRemovedEventArgs>(
        DockableRemovedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableRestoredEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableRestoredEventArgs>();
    ValueTypes::register_reference_identity::<DockableRestoredEventArgs>(
        DockableRestoredEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableSwappedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableSwappedEventArgs>();
    ValueTypes::register_reference_identity::<DockableSwappedEventArgs>(
        DockableSwappedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableUndockedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableUndockedEventArgs>();
    ValueTypes::register_reference_identity::<DockableUndockedEventArgs>(
        DockableUndockedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn DockableUnpinnedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DockableUnpinnedEventArgs>();
    ValueTypes::register_reference_identity::<DockableUnpinnedEventArgs>(
        DockableUnpinnedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn FocusedDockableChangedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<FocusedDockableChangedEventArgs>();
    ValueTypes::register_reference_identity::<FocusedDockableChangedEventArgs>(
        FocusedDockableChangedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn GlobalDockTrackingChangedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<GlobalDockTrackingChangedEventArgs>();
    ValueTypes::register_reference_identity::<GlobalDockTrackingChangedEventArgs>(
        GlobalDockTrackingChangedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowActivatedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowActivatedEventArgs>();
    ValueTypes::register_reference_identity::<WindowActivatedEventArgs>(
        WindowActivatedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowAddedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowAddedEventArgs>();
    ValueTypes::register_reference_identity::<WindowAddedEventArgs>(
        WindowAddedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowClosedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowClosedEventArgs>();
    ValueTypes::register_reference_identity::<WindowClosedEventArgs>(
        WindowClosedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowClosingEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowClosingEventArgs>();
    ValueTypes::register_reference_identity::<WindowClosingEventArgs>(
        WindowClosingEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowDeactivatedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowDeactivatedEventArgs>();
    ValueTypes::register_reference_identity::<WindowDeactivatedEventArgs>(
        WindowDeactivatedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowMoveDragBeginEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowMoveDragBeginEventArgs>();
    ValueTypes::register_reference_identity::<WindowMoveDragBeginEventArgs>(
        WindowMoveDragBeginEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowMoveDragEndEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowMoveDragEndEventArgs>();
    ValueTypes::register_reference_identity::<WindowMoveDragEndEventArgs>(
        WindowMoveDragEndEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowMoveDragEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowMoveDragEventArgs>();
    ValueTypes::register_reference_identity::<WindowMoveDragEventArgs>(
        WindowMoveDragEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowOpenedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowOpenedEventArgs>();
    ValueTypes::register_reference_identity::<WindowOpenedEventArgs>(
        WindowOpenedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn WindowRemovedEventArgsMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<WindowRemovedEventArgs>();
    ValueTypes::register_reference_identity::<WindowRemovedEventArgs>(
        WindowRemovedEventArgs::reference_identity,
    );
    MarkupType::register(<dyn GlobalDockTrackingStateMetadata as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<GlobalDockTrackingState>();
    ValueTypes::register_reference_identity::<GlobalDockTrackingState>(
        GlobalDockTrackingState::reference_identity,
    );
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[
    (||std::any::TypeId::of::<ActiveDockableChangedEventArgs>(),"::ferrodock_model::core::events::ActiveDockableChangedEventArgs"),
    (||std::any::TypeId::of::<DockableActivatedEventArgs>(),"::ferrodock_model::core::events::DockableActivatedEventArgs"),
    (||std::any::TypeId::of::<DockableAddedEventArgs>(),"::ferrodock_model::core::events::DockableAddedEventArgs"),
    (||std::any::TypeId::of::<DockableClosedEventArgs>(),"::ferrodock_model::core::events::DockableClosedEventArgs"),
    (||std::any::TypeId::of::<DockableClosingEventArgs>(),"::ferrodock_model::core::events::DockableClosingEventArgs"),
    (||std::any::TypeId::of::<DockableDeactivatedEventArgs>(),"::ferrodock_model::core::events::DockableDeactivatedEventArgs"),
    (||std::any::TypeId::of::<DockableDockedEventArgs>(),"::ferrodock_model::core::events::DockableDockedEventArgs"),
    (||std::any::TypeId::of::<DockableHiddenEventArgs>(),"::ferrodock_model::core::events::DockableHiddenEventArgs"),
    (||std::any::TypeId::of::<DockableInitEventArgs>(),"::ferrodock_model::core::events::DockableInitEventArgs"),
    (||std::any::TypeId::of::<DockableMovedEventArgs>(),"::ferrodock_model::core::events::DockableMovedEventArgs"),
    (||std::any::TypeId::of::<DockableOwnerChangingEventArgs>(),"::ferrodock_model::core::events::DockableOwnerChangingEventArgs"),
    (||std::any::TypeId::of::<DockablePinnedEventArgs>(),"::ferrodock_model::core::events::DockablePinnedEventArgs"),
    (||std::any::TypeId::of::<DockableRemovedEventArgs>(),"::ferrodock_model::core::events::DockableRemovedEventArgs"),
    (||std::any::TypeId::of::<DockableRestoredEventArgs>(),"::ferrodock_model::core::events::DockableRestoredEventArgs"),
    (||std::any::TypeId::of::<DockableSwappedEventArgs>(),"::ferrodock_model::core::events::DockableSwappedEventArgs"),
    (||std::any::TypeId::of::<DockableUndockedEventArgs>(),"::ferrodock_model::core::events::DockableUndockedEventArgs"),
    (||std::any::TypeId::of::<DockableUnpinnedEventArgs>(),"::ferrodock_model::core::events::DockableUnpinnedEventArgs"),
    (||std::any::TypeId::of::<FocusedDockableChangedEventArgs>(),"::ferrodock_model::core::events::FocusedDockableChangedEventArgs"),
    (||std::any::TypeId::of::<GlobalDockTrackingChangedEventArgs>(),"::ferrodock_model::core::events::GlobalDockTrackingChangedEventArgs"),
    (||std::any::TypeId::of::<WindowActivatedEventArgs>(),"::ferrodock_model::core::events::WindowActivatedEventArgs"),
    (||std::any::TypeId::of::<WindowAddedEventArgs>(),"::ferrodock_model::core::events::WindowAddedEventArgs"),
    (||std::any::TypeId::of::<WindowClosedEventArgs>(),"::ferrodock_model::core::events::WindowClosedEventArgs"),
    (||std::any::TypeId::of::<WindowClosingEventArgs>(),"::ferrodock_model::core::events::WindowClosingEventArgs"),
    (||std::any::TypeId::of::<WindowDeactivatedEventArgs>(),"::ferrodock_model::core::events::WindowDeactivatedEventArgs"),
    (||std::any::TypeId::of::<WindowMoveDragBeginEventArgs>(),"::ferrodock_model::core::events::WindowMoveDragBeginEventArgs"),
    (||std::any::TypeId::of::<WindowMoveDragEndEventArgs>(),"::ferrodock_model::core::events::WindowMoveDragEndEventArgs"),
    (||std::any::TypeId::of::<WindowMoveDragEventArgs>(),"::ferrodock_model::core::events::WindowMoveDragEventArgs"),
    (||std::any::TypeId::of::<WindowOpenedEventArgs>(),"::ferrodock_model::core::events::WindowOpenedEventArgs"),
    (||std::any::TypeId::of::<WindowRemovedEventArgs>(),"::ferrodock_model::core::events::WindowRemovedEventArgs"),
    (||std::any::TypeId::of::<GlobalDockTrackingState>(),"::ferrodock_model::core::GlobalDockTrackingState"),
    ]);}
}
ferroui_base::ferro_rust_paths! {classes:[],types:[],contracts:[crate::event_metadata::ActiveDockableChangedEventArgsMetadata,crate::event_metadata::DockableActivatedEventArgsMetadata,crate::event_metadata::DockableAddedEventArgsMetadata,crate::event_metadata::DockableClosedEventArgsMetadata,crate::event_metadata::DockableClosingEventArgsMetadata,crate::event_metadata::DockableDeactivatedEventArgsMetadata,crate::event_metadata::DockableDockedEventArgsMetadata,crate::event_metadata::DockableHiddenEventArgsMetadata,crate::event_metadata::DockableInitEventArgsMetadata,crate::event_metadata::DockableMovedEventArgsMetadata,crate::event_metadata::DockableOwnerChangingEventArgsMetadata,crate::event_metadata::DockablePinnedEventArgsMetadata,crate::event_metadata::DockableRemovedEventArgsMetadata,crate::event_metadata::DockableRestoredEventArgsMetadata,crate::event_metadata::DockableSwappedEventArgsMetadata,crate::event_metadata::DockableUndockedEventArgsMetadata,crate::event_metadata::DockableUnpinnedEventArgsMetadata,crate::event_metadata::FocusedDockableChangedEventArgsMetadata,crate::event_metadata::GlobalDockTrackingChangedEventArgsMetadata,crate::event_metadata::WindowActivatedEventArgsMetadata,crate::event_metadata::WindowAddedEventArgsMetadata,crate::event_metadata::WindowClosedEventArgsMetadata,crate::event_metadata::WindowClosingEventArgsMetadata,crate::event_metadata::WindowDeactivatedEventArgsMetadata,crate::event_metadata::WindowMoveDragBeginEventArgsMetadata,crate::event_metadata::WindowMoveDragEndEventArgsMetadata,crate::event_metadata::WindowMoveDragEventArgsMetadata,crate::event_metadata::WindowOpenedEventArgsMetadata,crate::event_metadata::WindowRemovedEventArgsMetadata,crate::event_metadata::GlobalDockTrackingStateMetadata],generics:[],generic_contracts:[]}
