//! Read-only interface/list casts and source parser results through independent native metadata.
use ferroribbon::metadata::model_icon::{
    NativeIconLayerList, NativeIconLayerListItems, NativeIconLayers, RibbonIconLayerListMetadata,
    RibbonIconLayerMetadata, RibbonIconLayersMetadata, RibbonIconMetadata,
};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonString,
    model::{RibbonIcon, RibbonIconKind, RibbonIconLayer},
};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use serde_json::{Value, json};
use std::rc::Rc;
struct OddList;
impl ferroribbon_core::collections::RibbonReadOnlyListImpl<Option<Rc<RibbonIconLayer>>>
    for OddList
{
    fn count(&self) -> i32 {
        panic!("source custom Count must not be read by indexer")
    }
    fn get(&self, index: i32) -> Option<Rc<RibbonIconLayer>> {
        Some(RibbonIconLayer::new(Some(index.to_string().into())))
    }
    fn iter_owned(
        self: Rc<Self>,
    ) -> ferroribbon_core::model::RibbonIterator<Option<Rc<RibbonIconLayer>>> {
        Box::new(std::iter::empty())
    }
}
fn cast<T: ferroui_base::PropertyValue>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn original_parse_results_survive_native_interface_list_casts_and_mutation() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/icon-sequence.json")).unwrap();
        let parse = RibbonIconLayerMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "Parse")
            .unwrap();
        let input = Some(Rc::new("M0,0 L1,1|[stroke=2]M2,2 L3,3".to_owned()) as BoxedValue);
        let a = (parse.invoke)(std::slice::from_ref(&input))
            .unwrap()
            .unwrap();
        let b = (parse.invoke)(&[input]).unwrap().unwrap();
        let list = cast::<NativeIconLayerList>(&a);
        let readonly = cast::<NativeIconLayers>(&a);
        let alias = ValueTypes::try_cast(
            &(Rc::new(list.clone()) as BoxedValue),
            ValueType::of::<NativeIconLayers>(),
        )
        .unwrap();
        let alias = cast::<NativeIconLayers>(&alias);
        let first = readonly.get(0).unwrap();
        let same = Rc::ptr_eq(&first, &list.get(0).unwrap());
        let index = &RibbonIconLayerListMetadata::MARKUP.indexers[0];
        (index.set.unwrap())(&[
            Some(Rc::new(list.clone())),
            Some(Rc::new(0i32)),
            Some(Rc::new(RibbonIconLayer::new(Some("changed".into())))),
        ])
        .unwrap();
        let add = RibbonIconLayerListMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "Add")
            .unwrap();
        (add.invoke)(&[Some(Rc::new(list.clone())), None]).unwrap();
        assert_eq!(
            json!({"sameAlias":Rc::ptr_eq(&readonly, &alias),"differentParse":Rc::ptr_eq(&readonly, &cast::<NativeIconLayers>(&b)),"sameItem":same,"changed":readonly.get(0).unwrap().data().unwrap().to_utf8().unwrap(),"aliasCount":alias.count(),"otherCount":cast::<NativeIconLayers>(&b).count(),"nullItem":readonly.get(2).is_none()}),
            source["corpus"]["shared"]
        );
        let count =
            (RibbonIconLayersMetadata::MARKUP.properties[0].get.unwrap())(&[Some(a.clone())])
                .unwrap()
                .unwrap();
        assert_eq!(cast::<i32>(&count), 3);
        let item = (RibbonIconLayersMetadata::MARKUP.indexers[0].get.unwrap())(&[
            Some(a),
            Some(Rc::new(0i32)),
        ])
        .unwrap()
        .unwrap();
        assert!(Rc::ptr_eq(
            &cast::<Rc<RibbonIconLayer>>(&item),
            &readonly.get(0).unwrap()
        ));
        for (i, key) in [(-1, "negativeIndex"), (readonly.count(), "endIndex")] {
            let error = (RibbonIconLayersMetadata::MARKUP.indexers[0].get.unwrap())(&[
                Some(Rc::new(readonly.clone())),
                Some(Rc::new(i)),
            ])
            .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("ArgumentOutOfRangeException: index")
            );
            assert_eq!(
                source["corpus"]["errors"][key],
                json!({"type":"ArgumentOutOfRangeException","parameter":"index"})
            );
        }
        let custom = Rc::new(OddList) as NativeIconLayers;
        let custom_value = (RibbonIconLayersMetadata::MARKUP.indexers[0].get.unwrap())(&[
            Some(Rc::new(custom)),
            Some(Rc::new(-1i32)),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(
            cast::<Rc<RibbonIconLayer>>(&custom_value)
                .data()
                .unwrap()
                .to_utf8()
                .unwrap(),
            source["corpus"]["customIndex"]
        );
        let array: NativeIconLayers = RibbonFixedItems::new(vec![None]);
        let value = Rc::new(array) as BoxedValue;
        assert!(
            ValueTypes::try_cast(&value, ValueType::of::<NativeIconLayerList>()).is_none(),
            "Array does not implement concrete List"
        );
        let op = RibbonIconMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "op_Implicit")
            .unwrap();
        for (input, expected) in [
            (None, "convertedNull"),
            (
                Some(Rc::new("plain".to_owned()) as BoxedValue),
                "convertedText",
            ),
        ] {
            let output = (op.invoke)(&[input]).unwrap().unwrap();
            let icon = cast::<Rc<RibbonIcon>>(&output);
            assert_eq!(icon.kind, RibbonIconKind::Glyph);
            assert_eq!(
                json!({"kind":icon.kind.raw(),"value":icon.value.as_ref().map(|v|v.as_utf16()),"viewbox":format!("{:016x}",icon.view_box_size.to_bits())}),
                source["corpus"][expected]
            );
        }
    });
}
#[test]
fn independent_compiled_and_runtime_xaml_index_shared_parser_results_without_list_notifications() {
    crate::tests::run(|| {
        use crate::icon_sequence_consumer::IconSequenceConsumer;
        use ferroui_controls::{StackPanel, TextBlock};
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("IconSequences.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/IconSequences.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let declared = host
                .resources()
                .try_get_resource(&"Declared".into(), None)
                .flatten()
                .unwrap();
            let declared = cast::<NativeIconLayerList>(&declared);
            assert_eq!(declared.len(), 1);
            assert_eq!(declared.get(0).unwrap().data(), Some("declared".into()));
            let data = host
                .resources()
                .try_get_resource(&"Data".into(), None)
                .flatten()
                .unwrap();
            let data = cast::<Rc<IconSequenceConsumer>>(&data);
            let count = host.children().get(0).cast::<TextBlock>().unwrap();
            let first = host.children().get(1).cast::<TextBlock>().unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(count.text().as_deref(), Some("2"));
            assert_eq!(first.text().as_deref(), Some("M0,0 L1,1"));
            let raw_count = host.children().get(2).cast::<TextBlock>().unwrap();
            let tag_count = host.children().get(3).cast::<TextBlock>().unwrap();
            assert_eq!(raw_count.text().as_deref(), Some("1"));
            assert_eq!(tag_count.text().as_deref(), Some("1"));
            let command = host
                .resources()
                .try_get_resource(&"CollectionCommand".into(), None)
                .flatten()
                .unwrap();
            let command = cast::<Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>>(&command);
            let replacement = NativeIconLayerListItems::from_values(vec![
                Some(RibbonIconLayer::new(Some("one".into()))),
                Some(RibbonIconLayer::new(Some("two".into()))),
            ]);
            ferroribbon_core::model::RibbonNodeModelExt::set_tag(
                command.as_ref(),
                Some(ferroribbon_core::model::RibbonObject::reference(
                    replacement.clone(),
                )),
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(tag_count.text().as_deref(), Some("2"));
            assert_eq!(raw_count.text().as_deref(), Some("1"));
            let mutable = (data.layers() as Rc<dyn std::any::Any>).downcast::<ferroribbon_core::collections::RibbonItems<Option<Rc<RibbonIconLayer>>>>().unwrap();
            mutable.add(None);
            mutable.set(0, Some(RibbonIconLayer::new(Some("Mutated".into()))));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                count.text().as_deref(),
                Some("2"),
                "source List has no collection notification"
            );
            assert_eq!(first.text().as_deref(), Some("M0,0 L1,1"));
            data.set_layers(RibbonIconLayer::parse(Some(&RibbonString::from(
                "replacement",
            ))));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(count.text().as_deref(), Some("1"));
            assert_eq!(first.text().as_deref(), Some("replacement"));
        }
    });
}

// Source regression: object identity spans concrete/interface views.
#[test]
fn source_icon_list_object_identity_across_interface_views_reproducer() {
    crate::tests::run(|| {
        use ferroribbon::metadata::models::RibbonNodeModelMetadata;
        use ferroribbon_core::model::{RibbonButtonModel, RibbonNodeModelImpl};
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/icon-sequence.json")).unwrap();
        let parsed = RibbonIconLayer::parse(Some(&"M0,0 L1,1".into()));
        let list = (parsed.clone() as Rc<dyn std::any::Any>)
            .downcast::<ferroribbon_core::collections::RibbonItems<Option<Rc<RibbonIconLayer>>>>()
            .unwrap();
        let model = RibbonButtonModel::new(Some("id".into()), Some("Label".into()))
            as Rc<dyn RibbonNodeModelImpl>;
        let owner = Rc::new(model) as BoxedValue;
        let tag = RibbonNodeModelMetadata::MARKUP
            .properties
            .iter()
            .find(|p| p.name == "Tag")
            .unwrap();
        let original = Rc::new(list) as BoxedValue;
        (tag.set.unwrap())(&[Some(owner.clone()), Some(original.clone())]).unwrap();
        let original_object = (tag.get.unwrap())(&[Some(owner.clone())]).unwrap().unwrap();
        let canonical = ValueTypes::reference_object(&original).unwrap_or_else(|| original.clone());
        assert!(Rc::ptr_eq(&original_object, &canonical));

        (tag.set.unwrap())(&[Some(owner.clone()), Some(Rc::new(parsed))]).unwrap();
        let retained = (tag.get.unwrap())(&[Some(owner)]).unwrap().unwrap();
        assert_eq!(
            Rc::ptr_eq(&original_object, &retained),
            source["corpus"]["boxedSame"].as_bool().unwrap(),
            "source retains the same List object across its interface view"
        );
    });
}

#[test]
fn native_layers_array_invoker_preserves_null_errors_joining_and_empty_params() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/icon-sequence.json")).unwrap();
        let method = RibbonIconMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "Layers" && m.parameters.len() == 2)
            .unwrap();
        let snapshot = |value: BoxedValue| {
            let icon = cast::<Rc<RibbonIcon>>(&value);
            json!({"kind":icon.kind.raw(),"value":icon.value.as_ref().map(|s|s.as_utf16()),"viewbox":format!("{:016x}",icon.view_box_size.to_bits())})
        };
        let array = RibbonFixedItems::new(vec![
            Some(RibbonIconLayer::new(None)),
            Some(RibbonIconLayer::new(Some("x".into()))),
            Some(RibbonIconLayer::new(None)),
        ]);
        let output = (method.invoke)(&[Some(Rc::new(32.0f64)), Some(Rc::new(array))])
            .unwrap()
            .unwrap();
        assert_eq!(snapshot(output), source["corpus"]["joined"]);
        let error = (method.invoke)(&[Some(Rc::new(24.0f64)), None]).unwrap_err();
        assert!(error.to_string().contains("ArgumentNullException: layers"));
        let invalid =
            RibbonFixedItems::new(vec![Some(RibbonIconLayer::new(Some("x".into()))), None]);
        let error = (method.invoke)(&[Some(Rc::new(24.0f64)), Some(Rc::new(invalid))]).unwrap_err();
        assert!(error.to_string().contains("NullReferenceException"));
        let empty = RibbonIconMetadata::MARKUP
            .methods
            .iter()
            .find(|m| m.name == "Layers" && m.parameters.len() == 1)
            .unwrap();
        let value = (empty.invoke)(&[Some(Rc::new(-0.0f64))]).unwrap().unwrap();
        assert_eq!(snapshot(value), source["corpus"]["empty"]);
    });
}
