//! Independent compiled Core consumers preserve actual engine objects and UI projections.
use crate::markup_extension_tests::run;
use ferrodock_model::core::*;
use ferrodock_model_ferro::*;
use ferroui_base::{
    metadata::{from_markup_value, MarkupAssembly, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{Button, CheckBox, ContentControl, Control, StackPanel, TextBlock, TextBox};
use std::rc::Rc;

fn load(name: &str) -> Ref<StackPanel> {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
    .downcast_ref::<Ref<StackPanel>>()
    .unwrap()
    .clone()
}

#[test]
fn compiled_core_dock_bindings_observe_original_list_and_execute_original_navigation_command() {
    run(|| {
        let factory = ferrodock_model_reactiveui::Factory::new();
        let dock: Rc<dyn IDock> = factory.create_document_dock();
        dock.set_title("Core dock".into());
        let first: Rc<dyn IDockable> = factory.create_document();
        first.set_title("First".into());
        let next: Rc<dyn IDockable> = factory.create_document();
        next.set_title("Next".into());
        let list = factory.create_list(vec![Some(first.clone())]);
        dock.set_visible_dockables(Some(list.clone()));
        dock.set_active_dockable(Some(first.clone()));
        dock.set_default_dockable(Some(next.clone()));
        let panel = load("CoreDockBindings");
        panel.set_data_context(Some(Rc::new(dock.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        let text = |index| {
            panel
                .children()
                .get(index)
                .cast::<TextBlock>()
                .unwrap()
                .text()
        };
        assert_eq!(text(0).as_deref(), Some("Core dock"));
        assert_eq!(text(1).as_deref(), Some("First"));
        assert_eq!(text(2).as_deref(), Some("1"));
        first.set_title("Updated first".into());
        list.add(Some(next.clone()));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(text(1).as_deref(), Some("Updated first"));
        assert_eq!(text(2).as_deref(), Some("2"));
        panel
            .children()
            .get(3)
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(dock.is_active());
        let button = panel.children().get(4).cast::<Button>().unwrap();
        let parameter = button.command_parameter();
        let actual = from_markup_value::<Rc<dyn IDockable>>(&parameter).unwrap();
        assert!(Rc::ptr_eq(&actual, &next));
        button.command().unwrap().execute(parameter.as_ref());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &next));
        assert_eq!(text(1).as_deref(), Some("Next"));
        assert!(dock.can_go_back());
        panel.set_data_context(None);
    });
}

#[test]
fn compiled_core_dockable_bindings_keep_raw_control_context_and_legacy_descriptor_identity() {
    run(|| {
        for (core, legacy) in [
            (
                <dyn IDockableMetadata>::MARKUP,
                <dyn ferrodock_ferro::IDockableMetadata>::MARKUP,
            ),
            (
                <dyn IDockMetadata>::MARKUP,
                <dyn ferrodock_ferro::IDockMetadata>::MARKUP,
            ),
            (
                <dyn DockableListMetadata>::MARKUP,
                <dyn ferrodock_ferro::DockableListMetadata>::MARKUP,
            ),
        ] {
            assert!(std::ptr::eq(core, legacy));
            assert_eq!(
                MarkupAssembly::of_module(core.module_path).unwrap().name,
                "FerroDock.Model"
            );
        }
        let factory = ferrodock_model_reactiveui::Factory::new();
        let model: Rc<dyn IDockable> = factory.create_document();
        model.set_title("Before".into());
        let control: Ref<Control> = TextBlock::new().upcast();
        let raw = Rc::new(control.clone());
        model.set_context(Some(raw.clone()));
        let boxed: BoxedValue = Rc::new(model.clone());
        let context = <dyn IDockableMetadata>::MARKUP
            .find_property("Context")
            .unwrap();
        let native = (context.get.unwrap())(&[Some(boxed.clone())])
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&native, &(raw.clone() as BoxedValue)));
        let panel = load("CoreDockableBindings");
        panel.set_data_context(Some(boxed.clone()));
        Dispatcher::ui_thread().run_jobs(None);
        let editor = panel.children().get(0).cast::<TextBox>().unwrap();
        assert_eq!(editor.text().as_deref(), Some("Before"));
        editor.set_text(Some("Edited"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(model.title(), "Edited");
        let host = panel.children().get(1).cast::<ContentControl>().unwrap();
        assert_eq!(
            Control::from_boxed(host.content().as_ref().unwrap()),
            Some(control)
        );
        panel.set_data_context(None);
        let replacement: BoxedValue = Rc::new(TextBlock::new().upcast::<Control>());
        (context.set.unwrap())(&[Some(boxed.clone()), Some(replacement.clone())]).unwrap();
        let source = model.context().unwrap().downcast::<Ref<Control>>().unwrap();
        assert!(Rc::ptr_eq(&(source as BoxedValue), &replacement));
        (context.set.unwrap())(&[Some(boxed), None]).unwrap();
        assert!(model.context().is_none());
    });
}

#[test]
fn recycling_model_metadata_drives_the_existing_native_cache_with_shared_controls() {
    run(|| {
        use ferrodock_controls_recycling::{ControlRecycling, IControlRecycling};
        use ferrodock_controls_recycling_model_ferro::IControlRecyclingMetadata;
        let cache = ControlRecycling::new();
        let contract: Rc<dyn IControlRecycling> = ControlRecycling::as_recycling(cache.clone());
        let boxed: BoxedValue = Rc::new(contract);
        let metadata = <dyn IControlRecyclingMetadata>::MARKUP;
        assert_eq!(
            MarkupAssembly::of_module(metadata.module_path)
                .unwrap()
                .name,
            "FerroDock.Controls.Recycling.Model"
        );
        let data: BoxedValue = Rc::new("original cache key".to_owned());
        let control: Ref<Control> = TextBlock::new().upcast();
        let raw: BoxedValue = Rc::new(control.clone());
        (metadata.find_methods("Add").next().unwrap().invoke)(&[
            Some(boxed.clone()),
            Some(data.clone()),
            Some(raw.clone()),
        ])
        .unwrap();
        let cached = cache.try_get_value(Some(&data)).unwrap();
        assert!(Rc::ptr_eq(&cached, &raw));
        let result = (metadata.find_methods("Build").next().unwrap().invoke)(&[
            Some(boxed.clone()),
            Some(data.clone()),
            None,
            None,
        ])
        .unwrap()
        .unwrap();
        assert_eq!(Control::from_boxed(&result), Some(control));
        (metadata.find_methods("Clear").next().unwrap().invoke)(&[Some(boxed)]).unwrap();
        assert!(cache.try_get_value(Some(&data)).is_none());
    });
}

#[test]
fn runtime_compiler_accepts_the_same_core_qualified_interface_consumers() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let documents = [
            ("Dock.xaml", include_str!("CoreDockBindings.xaml")),
            ("Dockable.xaml", include_str!("CoreDockableBindings.xaml")),
        ];
        let result = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(result.len(), documents.len());
        for document in result {
            assert!(document.source.is_ok(), "{:?}", document.source);
        }
        for (index, (_, source)) in documents.iter().enumerate() {
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
                Some(config.clone()),
            )
            .unwrap();
            let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
            let factory = ferrodock_model_reactiveui::Factory::new();
            let model: Rc<dyn IDockable> = if index == 0 {
                factory.create_document_dock()
            } else {
                factory.create_document()
            };
            model.set_title("Runtime Core".into());
            let boxed: BoxedValue = if index == 0 {
                Rc::new(model.clone().as_dock().unwrap())
            } else {
                Rc::new(model.clone())
            };
            panel.set_data_context(Some(boxed));
            Dispatcher::ui_thread().run_jobs(None);
            if index == 0 {
                let text = panel.children().get(0).cast::<TextBlock>().unwrap();
                assert_eq!(text.text().as_deref(), Some("Runtime Core"));
                model.set_title("Live runtime".into());
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(text.text().as_deref(), Some("Live runtime"));
            } else {
                let text = panel.children().get(0).cast::<TextBox>().unwrap();
                assert_eq!(text.text().as_deref(), Some("Runtime Core"));
                text.set_text(Some("Runtime edit"));
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(model.title(), "Runtime edit");
            }
            panel.set_data_context(None);
        }
    });
}
