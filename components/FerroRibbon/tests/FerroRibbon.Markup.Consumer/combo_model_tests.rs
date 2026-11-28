//! Independent construction/compiled bindings over actual shared Core object collections.
use ferroribbon::{
    controls::*,
    metadata::{
        combo_items::{RibbonObjectEnumerable, RibbonObjectItemsMetadata},
        models::RibbonComboBoxModelMetadata,
    },
};
use ferroribbon_core::{RibbonItemSize, model::*};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use ferroui_controls::{StackPanel, TextBlock, TextBox};
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
fn original_combo_constructor_arities_enumeration_defaults_and_payloads() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let sequence: RibbonObjectEnumerable = RibbonItemsSource::from_values(vec![
            Some(Rc::new("One".to_owned()) as BoxedValue),
            None,
            Some(Rc::new(23i32) as BoxedValue),
        ]);
        let args: Vec<Option<BoxedValue>> = vec![
            Some(Rc::new("id".to_owned())),
            Some(Rc::new("Label".to_owned())),
            Some(Rc::new(sequence)),
        ];
        let mut actual = vec![];
        for ctor in RibbonComboBoxModelMetadata::MARKUP.constructors {
            let arity = ctor.parameters.len();
            let v = (ctor.invoke)(&args[..arity]).unwrap().unwrap();
            let m = cast::<Rc<dyn RibbonComboBoxModelImpl>>(&v);
            let id = m.id().map(|s| s.to_utf8().unwrap());
            let ids = if arity == 0 {
                assert!(id.as_ref().unwrap().len() == 32);
                Some("<generated-guid>".to_owned())
            } else {
                id
            };
            let items = m
                .items()
                .iter()
                .map(|v| {
                    v.map(|v| {
                        if let Some(s) = v.downcast_ref::<String>() {
                            json!(s)
                        } else {
                            json!(v.downcast_ref::<i32>().unwrap())
                        }
                    })
                    .unwrap_or(Value::Null)
                })
                .collect::<Vec<_>>();
            actual.push(json!({"arity":arity,"id":ids,"label":m.label().map(|s|s.to_utf8().unwrap()),"size":m.size().raw(),"items":items,"width":m.input_width(),"max":m.max_drop_down_height(),"editable":m.is_editable(),"preview":m.preview_font_family(),"selected":null,"text":m.text().map(|s|s.to_utf8().unwrap())}));
            assert_eq!(m.size(), RibbonItemSize::Small);
        }
        let mut expected = source["corpus"]["comboConstructors"].clone();
        for entry in expected.as_array_mut().unwrap() {
            for key in ["width", "max"] {
                entry[key] = json!(entry[key].as_f64().unwrap());
            }
        }
        assert_eq!(Value::Array(actual), expected);
        // Constructor enumerates exactly once, retaining each actual native payload.
        let count = Rc::new(Cell::new(0));
        let n = count.clone();
        let payload: BoxedValue = Rc::new("same".to_owned());
        let p = payload.clone();
        let sequence = RibbonObjectEnumerable::new(move || {
            n.set(n.get() + 1);
            Box::new(vec![Some(p.clone())].into_iter())
        });
        let v = (RibbonComboBoxModelMetadata::MARKUP.constructors[3].invoke)(&[
            None,
            None,
            Some(Rc::new(sequence)),
        ])
        .unwrap()
        .unwrap();
        let m = cast::<Rc<dyn RibbonComboBoxModelImpl>>(&v);
        assert_eq!(count.get(), 1);
        let collection = Rc::new(m.items()) as BoxedValue;
        let get = RibbonObjectItemsMetadata::MARKUP.indexers[0].get.unwrap();
        let got = get(&[Some(collection), Some(Rc::new(0i32))])
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&got, &payload));
    });
}
#[test]
fn compiled_runtime_model_combo_shares_items_recreates_fonts_and_binds_two_way() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ComboModels.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ComboModels.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |name: &str| {
                host.resources()
                    .try_get_resource(&name.into(), None)
                    .flatten()
                    .unwrap()
            };
            let m = cast::<Rc<dyn RibbonComboBoxModelImpl>>(&resource("Combo"));
            let empty = cast::<Rc<dyn RibbonComboBoxModelImpl>>(&resource("Empty"));
            assert_eq!(empty.items().count(), 0);
            let initial = cast::<RibbonObjectItems>(&resource("Initial"));
            assert_eq!(initial.count(), 3);
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            let g = r.tabs().get(0).unwrap().groups().get(0).unwrap();
            let get = || g.items().get(0).unwrap().cast::<RibbonComboBox>().unwrap();
            let e = get();
            assert!(
                e.items_source()
                    .unwrap()
                    .downcast_ref::<RibbonObjectItems>()
                    .unwrap()
                    == &m.items()
            );
            assert_eq!(e.effective_items().len(), 3);
            assert_eq!(e.items().count(), 0);
            assert_eq!(
                host.children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("3")
            );
            assert_eq!(
                host.children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("One")
            );
            let inline = g.items().get(1).unwrap().cast::<RibbonComboBox>().unwrap();
            assert_eq!(
                inline
                    .get_item_text(inline.effective_items().get(0))
                    .as_deref(),
                Some("Nested")
            );
            assert_eq!(
                inline
                    .get_item_text(inline.effective_items().get(1))
                    .as_deref(),
                Some("Node label")
            );
            let native = Rc::new(m.items()) as BoxedValue;
            let add = RibbonObjectItemsMetadata::MARKUP
                .methods
                .iter()
                .find(|m| m.name == "Add")
                .unwrap();
            let appended: BoxedValue = Rc::new("Added".to_owned());
            (add.invoke)(&[Some(native), Some(appended)]).unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                host.children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("4")
            );
            assert_eq!(e.effective_items().len(), 4);
            assert_eq!(initial.count(), 3);
            e.commit(Some(Rc::new("typed".to_owned())));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(m.text().unwrap().to_utf8().unwrap(), "typed");
            let editor = host.children().get(3).cast::<TextBox>().unwrap();
            editor.set_text(Some("external"));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(e.text().as_deref(), Some("external"));
            m.set_preview_font_family(true);
            Dispatcher::ui_thread().run_jobs(None);
            let font = get();
            assert!(font.is::<RibbonFontComboBox>());
            assert!(!font.ptr_eq(&e));
            assert_eq!(font.items().count(), 0);
            assert_eq!(font.effective_items().len(), 4);
            m.set_input_width(160.);
            assert_eq!(font.input_width(), 160.);
            assert_eq!(e.input_width(), 120.);
            r.set_model(None);
        }
    });
}

#[test]
fn constructor_resource_runtime_type_mismatch_fails_in_both_loaders() {
    crate::tests::run(|| {
        for runtime in [false, true] {
            let error = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ComboConstructorInvalid.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .expect_err("incompatible resource must fail")
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ComboConstructorInvalid.xaml",
                )
                .expect_err("incompatible resource must fail")
            };
            assert!(error.to_string().contains("Unable to cast"), "{error}");
        }
    });
}
