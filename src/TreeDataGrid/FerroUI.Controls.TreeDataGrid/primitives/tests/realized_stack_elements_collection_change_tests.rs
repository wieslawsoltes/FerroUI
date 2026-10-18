//! `RealizedStackElementsCollectionChangeTests.cs`.
//!
//! The reference's `IndexedControl : Border` carries the index of the element and the item it
//! shows; here they are kept by the test, by control, next to plain borders.

use super::support::start_app;
use crate::primitives::RealizedStackElements;
use ferroui_base::Ref;
use ferroui_controls::{Border, Control};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// An item of a source: compared by identity.
type Item = Rc<()>;

/// The index and the item of each element (`IndexedControl.Index` and `IndexedControl.Item`).
#[derive(Default)]
struct Indexes {
    entries: RefCell<HashMap<Ref<Control>, (i32, Item)>>,
}

impl Indexes {
    fn index(&self, element: &Ref<Control>) -> i32 {
        self.entries.borrow()[element].0
    }

    fn item(&self, element: &Ref<Control>) -> Item {
        self.entries.borrow()[element].1.clone()
    }

    /// `UpdateIndex`: asserts the old index and stores the new one.
    fn update_index(&self, control: &Ref<Control>, old_index: i32, new_index: i32) {
        let mut entries = self.entries.borrow_mut();
        let entry = entries.get_mut(control).expect("an indexed control");
        assert_eq!(old_index, entry.0);
        entry.0 = new_index;
    }
}

fn create_source(count: usize) -> Vec<Item> {
    (0..count).map(|_| Rc::new(())).collect()
}

fn create_range_of(source: &[Item], first_index: i32, count: i32) -> (RealizedStackElements, Vec<Ref<Control>>, Rc<Indexes>) {
    let target = RealizedStackElements::new(false);
    let indexes = Rc::new(Indexes::default());
    let mut elements = Vec::new();
    let mut u = 100.0;

    for i in 0..count {
        let index = first_index + i;
        let element: Ref<Control> = Border::new().upcast();
        indexes.entries.borrow_mut().insert(element.clone(), (index, source[index as usize].clone()));
        elements.push(element.clone());
        target.add(index, element, u, f64::from(index));
        u += f64::from(index);
    }

    (target, elements, indexes)
}

fn create_range(first_index: i32, count: i32) -> (RealizedStackElements, Vec<Ref<Control>>, Rc<Indexes>) {
    let source = create_source((first_index + count) as usize);
    create_range_of(&source, first_index, count)
}

fn recycler() -> (Rc<RefCell<Vec<Ref<Control>>>>, impl Fn(&Ref<Control>)) {
    let recycled = Rc::new(RefCell::new(Vec::new()));
    let sink = recycled.clone();
    (recycled, move |element: &Ref<Control>| sink.borrow_mut().push(element.clone()))
}

fn elements_of(target: &RealizedStackElements) -> Vec<Option<Ref<Control>>> {
    target.elements_snapshot()
}

fn some(elements: &[Ref<Control>]) -> Vec<Option<Ref<Control>>> {
    elements.iter().cloned().map(Some).collect()
}

fn indexes_of(indexes: &Indexes, elements: &[Ref<Control>]) -> Vec<i32> {
    elements.iter().map(|element| indexes.index(element)).collect()
}

fn assert_valid_mapping(target: &RealizedStackElements, source: &[Item], recycled: &[Ref<Control>], indexes: &Indexes) {
    assert_eq!(target.elements().len(), target.size_u().len());
    let mut active: Vec<Ref<Control>> = Vec::new();

    for (i, element) in elements_of(target).into_iter().enumerate() {
        let Some(element) = element else { continue };

        assert!(!active.contains(&element));
        active.push(element.clone());
        assert_eq!(target.first_index() + i as i32, indexes.index(&element));
        let index = indexes.index(&element);
        assert!(index >= 0 && (index as usize) < source.len());
        assert!(Rc::ptr_eq(&source[index as usize], &indexes.item(&element)));
        assert!(!recycled.contains(&element));
    }
}

fn map_moved_index(index: i32, old_index: i32, new_index: i32, count: i32) -> i32 {
    if index >= old_index && index < old_index + count {
        return new_index + index - old_index;
    }
    if old_index < new_index && index >= old_index + count && index < new_index + count {
        return index - count;
    }
    if old_index > new_index && index >= new_index && index < old_index {
        return index + count;
    }
    index
}

#[allow(clippy::too_many_arguments)]
fn assert_move_preservation(
    target: &RealizedStackElements,
    source: &[Item],
    elements: &[Ref<Control>],
    old_indexes: &HashMap<Ref<Control>, i32>,
    recycled: &[Ref<Control>],
    indexes: &Indexes,
    first_index: i32,
    last_index: i32,
    old_index: i32,
    new_index: i32,
    count: i32,
) {
    assert_eq!(first_index, target.first_index());
    assert_eq!(last_index, target.last_index());
    assert_eq!((last_index - first_index + 1) as usize, target.count());
    assert_eq!(target.elements().len(), target.size_u().len());

    let realized = elements_of(target);
    for element in elements {
        let previous_index = old_indexes[element];
        let updated_index = map_moved_index(previous_index, old_index, new_index, count);

        if updated_index >= first_index && updated_index <= last_index {
            let realized_index = (updated_index - first_index) as usize;
            assert_eq!(Some(element), realized[realized_index].as_ref());
            assert_eq!(f64::from(previous_index), target.size_u_at(realized_index));
            assert_eq!(updated_index, indexes.index(element));
            assert!(!recycled.contains(element));
        } else {
            assert!(recycled.contains(element));
        }
    }

    let mut unique: Vec<Ref<Control>> = Vec::new();
    for element in recycled {
        if !unique.contains(element) {
            unique.push(element.clone());
        }
    }
    assert_eq!(recycled.len(), unique.len());
    assert_valid_mapping(target, source, recycled, indexes);
}

fn move_range(source: &mut Vec<Item>, old_index: usize, new_index: usize, count: usize) {
    let moved: Vec<Item> = source.drain(old_index..old_index + count).collect();
    for (i, item) in moved.into_iter().enumerate() {
        source.insert(new_index + i, item);
    }
}

#[test]
fn stable_range_requires_contiguous_measured_elements() {
    let _app = start_app();
    let (target, _, indexes) = create_range(10, 3);

    let (start, end) = target.try_get_stable_range().expect("a stable range");
    assert_eq!(100.0, start);
    assert_eq!(133.0, end);

    target.items_inserted(11, 1, |c, o, n| indexes.update_index(c, o, n), |_| {});

    assert!(target.try_get_stable_range().is_none());
}

#[test]
fn stable_range_is_invalidated_when_indexes_change_before_it() {
    let _app = start_app();
    let (target, _, indexes) = create_range(10, 3);

    target.items_inserted(0, 1, |c, o, n| indexes.update_index(c, o, n), |_| {});

    assert!(target.try_get_stable_range().is_none());
}

#[test]
fn insert_inside_range_preserves_and_reindexes_suffix() {
    let _app = start_app();
    let (target, elements, indexes) = create_range(10, 5);
    let (recycled, recycle) = recycler();

    target.items_inserted(12, 2, |c, o, n| indexes.update_index(c, o, n), recycle);

    assert_eq!(10, target.first_index());
    assert_eq!(16, target.last_index());
    assert_eq!(
        vec![
            Some(elements[0].clone()),
            Some(elements[1].clone()),
            None,
            None,
            Some(elements[2].clone()),
            Some(elements[3].clone()),
            Some(elements[4].clone()),
        ],
        elements_of(&target)
    );
    assert_eq!(vec![10, 11, 14, 15, 16], indexes_of(&indexes, &elements));
    assert!(recycled.borrow().is_empty());
}

#[test]
fn large_insert_recycles_suffix_instead_of_creating_a_sparse_range() {
    let _app = start_app();
    let (target, elements, indexes) = create_range(10, 5);
    let (recycled, recycle) = recycler();

    target.items_inserted(12, 3, |c, o, n| indexes.update_index(c, o, n), recycle);

    assert_eq!(some(&elements[..2]), elements_of(&target));
    assert_eq!(elements[2..].to_vec(), *recycled.borrow());
    assert_eq!(10, target.first_index());
    assert_eq!(11, target.last_index());
}

#[test]
fn remove_inside_range_recycles_only_removed_controls() {
    let _app = start_app();
    let (target, elements, indexes) = create_range(10, 5);
    let (recycled, recycle) = recycler();

    target.items_removed(12, 1, |c, o, n| indexes.update_index(c, o, n), recycle);

    assert_eq!(
        vec![
            Some(elements[0].clone()),
            Some(elements[1].clone()),
            Some(elements[3].clone()),
            Some(elements[4].clone()),
        ],
        elements_of(&target)
    );
    assert_eq!(vec![elements[2].clone()], *recycled.borrow());
    let realized: Vec<Ref<Control>> = elements_of(&target).into_iter().flatten().collect();
    assert_eq!(vec![10, 11, 12, 13], indexes_of(&indexes, &realized));
    assert_eq!(vec![10.0, 11.0, 13.0, 14.0], *target.size_u());
}

#[test]
fn remove_overlapping_start_recycles_overlap_and_preserves_remainder() {
    let _app = start_app();
    let (target, elements, indexes) = create_range(10, 5);
    let (recycled, recycle) = recycler();

    target.items_removed(8, 4, |c, o, n| indexes.update_index(c, o, n), recycle);

    assert_eq!(some(&elements[2..]), elements_of(&target));
    assert_eq!(elements[..2].to_vec(), *recycled.borrow());
    assert_eq!(8, target.first_index());
    let realized: Vec<Ref<Control>> = elements_of(&target).into_iter().flatten().collect();
    assert_eq!(vec![8, 9, 10], indexes_of(&indexes, &realized));
}

#[test]
fn remove_end_boundaries_do_not_recycle_unchanged_controls() {
    let _app = start_app();
    let (target, elements, indexes) = create_range(10, 5);
    let (recycled, recycle) = recycler();

    target.items_removed(8, 2, |c, o, n| indexes.update_index(c, o, n), &recycle);

    assert_eq!(8, target.first_index());
    assert_eq!(some(&elements), elements_of(&target));
    assert_eq!((8..13).collect::<Vec<i32>>(), indexes_of(&indexes, &elements));
    assert!(recycled.borrow().is_empty());

    target.items_removed(11, 20, |c, o, n| indexes.update_index(c, o, n), &recycle);

    assert_eq!(some(&elements[..3]), elements_of(&target));
    assert_eq!(elements[3..].to_vec(), *recycled.borrow());
}

#[test]
fn insert_then_remove_before_layout_restores_original_range() {
    let _app = start_app();
    let (target, elements, indexes) = create_range(0, 10);
    let (recycled, recycle) = recycler();

    target.items_inserted(2, 1, |c, o, n| indexes.update_index(c, o, n), &recycle);
    target.items_removed(2, 1, |c, o, n| indexes.update_index(c, o, n), &recycle);

    assert_eq!(some(&elements), elements_of(&target));
    assert_eq!((0..10).collect::<Vec<i32>>(), indexes_of(&indexes, &elements));
    assert!(recycled.borrow().is_empty());
}

#[test]
fn move_inside_range_preserves_controls_and_their_sizes() {
    let _app = start_app();
    let mut source = create_source(20);
    let (target, elements, indexes) = create_range_of(&source, 5, 10);
    let old_indexes: HashMap<Ref<Control>, i32> =
        elements.iter().map(|element| (element.clone(), indexes.index(element))).collect();
    let (recycled, recycle) = recycler();

    move_range(&mut source, 7, 11, 2);
    target.items_moved(7, 11, 2, |c, o, n| indexes.update_index(c, o, n), recycle);

    assert_move_preservation(&target, &source, &elements, &old_indexes, &recycled.borrow(), &indexes, 5, 14, 7, 11, 2);
    assert_eq!(100.0, target.get_element_u(5));
}

#[test]
fn move_crossing_range_boundaries_preserves_controls_that_remain_visible() {
    let _app = start_app();
    let mut source = create_source(40);
    let (target, elements, indexes) = create_range_of(&source, 10, 10);
    let old_indexes: HashMap<Ref<Control>, i32> =
        elements.iter().map(|element| (element.clone(), indexes.index(element))).collect();
    let (recycled, recycle) = recycler();

    move_range(&mut source, 2, 15, 3);
    target.items_moved(2, 15, 3, |c, o, n| indexes.update_index(c, o, n), recycle);

    assert_move_preservation(&target, &source, &elements, &old_indexes, &recycled.borrow(), &indexes, 10, 19, 2, 15, 3);
    assert!(target.get_element_u(10).is_nan());
}

#[test]
fn collection_change_matrix_preserves_item_mapping() {
    let _app = start_app();
    let first_indexes = [0, 5, 20];
    let realized_counts = [1, 5, 10];
    let change_counts = [1, 2, 5, 12];

    for first_index in first_indexes {
        for realized_count in realized_counts {
            for index in 0..=40 {
                for count in change_counts {
                    let mut insert_source = create_source(40);
                    let (insert_target, _, insert_indexes) = create_range_of(&insert_source, first_index, realized_count);
                    let (insert_recycled, insert_recycle) = recycler();

                    for i in 0..count {
                        insert_source.insert(index + i, Rc::new(()));
                    }
                    insert_target.items_inserted(
                        index as i32,
                        count as i32,
                        |c, o, n| insert_indexes.update_index(c, o, n),
                        insert_recycle,
                    );
                    assert_valid_mapping(&insert_target, &insert_source, &insert_recycled.borrow(), &insert_indexes);

                    if index < 40 {
                        let remove_count = count.min(40 - index);
                        let mut remove_source = create_source(40);
                        let (remove_target, _, remove_indexes) =
                            create_range_of(&remove_source, first_index, realized_count);
                        let (remove_recycled, remove_recycle) = recycler();

                        remove_source.drain(index..index + remove_count);
                        remove_target.items_removed(
                            index as i32,
                            remove_count as i32,
                            |c, o, n| remove_indexes.update_index(c, o, n),
                            remove_recycle,
                        );
                        assert_valid_mapping(&remove_target, &remove_source, &remove_recycled.borrow(), &remove_indexes);
                    }
                }
            }
        }
    }
}

#[test]
fn move_matrix_preserves_every_control_that_remains_in_the_realized_window() {
    let _app = start_app();
    const SOURCE_COUNT: i32 = 40;
    let first_indexes = [0, 5, 20];
    let realized_counts = [1, 5, 10];
    let move_counts = [1, 2, 5];

    for first_index in first_indexes {
        for realized_count in realized_counts {
            for move_count in move_counts {
                for old_index in 0..=SOURCE_COUNT - move_count {
                    for new_index in 0..=SOURCE_COUNT - move_count {
                        let mut source = create_source(SOURCE_COUNT as usize);
                        let (target, elements, indexes) = create_range_of(&source, first_index, realized_count);
                        let old_indexes: HashMap<Ref<Control>, i32> =
                            elements.iter().map(|element| (element.clone(), indexes.index(element))).collect();
                        let (recycled, recycle) = recycler();

                        move_range(&mut source, old_index as usize, new_index as usize, move_count as usize);
                        target.items_moved(
                            old_index,
                            new_index,
                            move_count,
                            |c, o, n| indexes.update_index(c, o, n),
                            recycle,
                        );

                        assert_move_preservation(
                            &target,
                            &source,
                            &elements,
                            &old_indexes,
                            &recycled.borrow(),
                            &indexes,
                            first_index,
                            first_index + realized_count - 1,
                            old_index,
                            new_index,
                            move_count,
                        );
                    }
                }
            }
        }
    }
}
