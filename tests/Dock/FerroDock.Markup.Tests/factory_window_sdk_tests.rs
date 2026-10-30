// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Independent compiled consumers of the original Factory and DockWindow classes.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_ferro::{DockableObjectMapReference, FactoryBaseReference};
use ferrodock_model_reactiveui_ferro::reactive_metadata::*;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{CheckBox, ContentControl, Slider, StackPanel, TextBlock, TextBox};
use std::rc::Rc;
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
}
fn invoke(
    value: BoxedValue,
    metadata: &'static ferroui_base::metadata::MarkupType,
    name: &str,
    args: &[Option<BoxedValue>],
) -> Option<BoxedValue> {
    let method = metadata
        .find_methods(name)
        .find(|m| m.parameters.len() == args.len())
        .unwrap();
    let mut arguments = vec![Some(value)];
    arguments.extend_from_slice(args);
    (method.invoke)(&arguments).unwrap()
}
#[test]
fn source_window_constructor_exports_every_declared_property_and_method_with_reactive_base_identity(
) {
    run(|| {
        let value = load("ReactiveDockWindowRoot");
        let reference = value.downcast_ref::<DockWindowReference>().unwrap();
        let model = reference.model();
        let actual = model.clone().boxed_reference();
        let actual = actual
            .downcast_ref::<Rc<ferrodock_model_reactiveui::core::DockWindow>>()
            .unwrap();
        assert_eq!(actual.id(), "source-window");
        assert_eq!(actual.title(), "Source window");
        assert_eq!(
            (actual.x(), actual.y(), actual.width(), actual.height()),
            (42.25, -12.0, 800.0, 600.0)
        );
        assert_eq!(actual.window_state(), DockWindowState::Maximized);
        assert!(actual.topmost());
        assert_eq!(actual.owner_mode(), DockWindowOwnerMode::RootWindow);
        assert!(actual.is_modal());
        assert_eq!(actual.show_in_taskbar(), Some(false));
        assert_eq!(actual.layout().unwrap().title(), "Window layout");
        assert!(std::ptr::eq(
            DockWindowReference::MARKUP.base_type().unwrap(),
            ReactiveBaseReference::MARKUP
        ));
        assert_eq!(DockWindowReference::MARKUP.properties.len(), 16);
        assert_eq!(DockWindowReference::MARKUP.methods.len(), 8);
        let base: BoxedValue = Rc::new(reference.base());
        assert!(ValueTypes::reference_equals(&value, &base));
        let interface: BoxedValue = Rc::new(model.clone());
        assert!(ValueTypes::reference_equals(&value, &interface));
        let projected =
            ValueTypes::try_cast(&interface, ValueType::of::<DockWindowReference>()).unwrap();
        assert!(ValueTypes::reference_equals(&value, &projected));
        for method in ["OnClose", "OnMoveDragBegin"] {
            assert_eq!(
                invoke(value.clone(), DockWindowReference::MARKUP, method, &[])
                    .unwrap()
                    .downcast_ref::<bool>(),
                Some(&true)
            );
        }
        for method in ["OnMoveDrag", "OnMoveDragEnd", "Save", "SetActive", "Exit"] {
            assert!(invoke(value.clone(), DockWindowReference::MARKUP, method, &[]).is_none());
        }
    });
}
#[test]
fn factory_constructor_uses_the_original_base_and_creates_every_original_model_through_virtual_dispatch(
) {
    run(|| {
        let value = load("ReactiveFactoryRoot");
        let reference = value.downcast_ref::<FactoryReference>().unwrap();
        let factory = reference.model();
        let actual = factory.clone().boxed_reference();
        assert!(actual
            .downcast_ref::<Rc<ferrodock_model_reactiveui::Factory>>()
            .is_some());
        assert!(factory.hide_tools_on_close());
        assert!(!factory.hide_documents_on_close());
        assert!(std::ptr::eq(
            FactoryReference::MARKUP.base_type().unwrap(),
            FactoryBaseReference::MARKUP
        ));
        assert!(FactoryBaseReference::MARKUP.constructors.is_empty());
        assert_eq!(FactoryReference::MARKUP.properties.len(), 10);
        assert_eq!(FactoryReference::MARKUP.methods.len(), 16);
        let base: BoxedValue = Rc::new(reference.base());
        let interface: BoxedValue = Rc::new(factory.clone());
        assert!(ValueTypes::reference_equals(&value, &base));
        assert!(ValueTypes::reference_equals(&value, &interface));
        assert!(ValueTypes::reference_equals(
            &value,
            &ValueTypes::try_cast(&interface, ValueType::of::<FactoryReference>()).unwrap()
        ));
        macro_rules! created {
            ($name:literal,$trait:ty,$concrete:ty) => {{
                let created = invoke(value.clone(), FactoryReference::MARKUP, $name, &[]).unwrap();
                let model = created.downcast_ref::<Rc<$trait>>().unwrap();
                let original = model.clone().boxed_reference();
                assert!(original.downcast_ref::<Rc<$concrete>>().is_some());
            }};
        }
        created!(
            "CreateRootDock",
            dyn IRootDock,
            ferrodock_model_reactiveui::controls::RootDock
        );
        created!(
            "CreateProportionalDock",
            dyn IProportionalDock,
            ferrodock_model_reactiveui::controls::ProportionalDock
        );
        created!(
            "CreateDockDock",
            dyn IDockDock,
            ferrodock_model_reactiveui::controls::DockDock
        );
        created!(
            "CreateStackDock",
            dyn IStackDock,
            ferrodock_model_reactiveui::controls::StackDock
        );
        created!(
            "CreateGridDock",
            dyn IGridDock,
            ferrodock_model_reactiveui::controls::GridDock
        );
        created!(
            "CreateWrapDock",
            dyn IWrapDock,
            ferrodock_model_reactiveui::controls::WrapDock
        );
        created!(
            "CreateUniformGridDock",
            dyn IUniformGridDock,
            ferrodock_model_reactiveui::controls::UniformGridDock
        );
        created!(
            "CreateProportionalDockSplitter",
            dyn IProportionalDockSplitter,
            ferrodock_model_reactiveui::controls::ProportionalDockSplitter
        );
        created!(
            "CreateGridDockSplitter",
            dyn IGridDockSplitter,
            ferrodock_model_reactiveui::controls::GridDockSplitter
        );
        created!(
            "CreateToolDock",
            dyn IToolDock,
            ferrodock_model_reactiveui::controls::ToolDock
        );
        created!(
            "CreateDocumentDock",
            dyn IDocumentDock,
            ferrodock_model_reactiveui::controls::DocumentDock
        );
        created!(
            "CreateSplitViewDock",
            dyn ISplitViewDock,
            ferrodock_model_reactiveui::controls::SplitViewDock
        );
        created!(
            "CreateDocument",
            dyn IDocument,
            ferrodock_model_reactiveui::controls::Document
        );
        created!(
            "CreateTool",
            dyn ITool,
            ferrodock_model_reactiveui::controls::Tool
        );
        created!(
            "CreateDockWindow",
            dyn IDockWindow,
            ferrodock_model_reactiveui::core::DockWindow
        );
        let root = invoke(
            value.clone(),
            FactoryReference::MARKUP,
            "CreateRootDock",
            &[],
        )
        .unwrap();
        let root = root.downcast_ref::<Rc<dyn IRootDock>>().unwrap();
        let lists = [
            root.left_pinned_dockables().unwrap(),
            root.right_pinned_dockables().unwrap(),
            root.top_pinned_dockables().unwrap(),
            root.bottom_pinned_dockables().unwrap(),
        ];
        for i in 0..4 {
            assert_eq!(lists[i].count(), 0);
            for j in 0..i {
                assert!(!lists[i].ptr_eq(&lists[j]));
            }
        }
        let layout = invoke(value, FactoryReference::MARKUP, "CreateLayout", &[]).unwrap();
        let layout =
            ferroui_base::metadata::from_markup_value::<Option<Rc<dyn IRootDock>>>(&Some(layout))
                .unwrap()
                .unwrap();
        assert!(layout.left_pinned_dockables().is_some());
    });
}
#[test]
fn factory_registry_metadata_retains_original_mutable_containers_and_is_read_only() {
    run(|| {
        let value = load("ReactiveFactoryRoot");
        let reference = value.downcast_ref::<FactoryReference>().unwrap();
        let factory = reference.model();
        for name in [
            "VisibleDockableControls",
            "VisibleRootControls",
            "PinnedDockableControls",
            "PinnedRootControls",
            "TabDockableControls",
            "TabRootControls",
            "ToolControls",
            "DocumentControls",
            "DockControls",
            "HostWindows",
        ] {
            assert!(FactoryReference::MARKUP
                .find_property(name)
                .unwrap()
                .set
                .is_none());
        }
        let getter = FactoryReference::MARKUP
            .find_property("VisibleRootControls")
            .unwrap()
            .get
            .unwrap();
        let first = getter(&[Some(value.clone())]).unwrap().unwrap();
        let second = getter(&[Some(value)]).unwrap().unwrap();
        assert!(ValueTypes::reference_equals(&first, &second));
        let map = first.downcast_ref::<DockableObjectMapReference>().unwrap();
        assert!(Rc::ptr_eq(&map.model(), &factory.visible_root_controls()));
        let item = ferrodock_model_reactiveui::controls::Document::new();
        factory
            .visible_root_controls()
            .borrow_mut()
            .insert(item.clone(), Rc::new("native value".to_owned()));
        assert_eq!(map.model().borrow().len(), 1);
        let count = DockableObjectMapReference::MARKUP
            .find_property("Count")
            .unwrap()
            .get
            .unwrap();
        assert_eq!(
            count(&[Some(first.clone())])
                .unwrap()
                .unwrap()
                .downcast_ref::<i32>(),
            Some(&1)
        );
        factory.visible_root_controls().borrow_mut().clear();
        assert!(map.model().borrow().is_empty());
    });
}
#[test]
fn window_live_bindings_and_factory_two_way_inputs_mutate_the_same_original_objects() {
    run(|| {
        let value = load("ReactiveDockWindowRoot");
        let model = value.downcast_ref::<DockWindowReference>().unwrap().model();
        let panel = load("ReactiveDockWindowBindings");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value));
        Dispatcher::ui_thread().run_jobs(None);
        let text = panel.children().get(0).cast::<TextBlock>().unwrap();
        let edit = panel.children().get(1).cast::<TextBox>().unwrap();
        let width = panel.children().get(2).cast::<Slider>().unwrap();
        let top = panel.children().get(3).cast::<CheckBox>().unwrap();
        let state = panel.children().get(4).cast::<TextBox>().unwrap();
        assert_eq!(text.text().as_deref(), Some("Source window"));
        assert_eq!(state.text().as_deref(), Some("Maximized"));
        edit.set_text(Some("Edited window"));
        width.set_range_value(700.0);
        top.set_is_checked(Some(false));
        state.set_text(Some("Normal"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(model.title(), "Edited window");
        assert_eq!(model.width(), 700.0);
        assert!(!model.topmost());
        assert_eq!(model.window_state(), DockWindowState::Normal);
        model.set_title("Engine window".into());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(text.text().as_deref(), Some("Engine window"));
        let value = load("ReactiveFactoryRoot");
        let model = value.downcast_ref::<FactoryReference>().unwrap().model();
        let panel = load("ReactiveFactoryBindings");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<CheckBox>()
                .unwrap()
                .is_checked(),
            Some(true)
        );
        for index in 2..5 {
            assert_eq!(
                panel
                    .children()
                    .get(index)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("0")
            );
        }
        panel
            .children()
            .get(0)
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(Some(false));
        panel
            .children()
            .get(1)
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!model.hide_tools_on_close());
        assert!(model.hide_documents_on_close());
        let host = ferrodock_ferro::controls::HostWindow::new();
        let host = ferrodock_ferro::controls::HostWindow::as_host_window(host);
        model.host_windows().add(host.clone());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            panel
                .children()
                .get(4)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("1")
        );
        model.host_windows().remove(&host);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            panel
                .children()
                .get(4)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("0")
        );
    });
}
#[test]
fn window_factory_parent_owner_and_layout_references_use_original_objects_owned_by_the_composition()
{
    run(|| {
        let panel = load("ReactiveFactoryWindowGraph");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        let item = |i| {
            panel
                .children()
                .get(i)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
        };
        let factory = item(0);
        let factory = factory.downcast_ref::<FactoryReference>().unwrap().model();
        let parent = item(1);
        let parent = parent
            .downcast_ref::<DockWindowReference>()
            .unwrap()
            .model();
        let owner = item(2);
        let owner = owner.downcast_ref::<RootDockReference>().unwrap().model();
        let child = item(3);
        let child = child.downcast_ref::<DockWindowReference>().unwrap().model();
        assert!(Rc::ptr_eq(&child.factory().unwrap(), &factory));
        assert!(Rc::ptr_eq(&child.parent_window().unwrap(), &parent));
        assert!(Rc::ptr_eq(
            &child.owner().unwrap(),
            &(owner.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(&child.layout().unwrap(), &owner));
        factory.init_dock_window(Some(child.clone()), Some(owner.clone()));
        assert!(Rc::ptr_eq(&child.factory().unwrap(), &factory));
        assert!(owner.window().is_none());
        assert!(Rc::ptr_eq(&owner.factory().unwrap(), &factory));
        assert!(child.host().is_none());
    });
}
#[test]
fn runtime_compiler_uses_original_factory_window_and_abstract_base_public_paths() {
    run(|| {
        let docs = [
            ("Factory.xaml", include_str!("ReactiveFactoryRoot.xaml")),
            ("Window.xaml", include_str!("ReactiveDockWindowRoot.xaml")),
            (
                "Graph.xaml",
                include_str!("ReactiveFactoryWindowGraph.xaml"),
            ),
        ];
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &docs,
            Some("ferres://ferrodock_markup_tests/"),
            &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        assert!(emitted[0].source.as_ref().unwrap().contains(
            "Result<::ferrodock_model_reactiveui_ferro::reactive_metadata::FactoryReference,"
        ));
        assert!(emitted[1].source.as_ref().unwrap().contains(
            "Result<::ferrodock_model_reactiveui_ferro::reactive_metadata::DockWindowReference,"
        ));
        for result in emitted {
            assert!(!result
                .source
                .as_ref()
                .unwrap()
                .contains(" as ::std::rc::Rc<dyn ::ferrodock_ferro::IFactoryMetadata>"));
        }
    });
}

#[test]
fn source_factory_base_metadata_initializes_and_closes_the_original_created_layout() {
    run(|| {
        let value = load("ReactiveFactoryRoot");
        let reference = value.downcast_ref::<FactoryReference>().unwrap();
        let base: BoxedValue = Rc::new(reference.base());
        let root_value = invoke(
            value.clone(),
            FactoryReference::MARKUP,
            "CreateRootDock",
            &[],
        )
        .unwrap();
        let root = root_value
            .downcast_ref::<Rc<dyn IRootDock>>()
            .unwrap()
            .clone();
        let dock_value = invoke(
            value.clone(),
            FactoryReference::MARKUP,
            "CreateDocumentDock",
            &[],
        )
        .unwrap();
        let dock = dock_value
            .downcast_ref::<Rc<dyn IDocumentDock>>()
            .unwrap()
            .clone();
        dock.set_can_close_last_dockable(true);
        dock.set_is_collapsable(false);
        let doc_value = invoke(value, FactoryReference::MARKUP, "CreateDocument", &[]).unwrap();
        let document = doc_value
            .downcast_ref::<Rc<dyn IDocument>>()
            .unwrap()
            .clone();
        invoke(
            base.clone(),
            FactoryBaseReference::MARKUP,
            "AddDockable",
            &[
                Some(Rc::new(root.clone() as Rc<dyn IDock>)),
                Some(Rc::new(dock.clone() as Rc<dyn IDockable>)),
            ],
        );
        invoke(
            base.clone(),
            FactoryBaseReference::MARKUP,
            "AddDockable",
            &[
                Some(Rc::new(dock.clone() as Rc<dyn IDock>)),
                Some(Rc::new(document.clone() as Rc<dyn IDockable>)),
            ],
        );
        invoke(
            base.clone(),
            FactoryBaseReference::MARKUP,
            "InitLayout",
            &[Some(Rc::new(root.clone() as Rc<dyn IDockable>))],
        );
        assert!(Rc::ptr_eq(
            &document.owner().unwrap(),
            &(dock.clone() as Rc<dyn IDockable>)
        ));
        assert_eq!(dock.visible_dockables().unwrap().count(), 1);
        invoke(
            base,
            FactoryBaseReference::MARKUP,
            "CloseDockable",
            &[Some(Rc::new(document as Rc<dyn IDockable>))],
        );
        assert_eq!(dock.visible_dockables().unwrap().count(), 0);
    });
}
