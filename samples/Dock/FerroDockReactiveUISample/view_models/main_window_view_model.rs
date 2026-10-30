// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DockFactory, LayoutFileViewModel};
use crate::models::DemoData;
use crate::services::{ILayoutFileStorage, ILayoutInitialization, ILayoutSerialization};
use ferrodock_model::{
    controls::IRootDock,
    core::{DockSerializationError, GlobalDockTrackingState, IFactory, SerializationResult},
};
use ferroui_reactiveui::{
    reactive_object, ICommand, IReactiveObjectExt, ReactiveCommand, ReactiveObject,
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
/// Original main sample view model, layout command and global focus status.
pub struct MainWindowViewModel {
    base: ReactiveObject,
    factory: Option<Rc<dyn IFactory>>,
    layout: RefCell<Option<Rc<dyn IRootDock>>>,
    global_status: RefCell<String>,
    new_layout: OnceCell<Rc<dyn ICommand>>,
    layout_serialization: Option<Rc<dyn ILayoutSerialization>>,
    layout_initialization: Option<Rc<dyn ILayoutInitialization>>,
    layout_files: RefCell<Option<Rc<LayoutFileViewModel>>>,
}
reactive_object!(MainWindowViewModel, base);
impl AsRef<ReactiveObject> for MainWindowViewModel {
    fn as_ref(&self) -> &ReactiveObject {
        &self.base
    }
}
macro_rules! debug_trace{($($arg:tt)*)=>{#[cfg(debug_assertions)]eprintln!($($arg)*);};}
impl MainWindowViewModel {
    /// Creates the original sample, initializes its layout, and navigates to Home.
    pub fn new() -> Rc<Self> {
        Self::build(DockFactory::new(DemoData::new()), None, None)
    }
    /// Creates the source model with application-composed persistence services.
    pub fn with_layout_services(
        factory: Rc<dyn IFactory>,
        serialization: Rc<dyn ILayoutSerialization>,
        initialization: Rc<dyn ILayoutInitialization>,
    ) -> Rc<Self> {
        Self::build(factory, Some(serialization), Some(initialization))
    }
    fn build(
        factory: Rc<dyn IFactory>,
        serialization: Option<Rc<dyn ILayoutSerialization>>,
        initialization: Option<Rc<dyn ILayoutInitialization>>,
    ) -> Rc<Self> {
        let this = Rc::new(Self {
            base: ReactiveObject::new(),
            factory: Some(factory),
            layout: RefCell::new(None),
            global_status: RefCell::new("Global: (none)".into()),
            new_layout: OnceCell::new(),
            layout_serialization: serialization,
            layout_initialization: initialization,
            layout_files: RefCell::new(None),
        });
        this.debug_factory_events(this.factory.as_ref().expect("source constructed factory"));
        let layout = this
            .factory
            .as_ref()
            .and_then(|factory| factory.create_layout());
        if let Some(layout) = &layout {
            if let Some(factory) = &this.factory {
                factory.init_layout(layout.clone());
            }
            layout.navigate().execute(Some(&"Home".to_owned()));
        }
        this.set_layout(layout);
        let status = if this.layout().is_none() {
            "Global: (none)".into()
        } else {
            Self::format_global_status(
                &this
                    .factory
                    .as_ref()
                    .map(|factory| factory.global_dock_tracking_state())
                    .unwrap_or_else(GlobalDockTrackingState::empty),
            )
        };
        this.set_global_status(status);
        let owner = this.clone();
        let _ = this
            .new_layout
            .set(Rc::new(ReactiveCommand::create(move || {
                owner.reset_layout()
            })));
        this
    }
    /// Gets whether the application provided its layout persistence services.
    pub fn can_persist_layout(&self) -> bool {
        self.layout_serialization.is_some() && self.layout_initialization.is_some()
    }
    /// Composes file actions for this model; native storage is provided by App startup.
    pub fn attach_layout_files(self: &Rc<Self>, storage: Rc<dyn ILayoutFileStorage>) {
        self.raise_and_set_if_changed(
            &self.layout_files,
            Some(LayoutFileViewModel::new(self, storage)),
            "LayoutFiles",
        );
    }
    /// Gets the optional file-action model used by compiled menu/status bindings.
    pub fn layout_files(&self) -> Option<Rc<LayoutFileViewModel>> {
        self.layout_files.borrow().clone()
    }
    /// Saves using the standard stream encoding rather than re-encoding JSON text.
    pub fn save_layout_bytes(&self) -> SerializationResult<Vec<u8>> {
        self.layout_serialization
            .as_ref()
            .ok_or_else(|| {
                DockSerializationError::new(std::io::Error::other(
                    "Layout persistence is not configured.",
                ))
            })?
            .save_bytes(self.layout())
    }
    /// Decodes file bytes before changing the active layout or closing any windows.
    pub fn load_layout_bytes(&self, bytes: Vec<u8>) -> SerializationResult<()> {
        let layout = self
            .layout_serialization
            .as_ref()
            .ok_or_else(|| {
                DockSerializationError::new(std::io::Error::other(
                    "Layout persistence is not configured.",
                ))
            })?
            .load_bytes(bytes)?;
        self.restore_layout(layout)
    }
    /// Serializes the current original layout through the configured application catalog.
    /// Models created by the compatibility constructor need injected services first.
    pub fn save_layout_json(&self) -> SerializationResult<String> {
        let serialization = self.layout_serialization.as_ref().ok_or_else(|| {
            DockSerializationError::new(std::io::Error::other(
                "Layout persistence is not configured.",
            ))
        })?;
        serialization.serialize(self.layout())
    }
    /// Replaces the current layout only after successful JSON decoding, then reconnects
    /// its original factory/locators. A JSON null closes and clears the current layout.
    pub fn load_layout_json(&self, json: &str) -> SerializationResult<()> {
        let serialization = self.layout_serialization.as_ref().ok_or_else(|| {
            DockSerializationError::new(std::io::Error::other(
                "Layout persistence is not configured.",
            ))
        })?;
        let layout = serialization.deserialize(json)?;
        self.restore_layout(layout)
    }
    fn restore_layout(&self, layout: Option<Rc<dyn IRootDock>>) -> SerializationResult<()> {
        let initialization = self.layout_initialization.as_ref().ok_or_else(|| {
            DockSerializationError::new(std::io::Error::other(
                "Layout persistence is not configured.",
            ))
        })?;
        self.close_layout();
        initialization.initialize(layout.clone());
        if layout.is_none() {
            self.set_global_status("Global: (none)".into());
        }
        self.set_layout(layout);
        Ok(())
    }
    /// Gets the current source layout.
    pub fn layout(&self) -> Option<Rc<dyn IRootDock>> {
        self.layout.borrow().clone()
    }
    /// Replaces the source layout and raises ReactiveUI notifications when changed.
    pub fn set_layout(&self, value: Option<Rc<dyn IRootDock>>) {
        self.raise_and_set_if_changed(&self.layout, value, "Layout");
    }
    /// Gets the original global tracking status text.
    pub fn global_status(&self) -> String {
        self.global_status.borrow().clone()
    }
    /// Replaces the status and raises ReactiveUI notifications when changed.
    pub fn set_global_status(&self, value: String) {
        self.raise_and_set_if_changed(&self.global_status, value, "GlobalStatus");
    }
    /// Gets the source ReactiveCommand that resets the layout.
    pub fn new_layout(&self) -> Rc<dyn ICommand> {
        self.new_layout
            .get()
            .expect("source constructor initialized command")
            .clone()
    }
    fn debug_factory_events(self: &Rc<Self>, factory: &Rc<dyn IFactory>) {
        factory.active_dockable_changed(Rc::new(|_args| {
            debug_trace!(
                "[ActiveDockableChanged] Title='{}', Root='{}', Window='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args
                    .root_dock()
                    .map(|value| value.id())
                    .unwrap_or_default(),
                _args.window().map(|value| value.id()).unwrap_or_default()
            );
        }));
        factory.focused_dockable_changed(Rc::new(|_args| {
            debug_trace!(
                "[FocusedDockableChanged] Title='{}', Root='{}', Window='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args
                    .root_dock()
                    .map(|value| value.id())
                    .unwrap_or_default(),
                _args.window().map(|value| value.id()).unwrap_or_default()
            );
        }));
        let owner = self.clone();
        factory.global_dock_tracking_changed(Rc::new(move |_args| {
            owner.set_global_status(Self::format_global_status(&_args.current()));
            debug_trace!(
                "[GlobalDockTrackingChanged] Reason='{:?}', Dockable='{}', Root='{}', Window='{}'",
                _args.reason(),
                _args
                    .current()
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args
                    .current()
                    .root_dock()
                    .map(|value| value.id())
                    .unwrap_or_default(),
                _args
                    .current()
                    .window()
                    .map(|value| value.id())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_added(Rc::new(|_args| {
            debug_trace!(
                "[DockableAdded] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_removed(Rc::new(|_args| {
            debug_trace!(
                "[DockableRemoved] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_closed(Rc::new(|_args| {
            debug_trace!(
                "[DockableClosed] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_moved(Rc::new(|_args| {
            debug_trace!(
                "[DockableMoved] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_docked(Rc::new(|_args| {
            debug_trace!(
                "[DockableDocked] Title='{}', Operation='{:?}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args.operation()
            );
        }));
        factory.dockable_undocked(Rc::new(|_args| {
            debug_trace!(
                "[DockableUndocked] Title='{}', Operation='{:?}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args.operation()
            );
        }));
        factory.dockable_swapped(Rc::new(|_args| {
            debug_trace!(
                "[DockableSwapped] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_pinned(Rc::new(|_args| {
            debug_trace!(
                "[DockablePinned] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_unpinned(Rc::new(|_args| {
            debug_trace!(
                "[DockableUnpinned] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.window_opened(Rc::new(|_args| {
            debug_trace!(
                "[WindowOpened] Title='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.window_closed(Rc::new(|_args| {
            debug_trace!(
                "[WindowClosed] Title='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.window_closing(Rc::new(|_args| {
            debug_trace!(
                "[WindowClosing] Title='{}', Cancel={}",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                if _args.cancel() { "True" } else { "False" }
            );
        }));
        factory.window_added(Rc::new(|_args| {
            debug_trace!(
                "[WindowAdded] Title='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.window_removed(Rc::new(|_args| {
            debug_trace!(
                "[WindowRemoved] Title='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.window_move_drag_begin(Rc::new(|_args| {
            debug_trace!(
                "[WindowMoveDragBegin] Title='{}', Cancel={}, X='{}', Y='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                if _args.cancel() { "True" } else { "False" },
                _args
                    .window()
                    .map(|value| value.x().to_string())
                    .unwrap_or_default(),
                _args
                    .window()
                    .map(|value| value.y().to_string())
                    .unwrap_or_default()
            );
        }));
        factory.window_move_drag(Rc::new(|_args| {
            debug_trace!(
                "[WindowMoveDrag] Title='{}', X='{}', Y='{}",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args
                    .window()
                    .map(|value| value.x().to_string())
                    .unwrap_or_default(),
                _args
                    .window()
                    .map(|value| value.y().to_string())
                    .unwrap_or_default()
            );
        }));
        factory.window_move_drag_end(Rc::new(|_args| {
            debug_trace!(
                "[WindowMoveDragEnd] Title='{}', X='{}', Y='{}",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default(),
                _args
                    .window()
                    .map(|value| value.x().to_string())
                    .unwrap_or_default(),
                _args
                    .window()
                    .map(|value| value.y().to_string())
                    .unwrap_or_default()
            );
        }));
        factory.window_activated(Rc::new(|_args| {
            debug_trace!(
                "[WindowActivated] Title='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_activated(Rc::new(|_args| {
            debug_trace!(
                "[DockableActivated] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.window_deactivated(Rc::new(|_args| {
            debug_trace!(
                "[WindowDeactivated] Title='{}'",
                _args
                    .window()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
        factory.dockable_deactivated(Rc::new(|_args| {
            debug_trace!(
                "[DockableDeactivated] Title='{}'",
                _args
                    .dockable()
                    .map(|value| value.title())
                    .unwrap_or_default()
            );
        }));
    }
    /// Executes the layout close command only when it can run.
    pub fn close_layout(&self) {
        if let Some(layout) = self.layout() {
            if layout.close().can_execute(None) {
                layout.close().execute(None);
            }
        }
    }
    /// Closes the current layout and replaces it with a newly initialized source layout.
    pub fn reset_layout(&self) {
        if let Some(layout) = self.layout() {
            if layout.close().can_execute(None) {
                layout.close().execute(None);
            }
        }
        let layout = self
            .factory
            .as_ref()
            .and_then(|factory| factory.create_layout());
        if let Some(layout) = layout {
            if let Some(factory) = &self.factory {
                factory.init_layout(layout.clone());
            }
            self.set_layout(Some(layout));
        }
    }
    fn format_global_status(state: &GlobalDockTrackingState) -> String {
        let dockable_title = state
            .dockable()
            .map(|dockable| dockable.title())
            .unwrap_or_else(|| "(none)".into());
        let root_id = state
            .root_dock()
            .map(|root| root.id())
            .unwrap_or_else(|| "(none)".into());
        let window_title = state
            .window()
            .map(|window| window.title())
            .unwrap_or_else(|| "(main)".into());
        let host = state
            .host_window()
            .map(|host| host.runtime_type_name())
            .unwrap_or("(main)");
        format!(
            "Dockable: {dockable_title} | Root: {root_id} | Window: {window_title} | Host: {host}"
        )
    }
}

impl PartialEq for MainWindowViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
