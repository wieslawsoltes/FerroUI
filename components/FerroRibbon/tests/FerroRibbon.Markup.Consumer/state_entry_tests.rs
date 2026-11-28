use ferroribbon::metadata::state_types::{
    RibbonStateGroupOrderMetadata, RibbonStateLabelsMetadata,
};
use ferroribbon_core::state::*;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupTyped, from_markup_value, into_markup_value},
};
use ferroui_controls::{StackPanel, TextBlock};
use std::rc::Rc;
#[test]
fn both_loaders_use_native_shared_state_members_constructors_indexers_and_compiled_counts() {
    crate::tests::run(|| {
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("StateEntries.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/StateEntries.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let owner = ValueTypes::try_cast(
                &panel
                    .children()
                    .get(0)
                    .cast::<TextBlock>()
                    .unwrap()
                    .data_context()
                    .unwrap(),
                ValueType::of::<Rc<RibbonCustomization>>(),
            )
            .unwrap();
            let owner = owner.downcast_ref::<Rc<RibbonCustomization>>().unwrap();
            let original = owner.labels().unwrap();
            let built = (RibbonStateLabelsMetadata::MARKUP.constructors[0].invoke)(&[]).unwrap();
            let labels = from_markup_value::<RibbonStateLabels>(&built).unwrap();
            assert!(!Rc::ptr_eq(&labels, &original));
            let boxed = into_markup_value(labels.clone());
            let index = &RibbonStateLabelsMetadata::MARKUP.indexers[0];
            (index.set.unwrap())(&[
                boxed.clone(),
                into_markup_value("home".to_string()),
                into_markup_value(Some("Start".to_string())),
            ])
            .unwrap();
            let out = (index.get.unwrap())(&[boxed.clone(), into_markup_value("home".to_string())])
                .unwrap();
            assert_eq!(
                from_markup_value::<Option<String>>(&out).unwrap(),
                Some("Start".into())
            );
            let contains = RibbonStateLabelsMetadata::MARKUP
                .find_methods("ContainsKey")
                .next()
                .unwrap();
            assert!(
                from_markup_value::<bool>(
                    &(contains.invoke)(&[boxed.clone(), into_markup_value("home".to_string())])
                        .unwrap()
                )
                .unwrap()
            );
            owner.set_labels(Some(labels.clone()));
            assert!(Rc::ptr_eq(&labels, &owner.labels().unwrap()));
            assert_eq!(labels.get(&"home".into()), Some(Some("Start".into())));
            let group =
                (RibbonStateGroupOrderMetadata::MARKUP.constructors[0].invoke)(&[]).unwrap();
            let groups = from_markup_value::<RibbonStateGroupOrder>(&group).unwrap();
            owner.set_group_order(Some(groups.clone()));
            assert!(Rc::ptr_eq(&groups, &owner.group_order().unwrap()));
            let remove = RibbonStateLabelsMetadata::MARKUP
                .find_methods("Remove")
                .next()
                .unwrap();
            assert!(
                from_markup_value::<bool>(
                    &(remove.invoke)(&[boxed, into_markup_value("home".to_string())]).unwrap()
                )
                .unwrap()
            );
            assert!(labels.is_empty());
            for i in [0, 1] {
                assert_eq!(
                    panel
                        .children()
                        .get(i)
                        .cast::<TextBlock>()
                        .unwrap()
                        .text()
                        .as_deref(),
                    Some("0"),
                    "state members retain the source's non-notifying collection contract"
                );
            }
        }
    });
}
