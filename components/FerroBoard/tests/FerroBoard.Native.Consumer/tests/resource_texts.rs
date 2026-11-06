use ferroboard_core::{
    layouts::{LayoutJson, ResourceTexts},
    text::Utf16Text,
};
use ferroboard_core_ferro::LayoutAuthorsList;

#[test]
fn resource_authors_share_native_storage_and_checked_markup_indices() {
    let storage = ResourceTexts::from_nullable(vec![Some("writer".into()), None]);
    let alias = ResourceTexts::from_shared(storage.shared_values());
    let facade = LayoutAuthorsList::new(storage.clone());
    assert!(facade.source().ptr_eq(&alias));
    assert_eq!(facade.count(), 2);
    assert_eq!(facade.get(0).unwrap(), Some("writer".into()));
    assert_eq!(facade.get(1).unwrap(), None);
    assert!(facade.get(-1).is_err());
    assert!(facade.get(i32::MAX).is_err());
    let snapshot = storage.iter();
    alias
        .set(0, Some(Utf16Text::from_units(vec![0xd800])))
        .unwrap();
    alias.push(Some("next".into()));
    assert_eq!(
        snapshot.collect::<Vec<_>>(),
        vec![Some("writer".into()), None]
    );
    assert_eq!(facade.count(), 3);
    assert_eq!(facade.get(0).unwrap().unwrap().units(), [0xd800]);
    let layout =
        LayoutJson::read_layout_document(r#"{"id":"a","authors":["writer"]}"#, "local".into())
            .unwrap();
    layout.set_authors(Some(alias));
    assert!(layout.authors().unwrap().ptr_eq(&storage));
    // Raw UTF-16 stays accessible through the native facade; String/XAML conversion
    // remains fallible and is exercised by the existing encoding consumer fixtures.
}
