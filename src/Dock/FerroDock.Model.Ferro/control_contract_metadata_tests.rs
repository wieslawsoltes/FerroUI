// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source model-controls remain usable without loading the native UI assembly.
use crate::*;
use ferrodock_model::{collections::ListHandle, controls::*, core::*};
use ferrodock_model_reactiveui::{controls::*, core::DockableBaseImpl, Factory};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupAssembly, MarkupType, MarkupTyped},
    BoxedValue,
};
use ferroui_reactiveui::reactive_object;
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{any::TypeId, cell::RefCell, rc::Rc};

fn get(m: &MarkupType, value: &BoxedValue, name: &str) -> Option<BoxedValue> {
    (m.find_property(name).unwrap().get.unwrap())(&[Some(value.clone())]).unwrap()
}
fn set(m: &MarkupType, value: &BoxedValue, name: &str, new: Option<BoxedValue>) {
    (m.find_property(name).unwrap().set.unwrap())(&[Some(value.clone()), new]).unwrap();
}
#[test]
fn original_model_assembly_owns_all_migrated_control_handles_and_their_registered_notifiers() {
    register_types();
    macro_rules! check {($($t:ty),+)=>{$(
        let m = MarkupType::find_by_handle(TypeId::of::<Rc<$t>>()).unwrap();
        assert_eq!(m.namespace(),"FerroDock.Model.Controls");
        assert_eq!(MarkupAssembly::of_module(m.module_path).unwrap().name,"FerroDock.Model");
        assert!(m.notify_property_changed.is_some());
        assert!(std::ptr::eq(m,MarkupType::find_by_handle(TypeId::of::<Option<Rc<$t>>>()).unwrap()));
    )+};}
    check!(
        dyn IDockDock,
        dyn IDocument,
        dyn IDocumentContent,
        dyn IDocumentDock,
        dyn IDocumentDockContent,
        dyn IDocumentTemplate,
        dyn IGridDock,
        dyn IGridDockSplitter,
        dyn IMdiDocument,
        dyn IProportionalDock,
        dyn IProportionalDockSplitter,
        dyn IRootDock,
        dyn ISplitViewDock,
        dyn ISplitter,
        dyn IStackDock,
        dyn ITool,
        dyn IToolContent,
        dyn IToolDock,
        dyn IToolDockContent,
        dyn IToolTemplate,
        dyn IUniformGridDock,
        dyn IWrapDock
    );
}
#[test]
fn source_mdi_inherits_document_and_local_target_and_keeps_original_notifier_and_setters() {
    register_types();
    let source = Document::new();
    let model: Rc<dyn IMdiDocument> = source.clone();
    let boxed: BoxedValue = Rc::new(model.clone());
    let m = MarkupType::find_by_handle(TypeId::of::<Rc<dyn IMdiDocument>>()).unwrap();
    assert_eq!(m.base_type().unwrap().name, "IDocument");
    assert_eq!(
        m.base_type().unwrap().base_type().unwrap().name,
        "IDockable"
    );
    for target in [
        ValueType::of::<Rc<dyn IDocument>>(),
        ValueType::of::<Rc<dyn ILocalTarget>>(),
        ValueType::of::<Rc<dyn IDockable>>(),
    ] {
        let view = ValueTypes::try_cast(&boxed, target).unwrap();
        assert!(ValueTypes::reference_equals(&boxed, &view));
    }
    let notifier = (m.notify_property_changed.unwrap())(boxed.as_ref()).unwrap();
    assert!(std::ptr::eq(
        notifier,
        model.as_notify_property_changed().unwrap()
    ));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let token = notifier.property_changed().add(Rc::new(move |name| {
        observed.borrow_mut().push(name.to_owned())
    }));
    set(m, &boxed, "MdiZIndex", Some(Rc::new(7_i32)));
    set(
        m,
        &boxed,
        "MdiState",
        Some(Rc::new(MdiWindowState::Maximized)),
    );
    assert_eq!(source.mdi_z_index(), 7);
    assert_eq!(source.mdi_state(), MdiWindowState::Maximized);
    assert_eq!(&*changes.borrow(), &["MdiZIndex", "MdiState"]);
    notifier.property_changed().remove(token);
    for m in [
        <dyn IDocumentDockMetadata>::MARKUP,
        <dyn IToolDockMetadata>::MARKUP,
    ] {
        assert!(m
            .interfaces
            .iter()
            .any(|v| v() == ValueType::of::<Rc<dyn ILocalTarget>>()));
    }
}
#[derive(Default)]
struct Template {
    content: RefCell<Option<ferrodock_model::BoxedValue>>,
    changed: Event<str>,
}
impl INotifyPropertyChanged for Template {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}
impl IToolTemplate for Template {
    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        Some(self)
    }
    fn content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.content.borrow().clone()
    }
    fn set_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        *self.content.borrow_mut() = value;
        self.changed.raise("Content");
    }
}
#[test]
fn source_tool_template_metadata_retains_arbitrary_content_identity_null_and_original_events() {
    register_types();
    let source = Rc::new(Template::default());
    let template: Rc<dyn IToolTemplate> = source.clone();
    let value: BoxedValue = Rc::new(template);
    let m = MarkupType::find_by_handle(TypeId::of::<Rc<dyn IToolTemplate>>()).unwrap();
    let notifier = (m.notify_property_changed.unwrap())(value.as_ref()).unwrap();
    assert!(std::ptr::eq(
        notifier,
        source.as_ref() as &dyn INotifyPropertyChanged
    ));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let token = notifier.property_changed().add(Rc::new(move |name| {
        observed.borrow_mut().push(name.to_owned())
    }));
    for content in [Rc::new("content".to_owned()) as BoxedValue, Rc::new(42_u32)] {
        set(m, &value, "Content", Some(content.clone()));
        assert!(Rc::ptr_eq(&get(m, &value, "Content").unwrap(), &content));
    }
    set(m, &value, "Content", None);
    assert!(source.content().is_none());
    assert!(get(m, &value, "Content").is_none());
    assert_eq!(&*changes.borrow(), &["Content", "Content", "Content"]);
    notifier.property_changed().remove(token);
}

struct ContentDock {
    base: ToolDock,
    template: RefCell<Option<Rc<dyn IToolTemplate>>>,
}
reactive_object!(ContentDock, base);
impl DockableBaseImpl for ContentDock {}
ferrodock_model_reactiveui::impl_dock!(@base ContentDock,ToolDock,
    fn as_tool_dock(self:Rc<Self>)->Option<Rc<dyn IToolDock>>{Some(self)},
    fn as_tool_dock_content(self:Rc<Self>)->Option<Rc<dyn IToolDockContent>>{Some(self)}
);
impl ILocalTarget for ContentDock {}
impl IToolDock for ContentDock {
    fn alignment(&self) -> Alignment {
        self.base.alignment()
    }
    fn set_alignment(&self, v: Alignment) {
        self.base.set_alignment(v)
    }
    fn is_expanded(&self) -> bool {
        self.base.is_expanded()
    }
    fn set_is_expanded(&self, v: bool) {
        self.base.set_is_expanded(v)
    }
    fn auto_hide(&self) -> bool {
        self.base.auto_hide()
    }
    fn set_auto_hide(&self, v: bool) {
        self.base.set_auto_hide(v)
    }
    fn grip_mode(&self) -> GripMode {
        self.base.grip_mode()
    }
    fn set_grip_mode(&self, v: GripMode) {
        self.base.set_grip_mode(v)
    }
    fn add_tool(&self, v: Option<Rc<dyn IDockable>>) {
        self.base.add_tool_nullable_core(v)
    }
}
impl IToolDockContent for ContentDock {
    fn tool_template(&self) -> Option<Rc<dyn IToolTemplate>> {
        self.template.borrow().clone()
    }
    fn set_tool_template(&self, v: Option<Rc<dyn IToolTemplate>>) {
        *self.template.borrow_mut() = v;
    }
    fn create_tool_from_template(&self) -> Option<ferrodock_model::BoxedValue> {
        self.tool_template().and_then(|t| t.content())
    }
}
#[test]
fn optional_tool_content_dispatches_enclosing_source_contract_and_checked_casts() {
    register_types();
    let source = Rc::new_cyclic(|weak| ContentDock {
        base: ToolDock::construct(weak),
        template: RefCell::new(None),
    });
    source.base.initialize(&source);
    let model: Rc<dyn IToolDockContent> = source.clone();
    let boxed: BoxedValue = Rc::new(model.clone());
    let source_view: Rc<dyn IDockable> = source.clone();
    let source_box: BoxedValue = Rc::new(source_view);
    let cast =
        ValueTypes::try_cast(&source_box, ValueType::of::<Rc<dyn IToolDockContent>>()).unwrap();
    assert!(ValueTypes::reference_equals(&boxed, &cast));
    let ordinary: Rc<dyn IDockable> = ToolDock::new();
    assert!(ValueTypes::try_cast(
        &(Rc::new(ordinary) as BoxedValue),
        ValueType::of::<Rc<dyn IToolDockContent>>()
    )
    .is_none());
    let template: Rc<dyn IToolTemplate> = Rc::new(Template::default());
    let content: BoxedValue = Rc::new(23_u32);
    template.set_content(Some(content.clone()));
    let m = <dyn IToolDockContentMetadata>::MARKUP;
    set(m, &boxed, "ToolTemplate", Some(Rc::new(template.clone())));
    assert!(Rc::ptr_eq(&source.tool_template().unwrap(), &template));
    let result = (m
        .find_methods("CreateToolFromTemplate")
        .next()
        .unwrap()
        .invoke)(&[Some(boxed.clone())])
    .unwrap()
    .unwrap();
    assert!(Rc::ptr_eq(&result, &content));
    let exposed =
        from_markup_value::<Rc<dyn IToolTemplate>>(&get(m, &boxed, "ToolTemplate")).unwrap();
    assert!(Rc::ptr_eq(&exposed, &template));
    set(m, &boxed, "ToolTemplate", None);
    assert!(source.tool_template().is_none());
    assert!((m
        .find_methods("CreateToolFromTemplate")
        .next()
        .unwrap()
        .invoke)(&[Some(boxed)])
    .unwrap()
    .is_none());
}
#[test]
fn root_window_collection_metadata_mutates_same_source_list_and_forwards_original_count_events() {
    register_types();
    assert_eq!(
        <dyn IRootDockMetadata>::MARKUP
            .properties
            .iter()
            .map(|p| p.name)
            .collect::<Vec<_>>(),
        [
            "IsFocusableRoot",
            "HiddenDockables",
            "LeftPinnedDockables",
            "RightPinnedDockables",
            "TopPinnedDockables",
            "BottomPinnedDockables",
            "PinnedDock",
            "PinnedDockDisplayMode",
            "Window",
            "Windows",
            "FloatingWindowHostMode",
            "RootDockCapabilityPolicy",
            "ShowWindows",
            "ExitWindows",
            "EnableAdaptiveGlobalDockTargets"
        ],
    );
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let windows = factory.create_list::<Option<Rc<dyn IDockWindow>>>(Vec::new());
    let boxed: BoxedValue = Rc::new(root.clone());
    set(
        <dyn IRootDockMetadata>::MARKUP,
        &boxed,
        "Windows",
        Some(Rc::new(windows.clone())),
    );
    let exposed = from_markup_value::<ListHandle<Option<Rc<dyn IDockWindow>>>>(&get(
        <dyn IRootDockMetadata>::MARKUP,
        &boxed,
        "Windows",
    ))
    .unwrap();
    assert!(exposed.ptr_eq(&windows));
    let list: BoxedValue = Rc::new(exposed);
    let m = MarkupType::find_by_handle(TypeId::of::<ListHandle<Option<Rc<dyn IDockWindow>>>>())
        .unwrap();
    assert!(ValueTypes::reference_equals(
        &list,
        &(Rc::new(windows.clone()) as BoxedValue)
    ));
    let notifier = (m.notify_property_changed.unwrap())(list.as_ref()).unwrap();
    assert!(std::ptr::eq(
        notifier,
        windows.as_notify_property_changed().unwrap()
    ));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let token = notifier.property_changed().add(Rc::new(move |name| {
        observed.borrow_mut().push(name.to_owned())
    }));
    let window = factory.create_dock_window();
    (m.find_methods("Add").next().unwrap().invoke)(&[
        Some(list.clone()),
        Some(Rc::new(window.clone())),
    ])
    .unwrap();
    assert!(Rc::ptr_eq(
        &root
            .windows()
            .unwrap()
            .get(0)
            .expect("nonnull window fixture"),
        &window
    ));
    assert_eq!(
        get(m, &list, "Count").unwrap().downcast_ref::<i32>(),
        Some(&1)
    );
    let result = (m.find_methods("Remove").next().unwrap().invoke)(&[
        Some(list.clone()),
        Some(Rc::new(window)),
    ])
    .unwrap()
    .unwrap();
    assert_eq!(result.downcast_ref::<bool>(), Some(&true));
    assert!(windows.is_empty());
    assert_eq!(
        changes
            .borrow()
            .iter()
            .filter(|n| n.as_str() == "Count")
            .count(),
        2
    );
    notifier.property_changed().remove(token);
}

#[test]
fn standalone_export_contains_original_control_owners_inheritance_and_no_unread_casts() {
    let model = ferroui_build_scan::model::AssemblyModel::parse(include_str!(concat!(
        env!("OUT_DIR"),
        "/ferrodock_model_ferro.xamlmeta"
    )))
    .unwrap();
    assert_eq!(model.name, "FerroDock.Model");
    assert!(
        model.unread_value_types.is_empty(),
        "{:?}",
        model.unread_value_types
    );
    let controls: Vec<_> = model
        .types
        .iter()
        .filter(|t| t.namespace == "FerroDock.Model.Controls")
        .collect();
    assert_eq!(controls.len(), 23);
    let mdi = controls.iter().find(|t| t.name == "IMdiDocument").unwrap();
    assert_eq!(
        mdi.base.as_ref().unwrap().text,
        "::std::rc::Rc<dyn ::ferrodock_model::controls::IDocument>"
    );
    for name in ["IDocumentDock", "IToolDock"] {
        let t = controls.iter().find(|t| t.name == name).unwrap();
        assert!(t
            .interfaces
            .iter()
            .any(|t| t.text == "::std::rc::Rc<dyn ::ferrodock_model::core::ILocalTarget>"));
    }
    assert!(controls.iter().all(|t| t
        .public_path
        .as_deref()
        .is_some_and(|path| path.starts_with("::ferrodock_model_ferro::"))));
}
