// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{
    docks::CustomDocumentDock,
    documents::DocumentViewModel,
    tools::*,
    views::{DashboardViewModel, HomeViewModel},
};
use crate::models::{documents::DemoDocument, tools::*};
use ferrodock_ferro::controls::{HostWindow, ManagedHostWindow};
use ferrodock_model::{controls::*, core::*, BoxedValue, FactoryBase, FactoryBaseImpl};
use ferrodock_model_reactiveui::{
    controls::{ProportionalDock, ProportionalDockSplitter, ToolDock},
    impl_factory, Factory, FactoryImpl,
};
use ferrodock_settings::DockSettings;
use ferroui_system::collections::generic::Dictionary;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
/// The original sample's complete layout and context/host locator factory.
pub struct DockFactory {
    base: Factory,
    runtime: Weak<Self>,
    context: BoxedValue,
    root_dock: RefCell<Option<Rc<dyn IRootDock>>>,
    document_dock: RefCell<Option<Rc<dyn IDocumentDock>>>,
}
impl DockFactory {
    /// Creates a factory retaining the supplied original sample context.
    pub fn new(context: BoxedValue) -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            base: Factory::construct(runtime.clone()),
            runtime: runtime.clone(),
            context,
            root_dock: RefCell::new(None),
            document_dock: RefCell::new(None),
        })
    }
}
impl FactoryBaseImpl for DockFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.base.base()
    }
}
impl FactoryImpl for DockFactory {
    fn factory(&self) -> &Factory {
        &self.base
    }
    fn create_document_dock(_this: &Self) -> Rc<dyn IDocumentDock> {
        CustomDocumentDock::new()
    }
    fn create_layout(this: &Self) -> Option<Rc<dyn IRootDock>> {
        let document1 = DocumentViewModel::new();
        document1.set_id("Document1".into());
        document1.set_title("Document1".into());
        let document2 = DocumentViewModel::new();
        document2.set_id("Document2".into());
        document2.set_title("Document2".into());
        let document3 = DocumentViewModel::new();
        document3.set_id("Document3".into());
        document3.set_title("Document3".into());
        document3.set_can_close(true);
        let tool1 = Tool1ViewModel::new();
        tool1.set_id("Tool1".into());
        tool1.set_title("Tool1".into());
        tool1.set_keep_pinned_dockable_visible(true);
        let tool2 = Tool2ViewModel::new();
        tool2.set_id("Tool2".into());
        tool2.set_title("Tool2".into());
        tool2.set_keep_pinned_dockable_visible(true);
        let tool3 = Tool3ViewModel::new();
        tool3.set_id("Tool3".into());
        tool3.set_title("Tool3".into());
        tool3.set_can_drag(false);
        let tool4 = Tool4ViewModel::new();
        tool4.set_id("Tool4".into());
        tool4.set_title("Tool4".into());
        tool4.set_can_drag(false);
        let tool5 = Tool5ViewModel::new();
        tool5.set_id("Tool5".into());
        tool5.set_title("Tool5".into());
        let tool6 = Tool6ViewModel::new();
        tool6.set_id("Tool6".into());
        tool6.set_title("Tool6".into());
        tool6.set_can_close(true);
        tool6.set_can_pin(true);
        let tool7 = Tool7ViewModel::new();
        tool7.set_id("Tool7".into());
        tool7.set_title("Tool7".into());
        tool7.set_can_close(false);
        tool7.set_can_pin(false);
        let tool8 = Tool8ViewModel::new();
        tool8.set_id("Tool8".into());
        tool8.set_title("Tool8".into());
        tool8.set_can_close(false);
        tool8.set_can_pin(true);
        let left_dock = ProportionalDock::new();
        left_dock.set_proportion(0.25);
        left_dock.set_orientation(Orientation::Vertical);
        left_dock.set_active_dockable(None);
        let left_top = ToolDock::new();
        left_top.set_active_dockable(Some(tool1.clone()));
        left_top.set_visible_dockables(Some(
            this.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(tool1), Some(tool2)]),
        ));
        left_top.set_alignment(Alignment::Left);
        let left_splitter = ProportionalDockSplitter::new();
        left_splitter.set_can_resize(true);
        let left_bottom = ToolDock::new();
        left_bottom.set_active_dockable(Some(tool3.clone()));
        left_bottom.set_visible_dockables(Some(
            this.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(tool3), Some(tool4)]),
        ));
        left_bottom.set_alignment(Alignment::Bottom);
        left_bottom.set_can_drag(false);
        left_bottom.set_can_drop(false);
        left_dock.set_visible_dockables(Some(this.create_list::<Option<Rc<dyn IDockable>>>(vec![
            Some(left_top),
            Some(left_splitter),
            Some(left_bottom),
        ])));
        let right_dock = ProportionalDock::new();
        right_dock.set_proportion(0.25);
        right_dock.set_orientation(Orientation::Vertical);
        right_dock.set_active_dockable(None);
        let right_top = ToolDock::new();
        right_top.set_active_dockable(Some(tool5.clone()));
        right_top.set_visible_dockables(Some(
            this.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(tool5), Some(tool6)]),
        ));
        right_top.set_alignment(Alignment::Top);
        right_top.set_grip_mode(GripMode::Hidden);
        let right_splitter = ProportionalDockSplitter::new();
        let right_bottom = ToolDock::new();
        right_bottom.set_active_dockable(Some(tool7.clone()));
        right_bottom.set_visible_dockables(Some(
            this.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(tool7), Some(tool8)]),
        ));
        right_bottom.set_alignment(Alignment::Right);
        right_bottom.set_grip_mode(GripMode::AutoHide);
        right_dock.set_visible_dockables(Some(this.create_list::<Option<Rc<dyn IDockable>>>(
            vec![Some(right_top), Some(right_splitter), Some(right_bottom)],
        )));
        let document_dock = CustomDocumentDock::new();
        document_dock.set_is_collapsable(false);
        document_dock.set_active_dockable(Some(document1.clone()));
        document_dock.set_visible_dockables(Some(this.create_list::<Option<Rc<dyn IDockable>>>(
            vec![Some(document1), Some(document2), Some(document3)],
        )));
        document_dock.set_can_create_document(true);
        document_dock.set_enable_window_drag(true);
        let main_layout = ProportionalDock::new();
        main_layout.set_orientation(Orientation::Horizontal);
        main_layout.set_visible_dockables(Some(this.create_list::<Option<Rc<dyn IDockable>>>(
            vec![
                Some(left_dock),
                Some(ProportionalDockSplitter::new()),
                Some(document_dock.clone()),
                Some(ProportionalDockSplitter::new()),
                Some(right_dock),
            ],
        )));
        let dashboard = DashboardViewModel::new();
        dashboard.set_id("Dashboard".into());
        dashboard.set_title("Dashboard".into());
        let home = HomeViewModel::new();
        home.set_id("Home".into());
        home.set_title("Home".into());
        home.set_active_dockable(Some(main_layout.clone()));
        home.set_visible_dockables(Some(
            this.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(main_layout)]),
        ));
        let root = this.create_root_dock();
        root.set_is_collapsable(false);
        root.set_active_dockable(Some(dashboard.clone()));
        root.set_default_dockable(Some(home.clone()));
        root.set_visible_dockables(Some(
            this.create_list::<Option<Rc<dyn IDockable>>>(vec![Some(dashboard), Some(home)]),
        ));
        root.set_left_pinned_dockables(Some(this.create_list(Vec::new())));
        root.set_right_pinned_dockables(Some(this.create_list(Vec::new())));
        root.set_top_pinned_dockables(Some(this.create_list(Vec::new())));
        root.set_bottom_pinned_dockables(Some(this.create_list(Vec::new())));
        root.set_pinned_dock(None);
        this.document_dock.replace(Some(document_dock));
        this.root_dock.replace(Some(root.clone()));
        Some(root)
    }
    fn create_window_from(
        this: &Self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        let window = this.base.base().create_window_from(dockable);
        if let Some(window) = &window {
            window.set_title("Dock Ferro Demo".into());
        }
        window
    }
    fn init_layout(this: &Self, layout: Rc<dyn IDockable>) {
        let contexts: ContextLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
        for id in ["Document1", "Document2", "Document3"] {
            contexts.borrow_mut().add(
                id.into(),
                Some(Rc::new(|| Some(DemoDocument::new() as BoxedValue))),
            );
        }
        macro_rules! tool_context {
            ($id:literal,$type:ty) => {
                contexts.borrow_mut().add(
                    $id.into(),
                    Some(Rc::new(|| Some(<$type>::new() as BoxedValue))),
                );
            };
        }
        tool_context!("Tool1", Tool1);
        tool_context!("Tool2", Tool2);
        tool_context!("Tool3", Tool3);
        tool_context!("Tool4", Tool4);
        tool_context!("Tool5", Tool5);
        tool_context!("Tool6", Tool6);
        tool_context!("Tool7", Tool7);
        tool_context!("Tool8", Tool8);
        let dashboard = layout.clone();
        contexts.borrow_mut().add(
            "Dashboard".into(),
            Some(Rc::new(move || {
                Some(Rc::new(dashboard.clone()) as BoxedValue)
            })),
        );
        let owner = this.runtime.upgrade().expect("enclosing source factory");
        contexts.borrow_mut().add(
            "Home".into(),
            Some(Rc::new(move || Some(owner.context.clone()))),
        );
        this.set_context_locator(Some(contexts));
        let dockables: DockableLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
        let owner = this.runtime.upgrade().expect("enclosing source factory");
        dockables.borrow_mut().add(
            "Root".into(),
            Some(Rc::new(move || {
                owner
                    .root_dock
                    .borrow()
                    .clone()
                    .map(|dock| dock as Rc<dyn IDockable>)
            })),
        );
        let owner = this.runtime.upgrade().expect("enclosing source factory");
        dockables.borrow_mut().add(
            "Documents".into(),
            Some(Rc::new(move || {
                owner
                    .document_dock
                    .borrow()
                    .clone()
                    .map(|dock| dock as Rc<dyn IDockable>)
            })),
        );
        this.set_dockable_locator(Some(dockables));
        let hosts: HostWindowLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
        hosts.borrow_mut().add(
            "IDockWindow".into(),
            Some(Rc::new(|| {
                Some(if DockSettings::use_managed_windows() {
                    ManagedHostWindow::new() as Rc<dyn IHostWindow>
                } else {
                    HostWindow::as_host_window(HostWindow::new())
                })
            })),
        );
        this.set_host_window_locator(Some(hosts));
        this.base.base().init_layout(layout);
    }
}
impl_factory!(DockFactory);

impl crate::services::ILayoutInitialization for DockFactory {
    fn initialize(&self, layout: Option<Rc<dyn IRootDock>>) {
        let Some(layout) = layout else {
            self.root_dock.replace(None);
            self.document_dock.replace(None);
            self.set_context_locator(None);
            self.set_dockable_locator(None);
            self.set_host_window_locator(None);
            return;
        };
        let documents = self
            .find_dockable(
                Some(layout.clone()),
                Some(Rc::new(|dockable| {
                    dockable
                        .and_then(|dockable| dockable.as_document_dock())
                        .is_some()
                })),
            )
            .and_then(|dockable| dockable.as_document_dock());
        self.root_dock.replace(Some(layout.clone()));
        self.document_dock.replace(documents);
        self.init_layout(layout);
    }
}
