//! Native closure ownership and the public request metadata boundary.
use ferroboard_core::text::{KeyProximity, SuggestionRequest, Utf16Text};
use ferroboard_core_ferro::{KeyProximityMetadata, KeyProximityRef, SuggestionRequestMetadata};
use ferroui_base::metadata::{MarkupTyped, from_markup_value, into_markup_value};
use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

#[test]
fn metadata_preserves_native_callback_identity_and_reports_invalid_text_and_locale() {
    ferroboard_core_ferro::register_types();
    let m = SuggestionRequestMetadata::MARKUP;
    assert_eq!(m.name, "SuggestionRequest");
    assert_eq!(m.properties.len(), 6);
    assert!(m.methods.is_empty());
    assert!(KeyProximityMetadata::MARKUP.methods.is_empty());
    let value = (m.constructors[0].invoke)(&[]).unwrap();
    let request: SuggestionRequest = from_markup_value(&value).unwrap();
    assert_eq!(request.word(), Some(Utf16Text::default()));
    assert_eq!(request.max_count(), 3);
    assert!(request.allow_auto_correct());
    let callback: KeyProximity = Rc::new(|a, b| if a == b { 0.0 } else { 0.5 });
    let p = m.find_property("KeyProximity").unwrap();
    (p.set.unwrap())(&[
        value.clone(),
        into_markup_value(Some(KeyProximityRef::new(callback.clone()))),
    ])
    .unwrap();
    let carried: Option<KeyProximityRef> =
        from_markup_value(&(p.get.unwrap())(std::slice::from_ref(&value)).unwrap()).unwrap();
    assert!(Rc::ptr_eq(&callback, &request.key_proximity().unwrap()));
    assert!(Rc::ptr_eq(&callback, &carried.unwrap().callback()));
    let culture = m.find_property("Culture").unwrap();
    let original = request.culture().unwrap();
    assert!(
        (culture.set.unwrap())(&[
            value.clone(),
            into_markup_value(Some("invalid locale !!!".to_owned()))
        ])
        .is_err()
    );
    assert_eq!(request.culture(), Some(original));
    for name in ["Word", "PrecedingText"] {
        let invalid = Some(Utf16Text::from_units(vec![0xd800]));
        if name == "Word" {
            request.set_word(invalid.clone())
        } else {
            request.set_preceding_text(invalid.clone())
        }
        let p = m.find_property(name).unwrap();
        assert!((p.get.unwrap())(std::slice::from_ref(&value)).is_err());
        assert_eq!(
            if name == "Word" {
                request.word()
            } else {
                request.preceding_text()
            },
            invalid
        );
        (p.set.unwrap())(&[value.clone(), None]).unwrap();
        assert_eq!(
            if name == "Word" {
                request.word()
            } else {
                request.preceding_text()
            },
            None
        );
    }
    (culture.set.unwrap())(&[value, None]).unwrap();
    assert!(request.culture().is_none());
}

#[test]
fn replacing_a_closure_releases_captures_after_the_request_borrow_ends() {
    struct Capture {
        request: ferroboard_core::text::WeakSuggestionRequest,
        released: Rc<Cell<bool>>,
    }
    impl Drop for Capture {
        fn drop(&mut self) {
            let request = self.request.upgrade().unwrap();
            assert!(request.key_proximity().is_none());
            request.set_word(Some("released".into()));
            self.released.set(true);
        }
    }
    let request = SuggestionRequest::default();
    let released = Rc::new(Cell::new(false));
    let capture = Capture {
        request: request.downgrade(),
        released: released.clone(),
    };
    request.set_key_proximity(Some(Rc::new(move |_, _| {
        let _keep = &capture;
        0.5
    })));
    request.set_key_proximity(None);
    assert!(released.get());
    assert_eq!(request.word(), Some("released".into()));
}

#[test]
fn rust_closure_panics_keep_the_original_payload() {
    struct Payload(u32);
    let callback: KeyProximity = Rc::new(|_, _| std::panic::panic_any(Payload(42)));
    let request = SuggestionRequest::default();
    request.set_key_proximity(Some(callback.clone()));
    let alias = KeyProximityRef::new(callback);
    let panic = catch_unwind(AssertUnwindSafe(|| alias.callback()(65, 66))).unwrap_err();
    assert_eq!(panic.downcast_ref::<Payload>().unwrap().0, 42);
    assert!(request.key_proximity().is_some());
    request.set_max_count(5);
    assert_eq!(request.max_count(), 5);
}
