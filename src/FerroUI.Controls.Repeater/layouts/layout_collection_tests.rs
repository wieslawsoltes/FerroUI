// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Behavioral ports of Repeater/APITests/FlowLayoutCollectionChangeTests.cs.

use super::layout_tests::{base, run, TestHost};
use super::*;
use ferroui_controls::{Control, ItemsChangedEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

/// Source identity is encoded in natural width. The host updates source indices
/// exactly when a real repeater's ViewManager would, before the layout observes
/// a collection event. Measuring then checks the actual control-to-item mapping.
struct CollectionHost {
    host: Rc<TestHost>,
    context: Ref<VirtualizingLayoutContext>,
    layout: Ref<Layout>,
    next_identity: Cell<usize>,
}
impl CollectionHost {
    fn new(count: usize, layout: Ref<Layout>) -> Self {
        let host = TestHost::new(
            (0..count).map(Self::item).collect(),
            Rect::new(0.0, 0.0, 200.0, 200.0),
        );
        let context = host.context();
        layout.initialize_for_context(&base(&context));
        let result = Self {
            host,
            context,
            layout,
            next_identity: Cell::new(count),
        };
        result.measure();
        result
    }
    fn stack(count: usize) -> Self {
        Self::new(count, StackLayout::new().upcast())
    }
    fn item(identity: usize) -> Size {
        Size::new(10.0 + identity as f64 * 0.01, 100.0)
    }
    fn measure(&self) -> Size {
        // A scroll host keeps the visible content anchored while LayoutOrigin
        // changes, and clamps the physical offset to the extent. At the start
        // edge it stays at zero. Complete the resulting viewport-driven passes
        // just as the upstream fixture's UpdateLayout does.
        for _ in 0..20 {
            let window = self.host.window.get();
            let origin = self.host.origin.get();
            let physical_offset = (window.y - origin.y).max(0.0);
            let at_start = physical_offset == 0.0;
            let anchor = if at_start {
                -1
            } else {
                self.host
                    .live
                    .borrow()
                    .iter()
                    .filter(|(index, element)| {
                        *index >= 0
                            && element.bounds().y + element.bounds().height > physical_offset
                    })
                    .min_by(|(_, a), (_, b)| {
                        (a.bounds().y - physical_offset)
                            .abs()
                            .total_cmp(&(b.bounds().y - physical_offset).abs())
                    })
                    .map_or(-1, |(index, _)| *index)
            };
            self.host.anchor.set(anchor);
            let desired = run(&self.layout, &self.context, Size::new(200.0, 200.0));
            let new_origin = self.host.origin.get();
            let next_y = if at_start {
                new_origin.y
            } else {
                window.y.clamp(
                    new_origin.y,
                    new_origin.y + (desired.height - window.height).max(0.0),
                )
            };
            if next_y == window.y {
                return desired;
            }
            self.host
                .window
                .set(Rect::new(window.x, next_y, window.width, window.height));
        }
        panic!("collection layout and viewport did not converge");
    }
    fn source_count(&self) -> usize {
        self.host.sizes.borrow().len()
    }
    fn verify(&self) -> usize {
        let live = self.host.live.borrow().clone();
        let sizes = self.host.sizes.borrow();
        let mut indices = std::collections::BTreeSet::new();
        for (index, element) in &live {
            assert!(
                *index >= 0 && (*index as usize) < sizes.len(),
                "invalid live index {index}"
            );
            assert!(indices.insert(*index), "duplicate realization of {index}");
            assert_eq!(
                element.desired_size(),
                sizes[*index as usize],
                "wrong source item at {index}"
            );
            assert!(element.bounds().x.is_finite() && element.bounds().y.is_finite());
        }
        live.len()
    }
    fn snapshot(&self) -> Vec<(i32, Ref<Control>)> {
        self.host.live.borrow().clone()
    }
    fn replace(&self, index: usize, removed: usize, added: usize) {
        use NotifyCollectionChangedAction::{Add, Remove, Replace};
        let action = if removed == 0 {
            Add
        } else if added == 0 {
            Remove
        } else {
            Replace
        };
        let first_identity = self.next_identity.get();
        self.next_identity.set(first_identity + added);
        self.host.sizes.borrow_mut().splice(
            index..index + removed,
            (first_identity..first_identity + added).map(Self::item),
        );
        for (item_index, _) in self.host.live.borrow_mut().iter_mut() {
            if *item_index >= index as i32 && *item_index < (index + removed) as i32 {
                *item_index = -1;
            } else if *item_index >= (index + removed) as i32 {
                *item_index += added as i32 - removed as i32;
            }
        }
        let old_items: Vec<Option<BoxedValue>> = vec![None; removed];
        let new_items: Vec<Option<BoxedValue>> = vec![None; added];
        self.layout.on_items_changed(
            &base(&self.context),
            &ItemsChangedEventArgs {
                action,
                old_items: (&old_items).into(),
                new_items: (&new_items).into(),
                old_starting_index: if removed > 0 { index as i32 } else { -1 },
                new_starting_index: if added > 0 { index as i32 } else { -1 },
            },
        );
    }
    fn move_item(&self, old: usize, new: usize) {
        let item = self.host.sizes.borrow_mut().remove(old);
        self.host.sizes.borrow_mut().insert(new, item);
        for (index, _) in self.host.live.borrow_mut().iter_mut() {
            if *index == old as i32 {
                *index = new as i32;
            } else if old < new && *index > old as i32 && *index <= new as i32 {
                *index -= 1;
            } else if new < old && *index >= new as i32 && *index < old as i32 {
                *index += 1;
            }
        }
        let items: [Option<BoxedValue>; 1] = [None];
        self.layout.on_items_changed(
            &base(&self.context),
            &ItemsChangedEventArgs {
                action: NotifyCollectionChangedAction::Move,
                old_items: (&items).into(),
                new_items: (&items).into(),
                old_starting_index: old as i32,
                new_starting_index: new as i32,
            },
        );
    }
    fn reset(&self, count: usize) {
        let first = self.next_identity.get();
        self.next_identity.set(first + count);
        *self.host.sizes.borrow_mut() = (first..first + count).map(Self::item).collect();
        self.layout
            .on_items_changed(&base(&self.context), &ItemsChangedEventArgs::RESET);
    }
}
impl Drop for CollectionHost {
    fn drop(&mut self) {
        self.layout.uninitialize_for_context(&base(&self.context));
    }
}

#[test]
fn validate_inserts() {
    let fixture = CollectionHost::stack(10);
    for index in [1, 0, 10] {
        fixture.replace(index, 0, 3);
        fixture.measure();
        assert!(fixture.verify() > 0);
    }
    assert_eq!(fixture.source_count(), 19);
}

#[test]
fn validate_sentinels_during_inserts() {
    let fixture = CollectionHost::stack(10);
    let created = fixture.host.created.get();
    let recycled = fixture.host.recycled.borrow().len();
    let first = fixture.host.element(0);
    fixture.replace(1, 0, 100);
    fixture.measure();
    assert_eq!(fixture.verify(), 3);
    assert!(fixture.host.element(0).ptr_eq(&first));
    assert!(fixture.host.created.get() - created < 6);
    assert!(fixture.host.recycled.borrow().len() - recycled < 6);
}

#[test]
fn can_remove_items_starting_before_realized_range() {
    let fixture = CollectionHost::stack(20);
    fixture.host.window.set(Rect::new(0.0, 600.0, 200.0, 200.0));
    fixture.measure();
    for (index, removed, added) in [(0, 2, 0), (0, 5, 0), (0, 0, 10), (19, 0, 10), (5, 20, 0)] {
        fixture.replace(index, removed, added);
        fixture.measure();
        assert!(fixture.verify() <= 4);
    }
}

#[test]
fn can_remove_items_starting_in_realized_range() {
    let fixture = CollectionHost::stack(10);
    assert_eq!(fixture.verify(), 3);
    fixture.replace(1, 3, 0);
    fixture.measure();
    assert_eq!(fixture.verify(), 3);
    fixture.replace(1, 6, 0);
    fixture.measure();
    assert_eq!(fixture.verify(), 1);
}

#[test]
fn can_remove_and_insert_items_in_realized_range() {
    let fixture = CollectionHost::stack(3);
    fixture.replace(1, 1, 0);
    fixture.measure();
    assert_eq!(fixture.verify(), 2);
    fixture.replace(1, 0, 5);
    fixture.measure();
    assert_eq!(fixture.verify(), 3);
}

#[test]
fn can_remove_items_after_realized_range() {
    let fixture = CollectionHost::stack(10);
    let before = fixture.snapshot();
    fixture.replace(8, 2, 0);
    fixture.measure();
    assert_eq!(fixture.verify(), 3);
    for (index, element) in before {
        assert!(fixture.host.element(index).ptr_eq(&element));
    }
}

#[test]
fn can_replace_single_item() {
    let fixture = CollectionHost::stack(20);
    fixture.host.window.set(Rect::new(0.0, 600.0, 200.0, 200.0));
    fixture.measure();
    for index in [0, 7, 19] {
        let before = fixture.snapshot();
        fixture.replace(index, 1, 1);
        fixture.measure();
        fixture.verify();
        for (old_index, element) in before {
            if old_index == index as i32 {
                assert!(!fixture.host.element(old_index).ptr_eq(&element));
            } else {
                assert!(fixture.host.element(old_index).ptr_eq(&element));
            }
        }
    }
}

#[test]
fn can_move_item() {
    let fixture = CollectionHost::stack(10);
    for (old, new) in [(0, 1), (1, 8), (8, 0), (9, 5), (2, 1)] {
        fixture.move_item(old, new);
        fixture.measure();
        assert_eq!(fixture.verify(), 3);
    }
}

#[test]
fn ensure_replace_of_anchor_does_not_reset_all_containers() {
    let fixture = CollectionHost::stack(10);
    let before = fixture.snapshot();
    let created = fixture.host.created.get();
    fixture.replace(0, 1, 1);
    fixture.measure();
    assert_eq!(fixture.verify(), 3);
    assert_eq!(fixture.host.created.get() - created, 1);
    for (index, element) in before {
        assert_eq!(fixture.host.element(index).ptr_eq(&element), index != 0);
    }
}

#[test]
fn replace_multiple_items() {
    let fixture = CollectionHost::stack(10);
    fixture.replace(0, 1, 3);
    fixture.measure();
    assert_eq!(fixture.source_count(), 12);
    assert_eq!(fixture.verify(), 3);
    fixture.replace(0, 2, 1);
    fixture.measure();
    assert_eq!(fixture.source_count(), 11);
    assert_eq!(fixture.verify(), 3);
    let before = fixture.snapshot();
    fixture.replace(8, 1, 4);
    fixture.measure();
    assert_eq!(fixture.source_count(), 14);
    assert_eq!(fixture.verify(), 3);
    for (index, element) in before {
        assert!(fixture.host.element(index).ptr_eq(&element));
    }
}

#[test]
fn validate_regular_resets() {
    let fixture = CollectionHost::stack(3);
    let before = fixture.snapshot();
    let created = fixture.host.created.get();
    fixture.reset(3);
    fixture.measure();
    assert_eq!(fixture.verify(), 3);
    // Three requested items plus the stack's first-item estimation probe.
    assert_eq!(fixture.host.created.get() - created, 4);
    for (index, element) in before {
        assert!(!fixture.host.element(index).ptr_eq(&element));
    }
}

#[test]
fn validate_clear() {
    for layout in [
        StackLayout::new().upcast(),
        FlowLayout::new().upcast(),
        UniformGridLayout::new().upcast(),
    ] {
        let fixture = CollectionHost::new(3, layout);
        fixture.reset(0);
        assert_eq!(fixture.measure(), Size::default());
        assert_eq!(fixture.verify(), 0);
    }
}
