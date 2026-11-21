use crate::{factory_consumer::ConsumerElementFactory, root_model_consumer::RootModelViewModel};
use ferroribbon::controls::{Ribbon, RibbonButton, RibbonItemHelper};
use ferroribbon_core::{commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    Rect, Size,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, Window};
use std::{cell::Cell, rc::Rc};
fn drain() {
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn independent_root_model_compiled_runtime_factory_replacement_lifecycle_and_linked_invocation() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        ValueTypes::register_reference::<RootModelViewModel>();
        MarkupType::register_all(&[RootModelViewModel::MARKUP]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("RootModel.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/RootModel.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            let vm = host
                .resources()
                .try_get_resource(&"ViewModel".into(), None)
                .flatten()
                .unwrap();
            let vm = ValueTypes::try_convert(Some(&vm), ValueType::of::<Rc<RootModelViewModel>>())
                .flatten()
                .unwrap();
            let vm = vm.downcast_ref::<Rc<RootModelViewModel>>().unwrap().clone();
            let factory = r.item_factory().cast::<ConsumerElementFactory>().unwrap();
            assert!(r.model().is_none());
            let declared = r.tabs().get(0).unwrap();
            let m = RibbonModel::new();
            let tab = RibbonTabModel::new(Some("home".into()), Some("Home".into()));
            let group = RibbonGroupModel::new(Some("tools".into()), Some("Tools".into()));
            let item = RibbonButtonModel::new(Some("run".into()), Some("Run".into()));
            let calls = Rc::new(Cell::new(0));
            let out = calls.clone();
            item.set_command(Some(RibbonRelayCommand::new(move || {
                out.set(out.get() + 1)
            })));
            group.items().add(Some(item.clone()));
            tab.groups().add(Some(group));
            m.tabs().add(Some(tab.clone()));
            m.quick_access_items().add(Some(item.clone()));
            vm.set_model(Some(m.clone()));
            assert_eq!(r.tabs().count(), 2);
            assert_eq!(factory.created_count(), 1);
            assert_eq!(factory.created_group_count(), 1);
            assert_eq!(r.tabs().get(0), Some(declared.clone()));
            let generated = r.tabs().get(1).unwrap();
            let original = r.find_item(Some("run".into())).unwrap();
            let get_model = <Ribbon as ferroui_base::StaticType>::TYPE
                .markup()
                .unwrap()
                .methods
                .iter()
                .find(|m| m.name == "GetModel")
                .unwrap();
            assert!(get_model.is_static);
            let association =
                (get_model.invoke)(&[Some(Control::boxed(original.clone()))]).unwrap();
            let association = ValueTypes::try_convert(
                association.as_ref(),
                ValueType::of::<Option<Rc<dyn RibbonNodeModelImpl>>>(),
            )
            .flatten()
            .unwrap();
            let association = association
                .downcast_ref::<Option<Rc<dyn RibbonNodeModelImpl>>>()
                .unwrap()
                .as_ref()
                .unwrap();
            assert!(std::ptr::addr_eq(
                Rc::as_ptr(association),
                Rc::as_ptr(&item)
            ));
            let copy = r.quick_access_tool_bar().unwrap().items().get(0).unwrap();
            assert_eq!(
                RibbonItemHelper::get_source_item(&copy),
                Some(original.clone())
            );
            copy.cast::<RibbonButton>().unwrap().invoke_core();
            assert_eq!(calls.get(), 1);
            let w = Window::new();
            w.set_width(1000.);
            w.set_height(550.);
            w.set_content(Some(Control::boxed(host.clone())));
            w.show();
            w.measure(Size::new(1000., 550.));
            w.arrange(Rect::new(0., 0., 1000., 550.));
            drain();
            assert!(generated.is_loaded());
            w.set_content(None);
            drain();
            tab.set_label(Some("Detached".into()));
            assert_eq!(generated.header().as_deref(), Some("Home"));
            w.set_content(Some(Control::boxed(host.clone())));
            drain();
            assert_eq!(r.tabs().get(1), Some(generated.clone()));
            assert_eq!(generated.header().as_deref(), Some("Detached"));
            item.set_label(Some("Live".into()));
            assert_eq!(
                copy.cast::<RibbonButton>().unwrap().label().as_deref(),
                Some("Live")
            );
            let next = RibbonModel::new();
            next.tabs().add(Some(RibbonTabModel::new(
                Some("next".into()),
                Some("Next".into()),
            )));
            vm.set_model(Some(next.clone()));
            drain();
            assert_eq!(r.tabs().count(), 2);
            assert_eq!(r.tabs().get(0), Some(declared.clone()));
            assert_eq!(r.tabs().get(1).unwrap().header().as_deref(), Some("Next"));
            assert!(r.quick_access_tool_bar().unwrap().items().is_empty());
            tab.set_label(Some("Old model".into()));
            item.set_label(Some("Old item".into()));
            assert_eq!(generated.header().as_deref(), Some("Detached"));
            assert_eq!(
                copy.cast::<RibbonButton>().unwrap().label().as_deref(),
                Some("Live")
            );
            assert!(
                RibbonItemHelper::get_source_item(&copy)
                    .unwrap()
                    .ptr_eq(&original)
            );
            vm.set_model(None);
            drain();
            assert_eq!(r.tabs().count(), 1);
            assert_eq!(r.tabs().get(0), Some(declared));
            assert!(r.model().is_none());
            w.close();
            w.set_content(None);
            drain();
        }
    });
}

#[test]
fn independent_factory_context_parent_ownership_before_and_after_model_assignment() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::independent_factory_context_parent_ownership_before_and_after_model_assignment"
    )) {
        return;
    }
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        ValueTypes::register_reference::<RootModelViewModel>();
        MarkupType::register_all(&[RootModelViewModel::MARKUP]);
        for runtime in [false, true] {
            for assign_model in [false, true] {
                let (factory, weak_ribbon) = {
                    let value = if runtime {
                        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                            include_str!("RootModel.xaml"),
                            Some(&crate::CONSUMER_ASSEMBLY),
                        )
                        .unwrap()
                    } else {
                        crate::compiled_xaml::try_load(
                            None,
                            "ferres://ferroribbon_markup_consumer/RootModel.xaml",
                        )
                        .unwrap()
                        .unwrap()
                    };
                    let host = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StackPanel>()
                        .unwrap();
                    let ribbon = host.children().get(0).cast::<Ribbon>().unwrap();
                    let factory = ribbon
                        .item_factory()
                        .cast::<ConsumerElementFactory>()
                        .unwrap();
                    assert!(ribbon.model().is_none());
                    assert!(factory.ribbon().is_none());
                    factory.set_ribbon(Some(ribbon.clone()));
                    ribbon.set_item_factory(Some(factory.clone().upcast()));
                    factory.set_ribbon(None);
                    factory.set_ribbon(Some(ribbon.clone()));
                    if assign_model {
                        ribbon.set_model(Some(RibbonModel::new()));
                        factory.set_ribbon(Some(ribbon.clone()));
                    }
                    assert!(factory.ribbon().unwrap().ptr_eq(&ribbon));
                    let weak = ribbon.downgrade();
                    drop(ribbon);
                    drop(host);
                    drop(value);
                    (factory, weak)
                };
                drain();
                assert!(
                    weak_ribbon.upgrade().is_none(),
                    "runtime={runtime}, model={assign_model}"
                );
                assert!(factory.ribbon().is_none());
            }
        }
    });
}
