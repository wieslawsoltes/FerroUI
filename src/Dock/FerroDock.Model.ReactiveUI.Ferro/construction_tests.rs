// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Tests the integration SDK without registering or depending on the UI control library.
use crate::{reactive_metadata::*, register_types};
use ferrodock_model::{controls::IDocument, core::*};
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    BoxedValue,
};
use std::rc::Rc;

#[test]
fn sdk_alone_registers_real_model_constructors_and_shared_class_identity() {
    register_types();
    let reference = DocumentReference::create();
    let value: BoxedValue = Rc::new(reference.clone());
    let model: BoxedValue = Rc::new(reference.model());
    let original = reference.model().boxed_reference();
    assert!(original.downcast_ref::<Rc<Document>>().is_some());
    assert!(ValueTypes::reference_equals(&value, &model));
    let original_value: BoxedValue =
        Rc::new(original.downcast_ref::<Rc<Document>>().unwrap().clone());
    assert!(ValueTypes::reference_equals(&value, &original_value));
    let document = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDocument>>()).unwrap();
    assert!(ValueTypes::reference_equals(&document, &model));
    let other: BoxedValue = Rc::new(DocumentReference::create());
    assert!(!ValueTypes::reference_equals(&value, &other));
    assert_eq!(
        DocumentReference::MARKUP.full_name(),
        "FerroDock.Model.ReactiveUI.Controls.Document"
    );
}

#[test]
fn sdk_constructors_expose_source_defaults_and_original_reactive_commands() {
    register_types();
    let document = DocumentReference::create();
    let tool = ToolReference::create();
    let documents = DocumentDockReference::create();
    let root = RootDockReference::create();
    assert_eq!(document.model().title(), "");
    assert_eq!(tool.model().title(), "");
    assert!(!document
        .model()
        .as_docking_window_state()
        .unwrap()
        .is_open());
    assert!(!documents.model().can_create_document());
    assert!(documents.model().create_document().is_some());
    assert!(documents.model().cascade_documents().is_some());
    assert!(root.model().is_focusable_root());
    assert_eq!(
        root.model().pinned_dock_display_mode(),
        PinnedDockDisplayMode::Overlay
    );
    let source_command = documents.model().create_document().unwrap();
    let getter = DocumentDockReference::MARKUP
        .find_property("CreateDocument")
        .unwrap()
        .get
        .unwrap();
    let value = getter(&[Some(Rc::new(documents.clone()))]).unwrap();
    let exported_command = ferroui_base::metadata::from_markup_value::<
        Option<Rc<dyn ferroui_reactiveui::ICommand>>,
    >(&value)
    .unwrap()
    .unwrap();
    assert!(Rc::ptr_eq(&source_command, &exported_command));
    let setter = DocumentDockReference::MARKUP
        .find_property("CreateDocument")
        .unwrap()
        .set
        .unwrap();
    let instance: BoxedValue = Rc::new(documents.clone());
    setter(&[Some(instance.clone()), None]).unwrap();
    assert!(documents.model().create_document().is_none());
    assert!(getter(&[Some(instance.clone())]).unwrap().is_none());
    setter(&[Some(instance), Some(Rc::new(source_command.clone()))]).unwrap();
    assert!(Rc::ptr_eq(
        &source_command,
        &documents.model().create_document().unwrap()
    ));

    document.model().set_title("Same engine".into());
    let original = document.model().boxed_reference();
    assert_eq!(
        original.downcast_ref::<Rc<Document>>().unwrap().title(),
        "Same engine"
    );
}

#[test]
fn layout_class_views_keep_original_concrete_interface_and_base_identity_without_controls() {
    register_types();
    macro_rules! check {
        ($model:ident,$reference:ident,$contract:ty) => {{
            let model = ferrodock_model_reactiveui::controls::$model::new();
            let weak = Rc::downgrade(&model);
            let reference = $reference::new(model.clone());
            let value: BoxedValue = Rc::new(reference.clone());
            let concrete: BoxedValue = Rc::new(model.clone());
            let interface: BoxedValue = Rc::new(reference.model());
            let base: BoxedValue = Rc::new(reference.base());
            assert!(ValueTypes::reference_equals(&value, &concrete));
            assert!(ValueTypes::reference_equals(&value, &interface));
            assert!(ValueTypes::reference_equals(&value, &base));
            let cast = ValueTypes::try_cast(&value, ValueType::of::<Rc<$contract>>()).unwrap();
            assert!(ValueTypes::reference_equals(&cast, &concrete));
            drop(model);
            drop(concrete);
            drop(interface);
            drop(base);
            drop(cast);
            drop(value);
            assert!(weak.upgrade().is_some());
            drop(reference);
            assert!(weak.upgrade().is_none());
        }};
    }
    check!(
        ToolDock,
        ToolDockReference,
        dyn ferrodock_model::controls::IToolDock
    );
    check!(
        ProportionalDock,
        ProportionalDockReference,
        dyn ferrodock_model::controls::IProportionalDock
    );
    check!(
        StackDock,
        StackDockReference,
        dyn ferrodock_model::controls::IStackDock
    );
    check!(
        GridDock,
        GridDockReference,
        dyn ferrodock_model::controls::IGridDock
    );
    check!(
        DockDock,
        DockDockReference,
        dyn ferrodock_model::controls::IDockDock
    );
    check!(
        WrapDock,
        WrapDockReference,
        dyn ferrodock_model::controls::IWrapDock
    );
    check!(
        UniformGridDock,
        UniformGridDockReference,
        dyn ferrodock_model::controls::IUniformGridDock
    );
    check!(
        SplitViewDock,
        SplitViewDockReference,
        dyn ferrodock_model::controls::ISplitViewDock
    );
    check!(
        ProportionalDockSplitter,
        ProportionalDockSplitterReference,
        dyn ferrodock_model::controls::IProportionalDockSplitter
    );
    check!(
        GridDockSplitter,
        GridDockSplitterReference,
        dyn ferrodock_model::controls::IGridDockSplitter
    );
}
#[test]
fn new_layout_constructors_keep_the_original_defaults_and_inherited_commands() {
    register_types();
    let tools_reference = ToolDockReference::create();
    let tools = tools_reference.model();
    assert_eq!(tools.alignment(), Alignment::Unset);
    assert!(tools.auto_hide());
    assert_eq!(tools.grip_mode(), GripMode::Visible);
    assert!(!tools.is_expanded());
    assert_eq!(
        ProportionalDockReference::create().model().orientation(),
        Orientation::Horizontal
    );
    let stack = StackDockReference::create().model();
    assert_eq!(stack.orientation(), Orientation::Horizontal);
    assert_eq!(stack.spacing(), 0.0);
    let grid = GridDockReference::create().model();
    assert!(grid.column_definitions().is_none());
    assert!(grid.row_definitions().is_none());
    assert!(DockDockReference::create().model().last_child_fill());
    assert_eq!(
        WrapDockReference::create().model().orientation(),
        Orientation::Horizontal
    );
    let uniform = UniformGridDockReference::create().model();
    assert_eq!((uniform.rows(), uniform.columns()), (0, 0));
    let split = SplitViewDockReference::create().model();
    assert_eq!(
        (split.compact_pane_length(), split.open_pane_length()),
        (48.0, 320.0)
    );
    assert_eq!(split.display_mode(), SplitViewDisplayMode::Overlay);
    assert_eq!(split.pane_placement(), SplitViewPanePlacement::Left);
    assert!(!split.is_pane_open());
    assert!(!split.use_light_dismiss_overlay_mode());
    assert!(split.pane_dockable().is_none());
    assert!(split.content_dockable().is_none());
    let splitter = ProportionalDockSplitterReference::create().model();
    assert!(splitter.can_resize());
    assert!(!splitter.resize_preview());
    assert_eq!(
        GridDockSplitterReference::create()
            .model()
            .resize_direction(),
        GridResizeDirection::Columns
    );
    let reference = tools_reference;
    let getter = DockBaseReference::MARKUP
        .find_property("Close")
        .unwrap()
        .get
        .unwrap();
    let command = getter(&[Some(Rc::new(reference.base()))]).unwrap().unwrap();
    let command = command
        .downcast_ref::<Rc<dyn ferroui_reactiveui::ICommand>>()
        .unwrap();
    assert!(Rc::ptr_eq(command, &tools.close()));
}

#[test]
fn factory_and_window_constructors_keep_original_defaults_and_erased_reference_identity() {
    register_types();
    let factory = FactoryReference::create();
    let model = factory.model();
    let actual = model.clone().boxed_reference();
    assert!(actual
        .downcast_ref::<Rc<ferrodock_model_reactiveui::Factory>>()
        .is_some());
    assert!(!model.hide_tools_on_close());
    assert!(!model.hide_documents_on_close());
    assert!(ferrodock_model_ferro::FactoryBaseReference::MARKUP
        .constructors
        .is_empty());
    let value: BoxedValue = Rc::new(factory.clone());
    let erased: BoxedValue = Rc::new(model);
    let projected = ValueTypes::try_cast(&erased, ValueType::of::<FactoryReference>()).unwrap();
    assert!(ValueTypes::reference_equals(&value, &projected));
    let window = DockWindowReference::create();
    let model = window.model();
    let actual = model.clone().boxed_reference();
    assert!(actual
        .downcast_ref::<Rc<ferrodock_model_reactiveui::core::DockWindow>>()
        .is_some());
    assert_eq!(model.id(), "IDockWindow");
    assert_eq!(model.title(), "IDockWindow");
    assert_eq!(
        (model.x(), model.y(), model.width(), model.height()),
        (0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(model.window_state(), DockWindowState::Normal);
    assert_eq!(model.owner_mode(), DockWindowOwnerMode::Default);
    assert!(!model.topmost());
    assert!(!model.is_modal());
    assert!(model.show_in_taskbar().is_none());
    assert!(model.layout().is_none());
    assert!(model.host().is_none());
    assert!(model.factory().is_none());
    assert!(model.owner().is_none());
    assert!(model.parent_window().is_none());
    let value: BoxedValue = Rc::new(window);
    let erased: BoxedValue = Rc::new(model);
    let projected = ValueTypes::try_cast(&erased, ValueType::of::<DockWindowReference>()).unwrap();
    assert!(ValueTypes::reference_equals(&value, &projected));
}
struct OverriddenWindow {
    base: ferrodock_model_reactiveui::core::DockWindow,
    calls: std::cell::RefCell<Vec<&'static str>>,
}
ferroui_reactiveui::reactive_object!(OverriddenWindow, base);
ferrodock_model_reactiveui::impl_dock_window!(OverriddenWindow);
impl ferrodock_model_reactiveui::core::DockWindowImpl for OverriddenWindow {
    fn on_close(this: &Self) -> bool {
        this.calls.borrow_mut().push("close");
        false
    }
    fn on_move_drag_begin(this: &Self) -> bool {
        this.calls.borrow_mut().push("begin");
        false
    }
    fn on_move_drag(this: &Self) {
        this.calls.borrow_mut().push("drag");
    }
    fn on_move_drag_end(this: &Self) {
        this.calls.borrow_mut().push("end");
    }
}
#[test]
fn window_class_view_keeps_custom_enclosing_type_and_virtual_callbacks_and_releases_it() {
    register_types();
    let model = Rc::new_cyclic(|weak: &std::rc::Weak<OverriddenWindow>| OverriddenWindow {
        base: ferrodock_model_reactiveui::core::DockWindow::construct(weak.clone()),
        calls: std::cell::RefCell::new(Vec::new()),
    });
    let weak = Rc::downgrade(&model);
    let view = DockWindowReference::new(model.clone());
    let actual = view.model().boxed_reference();
    assert!(Rc::ptr_eq(
        actual.downcast_ref::<Rc<OverriddenWindow>>().unwrap(),
        &model
    ));
    drop(actual);
    let value: BoxedValue = Rc::new(view);
    let invoke = |name: &str| {
        (DockWindowReference::MARKUP
            .find_methods(name)
            .next()
            .unwrap()
            .invoke)(&[Some(value.clone())])
        .unwrap()
    };
    assert_eq!(
        invoke("OnClose").unwrap().downcast_ref::<bool>(),
        Some(&false)
    );
    assert_eq!(
        invoke("OnMoveDragBegin").unwrap().downcast_ref::<bool>(),
        Some(&false)
    );
    invoke("OnMoveDrag");
    invoke("OnMoveDragEnd");
    assert_eq!(&*model.calls.borrow(), &["close", "begin", "drag", "end"]);
    drop(model);
    assert!(weak.upgrade().is_some());
    drop(value);
    assert!(weak.upgrade().is_none());
}

#[test]
fn document_factory_interface_and_class_property_retain_same_source_delegate_without_ui_controls() {
    use ferrodock_model::controls::{DocumentFactory, IDocumentDockFactory};
    use ferrodock_model_ferro::DocumentFactoryReference;
    use ferroui_base::metadata::from_markup_value;
    register_types();
    let reference = DocumentDockReference::create();
    let value: BoxedValue = Rc::new(reference.clone());
    let contract =
        ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDocumentDockFactory>>()).unwrap();
    assert!(ValueTypes::reference_equals(&value, &contract));
    let original: DocumentFactory = Rc::new(|| Some(Document::new()));
    let p = DocumentDockReference::MARKUP
        .find_property("DocumentFactory")
        .unwrap();
    (p.set.unwrap())(&[
        Some(value.clone()),
        Some(Rc::new(DocumentFactoryReference::new(original.clone()))),
    ])
    .unwrap();
    assert!(Rc::ptr_eq(
        &reference.factory_contract().document_factory().unwrap(),
        &original
    ));
    let exposed = from_markup_value::<DocumentFactoryReference>(
        &(p.get.unwrap())(&[Some(value.clone())]).unwrap(),
    )
    .unwrap();
    assert!(Rc::ptr_eq(&exposed.model(), &original));
    (p.set.unwrap())(&[Some(value), None]).unwrap();
    assert!(reference.factory_contract().document_factory().is_none());
}

#[test]
fn sdk_base_properties_preserve_original_restrictions_and_selector_values() {
    register_types();
    let document = Document::new();
    let view = DockableBaseReference::new(document.clone());
    let metadata = DockableBaseReference::MARKUP;
    let instance: BoxedValue = Rc::new(view);
    for (name, value) in [
        (
            "AllowedDockOperations",
            DockOperationMask::from_bits_retain(i32::MIN | 1),
        ),
        (
            "AllowedDropOperations",
            DockOperationMask::from_bits_retain(-1),
        ),
    ] {
        let property = metadata.find_property(name).unwrap();
        property.set.unwrap()(&[Some(instance.clone()), Some(Rc::new(value))]).unwrap();
        let result = property.get.unwrap()(&[Some(instance.clone())])
            .unwrap()
            .unwrap();
        assert_eq!(*result.downcast_ref::<DockOperationMask>().unwrap(), value);
    }
    assert_eq!(document.allowed_dock_operations().bits(), i32::MIN | 1);
    assert_eq!(document.allowed_drop_operations().bits(), -1);
    let visible = metadata.find_property("ShowInSelector").unwrap();
    assert!(document.show_in_selector());
    visible.set.unwrap()(&[Some(instance.clone()), Some(Rc::new(false))]).unwrap();
    assert!(!document.show_in_selector());
    let title = metadata.find_property("SelectorTitle").unwrap();
    title.set.unwrap()(&[
        Some(instance.clone()),
        Some(Rc::new(Some("Selector".to_owned()))),
    ])
    .unwrap();
    assert_eq!(document.selector_title().as_deref(), Some("Selector"));
    title.set.unwrap()(&[Some(instance.clone()), None]).unwrap();
    assert!(document.selector_title().is_none());
    assert_eq!(
        visible.get.unwrap()(&[Some(instance)])
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&false)
    );
}
