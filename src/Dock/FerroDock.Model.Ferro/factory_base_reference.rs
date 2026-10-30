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
use ferrodock_model::core::{
    DockWindowOptions, DocumentLayoutMode, DocumentTabLayout, GlobalDockTrackingState,
};
use ferrodock_model::{
    collections::ListHandle,
    controls::{
        IDockDock, IDocument, IDocumentDock, IGridDock, IGridDockSplitter, IProportionalDock,
        IProportionalDockSplitter, IRootDock, ISplitViewDock, IStackDock, ITool, IToolDock,
        IUniformGridDock, IWrapDock,
    },
    core::{IDock, IDockControl, IDockWindow, IDockable, IFactory, IHostWindow},
    FactoryBaseImpl,
};
use ferroui_base::metadata::MarkupDelegate;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Native view of the source abstract FactoryBase, retaining the enclosing engine.
#[derive(Clone)]
pub struct FactoryBaseReference {
    model: Rc<dyn IFactory>,
}
impl FactoryBaseReference {
    /// Views a real source factory base or composed subclass without copying state.
    pub fn new<T: FactoryBaseImpl + 'static>(model: Rc<T>) -> Self {
        Self { model }
    }
    /// Gets the original enclosing factory and its complete source contract.
    pub fn model(&self) -> Rc<dyn IFactory> {
        self.model.clone()
    }
}
impl std::ops::Deref for FactoryBaseReference {
    type Target = dyn IFactory;
    fn deref(&self) -> &Self::Target {
        self.model.as_ref()
    }
}
impl PartialEq for FactoryBaseReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
ferroui_base::ferro_markup_type!(class FactoryBaseReference as "FactoryBase" {
 namespace:"FerroDock.Model",this:FactoryBaseReference,handles:[FactoryBaseReference,Option<FactoryBaseReference>],interfaces:[Rc<dyn IFactory>],
 properties:[
GlobalDockTrackingState:GlobalDockTrackingState{get:|this:&FactoryBaseReference|this.global_dock_tracking_state()},
CurrentDockable:Option<Rc<dyn IDockable>>{get:|this:&FactoryBaseReference|this.current_dockable()},
CurrentRootDock:Option<Rc<dyn IRootDock>>{get:|this:&FactoryBaseReference|this.current_root_dock()},
CurrentDockWindow:Option<Rc<dyn IDockWindow>>{get:|this:&FactoryBaseReference|this.current_dock_window()},
CurrentHostWindow:Option<Rc<dyn IHostWindow>>{get:|this:&FactoryBaseReference|this.current_host_window()},
DefaultContextLocator:Option<ContextFactoryReference>{get:|this:&FactoryBaseReference|this.default_context_locator().map(ContextFactoryReference::new),set:|this:&FactoryBaseReference,value:Option<ContextFactoryReference>|this.set_default_context_locator(value.map(|value|value.model()))},
DefaultHostWindowLocator:Option<HostWindowFactoryReference>{get:|this:&FactoryBaseReference|this.default_host_window_locator().map(HostWindowFactoryReference::new),set:|this:&FactoryBaseReference,value:Option<HostWindowFactoryReference>|this.set_default_host_window_locator(value.map(|value|value.model()))},
ContextLocator:Option<ContextLocatorMapReference>{get:|this:&FactoryBaseReference|this.context_locator().map(ContextLocatorMapReference::new),set:|this:&FactoryBaseReference,value:Option<ContextLocatorMapReference>|this.set_context_locator(value.map(|value|value.model()))},
HostWindowLocator:Option<HostWindowLocatorMapReference>{get:|this:&FactoryBaseReference|this.host_window_locator().map(HostWindowLocatorMapReference::new),set:|this:&FactoryBaseReference,value:Option<HostWindowLocatorMapReference>|this.set_host_window_locator(value.map(|value|value.model()))},
DockableLocator:Option<DockableLocatorMapReference>{get:|this:&FactoryBaseReference|this.dockable_locator().map(DockableLocatorMapReference::new),set:|this:&FactoryBaseReference,value:Option<DockableLocatorMapReference>|this.set_dockable_locator(value.map(|value|value.model()))},

 HideToolsOnClose:bool{get:|this:&FactoryBaseReference|this.hide_tools_on_close(),set:|this:&FactoryBaseReference,value|this.set_hide_tools_on_close(value)},
 HideDocumentsOnClose:bool{get:|this:&FactoryBaseReference|this.hide_documents_on_close(),set:|this:&FactoryBaseReference,value|this.set_hide_documents_on_close(value)},
 VisibleDockableControls:DockableControlMapReference{get:|this:&FactoryBaseReference|DockableControlMapReference::new(this.visible_dockable_controls())},
 VisibleRootControls:DockableObjectMapReference{get:|this:&FactoryBaseReference|DockableObjectMapReference::new(this.visible_root_controls())},
 PinnedDockableControls:DockableControlMapReference{get:|this:&FactoryBaseReference|DockableControlMapReference::new(this.pinned_dockable_controls())},
 PinnedRootControls:DockableObjectMapReference{get:|this:&FactoryBaseReference|DockableObjectMapReference::new(this.pinned_root_controls())},
 TabDockableControls:DockableControlMapReference{get:|this:&FactoryBaseReference|DockableControlMapReference::new(this.tab_dockable_controls())},
 TabRootControls:DockableObjectMapReference{get:|this:&FactoryBaseReference|DockableObjectMapReference::new(this.tab_root_controls())},
 ToolControls:DockableObjectMapReference{get:|this:&FactoryBaseReference|DockableObjectMapReference::new(this.tool_controls())},
 DocumentControls:DockableObjectMapReference{get:|this:&FactoryBaseReference|DockableObjectMapReference::new(this.document_controls())},
 DockControls:ListHandle<Rc<dyn IDockControl>>{get:|this:&FactoryBaseReference|this.dock_controls()},
 HostWindows:ListHandle<Rc<dyn IHostWindow>>{get:|this:&FactoryBaseReference|this.host_windows()},
 ],events:[
ActiveDockableChanged(Option<ferroui_base::BoxedValue>,ActiveDockableChangedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::active_dockable_changed(&this.model(),handler)),
FocusedDockableChanged(Option<ferroui_base::BoxedValue>,FocusedDockableChangedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::focused_dockable_changed(&this.model(),handler)),
GlobalDockTrackingChanged(Option<ferroui_base::BoxedValue>,GlobalDockTrackingChangedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::global_dock_tracking_changed(&this.model(),handler)),
DockableInit(Option<ferroui_base::BoxedValue>,DockableInitEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_init(&this.model(),handler)),
DockableAdded(Option<ferroui_base::BoxedValue>,DockableAddedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_added(&this.model(),handler)),
DockableRemoved(Option<ferroui_base::BoxedValue>,DockableRemovedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_removed(&this.model(),handler)),
DockableClosing(Option<ferroui_base::BoxedValue>,DockableClosingEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_closing(&this.model(),handler)),
DockableClosed(Option<ferroui_base::BoxedValue>,DockableClosedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_closed(&this.model(),handler)),
DockableMoved(Option<ferroui_base::BoxedValue>,DockableMovedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_moved(&this.model(),handler)),
DockableDocked(Option<ferroui_base::BoxedValue>,DockableDockedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_docked(&this.model(),handler)),
DockableUndocked(Option<ferroui_base::BoxedValue>,DockableUndockedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_undocked(&this.model(),handler)),
DockableSwapped(Option<ferroui_base::BoxedValue>,DockableSwappedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_swapped(&this.model(),handler)),
DockablePinned(Option<ferroui_base::BoxedValue>,DockablePinnedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_pinned(&this.model(),handler)),
DockableUnpinned(Option<ferroui_base::BoxedValue>,DockableUnpinnedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_unpinned(&this.model(),handler)),
DockableHidden(Option<ferroui_base::BoxedValue>,DockableHiddenEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_hidden(&this.model(),handler)),
DockableRestored(Option<ferroui_base::BoxedValue>,DockableRestoredEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_restored(&this.model(),handler)),
WindowOpened(Option<ferroui_base::BoxedValue>,WindowOpenedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_opened(&this.model(),handler)),
WindowClosing(Option<ferroui_base::BoxedValue>,WindowClosingEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_closing(&this.model(),handler)),
WindowClosed(Option<ferroui_base::BoxedValue>,WindowClosedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_closed(&this.model(),handler)),
WindowAdded(Option<ferroui_base::BoxedValue>,WindowAddedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_added(&this.model(),handler)),
WindowRemoved(Option<ferroui_base::BoxedValue>,WindowRemovedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_removed(&this.model(),handler)),
WindowMoveDragBegin(Option<ferroui_base::BoxedValue>,WindowMoveDragBeginEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_move_drag_begin(&this.model(),handler)),
WindowMoveDrag(Option<ferroui_base::BoxedValue>,WindowMoveDragEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_move_drag(&this.model(),handler)),
WindowMoveDragEnd(Option<ferroui_base::BoxedValue>,WindowMoveDragEndEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_move_drag_end(&this.model(),handler)),
WindowActivated(Option<ferroui_base::BoxedValue>,WindowActivatedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_activated(&this.model(),handler)),
DockableActivated(Option<ferroui_base::BoxedValue>,DockableActivatedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_activated(&this.model(),handler)),
WindowDeactivated(Option<ferroui_base::BoxedValue>,WindowDeactivatedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::window_deactivated(&this.model(),handler)),
DockableDeactivated(Option<ferroui_base::BoxedValue>,DockableDeactivatedEventArgs) => (|this:&FactoryBaseReference,handler:MarkupDelegate|crate::factory_events::dockable_deactivated(&this.model(),handler)),
],
methods:[
fn GetContext(String)->Option<ferroui_base::BoxedValue> => (|this:&FactoryBaseReference,id|this.get_context(id).as_ref().map(ferrodock_runtime::from_source_value)),
 fn GetHostWindow(String)->Option<Rc<dyn IHostWindow>> => (|this:&FactoryBaseReference,id|this.get_host_window(id)),

fn OnActiveDockableChanged(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_active_dockable_changed(dockable)),
fn OnFocusedDockableChanged(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_focused_dockable_changed(dockable)),
fn OnDockableInit(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_init(dockable)),
fn OnDockableAdded(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_added(dockable)),
fn OnDockableRemoved(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_removed(dockable)),
fn OnDockableClosing(Option<Rc<dyn IDockable>>) -> bool => (|this:&FactoryBaseReference,dockable|this.on_dockable_closing(dockable)),
fn OnDockableClosed(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_closed(dockable)),
fn OnDockableMoved(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_moved(dockable)),
fn OnDockableDocked(Option<Rc<dyn IDockable>>,DockOperation) => (|this:&FactoryBaseReference,dockable,operation|this.on_dockable_docked(dockable,operation)),
fn OnDockableUndocked(Option<Rc<dyn IDockable>>,DockOperation) => (|this:&FactoryBaseReference,dockable,operation|this.on_dockable_undocked(dockable,operation)),
fn OnDockableSwapped(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_swapped(dockable)),
fn OnDockablePinned(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_pinned(dockable)),
fn OnDockableUnpinned(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_unpinned(dockable)),
fn OnDockableHidden(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_hidden(dockable)),
fn OnDockableRestored(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_restored(dockable)),
fn OnWindowOpened(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_opened(window)),
fn OnWindowClosing(Option<Rc<dyn IDockWindow>>) -> bool => (|this:&FactoryBaseReference,window|this.on_window_closing(window)),
fn OnWindowClosed(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_closed(window)),
fn OnWindowAdded(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_added(window)),
fn OnWindowRemoved(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_removed(window)),
fn OnWindowMoveDragBegin(Option<Rc<dyn IDockWindow>>) -> bool => (|this:&FactoryBaseReference,window|this.on_window_move_drag_begin(window)),
fn OnWindowMoveDrag(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_move_drag(window)),
fn OnWindowMoveDragEnd(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_move_drag_end(window)),
fn OnWindowActivated(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_activated(window)),
fn OnWindowActivated(Option<Rc<dyn IDockWindow>>,Option<Rc<dyn IRootDock>>) => (|this:&FactoryBaseReference,window,root_dock|this.on_window_activated_with_root(window,root_dock)),
fn OnDockableActivated(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_activated(dockable)),
fn OnWindowDeactivated(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.on_window_deactivated(window)),
fn OnDockableDeactivated(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.on_dockable_deactivated(dockable)),

fn GetContainerFromItem(Option<ferroui_base::BoxedValue>)->Option<Rc<dyn IDockable>> => (|this:&FactoryBaseReference,item|this.get_container_from_item(crate::factory_metadata::parameter(item))),
fn InitDockWindow(Option<Rc<dyn IDockWindow>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IHostWindow>>) => (|this:&FactoryBaseReference,window,owner,host|this.init_dock_window_with_host(window,owner,host)),
fn InitActiveDockable(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDock>>) => (|this:&FactoryBaseReference,dockable,owner|this.init_active_dockable(dockable,owner)),
fn ActivateWindow(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dockable|this.activate_window(dockable)),
fn InsertDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,i32) => (|this:&FactoryBaseReference,dock,dockable,index|this.insert_dockable(dock,dockable,index)),
fn RemoveDockable(Option<Rc<dyn IDockable>>,bool) => (|this:&FactoryBaseReference,dockable,collapse|this.remove_dockable(dockable,collapse)),
fn MoveDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dock,source,target|this.move_dockable(dock,source,target)),
fn MoveDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,source_dock,target_dock,source,target|this.move_dockable_between_docks(source_dock,target_dock,source,target)),
fn SwapDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,dock,source,target|this.swap_dockable(dock,source,target)),
fn SwapDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,source_dock,target_dock,source,target|this.swap_dockable_between_docks(source_dock,target_dock,source,target)),
fn IsDockablePinned(Option<Rc<dyn IDockable>>)->bool => (|this:&FactoryBaseReference,dockable|this.is_dockable_pinned(dockable,None)),
fn IsDockablePinned(Option<Rc<dyn IDockable>>,Option<Rc<dyn IRootDock>>)->bool => (|this:&FactoryBaseReference,dockable,root|this.is_dockable_pinned(dockable,root)),
fn MovePinnedDockable(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,source,target|this.move_pinned_dockable(source,target)),
fn AddWindow(Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,root,window|this.add_window(root,window)),
fn InsertWindow(Option<Rc<dyn IRootDock>>,Option<Rc<dyn IDockWindow>>,i32) => (|this:&FactoryBaseReference,root,window,index|this.insert_window(root,window,index)),
fn RemoveWindow(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,window|this.remove_window(window)),
fn CollapseDock(Rc<dyn IDock>) => (|this:&FactoryBaseReference,dock|this.collapse_dock(dock)),
fn CreateSplitLayout(Rc<dyn IDock>,Rc<dyn IDockable>,DockOperation)->Rc<dyn IDock> => (|this:&FactoryBaseReference,dock,dockable,operation|this.create_split_layout(dock,dockable,operation)),
fn SplitToDock(Rc<dyn IDock>,Rc<dyn IDockable>,DockOperation) => (|this:&FactoryBaseReference,dock,dockable,operation|this.split_to_dock(dock,dockable,operation)),
fn CreateWindowFrom(Option<Rc<dyn IDockable>>)->Option<Rc<dyn IDockWindow>> => (|this:&FactoryBaseReference,dockable|this.create_window_from(dockable)),
fn CreateWindowFrom(Option<Rc<dyn IDockable>>,Option<Rc<DockWindowOptions>>)->Option<Rc<dyn IDockWindow>> => (|this:&FactoryBaseReference,dockable,options|this.create_window_from_with_options(dockable,options)),
fn SplitToWindow(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,f64,f64,f64,f64) => (|this:&FactoryBaseReference,dock,dockable,x,y,width,height|this.split_to_window(dock,dockable,x,y,width,height)),
fn SplitToWindow(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>,f64,f64,f64,f64,Option<Rc<DockWindowOptions>>) => (|this:&FactoryBaseReference,dock,dockable,x,y,width,height,options|this.split_to_window_with_options(dock,dockable,x,y,width,height,options)),
        fn PinDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.pin_dockable(value)),
        fn PinDockable(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.pin_dockable_parameter(crate::factory_metadata::parameter(value))),
        fn PreviewPinnedDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.preview_pinned_dockable(value)),
        fn PreviewPinnedDockable(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.preview_pinned_dockable_parameter(crate::factory_metadata::parameter(value))),
        fn TogglePreviewPinnedDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.toggle_preview_pinned_dockable(value)),
        fn TogglePreviewPinnedDockable(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.toggle_preview_pinned_dockable_parameter(crate::factory_metadata::parameter(value))),
        fn FloatDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.float_dockable(value)),
        fn FloatDockable(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.float_dockable_parameter(crate::factory_metadata::parameter(value))),
        fn FloatAllDockables(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.float_all_dockables(value)),
        fn FloatAllDockables(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.float_all_dockables_parameter(crate::factory_metadata::parameter(value))),
        fn DockAsDocument(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.dock_as_document(value)),
        fn DockAsDocument(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.dock_as_document_parameter(crate::factory_metadata::parameter(value))),
        fn CloseDockable(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.close_dockable_parameter(crate::factory_metadata::parameter(value))),
        fn CloseOtherDockables(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.close_other_dockables(value)),
        fn CloseOtherDockables(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.close_other_dockables_parameter(crate::factory_metadata::parameter(value))),
        fn CloseAllDockables(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.close_all_dockables(value)),
        fn CloseAllDockables(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.close_all_dockables_parameter(crate::factory_metadata::parameter(value))),
        fn CloseLeftDockables(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.close_left_dockables(value)),
        fn CloseLeftDockables(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.close_left_dockables_parameter(crate::factory_metadata::parameter(value))),
        fn CloseRightDockables(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.close_right_dockables(value)),
        fn CloseRightDockables(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.close_right_dockables_parameter(crate::factory_metadata::parameter(value))),
        fn SetDocumentDockTabsLayoutLeft(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.set_document_dock_tabs_layout_left(value)),
        fn SetDocumentDockTabsLayoutLeft(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.set_document_dock_tabs_layout_left_parameter(crate::factory_metadata::parameter(value))),
        fn SetDocumentDockTabsLayoutTop(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.set_document_dock_tabs_layout_top(value)),
        fn SetDocumentDockTabsLayoutTop(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.set_document_dock_tabs_layout_top_parameter(crate::factory_metadata::parameter(value))),
        fn SetDocumentDockTabsLayoutRight(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.set_document_dock_tabs_layout_right(value)),
        fn SetDocumentDockTabsLayoutRight(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.set_document_dock_tabs_layout_right_parameter(crate::factory_metadata::parameter(value))),
        fn SetDocumentDockLayoutModeTabbed(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.set_document_dock_layout_mode_tabbed(value)),
        fn SetDocumentDockLayoutModeTabbed(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.set_document_dock_layout_mode_tabbed_parameter(crate::factory_metadata::parameter(value))),
        fn SetDocumentDockLayoutModeMdi(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.set_document_dock_layout_mode_mdi(value)),
        fn SetDocumentDockLayoutModeMdi(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.set_document_dock_layout_mode_mdi_parameter(crate::factory_metadata::parameter(value))),
        fn NewHorizontalDocumentDock(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.new_horizontal_document_dock(value)),
        fn NewHorizontalDocumentDock(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.new_horizontal_document_dock_parameter(crate::factory_metadata::parameter(value))),
        fn NewVerticalDocumentDock(Rc<dyn IDockable>) => (|this:&FactoryBaseReference,value|this.new_vertical_document_dock(value)),
        fn NewVerticalDocumentDock(Option<ferroui_base::BoxedValue>) => (|this:&FactoryBaseReference,value|this.new_vertical_document_dock_parameter(crate::factory_metadata::parameter(value))),
        fn UnpinDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.unpin_dockable(value)),
        fn HidePreviewingDockables(Option<Rc<dyn IRootDock>>) => (|this:&FactoryBaseReference,value|this.hide_previewing_dockables(value)),
        fn HideDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.hide_dockable(value)),
        fn HideDockable(String) => (|this:&FactoryBaseReference,value|this.hide_dockable_by_id(value)),
        fn RestoreDockable(Option<Rc<dyn IDockable>>) => (|this:&FactoryBaseReference,value|this.restore_dockable(value)),
        fn RestoreDockable(String)->Option<Rc<dyn IDockable>> => (|this:&FactoryBaseReference,value|this.restore_dockable_by_id(value)),
        fn CloseWindow(Option<Rc<dyn IDockWindow>>) => (|this:&FactoryBaseReference,value|this.close_window(value)),
        fn FloatDockable(Option<Rc<dyn IDockable>>,Option<Rc<DockWindowOptions>>) => (|this:&FactoryBaseReference,value,options|this.float_dockable_with_options(value,options)),
        fn FloatAllDockables(Option<Rc<dyn IDockable>>,Option<Rc<DockWindowOptions>>) => (|this:&FactoryBaseReference,value,options|this.float_all_dockables_with_options(value,options)),
        fn SetDocumentDockTabsLayout(Rc<dyn IDockable>,DocumentTabLayout) => (|this:&FactoryBaseReference,value,layout|this.set_document_dock_tabs_layout(value,layout)),
        fn SetDocumentDockLayoutMode(Rc<dyn IDockable>,DocumentLayoutMode) => (|this:&FactoryBaseReference,value,layout|this.set_document_dock_layout_mode(value,layout)),
fn FindRoot(Option<Rc<dyn IDockable>>)->Option<Rc<dyn IRootDock>> => (|this:&FactoryBaseReference,dockable|this.find_root(dockable,None)),
fn FindRoot(Option<Rc<dyn IDockable>>,Option<RootPredicateReference>)->Option<Rc<dyn IRootDock>> => (|this:&FactoryBaseReference,dockable,predicate:Option<RootPredicateReference>|this.find_root(dockable,predicate.map(|value|value.model()))),
fn FindDockable(Option<Rc<dyn IDock>>,Option<DockablePredicateReference>)->Option<Rc<dyn IDockable>> => (|this:&FactoryBaseReference,dock,predicate:Option<DockablePredicateReference>|this.find_dockable(dock,predicate.map(|value|value.model()))),
fn Find(Option<DockablePredicateReference>)->DockableQueryReference => (|this:&FactoryBaseReference,predicate:Option<DockablePredicateReference>|DockableQueryReference::new(this.find(predicate.map(|value|value.model())))),
fn Find(Option<Rc<dyn IDock>>,Option<DockablePredicateReference>)->DockableQueryReference => (|this:&FactoryBaseReference,dock,predicate:Option<DockablePredicateReference>|DockableQueryReference::new(this.find_in_dock(dock,predicate.map(|value|value.model())))),
 fn CreateRootDock()->Rc<dyn IRootDock> =>(|this:&FactoryBaseReference|this.create_root_dock()),
 fn CreateProportionalDock()->Rc<dyn IProportionalDock> =>(|this:&FactoryBaseReference|this.create_proportional_dock()),
 fn CreateDockDock()->Rc<dyn IDockDock> =>(|this:&FactoryBaseReference|this.create_dock_dock()),
 fn CreateStackDock()->Rc<dyn IStackDock> =>(|this:&FactoryBaseReference|this.create_stack_dock()),
 fn CreateGridDock()->Rc<dyn IGridDock> =>(|this:&FactoryBaseReference|this.create_grid_dock()),
 fn CreateWrapDock()->Rc<dyn IWrapDock> =>(|this:&FactoryBaseReference|this.create_wrap_dock()),
 fn CreateUniformGridDock()->Rc<dyn IUniformGridDock> =>(|this:&FactoryBaseReference|this.create_uniform_grid_dock()),
 fn CreateProportionalDockSplitter()->Rc<dyn IProportionalDockSplitter> =>(|this:&FactoryBaseReference|this.create_proportional_dock_splitter()),
 fn CreateGridDockSplitter()->Rc<dyn IGridDockSplitter> =>(|this:&FactoryBaseReference|this.create_grid_dock_splitter()),
 fn CreateToolDock()->Rc<dyn IToolDock> =>(|this:&FactoryBaseReference|this.create_tool_dock()),
 fn CreateDocumentDock()->Rc<dyn IDocumentDock> =>(|this:&FactoryBaseReference|this.create_document_dock()),
 fn CreateSplitViewDock()->Rc<dyn ISplitViewDock> =>(|this:&FactoryBaseReference|this.create_split_view_dock()),
 fn CreateDockWindow()->Rc<dyn IDockWindow> =>(|this:&FactoryBaseReference|this.create_dock_window()),
 fn CreateDocument()->Rc<dyn IDocument> =>(|this:&FactoryBaseReference|this.create_document()),
 fn CreateTool()->Rc<dyn ITool> =>(|this:&FactoryBaseReference|this.create_tool()),
 fn CreateLayout()->Option<Rc<dyn IRootDock>> =>(|this:&FactoryBaseReference|this.create_layout()),
 fn InitLayout(Rc<dyn IDockable>)=>(|this:&FactoryBaseReference,layout|this.init_layout(layout)),
 fn InitDockable(Option<Rc<dyn IDockable>>,Option<Rc<dyn IDockable>>)=>(|this:&FactoryBaseReference,dockable,owner|this.init_dockable(dockable,owner)),
 fn InitDockWindow(Option<Rc<dyn IDockWindow>>,Option<Rc<dyn IDockable>>)=>(|this:&FactoryBaseReference,window,owner|this.init_dock_window(window,owner)),
 fn AddDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>)=>(|this:&FactoryBaseReference,dock,value|this.add_dockable(dock,value)),
 fn SetActiveDockable(Option<Rc<dyn IDockable>>)=>(|this:&FactoryBaseReference,value|this.set_active_dockable(value)),
 fn SetFocusedDockable(Option<Rc<dyn IDock>>,Option<Rc<dyn IDockable>>)=>(|this:&FactoryBaseReference,dock,value|this.set_focused_dockable(dock,value)),
 fn CloseDockable(Option<Rc<dyn IDockable>>)=>(|this:&FactoryBaseReference,value|this.close_dockable(value)),
 ],
});
pub(crate) fn register() {
    MarkupType::register(FactoryBaseReference::MARKUP);
    ValueTypes::register_nullable::<FactoryBaseReference>();
    ValueTypes::register_nullable::<Rc<dyn IFactory>>();
    ValueTypes::register_reference_identity::<Rc<dyn IFactory>>(|v| Rc::as_ptr(v) as *const ());
    ValueTypes::register_reference_identity::<FactoryBaseReference>(|v| {
        Rc::as_ptr(&v.model) as *const ()
    });
    ValueTypes::register_cast::<FactoryBaseReference, Rc<dyn IFactory>>(|v| v.model());
    ValueTypes::register_boxed_cast::<Rc<dyn IFactory>, FactoryBaseReference>(|v| {
        let model = v.downcast_ref::<Rc<dyn IFactory>>()?.clone();
        model.clone().as_factory_base()?;
        Some(FactoryBaseReference { model })
    });
}
