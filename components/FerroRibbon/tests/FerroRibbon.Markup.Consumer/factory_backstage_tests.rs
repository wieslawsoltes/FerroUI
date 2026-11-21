use crate::factory_backstage_consumer::*;
use ferroribbon::{
    compat::{DataTemplateSelectorExt, DataTemplateSelectorImpl},
    controls::RibbonBackstage,
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::{
    commands::{RibbonCommandCatalog, RibbonCommandDescriptor, RibbonRelayCommand},
    model::*,
};
use ferroui_base::{
    Rect, Size, TypeInfo,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::{Border, Control, StackPanel, TextBlock, Window};
use std::{any::Any, cell::Cell, rc::Rc};
#[test]
fn independent_factory_backstage_compiled_runtime_selector_binding_identity_commands_and_disposal()
{
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        register();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::factory_backstage_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[ConsumerBackstageFactory::TYPE]);
        MarkupType::register_all(&[crate::text_input_consumer::InputViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::text_input_consumer::InputViewModel>();
        let probe = ConsumerTemplateSelector::new();
        let boxed = Rc::new(probe.clone()) as ferroui_base::BoxedValue;
        let raw = probe.clone() as ferroui_base::BoxedValue;
        assert!(
            ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn DataTemplateSelectorImpl>>())
                .is_some(),
            "handle cast"
        );
        assert!(
            ValueTypes::try_cast(&raw, ValueType::of::<Rc<dyn DataTemplateSelectorImpl>>())
                .is_some(),
            "object cast"
        );
        let target =
            (<ferroribbon::compat::DataTemplateSelector as MarkupTyped>::MARKUP.handles[0])();
        assert_eq!(
            target,
            ValueType::of::<Rc<dyn DataTemplateSelectorImpl>>(),
            "base markup first handle"
        );
        assert!(
            ValueTypes::is_assignable(ValueType::of::<ConsumerTemplateSelector>(), target),
            "object assignability"
        );
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("FactoryBackstage.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/FactoryBackstage.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |key: &str| {
                panel
                    .resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap()
            };
            let factory = ValueTypes::as_object(&*resource("Factory"))
                .unwrap()
                .cast::<ConsumerBackstageFactory>()
                .unwrap();
            let s = ValueTypes::try_convert(
                Some(&resource("Selector")),
                ValueType::of::<Rc<ConsumerTemplateSelector>>(),
            )
            .flatten()
            .unwrap();
            let selector = s
                .downcast_ref::<Rc<ConsumerTemplateSelector>>()
                .unwrap()
                .clone();
            assert!(
                factory
                    .content_template_selector()
                    .is_some_and(|s| Rc::ptr_eq(
                        &s,
                        &(selector.clone() as Rc<dyn DataTemplateSelectorImpl>)
                    ))
            );
            let default = ValueTypes::try_cast(
                &resource("DefaultSelector"),
                ValueType::of::<Rc<dyn DataTemplateSelectorImpl>>(),
            )
            .unwrap();
            assert!(
                default
                    .downcast_ref::<Rc<dyn DataTemplateSelectorImpl>>()
                    .unwrap()
                    .select_template(None)
                    .is_none()
            );
            let vm = crate::text_input_consumer::InputViewModel::new();
            let m = RibbonBackstageModel::new();
            m.set_title(Some("Generated".into()));
            let page =
                RibbonBackstageItemModel::new(Some("info".into()), Some("Information".into()));
            page.set_content(Some(RibbonObject::reference(vm.clone())));
            let save = RibbonBackstageItemModel::new(Some("save".into()), Some("Save".into()));
            save.set_command_id(Some("save".into()));
            save.set_command_parameter(Some(RibbonObject::value("payload".to_owned())));
            m.items().add(Some(page.clone()));
            m.items().add(Some(save.clone()));
            let calls = Rc::new(Cell::new(0));
            let out = calls.clone();
            let viewmodel = vm.clone();
            let command = RibbonRelayCommand::new_with_parameter(
                move |p| {
                    out.set(out.get() + 1);
                    viewmodel.set_text(format!(
                        "invoked:{}",
                        p.unwrap().downcast_ref::<String>().unwrap()
                    ));
                },
                None,
            );
            let catalog = RibbonCommandCatalog::new();
            catalog.register(Some(RibbonCommandDescriptor::new(
                Some("save".into()),
                Some("Save".into()),
                Some(command),
            )));
            let owner = ferroribbon::controls::Ribbon::new();
            owner.set_command_catalog(Some(catalog));
            factory.set_ribbon(Some(owner));
            let bindings = RibbonModelBindings::new();
            let b = factory.create_backstage(Some(m.clone()), Some(bindings.clone()));
            assert_eq!(factory.backstages(), 1);
            assert_eq!(factory.items(), 2);
            let made = b.items().get(0).unwrap();
            let original: Rc<dyn Any> = vm.clone();
            let content: Rc<dyn Any> = made.content().unwrap();
            let selected: Rc<dyn Any> = selector.last().unwrap();
            assert!(Rc::ptr_eq(&original, &content) && Rc::ptr_eq(&original, &selected));
            let model = RibbonElementFactory::get_model(&made).unwrap();
            assert!(Rc::ptr_eq(
                &model,
                &(page.clone() as Rc<dyn RibbonNodeModelImpl>)
            ));
            let host = panel.children().get(1).cast::<Border>().unwrap();
            host.set_child(Some(b.clone().upcast()));
            let w = Window::new();
            w.set_width(900.5);
            w.set_height(650.);
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.measure(Size::new(900.5, 650.));
            w.arrange(Rect::new(0., 0., 900.5, 650.));
            w.update_layout();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            let text = b
                .get_visual_descendants()
                .filter_map(|v| v.cast::<TextBlock>())
                .find(|t| t.text().as_deref() == Some("initial"))
                .expect("compiled data template bound to original model");
            assert!(text.is_loaded());
            vm.set_text("from-model".into());
            assert_eq!(text.text().as_deref(), Some("from-model"));
            b.items()
                .get(1)
                .unwrap()
                .navigation_button()
                .unwrap()
                .on_click();
            assert_eq!(calls.get(), 1);
            assert_eq!(text.text().as_deref(), Some("invoked:payload"));
            let declared = panel.children().get(0).cast::<RibbonBackstage>().unwrap();
            assert!(declared.is_loaded() && b.is_loaded());
            assert_eq!(
                declared.items().get(0).unwrap().header().as_deref(),
                Some("Declared page")
            );
            m.items().move_item(0, 1);
            assert_eq!(b.items().get(1), Some(made.clone()));
            assert_eq!(factory.items(), 2);
            page.set_label(Some("Renamed".into()));
            assert_eq!(made.header().as_deref(), Some("Renamed"));
            bindings.dispose();
            page.set_label(Some("Disposed".into()));
            assert_eq!(made.header().as_deref(), Some("Renamed"));
            assert_eq!(b.items().count(), 2);
            selector.clear_last();
            w.close();
            w.set_content(None);
            factory.set_ribbon(None);
        }
    });
}
