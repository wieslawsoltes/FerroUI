use ferroboard_core::text::*;
use ferroboard_core_ferro::{native_string_list, native_suggestion_provider};
use std::{cell::Cell, rc::Rc};
// Neither implementation needs PartialEq or an SDK AnyValue implementation.
struct Provider {
    calls: Cell<usize>,
}
impl ISuggestionProvider for Provider {
    fn suggest(&self, _: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        self.calls.set(self.calls.get() + 1);
        Ok(None)
    }
    fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
}
struct Predictions(PredictionListBuffer);
impl PredictionSource for Predictions {
    fn len(&self) -> SuggestionResult<usize> {
        self.0.len()
    }
    fn get(&self, index: usize) -> SuggestionResult<Option<Utf16Text>> {
        self.0.get(index)
    }
}
#[test]
fn native_traits_accept_non_equatable_custom_objects_and_retain_their_actual_rc() {
    let provider = Rc::new(Provider {
        calls: Cell::new(0),
    });
    let original: Rc<dyn ISuggestionProvider> = provider.clone();
    let weak = Rc::downgrade(&provider);
    let view = native_suggestion_provider(original.clone());
    assert!(Rc::ptr_eq(&view.interface(), &original));
    assert!(Rc::ptr_eq(&view.downcast::<Provider>().unwrap(), &provider));
    assert!(view.suggest(None).unwrap().is_none());
    assert_eq!(provider.calls.get(), 1);
    drop(original);
    drop(provider);
    assert!(weak.upgrade().is_some());
    drop(view);
    assert!(weak.upgrade().is_none());
    let list = Rc::new(Predictions(PredictionListBuffer::new(vec!["word".into()])));
    let original: PredictionList = list.clone();
    let view = native_string_list(original.clone());
    assert!(Rc::ptr_eq(&view.interface(), &original));
    assert_eq!(view.get(0).unwrap(), Some("word".into()));
    assert!(Rc::ptr_eq(&view.downcast::<Predictions>().unwrap(), &list));
}

struct Evaluator(Cell<ferroboard_core::keys::KeyboardMode>);
impl ferroboard_core::layouts::IKeyEvaluator for Evaluator {
    fn mode(&self) -> ferroboard_core::keys::KeyboardMode {
        self.0.get()
    }
    fn shift_state(&self) -> ferroboard_core::keys::ShiftState {
        ferroboard_core::keys::ShiftState::Unshifted
    }
    fn variation(&self) -> ferroboard_core::keys::KeyVariation {
        ferroboard_core::keys::KeyVariation::All
    }
    fn direction(&self) -> ferroboard_core::keys::LayoutDirection {
        ferroboard_core::keys::LayoutDirection::Ltr
    }
    fn is_char_half_width(&self) -> bool {
        false
    }
    fn is_kana_kata(&self) -> bool {
        false
    }
    fn culture(&self) -> Option<TextCulture> {
        None
    }
}
#[test]
fn evaluator_metadata_owns_the_actual_non_equatable_native_trait_implementor() {
    use ferroboard_core::keys::KeyboardMode;
    let evaluator = Rc::new(Evaluator(Cell::new(KeyboardMode::Characters)));
    let original: Rc<dyn ferroboard_core::layouts::IKeyEvaluator> = evaluator.clone();
    let view = ferroboard_core_ferro::native_key_evaluator(evaluator.clone());
    let second = ferroboard_core_ferro::native_key_evaluator(evaluator.clone());
    assert!(view == second);
    assert!(Rc::ptr_eq(&view.interface(), &original));
    assert!(Rc::ptr_eq(
        &view.downcast::<Evaluator>().unwrap(),
        &evaluator
    ));
    evaluator.0.set(KeyboardMode::Symbols);
    assert_eq!(view.mode(), KeyboardMode::Symbols);
    let weak = Rc::downgrade(&evaluator);
    drop(evaluator);
    drop(original);
    drop(second);
    assert!(weak.upgrade().is_some());
    drop(view);
    assert!(weak.upgrade().is_none());
}

#[test]
fn downstream_custom_suggestion_sources_use_native_indices_and_checked_markup_counts() {
    use ferroboard_controls::items_source::KeyboardItems;
    use ferroboard_core_ferro::SuggestionReadOnlyList;
    use std::cell::RefCell;
    struct Source {
        count: Cell<usize>,
        values: SharedSuggestions,
    }
    impl SuggestionSource for Source {
        fn len(&self) -> SuggestionResult<usize> {
            Ok(self.count.get())
        }
        fn get(&self, index: usize) -> SuggestionResult<SuggestionValue> {
            self.values
                .borrow()
                .get(index)
                .cloned()
                .ok_or(SuggestionError::SuggestionIndex(index))
        }
    }
    let candidate = Rc::new(Suggestion::new("native".into(), SuggestionKind::Completion));
    let source = Rc::new(Source {
        count: Cell::new(1),
        values: Rc::new(RefCell::new(vec![Some(candidate.clone())])),
    });
    let weak = Rc::downgrade(&source);
    let list = SuggestionList::from_custom(source.clone());
    assert!(Rc::ptr_eq(
        &list.iter().unwrap().next().unwrap().unwrap().unwrap(),
        &candidate
    ));
    let view = SuggestionReadOnlyList::from_original(list.clone());
    assert_eq!(view.count().unwrap(), 1);
    assert!(view.get(-1).is_err());
    assert!(Rc::ptr_eq(&view.get(0).unwrap().unwrap(), &candidate));
    let items = KeyboardItems::Suggestions(view.clone());
    assert_eq!(items.try_count().unwrap(), 1);
    source.count.set(usize::MAX);
    assert!(
        view.count().is_err(),
        "oversize native counts must not wrap into markup i32"
    );
    assert!(items.try_count().is_err());
    drop(source);
    drop(list);
    drop(view);
    assert!(
        weak.upgrade().is_some(),
        "native items view owns its actual source"
    );
    drop(items);
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_prediction_sources_retain_ownership_and_reject_invalid_markup_dimensions() {
    use ferroboard_controls::items_source::KeyboardItems;
    struct Source {
        count: Cell<usize>,
        reads: Cell<usize>,
    }
    impl PredictionSource for Source {
        fn len(&self) -> SuggestionResult<usize> {
            Ok(self.count.get())
        }
        fn get(&self, index: usize) -> SuggestionResult<PredictionValue> {
            self.reads.set(self.reads.get() + 1);
            if index == 0 {
                Ok(None)
            } else {
                Err(SuggestionError::PredictionIndex(index))
            }
        }
    }
    let source = Rc::new(Source {
        count: Cell::new(1),
        reads: Cell::new(0),
    });
    let weak = Rc::downgrade(&source);
    let original: PredictionList = source.clone();
    let view = native_string_list(original.clone());
    assert!(Rc::ptr_eq(&view.interface(), &original));
    assert_eq!(view.count().unwrap(), 1);
    assert!(view.get(-1).is_err());
    assert_eq!(source.reads.get(), 0);
    assert!(view.get(0).unwrap().is_none());
    let items = KeyboardItems::Predictions(view.clone());
    assert_eq!(items.try_count().unwrap(), 1);
    source.count.set(usize::MAX);
    assert!(view.count().is_err());
    assert!(items.try_count().is_err());
    source.count.set(1);
    let values: Vec<_> = original
        .iter()
        .unwrap()
        .collect::<SuggestionResult<_>>()
        .unwrap();
    assert_eq!(values, vec![None]);
    drop(source);
    drop(original);
    drop(view);
    assert!(weak.upgrade().is_some());
    drop(items);
    assert!(weak.upgrade().is_none());
}
