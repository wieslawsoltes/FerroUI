use crate::*;
use ferroboard_core::{layouts::*, text::*};
use ferroui_base::metadata::{MarkupTyped, from_markup_value, into_markup_value};
use std::{cell::Cell, rc::Rc};

#[test]
fn native_list_views_keep_storage_nulls_bounds_and_mutable_element_identity() {
    register_types();
    let text: TextKey = Rc::new(TextKeyData::default());
    let store = ResourceObjectList::new(vec![Some(text.clone()), None]);
    let view =
        TextKeyReadOnlyList::from_original(ResourceReadOnlyList::from_nullable(store.clone()));
    let copy = TextKeyReadOnlyList::from_original(view.original());
    assert!(view == copy);
    let upcast = view.as_keys();
    assert_eq!(upcast.count().unwrap(), 2);
    let actual = upcast.get(0).unwrap().unwrap();
    let expected: Key = text.clone();
    assert!(Rc::ptr_eq(&actual, &expected));
    assert!(upcast.get(1).unwrap().is_none());
    assert!(view.get(-1).is_err());
    assert!(view.get(2).is_err());
    store.push(Some(text.clone()));
    assert_eq!(upcast.count().unwrap(), 3);
    text.data().set_label(Some("shared".into()));
    assert_eq!(
        view.get(0).unwrap().unwrap().data().label(),
        Some("shared".into())
    );
    let candidate = Rc::new(Suggestion::new("word".into(), SuggestionKind::Typed));
    let suggestions = SuggestionList::from_nullable(vec![Some(candidate.clone()), None]);
    let view = SuggestionReadOnlyList::from_original(suggestions.clone());
    assert!(view == SuggestionReadOnlyList::from_original(suggestions));
    let value = into_markup_value(view);
    let item = (SuggestionReadOnlyListMetadata::MARKUP.indexers[0]
        .get
        .unwrap())(&[value, into_markup_value(0_i32)])
    .unwrap();
    assert!(Rc::ptr_eq(
        &candidate,
        &from_markup_value::<Rc<Suggestion>>(&item).unwrap()
    ));
}

struct Custom {
    reads: Cell<usize>,
}
impl IResourceReadOnlyList<Key> for Custom {
    fn count(&self) -> Result<i32, ResourceTextListError> {
        self.reads.set(self.reads.get() + 1);
        Ok(-4)
    }
    fn get(&self, index: i32) -> Result<Option<Key>, ResourceTextListError> {
        self.count()?;
        if index == 0 {
            Ok(None)
        } else {
            Err(ResourceTextListError::NotSupported)
        }
    }
    fn iter(&self) -> Result<ResourceItems<Key>, ResourceTextListError> {
        Ok(Box::new(std::iter::empty()))
    }
}
#[test]
fn native_key_view_preserves_custom_signed_count_null_and_error_dispatch() {
    register_types();
    let source = Rc::new(Custom {
        reads: Cell::new(0),
    });
    let view = KeyReadOnlyList::from_original(ResourceReadOnlyList::from_custom(source.clone()));
    let value = into_markup_value(view);
    let count = (KeyReadOnlyListMetadata::MARKUP
        .find_property("Count")
        .unwrap()
        .get
        .unwrap())(std::slice::from_ref(&value))
    .unwrap();
    assert_eq!(from_markup_value::<i32>(&count), Some(-4));
    let index = KeyReadOnlyListMetadata::MARKUP.indexers[0].get.unwrap();
    assert!(
        from_markup_value::<Option<Key>>(
            &index(&[value.clone(), into_markup_value(0_i32)]).unwrap()
        )
        .unwrap()
        .is_none()
    );
    assert!(index(&[value, into_markup_value(7_i32)]).is_err());
    assert_eq!(source.reads.get(), 3);
}

#[test]
fn native_prediction_indexer_reports_unpaired_utf16_instead_of_replacing_it() {
    register_types();
    let list = native_string_list(Rc::new(PredictionListBuffer::new(vec![
        Utf16Text::from_units(vec![0xd800]),
    ])));
    let get = StringReadOnlyListMetadata::MARKUP.indexers[0].get.unwrap();
    assert!(get(&[into_markup_value(list.clone()), into_markup_value(0_i32)]).is_err());
    assert_eq!(list.get(0).unwrap().unwrap().units(), [0xd800]);
}

struct NullTextCursor;
impl IResourceReadOnlyList<TextKey> for NullTextCursor {
    fn count(&self) -> Result<i32, ResourceTextListError> {
        Ok(0)
    }
    fn get(&self, _: i32) -> Result<Option<TextKey>, ResourceTextListError> {
        Err(ResourceTextListError::IndexOutOfRange)
    }
    fn iter(&self) -> Result<ResourceItems<TextKey>, ResourceTextListError> {
        Ok(Box::new(std::iter::empty()))
    }
}
#[test]
fn trait_upcast_preserves_null_cursor_and_domain_metadata_has_no_collection_facets() {
    register_types();
    let text = TextKeyReadOnlyList::from_original(ResourceReadOnlyList::from_custom(Rc::new(
        NullTextCursor,
    )));
    assert!(text.as_keys().original().iter().unwrap().next().is_none());
    for descriptor in [
        KeyReadOnlyListMetadata::MARKUP,
        TextKeyReadOnlyListMetadata::MARKUP,
        StringReadOnlyListMetadata::MARKUP,
        SuggestionReadOnlyListMetadata::MARKUP,
    ] {
        assert!(descriptor.namespace().starts_with("FerroBoard."));
        assert!(descriptor.interfaces.is_empty());
        assert!(descriptor.find_property("Count").is_some());
        assert_eq!(descriptor.indexers.len(), 1);
        assert!(descriptor.find_methods("GetEnumerator").next().is_none());
    }
}
