// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source layout classes exported by an independent model metadata library.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_reactiveui::{controls::*, Factory};
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
#[test]
fn every_original_layout_and_splitter_root_uses_the_actual_concrete_model_and_all_source_properties(
) {
    run(|| {
        macro_rules! model {
            ($name:ident, $reference:ident) => {{
                let value = load(concat!("Reactive", stringify!($name), "Root"));
                let model = value.downcast_ref::<$reference>().unwrap().model();
                let original = model.boxed_reference();
                let model = original.downcast_ref::<Rc<$name>>().unwrap().clone();
                assert_eq!(model.title(), concat!("Source ", stringify!($name)));
                model
            }};
        }
        let tools = model!(ToolDock, ToolDockReference);
        assert_eq!(tools.alignment(), Alignment::Right);
        assert!(tools.is_expanded());
        assert!(!tools.auto_hide());
        assert_eq!(tools.grip_mode(), GripMode::AutoHide);
        assert_eq!(
            model!(ProportionalDock, ProportionalDockReference).orientation(),
            Orientation::Vertical
        );
        let stack = model!(StackDock, StackDockReference);
        assert_eq!(stack.orientation(), Orientation::Vertical);
        assert_eq!(stack.spacing(), 8.5);
        let grid = model!(GridDock, GridDockReference);
        assert_eq!(grid.column_definitions().as_deref(), Some("2*,Auto,*"));
        assert_eq!(grid.row_definitions().as_deref(), Some("*,3*"));
        assert!(!model!(DockDock, DockDockReference).last_child_fill());
        assert_eq!(
            model!(WrapDock, WrapDockReference).orientation(),
            Orientation::Vertical
        );
        let uniform = model!(UniformGridDock, UniformGridDockReference);
        assert_eq!((uniform.rows(), uniform.columns()), (2, 3));
        let split = model!(SplitViewDock, SplitViewDockReference);
        assert_eq!(split.compact_pane_length(), 56.0);
        assert_eq!(split.open_pane_length(), 400.0);
        assert!(split.is_pane_open());
        assert!(split.use_light_dismiss_overlay_mode());
        assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactOverlay);
        assert_eq!(split.pane_placement(), SplitViewPanePlacement::Bottom);
        assert_eq!(split.pane_dockable().unwrap().title(), "Pane");
        assert_eq!(split.content_dockable().unwrap().title(), "Content");
        let splitter = model!(ProportionalDockSplitter, ProportionalDockSplitterReference);
        assert!(!splitter.can_resize());
        assert!(splitter.resize_preview());
        assert_eq!(
            model!(GridDockSplitter, GridDockSplitterReference).resize_direction(),
            GridResizeDirection::Rows
        );
    });
}
#[test]
fn class_metadata_preserves_original_inheritance_member_surface_and_shared_interface_identity() {
    run(|| {
        macro_rules! check {
   ($reference:ident,$contract:ty,$base:ident,$($property:literal),*) => {{
    let reference=$reference::create();let value:BoxedValue=Rc::new(reference.clone());
    let base:BoxedValue=Rc::new(reference.base());let model:BoxedValue=Rc::new(reference.model());
    assert!(ValueTypes::reference_equals(&value,&base));assert!(ValueTypes::reference_equals(&value,&model));
    let interface=ValueTypes::try_cast(&value,ValueType::of::<Rc<$contract>>()).unwrap();assert!(ValueTypes::reference_equals(&value,&interface));
    assert!(std::ptr::eq($reference::MARKUP.base_type().unwrap(),$base::MARKUP));
    assert_eq!($reference::MARKUP.properties.iter().map(|p|p.name).collect::<Vec<_>>(),vec![$($property),*]);
    assert_eq!($reference::MARKUP.constructors.len(),1);
   }};
  }
        check!(
            ToolDockReference,
            dyn IToolDock,
            DockBaseReference,
            "Alignment",
            "IsExpanded",
            "AutoHide",
            "GripMode"
        );
        check!(
            ProportionalDockReference,
            dyn IProportionalDock,
            DockBaseReference,
            "Orientation"
        );
        check!(
            StackDockReference,
            dyn IStackDock,
            DockBaseReference,
            "Orientation",
            "Spacing"
        );
        check!(
            GridDockReference,
            dyn IGridDock,
            DockBaseReference,
            "ColumnDefinitions",
            "RowDefinitions"
        );
        check!(
            DockDockReference,
            dyn IDockDock,
            DockBaseReference,
            "LastChildFill"
        );
        check!(
            WrapDockReference,
            dyn IWrapDock,
            DockBaseReference,
            "Orientation"
        );
        check!(
            UniformGridDockReference,
            dyn IUniformGridDock,
            DockBaseReference,
            "Rows",
            "Columns"
        );
        check!(
            SplitViewDockReference,
            dyn ISplitViewDock,
            DockBaseReference,
            "CompactPaneLength",
            "DisplayMode",
            "IsPaneOpen",
            "OpenPaneLength",
            "PanePlacement",
            "UseLightDismissOverlayMode",
            "PaneDockable",
            "ContentDockable"
        );
        check!(
            ProportionalDockSplitterReference,
            dyn IProportionalDockSplitter,
            DockableBaseReference,
            "CanResize",
            "ResizePreview"
        );
        check!(
            GridDockSplitterReference,
            dyn IGridDockSplitter,
            DockableBaseReference,
            "ResizeDirection"
        );
    });
}
#[test]
fn loaded_tool_dock_add_tool_method_uses_original_factory_ownership_activation_and_focus() {
    run(|| {
        let root = RootDock::new();
        let layout = load("ReactiveProportionalDockRoot");
        let layout = layout
            .downcast_ref::<ProportionalDockReference>()
            .unwrap()
            .model();
        let tools = load("ReactiveToolDockRoot");
        let reference = tools.downcast_ref::<ToolDockReference>().unwrap();
        let model = reference.model();
        let factory = Factory::new();
        factory.add_dockable(Some(root.clone()), Some(layout.clone()));
        factory.add_dockable(Some(layout), Some(model.clone()));
        factory.init_layout(root.clone());
        let tool = load("ReactiveToolRoot");
        let source = tool.downcast_ref::<ToolReference>().unwrap().model();
        (ToolDockReference::MARKUP
            .find_methods("AddTool")
            .next()
            .unwrap()
            .invoke)(&[Some(tools.clone()), Some(tool)])
        .unwrap();
        assert_eq!(model.visible_dockables().unwrap().count(), 1);
        assert!(Rc::ptr_eq(&model.active_dockable().unwrap(), &source));
        assert!(Rc::ptr_eq(&root.focused_dockable().unwrap(), &source));
        assert!(Rc::ptr_eq(
            &source.owner().unwrap(),
            &(model as Rc<dyn IDockable>)
        ));
    });
}
#[test]
fn tool_dock_compiled_bindings_follow_actual_enum_and_boolean_edits_and_original_notifications() {
    run(|| {
        let value = load("ReactiveToolDockRoot");
        let model = value.downcast_ref::<ToolDockReference>().unwrap().model();
        let panel = load("ReactiveLayoutBindings");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value));
        Dispatcher::ui_thread().run_jobs(None);
        let title = panel.children().get(0).cast::<TextBlock>().unwrap();
        let alignment = panel.children().get(1).cast::<TextBox>().unwrap();
        let expanded = panel.children().get(2).cast::<CheckBox>().unwrap();
        let auto_hide = panel.children().get(3).cast::<CheckBox>().unwrap();
        assert_eq!(title.text().as_deref(), Some("Source ToolDock"));
        assert_eq!(alignment.text().as_deref(), Some("Right"));
        alignment.set_text(Some("Left"));
        expanded.set_is_checked(Some(false));
        auto_hide.set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(model.alignment(), Alignment::Left);
        assert!(!model.is_expanded());
        assert!(model.auto_hide());
        model.set_title("Engine tools".into());
        model.set_alignment(Alignment::Bottom);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(title.text().as_deref(), Some("Engine tools"));
        assert_eq!(alignment.text().as_deref(), Some("Bottom"));
    });
}
#[test]
fn split_view_methods_properties_and_pane_references_stay_on_the_original_model() {
    run(|| {
        let value = load("ReactiveSplitViewDockRoot");
        let reference = value.downcast_ref::<SplitViewDockReference>().unwrap();
        let model = reference.model();
        let panel = load("ReactiveSplitViewBindings");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value.clone()));
        Dispatcher::ui_thread().run_jobs(None);
        let open = panel.children().get(0).cast::<CheckBox>().unwrap();
        let length = panel.children().get(1).cast::<Slider>().unwrap();
        let pane = panel.children().get(2).cast::<ContentControl>().unwrap();
        for (method, expected) in [
            ("ClosePane", false),
            ("OpenPane", true),
            ("TogglePane", false),
        ] {
            (SplitViewDockReference::MARKUP
                .find_methods(method)
                .next()
                .unwrap()
                .invoke)(&[Some(value.clone())])
            .unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(model.is_pane_open(), expected);
            assert_eq!(open.is_checked(), Some(expected));
        }
        length.set_range_value(200.0);
        open.set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(model.open_pane_length(), 200.0);
        assert!(model.is_pane_open());
        let pane_value = pane.content().unwrap();
        let original: BoxedValue = Rc::new(model.pane_dockable().unwrap());
        assert!(ValueTypes::reference_equals(&pane_value, &original));
        let setter = SplitViewDockReference::MARKUP
            .find_property("PaneDockable")
            .unwrap()
            .set
            .unwrap();
        setter(&[Some(value.clone()), None]).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(model.pane_dockable().is_none());
        assert!(pane.content().is_none());
    });
}
#[test]
fn original_model_enum_assembly_resolves_typed_layout_values_through_compiled_markup() {
    run(|| {
        let panel = load("CoreModelEnumAssembly");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<SplitViewDisplayMode>(),
            Some(&SplitViewDisplayMode::CompactOverlay)
        );
        assert_eq!(
            panel
                .children()
                .get(1)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<SplitViewPanePlacement>(),
            Some(&SplitViewPanePlacement::Bottom)
        );
        assert_eq!(
            panel
                .children()
                .get(2)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<Orientation>(),
            Some(&Orientation::Vertical)
        );
    });
}
#[test]
fn runtime_emitter_exports_all_ten_public_layout_class_handles_once() {
    run(|| {
        let documents = [
            ("ToolDock.xaml", include_str!("ReactiveToolDockRoot.xaml")),
            (
                "ProportionalDock.xaml",
                include_str!("ReactiveProportionalDockRoot.xaml"),
            ),
            ("StackDock.xaml", include_str!("ReactiveStackDockRoot.xaml")),
            ("GridDock.xaml", include_str!("ReactiveGridDockRoot.xaml")),
            ("DockDock.xaml", include_str!("ReactiveDockDockRoot.xaml")),
            ("WrapDock.xaml", include_str!("ReactiveWrapDockRoot.xaml")),
            (
                "UniformGridDock.xaml",
                include_str!("ReactiveUniformGridDockRoot.xaml"),
            ),
            (
                "SplitViewDock.xaml",
                include_str!("ReactiveSplitViewDockRoot.xaml"),
            ),
            (
                "ProportionalDockSplitter.xaml",
                include_str!("ReactiveProportionalDockSplitterRoot.xaml"),
            ),
            (
                "GridDockSplitter.xaml",
                include_str!("ReactiveGridDockSplitterRoot.xaml"),
            ),
        ];
        let results = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        for (result, (name, _)) in results.iter().zip(documents) {
            let class = name.strip_suffix(".xaml").unwrap();
            let source = result.source.as_ref().unwrap();
            assert!(source.contains(&format!("Result<::ferrodock_model_reactiveui_ferro::reactive_metadata::{class}Reference,")),"{source}");
            assert_eq!(
                source
                    .matches(&format!("::{class}Reference::__markup_new_0()"))
                    .count(),
                1
            );
            assert!(!source.contains("as ::std::rc::Rc<dyn ::ferrodock_ferro::IDockableMetadata>"));
        }
    });
}

#[test]
fn shared_foreign_interface_assignments_use_real_registered_casts_and_native_traits_remain_distinct(
) {
    use crate::carrier_model::ContractConsumer;
    use ferroui_base::metadata::MarkupType;
    use ferroui_markup_xaml_loader::rust_emitter::{
        emit_types::{EmitTypes, TypeKey},
        runtime_types::RuntimeEmitTypes,
    };
    run(|| {
        static TEST_ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
            ferroui_base::metadata::MarkupAssembly {
                name: "FerroDock.Markup.Tests",
                crate_name: "ferrodock_markup_tests",
                xmlns_definitions: &[],
                xmlns_prefixes: &[],
                metadata: &[],
            };
        ferroui_base::metadata::MarkupAssembly::register(&TEST_ASSEMBLY);
        MarkupType::register(ContractConsumer::MARKUP);
        ValueTypes::register_reference::<ContractConsumer>();
        MarkupType::register_rust_paths(&[(
            ContractConsumer::MARKUP,
            "ferrodock_markup_tests::carrier_model::ContractConsumer",
            false,
        )]);
        MarkupType::register(<dyn crate::carrier_model::NativeContract as MarkupTyped>::MARKUP);
        MarkupType::register(crate::carrier_model::MultiHandleValue::MARKUP);
        MarkupType::register_rust_paths(&[
            (
                <dyn crate::carrier_model::NativeContract as MarkupTyped>::MARKUP,
                "ferrodock_markup_tests::carrier_model::NativeContract",
                true,
            ),
            (
                crate::carrier_model::MultiHandleValue::MARKUP,
                "ferrodock_markup_tests::carrier_model::MultiHandleValue",
                false,
            ),
        ]);
        let types = RuntimeEmitTypes;
        let native = types
            .markup_by_handle(TypeKey::Id(std::any::TypeId::of::<
                Rc<dyn ferroui_base::styling::SetterBase>,
            >()))
            .unwrap();
        assert!(native.handle_is_own_trait());
        let multi = types
            .markup_by_handle(TypeKey::Id(std::any::TypeId::of::<
                Rc<crate::carrier_model::MultiHandleValue>,
            >()))
            .unwrap();
        assert!(!multi.handle_is_shared());
        assert!(multi.value_is_shared());
        assert_ne!(multi.handle(), multi.value());
        let multi_value = load("NativeContractConsumerRoot");
        let multi_value = multi_value.downcast_ref::<Rc<ContractConsumer>>().unwrap();
        assert_eq!(
            multi_value.native().source_policy().can_close(),
            Some(false)
        );
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[(
                "NativeContracts.xaml",
                include_str!("NativeContractConsumerRoot.xaml"),
            )],
            Some("ferres://ferrodock_markup_tests/"),
            &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        assert!(emitted[0].source.as_ref().unwrap().contains(
            " as ::std::rc::Rc<dyn ::ferrodock_markup_tests::carrier_model::NativeContract>"
        ));

        let foreign = types
            .markup_by_handle(TypeKey::Id(std::any::TypeId::of::<
                Rc<dyn IDockCapabilityPolicy>,
            >()))
            .unwrap();
        assert!(!foreign.handle_is_own_trait());
        let source = types
            .markup_by_handle(TypeKey::Id(std::any::TypeId::of::<
                Rc<DockCapabilityOverrides>,
            >()))
            .unwrap();
        assert!(source.handle_is_shared());
        assert_eq!(source.value(), source.handle());
        let value = load("ContractConsumerRoot");
        let value = value.downcast_ref::<Rc<ContractConsumer>>().unwrap();
        assert_eq!(value.policy().can_close(), Some(false));
        assert_eq!(value.optional().unwrap().can_float(), Some(false));
        let results = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("Contracts.xaml", include_str!("ContractConsumerRoot.xaml"))],
            Some("ferres://ferrodock_markup_tests/"),
            &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        let source = results[0].source.as_ref().unwrap();
        assert!(source.contains("rt::cast("));
        assert!(!source
            .contains(" as ::std::rc::Rc<dyn ::ferrodock_ferro::IDockCapabilityPolicyMetadata>"));
    });
}
