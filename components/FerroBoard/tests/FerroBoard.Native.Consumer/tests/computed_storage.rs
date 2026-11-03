//! The compiled-consumer facades keep the actual native computed buffers.
use ferroboard_core::{keys::KeyboardMode, layouts::*};
use ferroboard_core_ferro::computed_collection_metadata::{
    ComputedKeyQuery, ComputedKeyReadOnlyList, ComputedRowsReadOnlyList,
};
use std::rc::Rc;

#[test]
fn native_computed_views_share_buffers_and_nullable_snapshots_fail_explicitly() {
    let board = LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
        .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
        .unwrap();
    board
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    let rows = board.rows();
    let rows_view = ComputedRowsReadOnlyList::from_rows(rows.clone());
    assert!(rows_view.source().ptr_eq(&rows));
    let first = rows.get(0).unwrap();
    let view = ComputedKeyReadOnlyList::from_row(first.clone());
    assert!(view.source().ptr_eq(&first));
    assert!(rows_view.get(0).unwrap().unwrap().source().ptr_eq(&first));
    let key = first.get(0).unwrap();
    let count = first.len();
    first.push_nullable(None);
    assert_eq!(view.count().unwrap() as usize, count + 1);
    assert!(view.get(count as i32).unwrap().is_none());
    assert!(matches!(
        first.snapshot(),
        Err(ResourceTextListError::NullReference)
    ));
    let snapshot = ComputedKeyQuery::from_query(board.keys())
        .snapshot()
        .unwrap();
    assert!(Rc::ptr_eq(&snapshot.source().get(0).unwrap(), &key));
    assert!(view.get(-1).is_err());
    assert!(view.get(i32::MAX).is_err());
    assert!(first.get(usize::MAX).is_err());
    assert!(first.insert_nullable(usize::MAX, None).is_err());
    assert!(first.remove(count).unwrap().is_none());
    let snapshot = first.snapshot().unwrap();
    assert_eq!(snapshot.len(), count);
    assert!(Rc::ptr_eq(&snapshot[0], &key));
}
