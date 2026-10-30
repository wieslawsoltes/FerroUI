// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Behavioral ports of controls/dev/Repeater/APITests/{FlowLayoutTests,LayoutTests}.cs.
// The deterministic host replaces the native window and records actual recycling.

use super::*;
use ferroui_controls::{Control, ControlImpl};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Point, Rect, Ref,
    Size, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[repr(C)]
struct MeasureItem {
    base: Control,
    size: Cell<Size>,
}
ferro_class!(MeasureItem: Control);
ferro_impl_classes!(MeasureItem: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);
impl LayoutableImpl for MeasureItem {
    fn measure_override(this: &Self, _available: Size) -> Size {
        this.size.get()
    }
}
impl MeasureItem {
    fn new(size: Size) -> Ref<Self> {
        let item = instantiate(Self {
            base: Control::construct(),
            size: Cell::new(size),
        });
        item.set_use_layout_rounding(false);
        item
    }
}

pub(super) struct TestHost {
    pub(super) sizes: RefCell<Vec<Size>>,
    pub(super) live: RefCell<Vec<(i32, Ref<Control>)>>,
    pub(super) state: RefCell<Option<BoxedValue>>,
    pub(super) window: Cell<Rect>,
    pub(super) origin: Cell<Point>,
    pub(super) anchor: Cell<i32>,
    pub(super) created: Cell<usize>,
    pub(super) recycled: RefCell<Vec<i32>>,
}
impl TestHost {
    pub(super) fn new(sizes: Vec<Size>, window: Rect) -> Rc<Self> {
        Rc::new(Self {
            sizes: RefCell::new(sizes),
            live: RefCell::new(Vec::new()),
            state: RefCell::new(None),
            window: Cell::new(window),
            origin: Cell::new(Point::default()),
            anchor: Cell::new(-1),
            created: Cell::new(0),
            recycled: RefCell::new(Vec::new()),
        })
    }
    pub(super) fn uniform(count: usize, size: Size) -> Rc<Self> {
        Self::new(vec![size; count], Rect::new(0.0, 0.0, 1000.0, 1000.0))
    }
    pub(super) fn context(self: &Rc<Self>) -> Ref<VirtualizingLayoutContext> {
        VirtualizingLayoutContext::with_host(self.clone())
    }
    pub(super) fn element(&self, index: i32) -> Ref<Control> {
        self.live
            .borrow()
            .iter()
            .rev()
            .find(|(i, _)| *i == index)
            .expect("element is realized")
            .1
            .clone()
    }
    pub(super) fn set_size(&self, index: usize, size: Size) {
        self.sizes.borrow_mut()[index] = size;
        let elements: Vec<_> = self
            .live
            .borrow()
            .iter()
            .filter(|(i, _)| *i == index as i32)
            .map(|(_, e)| e.clone())
            .collect();
        for element in elements {
            element
                .cast::<MeasureItem>()
                .expect("test item")
                .size
                .set(size);
            element.invalidate_measure();
        }
    }
}
impl LayoutContextHost for TestHost {
    fn item_count(&self) -> i32 {
        self.sizes.borrow().len() as i32
    }
    fn get_item_at(&self, index: i32) -> Option<BoxedValue> {
        (index >= 0 && index < self.item_count()).then(|| Rc::new(index) as BoxedValue)
    }
    fn get_or_create_element_at(
        &self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        // A programmatically made anchor is reused even with ForceCreate.
        // Repeater's ViewManager performs this lookup after the ordinary
        // layout-owned lookup, which ForceCreate alone is allowed to skip.
        if !options.contains(ElementRealizationOptions::FORCE_CREATE) || self.anchor.get() == index
        {
            let existing = self
                .live
                .borrow()
                .iter()
                .rev()
                .find(|(i, _)| *i == index)
                .map(|(_, element)| element.clone());
            if let Some(element) = existing {
                return element;
            }
        }
        let size = self.sizes.borrow()[index as usize];
        let element: Ref<Control> = MeasureItem::new(size).upcast();
        self.created.set(self.created.get() + 1);
        self.live.borrow_mut().push((index, element.clone()));
        element
    }
    fn recycle_element(&self, element: &Ref<Control>) {
        let index = self
            .live
            .borrow()
            .iter()
            .position(|(_, candidate)| candidate.ptr_eq(element));
        if let Some(index) = index {
            let (data_index, _) = self.live.borrow_mut().remove(index);
            self.recycled.borrow_mut().push(data_index);
        }
    }
    fn layout_state(&self) -> Option<BoxedValue> {
        self.state.borrow().clone()
    }
    fn set_layout_state(&self, value: Option<BoxedValue>) {
        *self.state.borrow_mut() = value;
    }
    fn realization_rect(&self) -> Rect {
        self.window.get()
    }
    fn recommended_anchor_index(&self) -> i32 {
        self.anchor.get()
    }
    fn layout_origin(&self) -> Point {
        self.origin.get()
    }
    fn set_layout_origin(&self, value: Point) {
        self.origin.set(value);
    }
}

pub(super) fn base(context: &Ref<VirtualizingLayoutContext>) -> Ref<LayoutContext> {
    context.clone().upcast()
}
pub(super) fn run(
    layout: &Layout,
    context: &Ref<VirtualizingLayoutContext>,
    available: Size,
) -> Size {
    let desired = layout.measure(&base(context), available);
    let final_size = Size::new(
        if available.width.is_finite() {
            available.width
        } else {
            desired.width
        },
        if available.height.is_finite() {
            available.height
        } else {
            desired.height
        },
    );
    layout.arrange(&base(context), final_size);
    desired
}
fn transpose(size: Size, horizontal: bool) -> Size {
    if horizontal {
        size
    } else {
        Size::new(size.height, size.width)
    }
}
fn transpose_rect(rect: Rect, horizontal: bool) -> Rect {
    if horizontal {
        rect
    } else {
        Rect::new(rect.y, rect.x, rect.height, rect.width)
    }
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn validate_stack_layout() {
    for vertical in [true, false] {
        let host = TestHost::uniform(10, transpose(Size::new(50.0, 20.0), vertical));
        let context = host.context();
        let layout = StackLayout::new();
        layout.set_orientation(if vertical {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        });
        layout.set_spacing(5.0);
        layout.initialize_for_context(&base(&context));
        let desired = run(
            &layout,
            &context,
            transpose(Size::new(100.0, f64::INFINITY), vertical),
        );
        assert_eq!(desired, transpose(Size::new(50.0, 245.0), vertical));
        for i in 0..10 {
            assert_eq!(
                host.element(i).bounds(),
                transpose_rect(Rect::new(0.0, i as f64 * 25.0, 100.0, 20.0), vertical)
            );
        }
        layout.uninitialize_for_context(&base(&context));
        assert!(host.live.borrow().is_empty());
        assert!(context.layout_state().is_none());
    }
}

// Additional regression: distant scrolling must bound both realization and work.
#[test]
fn stack_scroll_recycles_out_of_window_containers() {
    let host = TestHost::new(
        vec![Size::new(100.0, 20.0); 100_000],
        Rect::new(0.0, 0.0, 100.0, 60.0),
    );
    let context = host.context();
    let layout = StackLayout::new();
    layout.initialize_for_context(&base(&context));
    assert_eq!(
        run(&layout, &context, Size::new(100.0, f64::INFINITY)),
        Size::new(100.0, 2_000_000.0)
    );
    let first = host.element(0);
    host.window.set(Rect::new(0.0, 20_000.0, 100.0, 60.0));
    assert_eq!(
        run(&layout, &context, Size::new(100.0, f64::INFINITY)),
        Size::new(100.0, 2_000_000.0)
    );
    assert!(host.recycled.borrow().contains(&0));
    assert!(!host.live.borrow().iter().any(|(_, e)| e.ptr_eq(&first)));
    assert!(host.live.borrow().iter().any(|(i, _)| *i == 1000));
    assert!(host.live.borrow().len() < 12);
    assert!(
        host.created.get() < 24,
        "distant scroll must not realize intervening data"
    );
}

#[test]
fn validate_stack_layout_disabled_virtualization_with_items_repeater() {
    let host = TestHost::new(
        vec![Size::new(50.0, 20.0); 100],
        Rect::new(0.0, 0.0, 100.0, 40.0),
    );
    let context = host.context();
    let layout = StackLayout::new();
    layout.set_is_virtualization_enabled(false);
    layout.initialize_for_context(&base(&context));
    assert_eq!(
        run(&layout, &context, Size::new(100.0, f64::INFINITY)).height,
        2000.0
    );
    assert_eq!(host.live.borrow().len(), 100);
    assert_eq!(host.element(99).bounds().y, 1980.0);
}

#[test]
fn validate_default_size_for_grid_layout_items_is_based_on_first_item() {
    for horizontal in [true, false] {
        let host = TestHost::new(
            vec![
                transpose(Size::new(50.0, 20.0), horizontal),
                transpose(Size::new(10.0, 10.0), horizontal),
                transpose(Size::new(100.0, 100.0), horizontal),
            ],
            Rect::new(0.0, 0.0, 1000.0, 1000.0),
        );
        let context = host.context();
        let layout = UniformGridLayout::new();
        layout.set_orientation(if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        });
        layout.initialize_for_context(&base(&context));
        run(
            &layout,
            &context,
            transpose(Size::new(100.0, f64::INFINITY), horizontal),
        );
        let state = UniformGridLayout::state(&context).unwrap();
        assert_eq!(
            state.effective_size(),
            transpose(Size::new(50.0, 20.0), horizontal)
        );
        for i in 0..3 {
            assert_eq!(
                state
                    .flow_algorithm
                    .element_manager
                    .layout_bounds_for_data_index(i),
                transpose_rect(
                    Rect::new((i % 2) as f64 * 50.0, (i / 2) as f64 * 20.0, 50.0, 20.0),
                    horizontal
                )
            );
        }
    }
}

#[test]
fn validate_resizing_first_item_resizes_other_items_in_grid_layout() {
    let host = TestHost::uniform(6, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = UniformGridLayout::new();
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, f64::INFINITY));
    host.set_size(0, Size::new(32.0, 40.0));
    run(&layout, &context, Size::new(100.0, f64::INFINITY));
    let state = UniformGridLayout::state(&context).unwrap();
    assert_eq!(state.effective_size(), Size::new(32.0, 40.0));
    assert_eq!(
        state
            .flow_algorithm
            .element_manager
            .layout_bounds_for_data_index(5),
        Rect::new(64.0, 40.0, 32.0, 40.0)
    );
}

#[test]
fn validate_default_size_for_first_item_doesnt_override_preset_one() {
    let host = TestHost::uniform(6, Size::new(100.0, 100.0));
    let context = host.context();
    let layout = UniformGridLayout::new();
    layout.set_min_item_width(50.0);
    layout.set_min_item_height(20.0);
    layout.initialize_for_context(&base(&context));
    run(&layout, &context, Size::new(100.0, f64::INFINITY));
    assert_eq!(
        UniformGridLayout::state(&context).unwrap().effective_size(),
        Size::new(50.0, 20.0)
    );
    host.set_size(0, Size::new(200.0, 200.0));
    run(&layout, &context, Size::new(100.0, f64::INFINITY));
    assert_eq!(
        UniformGridLayout::state(&context).unwrap().effective_size(),
        Size::new(50.0, 20.0)
    );
}

#[test]
fn validate_uniform_grid_with_available_width_narrower_than_one_item() {
    let host = TestHost::uniform(6, Size::new(100.0, 50.0));
    let context = host.context();
    let layout = UniformGridLayout::new();
    layout.set_min_item_width(100.0);
    layout.set_min_item_height(50.0);
    layout.initialize_for_context(&base(&context));
    let desired = run(&layout, &context, Size::new(5.0, f64::INFINITY));
    assert_eq!(desired, Size::new(100.0, 300.0));
    assert_eq!(
        UniformGridLayout::state(&context)
            .unwrap()
            .flow_algorithm
            .element_manager
            .layout_bounds_for_data_index(5),
        Rect::new(0.0, 250.0, 100.0, 50.0)
    );
}

#[test]
fn validate_grid_layout_maximum_rows_or_columns() {
    for (maximum, columns) in [(0, 1), (1, 1), (3, 3), (-1, 5)] {
        let host = TestHost::uniform(10, Size::new(20.0, 10.0));
        let context = host.context();
        let layout = UniformGridLayout::new();
        layout.set_maximum_rows_or_columns(maximum);
        layout.initialize_for_context(&base(&context));
        let desired = run(&layout, &context, Size::new(100.0, f64::INFINITY));
        assert_eq!(
            desired.height,
            ((10_i32 + columns - 1) / columns) as f64 * 10.0
        );
        assert_eq!(
            UniformGridLayout::state(&context)
                .unwrap()
                .flow_algorithm
                .element_manager
                .layout_bounds_for_data_index(9)
                .y,
            (9 / columns) as f64 * 10.0
        );
    }
}

#[test]
fn validate_uniform_grid_with_no_items() {
    let host = TestHost::uniform(0, Size::new(100.0, 50.0));
    let context = host.context();
    let layout = UniformGridLayout::new();
    layout.initialize_for_context(&base(&context));
    assert_eq!(
        run(&layout, &context, Size::new(100.0, f64::INFINITY)),
        Size::default()
    );
    assert_eq!(host.created.get(), 0);
}

#[test]
fn validate_flow_layout_varying_heights() {
    for horizontal in [true, false] {
        for spacing in [0.0, 10.0] {
            let sizes = (0..10)
                .map(|i| {
                    transpose(
                        Size::new(50.0, if i % 2 == 0 { 50.0 } else { 100.0 }),
                        horizontal,
                    )
                })
                .collect();
            let host = TestHost::new(sizes, Rect::new(0.0, 0.0, 1000.0, 1000.0));
            let context = host.context();
            let layout = FlowLayout::new();
            layout.set_orientation(if horizontal {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            });
            layout.set_min_item_spacing(spacing);
            layout.set_line_spacing(spacing);
            layout.initialize_for_context(&base(&context));
            run(
                &layout,
                &context,
                transpose(Size::new(180.0, f64::INFINITY), horizontal),
            );
            for i in 0..10 {
                assert_eq!(
                    host.element(i).bounds(),
                    transpose_rect(
                        Rect::new(
                            (i % 3) as f64 * (50.0 + spacing),
                            (i / 3) as f64 * (100.0 + spacing),
                            50.0,
                            100.0
                        ),
                        horizontal
                    )
                );
            }
        }
    }
}

#[test]
fn verify_flow_layout_line_alignment_positions() {
    let cases = [
        (FlowLayoutLineAlignment::Start, [0.0, 100.0, 200.0]),
        (FlowLayoutLineAlignment::End, [200.0, 300.0, 400.0]),
        (FlowLayoutLineAlignment::Center, [100.0, 200.0, 300.0]),
        (
            FlowLayoutLineAlignment::SpaceAround,
            [200.0 / 6.0, 200.0, 400.0 - 200.0 / 6.0],
        ),
        (FlowLayoutLineAlignment::SpaceBetween, [0.0, 200.0, 400.0]),
        (FlowLayoutLineAlignment::SpaceEvenly, [50.0, 200.0, 350.0]),
    ];
    for (alignment, offsets) in cases {
        let host = TestHost::uniform(3, Size::new(100.0, 20.0));
        let context = host.context();
        let layout = FlowLayout::new();
        layout.set_line_alignment(alignment);
        layout.initialize_for_context(&base(&context));
        run(&layout, &context, Size::new(500.0, f64::INFINITY));
        for (i, offset) in offsets.into_iter().enumerate() {
            close(host.element(i as i32).bounds().x, offset);
        }
    }
}

#[test]
fn validate_layout_sharing() {
    for layout in [
        StackLayout::new().upcast::<Layout>(),
        FlowLayout::new().upcast::<Layout>(),
        UniformGridLayout::new().upcast::<Layout>(),
    ] {
        let first = TestHost::uniform(3, Size::new(20.0, 10.0));
        let second = TestHost::uniform(7, Size::new(40.0, 30.0));
        let first_context = first.context();
        let second_context = second.context();
        layout.initialize_for_context(&base(&first_context));
        let expected = run(&layout, &first_context, Size::new(100.0, f64::INFINITY));
        let state = first_context.layout_state().unwrap();
        layout.initialize_for_context(&base(&second_context));
        run(&layout, &second_context, Size::new(100.0, f64::INFINITY));
        assert!(!Rc::ptr_eq(&state, &second_context.layout_state().unwrap()));
        assert_eq!(
            run(&layout, &first_context, Size::new(100.0, f64::INFINITY)),
            expected
        );
        layout.uninitialize_for_context(&base(&second_context));
        assert_eq!(
            run(&layout, &first_context, Size::new(100.0, f64::INFINITY)),
            expected
        );
    }
}

#[test]
fn verify_virtualizing_layout_hosted_by_non_virtualizing_layout_context() {
    let host = TestHost::uniform(4, Size::new(50.0, 20.0));
    let nonvirtual = NonVirtualizingLayoutContext::with_host(host.clone());
    let context = nonvirtual.as_virtualizing_context();
    assert!(context.ptr_eq(&nonvirtual.as_virtualizing_context()));
    assert_eq!(context.item_count(), 4);
    assert_eq!(
        context.realization_rect(),
        Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY)
    );
    assert_eq!(context.visible_rect(), context.realization_rect());
    assert_eq!(context.recommended_anchor_index(), -1);
    assert_eq!(context.layout_origin(), Point::default());
    for i in 0..4 {
        let element = context.get_or_create_element_at(i);
        assert!(ValueTypes::as_object(&*context.get_item_at(i).unwrap())
            .unwrap()
            .cast::<Control>()
            .unwrap()
            .ptr_eq(&element));
        assert!(context
            .get_or_create_element_at_with_options(i, ElementRealizationOptions::FORCE_CREATE)
            .ptr_eq(&element));
    }
    let first = context.get_or_create_element_at(0);
    context.recycle_element(&first);
    assert!(context.get_or_create_element_at(0).ptr_eq(&first));
    assert!(host.recycled.borrow().is_empty());
    context.set_layout_origin(Point::default());
    assert!(catch_unwind(AssertUnwindSafe(
        || context.set_layout_origin(Point::new(5.0, 0.0))
    ))
    .is_err());
    let state: BoxedValue = Rc::new(String::from("state"));
    context.set_layout_state(Some(state.clone()));
    assert!(Rc::ptr_eq(&state, &nonvirtual.layout_state().unwrap()));
    let ordinary = host.context();
    assert_eq!(ordinary.as_non_virtualizing_context().children().len(), 4);
}

#[repr(C)]
struct DerivedStackLayoutState {
    base: StackLayoutState,
}
ferro_class!(DerivedStackLayoutState: StackLayoutState);
ferro_impl_classes!(DerivedStackLayoutState: FerroObjectImpl);

#[test]
fn verify_built_in_layouts_adopt_or_reject_existing_layout_state() {
    let host = TestHost::uniform(3, Size::new(50.0, 20.0));
    let context = host.context();
    let layout = StackLayout::new();
    let derived = instantiate(DerivedStackLayoutState {
        base: StackLayoutState::construct(),
    });
    context.set_layout_state(Some(Rc::new(derived.clone())));
    layout.initialize_for_context(&base(&context));
    assert!(StackLayout::state(&context).unwrap().ptr_eq(&derived));
    assert_eq!(
        run(&layout, &context, Size::new(200.0, f64::INFINITY)).height,
        60.0
    );
    layout.uninitialize_for_context(&base(&context));
    for layout in [
        StackLayout::new().upcast::<Layout>(),
        FlowLayout::new().upcast::<Layout>(),
        UniformGridLayout::new().upcast::<Layout>(),
    ] {
        context.set_layout_state(Some(Rc::new(String::from("foreign state"))));
        assert!(catch_unwind(AssertUnwindSafe(
            || layout.initialize_for_context(&base(&context))
        ))
        .is_err());
        context.set_layout_state(None);
        layout.initialize_for_context(&base(&context));
        run(&layout, &context, Size::new(200.0, f64::INFINITY));
        layout.uninitialize_for_context(&base(&context));
    }
}
