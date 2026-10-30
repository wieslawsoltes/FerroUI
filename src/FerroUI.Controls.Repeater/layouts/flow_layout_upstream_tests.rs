// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Behavioral ports of Repeater/APITests/FlowLayoutTests.cs. Native controls
// replace themed WinUI Buttons so intrinsic measurements are deterministic.

use super::layout_tests::{base, run, TestHost};
use super::*;
use crate::LayoutPanel;
use ferroui_controls::{Control, ControlImpl, ItemsChangedEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy)]
struct Axis(bool);
impl Axis {
    fn size(self, minor: f64, major: f64) -> Size {
        if self.0 {
            Size::new(minor, major)
        } else {
            Size::new(major, minor)
        }
    }
    fn rect(self, minor: f64, major: f64, minor_size: f64, major_size: f64) -> Rect {
        if self.0 {
            Rect::new(minor, major, minor_size, major_size)
        } else {
            Rect::new(major, minor, major_size, minor_size)
        }
    }
    fn major(self, size: Size) -> f64 {
        if self.0 {
            size.height
        } else {
            size.width
        }
    }
    fn orientation(self, stack: bool) -> Orientation {
        if self.0 == stack {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        }
    }
}

struct Fixture {
    host: Rc<TestHost>,
    context: Ref<VirtualizingLayoutContext>,
    layout: Ref<Layout>,
    available: Size,
}
impl Fixture {
    fn new(layout: Ref<Layout>, sizes: Vec<Size>, available: Size, window: Rect) -> Self {
        let host = TestHost::new(sizes, window);
        let context = host.context();
        layout.initialize_for_context(&base(&context));
        Self {
            host,
            context,
            layout,
            available,
        }
    }
    fn measure(&self) -> Size {
        run(&self.layout, &self.context, self.available)
    }
    fn insert_item(&self, index: i32, size: Size) {
        self.host.sizes.borrow_mut().insert(index as usize, size);
        for (current, _) in self.host.live.borrow_mut().iter_mut() {
            if *current >= index {
                *current += 1;
            }
        }
        let new: [Option<BoxedValue>; 1] = [None];
        self.layout.on_items_changed(
            &base(&self.context),
            &ItemsChangedEventArgs {
                action: NotifyCollectionChangedAction::Add,
                old_items: (&[]).into(),
                new_items: (&new).into(),
                old_starting_index: -1,
                new_starting_index: index,
            },
        );
    }
    fn remove_first(&self) {
        self.host.sizes.borrow_mut().remove(0);
        for (index, _) in self.host.live.borrow_mut().iter_mut() {
            *index -= 1;
        }
        let old: [Option<BoxedValue>; 1] = [None];
        self.layout.on_items_changed(
            &base(&self.context),
            &ItemsChangedEventArgs {
                action: NotifyCollectionChangedAction::Remove,
                old_items: (&old).into(),
                new_items: (&[]).into(),
                old_starting_index: 0,
                new_starting_index: -1,
            },
        );
    }
    fn grid_bounds(&self, index: i32) -> Rect {
        let state = UniformGridLayout::state(&self.context).expect("grid state");
        let mut bounds = state
            .flow_algorithm
            .element_manager
            .layout_bounds_for_data_index(index);
        let origin = self.host.origin.get();
        bounds.x -= origin.x;
        bounds.y -= origin.y;
        bounds
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.layout.uninitialize_for_context(&base(&self.context));
    }
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}
fn rect_close(actual: Rect, expected: Rect) {
    close(actual.x, expected.x);
    close(actual.y, expected.y);
    close(actual.width, expected.width);
    close(actual.height, expected.height);
}

#[test]
fn validate_grid_layout() {
    for axis in [Axis(true), Axis(false)] {
        for minor in [100.0, 800.0] {
            let layout = UniformGridLayout::new();
            layout.set_orientation(axis.orientation(false));
            let fixture = Fixture::new(
                layout.clone().upcast(),
                vec![Size::default(); 10],
                axis.size(400.0, f64::INFINITY),
                axis.rect(0.0, 0.0, 400.0, 5000.0),
            );
            for (scale, row_spacing, column_spacing) in
                [(1.0, 0.0, 0.0), (1.0, 5.0, 10.0), (2.0, 5.0, 10.0)]
            {
                let item = axis.size(minor * scale, 50.0 * scale);
                layout.set_min_item_width(item.width);
                layout.set_min_item_height(item.height);
                layout.set_min_row_spacing(row_spacing);
                layout.set_min_column_spacing(column_spacing);
                let spacing = if axis.0 { column_spacing } else { row_spacing };
                let line_spacing = if axis.0 { row_spacing } else { column_spacing };
                let per_line = ((400.0 + spacing) / (minor * scale + spacing))
                    .floor()
                    .max(1.0) as i32;
                let desired = fixture.measure();
                for i in 0..10 {
                    rect_close(
                        fixture.grid_bounds(i),
                        axis.rect(
                            (i % per_line) as f64 * (minor * scale + spacing),
                            (i / per_line) as f64 * (50.0 * scale + line_spacing),
                            minor * scale,
                            50.0 * scale,
                        ),
                    );
                }
                close(
                    axis.major(desired),
                    ((10 + per_line - 1) / per_line) as f64 * (50.0 * scale + line_spacing)
                        - line_spacing,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Dimension {
    Width,
    Height,
    Both,
}
fn first_item_dimension(dimension: Dimension, remove: bool) {
    for axis in [Axis(true), Axis(false)] {
        let layout = UniformGridLayout::new();
        layout.set_orientation(axis.orientation(false));
        let fixture = Fixture::new(
            layout.upcast(),
            vec![Size::new(20.0, 30.0); 10],
            axis.size(400.0, f64::INFINITY),
            axis.rect(0.0, 0.0, 400.0, 5000.0),
        );
        fixture.measure();
        for index in 0..10 {
            assert_eq!(fixture.grid_bounds(index).size(), Size::new(20.0, 30.0));
        }
        for value in [100.0, 32.0] {
            let size = match dimension {
                Dimension::Width => Size::new(value, 30.0),
                Dimension::Height => Size::new(20.0, value),
                Dimension::Both => Size::new(value, value),
            };
            fixture.host.set_size(if remove { 1 } else { 0 }, size);
            if remove {
                fixture.remove_first();
            }
            fixture.measure();
            for index in 0..fixture.host.item_count() {
                assert_eq!(fixture.grid_bounds(index).size(), size);
            }
            if remove {
                break;
            }
        }
    }
}
#[test]
fn validate_default_width_for_grid_layout_items_is_based_on_first_item() {
    first_item_dimension(Dimension::Width, false);
}
#[test]
fn validate_default_height_for_grid_layout_items_is_based_on_first_item() {
    first_item_dimension(Dimension::Height, false);
}
#[test]
fn validate_adaptability_when_changing_first_element_width_for_grid_layout() {
    first_item_dimension(Dimension::Width, true);
}
#[test]
fn validate_adaptability_when_changing_first_element_height_for_grid_layout() {
    first_item_dimension(Dimension::Height, true);
}
#[test]
fn validate_adaptability_when_changing_first_element_size_for_grid_layout() {
    first_item_dimension(Dimension::Both, true);
}

#[test]
fn validate_grid_layout_justification_and_stretch() {
    for axis in [Axis(true), Axis(false)] {
        let layout = UniformGridLayout::new();
        layout.set_orientation(axis.orientation(false));
        let item = axis.size(200.0, 100.0);
        layout.set_min_item_width(item.width);
        layout.set_min_item_height(item.height);
        let fixture = Fixture::new(
            layout.clone().upcast(),
            vec![Size::default(); 2],
            axis.size(500.0, f64::INFINITY),
            axis.rect(0.0, 0.0, 500.0, 5000.0),
        );
        for (alignment, offsets) in [
            (UniformGridLayoutItemsJustification::Start, [0.0, 200.0]),
            (UniformGridLayoutItemsJustification::Center, [50.0, 250.0]),
            (UniformGridLayoutItemsJustification::End, [100.0, 300.0]),
            (
                UniformGridLayoutItemsJustification::SpaceBetween,
                [0.0, 300.0],
            ),
            (
                UniformGridLayoutItemsJustification::SpaceAround,
                [25.0, 275.0],
            ),
            (
                UniformGridLayoutItemsJustification::SpaceEvenly,
                [100.0 / 3.0, 800.0 / 3.0],
            ),
        ] {
            layout.set_items_justification(alignment);
            fixture.measure();
            for (i, offset) in offsets.into_iter().enumerate() {
                rect_close(
                    fixture.host.element(i as i32).bounds(),
                    axis.rect(offset, 0.0, 200.0, 100.0),
                );
            }
        }
        layout.set_items_justification(UniformGridLayoutItemsJustification::Start);
        layout.set_min_row_spacing(10.0);
        layout.set_min_column_spacing(10.0);
        for (stretch, height) in [
            (UniformGridLayoutItemsStretch::Fill, 100.0),
            (UniformGridLayoutItemsStretch::Uniform, 122.5),
        ] {
            layout.set_items_stretch(stretch);
            fixture.measure();
            for i in 0..2 {
                rect_close(
                    fixture.host.element(i).bounds(),
                    axis.rect(i as f64 * 255.0, 0.0, 245.0, height),
                );
            }
        }
    }
}

#[test]
fn validate_grid_layout_desired_sizing_with_max_rows_or_columns_set() {
    for axis in [Axis(true), Axis(false)] {
        let layout = UniformGridLayout::new();
        layout.set_orientation(axis.orientation(false));
        layout.set_min_item_width(100.0);
        layout.set_min_item_height(100.0);
        let fixture = Fixture::new(
            layout.clone().upcast(),
            vec![Size::new(100.0, 100.0); 4],
            Size::new(f64::INFINITY, f64::INFINITY),
            axis.rect(0.0, 0.0, 5000.0, 5000.0),
        );
        for maximum in 1..4 {
            layout.set_maximum_rows_or_columns(maximum);
            assert_eq!(
                fixture.measure(),
                axis.size(
                    100.0 * maximum as f64,
                    100.0 * ((4 + maximum - 1) / maximum) as f64
                )
            );
        }
    }
}

// Regression for the pinned source's ignored PC-GRID-ANCHOR-INSERT scenario.
// The edge sentinel can be the only realized child of its logical row. Its
// column and justification must still include the offscreen siblings.
#[test]
fn grid_partial_lines_preserve_source_columns_for_every_justification() {
    for axis in [Axis(true), Axis(false)] {
        for alignment in [
            UniformGridLayoutItemsJustification::Start,
            UniformGridLayoutItemsJustification::Center,
            UniformGridLayoutItemsJustification::End,
            UniformGridLayoutItemsJustification::SpaceBetween,
            UniformGridLayoutItemsJustification::SpaceAround,
            UniformGridLayoutItemsJustification::SpaceEvenly,
        ] {
            let layout = UniformGridLayout::new();
            layout.set_orientation(axis.orientation(false));
            layout.set_min_item_width(axis.size(100.0, 60.0).width);
            layout.set_min_item_height(axis.size(100.0, 60.0).height);
            layout.set_min_column_spacing(if axis.0 { 10.0 } else { 5.0 });
            layout.set_min_row_spacing(if axis.0 { 5.0 } else { 10.0 });
            layout.set_items_justification(alignment);
            let fixture = Fixture::new(
                layout.upcast(),
                vec![Size::default(); 45],
                axis.size(350.0, f64::INFINITY),
                axis.rect(0.0, 670.0, 350.0, 200.0),
            );
            let verify = || {
                let items = fixture.host.sizes.borrow().len() as i32;
                let live = fixture.host.live.borrow();
                assert!(live.len() < 24, "realization stays bounded to the viewport");
                for (index, element) in live.iter() {
                    let column = index % 3;
                    let row_start = index - column;
                    let row_count = (items - row_start).min(3);
                    let free = 350.0 - (row_count as f64 * 110.0 - 10.0);
                    let shift = match alignment {
                        UniformGridLayoutItemsJustification::Start => 0.0,
                        UniformGridLayoutItemsJustification::Center => free / 2.0,
                        UniformGridLayoutItemsJustification::End => free,
                        UniformGridLayoutItemsJustification::SpaceBetween => {
                            if row_count > 1 {
                                free * column as f64 / (row_count - 1) as f64
                            } else {
                                0.0
                            }
                        }
                        UniformGridLayoutItemsJustification::SpaceAround => {
                            free * (column as f64 + 0.5) / row_count as f64
                        }
                        UniformGridLayoutItemsJustification::SpaceEvenly => {
                            free * (column + 1) as f64 / (row_count + 1) as f64
                        }
                    };
                    rect_close(
                        element.bounds(),
                        axis.rect(
                            column as f64 * 110.0 + shift,
                            (index / 3) as f64 * 65.0,
                            100.0,
                            60.0,
                        ),
                    );
                }
            };
            fixture.measure();
            assert!(fixture
                .host
                .live
                .borrow()
                .iter()
                .any(|(index, _)| *index == 29));
            verify();

            let anchor = fixture.host.element(31);
            // First move the anchor into a nonzero column, then across a row.
            for index in [32, 33] {
                fixture.insert_item(0, Size::default());
                fixture.host.anchor.set(index);
                fixture.measure();
                assert!(fixture.host.element(index).ptr_eq(&anchor));
                verify();
            }

            fixture.host.anchor.set(-1);
            fixture.host.window.set(axis.rect(0.0, 930.0, 350.0, 200.0));
            fixture.measure();
            verify();
            assert!(fixture
                .host
                .live
                .borrow()
                .iter()
                .any(|(index, _)| *index == 46));
        }
    }
}

#[test]
fn validate_flow_layout() {
    for axis in [Axis(true), Axis(false)] {
        for width in [10.0, 50.0] {
            let layout = FlowLayout::new();
            layout.set_orientation(axis.orientation(false));
            let fixture = Fixture::new(
                layout.clone().upcast(),
                vec![axis.size(20.0, 30.0); 10],
                axis.size(width, f64::INFINITY),
                axis.rect(0.0, 0.0, 50.0, 5000.0),
            );
            for spacing in [0.0, 10.0] {
                layout.set_line_spacing(spacing);
                layout.set_min_item_spacing(spacing);
                // The source helper uses child.DesiredSize, not the intrinsic
                // size before native measurement constrains it to the panel.
                let item_minor = width.min(20.0);
                let count = ((width + spacing) / (item_minor + spacing))
                    .floor()
                    .max(1.0) as i32;
                let desired = fixture.measure();
                for i in 0..10 {
                    assert_eq!(
                        fixture.host.element(i).desired_size(),
                        axis.size(item_minor, 30.0)
                    );
                    assert_eq!(
                        fixture.host.element(i).bounds(),
                        axis.rect(
                            (i % count) as f64 * (item_minor + spacing),
                            (i / count) as f64 * (30.0 + spacing),
                            item_minor,
                            30.0
                        )
                    );
                }
                assert_eq!(
                    axis.major(desired),
                    ((10 + count - 1) / count) as f64 * (30.0 + spacing) - spacing
                );
            }
        }
    }
}

#[repr(C)]
struct InspectingFlowLayout {
    base: FlowLayout,
    lines: RefCell<Vec<(i32, i32, f64)>>,
    forced_extent: Cell<Option<Rect>>,
}
ferro_class!(InspectingFlowLayout: FlowLayout);
ferro_impl_classes!(InspectingFlowLayout: FerroObjectImpl, LayoutImpl, VirtualizingLayoutImpl);
impl FlowLayoutImpl for InspectingFlowLayout {
    fn on_line_arranged(
        this: &Self,
        index: i32,
        count: i32,
        size: f64,
        context: &Ref<VirtualizingLayoutContext>,
    ) {
        Self::parent_on_line_arranged(this, index, count, size, context);
        this.lines.borrow_mut().push((index, count, size));
    }
    fn get_extent(
        this: &Self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        first: FlowLayoutRealizedElement,
        last: FlowLayoutRealizedElement,
    ) -> Rect {
        this.forced_extent
            .get()
            .unwrap_or_else(|| Self::parent_get_extent(this, available, context, first, last))
    }
}
impl InspectingFlowLayout {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: FlowLayout::construct(),
            lines: RefCell::new(Vec::new()),
            forced_extent: Cell::new(None),
        })
    }
}

#[test]
fn verify_flow_layout_on_line_arranged() {
    for axis in [Axis(true), Axis(false)] {
        let layout = InspectingFlowLayout::new();
        layout.set_orientation(axis.orientation(false));
        let fixture = Fixture::new(
            layout.clone().upcast(),
            vec![Size::new(50.0, 50.0); 5],
            axis.size(100.0, f64::INFINITY),
            axis.rect(0.0, 0.0, 100.0, 1000.0),
        );
        fixture.measure();
        assert_eq!(
            &*layout.lines.borrow(),
            &[(0, 2, 50.0), (2, 2, 50.0), (4, 1, 50.0)]
        );
    }
}

fn estimations(axis: Axis, choice: i32) {
    let flow = InspectingFlowLayout::new();
    let layout: Ref<Layout> = match choice {
        0 => {
            let layout = StackLayout::new();
            layout.set_orientation(axis.orientation(true));
            layout.set_spacing(10.0);
            layout.upcast()
        }
        1 => {
            let layout = UniformGridLayout::new();
            layout.set_orientation(axis.orientation(false));
            layout.set_min_item_width(90.0);
            layout.set_min_item_height(90.0);
            if axis.0 {
                layout.set_min_row_spacing(10.0)
            } else {
                layout.set_min_column_spacing(10.0)
            };
            layout.upcast()
        }
        _ => {
            flow.set_orientation(axis.orientation(false));
            flow.set_line_spacing(10.0);
            flow.clone().upcast()
        }
    };
    let fixture = Fixture::new(
        layout,
        vec![Size::new(90.0, 90.0); 1000],
        axis.size(400.0, f64::INFINITY),
        axis.rect(0.0, 0.0, 400.0, 400.0),
    );
    let expected = if choice == 0 { 99_990.0 } else { 24_990.0 };
    for offset in [0.0, 200.0, 20_000.0] {
        fixture
            .host
            .window
            .set(axis.rect(0.0, offset, 400.0, 400.0));
        assert_eq!(axis.major(fixture.measure()), expected);
        assert!(fixture.host.live.borrow().len() < 32);
        if offset == 20_000.0 {
            let mut actual: Vec<_> = fixture.host.live.borrow().iter().map(|(i, _)| *i).collect();
            actual.sort_unstable();
            let expected_indices: Vec<_> = if choice == 0 {
                (199..205).collect()
            } else {
                (799..817).collect()
            };
            assert_eq!(actual, expected_indices);
        }
    }
    if choice == 2 {
        assert!(flow
            .lines
            .borrow()
            .iter()
            .all(|(start, count, size)| start % 4 == 0 && *count == 4 && *size == 90.0));
        flow.forced_extent
            .set(Some(axis.rect(0.0, 0.0, 400.0, 22_500.0)));
    }
    fixture
        .host
        .window
        .set(axis.rect(0.0, 10_000.0, 400.0, 400.0));
    assert_eq!(
        axis.major(fixture.measure()),
        if choice == 2 { 22_500.0 } else { expected }
    );
}
#[test]
fn validate_estimations_stack_horizontal() {
    estimations(Axis(false), 0);
}
#[test]
fn validate_estimations_stack_vertical() {
    estimations(Axis(true), 0);
}
#[test]
fn validate_estimations_grid_horizontal() {
    estimations(Axis(false), 1);
}
#[test]
fn validate_estimations_grid_vertical() {
    estimations(Axis(true), 1);
}
#[test]
fn validate_estimations_flow_horizontal() {
    estimations(Axis(false), 2);
}
#[test]
fn validate_estimations_flow_vertical() {
    estimations(Axis(true), 2);
}

#[test]
fn validate_make_anchor() {
    for axis in [Axis(true), Axis(false)] {
        let layout = FlowLayout::new();
        layout.set_orientation(axis.orientation(false));
        let fixture = Fixture::new(
            layout.upcast(),
            vec![Size::new(90.0, 90.0); 1000],
            axis.size(400.0, f64::INFINITY),
            axis.rect(0.0, 0.0, 400.0, 400.0),
        );
        fixture.measure();
        let created = fixture.host.created.get();
        let first = fixture.host.element(1);
        assert!(fixture.context.get_or_create_element_at(1).ptr_eq(&first));
        assert_eq!(fixture.host.created.get(), created);
        let old: Vec<_> = fixture
            .host
            .live
            .borrow()
            .iter()
            .map(|(_, e)| e.clone())
            .collect();
        let anchor = fixture.context.get_or_create_element_at(900);
        assert_eq!(fixture.host.created.get(), created + 1);
        fixture.host.anchor.set(900);
        fixture.measure();
        assert!(fixture.host.element(900).ptr_eq(&anchor));
        assert!(!fixture
            .host
            .live
            .borrow()
            .iter()
            .any(|(_, e)| old.iter().any(|previous| previous.ptr_eq(e))));
        assert!(fixture.host.live.borrow().len() > 1 && fixture.host.live.borrow().len() < 150);
    }
}

#[test]
fn validate_layout_with_infinite_measure_size_in_non_virtualizing_direction() {
    for axis in [Axis(true), Axis(false)] {
        let stack = StackLayout::new();
        stack.set_orientation(axis.orientation(true));
        stack.set_spacing(10.0);
        let grid = UniformGridLayout::new();
        grid.set_orientation(axis.orientation(false));
        grid.set_min_item_width(40.0);
        grid.set_min_item_height(40.0);
        grid.set_min_row_spacing(10.0);
        grid.set_min_column_spacing(10.0);
        let flow = FlowLayout::new();
        flow.set_orientation(axis.orientation(false));
        flow.set_line_spacing(10.0);
        flow.set_min_item_spacing(10.0);
        for (layout, expected) in [
            (stack.upcast::<Layout>(), axis.size(160.0, 430.0)),
            (grid.upcast::<Layout>(), axis.size(190.0, 40.0)),
            (flow.upcast::<Layout>(), axis.size(430.0, 160.0)),
        ] {
            let fixture = Fixture::new(
                layout,
                (1..=4)
                    .map(|i| Size::new(i as f64 * 40.0, i as f64 * 40.0))
                    .collect(),
                Size::new(f64::INFINITY, f64::INFINITY),
                Rect::new(0.0, 0.0, 600.0, 600.0),
            );
            assert_eq!(fixture.measure(), expected);
        }
    }
}

#[test]
fn validate_intersection_with_realization_window() {
    for axis in [Axis(true), Axis(false)] {
        let stack = StackLayout::new();
        stack.set_orientation(axis.orientation(true));
        stack.set_spacing(10.0);
        let grid = UniformGridLayout::new();
        grid.set_orientation(axis.orientation(false));
        grid.set_min_item_width(40.0);
        grid.set_min_item_height(40.0);
        grid.set_min_row_spacing(10.0);
        grid.set_min_column_spacing(10.0);
        let flow = FlowLayout::new();
        flow.set_orientation(axis.orientation(false));
        flow.set_line_spacing(10.0);
        flow.set_min_item_spacing(10.0);
        for layout in [
            stack.upcast::<Layout>(),
            grid.upcast::<Layout>(),
            flow.upcast::<Layout>(),
        ] {
            let fixture = Fixture::new(
                layout,
                vec![Size::new(40.0, 40.0); 4],
                axis.size(200.0, f64::INFINITY),
                axis.rect(0.0, -300.0, 200.0, 200.0),
            );
            let desired = fixture.measure();
            assert!(axis.major(desired) > 0.0);
            assert!(fixture.host.live.borrow().is_empty());
            fixture
                .host
                .window
                .set(axis.rect(0.0, -300.0, 200.0, 800.0));
            fixture.measure();
            assert_eq!(fixture.host.live.borrow().len(), 4);
            for index in 0..4 {
                assert_eq!(
                    fixture.host.element(index).desired_size(),
                    Size::new(40.0, 40.0)
                );
            }
        }
    }
}

#[test]
fn validate_flow_layout_with_one_item_has_non_zero_extent() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let layout = FlowLayout::new();
        layout.set_orientation(orientation);
        let fixture = Fixture::new(
            layout.upcast(),
            vec![Size::new(40.0, 20.0)],
            Size::new(400.0, 400.0),
            Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        let size = fixture.measure();
        assert!(size.width > 10.0 && size.height > 10.0);
        assert_eq!(
            fixture.host.element(0).bounds().size(),
            Size::new(40.0, 20.0)
        );
    }
}

#[test]
fn validate_grid_layout_with_same_orientation_as_scrolling() {
    for axis in [Axis(true), Axis(false)] {
        let layout = UniformGridLayout::new();
        layout.set_orientation(axis.orientation(true));
        let fixture = Fixture::new(
            layout.upcast(),
            vec![Size::new(100.0, 100.0); 1000],
            axis.size(400.0, f64::INFINITY),
            axis.rect(0.0, 0.0, 400.0, 200.0),
        );
        for offset in [0.0, 80.0, 160.0, 240.0] {
            fixture
                .host
                .window
                .set(axis.rect(0.0, offset, 400.0, 200.0));
            fixture.measure();
            let live = fixture.host.live.borrow();
            assert!(!live.is_empty() && live.len() < 10);
            let first = live.iter().map(|(i, _)| *i).min().unwrap();
            let last = live.iter().map(|(i, _)| *i).max().unwrap();
            assert!(last - first < 10);
        }
    }
}

#[test]
fn verify_unconstrained_uniform_grid_layout() {
    for stretch in [
        UniformGridLayoutItemsStretch::Fill,
        UniformGridLayoutItemsStretch::Uniform,
    ] {
        let layout = UniformGridLayout::new();
        layout.set_min_item_width(100.0);
        layout.set_min_item_height(100.0);
        layout.set_items_stretch(stretch);
        let fixture = Fixture::new(
            layout.upcast(),
            vec![Size::default()],
            Size::new(f64::INFINITY, f64::INFINITY),
            Rect::new(0.0, 0.0, 500.0, 500.0),
        );
        assert_eq!(fixture.measure(), Size::new(100.0, 100.0));
        assert_eq!(
            fixture.host.element(0).bounds(),
            Rect::new(0.0, 0.0, 100.0, 100.0)
        );
    }
}

#[test]
fn verify_items_get_full_space_in_major_direction_when_smaller_than_line_size() {
    let fixture = Fixture::new(
        FlowLayout::new().upcast(),
        vec![
            Size::new(300.0, 300.0),
            Size::new(350.0, 150.0),
            Size::new(300.0, 300.0),
            Size::new(350.0, 250.0),
        ],
        Size::new(800.0, f64::INFINITY),
        Rect::new(0.0, 0.0, 800.0, 1000.0),
    );
    fixture.measure();
    for (index, bounds) in [
        Rect::new(0.0, 0.0, 300.0, 300.0),
        Rect::new(300.0, 0.0, 350.0, 300.0),
        Rect::new(0.0, 300.0, 300.0, 300.0),
        Rect::new(300.0, 300.0, 350.0, 300.0),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(fixture.host.element(index as i32).bounds(), bounds);
    }
}

#[test]
fn validate_stack_layout_arrange_with_stretch() {
    let fixture = Fixture::new(
        StackLayout::new().upcast(),
        vec![Size::new(20.0, 30.0); 10],
        Size::new(f64::INFINITY, f64::INFINITY),
        Rect::new(0.0, 0.0, 1000.0, 400.0),
    );
    fixture
        .layout
        .measure(&base(&fixture.context), fixture.available);
    fixture
        .layout
        .arrange(&base(&fixture.context), Size::new(1000.0, 400.0));
    for index in 0..10 {
        assert_eq!(fixture.host.element(index).bounds().width, 1000.0);
    }
}

#[test]
fn validate_stack_layout_arrange_with_insertion() {
    let layout = StackLayout::new();
    layout.set_orientation(Orientation::Horizontal);
    let fixture = Fixture::new(
        layout.upcast(),
        vec![Size::new(64.0, 32.0); 3],
        Size::new(1000.0, 32.0),
        Rect::new(0.0, 0.0, 1000.0, 32.0),
    );
    for insertion in -1..3 {
        if insertion >= 0 {
            fixture
                .host
                .sizes
                .borrow_mut()
                .insert(insertion as usize, Size::new(64.0, 32.0));
            for (index, _) in fixture.host.live.borrow_mut().iter_mut() {
                if *index >= insertion {
                    *index += 1;
                }
            }
            let new: [Option<BoxedValue>; 1] = [None];
            fixture.layout.on_items_changed(
                &base(&fixture.context),
                &ItemsChangedEventArgs {
                    action: NotifyCollectionChangedAction::Add,
                    old_items: (&[]).into(),
                    new_items: (&new).into(),
                    old_starting_index: -1,
                    new_starting_index: insertion,
                },
            );
        }
        fixture.measure();
        for index in 0..fixture.host.item_count() {
            assert_eq!(
                fixture.host.element(index).bounds(),
                Rect::new(index as f64 * 64.0, 0.0, 64.0, 32.0)
            );
        }
    }
}

#[repr(C)]
struct AspectRatioRespectingControl {
    base: Control,
}
ferro_class!(AspectRatioRespectingControl: Control);
ferro_impl_classes!(AspectRatioRespectingControl: FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl AspectRatioRespectingControl {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
        })
    }
    fn two_by_one(size: Size) -> Size {
        if !size.height.is_finite() {
            Size::new(size.width / 2.0, size.width)
        } else if !size.width.is_finite() {
            Size::new(size.height / 2.0, size.height)
        } else {
            Size::new((size.height / 2.0).min(100.0), size.height.min(100.0))
        }
    }
}
impl LayoutableImpl for AspectRatioRespectingControl {
    fn measure_override(_this: &Self, available: Size) -> Size {
        Self::two_by_one(available)
    }
    fn arrange_override(_this: &Self, final_size: Size) -> Size {
        Self::two_by_one(final_size)
    }
}

#[test]
fn validate_uniform_grid_with_items_relying_on_aspect_ratio() {
    let _scope = crate::test_support::test_scope();
    let panel = LayoutPanel::new();
    let layout = UniformGridLayout::new();
    layout.set_orientation(Orientation::Vertical);
    layout.set_maximum_rows_or_columns(1);
    layout.set_min_item_width(100.0);
    panel.set_layout(Some(layout.upcast()));
    panel.set_max_width(300.0);
    for _ in 0..6 {
        panel
            .children()
            .add(AspectRatioRespectingControl::new().upcast::<Control>());
    }
    panel.measure(Size::new(f64::INFINITY, f64::INFINITY));
    close(panel.desired_size().width, 300.0);
    assert!(panel.desired_size().height > 1.0);
    panel.arrange(Rect::new(
        0.0,
        0.0,
        panel.desired_size().width,
        panel.desired_size().height,
    ));
    for element in panel.children().snapshot().iter() {
        assert!(element.bounds().width.is_finite() && element.bounds().height > 0.0);
    }
}

// FlowLayoutCollectionChangeTests.VerifyElement0OwnershipInUniformGridLayout.
#[test]
fn verify_element0_ownership_in_uniform_grid_layout() {
    let fixture = Fixture::new(
        UniformGridLayout::new().upcast(),
        Vec::new(),
        Size::new(200.0, 200.0),
        Rect::new(0.0, 0.0, 200.0, 200.0),
    );
    assert_eq!(fixture.measure(), Size::default());
    fixture.insert_item(0, Size::new(50.0, 50.0));
    fixture.insert_item(1, Size::new(50.0, 50.0));
    fixture.measure();
    assert_eq!(fixture.host.live.borrow().len(), 2);
    let first = fixture.host.element(0);
    let second = fixture.host.element(1);
    for (index, _) in fixture.host.live.borrow_mut().iter_mut() {
        if *index == 0 {
            *index = -1;
        }
    }
    let items: [Option<BoxedValue>; 1] = [None];
    fixture.layout.on_items_changed(
        &base(&fixture.context),
        &ItemsChangedEventArgs {
            action: NotifyCollectionChangedAction::Replace,
            old_items: (&items).into(),
            new_items: (&items).into(),
            old_starting_index: 0,
            new_starting_index: 0,
        },
    );
    fixture.measure();
    assert_eq!(fixture.host.live.borrow().len(), 2);
    assert!(!fixture.host.element(0).ptr_eq(&first));
    assert!(fixture.host.element(1).ptr_eq(&second));
    fixture.remove_first();
    fixture.remove_first();
    fixture.measure();
    assert!(fixture.host.live.borrow().is_empty());
    assert_eq!(
        fixture.host.created.get(),
        fixture.host.recycled.borrow().len()
    );
}
