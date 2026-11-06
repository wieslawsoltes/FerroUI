//! Independent consumers see nullable key items and native Result failures.
use ferroboard_core::{keys::KeyboardMode, layouts::*};
use ferroboard_core_ferro::{
    ComputedKeyboardMetadata, computed_collection_metadata::ComputedKeyQuery,
};
use ferroui_base::metadata::{MarkupTyped, into_markup_value};
use std::{error::Error, rc::Rc};

fn first(board: &ComputedKeyboard) -> Result<Option<Option<Rc<ComputedKey>>>, Box<dyn Error>> {
    Ok(board.keys().next().transpose()?)
}

#[test]
fn query_geometry_and_metadata_failures_use_results_and_allow_recovery() {
    ferroboard_core_ferro::register_types();
    let board = Rc::new(
        LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
            .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
            .unwrap(),
    );
    board
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    let original = board.rows().get(0).unwrap();
    board.rows().set_nullable(0, None).unwrap();
    let error = first(&board).err().expect("null row must return an error");
    assert_eq!(
        error.downcast_ref::<KeyDataError>(),
        Some(&KeyDataError::Collection(
            ResourceTextListError::NullReference
        ))
    );
    assert!(
        ComputedKeyQuery::from_query(board.keys())
            .snapshot()
            .is_err()
    );
    for (name, expected) in [
        (
            "Layout",
            KeyDataError::Collection(ResourceTextListError::ArgumentNull),
        ),
        (
            "HitTest",
            KeyDataError::Collection(ResourceTextListError::NullReference),
        ),
        (
            "FindKey",
            KeyDataError::Collection(ResourceTextListError::NullReference),
        ),
    ] {
        let arity = if name == "FindKey" { 1 } else { 2 };
        let method = ComputedKeyboardMetadata::MARKUP
            .find_methods(name)
            .find(|m| m.parameters.len() == arity)
            .unwrap();
        let mut args = vec![into_markup_value(board.clone())];
        if name == "FindKey" {
            args.push(into_markup_value(97_i32));
        } else {
            args.push(into_markup_value(400.5_f64));
            args.push(into_markup_value(150.25_f64));
        }
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (method.invoke)(&args)));
        assert!(
            result.is_ok(),
            "{name} must return its error without unwinding"
        );
        let error = result.unwrap().unwrap_err();
        assert!(
            matches!(error, ferroui_base::metadata::MarkupInvokeError::Failed(message) if message == expected.to_string()),
            "{name} must preserve the native error diagnostic"
        );
    }
    board.rows().set(0, original).unwrap();
    board.layout(400.5, 150.25, None).unwrap();
    assert!(first(&board).unwrap().unwrap().is_some());
    assert!(
        ComputedKeyQuery::from_query(board.keys())
            .snapshot()
            .is_ok()
    );
}
