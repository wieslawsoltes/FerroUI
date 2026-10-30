// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    ContextFactoryReference, ContextLocatorMapReference, DockableLocatorMapReference,
    HostWindowFactoryReference, HostWindowLocatorMapReference,
};
use crate::{DockableControlMapReference, DockableObjectMapReference};
use crate::{DockablePredicateReference, DockableQueryReference, RootPredicateReference};
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
    DockOperation,
};
use ferrodock_model::core::{GlobalDockTrackingState, IHostWindow};
use ferrodock_model::{
    collections::ListHandle,
    controls::{
        IDockDock, IDocument, IDocumentDock, IGridDock, IGridDockSplitter, IProportionalDock,
        IProportionalDockSplitter, ISplitViewDock, IStackDock, ITool, IToolDock, IUniformGridDock,
        IWrapDock,
    },
    core::{IDock, IDockControl},
};
use ferrodock_model::{
    controls::IRootDock,
    core::{
        DockWindowOptions, DocumentLayoutMode, DocumentTabLayout, IDockWindow, IDockable, IFactory,
    },
};
use ferroui_base::metadata::MarkupDelegate;
use ferroui_base::{data::core::ValueTypes, BoxedValue};
use std::rc::Rc;
/// Descriptor carrier retaining the original factory; non-command members are still being ported.
#[doc(hidden)]
pub trait IFactoryMetadata {}
pub(crate) fn parameter(value: Option<BoxedValue>) -> Option<ferrodock_model::BoxedValue> {
    let value = value.and_then(ValueTypes::normalize)?;
    if let Some(dockable) =
        ferroui_base::metadata::from_markup_value::<Rc<dyn IDockable>>(&Some(value.clone()))
    {
        return Some(Rc::new(dockable));
    }
    // Command parameters only cross into Domain here; an ordinary value can
    // upcast directly to Any, retaining its identity and concrete type.
    Some(value)
}
ferroui_base::ferro_markup_type!(interface dyn IFactoryMetadata as "IFactory" {
    namespace:"FerroDock.Model.Core",this:Rc<dyn IFactory>,handles:[Rc<dyn IFactory>,Option<Rc<dyn IFactory>>],
    properties:[
GlobalDockTrackingState:GlobalDockTrackingState{get:|this:&Rc<dyn IFactory>|this.global_dock_tracking_state()},
CurrentDockable:Option<Rc<dyn IDockable>>{get:|this:&Rc<dyn IFactory>|this.current_dockable()},
CurrentRootDock:Option<Rc<dyn IRootDock>>{get:|this:&Rc<dyn IFactory>|this.current_root_dock()},
CurrentDockWindow:Option<Rc<dyn IDockWindow>>{get:|this:&Rc<dyn IFactory>|this.current_dock_window()},
CurrentHostWindow:Option<Rc<dyn IHostWindow>>{get:|this:&Rc<dyn IFactory>|this.current_host_window()},
DefaultContextLocator:Option<ContextFactoryReference>{get:|this:&Rc<dyn IFactory>|this.default_context_locator().map(ContextFactoryReference::new),set:|this:&Rc<dyn IFactory>,value:Option<ContextFactoryReference>|this.set_default_context_locator(value.map(|value|value.model()))},
DefaultHostWindowLocator:Option<HostWindowFactoryReference>{get:|this:&Rc<dyn IFactory>|this.default_host_window_locator().map(HostWindowFactoryReference::new),set:|this:&Rc<dyn IFactory>,value:Option<HostWindowFactoryReference>|this.set_default_host_window_locator(value.map(|value|value.model()))},
ContextLocator:Option<ContextLocatorMapReference>{get:|this:&Rc<dyn IFactory>|this.context_locator().map(ContextLocatorMapReference::new),set:|this:&Rc<dyn IFactory>,value:Option<ContextLocatorMapReference>|this.set_context_locator(value.map(|value|value.model()))},
HostWindowLocator:Option<HostWindowLocatorMapReference>{get:|this:&Rc<dyn IFactory>|this.host_window_locator().map(HostWindowLocatorMapReference::new),set:|this:&Rc<dyn IFactory>,value:Option<HostWindowLocatorMapReference>|this.set_host_window_locator(value.map(|value|value.model()))},
DockableLocator:Option<DockableLocatorMapReference>{get:|this:&Rc<dyn IFactory>|this.dockable_locator().map(DockableLocatorMapReference::new),set:|this:&Rc<dyn IFactory>,value:Option<DockableLocatorMapReference>|this.set_dockable_locator(value.map(|value|value.model()))},

        HideToolsOnClose:bool{get:|this:&Rc<dyn IFactory>|this.hide_tools_on_close(),set:|this:&Rc<dyn IFactory>,value|this.set_hide_tools_on_close(value)},
        HideDocumentsOnClose:bool{get:|this:&Rc<dyn IFactory>|this.hide_documents_on_close(),set:|this:&Rc<dyn IFactory>,value|this.set_hide_documents_on_close(value)},
 VisibleDockableControls:DockableControlMapReference{get:|this:&Rc<dyn IFactory>|DockableControlMapReference::new(this.visible_dockable_controls())},
 VisibleRootControls:DockableObjectMapReference{get:|this:&Rc<dyn IFactory>|DockableObjectMapReference::new(this.visible_root_controls())},
 PinnedDockableControls:DockableControlMapReference{get:|this:&Rc<dyn IFactory>|DockableControlMapReference::new(this.pinned_dockable_controls())},
 PinnedRootControls:DockableObjectMapReference{get:|this:&Rc<dyn IFactory>|DockableObjectMapReference::new(this.pinned_root_controls())},
 TabDockableControls:DockableControlMapReference{get:|this:&Rc<dyn IFactory>|DockableControlMapReference::new(this.tab_dockable_controls())},
 TabRootControls:DockableObjectMapReference{get:|this:&Rc<dyn IFactory>|DockableObjectMapReference::new(this.tab_root_controls())},
 ToolControls:DockableObjectMapReference{get:|this:&Rc<dyn IFactory>|DockableObjectMapReference::new(this.tool_controls())},
 DocumentControls:DockableObjectMapReference{get:|this:&Rc<dyn IFactory>|DockableObjectMapReference::new(this.document_controls())},
 DockControls:ListHandle<Rc<dyn IDockControl>>{get:|this:&Rc<dyn IFactory>|this.dock_controls()},
 HostWindows:ListHandle<Rc<dyn IHostWindow>>{get:|this:&Rc<dyn IFactory>|this.host_windows()},
    ],
events:[
ActiveDockableChanged(Option<ferroui_base::BoxedValue>,ActiveDockableChangedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::active_dockable_changed(this,handler)),
FocusedDockableChanged(Option<ferroui_base::BoxedValue>,FocusedDockableChangedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::focused_dockable_changed(this,handler)),
GlobalDockTrackingChanged(Option<ferroui_base::BoxedValue>,GlobalDockTrackingChangedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::global_dock_tracking_changed(this,handler)),
DockableInit(Option<ferroui_base::BoxedValue>,DockableInitEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_init(this,handler)),
DockableAdded(Option<ferroui_base::BoxedValue>,DockableAddedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_added(this,handler)),
DockableRemoved(Option<ferroui_base::BoxedValue>,DockableRemovedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_removed(this,handler)),
DockableClosing(Option<ferroui_base::BoxedValue>,DockableClosingEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_closing(this,handler)),
DockableClosed(Option<ferroui_base::BoxedValue>,DockableClosedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_closed(this,handler)),
DockableMoved(Option<ferroui_base::BoxedValue>,DockableMovedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_moved(this,handler)),
DockableDocked(Option<ferroui_base::BoxedValue>,DockableDockedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_docked(this,handler)),
DockableUndocked(Option<ferroui_base::BoxedValue>,DockableUndockedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_undocked(this,handler)),
DockableSwapped(Option<ferroui_base::BoxedValue>,DockableSwappedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_swapped(this,handler)),
DockablePinned(Option<ferroui_base::BoxedValue>,DockablePinnedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_pinned(this,handler)),
DockableUnpinned(Option<ferroui_base::BoxedValue>,DockableUnpinnedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_unpinned(this,handler)),
DockableHidden(Option<ferroui_base::BoxedValue>,DockableHiddenEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_hidden(this,handler)),
DockableRestored(Option<ferroui_base::BoxedValue>,DockableRestoredEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_restored(this,handler)),
WindowOpened(Option<ferroui_base::BoxedValue>,WindowOpenedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_opened(this,handler)),
WindowClosing(Option<ferroui_base::BoxedValue>,WindowClosingEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_closing(this,handler)),
WindowClosed(Option<ferroui_base::BoxedValue>,WindowClosedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_closed(this,handler)),
WindowAdded(Option<ferroui_base::BoxedValue>,WindowAddedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_added(this,handler)),
WindowRemoved(Option<ferroui_base::BoxedValue>,WindowRemovedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_removed(this,handler)),
WindowMoveDragBegin(Option<ferroui_base::BoxedValue>,WindowMoveDragBeginEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_move_drag_begin(this,handler)),
WindowMoveDrag(Option<ferroui_base::BoxedValue>,WindowMoveDragEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_move_drag(this,handler)),
WindowMoveDragEnd(Option<ferroui_base::BoxedValue>,WindowMoveDragEndEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_move_drag_end(this,handler)),
WindowActivated(Option<ferroui_base::BoxedValue>,WindowActivatedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_activated(this,handler)),
DockableActivated(Option<ferroui_base::BoxedValue>,DockableActivatedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_activated(this,handler)),
WindowDeactivated(Option<ferroui_base::BoxedValue>,WindowDeactivatedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::window_deactivated(this,handler)),
DockableDeactivated(Option<ferroui_base::BoxedValue>,DockableDeactivatedEventArgs) => (|this:&Rc<dyn IFactory>,handler:MarkupDelegate|crate::factory_events::dockable_deactivated(this,handler)),
],
    methods:[
fn GetContainerFromItem(Option<ferroui_base::BoxedValue>)->Option<Rc<dyn IDockable>> => (|this:&Rc<dyn IFactory>,item|this.get_container_from_item(crate::factory_metadata::parameter(item))),
fn InitDockWindow(Option<Rc<dyn IDockWindow>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IHostWindow>>) => (|this:&Rc<dyn IFactory>,window,owner,host|this.init_dock_window_with_host(window,owner,host)),
fn InitActiveDockable(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDock>>) => (|this:&Rc<dyn IFactory>,dockable,owner|this.init_active_dockable(dockable,owner)),
fn ActivateWindow(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.activate_window(dockable)),
fn InsertDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,i32) => (|this:&Rc<dyn IFactory>,dock,dockable,index|this.insert_dockable(dock,dockable,index)),
fn RemoveDockable(Option<Rc<dyn IDockable>>,bool) => (|this:&Rc<dyn IFactory>,dockable,collapse|this.remove_dockable(dockable,collapse)),
fn MoveDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dock,source,target|this.move_dockable(dock,source,target)),
fn MoveDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,source_dock,target_dock,source,target|this.move_dockable_between_docks(source_dock,target_dock,source,target)),
fn SwapDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dock,source,target|this.swap_dockable(dock,source,target)),
fn SwapDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,source_dock,target_dock,source,target|this.swap_dockable_between_docks(source_dock,target_dock,source,target)),
fn IsDockablePinned(Option<Rc<dyn IDockable>>)->bool => (|this:&Rc<dyn IFactory>,dockable|this.is_dockable_pinned(dockable,None)),
fn IsDockablePinned(Option<Rc<dyn IDockable>>,Option<Rc<dyn IRootDock>>)->bool => (|this:&Rc<dyn IFactory>,dockable,root|this.is_dockable_pinned(dockable,root)),
fn MovePinnedDockable(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,source,target|this.move_pinned_dockable(source,target)),
fn AddWindow(Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,root,window|this.add_window(root,window)),
fn InsertWindow(Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>,i32) => (|this:&Rc<dyn IFactory>,root,window,index|this.insert_window(root,window,index)),
fn RemoveWindow(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.remove_window(window)),
fn CollapseDock(Rc<dyn IDock>) => (|this:&Rc<dyn IFactory>,dock|this.collapse_dock(dock)),
fn CreateSplitLayout(Rc<dyn IDock>,Rc<dyn IDockable>,DockOperation)->Rc<dyn IDock> => (|this:&Rc<dyn IFactory>,dock,dockable,operation|this.create_split_layout(dock,dockable,operation)),
fn SplitToDock(Rc<dyn IDock>,Rc<dyn IDockable>,DockOperation) => (|this:&Rc<dyn IFactory>,dock,dockable,operation|this.split_to_dock(dock,dockable,operation)),
fn CreateWindowFrom(Option<Rc<dyn IDockable>>)->Option<Rc<dyn IDockWindow>> => (|this:&Rc<dyn IFactory>,dockable|this.create_window_from(dockable)),
fn CreateWindowFrom(Option<Rc<dyn IDockable>>,Option<Rc<DockWindowOptions>>)->Option<Rc<dyn IDockWindow>> => (|this:&Rc<dyn IFactory>,dockable,options|this.create_window_from_with_options(dockable,options)),
fn SplitToWindow(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,f64,f64,f64,f64) => (|this:&Rc<dyn IFactory>,dock,dockable,x,y,width,height|this.split_to_window(dock,dockable,x,y,width,height)),
fn SplitToWindow(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,f64,f64,f64,f64,Option<Rc<DockWindowOptions>>) => (|this:&Rc<dyn IFactory>,dock,dockable,x,y,width,height,options|this.split_to_window_with_options(dock,dockable,x,y,width,height,options)),
fn FindRoot(Option<Rc<dyn IDockable>>)->Option<Rc<dyn IRootDock>> => (|this:&Rc<dyn IFactory>,dockable|this.find_root(dockable,None)),
fn FindRoot(Option<Rc<dyn IDockable>>,Option<RootPredicateReference>)->Option<Rc<dyn IRootDock>> => (|this:&Rc<dyn IFactory>,dockable,predicate:Option<RootPredicateReference>|this.find_root(dockable,predicate.map(|value|value.model()))),
fn FindDockable(Option<Rc<dyn IDock>>,Option<DockablePredicateReference>)->Option<Rc<dyn IDockable>> => (|this:&Rc<dyn IFactory>,dock,predicate:Option<DockablePredicateReference>|this.find_dockable(dock,predicate.map(|value|value.model()))),
fn Find(Option<DockablePredicateReference>)->DockableQueryReference => (|this:&Rc<dyn IFactory>,predicate:Option<DockablePredicateReference>|DockableQueryReference::new(this.find(predicate.map(|value|value.model())))),
fn Find(Option<Rc<dyn IDock>>,Option<DockablePredicateReference>)->DockableQueryReference => (|this:&Rc<dyn IFactory>,dock,predicate:Option<DockablePredicateReference>|DockableQueryReference::new(this.find_in_dock(dock,predicate.map(|value|value.model())))),
 fn CreateRootDock()->Rc<dyn IRootDock> =>(|this:&Rc<dyn IFactory>|this.create_root_dock()),
 fn CreateProportionalDock()->Rc<dyn IProportionalDock> =>(|this:&Rc<dyn IFactory>|this.create_proportional_dock()),
 fn CreateDockDock()->Rc<dyn IDockDock> =>(|this:&Rc<dyn IFactory>|this.create_dock_dock()),
 fn CreateStackDock()->Rc<dyn IStackDock> =>(|this:&Rc<dyn IFactory>|this.create_stack_dock()),
 fn CreateGridDock()->Rc<dyn IGridDock> =>(|this:&Rc<dyn IFactory>|this.create_grid_dock()),
 fn CreateWrapDock()->Rc<dyn IWrapDock> =>(|this:&Rc<dyn IFactory>|this.create_wrap_dock()),
 fn CreateUniformGridDock()->Rc<dyn IUniformGridDock> =>(|this:&Rc<dyn IFactory>|this.create_uniform_grid_dock()),
 fn CreateProportionalDockSplitter()->Rc<dyn IProportionalDockSplitter> =>(|this:&Rc<dyn IFactory>|this.create_proportional_dock_splitter()),
 fn CreateGridDockSplitter()->Rc<dyn IGridDockSplitter> =>(|this:&Rc<dyn IFactory>|this.create_grid_dock_splitter()),
 fn CreateToolDock()->Rc<dyn IToolDock> =>(|this:&Rc<dyn IFactory>|this.create_tool_dock()),
 fn CreateDocumentDock()->Rc<dyn IDocumentDock> =>(|this:&Rc<dyn IFactory>|this.create_document_dock()),
 fn CreateSplitViewDock()->Rc<dyn ISplitViewDock> =>(|this:&Rc<dyn IFactory>|this.create_split_view_dock()),
 fn CreateDockWindow()->Rc<dyn IDockWindow> =>(|this:&Rc<dyn IFactory>|this.create_dock_window()),
 fn CreateDocument()->Rc<dyn IDocument> =>(|this:&Rc<dyn IFactory>|this.create_document()),
 fn CreateTool()->Rc<dyn ITool> =>(|this:&Rc<dyn IFactory>|this.create_tool()),
 fn CreateLayout()->Option<Rc<dyn IRootDock>> =>(|this:&Rc<dyn IFactory>|this.create_layout()),
 fn InitLayout(Rc<dyn IDockable>)=>(|this:&Rc<dyn IFactory>,layout|this.init_layout(layout)),
 fn InitDockable(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>)=>(|this:&Rc<dyn IFactory>,dockable,owner|this.init_dockable(dockable,owner)),
 fn InitDockWindow(Option<Rc<dyn IDockWindow>>,Option<Rc<dyn IDockable>>)=>(|this:&Rc<dyn IFactory>,window,owner|this.init_dock_window(window,owner)),
 fn AddDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>)=>(|this:&Rc<dyn IFactory>,dock,value|this.add_dockable(dock,value)),
 fn SetActiveDockable(Option<Rc<dyn IDockable>>)=>(|this:&Rc<dyn IFactory>,value|this.set_active_dockable(value)),
 fn SetFocusedDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>)=>(|this:&Rc<dyn IFactory>,dock,value|this.set_focused_dockable(dock,value)),

fn GetContext(String)->Option<ferroui_base::BoxedValue> => (|this:&Rc<dyn IFactory>,id|this.get_context(id).as_ref().map(ferrodock_runtime::from_source_value)),
 fn GetHostWindow(String)->Option<Rc<dyn IHostWindow>> => (|this:&Rc<dyn IFactory>,id|this.get_host_window(id)),

fn OnActiveDockableChanged(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_active_dockable_changed(dockable)),
fn OnFocusedDockableChanged(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_focused_dockable_changed(dockable)),
fn OnDockableInit(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_init(dockable)),
fn OnDockableAdded(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_added(dockable)),
fn OnDockableRemoved(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_removed(dockable)),
fn OnDockableClosing(Option<Rc<dyn IDockable>>) -> bool => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_closing(dockable)),
fn OnDockableClosed(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_closed(dockable)),
fn OnDockableMoved(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_moved(dockable)),
fn OnDockableDocked(Option<Rc<dyn IDockable>>,DockOperation) => (|this:&Rc<dyn IFactory>,dockable,operation|this.on_dockable_docked(dockable,operation)),
fn OnDockableUndocked(Option<Rc<dyn IDockable>>,DockOperation) => (|this:&Rc<dyn IFactory>,dockable,operation|this.on_dockable_undocked(dockable,operation)),
fn OnDockableSwapped(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_swapped(dockable)),
fn OnDockablePinned(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_pinned(dockable)),
fn OnDockableUnpinned(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_unpinned(dockable)),
fn OnDockableHidden(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_hidden(dockable)),
fn OnDockableRestored(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_restored(dockable)),
fn OnWindowOpened(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_opened(window)),
fn OnWindowClosing(Option<Rc<dyn IDockWindow>>) -> bool => (|this:&Rc<dyn IFactory>,window|this.on_window_closing(window)),
fn OnWindowClosed(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_closed(window)),
fn OnWindowAdded(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_added(window)),
fn OnWindowRemoved(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_removed(window)),
fn OnWindowMoveDragBegin(Option<Rc<dyn IDockWindow>>) -> bool => (|this:&Rc<dyn IFactory>,window|this.on_window_move_drag_begin(window)),
fn OnWindowMoveDrag(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_move_drag(window)),
fn OnWindowMoveDragEnd(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_move_drag_end(window)),
fn OnWindowActivated(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_activated(window)),
fn OnWindowActivated(Option<Rc<dyn IDockWindow>>,Option<Rc<dyn IRootDock>>) => (|this:&Rc<dyn IFactory>,window,root_dock|this.on_window_activated_with_root(window,root_dock)),
fn OnDockableActivated(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_activated(dockable)),
fn OnWindowDeactivated(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,window|this.on_window_deactivated(window)),
fn OnDockableDeactivated(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,dockable|this.on_dockable_deactivated(dockable)),

        fn PinDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.pin_dockable(value)),
        fn PinDockable(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.pin_dockable_parameter(parameter(value))),
        fn PreviewPinnedDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.preview_pinned_dockable(value)),
        fn PreviewPinnedDockable(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.preview_pinned_dockable_parameter(parameter(value))),
        fn TogglePreviewPinnedDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.toggle_preview_pinned_dockable(value)),
        fn TogglePreviewPinnedDockable(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.toggle_preview_pinned_dockable_parameter(parameter(value))),
        fn FloatDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.float_dockable(value)),
        fn FloatDockable(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.float_dockable_parameter(parameter(value))),
        fn FloatAllDockables(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.float_all_dockables(value)),
        fn FloatAllDockables(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.float_all_dockables_parameter(parameter(value))),
        fn DockAsDocument(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.dock_as_document(value)),
        fn DockAsDocument(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.dock_as_document_parameter(parameter(value))),
        fn CloseDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.close_dockable(value)),
        fn CloseDockable(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.close_dockable_parameter(parameter(value))),
        fn CloseOtherDockables(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.close_other_dockables(value)),
        fn CloseOtherDockables(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.close_other_dockables_parameter(parameter(value))),
        fn CloseAllDockables(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.close_all_dockables(value)),
        fn CloseAllDockables(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.close_all_dockables_parameter(parameter(value))),
        fn CloseLeftDockables(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.close_left_dockables(value)),
        fn CloseLeftDockables(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.close_left_dockables_parameter(parameter(value))),
        fn CloseRightDockables(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.close_right_dockables(value)),
        fn CloseRightDockables(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.close_right_dockables_parameter(parameter(value))),
        fn SetDocumentDockTabsLayoutLeft(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_tabs_layout_left(value)),
        fn SetDocumentDockTabsLayoutLeft(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_tabs_layout_left_parameter(parameter(value))),
        fn SetDocumentDockTabsLayoutTop(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_tabs_layout_top(value)),
        fn SetDocumentDockTabsLayoutTop(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_tabs_layout_top_parameter(parameter(value))),
        fn SetDocumentDockTabsLayoutRight(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_tabs_layout_right(value)),
        fn SetDocumentDockTabsLayoutRight(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_tabs_layout_right_parameter(parameter(value))),
        fn SetDocumentDockLayoutModeTabbed(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_layout_mode_tabbed(value)),
        fn SetDocumentDockLayoutModeTabbed(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_layout_mode_tabbed_parameter(parameter(value))),
        fn SetDocumentDockLayoutModeMdi(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_layout_mode_mdi(value)),
        fn SetDocumentDockLayoutModeMdi(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.set_document_dock_layout_mode_mdi_parameter(parameter(value))),
        fn NewHorizontalDocumentDock(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.new_horizontal_document_dock(value)),
        fn NewHorizontalDocumentDock(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.new_horizontal_document_dock_parameter(parameter(value))),
        fn NewVerticalDocumentDock(Rc<dyn IDockable>) => (|this:&Rc<dyn IFactory>,value|this.new_vertical_document_dock(value)),
        fn NewVerticalDocumentDock(Option<BoxedValue>) => (|this:&Rc<dyn IFactory>,value|this.new_vertical_document_dock_parameter(parameter(value))),
        fn UnpinDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.unpin_dockable(value)),
        fn HidePreviewingDockables(Option<Rc<dyn IRootDock>>) => (|this:&Rc<dyn IFactory>,value|this.hide_previewing_dockables(value)),
        fn HideDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.hide_dockable(value)),
        fn HideDockable(String) => (|this:&Rc<dyn IFactory>,value|this.hide_dockable_by_id(value)),
        fn RestoreDockable(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IFactory>,value|this.restore_dockable(value)),
        fn RestoreDockable(String)->Option<Rc<dyn IDockable>> => (|this:&Rc<dyn IFactory>,value|this.restore_dockable_by_id(value)),
        fn CloseWindow(Option<Rc<dyn IDockWindow>>) => (|this:&Rc<dyn IFactory>,value|this.close_window(value)),
        fn FloatDockable(Option<Rc<dyn IDockable>>,Option<Rc<DockWindowOptions>>) => (|this:&Rc<dyn IFactory>,value,options|this.float_dockable_with_options(value,options)),
        fn FloatAllDockables(Option<Rc<dyn IDockable>>,Option<Rc<DockWindowOptions>>) => (|this:&Rc<dyn IFactory>,value,options|this.float_all_dockables_with_options(value,options)),
        fn SetDocumentDockTabsLayout(Rc<dyn IDockable>,DocumentTabLayout) => (|this:&Rc<dyn IFactory>,value,layout|this.set_document_dock_tabs_layout(value,layout)),
        fn SetDocumentDockLayoutMode(Rc<dyn IDockable>,DocumentLayoutMode) => (|this:&Rc<dyn IFactory>,value,layout|this.set_document_dock_layout_mode(value,layout)),
    ],
});

#[cfg(test)]
#[path = "factory_metadata_tests.rs"]
mod tests;

/// Core-owned native descriptor for the original model-layer factory helper class.
#[doc(hidden)]
pub struct FactoryExtensionsMetadata;
ferroui_base::ferro_markup_type!(static FactoryExtensionsMetadata as "FactoryExtensions" {
    namespace: "FerroDock.Model.Core",
    methods: [
        static fn GetActiveRoot(Option<Rc<dyn IFactory>>) -> Option<Rc<dyn IRootDock>> =>
            |factory: Option<Rc<dyn IFactory>>| ferrodock_model::core::FactoryExtensions::get_active_root(factory.as_deref()),
        static fn GetCurrentDocument(Option<Rc<dyn IFactory>>) -> Option<Rc<dyn ferrodock_model::controls::IDocument>> =>
            |factory: Option<Rc<dyn IFactory>>| ferrodock_model::core::FactoryExtensions::get_current_document(factory.as_deref()),
        static fn CloseFocusedDockable(Option<Rc<dyn IFactory>>) =>
            |factory: Option<Rc<dyn IFactory>>| ferrodock_model::core::FactoryExtensions::close_focused_dockable(factory.as_deref()),
        static fn GetCurrentDockWindow(Option<Rc<dyn IFactory>>) -> Option<Rc<dyn IDockWindow>> =>
            |factory: Option<Rc<dyn IFactory>>| ferrodock_model::core::FactoryExtensions::get_current_dock_window(factory.as_deref()),
        static fn GetCurrentHostWindow(Option<Rc<dyn IFactory>>) -> Option<Rc<dyn ferrodock_model::core::IHostWindow>> =>
            |factory: Option<Rc<dyn IFactory>>| ferrodock_model::core::FactoryExtensions::get_current_host_window(factory.as_deref()),
    ],
});

pub(crate) fn register() {
    use ferroui_base::metadata::{MarkupType, MarkupTyped};
    MarkupType::register(<dyn IFactoryMetadata as MarkupTyped>::MARKUP);
    MarkupType::register(FactoryExtensionsMetadata::MARKUP);
}
