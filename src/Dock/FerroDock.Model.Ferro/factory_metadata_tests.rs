// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native overload marshalling compared with direct calls to the original production engine.
use super::*;
use ferrodock_model::{
    collections::ListHandle,
    controls::{IDocumentDock, IToolDock},
    core::DockWindowOwnerMode,
};
use ferrodock_model_reactiveui::{
    controls::{Document, Tool},
    Factory,
};
use ferroui_base::{data::core::ValueType, metadata::MarkupTyped};
struct Scene {
    factory: Rc<Factory>,
    root: Rc<dyn IRootDock>,
    documents: Rc<dyn IDocumentDock>,
    tools: Rc<dyn IToolDock>,
    doc: Rc<dyn IDockable>,
    tool: Rc<dyn IDockable>,
}
impl Scene {
    fn new() -> Self {
        let factory = Factory::new();
        let root = factory.create_layout().unwrap();
        let documents = factory.create_document_dock();
        let tools = factory.create_tool_dock();
        factory.add_dockable(Some(root.clone()), Some(documents.clone()));
        factory.add_dockable(Some(root.clone()), Some(tools.clone()));
        let mut docs = Vec::new();
        for id in ["doc1", "doc2", "doc3"] {
            let d = Document::new();
            d.set_id(id.into());
            factory.add_dockable(Some(documents.clone()), Some(d.clone()));
            docs.push(d as Rc<dyn IDockable>);
        }
        let tool = Tool::new();
        tool.set_id("tool1".into());
        factory.add_dockable(Some(tools.clone()), Some(tool.clone()));
        let extra = Tool::new();
        extra.set_id("tool2".into());
        factory.add_dockable(Some(tools.clone()), Some(extra));
        factory.init_layout(root.clone());
        documents.set_can_close_last_dockable(true);
        tools.set_can_close_last_dockable(true);
        Self {
            factory,
            root,
            documents,
            tools,
            doc: docs[1].clone(),
            tool,
        }
    }
    fn target(&self, name: &str) -> Rc<dyn IDockable> {
        if name.starts_with("SetDocumentDock") {
            return self.documents.clone();
        }
        if [
            "PinDockable",
            "PreviewPinnedDockable",
            "TogglePreviewPinnedDockable",
            "UnpinDockable",
            "FloatDockable",
            "FloatAllDockables",
            "DockAsDocument",
        ]
        .contains(&name)
        {
            self.tool.clone()
        } else {
            self.doc.clone()
        }
    }
    fn prepare(&self, name: &str) {
        if [
            "PreviewPinnedDockable",
            "TogglePreviewPinnedDockable",
            "UnpinDockable",
            "HidePreviewingDockables",
        ]
        .contains(&name)
        {
            self.factory.pin_dockable(Some(self.tool.clone()));
        }
        if name == "HidePreviewingDockables" {
            self.factory
                .preview_pinned_dockable(Some(self.tool.clone()));
        }
        if name == "RestoreDockable" {
            self.factory.hide_dockable(Some(self.doc.clone()));
        }
        if name == "SetDocumentDockLayoutModeTabbed" {
            self.documents.set_layout_mode(DocumentLayoutMode::Mdi);
        }
        if name == "SetDocumentDockTabsLayoutTop" {
            self.documents.set_tabs_layout(DocumentTabLayout::Right);
        }
    }
    fn snapshot(&self) -> String {
        format!(
            "{}|doc:{}|tool:{}|tool-list:{}",
            tree(self.root.clone()),
            leaf(&self.doc),
            leaf(&self.tool),
            list(self.tools.visible_dockables())
        )
    }
}
fn kind(d: Rc<dyn IDockable>) -> String {
    let id = d.id();
    if ["doc1", "doc2", "doc3", "tool1", "tool2"].contains(&id.as_str()) {
        return id;
    }
    if d.clone().as_document_dock().is_some() {
        "documents".into()
    } else if d.clone().as_tool_dock().is_some() {
        "tools".into()
    } else if d.clone().as_root_dock().is_some() {
        "root".into()
    } else {
        "container".into()
    }
}
fn leaf(d: &Rc<dyn IDockable>) -> String {
    format!(
        "{}({:?},owner={:?},original={:?})",
        kind(d.clone()),
        d.docking_state(),
        d.owner().map(kind),
        d.original_owner().map(kind)
    )
}
fn list(l: Option<ListHandle<Option<Rc<dyn IDockable>>>>) -> String {
    l.map(|l| {
        l.iter()
            .map(|item| tree(item.expect("nonnull tree fixture")))
            .collect::<Vec<_>>()
            .join(",")
    })
    .unwrap_or_default()
}
fn tree(d: Rc<dyn IDockable>) -> String {
    let mut result = leaf(&d);
    if let Some(dock) = d.clone().as_dock() {
        result.push_str(&format!(
            "[{}];active={:?}",
            list(dock.visible_dockables()),
            dock.active_dockable().map(kind)
        ));
    }
    if let Some(doc) = d.clone().as_document_dock() {
        result.push_str(&format!(";{:?}/{:?}", doc.layout_mode(), doc.tabs_layout()));
    }
    if let Some(root) = d.as_root_dock() {
        result.push_str(&format!(
            ";hidden={};pins={}|{}|{}|{};preview={};windows={}",
            list(root.hidden_dockables()),
            list(root.left_pinned_dockables()),
            list(root.right_pinned_dockables()),
            list(root.top_pinned_dockables()),
            list(root.bottom_pinned_dockables()),
            root.pinned_dock().map(|d| tree(d)).unwrap_or_default(),
            root.windows()
                .map(|l| l
                    .iter()
                    .map(|w| w
                        .expect("nonnull tree fixture")
                        .layout()
                        .map(|r| tree(r))
                        .unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(","))
                .unwrap_or_default()
        ));
    }
    result
}
fn invoke(
    factory: Rc<dyn IFactory>,
    name: &str,
    types: &[ValueType],
    arguments: &[Option<BoxedValue>],
) -> Option<BoxedValue> {
    let m = <dyn IFactoryMetadata as MarkupTyped>::MARKUP
        .find_methods(name)
        .find(|m| {
            m.parameters.len() == types.len()
                && m.parameters
                    .iter()
                    .zip(types)
                    .all(|(a, b)| a().id() == b.id())
        })
        .unwrap_or_else(|| panic!("missing overload {name}"));
    let mut args = vec![Some(Rc::new(factory) as BoxedValue)];
    args.extend_from_slice(arguments);
    (m.invoke)(&args).unwrap()
}
fn direct(f: &dyn IFactory, name: &str, d: Rc<dyn IDockable>) {
    match name {
        "PinDockable" => f.pin_dockable(Some(d)),
        "PreviewPinnedDockable" => f.preview_pinned_dockable(Some(d)),
        "TogglePreviewPinnedDockable" => f.toggle_preview_pinned_dockable(Some(d)),
        "FloatDockable" => f.float_dockable(Some(d)),
        "FloatAllDockables" => f.float_all_dockables(Some(d)),
        "DockAsDocument" => f.dock_as_document(d),
        "CloseDockable" => f.close_dockable(Some(d)),
        "CloseOtherDockables" => f.close_other_dockables(d),
        "CloseAllDockables" => f.close_all_dockables(d),
        "CloseLeftDockables" => f.close_left_dockables(d),
        "CloseRightDockables" => f.close_right_dockables(d),
        "SetDocumentDockTabsLayoutLeft" => f.set_document_dock_tabs_layout_left(d),
        "SetDocumentDockTabsLayoutTop" => f.set_document_dock_tabs_layout_top(d),
        "SetDocumentDockTabsLayoutRight" => f.set_document_dock_tabs_layout_right(d),
        "SetDocumentDockLayoutModeTabbed" => f.set_document_dock_layout_mode_tabbed(d),
        "SetDocumentDockLayoutModeMdi" => f.set_document_dock_layout_mode_mdi(d),
        "NewHorizontalDocumentDock" => f.new_horizontal_document_dock(d),
        "NewVerticalDocumentDock" => f.new_vertical_document_dock(d),
        "UnpinDockable" => f.unpin_dockable(Some(d)),
        "HideDockable" => f.hide_dockable(Some(d)),
        "RestoreDockable" => f.restore_dockable(Some(d)),
        _ => panic!("unknown source operation"),
    }
}
const COMMANDS: [&str; 18] = [
    "PinDockable",
    "PreviewPinnedDockable",
    "TogglePreviewPinnedDockable",
    "FloatDockable",
    "FloatAllDockables",
    "DockAsDocument",
    "CloseDockable",
    "CloseOtherDockables",
    "CloseAllDockables",
    "CloseLeftDockables",
    "CloseRightDockables",
    "SetDocumentDockTabsLayoutLeft",
    "SetDocumentDockTabsLayoutTop",
    "SetDocumentDockTabsLayoutRight",
    "SetDocumentDockLayoutModeTabbed",
    "SetDocumentDockLayoutModeMdi",
    "NewHorizontalDocumentDock",
    "NewVerticalDocumentDock",
];
#[test]
fn typed_and_object_factory_overloads_produce_same_original_engine_graph_as_direct_source_calls() {
    crate::register_types();
    for name in COMMANDS
        .into_iter()
        .chain(["UnpinDockable", "HideDockable", "RestoreDockable"])
    {
        let expected = Scene::new();
        expected.prepare(name);
        direct(&*expected.factory, name, expected.target(name));
        let snapshot = expected.snapshot();
        for object in [false, true] {
            if object && ["UnpinDockable", "HideDockable", "RestoreDockable"].contains(&name) {
                continue;
            }
            let s = Scene::new();
            s.prepare(name);
            let t = if object {
                ValueType::of::<Option<BoxedValue>>()
            } else if [
                "CloseDockable",
                "PinDockable",
                "UnpinDockable",
                "PreviewPinnedDockable",
                "TogglePreviewPinnedDockable",
                "HideDockable",
                "RestoreDockable",
                "FloatDockable",
                "FloatAllDockables",
            ]
            .contains(&name)
            {
                ValueType::of::<Option<Rc<dyn IDockable>>>()
            } else {
                ValueType::of::<Rc<dyn IDockable>>()
            };
            invoke(
                s.factory.clone(),
                name,
                &[t],
                &[Some(Rc::new(s.target(name)))],
            );
            assert_eq!(s.snapshot(), snapshot, "{name}, object={object}");
        }
    }
}
#[test]
fn object_overloads_preserve_source_null_non_dockable_noop_and_parameter_value_identity() {
    crate::register_types();
    let s = Scene::new();
    let before = s.snapshot();
    let text: BoxedValue = Rc::new("invalid parameter".to_owned());
    let expected: ferrodock_model::BoxedValue = text.clone();
    let actual = parameter(Some(text.clone())).unwrap();
    assert!(Rc::ptr_eq(&actual, &expected));
    assert_eq!(
        actual.downcast_ref::<String>().unwrap(),
        "invalid parameter"
    );
    for name in COMMANDS {
        for arg in [None, Some(text.clone())] {
            invoke(
                s.factory.clone(),
                name,
                &[ValueType::of::<Option<BoxedValue>>()],
                &[arg],
            );
            assert_eq!(s.snapshot(), before, "{name}");
        }
    }
    let v: BoxedValue = Rc::new(Some(Rc::new(s.doc.clone()) as BoxedValue));
    assert!(Rc::ptr_eq(
        parameter(Some(v))
            .unwrap()
            .downcast_ref::<Rc<dyn IDockable>>()
            .unwrap(),
        &s.doc
    ));
}
#[test]
fn remaining_command_overloads_preserve_original_options_enum_null_and_window_operations() {
    crate::register_types();
    for name in ["FloatDockable", "FloatAllDockables"] {
        let s = Scene::new();
        let options = DockWindowOptions::new();
        options.set_owner_mode(DockWindowOwnerMode::None);
        options.set_show_in_taskbar(Some(false));
        invoke(
            s.factory.clone(),
            name,
            &[
                ValueType::of::<Option<Rc<dyn IDockable>>>(),
                ValueType::of::<Option<Rc<DockWindowOptions>>>(),
            ],
            &[Some(Rc::new(s.tool.clone())), Some(Rc::new(options))],
        );
        let windows = s.root.windows().unwrap();
        assert_eq!(windows.count(), 1);
        let w = windows.get(0).expect("nonnull window fixture");
        assert_eq!(w.owner_mode(), DockWindowOwnerMode::None);
        assert_eq!(w.show_in_taskbar(), Some(false));
        invoke(
            s.factory.clone(),
            "CloseWindow",
            &[ValueType::of::<Option<Rc<dyn IDockWindow>>>()],
            &[Some(Rc::new(w))],
        );
        assert_eq!(windows.count(), 0);
    }
    let s = Scene::new();
    s.prepare("HidePreviewingDockables");
    invoke(
        s.factory.clone(),
        "HidePreviewingDockables",
        &[ValueType::of::<Option<Rc<dyn IRootDock>>>()],
        &[Some(Rc::new(s.root.clone()))],
    );
    assert!(s
        .root
        .pinned_dock()
        .is_none_or(|d| d.visible_dockables().is_none_or(|l| l.count() == 0)));
    invoke(
        s.factory.clone(),
        "SetDocumentDockTabsLayout",
        &[
            ValueType::of::<Rc<dyn IDockable>>(),
            ValueType::of::<DocumentTabLayout>(),
        ],
        &[
            Some(Rc::new(s.documents.clone() as Rc<dyn IDockable>)),
            Some(Rc::new(DocumentTabLayout::Right)),
        ],
    );
    assert_eq!(s.documents.tabs_layout(), DocumentTabLayout::Right);
    invoke(
        s.factory.clone(),
        "SetDocumentDockLayoutMode",
        &[
            ValueType::of::<Rc<dyn IDockable>>(),
            ValueType::of::<DocumentLayoutMode>(),
        ],
        &[
            Some(Rc::new(s.documents.clone() as Rc<dyn IDockable>)),
            Some(Rc::new(DocumentLayoutMode::Mdi)),
        ],
    );
    assert_eq!(s.documents.layout_mode(), DocumentLayoutMode::Mdi);
    assert!(invoke(
        s.factory.clone(),
        "RestoreDockable",
        &[ValueType::of::<String>()],
        &[Some(Rc::new("missing".to_owned()))]
    )
    .is_none());
    invoke(
        s.factory.clone(),
        "HideDockable",
        &[ValueType::of::<String>()],
        &[Some(Rc::new("missing".to_owned()))],
    );
    invoke(
        s.factory.clone(),
        "CloseDockable",
        &[ValueType::of::<Option<Rc<dyn IDockable>>>()],
        &[None],
    );
    let m = <dyn IFactoryMetadata as MarkupTyped>::MARKUP;
    let value = Rc::new(s.factory.clone() as Rc<dyn IFactory>) as BoxedValue;
    for name in ["HideToolsOnClose", "HideDocumentsOnClose"] {
        let p = m.find_property(name).unwrap();
        (p.set.unwrap())(&[Some(value.clone()), Some(Rc::new(true))]).unwrap();
        assert_eq!(
            (p.get.unwrap())(&[Some(value.clone())])
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>(),
            Some(&true)
        );
    }
}

type FloatingRequest = (Rc<dyn IDockable>, Option<Rc<DockWindowOptions>>);
struct ApplicationFactory {
    component: Factory,
    closed: std::cell::RefCell<Vec<Option<Rc<dyn IDockable>>>>,
    floated: std::cell::RefCell<Option<FloatingRequest>>,
}
impl ApplicationFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            component: Factory::construct(runtime.clone()),
            closed: Default::default(),
            floated: Default::default(),
        })
    }
}
impl ferrodock_model::FactoryBaseImpl for ApplicationFactory {
    fn factory_base(&self) -> &ferrodock_model::FactoryBase {
        self.component.base()
    }
}
impl ferrodock_model_reactiveui::FactoryImpl for ApplicationFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn close_dockable(this: &Self, dockable: Option<Rc<dyn IDockable>>) {
        this.closed.borrow_mut().push(dockable);
    }
    fn float_dockable_with_options(
        this: &Self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        *this.floated.borrow_mut() =
            Some((dockable.expect("nonnull application fixture"), options));
    }
}
ferrodock_model_reactiveui::impl_factory!(ApplicationFactory);
#[test]
fn native_command_metadata_dispatches_enclosing_factory_overrides_with_original_identity() {
    crate::register_types();
    let f = ApplicationFactory::new();
    let d: Rc<dyn IDockable> = Document::new();
    let typed = ValueType::of::<Option<Rc<dyn IDockable>>>();
    let object = ValueType::of::<Option<BoxedValue>>();
    invoke(
        f.clone(),
        "CloseDockable",
        &[typed],
        &[Some(Rc::new(d.clone()))],
    );
    invoke(
        f.clone(),
        "CloseDockable",
        &[object],
        &[Some(Rc::new(d.clone()))],
    );
    invoke(f.clone(), "CloseDockable", &[object], &[None]);
    invoke(f.clone(), "CloseDockable", &[typed], &[None]);
    let calls = f.closed.borrow();
    assert_eq!(calls.len(), 3);
    assert!(Rc::ptr_eq(calls[0].as_ref().unwrap(), &d));
    assert!(Rc::ptr_eq(calls[1].as_ref().unwrap(), &d));
    assert!(calls[2].is_none());
    drop(calls);
    let options = DockWindowOptions::new();
    invoke(
        f.clone(),
        "FloatDockable",
        &[
            ValueType::of::<Option<Rc<dyn IDockable>>>(),
            ValueType::of::<Option<Rc<DockWindowOptions>>>(),
        ],
        &[Some(Rc::new(d.clone())), Some(Rc::new(options.clone()))],
    );
    let call = f.floated.borrow();
    let (item, actual) = call.as_ref().unwrap();
    assert!(Rc::ptr_eq(item, &d));
    assert!(Rc::ptr_eq(actual.as_ref().unwrap(), &options));
}

#[test]
fn source_factory_helpers_export_all_static_names_nullable_parameters_and_actual_results() {
    crate::register_types();
    let metadata = <crate::FactoryExtensionsMetadata as MarkupTyped>::MARKUP;
    let expected = [
        "GetActiveRoot",
        "GetCurrentDocument",
        "CloseFocusedDockable",
        "GetCurrentDockWindow",
        "GetCurrentHostWindow",
    ];
    assert_eq!(metadata.methods.len(), expected.len());
    for name in expected {
        let method = metadata.find_methods(name).next().unwrap();
        assert_eq!(method.parameters.len(), 1);
        assert!(
            (method.invoke)(&[None])
                .unwrap()
                .and_then(ValueTypes::normalize)
                .is_none(),
            "{name}"
        );
    }
    let scene = Scene::new();
    scene
        .factory
        .set_focused_dockable(Some(scene.documents.clone()), Some(scene.doc.clone()));
    let factory: Rc<dyn IFactory> = scene.factory.clone();
    let argument: BoxedValue = Rc::new(factory);
    let method = metadata.find_methods("GetActiveRoot").next().unwrap();
    let root = (method.invoke)(&[Some(argument.clone())]).unwrap().unwrap();
    let root = ferroui_base::metadata::from_markup_value::<Rc<dyn IRootDock>>(&Some(root)).unwrap();
    assert!(Rc::ptr_eq(&root, &scene.root));
    let method = metadata.find_methods("GetCurrentDocument").next().unwrap();
    let document = (method.invoke)(&[Some(argument)]).unwrap().unwrap();
    let document = ferroui_base::metadata::from_markup_value::<
        Rc<dyn ferrodock_model::controls::IDocument>,
    >(&Some(document))
    .unwrap();
    assert!(Rc::ptr_eq(&(document as Rc<dyn IDockable>), &scene.doc));
}

#[test]
fn factory_base_exports_original_commands_with_the_same_engine_and_object_overloads() {
    crate::register_types();
    let metadata = crate::FactoryBaseReference::MARKUP;
    for name in COMMANDS
        .into_iter()
        .chain(["UnpinDockable", "HideDockable", "RestoreDockable"])
    {
        let expected = Scene::new();
        expected.prepare(name);
        direct(&*expected.factory, name, expected.target(name));
        let snapshot = expected.snapshot();
        for object in [false, true] {
            if object && ["UnpinDockable", "HideDockable", "RestoreDockable"].contains(&name) {
                continue;
            }
            let scene = Scene::new();
            scene.prepare(name);
            let type_ = if object {
                ValueType::of::<Option<BoxedValue>>()
            } else if [
                "CloseDockable",
                "PinDockable",
                "UnpinDockable",
                "PreviewPinnedDockable",
                "TogglePreviewPinnedDockable",
                "HideDockable",
                "RestoreDockable",
                "FloatDockable",
                "FloatAllDockables",
            ]
            .contains(&name)
            {
                ValueType::of::<Option<Rc<dyn IDockable>>>()
            } else {
                ValueType::of::<Rc<dyn IDockable>>()
            };
            let method = metadata
                .find_methods(name)
                .find(|method| method.parameters.len() == 1 && (method.parameters[0])() == type_)
                .unwrap();
            (method.invoke)(&[
                Some(Rc::new(crate::FactoryBaseReference::new(
                    scene.factory.clone(),
                ))),
                Some(Rc::new(scene.target(name))),
            ])
            .unwrap();
            assert_eq!(scene.snapshot(), snapshot, "{name}, object={object}");
        }
    }
}
