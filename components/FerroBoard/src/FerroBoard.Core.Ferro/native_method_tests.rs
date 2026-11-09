use crate::*;
use ferroboard_core::{keys::KeyType, layouts::*, text::*};
use ferroui_base::metadata::{
    MarkupType, MarkupTyped, MarkupValue, from_markup_value, into_markup_value,
};
use std::{rc::Rc, sync::Arc};

fn invoke(descriptor: &MarkupType, name: &str, arity: usize, args: &[MarkupValue]) -> MarkupValue {
    (descriptor
        .find_methods(name)
        .find(|m| m.parameters.len() == arity)
        .unwrap()
        .invoke)(args)
    .unwrap()
}

#[test]
fn explicit_factory_and_with_overloads_keep_defaults_and_virtual_dispatch() {
    register_types();
    let descriptor = TextKeyDataMetadata::MARKUP;
    let function: TextKey = from_markup_value(&invoke(
        descriptor,
        "Function",
        1,
        &[into_markup_value(-7_i32)],
    ))
    .unwrap();
    assert_eq!(function.data().key_type(), KeyType::Function);
    assert_eq!(function.data().code(), -7);
    assert_eq!(
        function.data().label(),
        TextKeyData::function(-7, KeyType::Function, None).label()
    );
    let function: TextKey = from_markup_value(&invoke(
        descriptor,
        "Function",
        2,
        &[
            into_markup_value(-7_i32),
            into_markup_value(KeyType::Character),
        ],
    ))
    .unwrap();
    assert_eq!(function.data().key_type(), KeyType::Character);
    let key: TextKey = from_markup_value(&invoke(
        descriptor,
        "Char",
        1,
        &[into_markup_value(Some(Utf16Text::from("a")))],
    ))
    .unwrap();
    assert_eq!(key.data().code(), 97);
    assert!(key.data().popup().is_none());
    let popup = Rc::new(PopupSet::default());
    let key: TextKey = from_markup_value(&invoke(
        descriptor,
        "Char",
        2,
        &[
            into_markup_value(Some(Utf16Text::from("a"))),
            into_markup_value(Some(popup.clone())),
        ],
    ))
    .unwrap();
    assert!(Rc::ptr_eq(&key.data().popup().unwrap(), &popup));
    let objects = [
        (TextKeyDataMetadata::MARKUP, into_markup_value(key)),
        (
            AutoTextKeyDataMetadata::MARKUP,
            into_markup_value(Rc::new(AutoTextKeyData {
                base: TextKeyData::default(),
            })),
        ),
        (
            MultiTextKeyDataMetadata::MARKUP,
            into_markup_value(Rc::new(MultiTextKeyData::default())),
        ),
    ];
    for (descriptor, original) in objects {
        let changes = [
            into_markup_value(Some(123_i32)),
            into_markup_value(Some(Utf16Text::from("z"))),
            into_markup_value(Some(KeyType::Function)),
            into_markup_value(Some(popup.clone())),
            into_markup_value(Some(8_i32)),
        ];
        for count in 0..=5 {
            let mut arguments = vec![original.clone()];
            arguments.extend_from_slice(&changes[..count]);
            let actual: TextKey =
                from_markup_value(&invoke(descriptor, "With", count, &arguments)).unwrap();
            let before: TextKey = from_markup_value(&original).unwrap();
            let expected = before.with(KeyChanges {
                code: (count >= 1).then_some(123),
                label: (count >= 2).then(|| "z".into()),
                key_type: (count >= 3).then_some(KeyType::Function),
                popup: (count >= 4).then(|| popup.clone()),
                group_id: (count >= 5).then_some(8),
            });
            assert_eq!(actual.data().code(), expected.data().code());
            assert_eq!(actual.data().label(), expected.data().label());
            assert_eq!(actual.data().key_type(), expected.data().key_type());
            assert_eq!(actual.data().group_id(), expected.data().group_id());
            assert!(actual.data().popup() == expected.data().popup());
            assert_eq!(
                (&*actual as &dyn std::any::Any).type_id(),
                (&*expected as &dyn std::any::Any).type_id()
            );
        }
    }
}

#[test]
fn dictionary_default_overloads_keep_language_errors_and_actual_arc_storage() {
    register_types();
    let source = Some(Utf16Text::from("hello 42\nworld 7"));
    let word: Arc<WordDictionary> = from_markup_value(&invoke(
        WordDictionaryMetadata::MARKUP,
        "FromText",
        1,
        &[into_markup_value(source.clone())],
    ))
    .unwrap();
    assert_eq!(word.language_tag(), Some("en".into()));
    assert_eq!(word.get_frequency(&"hello".into()).unwrap(), 42);
    let explicit: Arc<WordDictionary> = from_markup_value(&invoke(
        WordDictionaryMetadata::MARKUP,
        "FromText",
        2,
        &[
            into_markup_value(source),
            into_markup_value(None::<Utf16Text>),
        ],
    ))
    .unwrap();
    assert!(explicit.language_tag().is_none());
    let value = into_markup_value(word.clone());
    assert!(Arc::ptr_eq(
        &word,
        &from_markup_value::<Arc<WordDictionary>>(&value).unwrap()
    ));
    let method = WordDictionaryMetadata::MARKUP
        .find_methods("FromText")
        .find(|m| m.parameters.len() == 1)
        .unwrap();
    assert!((method.invoke)(&[into_markup_value(None::<Utf16Text>)]).is_err());
    let json: Arc<WordDictionary> = from_markup_value(&invoke(
        WordDictionaryMetadata::MARKUP,
        "FromJson",
        1,
        &[into_markup_value(Some(Utf16Text::from("{\"hello\":42}")))],
    ))
    .unwrap();
    assert_eq!(json.language_tag(), Some("en".into()));
    assert_eq!(json.get_frequency(&"hello".into()).unwrap(), 42);
}

#[test]
fn layout_overloads_preserve_omitted_number_row_and_geometry_options() {
    use ferroboard_core::keys::KeyboardMode;
    register_types();
    let computer = Rc::new(LayoutComputer::new(Rc::new(
        KeyboardResources::create_default().unwrap(),
    )));
    let subtype = Arc::new(Subtype::default());
    let args = [
        into_markup_value(computer.clone()),
        into_markup_value(KeyboardMode::Characters),
        into_markup_value(Some(subtype)),
    ];
    let omitted: Rc<ComputedKeyboard> =
        from_markup_value(&invoke(LayoutComputerMetadata::MARKUP, "Compute", 2, &args)).unwrap();
    let mut explicit = args.to_vec();
    explicit.push(into_markup_value(false));
    let explicit: Rc<ComputedKeyboard> = from_markup_value(&invoke(
        LayoutComputerMetadata::MARKUP,
        "Compute",
        3,
        &explicit,
    ))
    .unwrap();
    assert_eq!(omitted.row_count(), explicit.row_count());
    omitted
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    explicit
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    let _ = invoke(
        ComputedKeyboardMetadata::MARKUP,
        "Layout",
        2,
        &[
            into_markup_value(omitted.clone()),
            into_markup_value(480.5_f64),
            into_markup_value(180.25_f64),
        ],
    );
    explicit.layout(480.5, 180.25, None).unwrap();
    let bounds = |v: &ComputedKeyboard| {
        v.keys()
            .map(|key| key.unwrap().unwrap())
            .map(|key| {
                (
                    key.code(),
                    key.is_visible(),
                    key.touch_bounds(),
                    key.visible_bounds(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(bounds(&omitted), bounds(&explicit));
}
