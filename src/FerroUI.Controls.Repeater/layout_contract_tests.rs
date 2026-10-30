// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Behavioral ports of Repeater/APITests/LayoutTests.cs. Custom class overrides
// exercise the native vtables, independently of LayoutContextHost adapters.

use super::*;
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate, IDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::{Border, Button, Control, ItemsSource, ScrollViewer, TextBlock};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue,
    FerroObjectImpl, Point, Rect, Ref, Size,
};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[repr(C)]
struct CustomVirtualizingContext {
    base: VirtualizingLayoutContext,
    elements: Vec<Ref<Control>>,
    last_options: Cell<ElementRealizationOptions>,
    last_recycled: RefCell<Option<Ref<Control>>>,
    origin: Cell<Point>,
    state: RefCell<Option<BoxedValue>>,
    state_set_count: Cell<usize>,
}
ferro_class!(CustomVirtualizingContext: VirtualizingLayoutContext);
ferro_impl_classes!(CustomVirtualizingContext: FerroObjectImpl);
impl LayoutContextImpl for CustomVirtualizingContext {
    fn layout_state_core(this: &Self) -> Option<BoxedValue> {
        this.state.borrow().clone()
    }
    fn set_layout_state_core(this: &Self, value: Option<BoxedValue>) {
        this.state_set_count.set(this.state_set_count.get() + 1);
        *this.state.borrow_mut() = value;
    }
}
impl VirtualizingLayoutContextImpl for CustomVirtualizingContext {
    fn item_count_core(this: &Self) -> i32 {
        this.elements.len() as i32
    }
    fn get_item_at_core(_this: &Self, index: i32) -> Option<BoxedValue> {
        Some(Rc::new(format!("Data {index}")))
    }
    fn get_or_create_element_at_core(
        this: &Self,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        this.last_options.set(options);
        this.elements[index as usize].clone()
    }
    fn recycle_element_core(this: &Self, element: &Ref<Control>) {
        *this.last_recycled.borrow_mut() = Some(element.clone());
    }
    fn realization_rect_core(_this: &Self) -> Rect {
        Rect::new(0.0, 10.0, 300.0, 400.0)
    }
    fn visible_rect_core(_this: &Self) -> Rect {
        Rect::new(0.0, 20.0, 300.0, 200.0)
    }
    fn recommended_anchor_index_core(_this: &Self) -> i32 {
        2
    }
    fn layout_origin_core(this: &Self) -> Point {
        this.origin.get()
    }
    fn set_layout_origin_core(this: &Self, value: Point) {
        this.origin.set(value);
    }
}
impl CustomVirtualizingContext {
    fn new(count: usize) -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayoutContext::construct(),
            elements: (0..count)
                .map(|_| {
                    let child = Border::new();
                    child.set_width(100.0);
                    child.set_height(10.0);
                    child.upcast()
                })
                .collect(),
            last_options: Cell::new(ElementRealizationOptions::all()),
            last_recycled: RefCell::new(None),
            origin: Cell::new(Point::default()),
            state: RefCell::new(None),
            state_set_count: Cell::new(0),
        })
    }
}

#[repr(C)]
struct CustomNonVirtualizingContext {
    base: NonVirtualizingLayoutContext,
    children: Vec<Ref<Control>>,
}
ferro_class!(CustomNonVirtualizingContext: NonVirtualizingLayoutContext);
ferro_impl_classes!(CustomNonVirtualizingContext: FerroObjectImpl, LayoutContextImpl);
impl NonVirtualizingLayoutContextImpl for CustomNonVirtualizingContext {
    fn children_core(this: &Self) -> Vec<Ref<Control>> {
        this.children.clone()
    }
}
impl CustomNonVirtualizingContext {
    fn new(children: Vec<Ref<Control>>) -> Ref<Self> {
        instantiate(Self {
            base: NonVirtualizingLayoutContext::construct(),
            children,
        })
    }
}

#[repr(C)]
struct ContractProbeLayout {
    base: VirtualizingLayout,
    elements: RefCell<Vec<Ref<Control>>>,
    callback: RefCell<Option<Rc<dyn Fn(&Ref<VirtualizingLayoutContext>)>>>,
    measures: Cell<usize>,
    arranges: Cell<usize>,
    provider_calls: Cell<usize>,
    fixed_extent: Cell<Option<Size>>,
}
ferro_class!(ContractProbeLayout: VirtualizingLayout);
ferro_impl_classes!(ContractProbeLayout: FerroObjectImpl);
impl LayoutImpl for ContractProbeLayout {
    fn create_default_item_transition_provider(
        this: &Self,
    ) -> Option<Ref<ItemCollectionTransitionProvider>> {
        this.provider_calls.set(this.provider_calls.get() + 1);
        None
    }
}
impl VirtualizingLayoutImpl for ContractProbeLayout {
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        this.measures.set(this.measures.get() + 1);
        let callback = this.callback.borrow().clone();
        if let Some(callback) = callback {
            callback(context);
        }
        let mut height = 0.0;
        let mut elements = Vec::new();
        for index in 0..context.item_count() {
            let element = context.get_or_create_element_at(index);
            element.measure(available);
            height += element.desired_size().height;
            elements.push(element);
        }
        *this.elements.borrow_mut() = elements;
        this.fixed_extent.get().unwrap_or(Size::new(100.0, height))
    }
    fn arrange_override(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        size: Size,
    ) -> Size {
        this.arranges.set(this.arranges.get() + 1);
        let mut y = 0.0;
        for child in this.elements.borrow().clone() {
            let height = child.desired_size().height;
            child.arrange(Rect::new(0.0, y, size.width, height));
            y += height;
        }
        size
    }
}
impl ContractProbeLayout {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            elements: RefCell::new(Vec::new()),
            callback: RefCell::new(None),
            measures: Cell::new(0),
            arranges: Cell::new(0),
            provider_calls: Cell::new(0),
            fixed_extent: Cell::new(None),
        })
    }
}

#[repr(C)]
struct WrappingNonVirtualizingLayout {
    base: NonVirtualizingLayout,
    wrapped: Ref<VirtualizingLayout>,
}
ferro_class!(WrappingNonVirtualizingLayout: NonVirtualizingLayout);
ferro_impl_classes!(WrappingNonVirtualizingLayout: FerroObjectImpl, LayoutImpl);
impl NonVirtualizingLayoutImpl for WrappingNonVirtualizingLayout {
    fn initialize_for_context_core(this: &Self, context: &Ref<NonVirtualizingLayoutContext>) {
        this.wrapped
            .initialize_for_context(&context.clone().upcast());
    }
    fn uninitialize_for_context_core(this: &Self, context: &Ref<NonVirtualizingLayoutContext>) {
        this.wrapped
            .uninitialize_for_context(&context.clone().upcast());
    }
    fn measure_override(
        this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        size: Size,
    ) -> Size {
        this.wrapped.measure(&context.clone().upcast(), size)
    }
    fn arrange_override(
        this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        size: Size,
    ) -> Size {
        this.wrapped.arrange(&context.clone().upcast(), size)
    }
}
impl WrappingNonVirtualizingLayout {
    fn new(wrapped: Ref<VirtualizingLayout>) -> Ref<Self> {
        instantiate(Self {
            base: NonVirtualizingLayout::construct(),
            wrapped,
        })
    }
}

fn button_template(height: f64) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(
        |_| true,
        move |_, _| {
            let child = Button::new();
            child.set_width(100.0);
            child.set_height(height);
            child.bind_binding(
                ferroui_controls::ContentControl::content_property().as_property(),
                &*ReflectionBinding::empty(),
            );
            Some(child.upcast())
        },
        true,
    )
}
fn viewer(repeater: &Ref<ItemsRepeater>, height: f64) -> Ref<ScrollViewer> {
    let viewer = ScrollViewer::new();
    viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::test_support::create_template,
    )));
    viewer.set_width(200.0);
    viewer.set_height(height);
    viewer.set_content(Some(Control::boxed(repeater.clone())));
    viewer
}
fn assert_panics(f: impl FnOnce()) {
    assert!(catch_unwind(AssertUnwindSafe(f)).is_err());
}

#[test]
fn verify_custom_virtualizing_layout_context_overrides_are_used() {
    let _scope = test_scope();
    let context = CustomVirtualizingContext::new(3);
    assert_eq!(context.item_count(), 3);
    assert_eq!(
        context
            .get_item_at(1)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Data 1")
    );
    assert_eq!(context.get_or_create_element_at(2), context.elements[2]);
    assert_eq!(context.last_options.get(), ElementRealizationOptions::NONE);
    let options =
        ElementRealizationOptions::FORCE_CREATE | ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE;
    assert_eq!(
        context.get_or_create_element_at_with_options(0, options),
        context.elements[0]
    );
    assert_eq!(context.last_options.get(), options);
    context.recycle_element(&context.elements[1]);
    assert_eq!(
        *context.last_recycled.borrow(),
        Some(context.elements[1].clone())
    );
    assert_eq!(
        context.realization_rect(),
        Rect::new(0.0, 10.0, 300.0, 400.0)
    );
    assert_eq!(context.visible_rect(), Rect::new(0.0, 20.0, 300.0, 200.0));
    let layout = ContractProbeLayout::new();
    assert_eq!(
        layout.measure(&context.clone().upcast(), Size::new(300.0, f64::INFINITY)),
        Size::new(100.0, 30.0)
    );
    assert_eq!(
        layout.arrange(&context.upcast(), Size::new(300.0, 30.0)),
        Size::new(300.0, 30.0)
    );
}

// The three property override cases are ignored upstream with
// PC-CONTEXT-PROPERTY-OVERRIDES. Native property calls dispatch through vtables.
#[test]
fn verify_custom_virtualizing_layout_context_property_overrides_are_used() {
    let _scope = test_scope();
    let context = CustomVirtualizingContext::new(1);
    assert_eq!(context.recommended_anchor_index(), 2);
    assert_eq!(context.layout_origin(), Point::default());
    context.set_layout_origin(Point::new(4.0, 6.0));
    assert_eq!(context.layout_origin(), Point::new(4.0, 6.0));
}
#[test]
fn verify_custom_non_virtualizing_layout_context_children_are_used() {
    let _scope = test_scope();
    let children: Vec<Ref<Control>> = (0..4)
        .map(|_| {
            let child = Border::new();
            child.set_width(25.0);
            child.set_height(10.0);
            child.upcast()
        })
        .collect();
    let context = CustomNonVirtualizingContext::new(children.clone());
    assert_eq!(context.children().len(), 4);
    assert_eq!(context.children()[3], children[3]);
    let layout = WrappingNonVirtualizingLayout::new(ContractProbeLayout::new().upcast());
    assert_eq!(
        layout.measure(&context.upcast(), Size::new(200.0, f64::INFINITY)),
        Size::new(100.0, 40.0)
    );
}
#[test]
fn verify_layout_state_core_override_is_used() {
    let _scope = test_scope();
    let context = CustomVirtualizingContext::new(1);
    let state: BoxedValue = Rc::new(42_i32);
    context.set_layout_state(Some(state.clone()));
    assert_eq!(context.state_set_count.get(), 1);
    assert!(Rc::ptr_eq(&state, &context.layout_state().unwrap()));
}

#[test]
fn verify_default_overridable_implementations() {
    let _scope = test_scope();
    let context = VirtualizingLayoutContext::new();
    assert_panics(|| {
        context.item_count();
    });
    assert_panics(|| {
        context.get_item_at(0);
    });
    assert_panics(|| {
        context.get_or_create_element_at(0);
    });
    assert_panics(|| {
        context.get_or_create_element_at_with_options(0, ElementRealizationOptions::FORCE_CREATE);
    });
    assert_panics(|| context.recycle_element(&Border::new().upcast()));
    assert_panics(|| {
        context.realization_rect();
    });
    assert_panics(|| {
        context.visible_rect();
    });
    assert_panics(|| {
        context.recommended_anchor_index();
    });
    assert_panics(|| {
        context.layout_origin();
    });
    assert_panics(|| context.set_layout_origin(Point::new(1.0, 1.0)));
    assert_panics(|| {
        context.layout_state();
    });
    assert_panics(|| context.set_layout_state(Some(Rc::new(1_i32))));
    let non_virtual_context = NonVirtualizingLayoutContext::new();
    assert_panics(|| {
        non_virtual_context.children();
    });
    let factory = ElementFactory::new();
    assert_eq!(
        factory.get_element(&ElementFactoryGetArgs::new(Some(Rc::new(1_i32)), None)),
        Err(ElementFactoryError::AbstractMethod("GetElementCore"))
    );
    assert_eq!(
        factory.recycle_element(&ElementFactoryRecycleArgs::new(
            Border::new().upcast(),
            None
        )),
        Err(ElementFactoryError::AbstractMethod("RecycleElementCore"))
    );
    let provider = ItemCollectionTransitionProvider::new();
    let transition = ItemCollectionTransition::new(
        provider.clone(),
        Border::new().upcast(),
        ItemCollectionTransitionOperation::Add,
        ItemCollectionTransitionTriggers::COLLECTION_CHANGE_ADD,
    );
    assert_panics(|| {
        provider.should_animate(transition);
    });
    assert_panics(|| provider.start_transitions(Vec::new()));
    let non_virtual = NonVirtualizingLayout::new();
    let base = non_virtual_context.upcast();
    assert_panics(|| {
        non_virtual.measure(&base, Size::new(120.0, 48.0));
    });
    assert_panics(|| {
        non_virtual.arrange(&base, Size::new(120.0, 48.0));
    });
    let virtual_layout = VirtualizingLayout::new();
    let base = context.upcast();
    assert_panics(|| {
        virtual_layout.measure(&base, Size::new(120.0, 48.0));
    });
    assert_eq!(
        virtual_layout.arrange(&base, Size::new(120.0, 48.0)),
        Size::new(120.0, 48.0)
    );
}

#[test]
fn verify_virtualizing_layout_hosted_by_non_virtualizing_layout_context() {
    let _scope = test_scope();
    let probe = ContractProbeLayout::new();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    *probe.callback.borrow_mut() = Some(Rc::new(move |context| {
        if observed.replace(true) {
            return;
        }
        assert_eq!(context.item_count(), 4);
        let infinite = Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY);
        assert_eq!(context.realization_rect(), infinite);
        assert_eq!(context.visible_rect(), infinite);
        assert_eq!(context.recommended_anchor_index(), -1);
        assert_eq!(context.layout_origin(), Point::default());
        for index in 0..4 {
            let element = context.get_or_create_element_at(index);
            assert_eq!(
                Control::from_boxed(&context.get_item_at(index).unwrap()),
                Some(element.clone())
            );
            assert_eq!(
                context
                    .get_or_create_element_at_with_options(index, ElementRealizationOptions::NONE),
                element
            );
            assert_eq!(
                element.cast::<TextBlock>().unwrap().text(),
                Some(format!("Item #{index}"))
            );
        }
        let first = context.get_or_create_element_at(0);
        context.recycle_element(&first);
        assert_eq!(context.get_or_create_element_at(0), first);
        context.set_layout_origin(Point::default());
        assert_panics(|| context.set_layout_origin(Point::new(5.0, 0.0)));
        assert_eq!(context.layout_origin(), Point::default());
    }));
    let repeater = ItemsRepeater::new();
    repeater.set_vertical_alignment(VerticalAlignment::Top);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let text = TextBlock::new();
            text.set_height(20.0);
            text.bind_binding(
                TextBlock::text_property().as_property(),
                &*ReflectionBinding::empty(),
            );
            Some(text.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(
        (0..4).map(|index| format!("Item #{index}")),
    )));
    repeater.set_layout(Some(
        WrappingNonVirtualizingLayout::new(probe.upcast()).upcast(),
    ));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    assert!(called.get());
    assert_eq!(repeater.bounds().height, 80.0);
}

#[test]
fn verify_custom_layout_orientation_and_default_transition_provider() {
    let _scope = test_scope();
    let layout = ContractProbeLayout::new();
    assert_eq!(
        layout.index_based_layout_orientation(),
        IndexBasedLayoutOrientation::None
    );
    for value in [
        IndexBasedLayoutOrientation::LeftToRight,
        IndexBasedLayoutOrientation::TopToBottom,
    ] {
        layout.set_index_based_layout_orientation(value);
        assert_eq!(layout.index_based_layout_orientation(), value);
    }
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(button_template(20.0)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..2)));
    repeater.set_layout(Some(layout.clone().upcast()));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    assert_eq!(layout.provider_calls.get(), 1);
    assert!(repeater.try_get_element(1).is_some());
}

#[test]
fn validate_non_virtual_layout_with_items_repeater() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(
        WrappingNonVirtualizingLayout::new(ContractProbeLayout::new().upcast()).upcast(),
    ));
    repeater.set_items_source(Some(ItemsSource::from_values(0..10)));
    repeater.set_data_template(Some(button_template(100.0)));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    for index in 0..10 {
        let child = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<Button>()
            .unwrap();
        assert_eq!(child.previous_arrange().unwrap().y, index as f64 * 100.0);
        assert_eq!(child.content().unwrap().downcast_ref::<i32>(), Some(&index));
    }
}

#[test]
fn verify_items_repeater_null_layout_is_treated_as_stack_layout() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_width(500.0);
    repeater.set_height(500.0);
    repeater.set_horizontal_alignment(HorizontalAlignment::Left);
    repeater.set_vertical_alignment(VerticalAlignment::Top);
    repeater.set_items_source(Some(ItemsSource::from_values(0..10)));
    repeater.set_data_template(Some(button_template(10.0)));
    let root = TestRoot::with_child(&repeater);
    assert!(repeater.layout().is_none());
    root.execute_initial_layout_pass();
    let validate = |grid: bool| {
        for index in 0..10 {
            let child = repeater
                .try_get_element(index)
                .unwrap()
                .cast::<Button>()
                .unwrap();
            let slot = child.previous_arrange().unwrap();
            assert_eq!(
                slot.y,
                if grid {
                    (index / 5) as f64 * 10.0
                } else {
                    index as f64 * 10.0
                }
            );
            if grid {
                assert_eq!(slot.x, (index % 5) as f64 * 100.0);
            }
            assert_eq!(child.content().unwrap().downcast_ref::<i32>(), Some(&index));
        }
    };
    validate(false);
    let grid = UniformGridLayout::new();
    grid.set_maximum_rows_or_columns(5);
    grid.set_min_item_height(10.0);
    grid.set_min_item_width(100.0);
    repeater.set_layout(Some(grid.upcast()));
    root.layout_manager().execute_layout_pass();
    validate(true);
    repeater.set_layout(None);
    root.layout_manager().execute_layout_pass();
    validate(false);
    assert!(repeater.layout().is_none());
}

#[test]
fn validate_non_virtual_layout_does_not_get_measured_for_viewport_changes() {
    let _scope = test_scope();
    let probe = ContractProbeLayout::new();
    probe.fixed_extent.set(Some(Size::new(100.0, 800.0)));
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(
        WrappingNonVirtualizingLayout::new(probe.clone().upcast()).upcast(),
    ));
    repeater.set_items_source(Some(ItemsSource::from_values(0..10)));
    repeater.set_data_template(Some(button_template(100.0)));
    let viewer = viewer(&repeater, 100.0);
    let root = TestRoot::with_child(&viewer);
    root.execute_initial_layout_pass();
    assert_eq!(probe.measures.get(), 1);
    assert_eq!(probe.arranges.get(), 1);
    let virtual_probe = ContractProbeLayout::new();
    virtual_probe
        .fixed_extent
        .set(Some(Size::new(100.0, 800.0)));
    repeater.set_layout(Some(virtual_probe.clone().upcast()));
    root.layout_manager().execute_layout_pass();
    assert!(virtual_probe.measures.get() > 1);
    assert!(virtual_probe.arranges.get() > 1);
}

#[test]
fn validate_stack_layout_disabled_virtualization_with_items_repeater() {
    let _scope = test_scope();
    let stack = StackLayout::new();
    stack.set_is_virtualization_enabled(false);
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(stack.upcast()));
    repeater.set_items_source(Some(ItemsSource::from_values(0..10)));
    repeater.set_data_template(Some(button_template(100.0)));
    let viewer = viewer(&repeater, 100.0);
    let root = TestRoot::with_child(&viewer);
    root.execute_initial_layout_pass();
    for index in 0..10 {
        assert!(repeater
            .try_get_element(index)
            .unwrap()
            .cast::<Button>()
            .is_some());
    }
}
