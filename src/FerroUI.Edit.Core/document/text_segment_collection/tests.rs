use super::*;
use std::cell::Cell;

fn segment(offset: usize, length: usize) -> TextSegment {
    TextSegment::from_range(offset, length).unwrap()
}

#[test]
fn overlap_queries_include_touching_endpoints_and_keep_stable_equal_starts() {
    let collection = TextSegmentCollection::new();
    let a = segment(10, 10);
    let b = segment(20, 0);
    let c = segment(20, 8);
    let d = segment(5, 1);
    for value in [&a, &b, &c, &d] {
        collection.add(value.clone()).unwrap();
    }
    assert_eq!(
        collection.segments(),
        vec![d.clone(), a.clone(), b.clone(), c.clone()]
    );
    assert_eq!(
        collection.find_segments_containing(20),
        vec![a.clone(), b.clone(), c.clone()]
    );
    assert_eq!(
        collection.find_overlapping_segments(6, 4).unwrap(),
        vec![d.clone(), a.clone()]
    );
    assert_eq!(
        collection.find_first_segment_with_start_after(20),
        Some(b.clone())
    );
    assert_eq!(collection.find_first_segment_with_start_after(21), None);
    assert_eq!(collection.get_next_segment(&a).unwrap(), Some(b.clone()));
    assert_eq!(collection.get_previous_segment(&c).unwrap(), Some(b));
    assert_eq!(collection.first_segment(), Some(d));
    assert_eq!(collection.last_segment(), Some(c));
    collection.0.tree.borrow().validate();
}

#[test]
fn replacement_boundaries_zero_length_segments_and_undo_follow_reference_rules() {
    let document = TextDocument::from_text("01234567890123456789");
    let collection = TextSegmentCollection::with_document(&document);
    let values = [
        segment(5, 0),
        segment(5, 3),
        segment(6, 1),
        segment(0, 20),
        segment(8, 2),
    ];
    for value in &values {
        collection.add(value.clone()).unwrap();
    }
    document.insert(5, "xx").unwrap();
    assert_eq!(
        values
            .iter()
            .map(|s| (s.start_offset(), s.length()))
            .collect::<Vec<_>>(),
        [(7, 0), (7, 3), (8, 1), (0, 22), (10, 2)]
    );
    document.undo().unwrap();
    assert_eq!(
        values
            .iter()
            .map(|s| (s.start_offset(), s.length()))
            .collect::<Vec<_>>(),
        [(5, 0), (5, 3), (6, 1), (0, 20), (8, 2)]
    );
    document.replace(5, 3, "z").unwrap();
    // A normal replacement keeps its starting boundary; an interval wholly
    // removed from its interior survives with length zero at the new end.
    assert_eq!(
        values
            .iter()
            .map(|s| (s.start_offset(), s.length()))
            .collect::<Vec<_>>(),
        [(5, 0), (5, 1), (6, 0), (0, 18), (6, 2)]
    );
    assert_eq!(collection.count(), 5);
    collection.0.tree.borrow().validate();
}

#[test]
fn direct_mutation_reindexes_and_notifies_outside_borrows() {
    let collection = TextSegmentCollection::new();
    let a = segment(5, 4);
    let b = segment(10, 5);
    collection.add(a.clone()).unwrap();
    collection.add(b.clone()).unwrap();
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let callback_collection = collection.clone();
    let callback_a = a.clone();
    let subscription = a.segment_changed(move || {
        callback_calls.set(callback_calls.get() + 1);
        assert!(callback_collection.contains(&callback_a));
        assert_eq!(
            callback_collection
                .find_segments_containing(callback_a.start_offset())
                .len(),
            1
        );
    });
    a.set_start_offset(20).unwrap();
    a.set_end_offset(22).unwrap();
    assert_eq!(collection.last_segment(), Some(a.clone()));
    assert_eq!((a.start_offset(), a.length()), (20, 2));
    assert_eq!(calls.get(), 2);
    collection
        .update_offsets_entry(OffsetChangeMapEntry::new(0, 0, 2))
        .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!((a.start_offset(), a.length()), (22, 2));
    subscription.dispose();
    collection.0.tree.borrow().validate();
}

#[test]
fn ownership_disconnect_clear_and_drop_preserve_standalone_positions() {
    let document = TextDocument::from_text("0123456789");
    let collection = TextSegmentCollection::with_document(&document);
    let value = segment(3, 2);
    collection.add(value.clone()).unwrap();
    assert_eq!(
        collection.add(value.clone()),
        Err(SegmentError::AlreadyConnected)
    );
    assert_eq!(
        collection.update_offsets_entry(OffsetChangeMapEntry::new(0, 0, 1)),
        Err(SegmentError::AutomaticUpdates)
    );
    assert!(matches!(
        value.set_end_offset(11),
        Err(SegmentError::OutsideDocument { .. })
    ));
    document.insert(0, "ab").unwrap();
    collection.disconnect();
    document.insert(0, "c").unwrap();
    assert_eq!(value.start_offset(), 5);
    collection
        .update_offsets_entry(OffsetChangeMapEntry::new(0, 0, 1))
        .unwrap();
    assert_eq!(value.start_offset(), 6);
    let last = collection.clone();
    drop(collection);
    assert!(value.is_connected_to_collection());
    drop(last);
    assert!(!value.is_connected_to_collection());
    assert_eq!(value.start_offset(), 6);
    let disconnected = TextSegmentCollection::new();
    disconnected.add(value.clone()).unwrap();
    disconnected.clear();
    assert_eq!(value.start_offset(), 6);
    assert!(!value.is_connected_to_collection());
}

#[test]
fn typed_payload_clone_can_reenter_collection_queries() {
    struct Mark {
        segment: TextSegment,
        on_clone: Rc<dyn Fn()>,
    }
    impl Clone for Mark {
        fn clone(&self) -> Self {
            (self.on_clone)();
            Self {
                segment: self.segment.clone(),
                on_clone: self.on_clone.clone(),
            }
        }
    }
    impl ITextSegment for Mark {
        fn text_segment(&self) -> &TextSegment {
            &self.segment
        }
    }
    let collection = TextSegmentCollection::<Mark>::new();
    let weak = Rc::downgrade(&collection.0);
    collection
        .add(Mark {
            segment: segment(3, 4),
            on_clone: Rc::new(move || {
                if let Some(inner) = weak.upgrade() {
                    inner.tree.borrow_mut().fix_ancestors(None);
                }
            }),
        })
        .unwrap();
    assert_eq!(collection.segments()[0].segment.start_offset(), 3);
    assert!(collection.first_segment().is_some());
    collection.clear();
}

#[test]
fn query_snapshots_allow_removal_and_range_errors_are_atomic() {
    let collection = TextSegmentCollection::new();
    for offset in 0..10 {
        collection.add(segment(offset, 2)).unwrap();
    }
    for value in collection.find_overlapping_segments(0, 20).unwrap() {
        assert!(collection.remove(&value));
    }
    assert!(collection.is_empty());
    let value = segment(usize::MAX - 2, 2);
    collection.add(value.clone()).unwrap();
    assert_eq!(value.set_length(3), Err(SegmentError::RangeOverflow));
    assert_eq!(
        collection.update_offsets_entry(OffsetChangeMapEntry::new(0, 0, 1)),
        Err(SegmentError::RangeOverflow)
    );
    assert_eq!((value.start_offset(), value.length()), (usize::MAX - 2, 2));
    assert!(collection.find_overlapping_segments(usize::MAX, 1).is_err());
}

#[test]
fn deterministic_interval_edits_match_flat_model_and_augmented_tree() {
    let collection = TextSegmentCollection::new();
    let mut model: Vec<(TextSegment, usize, usize)> = Vec::new();
    let mut random = 0x123456789abcdef0_u64;
    let mut next = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random as usize
    };
    for round in 0..2500 {
        match next() % 5 {
            0 | 1 => {
                let start = next() % 300;
                let length = next() % 90;
                let item = segment(start, length);
                collection.add(item.clone()).unwrap();
                model.push((item, start, length));
            }
            2 if !model.is_empty() => {
                let index = next() % model.len();
                let start = next() % 300;
                model[index].0.set_start_offset(start).unwrap();
                model[index].1 = start;
                let length = next() % 70;
                model[index].0.set_length(length).unwrap();
                model[index].2 = length;
            }
            3 if !model.is_empty() => {
                let index = next() % model.len();
                let (item, _, _) = model.swap_remove(index);
                assert!(collection.remove(&item));
            }
            _ => {
                let offset = next() % 300;
                let removed = next() % 30;
                let inserted = next() % 20;
                collection
                    .update_offsets_entry(OffsetChangeMapEntry::new(offset, removed, inserted))
                    .unwrap();
                for (_, start, length) in &mut model {
                    let end = *start + *length;
                    if removed == 0 {
                        if *start < offset && offset < end {
                            *length += inserted;
                        }
                        if *start >= offset {
                            *start += inserted;
                        }
                    } else if *start <= offset {
                        if end >= offset + removed {
                            *length = *length - removed + inserted;
                        } else if end >= offset {
                            *length = offset - *start;
                        }
                    } else if *start <= offset + removed {
                        *start = offset + inserted;
                        *length = end.saturating_sub(offset + removed);
                    } else {
                        *start = *start - removed + inserted;
                    }
                }
            }
        }
        collection.0.tree.borrow().validate();
        for (item, start, length) in &model {
            assert_eq!(
                (item.start_offset(), item.length()),
                (*start, *length),
                "round {round}"
            );
        }
        let offset = next() % 400;
        let length = next() % 80;
        let actual = collection
            .find_overlapping_segments(offset, length)
            .unwrap();
        let expected = model
            .iter()
            .filter(|(_, start, len)| *start <= offset + length && *start + *len >= offset)
            .map(|(s, _, _)| s)
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), expected.len(), "query round {round}");
        assert!(expected.iter().all(|s| actual.contains(s)));
        assert!(actual
            .windows(2)
            .all(|pair| pair[0].start_offset() <= pair[1].start_offset()));
        if round % 100 == 0 {
            let ordered = collection.segments();
            for (index, item) in ordered.iter().enumerate() {
                assert_eq!(
                    collection.get_next_segment(item).unwrap(),
                    ordered.get(index + 1).cloned()
                );
                assert_eq!(
                    collection.get_previous_segment(item).unwrap(),
                    index.checked_sub(1).and_then(|i| ordered.get(i)).cloned()
                );
            }
        }
    }
}

#[test]
fn dense_unaffected_suffix_moves_lazily() {
    let collection = TextSegmentCollection::new();
    let items = (0..8000)
        .map(|index| segment(100 + index * 3, 1))
        .collect::<Vec<_>>();
    for item in &items {
        collection.add(item.clone()).unwrap();
    }
    for _ in 0..200 {
        collection
            .update_offsets_entry(OffsetChangeMapEntry::new(0, 0, 1))
            .unwrap();
    }
    assert_eq!(items[0].start_offset(), 300);
    assert_eq!(items[7999].start_offset(), 300 + 7999 * 3);
    collection.0.tree.borrow().validate();
}
