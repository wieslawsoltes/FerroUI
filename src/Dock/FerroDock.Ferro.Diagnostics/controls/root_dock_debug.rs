// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::view_models::{EventLogView, RootDockDebugViewModel};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{ICommand, InputElementImpl},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::{
    primitives::TemplatedControlImpl, ContentControlImpl, ControlImpl, UserControl,
};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
    events::Event,
};
use std::rc::Rc;
/// Original complete layout and event inspector, with passive compiled markup.
#[repr(C)]
pub struct RootDockDebug {
    base: UserControl,
    presentation: Rc<RootDockDebugViewModel>,
}
ferro_class!(RootDockDebug:UserControl);
ferro_class_info!(RootDockDebug {
    new: RootDockDebug::new
});
ferro_impl_classes!(RootDockDebug:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl FerroObjectImpl for RootDockDebug {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.initialize_component();
    }
}
ferro_properties! {impl RootDockDebug {
 /// Identifies the original shared log collection.
 pub fn events_property()->DirectProperty<RootDockDebug,ObservableCollection<String>>{FerroProperty::register_direct::<RootDockDebug,_>("Events",RootDockDebug::events,None,ObservableCollection::new())}
 /// Identifies the original editable nullable event filter.
 pub fn filter_property()->DirectProperty<RootDockDebug,Option<String>>{FerroProperty::register_direct::<RootDockDebug,_>("Filter",RootDockDebug::filter,Some(RootDockDebug::set_filter),None)}
 /// Identifies the original filtered enumerable projection.
 pub fn filtered_events_property()->DirectProperty<RootDockDebug,EventLogView>{FerroProperty::register_direct::<RootDockDebug,_>("FilteredEvents",RootDockDebug::filtered_events,None,EventLogView::empty())}
 /// Identifies the command replacing the source clear-event handler.
 pub fn clear_events_command_property()->DirectProperty<RootDockDebug,Option<Rc<dyn ICommand>>>{FerroProperty::register_direct::<RootDockDebug,_>("ClearEventsCommand",RootDockDebug::clear_events_command,None,None)}
}}
impl RootDockDebug {
    /// Creates original field state using the library composition root.
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            presentation: crate::infrastructure::create_presentation(),
        }
    }
    /// Creates the source inspector and initializes its compiled component.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the original mutable shared event log.
    pub fn events(&self) -> ObservableCollection<String> {
        self.presentation.events()
    }
    /// Gets the original nullable filter.
    pub fn filter(&self) -> Option<String> {
        self.presentation.filter()
    }
    /// Sets the original filter through the presentation state.
    pub fn set_filter(&self, value: Option<String>) {
        self.presentation.set_filter(value);
    }
    /// Gets the original lazy event projection.
    pub fn filtered_events(&self) -> EventLogView {
        self.presentation.filtered_events()
    }
    /// Gets the native adapter for the source clear-event command.
    pub fn clear_events_command(&self) -> Option<Rc<dyn ICommand>> {
        Some(
            ferroui_reactiveui_controls::ReactiveCommandAdapter::command_of(
                self.presentation.clear_events_command(),
            ),
        )
    }
    pub(crate) fn presentation(&self) -> Rc<RootDockDebugViewModel> {
        self.presentation.clone()
    }
    fn initialize_component(&self) {
        crate::register_types();
        crate::compiled_root_dock_debug::populate(None, &self.to_ref())
            .unwrap_or_else(|error| panic!("{error}"));
    }
}
impl INotifyPropertyChanged for RootDockDebug {
    fn property_changed(&self) -> &Event<str> {
        self.presentation.property_changed()
    }
}
