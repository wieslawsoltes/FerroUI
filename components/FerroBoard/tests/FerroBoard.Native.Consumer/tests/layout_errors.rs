//! Native layout loading composes with standard Rust error handling and retryable I/O.
use ferroboard_core::{
    layouts::{
        ComponentName, KeyboardLayout, KeyboardResources, LayoutJson, LayoutJsonError, LayoutType,
        ResourceError,
    },
    text::Utf16Text,
};
use std::{cell::Cell, error::Error, io, rc::Rc};

type NativeResult<T> = Result<T, Box<dyn Error>>;

fn roundtrip(json: &str) -> NativeResult<String> {
    let layout = LayoutJson::read_layout_document(json, "consumer".into())?;
    Ok(LayoutJson::write_layout_document(&layout, false)?)
}

fn load(resources: &KeyboardResources, name: &ComponentName) -> NativeResult<Rc<KeyboardLayout>> {
    resources
        .get_layout(LayoutType::Characters, name)?
        .ok_or_else(|| "fixture layout is missing".into())
}

#[test]
fn independent_layout_json_roundtrip_and_errors_use_standard_question_mark() -> NativeResult<()> {
    let encoded =
        roundtrip(r#"{"type":"characters","id":"native-json","arrangement":[["a","β"]]}"#)?;
    let decoded = LayoutJson::read_layout_document(&encoded, "consumer".into())?;
    assert_eq!(decoded.id(), Some("native-json".into()));
    assert_eq!(roundtrip(&encoded)?, encoded);

    let error = roundtrip(r#"{"type":"characters","id":"broken","arrangement":true}"#).unwrap_err();
    let json = error.downcast_ref::<LayoutJsonError>().unwrap();
    assert!(json.0.contains("Layout arrangement must be a JSON array"));
    assert_eq!(json.to_string(), json.0);
    let wrapped = ResourceError::Json(json.clone());
    let cause = wrapped.source().unwrap();
    let ResourceError::Json(stored) = &wrapped else {
        unreachable!()
    };
    assert!(std::ptr::eq(
        cause.downcast_ref::<LayoutJsonError>().unwrap(),
        stored
    ));
    assert_eq!(wrapped.to_string(), json.0);
    Ok(())
}

#[test]
fn independent_lazy_layout_io_preserves_typed_denial_retry_and_cached_identity() -> NativeResult<()>
{
    let resources = KeyboardResources::create_empty();
    let manifest = LayoutJson::read_extension_manifest(
        r#"{"meta":{"id":"io-consumer"},"layouts":{"characters":[{"id":"test"}]}}"#,
    )?;
    let ready = Rc::new(Cell::new(false));
    let attempts = Rc::new(Cell::new(0));
    let can_read = ready.clone();
    let count = attempts.clone();
    resources.add_extension(
        Rc::new(manifest),
        Rc::new(move |path| {
            assert_eq!(path, &Utf16Text::from("layouts/characters/test.json"));
            count.set(count.get() + 1);
            if can_read.get() {
                Ok(Some(r#"[["β"]]"#.into()))
            } else {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture denied").into())
            }
        }),
    )?;
    assert_eq!(attempts.get(), 0);
    let name = ComponentName::new("io-consumer", "test");
    let error = match load(&resources, &name) {
        Err(error) => error,
        Ok(_) => panic!("denied provider must fail"),
    };
    let resource = error.downcast_ref::<ResourceError>().unwrap();
    let denial = resource
        .source()
        .unwrap()
        .downcast_ref::<io::Error>()
        .unwrap();
    assert_eq!(denial.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(resource.to_string(), "fixture denied");
    assert_eq!(attempts.get(), 1);

    ready.set(true);
    let loaded = load(&resources, &name)?;
    assert!(Rc::ptr_eq(&loaded, &load(&resources, &name)?));
    assert_eq!(loaded.id(), Some("test".into()));
    assert_eq!(attempts.get(), 2);
    Ok(())
}
