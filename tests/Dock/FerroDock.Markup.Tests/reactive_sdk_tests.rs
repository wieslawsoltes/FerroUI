// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original model construction through a separately exported backend SDK.
use crate::markup_extension_tests::{context, provider, run};
use ferrodock_markup_extension::LoadExtension;
use ferrodock_model::core::*;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock, Tool},
    Factory,
};
use ferrodock_model_reactiveui_ferro::{reactive_metadata::*, ASSEMBLY};
use ferroui_base::{
    controls::{INameScope, NameScope},
    metadata::{MarkupAssembly, MarkupTyped},
    threading::Dispatcher,
    utilities::Uri,
    BoxedValue, Ref,
};
use ferroui_controls::{CheckBox, StackPanel, TextBlock, TextBox};
use std::rc::Rc;
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(None, &format!("ferres://ferrodock_markup_tests/{name}"))
        .unwrap()
        .unwrap()
}
#[test]
fn four_original_model_constructors_create_real_engine_instances_with_source_properties() {
    run(|| {
        let document = load("ReactiveDocumentRoot.xaml");
        let document = document
            .downcast_ref::<DocumentReference>()
            .unwrap()
            .model();
        let source = document.clone().boxed_reference();
        let actual = source.downcast_ref::<Rc<Document>>().unwrap();
        assert_eq!(actual.title(), "Source document");
        assert!(actual.is_open());
        assert_eq!(actual.mdi_z_index(), 7);
        let tool = load("ReactiveToolRoot.xaml");
        let tool = tool
            .downcast_ref::<ToolReference>()
            .unwrap()
            .model()
            .boxed_reference();
        let tool = tool.downcast_ref::<Rc<Tool>>().unwrap();
        assert_eq!(tool.title(), "Source tool");
        assert!(tool.is_selected());
        let dock = load("ReactiveDocumentDockRoot.xaml");
        let dock = dock
            .downcast_ref::<DocumentDockReference>()
            .unwrap()
            .model()
            .boxed_reference();
        let dock = dock.downcast_ref::<Rc<DocumentDock>>().unwrap();
        assert_eq!(dock.title(), "Source documents");
        assert!(!dock.can_create_document());
        let root = load("ReactiveRootDockRoot.xaml");
        let root = root
            .downcast_ref::<RootDockReference>()
            .unwrap()
            .model()
            .boxed_reference();
        let root = root.downcast_ref::<Rc<RootDock>>().unwrap();
        assert_eq!(root.title(), "Source root");
        assert!(root.is_focusable_root());
    });
}
#[test]
fn original_assembly_and_inheritance_are_exported_without_constructors_on_abstract_bases() {
    run(|| {
        assert_eq!(ASSEMBLY.name, "FerroDock.Model.ReactiveUI");
        assert!(std::ptr::eq(
            MarkupAssembly::find(ASSEMBLY.name).unwrap(),
            &ASSEMBLY
        ));
        for meta in [
            DocumentReference::MARKUP,
            ToolReference::MARKUP,
            DocumentDockReference::MARKUP,
            RootDockReference::MARKUP,
        ] {
            assert_eq!(meta.constructors.len(), 1);
            assert_eq!(meta.namespace(), "FerroDock.Model.ReactiveUI.Controls");
        }
        for meta in [
            ReactiveBaseReference::MARKUP,
            DockableBaseReference::MARKUP,
            DockBaseReference::MARKUP,
        ] {
            assert!(meta.constructors.is_empty());
        }
        assert!(std::ptr::eq(
            DocumentReference::MARKUP.base_type().unwrap(),
            DockableBaseReference::MARKUP
        ));
        assert!(std::ptr::eq(
            RootDockReference::MARKUP.base_type().unwrap(),
            DockBaseReference::MARKUP
        ));
        let wrong_assembly = include_str!("ReactiveDocumentRoot.xaml").replace(
            "assembly=FerroDock.Model.ReactiveUI",
            "assembly=FerroDock.Ferro",
        );
        let result = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("WrongAssembly.xaml", &wrong_assembly)],
            Some("ferres://ferrodock_markup_tests/"),
            &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        assert!(
            result[0].source.is_err(),
            "model leaked into the control assembly"
        );
    });
}
#[test]
fn sdk_loaded_models_share_identity_and_work_with_the_original_factory_engine() {
    run(|| {
        let root = load("ReactiveRootDockRoot.xaml");
        let root = root.downcast_ref::<RootDockReference>().unwrap().model();
        let dock = load("ReactiveDocumentDockRoot.xaml");
        let dock = dock
            .downcast_ref::<DocumentDockReference>()
            .unwrap()
            .model();
        let document = load("ReactiveDocumentRoot.xaml");
        let document = document
            .downcast_ref::<DocumentReference>()
            .unwrap()
            .model();
        let factory = Factory::new();
        factory.add_dockable(Some(root.clone()), Some(dock.clone()));
        factory.add_dockable(Some(dock.clone()), Some(document.clone()));
        factory.init_layout(root.clone());
        assert_eq!(dock.visible_dockables().unwrap().count(), 1);
        assert!(Rc::ptr_eq(
            &document.owner().unwrap(),
            &(dock.clone() as Rc<dyn IDockable>)
        ));
        dock.set_can_close_last_dockable(true);
        factory.close_dockable(Some(document));
        assert_eq!(dock.visible_dockables().unwrap().count(), 0);
    });
}
#[test]
fn sdk_class_view_bindings_retain_live_engine_notifications_and_two_way_state() {
    run(|| {
        let value = load("ReactiveDocumentRoot.xaml");
        let reference = value.downcast_ref::<DocumentReference>().unwrap();
        let document = reference.model();
        let panel = load("ReactiveModelBindings.xaml");
        let panel = panel.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(value.clone()));
        Dispatcher::ui_thread().run_jobs(None);
        let text = panel.children().get(0).cast::<TextBlock>().unwrap();
        let edit = panel.children().get(1).cast::<TextBox>().unwrap();
        let open = panel.children().get(2).cast::<CheckBox>().unwrap();
        assert_eq!(text.text().as_deref(), Some("Source document"));
        document.set_title("Changed engine".into());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(text.text().as_deref(), Some("Changed engine"));
        edit.set_text(Some("Edited"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(document.title(), "Edited");
        open.set_is_checked(Some(false));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!document.as_docking_window_state().unwrap().is_open());
    });
}
#[test]
fn original_load_extension_constructs_fresh_sdk_models_and_completes_the_supplied_scope() {
    run(|| {
        let extension = LoadExtension::with_source(
            Uri::absolute("ferres://ferrodock_markup_tests/ReactiveDocumentRoot.xaml").unwrap(),
        );
        let service = provider(
            Some(context(Some("ferres://ferrodock_markup_tests/"))),
            None,
        );
        let first = extension.provide_value(&service).unwrap().unwrap();
        let second = extension.provide_value(&service).unwrap().unwrap();
        let first = first.downcast_ref::<DocumentReference>().unwrap();
        let second = second.downcast_ref::<DocumentReference>().unwrap();
        assert!(!Rc::ptr_eq(&first.model(), &second.model()));
        let scope = Rc::new(NameScope::new());
        let result: DocumentReference = crate::compiled_xaml::build_reactivedocumentroot_xaml(
            Some(provider(None, Some(scope.clone()))),
        )
        .unwrap();
        assert!(scope.is_completed());
        assert_eq!(result.model().title(), "Source document");
    });
}

#[test]
fn runtime_emitter_uses_the_public_sdk_export_for_all_four_model_roots() {
    run(|| {
        let results = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[
                ("Document.xaml", include_str!("ReactiveDocumentRoot.xaml")),
                ("Tool.xaml", include_str!("ReactiveToolRoot.xaml")),
                (
                    "DocumentDock.xaml",
                    include_str!("ReactiveDocumentDockRoot.xaml"),
                ),
                ("RootDock.xaml", include_str!("ReactiveRootDockRoot.xaml")),
            ],
            Some("ferres://ferrodock_markup_tests/"),
            &ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default(),
            &[],
        );
        for (result, name) in results.iter().zip([
            "DocumentReference",
            "ToolReference",
            "DocumentDockReference",
            "RootDockReference",
        ]) {
            let source = result.source.as_ref().unwrap();
            assert!(
                source.contains(&format!(
                    "Result<::ferrodock_model_reactiveui_ferro::reactive_metadata::{name},"
                )),
                "{source}"
            );
            assert_eq!(source.matches("::__markup_new_0()").count(), 1);
        }
    });
}
