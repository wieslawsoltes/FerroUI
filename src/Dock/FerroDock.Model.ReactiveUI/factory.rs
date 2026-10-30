// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::DockWindow;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::{FactoryBase, FactoryBaseImpl};
use ferroui_system::collections::generic::Dictionary;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type DockableListCreator = Box<dyn Fn() -> ListHandle<Option<Rc<dyn IDockable>>>>;

/// The concrete ReactiveUI factory, composing the full source FactoryBase engine.
/// Presentation registries retain shared dictionary/list identity. Factory and
/// model callbacks use the enclosing runtime when this component is subclassed.
pub struct Factory {
    base: FactoryBase,
    runtime: Weak<dyn IFactory>,
    create_dockable_list: DockableListCreator,
    visible_dockable_controls: DockableControlMap,
    visible_root_controls: DockableObjectMap,
    pinned_dockable_controls: DockableControlMap,
    pinned_root_controls: DockableObjectMap,
    tab_dockable_controls: DockableControlMap,
    tab_root_controls: DockableObjectMap,
    tool_controls: DockableObjectMap,
    document_controls: DockableObjectMap,
    dock_controls: ListHandle<Rc<dyn IDockControl>>,
    host_windows: ListHandle<Rc<dyn IHostWindow>>,
}

impl Factory {
    /// Creates a shared concrete factory with source defaults and fresh registries.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self::construct(runtime.clone()))
    }

    /// Constructs the component for a composed subclass implementing FactoryImpl and IFactory.
    /// The typed weak thunk preserves a subclass's generic CreateList override for pinned lists.
    pub fn construct<T: crate::FactoryImpl + 'static>(runtime: Weak<T>) -> Self {
        let creator = runtime.clone();
        Self {
            base: FactoryBase::construct(runtime.clone()),
            runtime,
            create_dockable_list: Box::new(move || {
                creator
                    .upgrade()
                    .expect("the enclosing factory is alive")
                    .create_list(Vec::new())
            }),
            visible_dockable_controls: Rc::new(RefCell::new(Dictionary::new())),
            pinned_dockable_controls: Rc::new(RefCell::new(Dictionary::new())),
            tab_dockable_controls: Rc::new(RefCell::new(Dictionary::new())),
            visible_root_controls: Rc::new(RefCell::new(Dictionary::new())),
            pinned_root_controls: Rc::new(RefCell::new(Dictionary::new())),
            tab_root_controls: Rc::new(RefCell::new(Dictionary::new())),
            tool_controls: Rc::new(RefCell::new(Dictionary::new())),
            document_controls: Rc::new(RefCell::new(Dictionary::new())),
            dock_controls: ListHandle::new(),
            host_windows: ListHandle::new(),
        }
    }

    /// Returns the shared source engine component, including protected base-method bodies.
    pub fn base(&self) -> &FactoryBase {
        &self.base
    }

    /// Creates the source ObservableCollection-backed list without initializing its elements.
    pub fn create_list<T: Clone + PartialEq + 'static>(&self, items: Vec<T>) -> ListHandle<T> {
        ListHandle::from_items(items)
    }

    /// Creates a root with four distinct pinned lists supplied by the runtime's virtual CreateList.
    pub fn create_root_dock(&self) -> Rc<dyn ferrodock_model::controls::IRootDock> {
        let root = RootDock::new();
        root.set_left_pinned_dockables(Some((self.create_dockable_list)()));
        root.set_right_pinned_dockables(Some((self.create_dockable_list)()));
        root.set_top_pinned_dockables(Some((self.create_dockable_list)()));
        root.set_bottom_pinned_dockables(Some((self.create_dockable_list)()));
        root
    }

    /// Creates the default layout through the enclosing virtual CreateRootDock override.
    pub fn create_layout(&self) -> Option<Rc<dyn ferrodock_model::controls::IRootDock>> {
        Some(
            self.runtime
                .upgrade()
                .expect("the enclosing factory is alive")
                .create_root_dock(),
        )
    }
}

macro_rules! registry {
    ($name:ident, $type:ty) => {
        impl Factory {
            #[doc = concat!("Gets the shared source `", stringify!($name), "` registry.")]
            pub fn $name(&self) -> $type {
                self.$name.clone()
            }
        }
    };
}
registry!(visible_dockable_controls, DockableControlMap);
registry!(visible_root_controls, DockableObjectMap);
registry!(pinned_dockable_controls, DockableControlMap);
registry!(pinned_root_controls, DockableObjectMap);
registry!(tab_dockable_controls, DockableControlMap);
registry!(tab_root_controls, DockableObjectMap);
registry!(tool_controls, DockableObjectMap);
registry!(document_controls, DockableObjectMap);
registry!(dock_controls, ListHandle<Rc<dyn IDockControl>>);
registry!(host_windows, ListHandle<Rc<dyn IHostWindow>>);

macro_rules! creation {
    ($method:ident, $contract:ty, $model:ty) => {
        impl Factory {
            #[doc = concat!("Creates a fresh reactive `", stringify!($model), "` with source defaults.")]
            pub fn $method(&self) -> Rc<$contract> { <$model>::new() }
        }
    };
}
creation!(
    create_proportional_dock,
    dyn ferrodock_model::controls::IProportionalDock,
    ProportionalDock
);
creation!(
    create_dock_dock,
    dyn ferrodock_model::controls::IDockDock,
    DockDock
);
creation!(
    create_stack_dock,
    dyn ferrodock_model::controls::IStackDock,
    StackDock
);
creation!(
    create_grid_dock,
    dyn ferrodock_model::controls::IGridDock,
    GridDock
);
creation!(
    create_wrap_dock,
    dyn ferrodock_model::controls::IWrapDock,
    WrapDock
);
creation!(
    create_uniform_grid_dock,
    dyn ferrodock_model::controls::IUniformGridDock,
    UniformGridDock
);
creation!(
    create_proportional_dock_splitter,
    dyn ferrodock_model::controls::IProportionalDockSplitter,
    ProportionalDockSplitter
);
creation!(
    create_grid_dock_splitter,
    dyn ferrodock_model::controls::IGridDockSplitter,
    GridDockSplitter
);
creation!(
    create_tool_dock,
    dyn ferrodock_model::controls::IToolDock,
    ToolDock
);
creation!(
    create_document_dock,
    dyn ferrodock_model::controls::IDocumentDock,
    DocumentDock
);
creation!(
    create_split_view_dock,
    dyn ferrodock_model::controls::ISplitViewDock,
    SplitViewDock
);
creation!(
    create_document,
    dyn ferrodock_model::controls::IDocument,
    Document
);
creation!(create_tool, dyn ferrodock_model::controls::ITool, Tool);
creation!(create_dock_window, dyn IDockWindow, DockWindow);

impl FactoryBaseImpl for Factory {
    fn factory_base(&self) -> &FactoryBase {
        &self.base
    }
}
impl crate::FactoryImpl for Factory {
    fn factory(&self) -> &Factory {
        self
    }
}
crate::impl_factory!(Factory);
