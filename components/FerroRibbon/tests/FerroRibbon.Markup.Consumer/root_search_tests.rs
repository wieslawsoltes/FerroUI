use ferroribbon::{
    controls::{Ribbon, RibbonButton},
    search::{RibbonSearchEngine, RibbonSearchEntry, RibbonSearchResult},
};
use ferroui_base::{
    controls::NameScope,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
};
use ferroui_controls::{Control, TextBlock, Window};
use std::rc::Rc;
fn value<T: ferroui_base::PropertyValue>(v: &ferroui_base::BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn source_root_search_compiles_and_loads_records_collections_bindings_and_actual_targets() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let loaded = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("RootSearch.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/RootSearch.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*loaded)
                .unwrap()
                .cast::<Control>()
                .unwrap();
            let scope = NameScope::get_name_scope(&root).unwrap();
            let r = scope.find_as::<Ribbon>("SearchRoot").unwrap();
            let w = Window::new();
            w.set_content(Some(Control::boxed(root.clone())));
            w.show();
            w.update_layout();
            let count = scope.find_as::<TextBlock>("Count").unwrap();
            assert_eq!(count.text().as_deref(), Some("1"));
            let help = value::<Rc<RibbonSearchEntry>>(
                &root
                    .resources()
                    .try_get_resource(&"Help".into(), None)
                    .flatten()
                    .unwrap(),
            );
            assert!(Rc::ptr_eq(
                &r.additional_search_entries().get(0).unwrap(),
                &help
            ));
            let full = value::<Rc<RibbonSearchEntry>>(
                &root
                    .resources()
                    .try_get_resource(&"FullEntry".into(), None)
                    .flatten()
                    .unwrap(),
            );
            assert_eq!(full.id().unwrap().to_utf8().unwrap(), "full");
            assert_eq!(full.label().unwrap().to_utf8().unwrap(), "Hé𐐨");
            assert_eq!(full.path().unwrap().to_utf8().unwrap(), "Path");
            assert_eq!(
                full.description().unwrap().to_utf8().unwrap(),
                "Description"
            );
            assert!(full.keywords().is_none());
            assert_eq!(full.shortcut().unwrap().to_utf8().unwrap(), "Ctrl+H");
            assert!(!full.is_enabled());
            let meta = ferroribbon::metadata::search::RibbonSearchEntryMetadata::MARKUP;
            let payload = (meta.find_property("Target").unwrap().get.unwrap())(&[Some(Rc::new(
                full.clone(),
            ))])
            .unwrap()
            .unwrap();
            assert_eq!(value::<String>(&payload), "payload");
            for name in [
                "Id",
                "Label",
                "Path",
                "Description",
                "Keywords",
                "Shortcut",
                "Target",
                "IsEnabled",
            ] {
                assert!(
                    meta.find_property(name).unwrap().set.is_none(),
                    "immutable {name}"
                );
            }
            let result = value::<Rc<RibbonSearchResult>>(
                &root
                    .resources()
                    .try_get_resource(&"FullResult".into(), None)
                    .flatten()
                    .unwrap(),
            );
            assert!(Rc::ptr_eq(&result.entry().unwrap(), &full));
            assert_eq!(result.score(), 1.25);
            let entries = r.build_search_entries();
            assert_eq!(entries.count(), 2);
            let alpha = scope.find_as::<RibbonButton>("Alpha").unwrap();
            let target = (ferroribbon::metadata::search::RibbonSearchEntryMetadata::MARKUP
                .find_property("Target")
                .unwrap()
                .get
                .unwrap())(&[Some(Rc::new(entries.get(0).unwrap()))])
            .unwrap()
            .unwrap();
            assert!(ValueTypes::as_object(&*target).unwrap().ptr_eq(&alpha));
            assert_eq!(r.search_default(Some(&"Alpha".into())).len(), 1);
            r.additional_search_entries()
                .add(Some(RibbonSearchEntry::new(
                    Some("other".into()),
                    Some("Other".into()),
                )));
            w.update_layout();
            assert_eq!(count.text().as_deref(), Some("2"));
            let engine = value::<Rc<RibbonSearchEngine>>(
                &root
                    .resources()
                    .try_get_resource(&"Engine".into(), None)
                    .flatten()
                    .unwrap(),
            );
            assert_eq!(engine.max_recent(), 3);
            assert_eq!(
                scope
                    .find_as::<TextBlock>("RecentLimit")
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("3")
            );
            w.close();
        }
    });
}
