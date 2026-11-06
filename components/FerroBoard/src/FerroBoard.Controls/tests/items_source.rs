#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::items_source::KeyboardItems;
use ferroboard_core::{
    layouts::{
        IResourceReadOnlyList, Key, ResourceItems, ResourceObjectList, ResourceReadOnlyList,
        ResourceTextListError, TextKey, TextKeyData,
    },
    text::{PredictionListBuffer, Suggestion, SuggestionKind, SuggestionList, Utf16Text},
};
use ferroboard_core_ferro::{
    KeyReadOnlyList, StringReadOnlyList, SuggestionReadOnlyList, TextKeyReadOnlyList,
    native_string_list,
};
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupInvokeError, from_markup_value, into_markup_value},
};
use ferroui_controls::{IItemsList, ItemsSource};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn settings_and_enter_action_sources_share_native_vectors_without_notifications() {
    headless::run(|| {
        use ferroboard_core::{
            editor::{EnterAction, ITextInputTarget, TextBufferTarget},
            settings::KeyboardSettings,
        };
        use ferroboard_core_ferro::{PerformedActionList, SettingsSubtypeList};
        ferroboard_controls::register_types();
        let settings = KeyboardSettings::default();
        let subtypes = Rc::new(RefCell::new(vec![Some("en-US/qwerty".into()), None]));
        settings.set_subtypes(Some(subtypes.clone()));
        let view = SettingsSubtypeList::from_original(settings.subtypes());
        let source = convert(into_markup_value(view.clone()));
        assert!(source.ptr_eq(&convert(into_markup_value(view))));
        assert!(!source.is_notifying());
        assert_eq!(source.count(), 2);
        assert_eq!(
            from_markup_value::<String>(&source.get_at(0)).as_deref(),
            Some("en-US/qwerty")
        );
        assert!(source.get_at(1).is_none());
        subtypes.borrow_mut().push(Some("ja-JP/kana".into()));
        assert_eq!(source.count(), 3);
        assert_eq!(
            from_markup_value::<String>(&source.get_at(2)).as_deref(),
            Some("ja-JP/kana")
        );
        assert!(
            source
                .downcast_ref::<KeyboardItems>()
                .unwrap()
                .try_get_at(3)
                .is_err()
        );

        let target = TextBufferTarget::default();
        target.perform_enter_action(EnterAction::Go);
        let actions = PerformedActionList::from_original(target.performed_actions());
        let source = convert(into_markup_value(actions.clone()));
        assert!(source.ptr_eq(&convert(into_markup_value(actions.clone()))));
        assert!(!source.is_notifying());
        assert_eq!(source.count(), 1);
        assert_eq!(
            from_markup_value::<EnterAction>(&source.get_at(0)),
            Some(EnterAction::Go)
        );
        target.perform_enter_action(EnterAction::Search);
        assert_eq!(source.count(), 2);
        assert_eq!(
            from_markup_value::<EnterAction>(&source.get_at(1)),
            Some(EnterAction::Search)
        );
        actions.clear();
        assert_eq!(source.count(), 0);
        assert!(
            source
                .downcast_ref::<KeyboardItems>()
                .unwrap()
                .try_get_at(0)
                .is_err()
        );
        let optional = ValueTypes::try_convert(
            into_markup_value(Some(actions)).as_ref(),
            ValueType::of::<Option<ItemsSource>>(),
        )
        .unwrap();
        let optional = from_markup_value::<Option<ItemsSource>>(&optional)
            .unwrap()
            .unwrap();
        assert!(source.ptr_eq(&optional));
        for value in [
            into_markup_value(Option::<SettingsSubtypeList>::None),
            into_markup_value(Option::<PerformedActionList>::None),
        ] {
            let actual =
                ValueTypes::try_convert(value.as_ref(), ValueType::of::<Option<ItemsSource>>())
                    .unwrap();
            assert!(
                from_markup_value::<Option<ItemsSource>>(&actual)
                    .unwrap()
                    .is_none()
            );
        }
    });
}

fn convert(value: Option<BoxedValue>) -> ItemsSource {
    let converted =
        ValueTypes::try_convert(value.as_ref(), ValueType::of::<ItemsSource>()).unwrap();
    from_markup_value(&converted).unwrap()
}

#[test]
fn registered_native_sources_share_backing_values_nulls_and_live_mutations() {
    headless::run(|| {
        ferroboard_controls::register_types();
        let key: TextKey = Rc::new(TextKeyData::default());
        let store = ResourceObjectList::new(vec![Some(key.clone()), None]);
        let text_view =
            TextKeyReadOnlyList::from_original(ResourceReadOnlyList::from_nullable(store.clone()));
        let base_view = text_view.as_keys();
        let candidate = Rc::new(Suggestion::new("word 😀".into(), SuggestionKind::Typed));
        let suggestions =
            SuggestionReadOnlyList::from_original(SuggestionList::from_nullable(vec![
                Some(candidate.clone()),
                None,
            ]));
        let predictions = Rc::new(PredictionListBuffer::from_nullable(vec![
            Some("prediction 😀".into()),
            None,
        ]));
        let strings: StringReadOnlyList = native_string_list(predictions.clone());
        let null = ValueTypes::try_convert(
            into_markup_value(Option::<TextKeyReadOnlyList>::None).as_ref(),
            ValueType::of::<Option<ItemsSource>>(),
        )
        .expect("registered optional source conversion");
        assert!(
            from_markup_value::<Option<ItemsSource>>(&null)
                .unwrap()
                .is_none()
        );
        let mut sources = Vec::new();
        for value in [
            into_markup_value(text_view.clone()),
            into_markup_value(base_view.clone()),
            into_markup_value(suggestions.clone()),
            into_markup_value(strings.clone()),
        ] {
            let source = convert(value.clone());
            assert!(source.ptr_eq(&convert(value)));
            assert_eq!(source.count(), 2);
            assert!(!source.is_notifying());
            assert!(source.get_at(1).is_none());
            sources.push(source);
        }
        let actual: TextKey = from_markup_value(&sources[0].get_at(0)).unwrap();
        assert!(Rc::ptr_eq(&key, &actual));
        let actual: Key = from_markup_value(&sources[1].get_at(0)).unwrap();
        assert!(Rc::ptr_eq(&(key.clone() as Key), &actual));
        let actual: Rc<Suggestion> = from_markup_value(&sources[2].get_at(0)).unwrap();
        assert!(Rc::ptr_eq(&candidate, &actual));
        assert_eq!(
            from_markup_value::<String>(&sources[3].get_at(0)).as_deref(),
            Some("prediction 😀")
        );
        store.push(Some(key.clone()));
        assert_eq!(sources[0].count(), 3);
        assert_eq!(sources[1].count(), 3);
        predictions.set(0, "changed".into()).unwrap();
        assert_eq!(
            from_markup_value::<String>(&sources[3].get_at(0)).as_deref(),
            Some("changed")
        );
        let distinct = convert(into_markup_value(native_string_list(Rc::new(
            PredictionListBuffer::new(vec!["changed".into()]),
        ))));
        assert!(!sources[3].ptr_eq(&distinct));
        let invalid = KeyboardItems::Predictions(native_string_list(Rc::new(
            PredictionListBuffer::new(vec![Utf16Text::from_units(vec![0xd800])]),
        )));
        assert!(invalid.try_get_at(0).is_err());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| invalid.get_at(0))).is_err()
        );
        assert!(
            KeyboardItems::TextKeys(text_view)
                .try_get_at(usize::MAX)
                .is_err()
        );
    });
}

struct Custom {
    count: Cell<i32>,
    gets: Cell<usize>,
}
impl IResourceReadOnlyList<Key> for Custom {
    fn count(&self) -> Result<i32, ResourceTextListError> {
        Ok(self.count.get())
    }
    fn get(&self, _: i32) -> Result<Option<Key>, ResourceTextListError> {
        self.gets.set(self.gets.get() + 1);
        Err(ResourceTextListError::Modified)
    }
    fn iter(&self) -> Result<ResourceItems<Key>, ResourceTextListError> {
        panic!("items access must be indexed and lazy")
    }
}
#[test]
fn invalid_custom_counts_reader_failures_and_bounds_are_explicit() {
    headless::run(|| {
        ferroboard_controls::register_types();
        let custom = Rc::new(Custom {
            count: Cell::new(-4),
            gets: Cell::new(0),
        });
        let view =
            KeyReadOnlyList::from_original(ResourceReadOnlyList::from_custom(custom.clone()));
        let source = convert(into_markup_value(view.clone()));
        assert_eq!(custom.gets.get(), 0, "conversion must not read items");
        let typed = source.downcast_ref::<KeyboardItems>().unwrap();
        assert!(
            typed
                .try_count()
                .unwrap_err()
                .to_string()
                .contains("negative")
        );
        let panic =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.count())).unwrap_err();
        assert!(panic.downcast_ref::<MarkupInvokeError>().is_some());
        assert_eq!(custom.gets.get(), 0);
        custom.count.set(1);
        assert!(typed.try_get_at(1).is_err());
        assert_eq!(custom.gets.get(), 0);
        assert!(
            typed
                .try_get_at(0)
                .unwrap_err()
                .to_string()
                .contains(&ResourceTextListError::Modified.to_string())
        );
        assert_eq!(custom.gets.get(), 1);
    });
}

#[test]
fn replacing_and_dropping_sources_releases_reader_and_item_ownership() {
    headless::run(|| {
        ferroboard_controls::register_types();
        let key: TextKey = Rc::new(TextKeyData::default());
        let weak_key = Rc::downgrade(&key);
        let list =
            TextKeyReadOnlyList::from_original(ResourceObjectList::new(vec![key.clone()]).into());
        let source = convert(into_markup_value(list));
        drop(key);
        assert!(weak_key.upgrade().is_some());
        let control = ferroui_controls::ItemsControl::new();
        control.set_items_source(Some(source.clone()));
        control.set_items_source(Some(source.clone()));
        drop(source);
        assert!(weak_key.upgrade().is_some());
        control.set_items_source(None);
        assert!(weak_key.upgrade().is_none());
        let custom = Rc::new(Custom {
            count: Cell::new(0),
            gets: Cell::new(0),
        });
        let weak = Rc::downgrade(&custom);
        let source = convert(into_markup_value(KeyReadOnlyList::from_original(
            ResourceReadOnlyList::from_custom(custom.clone()),
        )));
        drop(custom);
        assert!(weak.upgrade().is_some());
        drop(source);
        assert!(weak.upgrade().is_none());
    });
}
