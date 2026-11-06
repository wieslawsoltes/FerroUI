//! Native classification errors cross extension, layout and engine boundaries through Result.
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{EngineError, KeyboardEngine, LanguageModels},
    keys::*,
    layouts::*,
    settings::KeyboardSettings,
    text::Utf16Text,
};
use std::{cell::Cell, error::Error, rc::Rc, sync::Arc};
struct KeyFailure {
    base: TextKeyData,
    fail: Cell<bool>,
    output_calls: Cell<usize>,
}
impl KeyFailure {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            base: TextKeyData::from_fields(TextKeyDataFields {
                key_type: KeyType::Character,
                code: 97,
                label: Some("a".into()),
                ..Default::default()
            }),
            fail: Cell::new(false),
            output_calls: Cell::new(0),
        })
    }
}
impl AbstractKeyData for KeyFailure {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        self.output_calls.set(self.output_calls.get() + 1);
        Ok(Some("a".into()))
    }
}
impl ITextKeyData for KeyFailure {
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn produces_text(&self) -> Result<bool, KeyDataError> {
        if self.fail.get() {
            Err(KeyDataError::InvalidCodePoint(0x110000))
        } else {
            Ok(true)
        }
    }
}
fn query(keyboard: &ComputedKeyboard) -> Result<usize, Box<dyn Error>> {
    Ok(keyboard.character_keys()?.len())
}

#[test]
fn character_queries_return_custom_errors_and_recover_without_partial_success() {
    let source = KeyFailure::new();
    let resources = Rc::new(KeyboardResources::create_empty());
    resources.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Numeric,
        "probe".into(),
        "main".into(),
        Rc::new(LayoutArrangement::from_rows(vec![vec![source.clone()]])),
    )));
    let subtype = Arc::new(Subtype::from_values(SubtypeValues {
        layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
            SubtypeLayoutMapValues {
                numeric: ComponentName::new("probe", "main"),
                ..Subtype::shared_default().layouts().unwrap().values()
            },
        ))),
        ..Subtype::shared_default().values()
    }));
    let keyboard = LayoutComputer::new(resources)
        .compute(KeyboardMode::Numeric, subtype, false)
        .unwrap();
    keyboard
        .compute(Rc::new(KeyComputeContext::from_values(
            KeyComputeContextValues {
                mode: KeyboardMode::Numeric,
                ..Default::default()
            },
        )))
        .unwrap();
    let key = keyboard
        .character_keys()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    source.fail.set(true);
    assert_eq!(
        key.is_character_key(),
        Err(KeyDataError::InvalidCodePoint(0x110000))
    );
    let error = query(&keyboard).unwrap_err();
    assert_eq!(
        error.downcast_ref::<KeyDataError>(),
        Some(&KeyDataError::InvalidCodePoint(0x110000))
    );
    source.fail.set(false);
    assert_eq!(query(&keyboard).unwrap(), 1);
}
#[test]
fn classification_failure_propagates_before_engine_text_output_and_allows_recovery() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_glide_typing(false);
    settings.set_auto_correct(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone()).unwrap();
    let key = KeyFailure::new();
    key.fail.set(true);
    assert!(matches!(
        engine.input_key(key.as_ref()),
        Err(EngineError::Key(KeyDataError::InvalidCodePoint(0x110000)))
    ));
    assert!(target.text().is_empty());
    assert_eq!(key.output_calls.get(), 0);
    key.fail.set(false);
    engine.input_key(key.as_ref()).unwrap();
    assert_eq!(target.text(), Utf16Text::from("a"));
    engine.dispose().unwrap();
}

#[test]
fn invalid_rows_return_errors_through_queries_layout_engine_and_hit_testing() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let board = engine.keyboard().unwrap();
    let original = board.rows().get(0).unwrap();
    board.rows().set_nullable(0, None).unwrap();
    assert!(matches!(
        board.keys().next(),
        Some(Err(KeyDataError::Collection(
            ResourceTextListError::NullReference
        )))
    ));
    assert!(matches!(
        board.visible_keys().next(),
        Some(Err(KeyDataError::Collection(
            ResourceTextListError::NullReference
        )))
    ));
    assert!(matches!(
        board.find_key(97),
        Err(KeyDataError::Collection(
            ResourceTextListError::NullReference
        ))
    ));
    assert!(matches!(
        board.hit_test(10.0, 10.0),
        Err(KeyDataError::Collection(
            ResourceTextListError::NullReference
        ))
    ));
    assert!(matches!(
        board.layout(320.5, 180.25, None),
        Err(KeyDataError::Collection(
            ResourceTextListError::ArgumentNull
        ))
    ));
    assert_eq!((board.width(), board.height()), (320.5, 180.25));
    assert!(matches!(
        engine.update_layout(400.5, 200.25, None),
        Err(EngineError::Key(KeyDataError::Collection(
            ResourceTextListError::ArgumentNull
        )))
    ));
    board.rows().set(0, original).unwrap();
    engine.update_layout(400.5, 200.25, None).unwrap();
    assert!(board.find_key(97).unwrap().is_some());
    assert!(board.hit_test(10.0, 10.0).unwrap().is_some());
    engine.dispose().unwrap();
}
