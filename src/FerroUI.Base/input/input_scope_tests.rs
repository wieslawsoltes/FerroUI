use super::text_input::{TextInputContentType, TextInputOptions};
use super::{InputScope, InputScopeName, InputScopeNameValue as Scope};
use crate::metadata::{from_markup_value, into_markup_value, MarkupTyped};
use crate::{StaticType, StyledElement};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn input_scope_enum_preserves_every_native_value_and_rejects_gaps() {
    let expected = [
        0, 1, 5, 7, 20, 21, 23, 24, 25, 28, 29, 31, 32, 33, 34, 35, 37, 38, 39, 40, 41, 44, 45, 46,
        47, 48, 49, 50, 51, 52, 53, 54, 55, 57, 58, 59, 60, 62, 63, 64, 65, 67, 68,
    ];
    assert_eq!(Scope::ALL.map(|value| value as i32), expected);
    for value in -1..=69 {
        assert_eq!(
            Scope::from_raw(value).is_some(),
            expected.contains(&value),
            "{value}"
        );
    }
    for scope in Scope::ALL {
        assert_eq!(Scope::parse(&format!("{scope:?}")), Ok(scope));
        assert_eq!(Scope::parse(&(scope as i32).to_string()), Ok(scope));
    }
    assert!(Scope::parse("68,0").is_err());
    assert!(Scope::parse("-1").is_err());
    assert!(Scope::parse("unknown").is_err());
}

#[test]
fn input_scope_defaults_and_string_conversion_match_native_object_creation() {
    assert_eq!(InputScopeName::new().name_value(), Scope::Default);
    let empty = InputScope::new();
    assert_eq!(empty.names().count(), 0);
    for scope in Scope::ALL {
        let parsed = InputScope::parse(&format!("{scope:?}")).unwrap();
        assert_eq!(&*parsed.snapshot(), &[scope]);
        assert_eq!(
            InputScopeName::parse(&format!("{scope:?}"))
                .unwrap()
                .name_value(),
            scope
        );
    }
    for invalid in ["", "-1", "2", "EmailSmtpAddress,Url", "does-not-exist"] {
        assert!(InputScope::parse(invalid).is_err(), "{invalid}");
        assert!(InputScopeName::parse(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn input_scope_names_are_shared_ordered_and_snapshots_are_immutable() {
    let scope = InputScope::new();
    let names = scope.names();
    let first = InputScopeName::with_name_value(Scope::EmailSmtpAddress);
    let second = InputScopeName::with_name_value(Scope::Url);
    names.add(first.clone());
    names.add(second.clone());
    let before = scope.snapshot();
    assert_eq!(scope.names().count(), 2);
    assert_eq!(&*before, &[Scope::EmailSmtpAddress, Scope::Url]);
    first.set_name_value(Scope::Number);
    names.move_item(1, 0);
    assert_eq!(&*scope.snapshot(), &[Scope::Url, Scope::Number]);
    assert_eq!(&*before, &[Scope::EmailSmtpAddress, Scope::Url]);
}

#[test]
fn input_scope_observes_duplicate_names_once_and_unsubscribes_removed_names() {
    let scope = InputScope::new();
    let shared = InputScopeName::with_name_value(Scope::Number);
    scope.names().add(shared.clone());
    scope.names().add(shared.clone());
    let changes = Rc::new(Cell::new(0));
    let captured = changes.clone();
    let subscription = scope.changed(move || captured.set(captured.get() + 1));
    shared.set_name_value(Scope::Url);
    assert_eq!(changes.get(), 1);
    scope.names().remove_at(0);
    shared.set_name_value(Scope::Search);
    assert_eq!(changes.get(), 3);
    scope.names().clear();
    shared.set_name_value(Scope::Text);
    assert_eq!(changes.get(), 4);
    subscription.dispose();
    scope.names().add(shared);
    assert_eq!(changes.get(), 4);
}

#[test]
fn input_scope_callbacks_can_mutate_the_list_and_shared_name() {
    let scope = InputScope::new();
    let first = InputScopeName::new();
    scope.names().add(first.clone());
    let snapshots = Rc::new(RefCell::new(Vec::new()));
    let collected = snapshots.clone();
    let weak = scope.downgrade();
    let subscription = scope.changed(move || {
        let scope = weak.upgrade().unwrap();
        let snapshot = scope.snapshot();
        collected.borrow_mut().push(snapshot.clone());
        if &*snapshot == [Scope::EmailSmtpAddress] {
            scope
                .names()
                .add(InputScopeName::with_name_value(Scope::Url));
        }
    });
    first.set_name_value(Scope::EmailSmtpAddress);
    assert_eq!(snapshots.borrow().len(), 2);
    assert_eq!(&*scope.snapshot(), &[Scope::EmailSmtpAddress, Scope::Url]);
    subscription.dispose();
}

#[test]
fn input_scope_list_and_items_do_not_retain_the_owner() {
    let scope = InputScope::new();
    let names = scope.names();
    let name = InputScopeName::new();
    names.add(name.clone());
    let weak = scope.downgrade();
    let subscription = scope.changed(|| {});
    drop(scope);
    assert!(weak.upgrade().is_none());
    assert!(!names.has_collection_changed_subscribers());
    name.set_name_value(Scope::Chat);
    names.add(InputScopeName::new());
    subscription.dispose();
}

#[test]
fn input_scope_transport_preserves_legacy_hints_and_first_name_precedence() {
    let alpha = InputScope::parse("Alpha").unwrap();
    assert_eq!(&*alpha.snapshot(), &[Scope::AlphanumericHalfWidth]);
    assert_eq!(
        &*InputScope::parse(" aLpHa ").unwrap().snapshot(),
        &[Scope::AlphanumericHalfWidth]
    );
    assert_eq!(
        Scope::from(TextInputContentType::Alpha),
        Scope::AlphanumericHalfWidth
    );
    assert_eq!(
        TextInputOptions {
            input_scope_names: Some(alpha.snapshot()),
            ..Default::default()
        }
        .effective_content_type(),
        TextInputContentType::Alpha
    );
    let mut options = TextInputOptions {
        content_type: TextInputContentType::Email,
        ..Default::default()
    };
    assert_eq!(
        options.effective_content_type(),
        TextInputContentType::Email
    );
    options.input_scope_names = Some(Rc::from([]));
    assert_eq!(
        options.effective_content_type(),
        TextInputContentType::Email
    );
    options.input_scope_names = Some(Rc::from([Scope::Default, Scope::Search]));
    assert_eq!(options.effective_input_scope(), Some(Scope::Default));
    assert_eq!(
        options.effective_content_type(),
        TextInputContentType::Normal
    );
    assert_eq!(options.content_type, TextInputContentType::Email);
    options.input_scope_names = Some(Rc::from([Scope::Search, Scope::Default]));
    assert_eq!(
        options.effective_content_type(),
        TextInputContentType::Search
    );
}

#[test]
fn input_scope_transport_is_inherited_without_retaining_live_objects() {
    let parent = StyledElement::new();
    let child = StyledElement::new();
    child.set_inheritance_parent(Some(parent.clone().upcast()));
    let scope = InputScope::with_name_value(Scope::Hiragana);
    TextInputOptions::set_input_scope_names(&parent, Some(scope.snapshot()));
    let options = TextInputOptions::from_styled_element(&child);
    assert_eq!(options.effective_input_scope(), Some(Scope::Hiragana));
    scope.names().get(0).set_name_value(Scope::Chat);
    assert_eq!(options.effective_input_scope(), Some(Scope::Hiragana));
    TextInputOptions::set_input_scope_names(&child, Some(Rc::from([Scope::Url])));
    assert_eq!(
        TextInputOptions::from_styled_element(&child).effective_input_scope(),
        Some(Scope::Url)
    );
    child.clear_value(TextInputOptions::input_scope_names_property());
    assert_eq!(
        TextInputOptions::from_styled_element(&child).effective_input_scope(),
        Some(Scope::Hiragana)
    );
}

#[test]
fn input_scope_metadata_converts_to_real_objects_and_checked_enum_values() {
    crate::register_types();
    for value in Scope::ALL {
        let markup = InputScope::TYPE.markup().unwrap();
        let converted =
            (markup.parse.unwrap())(&[into_markup_value(format!("{value:?}"))]).unwrap();
        let object = from_markup_value::<crate::Ref<InputScope>>(&converted).unwrap();
        assert_eq!(&*object.snapshot(), &[value]);
        let enum_markup = <Scope as MarkupTyped>::MARKUP;
        assert!(enum_markup.enum_from_value.unwrap()(value as i64).is_some());
    }
    assert!(<Scope as MarkupTyped>::MARKUP.enum_from_value.unwrap()(2).is_none());
    assert_eq!(
        InputScopeName::TYPE.markup().unwrap().content_property,
        Some("NameValue")
    );

    // Names exposes the same complete notifying-list metadata as the other
    // framework collections, including checked Capacity rather than only Add.
    let scope = InputScope::new();
    let names = (InputScope::TYPE
        .markup()
        .unwrap()
        .find_property("Names")
        .unwrap()
        .get
        .unwrap())(&[into_markup_value(scope.clone())])
    .unwrap();
    let list = <crate::collections::FerroList<crate::Ref<InputScopeName>> as MarkupTyped>::MARKUP;
    let capacity = list.find_property("Capacity").unwrap();
    (capacity.set.unwrap())(&[names.clone(), into_markup_value(4i32)]).unwrap();
    assert_eq!(
        from_markup_value::<i32>(&(capacity.get.unwrap())(&[names.clone()]).unwrap()).unwrap(),
        4
    );
    let name = InputScopeName::with_name_value(Scope::EmailSmtpAddress);
    (list.find_methods("Add").next().unwrap().invoke)(&[
        names.clone(),
        into_markup_value(name.clone()),
    ])
    .unwrap();
    assert_eq!(
        from_markup_value::<i32>(
            &(list.find_property("Count").unwrap().get.unwrap())(&[names.clone()]).unwrap()
        )
        .unwrap(),
        1
    );
    assert_eq!(scope.names().get(0), name);
    assert_eq!(&*scope.snapshot(), &[Scope::EmailSmtpAddress]);
    for invalid in [-1, 0] {
        assert!((capacity.set.unwrap())(&[names.clone(), into_markup_value(invalid)]).is_err());
        assert_eq!(&*scope.snapshot(), &[Scope::EmailSmtpAddress]);
    }
}
