// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Behavioral ports of Repeater/APITests/LinedFlowLayoutTests.cs.

use super::layout_tests::{base, run, TestHost};
use super::*;
use ferroui_controls::ItemsChangedEventArgs;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const LINE_HEIGHT: f64 = 50.0;
struct LinedHost {
    host: Rc<TestHost>,
    context: Ref<VirtualizingLayoutContext>,
    layout: Ref<LinedFlowLayout>,
    width: f64,
    desired: Cell<Size>,
    detached: Cell<bool>,
}
impl LinedHost {
    fn new(widths: impl IntoIterator<Item = f64>, width: f64, viewport_height: f64) -> Self {
        let host = TestHost::new(
            widths
                .into_iter()
                .map(|width| Size::new(width, LINE_HEIGHT))
                .collect(),
            Rect::new(0.0, 0.0, width, viewport_height),
        );
        let context = host.context();
        let layout = LinedFlowLayout::new();
        layout.set_line_height(LINE_HEIGHT);
        layout.initialize_for_context(&base(&context));
        Self {
            host,
            context,
            layout,
            width,
            desired: Cell::new(Size::default()),
            detached: Cell::new(false),
        }
    }
    fn uniform(count: usize) -> Self {
        Self::new(vec![100.0; count], 520.0, 600.0)
    }
    fn settle(&self) {
        for _ in 0..5 {
            self.desired.set(run(
                &self.layout,
                &self.context,
                Size::new(self.width, self.host.window.get().height),
            ));
        }
    }
    fn scroll(&self, offset: f64) {
        let mut window = self.host.window.get();
        window.y = offset.clamp(0.0, (self.desired.get().height - window.height).max(0.0));
        self.host.window.set(window);
        self.settle();
    }
    fn detach(&self) {
        if !self.detached.replace(true) {
            self.layout.uninitialize_for_context(&base(&self.context));
        }
    }
    fn change_last(&self, add: bool) {
        let index = if add {
            let index = self.host.sizes.borrow().len();
            self.host
                .sizes
                .borrow_mut()
                .push(Size::new(100.0, LINE_HEIGHT));
            index
        } else {
            self.host.sizes.borrow_mut().pop().unwrap();
            self.host.sizes.borrow().len()
        };
        let item: [Option<BoxedValue>; 1] = [None];
        let empty: [Option<BoxedValue>; 0] = [];
        self.layout.on_items_changed(
            &base(&self.context),
            &ItemsChangedEventArgs {
                action: if add {
                    NotifyCollectionChangedAction::Add
                } else {
                    NotifyCollectionChangedAction::Remove
                },
                new_items: if add { (&item).into() } else { (&empty).into() },
                old_items: if add { (&empty).into() } else { (&item).into() },
                new_starting_index: if add { index as i32 } else { -1 },
                old_starting_index: if add { -1 } else { index as i32 },
            },
        );
    }
}
impl Drop for LinedHost {
    fn drop(&mut self) {
        self.detach();
    }
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-7, "{actual} != {expected}");
}

#[test]
fn verify_default_property_values() {
    let _scope = crate::test_support::test_scope();
    let layout = LinedFlowLayout::new();
    assert_eq!(layout.actual_line_height(), 0.0);
    assert!(layout.line_height().is_nan());
    assert_eq!(layout.line_spacing(), 0.0);
    assert_eq!(layout.min_item_spacing(), 0.0);
    assert_eq!(
        layout.items_justification(),
        LinedFlowLayoutItemsJustification::Start
    );
    assert_eq!(layout.items_stretch(), LinedFlowLayoutItemsStretch::None);
}

#[test]
fn verify_lock_item_to_line_throws_for_out_of_range_index() {
    let _scope = crate::test_support::test_scope();
    assert_eq!(
        LinedFlowLayout::new().lock_item_to_line(0),
        Err(LinedFlowLayoutError::ItemIndexOutOfBounds)
    );
    let fixture = LinedHost::uniform(20);
    fixture.settle();
    for index in [-1, 20, i32::MAX] {
        assert_eq!(
            fixture.layout.lock_item_to_line(index),
            Err(LinedFlowLayoutError::ItemIndexOutOfBounds)
        );
    }
}

#[test]
fn verify_lock_item_to_line_returns_minus_one_before_first_measure() {
    let _scope = crate::test_support::test_scope();
    let fixture = LinedHost::uniform(10);
    for index in [0, 5, 9] {
        assert_eq!(fixture.layout.lock_item_to_line(index), Ok(-1));
    }
    assert_eq!(
        fixture.layout.lock_item_to_line(10),
        Err(LinedFlowLayoutError::ItemIndexOutOfBounds)
    );
}

fn verify_arranged_lock_lines(metadata: bool) {
    let fixture = LinedHost::uniform(23);
    fixture.layout.set_line_spacing(10.0);
    let requested = Rc::new(Cell::new(0));
    let observed = requested.clone();
    let _metadata = metadata.then(|| {
        fixture.layout.items_info_requested(move |args| {
            observed.set(observed.get() + 1);
            args.set_items_range_start_index(0).unwrap();
            args.set_desired_aspect_ratios(&[2.0; 23]).unwrap();
        })
    });
    fixture.settle();
    assert_eq!(requested.get() > 0, metadata);
    let lines: Vec<_> = (0..23)
        .map(|index| {
            let y = fixture.host.element(index).bounds().y / 60.0;
            close(y, y.round());
            y.round() as i32
        })
        .collect();
    assert!(lines[22] > 1);
    for index in [0, 1, 11, 21, 22] {
        let line = fixture.layout.lock_item_to_line(index).unwrap();
        assert_eq!(line, lines[index as usize]);
        assert_eq!(fixture.layout.lock_item_to_line(index).unwrap(), line);
    }
}
#[test]
fn verify_lock_item_to_line_returns_arranged_line_index() {
    let _scope = crate::test_support::test_scope();
    verify_arranged_lock_lines(false);
}
#[test]
fn verify_lock_item_to_line_returns_arranged_line_index_with_items_info() {
    let _scope = crate::test_support::test_scope();
    verify_arranged_lock_lines(true);
}

#[test]
fn verify_items_unlocked_raised_when_locks_are_invalidated() {
    let _scope = crate::test_support::test_scope();
    let fixture = LinedHost::uniform(15);
    fixture.settle();
    let events = Rc::new(Cell::new(0));
    let observed = events.clone();
    let _events = fixture
        .layout
        .items_unlocked(move || observed.set(observed.get() + 1));
    fixture.change_last(true);
    assert_eq!(events.get(), 0);
    fixture.settle();
    assert_eq!(events.get(), 0);
    assert!(fixture.layout.lock_item_to_line(7).unwrap() >= 0);
    fixture.change_last(true);
    assert_eq!(events.get(), 1);
    fixture.change_last(false);
    assert_eq!(events.get(), 1);
    fixture.settle();
    assert_eq!(fixture.layout.lock_item_to_line(0), Ok(0));
    fixture.change_last(false);
    assert_eq!(events.get(), 2);
    fixture.settle();
    assert_eq!(events.get(), 2);
}

#[test]
fn verify_items_unlocked_raised_when_layout_detached() {
    let _scope = crate::test_support::test_scope();
    let fixture = LinedHost::uniform(15);
    fixture.settle();
    let events = Rc::new(Cell::new(0));
    let observed = events.clone();
    let _events = fixture
        .layout
        .items_unlocked(move || observed.set(observed.get() + 1));
    assert!(fixture.layout.lock_item_to_line(3).unwrap() >= 0);
    fixture.detach();
    assert_eq!(events.get(), 1);
}

#[test]
fn verify_items_justification_without_stretch() {
    let _scope = crate::test_support::test_scope();
    use LinedFlowLayoutItemsJustification::*;
    for (justification, positions) in [
        (Start, [0.0, 100.0, 200.0]),
        (Center, [100.0, 200.0, 300.0]),
        (End, [200.0, 300.0, 400.0]),
        (SpaceEvenly, [50.0, 200.0, 350.0]),
        (SpaceAround, [200.0 / 6.0, 200.0, 400.0 - 200.0 / 6.0]),
        (SpaceBetween, [0.0, 200.0, 400.0]),
    ] {
        let fixture = LinedHost::new([100.0; 3], 500.0, 600.0);
        fixture.layout.set_items_justification(justification);
        assert_eq!(
            fixture.layout.items_stretch(),
            LinedFlowLayoutItemsStretch::None
        );
        fixture.settle();
        close(fixture.layout.actual_line_height(), LINE_HEIGHT);
        for (index, position) in positions.into_iter().enumerate() {
            let bounds = fixture.host.element(index as i32).bounds();
            close(bounds.x, position);
            close(bounds.y, 0.0);
            close(bounds.width, 100.0);
        }
    }
}

#[test]
fn verify_unconstrained_width_arranges_single_line() {
    let _scope = crate::test_support::test_scope();
    use LinedFlowLayoutItemsJustification::*;
    for stretch in [
        LinedFlowLayoutItemsStretch::None,
        LinedFlowLayoutItemsStretch::Fill,
    ] {
        for (justification, leading) in [
            (Start, 0.0),
            (End, 0.0),
            (SpaceEvenly, 10.0),
            (SpaceAround, 5.0),
        ] {
            // A finite realization area still represents an ItemsRepeater;
            // only its measure width is unconstrained by its horizontal panel.
            let mut fixture = LinedHost::new([40.0, 60.0, 80.0, 100.0], 1000.0, 600.0);
            fixture.width = f64::INFINITY;
            fixture.layout.set_items_stretch(stretch);
            fixture.layout.set_items_justification(justification);
            fixture.layout.set_min_item_spacing(10.0);
            fixture.settle();
            assert_eq!(fixture.desired.get(), Size::new(310.0, LINE_HEIGHT));
            let mut x = if stretch == LinedFlowLayoutItemsStretch::None {
                leading
            } else {
                0.0
            };
            for (index, width) in [40.0, 60.0, 80.0, 100.0].into_iter().enumerate() {
                let bounds = fixture.host.element(index as i32).bounds();
                close(bounds.x, x);
                close(bounds.y, 0.0);
                x += width + 10.0;
            }
        }
    }
}

#[test]
fn verify_lined_flow_layout_cannot_be_shared() {
    let _scope = crate::test_support::test_scope();
    let fixture = LinedHost::uniform(3);
    let second = TestHost::uniform(4, Size::new(50.0, LINE_HEIGHT));
    let second_context = second.context();
    assert_eq!(
        fixture.layout.try_initialize_for_context(&second_context),
        Err(LinedFlowLayoutError::CannotShareLayout)
    );
    fixture.detach();
    fixture
        .layout
        .try_initialize_for_context(&second_context)
        .unwrap();
    run(&fixture.layout, &second_context, Size::new(300.0, 600.0));
    close(fixture.layout.actual_line_height(), LINE_HEIGHT);
    for index in 0..4 {
        assert_eq!(
            second.element(index).desired_size(),
            Size::new(50.0, LINE_HEIGHT)
        );
    }
    fixture
        .layout
        .uninitialize_for_context(&base(&second_context));
}

#[test]
fn verify_requested_range_reflects_items_info() {
    let _scope = crate::test_support::test_scope();
    let detached = LinedFlowLayout::new();
    assert_eq!(
        (
            detached.requested_range_start_index(),
            detached.requested_range_length()
        ),
        (-1, 0)
    );
    let fixture = LinedHost::uniform(12);
    fixture.settle();
    assert_eq!(
        (
            fixture.layout.requested_range_start_index(),
            fixture.layout.requested_range_length()
        ),
        (-1, 0)
    );
    let requested = Rc::new(Cell::new(0));
    let observed = requested.clone();
    let _metadata = fixture.layout.items_info_requested(move |args| {
        observed.set(observed.get() + 1);
        args.set_items_range_start_index(0).unwrap();
        args.set_desired_aspect_ratios(&[2.0; 12]).unwrap();
    });
    fixture.layout.invalidate_items_info();
    fixture.settle();
    assert!(requested.get() > 0);
    assert_eq!(
        (
            fixture.layout.requested_range_start_index(),
            fixture.layout.requested_range_length()
        ),
        (0, 12)
    );
}

#[test]
fn verify_locked_items_keep_their_lines_while_scrolling() {
    let _scope = crate::test_support::test_scope();
    let normal = |index: usize| 40.0 + (index % 5) as f64 * 20.0;
    let changed = move |index: usize| {
        if index < 10 {
            20.0
        } else if index >= 290 {
            normal(index) + 60.0
        } else {
            normal(index)
        }
    };
    let reference = LinedHost::new((0..300).map(changed), 520.0, 300.0);
    let _metadata = reference.layout.items_info_requested(move |args| {
        let start = args.items_range_start_index() as usize;
        let end = start + args.items_range_requested_length() as usize;
        args.set_desired_aspect_ratios(
            &(start..end)
                .map(|i| changed(i) / LINE_HEIGHT)
                .collect::<Vec<_>>(),
        )
        .unwrap();
    });
    reference.settle();
    let unlocked_changed_line = reference.layout.lock_item_to_line(10).unwrap();
    reference.detach();
    let fixture = LinedHost::new((0..300).map(normal), 520.0, 300.0);
    let perturbed = Rc::new(Cell::new(false));
    let observed = perturbed.clone();
    let _metadata = fixture.layout.items_info_requested(move |args| {
        let start = args.items_range_start_index() as usize;
        let end = start + args.items_range_requested_length() as usize;
        args.set_desired_aspect_ratios(
            &(start..end)
                .map(|i| {
                    (if observed.get() {
                        changed(i)
                    } else {
                        normal(i)
                    }) / LINE_HEIGHT
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
    });
    let unlocked = Rc::new(Cell::new(0));
    let observed = unlocked.clone();
    let _events = fixture
        .layout
        .items_unlocked(move || observed.set(observed.get() + 1));
    fixture.settle();
    let locks = [
        (3, fixture.layout.lock_item_to_line(3).unwrap()),
        (10, fixture.layout.lock_item_to_line(10).unwrap()),
    ];
    assert!(locks[0].1 >= 0 && locks[1].1 >= locks[0].1);
    fixture.scroll(3000.0);
    assert!(!fixture
        .host
        .live
        .borrow()
        .iter()
        .any(|(index, _)| *index == 3));
    fixture.scroll(0.0);
    assert_eq!(unlocked.get(), 0);
    for (index, line) in locks {
        assert_eq!(fixture.layout.lock_item_to_line(index).unwrap(), line);
        close(
            fixture.host.element(index).bounds().y,
            line as f64 * LINE_HEIGHT,
        );
    }
    assert_ne!(locks[1].1, unlocked_changed_line);
    perturbed.set(true);
    fixture.layout.invalidate_items_info();
    fixture.settle();
    assert_eq!(unlocked.get(), 0);
    assert_eq!(fixture.layout.lock_item_to_line(10).unwrap(), locks[1].1);
    close(
        fixture.host.element(10).bounds().y,
        locks[1].1 as f64 * LINE_HEIGHT,
    );
}

#[test]
fn lowering_global_minimum_applies_to_retained_metadata_overlap() {
    let _scope = crate::test_support::test_scope();
    let fixture = LinedHost::new(vec![20.0; 10_000], 100.0, 100.0);
    fixture.layout.set_line_height(20.0);
    let minimum = Rc::new(Cell::new(50.0));
    let observed = minimum.clone();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let seen = requests.clone();
    let _metadata = fixture.layout.items_info_requested(move |args| {
        seen.borrow_mut().push((
            args.items_range_start_index(),
            args.items_range_requested_length(),
        ));
        args.set_min_width(observed.get());
        args.set_desired_aspect_ratios(&vec![1.0; args.items_range_requested_length() as usize])
            .unwrap();
    });
    fixture.settle();
    let retained = fixture.host.element(0);
    close(retained.bounds().width, 50.0);
    let requests_before_scroll = requests.borrow().len();
    let old_start = fixture.layout.requested_range_start_index();
    let old_end = old_start + fixture.layout.requested_range_length();
    minimum.set(10.0);
    // Startup prefetch is based on an estimate that precedes MinWidth. Cross
    // its actual boundary in viewport-sized steps, retaining overlap instead
    // of assuming a fixed scroll distance will request more metadata.
    for step in 1..=old_end - old_start + 1 {
        fixture.scroll(step as f64 * 100.0);
        if requests.borrow().len() > requests_before_scroll {
            break;
        }
    }
    assert!(
        requests.borrow().len() > requests_before_scroll,
        "scrolling must expose metadata beyond {old_start}..{old_end}"
    );
    // The source deliberately preserves widths inside its frozen lines. A
    // layout invalidation recomputes those widths using the retained metadata,
    // without discarding the overlap as InvalidateItemsInfo would.
    fixture.layout.invalidate_layout();
    fixture.settle();
    let refreshed = requests.borrow()[requests_before_scroll..].to_vec();
    let retained: Vec<_> = fixture
        .host
        .live
        .borrow()
        .iter()
        .filter(|(index, _)| {
            (old_start..old_end).contains(index)
                && !refreshed
                    .iter()
                    .any(|(start, length)| (*start..*start + *length).contains(index))
        })
        .cloned()
        .collect();
    assert!(
        !retained.is_empty(),
        "the width check must use metadata retained from {old_start}..{old_end}, \
         not rerequested entries: requests={refreshed:?}, viewport={:?}",
        fixture.host.window.get()
    );
    for (_, element) in retained {
        close(element.bounds().width, 20.0);
    }
}
