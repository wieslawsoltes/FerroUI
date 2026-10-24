// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Behavioral ports of Repeater/APITests/{AccessibilityTests,ItemTemplateTests,
// LayoutTests,RepeaterApiContractTests,RepeaterTests,ViewManagerTests}.cs.
// Native roots replace the Windows test window; additional regressions are labeled.

use super::*;
use ferroui_controls::automation::peers::{AutomationControlType, AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, Button, Control, ItemsChangedEventArgs, ItemsSource, Panel, StackPanel};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, NavigationDirection};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Point, Rect, Ref,
    Size,
};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// A deterministic realization window; it exercises the real repeater managers
/// while leaving collection invalidation policy with a custom layout.
#[repr(C)]
struct ProbeLayout {
    base: VirtualizingLayout,
    indices: RefCell<Option<Vec<i32>>>,
    live: RefCell<Vec<(i32, Ref<Control>)>>,
    recycle_index: Cell<Option<i32>>,
    measures: Cell<usize>,
    arranges: Cell<usize>,
    changes: Cell<usize>,
    items_changed: RefCell<Option<Rc<dyn Fn()>>>,
    first_origin: Cell<Option<Point>>,
    observed_origins: RefCell<Vec<Point>>,
}
ferro_class!(ProbeLayout: VirtualizingLayout);
ferro_impl_classes!(ProbeLayout: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for ProbeLayout {
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        this.measures.set(this.measures.get() + 1);
        this.observed_origins
            .borrow_mut()
            .push(context.layout_origin());
        if let Some(origin) = this.first_origin.take() {
            context.set_layout_origin(origin);
        }
        let indices = this
            .indices
            .borrow()
            .clone()
            .unwrap_or_else(|| (0..context.item_count()).collect());
        let mut live = Vec::new();
        for index in indices {
            let element = context.get_or_create_element_at(index);
            element.measure(available);
            if this.recycle_index.get() == Some(index) {
                context.recycle_element(&element);
            } else {
                live.push((index, element));
            }
        }
        *this.live.borrow_mut() = live;
        Size::new(100.0, context.item_count() as f64 * 20.0)
    }
    fn arrange_override(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        this.arranges.set(this.arranges.get() + 1);
        for (index, element) in this.live.borrow().clone() {
            element.arrange(Rect::new(0.0, index as f64 * 20.0, 100.0, 20.0));
        }
        final_size
    }
    fn on_items_changed_core(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        _args: &ItemsChangedEventArgs<'_>,
    ) {
        this.changes.set(this.changes.get() + 1);
        let callback = this.items_changed.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        this.invalidate_measure();
    }
}
impl ProbeLayout {
    fn new(indices: Option<&[i32]>) -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            indices: RefCell::new(indices.map(<[i32]>::to_vec)),
            live: RefCell::new(Vec::new()),
            recycle_index: Cell::new(None),
            measures: Cell::new(0),
            arranges: Cell::new(0),
            changes: Cell::new(0),
            items_changed: RefCell::new(None),
            first_origin: Cell::new(None),
            observed_origins: RefCell::new(Vec::new()),
        })
    }
    fn set_indices(&self, indices: &[i32]) {
        *self.indices.borrow_mut() = Some(indices.to_vec());
        self.invalidate_measure();
    }
}

#[repr(C)]
struct StatefulStackLayout {
    base: NonVirtualizingLayout,
    initial_state: BoxedValue,
    measured_states: RefCell<Vec<BoxedValue>>,
    uninitialized_state: RefCell<Option<BoxedValue>>,
}
ferro_class!(StatefulStackLayout: NonVirtualizingLayout);
ferro_impl_classes!(StatefulStackLayout: FerroObjectImpl, LayoutImpl);
impl NonVirtualizingLayoutImpl for StatefulStackLayout {
    fn initialize_for_context_core(this: &Self, context: &Ref<NonVirtualizingLayoutContext>) {
        context.set_layout_state(Some(this.initial_state.clone()));
    }
    fn uninitialize_for_context_core(this: &Self, context: &Ref<NonVirtualizingLayoutContext>) {
        *this.uninitialized_state.borrow_mut() = context.layout_state();
    }
    fn measure_override(
        this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        this.measured_states
            .borrow_mut()
            .push(context.layout_state().expect("initialized state"));
        let children = context.children();
        for child in &children {
            child.measure(available);
        }
        Size::new(100.0, children.len() as f64 * 20.0)
    }
    fn arrange_override(
        _this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        for (index, child) in context.children().iter().enumerate() {
            child.arrange(Rect::new(0.0, index as f64 * 20.0, final_size.width, 20.0));
        }
        final_size
    }
}
impl StatefulStackLayout {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: NonVirtualizingLayout::construct(),
            initial_state: Rc::new(42_i32),
            measured_states: RefCell::new(Vec::new()),
            uninitialized_state: RefCell::new(None),
        })
    }
}

#[repr(C)]
struct CyclingStackLayout {
    base: StackLayout,
    measures: Cell<usize>,
    arranges: Cell<usize>,
}
ferro_class!(CyclingStackLayout: StackLayout);
ferro_impl_classes!(CyclingStackLayout: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for CyclingStackLayout {
    fn measure_override(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        _available: Size,
    ) -> Size {
        this.measures.set(this.measures.get() + 1);
        this.invalidate_measure();
        Size::new(100.0, 200.0 + this.measures.get() as f64)
    }
    fn arrange_override(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        _final_size: Size,
    ) -> Size {
        this.arranges.set(this.arranges.get() + 1);
        if this.arranges.get() < 70 {
            this.invalidate_measure();
        }
        Size::new(100.0, 200.0 + this.arranges.get() as f64)
    }
}

fn template(buttons: bool) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(
        |_| true,
        move |_, _| {
            let child: Ref<Control> = if buttons {
                Button::new().upcast()
            } else {
                Border::new().upcast()
            };
            child.set_width(100.0);
            child.set_height(20.0);
            Some(child)
        },
        true,
    )
}

fn fixture(count: i32, indices: Option<&[i32]>) -> (Ref<ItemsRepeater>, Ref<ProbeLayout>) {
    let repeater = ItemsRepeater::new();
    repeater.set_use_layout_rounding(false);
    let layout = ProbeLayout::new(indices);
    repeater.set_layout(Some(layout.clone().upcast()));
    repeater.set_data_template(Some(template(true)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..count)));
    (repeater, layout)
}

fn run(repeater: &ItemsRepeater) {
    repeater.measure(Size::new(100.0, 100.0));
    repeater.arrange(Rect::new(0.0, 0.0, 100.0, repeater.desired_size().height));
}
fn item_value(element: &Control) -> i32 {
    *element
        .data_context()
        .expect("data context")
        .downcast_ref::<i32>()
        .expect("integer data")
}
fn child_owners(peer: &AutomationPeer) -> Vec<Ref<Control>> {
    peer.get_children()
        .iter()
        .map(|child| {
            child
                .cast::<ControlAutomationPeer>()
                .expect("control peer")
                .owner()
        })
        .collect()
}

macro_rules! roundtrip {
    ($object:expr, $ty:ty, $property:ident, $get:ident, $set:ident, $default:expr, $first:expr, $second:expr) => {{
        assert_eq!($object.$get(), $default);
        assert_eq!($object.get_value(<$ty>::$property()), $default);
        $object.$set($first);
        assert_eq!($object.$get(), $first);
        assert_eq!($object.get_value(<$ty>::$property()), $first);
        $object.set_value(<$ty>::$property(), $second);
        assert_eq!($object.$get(), $second);
        if $second == $default {
            // Start from a nondefault value so ClearValue has an observable effect.
            $object.set_value(<$ty>::$property(), $first);
            assert_eq!($object.$get(), $first);
        }
        $object.clear_value(<$ty>::$property());
        assert_eq!($object.$get(), $default);
    }};
}

#[test]
fn verify_items_repeater_dependency_properties() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    assert!(repeater.items_source().is_none());
    repeater.set_items_source(Some(ItemsSource::from_values([1])));
    assert_eq!(
        repeater
            .get_value(ItemsRepeater::items_source_property())
            .unwrap()
            .count(),
        1
    );
    repeater.set_value(
        ItemsRepeater::items_source_property(),
        Some(ItemsSource::from_values([2, 3])),
    );
    assert_eq!(repeater.items_source().unwrap().count(), 2);
    repeater.clear_value(ItemsRepeater::items_source_property());
    assert!(repeater.items_source().is_none());

    let stack: Ref<Layout> = StackLayout::new().upcast();
    let grid: Ref<Layout> = UniformGridLayout::new().upcast();
    roundtrip!(
        repeater,
        ItemsRepeater,
        layout_property,
        layout,
        set_layout,
        None,
        Some(stack.clone()),
        Some(grid.clone())
    );
    let provider1 = ItemCollectionTransitionProvider::new();
    let provider2: Ref<ItemCollectionTransitionProvider> =
        LinedFlowLayoutItemCollectionTransitionProvider::new().upcast();
    roundtrip!(
        repeater,
        ItemsRepeater,
        item_transition_provider_property,
        item_transition_provider,
        set_item_transition_provider,
        None,
        Some(provider1.clone()),
        Some(provider2.clone())
    );
    roundtrip!(
        repeater,
        ItemsRepeater,
        horizontal_cache_length_property,
        horizontal_cache_length,
        set_horizontal_cache_length,
        2.0,
        4.0,
        0.0
    );
    roundtrip!(
        repeater,
        ItemsRepeater,
        vertical_cache_length_property,
        vertical_cache_length,
        set_vertical_cache_length,
        2.0,
        0.5,
        6.0
    );

    let red: Rc<dyn IBrush> = Brushes::red();
    let blue: Rc<dyn IBrush> = Brushes::blue();
    assert!(repeater.background().is_none());
    repeater.set_background(Some(red.clone()));
    assert!(Rc::ptr_eq(
        &repeater.get_value(Panel::background_property()).unwrap(),
        &red
    ));
    repeater.set_value(Panel::background_property(), Some(blue.clone()));
    assert!(Rc::ptr_eq(&repeater.background().unwrap(), &blue));
    repeater.clear_value(Panel::background_property());
    assert!(repeater.background().is_none());

    let first: BoxedValue = Rc::new(template(false));
    let second: BoxedValue = Rc::new(template(true));
    assert!(repeater.item_template().is_none());
    repeater.set_item_template(Some(first.clone()));
    assert!(Rc::ptr_eq(
        &repeater
            .get_value(ItemsRepeater::item_template_property())
            .unwrap(),
        &first
    ));
    repeater.set_value(
        ItemsRepeater::item_template_property(),
        Some(second.clone()),
    );
    assert!(Rc::ptr_eq(&repeater.item_template().unwrap(), &second));
}

#[test]
fn validate_get_set_items_source_preserves_supplied_view_identity() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::from_items(0..3));
    let view = ferroui_controls::ItemsSourceView::new(source.clone().into());
    let (repeater, _) = fixture(0, None);
    repeater.set_items_source(Some(view.clone().into()));
    assert!(Rc::ptr_eq(&view, &repeater.items_source_view().unwrap()));
    run(&repeater);
    assert_eq!(repeater.realized_elements().len(), 3);
    source.add(3);
    run(&repeater);
    assert_eq!(item_value(&repeater.try_get_element(3).unwrap()), 3);
    repeater.set_items_source(None);
    assert!(repeater.items_source_view().is_none());
    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn verify_repeater_does_not_leak_item_containers() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::from_items(0..10));
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(source.clone().into()));
    repeater.set_data_template(Some(template(true)));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let containers = repeater
        .realized_elements()
        .iter()
        .map(Ref::downgrade)
        .collect::<Vec<_>>();
    assert_eq!(containers.len(), 10);
    let weak = repeater.downgrade();
    root.set_child(None::<Ref<Control>>);
    root.layout_manager().execute_layout_pass();
    drop(repeater);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    assert!(containers
        .iter()
        .all(|container| container.upgrade().is_none()));
    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn verify_repeater_with_recycled_elements_does_not_leak() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::from_items(0..5));
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(source.clone().into()));
    let template = template(true);
    let weak_template = Rc::downgrade(&template);
    repeater.set_data_template(Some(template));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let containers = repeater
        .realized_elements()
        .iter()
        .map(Ref::downgrade)
        .collect::<Vec<_>>();
    let weak = repeater.downgrade();
    source.clear();
    root.layout_manager().execute_layout_pass();
    assert!(repeater.realized_elements().is_empty());
    assert!(containers
        .iter()
        .all(|container| container.upgrade().is_some()));
    root.set_child(None::<Ref<Control>>);
    root.layout_manager().execute_layout_pass();
    drop(repeater);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    assert!(weak_template.upgrade().is_none());
    assert!(containers
        .iter()
        .all(|container| container.upgrade().is_none()));
    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn verify_repeater_with_recycling_element_factory_does_not_leak() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::from_items(0..6));
    let repeater = ItemsRepeater::new();
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().set("even".into(), template(true));
    factory.templates().set("odd".into(), template(true));
    factory.select_template_key(|args| {
        let item = *args.data_context().unwrap().downcast_ref::<i32>().unwrap();
        args.set_template_key(if item % 2 == 0 { "even" } else { "odd" });
    });
    let weak_factory = factory.downgrade();
    repeater.set_element_factory(Some(factory.as_element_factory()));
    drop(factory);
    repeater.set_items_source(Some(source.clone().into()));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let containers = repeater
        .realized_elements()
        .iter()
        .map(Ref::downgrade)
        .collect::<Vec<_>>();
    let weak = repeater.downgrade();
    source.clear();
    root.layout_manager().execute_layout_pass();
    assert!(repeater.realized_elements().is_empty());
    root.set_child(None::<Ref<Control>>);
    root.layout_manager().execute_layout_pass();
    drop(repeater);
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    assert!(weak_factory.upgrade().is_none());
    assert!(containers
        .iter()
        .all(|container| container.upgrade().is_none()));
    assert!(!source.has_collection_changed_subscribers());
}

#[test]
fn verify_stack_layout_dependency_properties() {
    let _scope = test_scope();
    let layout = StackLayout::new();
    roundtrip!(
        layout,
        StackLayout,
        orientation_property,
        orientation,
        set_orientation,
        Orientation::Vertical,
        Orientation::Horizontal,
        Orientation::Vertical
    );
    roundtrip!(
        layout,
        StackLayout,
        spacing_property,
        spacing,
        set_spacing,
        0.0,
        12.0,
        3.0
    );
    roundtrip!(
        layout,
        StackLayout,
        is_virtualization_enabled_property,
        is_virtualization_enabled,
        set_is_virtualization_enabled,
        true,
        false,
        true
    );
}

#[test]
fn verify_uniform_grid_layout_dependency_properties() {
    let _scope = test_scope();
    let layout = UniformGridLayout::new();
    roundtrip!(
        layout,
        UniformGridLayout,
        orientation_property,
        orientation,
        set_orientation,
        Orientation::Horizontal,
        Orientation::Vertical,
        Orientation::Horizontal
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        min_item_width_property,
        min_item_width,
        set_min_item_width,
        0.0,
        40.0,
        25.0
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        min_item_height_property,
        min_item_height,
        set_min_item_height,
        0.0,
        30.0,
        15.0
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        min_row_spacing_property,
        min_row_spacing,
        set_min_row_spacing,
        0.0,
        5.0,
        7.0
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        min_column_spacing_property,
        min_column_spacing,
        set_min_column_spacing,
        0.0,
        6.0,
        8.0
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        items_justification_property,
        items_justification,
        set_items_justification,
        UniformGridLayoutItemsJustification::Start,
        UniformGridLayoutItemsJustification::SpaceEvenly,
        UniformGridLayoutItemsJustification::End
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        items_stretch_property,
        items_stretch,
        set_items_stretch,
        UniformGridLayoutItemsStretch::None,
        UniformGridLayoutItemsStretch::Uniform,
        UniformGridLayoutItemsStretch::Fill
    );
    roundtrip!(
        layout,
        UniformGridLayout,
        maximum_rows_or_columns_property,
        maximum_rows_or_columns,
        set_maximum_rows_or_columns,
        -1,
        3,
        5
    );
}

#[test]
fn verify_flow_layout_dependency_properties() {
    let _scope = test_scope();
    let layout = FlowLayout::new();
    roundtrip!(
        layout,
        FlowLayout,
        orientation_property,
        orientation,
        set_orientation,
        Orientation::Horizontal,
        Orientation::Vertical,
        Orientation::Horizontal
    );
    roundtrip!(
        layout,
        FlowLayout,
        line_spacing_property,
        line_spacing,
        set_line_spacing,
        0.0,
        10.0,
        4.0
    );
    roundtrip!(
        layout,
        FlowLayout,
        min_item_spacing_property,
        min_item_spacing,
        set_min_item_spacing,
        0.0,
        9.0,
        2.0
    );
    roundtrip!(
        layout,
        FlowLayout,
        line_alignment_property,
        line_alignment,
        set_line_alignment,
        FlowLayoutLineAlignment::Start,
        FlowLayoutLineAlignment::SpaceAround,
        FlowLayoutLineAlignment::End
    );
}

#[test]
fn verify_lined_flow_layout_dependency_properties() {
    let _scope = test_scope();
    let layout = LinedFlowLayout::new();
    roundtrip!(
        layout,
        LinedFlowLayout,
        items_justification_property,
        items_justification,
        set_items_justification,
        LinedFlowLayoutItemsJustification::Start,
        LinedFlowLayoutItemsJustification::SpaceBetween,
        LinedFlowLayoutItemsJustification::Center
    );
    roundtrip!(
        layout,
        LinedFlowLayout,
        items_stretch_property,
        items_stretch,
        set_items_stretch,
        LinedFlowLayoutItemsStretch::None,
        LinedFlowLayoutItemsStretch::Fill,
        LinedFlowLayoutItemsStretch::None
    );
    roundtrip!(
        layout,
        LinedFlowLayout,
        min_item_spacing_property,
        min_item_spacing,
        set_min_item_spacing,
        0.0,
        6.0,
        3.0
    );
    roundtrip!(
        layout,
        LinedFlowLayout,
        line_spacing_property,
        line_spacing,
        set_line_spacing,
        0.0,
        8.0,
        2.0
    );
    assert!(layout.line_height().is_nan());
    assert!(layout
        .get_value(LinedFlowLayout::line_height_property())
        .is_nan());
    layout.set_line_height(90.0);
    assert_eq!(
        layout.get_value(LinedFlowLayout::line_height_property()),
        90.0
    );
    layout.set_value(LinedFlowLayout::line_height_property(), 45.0);
    assert_eq!(layout.line_height(), 45.0);
    layout.clear_value(LinedFlowLayout::line_height_property());
    assert!(layout.line_height().is_nan());
    assert_eq!(layout.actual_line_height(), 0.0);
    assert_eq!(
        layout.get_direct_value(LinedFlowLayout::actual_line_height_property()),
        0.0
    );
}

#[test]
fn validate_children_peers() {
    let _scope = test_scope();
    let first = Button::new();
    let discarded = Button::new();
    let group = StackPanel::new();
    AutomationProperties::set_accessibility_view(&group, AccessibilityView::Raw);
    let nested1 = Button::new();
    let nested2 = Button::new();
    group.children().add(&nested1);
    group.children().add(&nested2);
    let (repeater, layout) = fixture(0, Some(&[2, 0, 1]));
    repeater.set_item_template(None);
    repeater.set_items_source(Some(ItemsSource::from_items([
        Some(Control::boxed(first.clone())),
        Some(Control::boxed(discarded.clone())),
        Some(Control::boxed(group)),
    ])));
    layout.recycle_index.set(Some(1));
    run(&repeater);
    let peer = ControlAutomationPeer::create_peer_for_element(&repeater);
    assert_eq!(
        child_owners(&peer),
        vec![
            first.upcast::<Control>(),
            nested1.upcast::<Control>(),
            nested2.upcast::<Control>()
        ]
    );
    assert!(repeater.try_get_element(1).is_none());
}

#[test]
fn verify_repeater_automation_peer_created_directly() {
    let _scope = test_scope();
    let (repeater, _) = fixture(3, None);
    run(&repeater);
    let peer = RepeaterAutomationPeer::new(&repeater);
    assert_eq!(peer.owner(), repeater);
    assert_eq!(
        peer.get_automation_control_type(),
        AutomationControlType::Group
    );
    assert_eq!(
        child_owners(&peer),
        (0..3)
            .map(|i| repeater.try_get_element(i).unwrap())
            .collect::<Vec<_>>()
    );
    assert!(
        ControlAutomationPeer::create_peer_for_element(&repeater).is::<RepeaterAutomationPeer>()
    );
}

#[test]
fn verify_non_virtualizing_layout_state_round_trip() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let layout = StatefulStackLayout::new();
    repeater.set_data_template(Some(template(false)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..2)));
    repeater.set_layout(Some(layout.clone().upcast()));
    assert!(Rc::ptr_eq(
        &repeater.layout_state().unwrap(),
        &layout.initial_state
    ));
    run(&repeater);
    repeater.invalidate_measure();
    run(&repeater);
    assert_eq!(layout.measured_states.borrow().len(), 2);
    assert!(layout
        .measured_states
        .borrow()
        .iter()
        .all(|state| Rc::ptr_eq(state, &layout.initial_state)));
    repeater.set_layout(Some(StackLayout::new().upcast()));
    assert!(Rc::ptr_eq(
        layout.uninitialized_state.borrow().as_ref().unwrap(),
        &layout.initial_state
    ));
}

#[test]
fn verify_non_virtual_layout_remeasures_on_collection_change() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let layout = StatefulStackLayout::new();
    let source = Rc::new(FerroList::from_items(0..3));
    repeater.set_data_template(Some(template(false)));
    repeater.set_items_source(Some(source.clone().into()));
    repeater.set_layout(Some(layout.clone().upcast()));
    run(&repeater);
    assert_eq!(repeater.desired_size().height, 60.0);
    source.add(3);
    run(&repeater);
    assert_eq!(repeater.desired_size().height, 80.0);
    assert_eq!(item_value(&repeater.try_get_element(3).unwrap()), 3);
    source.remove_at(0);
    run(&repeater);
    assert_eq!(repeater.desired_size().height, 60.0);
    for index in 0..3 {
        assert_eq!(
            item_value(&repeater.try_get_element(index).unwrap()),
            index + 1
        );
    }
    assert_eq!(layout.measured_states.borrow().len(), 3);
}

#[test]
fn verify_layout_invalidate_arrange_rearranges_repeater() {
    let _scope = test_scope();
    let (repeater, layout) = fixture(3, None);
    run(&repeater);
    let measures = layout.measures.get();
    let arranges = layout.arranges.get();
    let arrange_events = Rc::new(Cell::new(0));
    let observed = arrange_events.clone();
    layout.arrange_invalidated(move || observed.set(observed.get() + 1));
    let measure_events = Rc::new(Cell::new(0));
    let observed = measure_events.clone();
    layout.measure_invalidated(move || observed.set(observed.get() + 1));
    layout.invalidate_arrange();
    assert_eq!(arrange_events.get(), 1);
    assert_eq!(measure_events.get(), 0);
    run(&repeater);
    assert_eq!(layout.arranges.get(), arranges + 1);
    assert_eq!(layout.measures.get(), measures);
}

#[test]
fn verify_stack_layout_cycle_shortcut() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let layout = instantiate(CyclingStackLayout {
        base: StackLayout::construct(),
        measures: Cell::new(0),
        arranges: Cell::new(0),
    });
    repeater.set_layout(Some(layout.clone().upcast()));
    let mut desired = Size::default();
    // Exercise 70 unsettled passes without the native layout manager's own
    // cycle limit or LayoutUpdated notification resetting the counter.
    for _ in 0..70 {
        desired =
            <ItemsRepeater as LayoutableImpl>::measure_override(&repeater, Size::new(100.0, 500.0));
        <ItemsRepeater as LayoutableImpl>::arrange_override(&repeater, desired);
    }
    assert_eq!(layout.measures.get(), 59);
    assert_eq!(layout.arranges.get(), 70);
    assert_eq!(desired, Size::new(100.0, 259.0));
}

// Additional regression for ItemsRepeater.cpp's persistent m_layoutOrigin.
#[test]
fn layout_origin_survives_until_layout_explicitly_changes_it() {
    let _scope = test_scope();
    let (repeater, layout) = fixture(1, None);
    layout.first_origin.set(Some(Point::new(25.0, 60.0)));
    run(&repeater);
    repeater.invalidate_measure();
    run(&repeater);
    assert_eq!(
        &*layout.observed_origins.borrow(),
        &[Point::default(), Point::new(25.0, 60.0)]
    );
    assert_eq!(repeater.layout_origin(), Point::new(25.0, 60.0));
}

// Additional regression: an empty template skips measure, but not arrange.
#[test]
fn empty_template_still_invokes_layout_arrange() {
    let _scope = test_scope();
    let (repeater, layout) = fixture(3, None);
    repeater.set_data_template(Some(FuncDataTemplate::new(|_| true, |_, _| None, false)));
    run(&repeater);
    assert_eq!(layout.measures.get(), 0);
    assert_eq!(layout.arranges.get(), 1);
    assert_eq!(repeater.desired_size(), Size::default());
}

#[test]
fn verify_focused_element_is_reused_from_pinned_pool() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let (repeater, layout) = fixture(100, Some(&[0, 1]));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let first = repeater.try_get_element(0).unwrap();
    assert!(first.focus());
    layout.set_indices(&[50, 51]);
    root.layout_manager().execute_layout_pass();
    assert!(repeater.try_get_element(50).is_some());
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&first).owner(),
        ElementOwner::PinnedPool
    );
    assert_eq!(repeater.get_element_index(&first), 0);
    assert_eq!(repeater.try_get_element(0), Some(first.clone()));
    assert_eq!(
        root.focus_manager().get_focused_element(),
        Some(first.clone().upcast())
    );

    let prepared = Rc::new(RefCell::new(Vec::new()));
    let observed = prepared.clone();
    repeater.element_prepared(move |args| observed.borrow_mut().push(args.index));
    layout.set_indices(&[0, 1]);
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.try_get_element(0), Some(first.clone()));
    assert_eq!(item_value(&first), 0);
    assert!(!prepared.borrow().contains(&0));
    assert_eq!(
        root.focus_manager().get_focused_element(),
        Some(first.upcast())
    );
}

#[test]
fn verify_pinned_element_index_updates_on_insert_outside_realized_range() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let (repeater, layout) = fixture(0, Some(&[90]));
    let source = Rc::new(FerroList::from_items(0..100));
    repeater.set_items_source(Some(source.clone().into()));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let pinned = repeater.try_get_element(90).unwrap();
    assert!(pinned.focus());
    layout.set_indices(&[0, 1]);
    root.layout_manager().execute_layout_pass();
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&pinned).owner(),
        ElementOwner::PinnedPool
    );

    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    repeater.element_index_changed(move |args| {
        observed
            .borrow_mut()
            .push((args.element.clone(), args.old_index, args.new_index))
    });
    source.insert(50, 1000);
    assert_eq!(&*changes.borrow(), &[(pinned.clone(), 90, 91)]);
    assert_eq!(repeater.get_element_index(&pinned), 91);
    layout.set_indices(&[91]);
    root.layout_manager().execute_layout_pass();
    assert_eq!(repeater.try_get_element(91), Some(pinned.clone()));
    assert_eq!(item_value(&pinned), 90);
    assert_eq!(
        root.focus_manager().get_focused_element(),
        Some(pinned.upcast())
    );
}

// Additional regression for ViewManager.cpp's next-index / last-previous policy.
#[test]
fn clearing_focus_prefers_next_layout_item_then_last_previous_descendant() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let (repeater, _) = fixture(10, Some(&[0, 2, 9]));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let panel = StackPanel::new();
            panel.children().add(Button::new());
            panel.children().add(Button::new());
            Some(panel.upcast())
        },
        false,
    )));
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let previous = repeater
        .try_get_element(0)
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    let focused = repeater
        .try_get_element(2)
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    let next = repeater
        .try_get_element(9)
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    assert!(focused.children().get(0).focus());
    // Collection changes and animator completion call this terminal ownership
    // operation even for pinned elements. It must skip closer previous indices.
    repeater
        .view_manager
        .clear_element_to_element_factory(&repeater, &focused.upcast());
    assert_eq!(
        root.focus_manager().get_focused_element(),
        Some(next.children().get(0).upcast())
    );
    repeater
        .view_manager
        .clear_element_to_element_factory(&repeater, &next.upcast());
    assert_eq!(
        root.focus_manager().get_focused_element(),
        Some(previous.children().get(1).upcast())
    );
}

// Additional native keyboard regression: each item's focusable descendants are
// visited in item order, independently of visual insertion order.
#[test]
fn keyboard_navigation_orders_descendants_and_skips_non_tab_stops() {
    use ferroui_base::input::{
        KeyboardNavigation, KeyboardNavigationHandler, KeyboardNavigationMode,
    };
    let _scope = test_scope();
    let (repeater, _) = fixture(3, Some(&[2, 0, 1]));
    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Local);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let panel = StackPanel::new();
            panel.children().add(Button::new());
            let skipped = Button::new();
            ferroui_base::input::KeyboardNavigation::set_is_tab_stop(&skipped, false);
            panel.children().add(skipped);
            panel.children().add(Button::new());
            Some(panel.upcast())
        },
        false,
    )));
    let _root = TestRoot::with_child(&repeater);
    run(&repeater);
    let mut expected = Vec::<Ref<InputElement>>::new();
    for index in 0..3 {
        let root = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        expected.push(root.children().get(0).upcast());
        expected.push(root.children().get(2).upcast());
    }
    let owner: Ref<InputElement> = repeater.clone().upcast();
    assert_eq!(
        KeyboardNavigationHandler::get_next(&owner, NavigationDirection::First),
        expected.first().cloned()
    );
    assert_eq!(
        KeyboardNavigationHandler::get_next(&owner, NavigationDirection::Last),
        expected.last().cloned()
    );
    for pair in expected.windows(2) {
        assert_eq!(
            KeyboardNavigationHandler::get_next(&pair[0], NavigationDirection::Next),
            Some(pair[1].clone())
        );
        assert_eq!(
            KeyboardNavigationHandler::get_next(&pair[1], NavigationDirection::Previous),
            Some(pair[0].clone())
        );
    }
}

fn verify_template_switch(layout: Ref<Layout>) {
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(layout));
    repeater.set_data_template(Some(template(false)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..10)));
    run(&repeater);
    assert_eq!(repeater.children().count(), 10);
    let old = repeater.realized_elements();
    assert!(old.iter().all(|child| child.is::<Border>()));
    repeater.set_data_template(Some(template(true)));
    run(&repeater);
    assert_eq!(repeater.children().count(), 20);
    for index in 0..10 {
        let element = repeater.try_get_element(index).unwrap();
        assert!(element.is::<Button>());
        assert_eq!(item_value(&element), index);
    }
    for old in old {
        assert_eq!(repeater.get_element_index(&old), -1);
        // FerroUI keeps explicit Width/Height in the actual bounds even when
        // the layout assigns an empty slot. The recycled slot and offscreen
        // placement express the source's nonparticipating-child contract.
        assert_eq!(old.previous_arrange().unwrap().size(), Size::default());
        assert!(old.bounds().y < -10000.0);
        assert!(old.visual_parent().is_some());
    }
}

#[test]
fn validate_template_switching_refreshes_elements_virtualizing_layout() {
    let _scope = test_scope();
    verify_template_switch(StackLayout::new().upcast());
}

#[test]
fn validate_template_switching_refreshes_elements_non_virtualizing_layout() {
    let _scope = test_scope();
    verify_template_switch(StatefulStackLayout::new().upcast());
}

#[test]
fn verify_changing_layout_clears_realized_elements() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(template(false)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..5)));
    run(&repeater);
    let old = repeater.realized_elements();
    assert_eq!(old.len(), 5);
    let cleared = Rc::new(RefCell::new(Vec::new()));
    let observed = cleared.clone();
    repeater.element_clearing(move |args| observed.borrow_mut().push(args.element.clone()));
    let prepared = Rc::new(RefCell::new(Vec::new()));
    let observed = prepared.clone();
    repeater.element_prepared(move |args| observed.borrow_mut().push(args.index));
    let grid = UniformGridLayout::new();
    grid.set_min_item_width(50.0);
    grid.set_min_item_height(20.0);
    repeater.set_layout(Some(grid.upcast()));
    assert_eq!(cleared.borrow().len(), 5);
    assert!(old.iter().all(|child| cleared.borrow().contains(child)));
    assert!(prepared.borrow().is_empty());
    assert!(repeater.realized_elements().is_empty());
    run(&repeater);
    prepared.borrow_mut().sort_unstable();
    prepared.borrow_mut().dedup();
    assert_eq!(&*prepared.borrow(), &[0, 1, 2, 3, 4]);
    for index in 0..5 {
        assert_eq!(item_value(&repeater.try_get_element(index).unwrap()), index);
    }
}

#[test]
fn verify_clearing_items_source_clears_elements() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(template(false)));
    repeater.set_items_source(Some(ItemsSource::from_values(0..5)));
    run(&repeater);
    let old = repeater.realized_elements();
    repeater.set_items_source(None);
    assert!(repeater.items_source_view().is_none());
    assert!(repeater.realized_elements().is_empty());
    run(&repeater);
    assert_eq!(repeater.desired_size(), Size::default());
    for old in old {
        assert_eq!(repeater.get_element_index(&old), -1);
        assert!(old.data_context().is_none());
    }
}

#[test]
fn validate_null_item_template_and_container_in_items() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(StackLayout::new().upcast()));
    let buttons: Vec<_> = (0..10)
        .map(|index| {
            let button = Button::new();
            button.set_content(Some(Rc::new(index)));
            button.set_height(20.0);
            button
        })
        .collect();
    repeater.set_items_source(Some(ItemsSource::from_items(
        buttons.iter().map(|button| Some(Control::boxed(button))),
    )));
    run(&repeater);
    assert!(repeater.item_template().is_none());
    assert_eq!(repeater.children().count(), 10);
    for (index, button) in buttons.iter().enumerate() {
        assert_eq!(
            repeater.try_get_element(index as i32),
            Some(button.clone().upcast())
        );
        assert_eq!(
            *button.content().unwrap().downcast_ref::<i32>().unwrap(),
            index as i32
        );
        assert!(button.visual_parent().is_some());
    }
    repeater.set_items_source(None);
    for button in buttons {
        assert!(button.visual_parent().is_none());
    }
}

#[test]
fn verify_stack_layout_alignment() {
    use ferroui_base::layout::HorizontalAlignment;
    let _scope = test_scope();
    for alignment in [
        HorizontalAlignment::Left,
        HorizontalAlignment::Center,
        HorizontalAlignment::Right,
        HorizontalAlignment::Stretch,
    ] {
        let repeater = ItemsRepeater::new();
        repeater.set_horizontal_alignment(alignment);
        repeater.set_items_source(Some(ItemsSource::from_values(0..2)));
        repeater.set_data_template(Some(template(false)));
        let border = Border::new();
        border.set_width(400.0);
        border.set_height(400.0);
        border.set_child(&repeater);
        let root = TestRoot::with_child(&border);
        root.execute_initial_layout_pass();
        assert_eq!(repeater.horizontal_alignment(), alignment);
        let x = repeater
            .translate_point(Point::default(), &border)
            .unwrap()
            .x;
        let remaining = border.bounds().width - repeater.bounds().width;
        let expected = match alignment {
            HorizontalAlignment::Left | HorizontalAlignment::Stretch => 0.0,
            HorizontalAlignment::Center => remaining / 2.0,
            HorizontalAlignment::Right => remaining,
        };
        assert_eq!(x, expected);
        if matches!(
            alignment,
            HorizontalAlignment::Center | HorizontalAlignment::Right
        ) {
            assert!(x > 0.0);
        }
        for index in 0..2 {
            assert!(repeater.try_get_element(index).is_some());
        }
    }
}

#[test]
fn verify_invalid_item_template_type_is_rejected() {
    let _scope = test_scope();
    let (repeater, _) = fixture(3, None);
    run(&repeater);
    let old = repeater.realized_elements();
    assert!(catch_unwind(AssertUnwindSafe(
        || repeater.set_item_template(Some(Rc::new("not an element factory".to_owned())))
    ))
    .is_err());
    run(&repeater);
    assert_eq!(repeater.realized_elements(), old);
    for index in 0..3 {
        assert_eq!(item_value(&repeater.try_get_element(index).unwrap()), index);
    }
}

#[test]
fn verify_invalid_cache_lengths_are_rejected() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    for invalid in [-1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(catch_unwind(AssertUnwindSafe(
            || repeater.set_horizontal_cache_length(invalid)
        ))
        .is_err());
        assert!(catch_unwind(AssertUnwindSafe(
            || repeater.set_vertical_cache_length(invalid)
        ))
        .is_err());
    }
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    assert_eq!(repeater.horizontal_cache_length(), 0.0);
    assert_eq!(repeater.vertical_cache_length(), 0.0);
}

#[test]
fn verify_collection_change_during_collection_change_throws() {
    let _scope = test_scope();
    let (repeater, layout) = fixture(0, None);
    let source = Rc::new(FerroList::from_items(0..5));
    repeater.set_items_source(Some(source.clone().into()));
    run(&repeater);
    let before = layout.changes.get();
    let rejected = Rc::new(Cell::new(false));
    let observed = rejected.clone();
    let source_in_callback = source.clone();
    *layout.items_changed.borrow_mut() = Some(Rc::new(move || {
        observed.set(catch_unwind(AssertUnwindSafe(|| source_in_callback.insert(0, 100))).is_err());
    }));
    source.remove_at(4);
    assert!(rejected.get());
    assert_eq!(layout.changes.get(), before + 1);
    *layout.items_changed.borrow_mut() = None;
    // The source owns the mutation; clear to a coherent state after rejecting
    // its nested notification, then verify the processing guard was released.
    source.clear();
    run(&repeater);
    assert!(repeater.realized_elements().is_empty());
}

#[test]
fn verify_layout_during_collection_change_is_rejected() {
    let _scope = test_scope();
    let (repeater, layout) = fixture(0, None);
    let source = Rc::new(FerroList::from_items(0..5));
    repeater.set_items_source(Some(source.clone().into()));
    run(&repeater);
    let rejections = Rc::new(Cell::new((false, false)));
    let observed = rejections.clone();
    let weak = repeater.downgrade();
    *layout.items_changed.borrow_mut() = Some(Rc::new(move || {
        let repeater = weak.upgrade().unwrap();
        let arrange = catch_unwind(AssertUnwindSafe(|| {
            <ItemsRepeater as LayoutableImpl>::arrange_override(&repeater, Size::new(100.0, 100.0));
        }))
        .is_err();
        let measure = catch_unwind(AssertUnwindSafe(|| {
            <ItemsRepeater as LayoutableImpl>::measure_override(&repeater, Size::new(100.0, 100.0));
        }))
        .is_err();
        observed.set((measure, arrange));
    }));
    source.insert(0, 100);
    assert_eq!(rejections.get(), (true, true));
    *layout.items_changed.borrow_mut() = None;
    run(&repeater);
    assert_eq!(repeater.items_source_view().unwrap().count(), 6);
    assert!(repeater.try_get_element(5).is_some());
}

#[test]
fn validate_data_context_does_not_get_overwritten() {
    let _scope = test_scope();
    struct IdentityFactory;
    impl IElementFactory for IdentityFactory {
        fn get_element(
            &self,
            args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            Ok(Control::from_boxed(args.data().as_ref().unwrap()).unwrap())
        }
        fn recycle_element(
            &self,
            _args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            Ok(())
        }
    }
    let item = Button::new();
    let context: BoxedValue = Rc::new("Element1_DataContext".to_owned());
    item.set_data_context(Some(context.clone()));
    let repeater = ItemsRepeater::new();
    repeater.set_element_factory(Some(Rc::new(IdentityFactory)));
    repeater.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(
        item.clone(),
    ))])));
    run(&repeater);
    assert_eq!(repeater.try_get_element(0), Some(item.clone().upcast()));
    assert!(Rc::ptr_eq(&item.data_context().unwrap(), &context));
    repeater.set_items_source(None);
    assert!(Rc::ptr_eq(&item.data_context().unwrap(), &context));
}

#[test]
fn validate_data_context_gets_propagated() {
    let _scope = test_scope();
    struct WrappingFactory;
    impl IElementFactory for WrappingFactory {
        fn get_element(
            &self,
            args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            let button = Button::new();
            button.set_height(20.0);
            button.set_content(args.data());
            Ok(button.upcast())
        }
        fn recycle_element(
            &self,
            _args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            Ok(())
        }
    }
    let item = Button::new();
    let context: BoxedValue = Rc::new("Element1_DataContext".to_owned());
    item.set_data_context(Some(context.clone()));
    let repeater = ItemsRepeater::new();
    repeater.set_element_factory(Some(Rc::new(WrappingFactory)));
    repeater.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(
        item.clone(),
    ))])));
    run(&repeater);
    let element = repeater
        .try_get_element(0)
        .unwrap()
        .cast::<Button>()
        .unwrap();
    assert!(!element.ptr_eq(&item));
    assert!(Rc::ptr_eq(&element.data_context().unwrap(), &context));
    assert_eq!(
        Control::from_boxed(element.content().as_ref().unwrap()),
        Some(item.upcast())
    );
    repeater.set_items_source(None);
    assert!(element.data_context().is_none());
}

// Additional regression: ViewManager shifts subsequent indices; StackLayout
// owns recycling the replaced slot, so its manager cannot retain a stale root.
#[test]
fn stack_replace_rebinds_the_replaced_slot_and_preserves_other_elements() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(template(false)));
    let source = Rc::new(FerroList::from_items([10, 20, 30, 40, 50]));
    repeater.set_items_source(Some(source.clone().into()));
    run(&repeater);
    let original: Vec<_> = (0..5)
        .map(|index| repeater.try_get_element(index).unwrap())
        .collect();
    let cleared = Rc::new(RefCell::new(Vec::new()));
    let observed = cleared.clone();
    repeater.element_clearing(move |args| observed.borrow_mut().push(item_value(&args.element)));
    source.set(2, 300);
    run(&repeater);
    assert_eq!(&*cleared.borrow(), &[30]);
    assert_eq!(item_value(&repeater.try_get_element(2).unwrap()), 300);
    for index in [0, 1, 3, 4] {
        assert_eq!(
            repeater.try_get_element(index),
            Some(original[index as usize].clone())
        );
    }
}

#[test]
fn verify_invalid_replace_notifications_throw() {
    use ferroui_base::collections::NotifyCollectionChangedAction;
    let _scope = test_scope();
    let (repeater, _) = fixture(3, None);
    run(&repeater);
    let one: [Option<BoxedValue>; 1] = [Some(Rc::new(1_i32))];
    let empty: [Option<BoxedValue>; 0] = [];
    for (old, new, old_index, new_index) in [
        (one.as_slice(), one.as_slice(), 0, 1),
        (empty.as_slice(), one.as_slice(), 0, 0),
        (one.as_slice(), empty.as_slice(), 0, 0),
    ] {
        let args = ItemsChangedEventArgs {
            action: NotifyCollectionChangedAction::Replace,
            old_items: old.into(),
            new_items: new.into(),
            old_starting_index: old_index,
            new_starting_index: new_index,
        };
        assert!(catch_unwind(AssertUnwindSafe(|| repeater
            .view_manager
            .on_items_source_changed(&repeater, &args)))
        .is_err());
    }
    assert_eq!(repeater.realized_elements().len(), 3);
}

// The removal case is also part of VerifyRuntimeClassNamesOfRepeaterObjects;
// native metadata identity is checked separately by the markup contract tests.
#[test]
fn removing_last_index_changed_handler_detaches_it() {
    let _scope = test_scope();
    let (repeater, _) = fixture(0, None);
    let source = Rc::new(FerroList::from_items(0..3));
    repeater.set_items_source(Some(source.clone().into()));
    run(&repeater);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let subscription = repeater.element_index_changed(move |_| observed.set(observed.get() + 1));
    source.insert(0, 100);
    assert_eq!(calls.get(), 3);
    subscription.dispose();
    source.insert(0, 200);
    assert_eq!(calls.get(), 3);
}

#[test]
fn verify_removing_last_event_handler_detaches_it() {
    use super::selection::SelectionModel;
    let _scope = test_scope();
    let source = || {
        ItemsSource::from_values([
            ItemsSource::from_values([1, 2, 3]),
            ItemsSource::from_values([4, 5, 6]),
        ])
    };
    let model = SelectionModel::new();
    model.set_source(Some(source()));
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let subscription = model.children_requested(move |args| {
        observed.set(observed.get() + 1);
        args.set_children(
            args.source()
                .unwrap()
                .as_ref()
                .and_then(SelectionModel::source_from_item),
        );
    });
    model.select_with_group(0, 1).unwrap();
    assert!(calls.get() > 0);
    subscription.dispose();
    let before = calls.get();
    model.set_source(Some(source()));
    model.select_with_group(1, 2).unwrap();
    assert_eq!(calls.get(), before);
    assert_eq!(model.is_selected_with_group(1, 2), Some(true));

    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().set("T".into(), template(false));
    factory.templates().set("U".into(), template(true));
    let selects = Rc::new(Cell::new(0));
    let observed = selects.clone();
    let subscription = factory.select_template_key(move |args| {
        observed.set(observed.get() + 1);
        args.set_template_key("U");
    });
    let owner = StackPanel::new();
    let args = ElementFactoryGetArgs::new(Some(Rc::new(1_i32)), Some(owner.upcast()));
    assert!(factory.get_element(&args).unwrap().is::<Button>());
    assert_eq!(selects.get(), 1);
    subscription.dispose();
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::EmptyTemplateKey)
    );
    assert_eq!(selects.get(), 1);
}

// Additional source-fidelity regression: implicit template installation is a
// public ItemTemplate change and a Reset delivered before replacing the shim.
#[test]
fn implicit_template_is_published_before_factory_reset_and_handles_null_items() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let layout = ProbeLayout::new(None);
    repeater.set_layout(Some(layout.clone().upcast()));
    let item = Button::new();
    item.set_data_context(Some(Rc::new("control context".to_owned())));
    repeater.set_items_source(Some(ItemsSource::from_items([
        None,
        Some(Control::boxed(item)),
        Some(Rc::new(7_i32) as BoxedValue),
    ])));
    assert!(repeater.item_template().is_none());
    let changes_before = layout.changes.get();
    let factory_order = Rc::new(RefCell::new(Vec::new()));
    let observed = factory_order.clone();
    let weak = repeater.downgrade();
    *layout.items_changed.borrow_mut() = Some(Rc::new(move || {
        let owner = weak.upgrade().unwrap();
        observed.borrow_mut().push((
            owner.item_template().is_some(),
            owner.item_template_shim().is_none(),
        ));
    }));
    let property_changes = Rc::new(Cell::new(0));
    let observed = property_changes.clone();
    repeater.property_changed(move |args| {
        if args.property() == ItemsRepeater::item_template_property().as_property() {
            observed.set(observed.get() + 1);
        }
    });
    run(&repeater);
    let published = repeater.item_template().unwrap();
    assert_eq!(property_changes.get(), 1);
    assert_eq!(layout.changes.get(), changes_before + 1);
    assert_eq!(&*factory_order.borrow(), &[(true, true)]);
    assert_eq!(repeater.items_source_view().unwrap().count(), 3);
    let first = repeater
        .try_get_element(0)
        .unwrap()
        .cast::<ferroui_controls::TextBlock>()
        .unwrap();
    assert!(first.text().unwrap_or_default().is_empty());
    assert!(first.data_context().is_none());
    assert_eq!(
        repeater
            .try_get_element(1)
            .unwrap()
            .cast::<ferroui_controls::TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("control context")
    );
    assert_eq!(
        repeater
            .try_get_element(2)
            .unwrap()
            .cast::<ferroui_controls::TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("7")
    );
    repeater.invalidate_measure();
    run(&repeater);
    assert!(Rc::ptr_eq(&published, &repeater.item_template().unwrap()));
    assert_eq!(property_changes.get(), 1);
    assert_eq!(layout.changes.get(), changes_before + 1);
}

// Additional regression: a direct control preceding the first ordinary item
// participates in the implicit-template Reset through the real StackLayout.
#[test]
fn mixed_control_and_data_source_installs_one_implicit_template() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let first = Button::new();
    first.set_height(20.0);
    first.set_data_context(Some(Rc::new("first context".to_owned())));
    let last = Button::new();
    last.set_data_context(Some(Rc::new("last context".to_owned())));
    repeater.set_items_source(Some(ItemsSource::from_items([
        Some(Control::boxed(first.clone())),
        Some(Rc::new("ordinary".to_owned()) as BoxedValue),
        None,
        Some(Control::boxed(last)),
    ])));
    let root = TestRoot::with_child(&repeater);
    root.set_width(300.0);
    root.set_height(300.0);
    root.execute_initial_layout_pass();
    let published = repeater
        .item_template()
        .expect("implicit template is public");
    assert!(repeater.layout().is_none());
    for (index, expected) in ["first context", "ordinary", "", "last context"]
        .into_iter()
        .enumerate()
    {
        let element = repeater
            .try_get_element(index as i32)
            .expect("mixed item is realized");
        let text = element
            .cast::<ferroui_controls::TextBlock>()
            .expect("the implicit template applies to every subsequent realization");
        assert_eq!(text.text().unwrap_or_default(), expected);
    }
    assert!(first.visual_parent().is_none());
    assert_eq!(
        first
            .data_context()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("first context")
    );
    repeater.invalidate_measure();
    root.layout_manager().execute_layout_pass();
    assert!(Rc::ptr_eq(&published, &repeater.item_template().unwrap()));
}

#[test]
fn verify_get_or_create_element_rejects_invalid_requests() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    assert!(catch_unwind(AssertUnwindSafe(|| repeater.get_or_create_element(0))).is_err());
    repeater.set_items_source(Some(ItemsSource::from_values(0..3)));
    assert!(catch_unwind(AssertUnwindSafe(|| repeater.get_or_create_element(-1))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| repeater.get_or_create_element(3))).is_err());
    assert_eq!(
        repeater.get_element_index(&repeater.get_or_create_element(2)),
        2
    );
}
