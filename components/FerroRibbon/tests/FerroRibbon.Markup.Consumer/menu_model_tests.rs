//! Independent source-style model XAML drives the public native factory.
use ferroribbon::{compat::*, controls::*, metadata::models::*};
use ferroribbon_core::{RibbonItemSize, commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use ferroui_controls::{CheckBox, Control, MenuFlyout, StackPanel, TextBlock};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};
fn cast<T: Clone + 'static>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn all_source_menu_model_constructor_arities_and_defaults_match_native_metadata() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let mut actual = vec![];
        for meta in [
            RibbonDropDownButtonModelMetadata::MARKUP,
            RibbonSplitButtonModelMetadata::MARKUP,
            RibbonCheckBoxModelMetadata::MARKUP,
            RibbonMenuItemModelMetadata::MARKUP,
            RibbonMenuSeparatorModelMetadata::MARKUP,
            RibbonMenuHeaderModelMetadata::MARKUP,
        ] {
            let args: Vec<Option<BoxedValue>> = match meta.name {
                "RibbonMenuSeparatorModel" => vec![],
                "RibbonMenuHeaderModel" => vec![Some(Rc::new("Label".to_owned()))],
                "RibbonCheckBoxModel" => vec![
                    Some(Rc::new("id".to_owned())),
                    Some(Rc::new("Label".to_owned())),
                    Some(Rc::new(true)),
                ],
                "RibbonDropDownButtonModel" => vec![
                    Some(Rc::new("id".to_owned())),
                    Some(Rc::new("Label".to_owned())),
                    None,
                    Some(Rc::new(RibbonItemSize::Small)),
                ],
                _ => vec![
                    Some(Rc::new("id".to_owned())),
                    Some(Rc::new("Label".to_owned())),
                    None,
                    None,
                    Some(Rc::new(RibbonItemSize::Small)),
                ],
            };
            for ctor in meta.constructors {
                let arity = ctor.parameters.len();
                let value = (ctor.invoke)(&args[..arity]).unwrap().unwrap();
                let node = cast::<Rc<dyn RibbonNodeModelImpl>>(&value);
                let item = try_cast_node::<dyn RibbonItemModelImpl>(Some(node.clone())).unwrap();
                let check =
                    try_cast_node::<dyn RibbonCheckBoxModelImpl>(Some(node.clone())).unwrap();
                let split =
                    try_cast_node::<dyn RibbonSplitButtonModelImpl>(Some(node.clone())).unwrap();
                let drop =
                    try_cast_node::<dyn RibbonDropDownButtonModelImpl>(Some(node.clone())).unwrap();
                let menu =
                    try_cast_node::<dyn RibbonMenuItemModelImpl>(Some(node.clone())).unwrap();
                let id = node.id().map(|s| s.to_utf8().unwrap());
                let generated = id
                    .as_ref()
                    .is_some_and(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()));
                actual.push(json!({"type":meta.name,"arity":arity,"id":if generated{Some("<generated-guid>".to_owned())}else{id},"label":node.label().map(|s|s.to_utf8().unwrap()),"size":item.map(|i|i.size().raw()),"isChecked":check.map(|c|c.is_checked()).unwrap_or_else(||split.as_ref().map(|s|s.is_checked())),"isCheckable":split.as_ref().map(|s|s.is_checkable()),"followLastChoice":split.map(|s|s.follow_last_choice()),"menuCount":drop.map(|m|m.menu_items().count()).or_else(||menu.map(|m|m.items().count()))}));
            }
        }
        assert_eq!(Value::Array(actual), source["corpus"]["menuConstructors"]);
    });
}
#[test]
fn compiled_and_runtime_menu_models_create_real_controls_bindings_commands_and_rebuilds() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("MenuModels.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/MenuModels.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |key: &str| {
                host.resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap()
            };
            let m = cast::<Rc<dyn RibbonDropDownButtonModelImpl>>(&resource("Drop"));
            let check = cast::<Rc<dyn RibbonCheckBoxModelImpl>>(&resource("Check"));
            let header = cast::<Rc<dyn RibbonNodeModelImpl>>(&resource("Header"));
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            let group = r.tabs().get(0).unwrap().groups().get(0).unwrap();
            let e = group
                .items()
                .get(0)
                .unwrap()
                .cast::<RibbonDropDownButton>()
                .unwrap();
            assert!(Rc::ptr_eq(
                &Ribbon::get_model(&e).unwrap(),
                &(m.clone() as Rc<dyn RibbonNodeModelImpl>)
            ));
            let flyout = e.flyout().unwrap();
            let menu = flyout.cast::<MenuFlyout>().unwrap();
            assert_eq!(menu.items().count(), 4);
            let first = Control::from_boxed(&menu.items().get_at(0).unwrap())
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            assert!(!first.is_enabled());
            assert_eq!(first.text().as_deref(), Some("Clipboard"));
            header.set_label(Some("Changed header".into()));
            assert_eq!(first.text().as_deref(), Some("Changed header"));
            assert_eq!(
                host.children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Changed header")
            );
            let entry = try_cast_node::<dyn RibbonMenuItemModelImpl>(m.menu_items().get(1))
                .unwrap()
                .unwrap();
            let invocations = Rc::new(Cell::new(0));
            let c = invocations.clone();
            entry.set_command(Some(RibbonRelayCommand::new(move || c.set(c.get() + 1))));
            let command = Control::from_boxed(&menu.items().get_at(1).unwrap())
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            command.invoke_menu_core();
            let reference: Value =
                serde_json::from_str(include_str!("../fixtures/menus.json")).unwrap();
            let counts = reference["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["name"] == "commandInvokerCopy")
                .unwrap();
            assert_eq!(
                invocations.get(),
                counts["afterSource"].as_i64().unwrap() as i32
            );
            let copy = RibbonMenuCloner::clone(Some(RibbonItemsSource::from_values(vec![Some(
                command.clone().upcast(),
            )])))
            .iter()
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
            copy.invoke_menu_core();
            // Preserve the source ordinary-copy double execution: invoker, then copied Command.
            assert_eq!(
                invocations.get(),
                counts["afterCopy"].as_i64().unwrap() as i32
            );
            entry.set_label(Some("Updated".into()));
            assert_eq!(command.text().as_deref(), Some("Updated"));
            assert!(flyout.ptr_eq(&e.flyout().unwrap()));
            entry.set_is_checkable(true);
            assert!(!flyout.ptr_eq(&e.flyout().unwrap()));
            let updated = e.flyout().unwrap().cast::<MenuFlyout>().unwrap();
            assert!(
                Control::from_boxed(&updated.items().get_at(1).unwrap())
                    .unwrap()
                    .is::<ToggleMenuFlyoutItem>()
            );
            assert!(group.items().get(1).unwrap().is::<RibbonSplitButton>());
            let native_check = group
                .items()
                .get(2)
                .unwrap()
                .cast::<RibbonCheckBox>()
                .unwrap();
            assert_eq!(native_check.is_checked(), None);
            let external = host.children().get(3).cast::<CheckBox>().unwrap();
            external.set_is_checked(Some(true));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(check.is_checked(), Some(true));
            assert_eq!(native_check.is_checked(), Some(true));
            native_check.set_is_checked(None);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(check.is_checked(), None);
            assert_eq!(external.is_checked(), None);
            r.set_model(None);
            entry.set_label(Some("Disposed".into()));
            assert_eq!(command.text().as_deref(), Some("Updated"));
        }
    });
}
