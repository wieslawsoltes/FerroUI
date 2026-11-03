use ferroboard_core::layouts::{CodePointArray, KeyDataError};
#[test]
fn fixed_codepoint_storage_shares_mutations_and_rejects_invalid_indices() {
    let values = CodePointArray::new(vec![65, 0x1f600]);
    let alias = values.clone();
    let worker = alias.clone();
    std::thread::spawn(move || worker.set(0, 66).unwrap())
        .join()
        .unwrap();
    assert!(values.ptr_eq(&alias));
    assert_eq!(values.snapshot(), vec![66, 0x1f600]);
    assert_eq!(values.len(), 2);
    for index in [-1, 2, i32::MAX] {
        assert!(matches!(
            values.get(index),
            Err(KeyDataError::ArrayIndexOutOfRange)
        ));
        assert!(matches!(
            values.set(index, 0),
            Err(KeyDataError::ArrayIndexOutOfRange)
        ));
    }
    let fresh = CodePointArray::new(values.snapshot());
    assert!(!fresh.ptr_eq(&values));
    alias.set(1, -1).unwrap();
    assert_eq!(
        values.get(1).unwrap(),
        -1,
        "validation happens at key output, not storage assignment"
    );
    assert_eq!(fresh.get(1).unwrap(), 0x1f600);
}
