// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Scenario ports from the pinned RepeaterTests.cs and ViewManagerTests.cs.
// Native roots, collections and focus services replace the Windows fixtures.

use super::*;
use ferroui_controls::ItemsChangedHandler;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::templates::{FuncControlTemplate, FuncDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    Button, Control, IItemsList, IKeyIndexMapping, ItemsChangedEventArgs, ItemsSource,
    ScrollViewer, StackPanel, TextBlock,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::input::{
    IKeyboardNavigationHandler, InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation,
    KeyboardNavigationHandler, KeyboardNavigationMode, NavigationDirection, NavigationMethod,
};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Rect, Ref, Size,
    Vector,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
struct WindowLayout {
    base: VirtualizingLayout,
    first: Cell<i32>,
    count: Cell<i32>,
    live: RefCell<Vec<(i32, Ref<Control>)>>,
    probe_cache: Cell<bool>,
    cache_probed: Cell<bool>,
}
ferro_class!(WindowLayout: VirtualizingLayout);
ferro_impl_classes!(WindowLayout: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for WindowLayout {
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        if this.probe_cache.get() {
            assert_eq!(context.item_count(), 15);
            let zero = context.get_or_create_element_at(0);
            let one = context.get_or_create_element_at(1);
            assert_ne!(zero, one);
            assert_eq!(one, context.get_or_create_element_at(1));
            let ten = context.get_or_create_element_at(10);
            assert_ne!(zero, ten);
            assert_ne!(one, ten);
            context.recycle_element(&one);
            assert_eq!(zero, context.get_or_create_element_at(0));
            context.recycle_element(&ten);
            context.recycle_element(&zero);
            this.cache_probed.set(true);
            return Size::new(10.0, 10.0);
        }
        let end = (this.first.get() + this.count.get()).min(context.item_count());
        let mut live = Vec::new();
        for index in this.first.get()..end {
            let element = context.get_or_create_element_at(index);
            element.measure(available);
            live.push((index, element));
        }
        *this.live.borrow_mut() = live;
        Size::new(100.0, context.item_count() as f64 * 20.0)
    }
    fn arrange_override(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        size: Size,
    ) -> Size {
        for (index, element) in this.live.borrow().iter() {
            element.arrange(Rect::new(0.0, *index as f64 * 20.0, 100.0, 20.0));
        }
        size
    }
    fn on_items_changed_core(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        args: &ItemsChangedEventArgs<'_>,
    ) {
        if args.action == NotifyCollectionChangedAction::Reset {
            let live = this.live.borrow_mut().drain(..).collect::<Vec<_>>();
            for (_, element) in live {
                context.recycle_element(&element);
            }
        }
        this.invalidate_measure();
    }
}
impl WindowLayout {
    fn new(first: i32, count: i32) -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            first: Cell::new(first),
            count: Cell::new(count),
            live: RefCell::new(Vec::new()),
            probe_cache: Cell::new(false),
            cache_probed: Cell::new(false),
        })
    }
}

#[repr(C)]
struct AllItemsLayout {
    base: NonVirtualizingLayout,
}
ferro_class!(AllItemsLayout: NonVirtualizingLayout);
ferro_impl_classes!(AllItemsLayout: FerroObjectImpl, LayoutImpl);
impl NonVirtualizingLayoutImpl for AllItemsLayout {
    fn measure_override(
        _this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        let children = context.children();
        for child in &children {
            child.measure(available);
        }
        Size::new(100.0, children.len() as f64 * 20.0)
    }
    fn arrange_override(
        _this: &Self,
        context: &Ref<NonVirtualizingLayoutContext>,
        size: Size,
    ) -> Size {
        for (index, child) in context.children().iter().enumerate() {
            child.arrange(Rect::new(0.0, index as f64 * 20.0, size.width, 20.0));
        }
        size
    }
}
impl AllItemsLayout {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: NonVirtualizingLayout::construct(),
        })
    }
}

struct KeyedSource {
    values: RefCell<Vec<i32>>,
    changed: HandlerList<ItemsChangedHandler>,
}
impl KeyedSource {
    fn new(values: impl IntoIterator<Item = i32>) -> Rc<Self> {
        Rc::new(Self {
            values: RefCell::new(values.into_iter().collect()),
            changed: HandlerList::new(),
        })
    }
    fn reset(&self, values: impl IntoIterator<Item = i32>) {
        *self.values.borrow_mut() = values.into_iter().collect();
        for (_, callback) in self.changed.snapshot().iter() {
            callback(&ItemsChangedEventArgs::RESET);
        }
    }
}
impl IKeyIndexMapping for KeyedSource {
    fn key_from_index(&self, index: i32) -> String {
        self.values.borrow()[index as usize].to_string()
    }
    fn index_from_key(&self, key: &str) -> i32 {
        self.values
            .borrow()
            .iter()
            .position(|value| value.to_string() == key)
            .map_or(-1, |index| index as i32)
    }
}
impl IItemsList for KeyedSource {
    fn key_index_mapping(&self) -> Option<&dyn IKeyIndexMapping> {
        Some(self)
    }
    fn count(&self) -> usize {
        self.values.borrow().len()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.values.borrow()[index]))
    }
    fn is_notifying(&self) -> bool {
        true
    }
    fn add_collection_changed(&self, callback: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.changed.add(callback))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.changed.remove(token);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn fixture(
    source: ItemsSource,
    count: i32,
    descendant: bool,
) -> (Ref<ItemsRepeater>, Ref<WindowLayout>) {
    let repeater = ItemsRepeater::new();
    let layout = WindowLayout::new(0, count);
    repeater.set_layout(Some(layout.clone().upcast()));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        move |_, _| {
            let button = Button::new();
            button.set_height(20.0);
            if descendant {
                let panel = StackPanel::new();
                panel.children().add(button);
                Some(panel.upcast())
            } else {
                Some(button.upcast())
            }
        },
        true,
    )));
    repeater.set_items_source(Some(source));
    (repeater, layout)
}
fn run(repeater: &ItemsRepeater) {
    repeater.measure(Size::new(100.0, 200.0));
    repeater.arrange(Rect::new(
        0.0,
        0.0,
        100.0,
        repeater.desired_size().height.max(200.0),
    ));
}
fn settle(root: &TestRoot) {
    for _ in 0..16 {
        root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
    }
}
fn scroller(repeater: &Ref<ItemsRepeater>) -> Ref<ScrollViewer> {
    let scroll = ScrollViewer::new();
    scroll.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::test_support::create_template,
    )));
    scroll.set_content(Some(Control::boxed(repeater)));
    scroll.set_width(100.0);
    scroll.set_height(200.0);
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroll
}
fn scrolling_repeater(
    source: Option<ItemsSource>,
    height: f64,
    button: bool,
) -> Ref<ItemsRepeater> {
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(StackLayout::new().upcast()));
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().set(
        "Item".into(),
        FuncDataTemplate::new(
            |_| true,
            move |_, _| {
                let element: Ref<Control> = if button {
                    Button::new().upcast()
                } else {
                    // Preserve the source template's Text='{Binding}'. An
                    // empty text block has zero desired width, which changes
                    // FlowLayout's backward realization intersection test.
                    let text = TextBlock::new();
                    text.bind_binding(
                        TextBlock::text_property().as_property(),
                        &*ReflectionBinding::empty(),
                    );
                    text.upcast()
                };
                element.set_height(height);
                Some(element)
            },
            true,
        ),
    );
    repeater.set_element_factory(Some(factory.as_element_factory()));
    repeater.set_items_source(source);
    repeater
}
fn focus_target(repeater: &ItemsRepeater, index: i32) -> Ref<InputElement> {
    let element = repeater.try_get_element(index).unwrap();
    if let Some(button) = element.cast::<Button>() {
        button.upcast()
    } else {
        element
            .cast::<StackPanel>()
            .unwrap()
            .children()
            .get(0)
            .upcast()
    }
}

#[derive(Default)]
struct Events {
    prepared: RefCell<Vec<i32>>,
    cleared: RefCell<Vec<i32>>,
    changed: RefCell<Vec<(i32, i32)>>,
}
impl Events {
    fn attach(repeater: &Ref<ItemsRepeater>) -> Rc<Self> {
        let events = Rc::new(Self::default());
        let captured = events.clone();
        repeater.element_prepared(move |args| captured.prepared.borrow_mut().push(args.index));
        let captured = events.clone();
        let owner = repeater.downgrade();
        repeater.element_clearing(move |args| {
            captured
                .cleared
                .borrow_mut()
                .push(owner.upgrade().unwrap().get_element_index(&args.element));
        });
        let captured = events.clone();
        repeater.element_index_changed(move |args| {
            captured
                .changed
                .borrow_mut()
                .push((args.old_index, args.new_index))
        });
        events
    }
    fn assert_and_clear(&self, prepared: &[i32], cleared: &[i32], changed: &[(i32, i32)]) {
        assert_eq!(&*self.prepared.borrow(), prepared);
        assert_eq!(&*self.cleared.borrow(), cleared);
        let mut observed = self.changed.borrow().clone();
        let mut expected = changed.to_vec();
        observed.sort_unstable();
        expected.sort_unstable();
        assert_eq!(observed, expected);
        self.prepared.borrow_mut().clear();
        self.cleared.borrow_mut().clear();
        self.changed.borrow_mut().clear();
    }
}

#[test]
fn validate_element_events_through_insert_and_remove() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::from_items(0..10));
    let (repeater, _) = fixture(source.clone().into(), 3, false);
    run(&repeater);
    let events = Events::attach(&repeater);
    source.insert(1, 2000);
    run(&repeater);
    events.assert_and_clear(&[1], &[3], &[(1, 2), (2, 3)]);
    source.remove_at(0);
    run(&repeater);
    events.assert_and_clear(&[2], &[0], &[(1, 0), (2, 1)]);
}

#[test]
fn validate_stable_reset_element_index_event_trace() {
    let _scope = test_scope();
    let source = KeyedSource::new(0..10);
    let (repeater, _) = fixture(ItemsSource::new(source.clone()), 3, false);
    run(&repeater);
    let events = Events::attach(&repeater);
    source.reset([0, 2000].into_iter().chain(1..10));
    run(&repeater);
    events.assert_and_clear(&[1], &[2], &[(1, 2)]);
    source.reset([2000].into_iter().chain(1..10));
    run(&repeater);
    events.assert_and_clear(&[2], &[0], &[(1, 0), (2, 1)]);
}

#[test]
fn changing_template_during_a_keyed_reset_returns_every_old_element_to_its_factory() {
    let _scope = test_scope();

    struct OldFactory {
        recycled: Rc<RefCell<Vec<Ref<Control>>>>,
    }
    impl IElementFactory for OldFactory {
        fn get_element(
            &self,
            _args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            let button = Button::new();
            button.set_height(20.0);
            Ok(button.upcast())
        }
        fn recycle_element(
            &self,
            args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            self.recycled.borrow_mut().push(args.element());
            args.parent()
                .unwrap()
                .cast::<ItemsRepeater>()
                .unwrap()
                .children()
                .remove(&args.element());
            Ok(())
        }
    }

    // A source Reset and a layout replacement both preserve keyed controls
    // until arrange. A template replacement in that interval must end reuse.
    for change_layout in [false, true] {
        let source = KeyedSource::new(0..3);
        let (repeater, _) = fixture(ItemsSource::new(source.clone()), 3, false);
        let recycled = Rc::new(RefCell::new(Vec::new()));
        repeater.set_element_factory(Some(Rc::new(OldFactory {
            recycled: recycled.clone(),
        })));
        run(&repeater);
        let old: Vec<_> = (0..3)
            .map(|index| repeater.try_get_element(index).unwrap())
            .collect();
        if change_layout {
            repeater.set_layout(Some(StackLayout::new().upcast()));
        } else {
            source.reset(0..3);
        }
        assert!(recycled.borrow().is_empty());
        assert!(old.iter().all(|element| {
            ItemsRepeater::get_virtualization_info(element).is_in_unique_id_reset_pool()
        }));

        repeater.set_data_template(Some(FuncDataTemplate::new(
            |_| true,
            |_, _| {
                let text = TextBlock::new();
                text.set_height(20.0);
                Some(text.upcast())
            },
            true,
        )));
        assert_eq!(recycled.borrow().len(), old.len());
        for element in &old {
            assert!(recycled.borrow().contains(element));
            assert!(element.visual_parent().is_none());
            assert!(element.data_context().is_none());
        }
        run(&repeater);
        for index in 0..3 {
            let element = repeater.try_get_element(index).unwrap();
            assert!(element.is::<TextBlock>());
            assert!(!old.contains(&element));
            assert_eq!(
                element.data_context().unwrap().downcast_ref::<i32>(),
                Some(&index)
            );
        }
        // Completing the reset/arrange must not recycle the old roots twice.
        assert_eq!(recycled.borrow().len(), old.len());
    }
}

#[test]
fn validate_get_element_at_caching_request_trace() {
    let _scope = test_scope();
    let (repeater, layout) = fixture(ItemsSource::from_values(0..15), 0, false);
    layout.probe_cache.set(true);
    run(&repeater);
    assert!(layout.cache_probed.get());
    assert!(repeater.realized_elements().is_empty());
}

#[test]
fn repeated_top_removals_prepare_new_items_in_the_realization_window() {
    let _scope = test_scope();
    let source = Rc::new(FerroList::from_items(0..20));
    let repeater = scrolling_repeater(Some(source.clone().into()), 24.0, false);
    repeater.set_vertical_cache_length(2.0);
    let scroll = scroller(&repeater);
    scroll.set_width(200.0);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    // Fill the gradually expanding cache before observing removals, as in
    // ValidateElementCreationUponRemovals' six compositor ticks.
    settle(&root);
    let events = Events::attach(&repeater);
    for _ in 0..16 {
        source.remove_at(0);
        settle(&root);
    }
    assert!(!events.prepared.borrow().is_empty());
    assert_eq!(repeater.realized_elements().len(), 4);
    for index in 0..4 {
        assert_eq!(
            repeater.get_element_index(&repeater.try_get_element(index).unwrap()),
            index
        );
    }
}

fn focus_moves_after_collection_removals(descendant: bool) {
    let source = Rc::new(FerroList::from_items(0..5));
    let (repeater, _) = fixture(source.clone().into(), 5, descendant);
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    for (remove, expected_index, expected_value) in [(2, 2, 3), (0, 0, 1), (2, 1, 3)] {
        assert!(focus_target(&repeater, remove).focus());
        source.remove_at(remove as usize);
        root.layout_manager().execute_layout_pass();
        assert_eq!(
            root.focus_manager().get_focused_element(),
            Some(focus_target(&repeater, expected_index))
        );
        let container = repeater.try_get_element(expected_index).unwrap();
        assert_eq!(
            *container
                .data_context()
                .unwrap()
                .downcast_ref::<i32>()
                .unwrap(),
            expected_value
        );
    }
}

#[test]
fn focused_item_removal_moves_to_adjacent_first_middle_and_last_items() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    focus_moves_after_collection_removals(false);
}

#[test]
fn focused_descendant_removal_moves_to_adjacent_first_middle_and_last_items() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    focus_moves_after_collection_removals(true);
}

#[test]
fn focus_survives_a_stable_reset_and_can_move_to_another_item() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let source = KeyedSource::new(0..2);
    let (repeater, _) = fixture(ItemsSource::new(source.clone()), 2, false);
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    let first = focus_target(&repeater, 0);
    assert!(first.focus());
    source.reset(0..2);
    root.layout_manager().execute_layout_pass();
    assert_eq!(root.focus_manager().get_focused_element(), Some(first));
    let second = focus_target(&repeater, 1);
    assert!(second.focus());
    assert_eq!(root.focus_manager().get_focused_element(), Some(second));
}

#[test]
fn keyed_reset_removing_a_focused_item_moves_focus_to_the_next_survivor() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let source = KeyedSource::new(0..5);
    let (repeater, _) = fixture(ItemsSource::new(source.clone()), 5, false);
    let root = TestRoot::with_child(&repeater);
    root.execute_initial_layout_pass();
    assert!(focus_target(&repeater, 2).focus());
    source.reset([0, 1, 3, 4]);
    root.layout_manager().execute_layout_pass();
    assert_eq!(
        root.focus_manager().get_focused_element(),
        Some(focus_target(&repeater, 2))
    );
    assert_eq!(
        *repeater
            .try_get_element(2)
            .unwrap()
            .data_context()
            .unwrap()
            .downcast_ref::<i32>()
            .unwrap(),
        3
    );
}

#[test]
fn focus_fallback_handles_first_last_and_multiple_removals_with_and_without_keys() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    for keyed in [false, true] {
        let keys = KeyedSource::new(0..5);
        let list = Rc::new(FerroList::from_items(0..5));
        let source = if keyed {
            ItemsSource::new(keys.clone())
        } else {
            list.clone().into()
        };
        let (repeater, _) = fixture(source, 5, false);
        let root = TestRoot::with_child(&repeater);
        root.execute_initial_layout_pass();
        for (remove, count, expected_index, expected_value) in
            [(0, 1, 0, 1), (3, 1, 2, 3), (0, 2, 0, 3)]
        {
            assert!(focus_target(&repeater, remove as i32).focus());
            if keyed {
                // A reset with unchanged keys must preserve the focused root.
                let focused = root.focus_manager().get_focused_element();
                let mut values = keys.values.borrow().clone();
                keys.reset(values.clone());
                root.layout_manager().execute_layout_pass();
                assert_eq!(root.focus_manager().get_focused_element(), focused);
                values.drain(remove..remove + count);
                keys.reset(values);
            } else {
                list.remove_range(remove, count);
            }
            root.layout_manager().execute_layout_pass();
            assert_eq!(
                root.focus_manager().get_focused_element(),
                Some(focus_target(&repeater, expected_index))
            );
            let container = repeater.try_get_element(expected_index).unwrap();
            assert_eq!(
                *container
                    .data_context()
                    .unwrap()
                    .downcast_ref::<i32>()
                    .unwrap(),
                expected_value
            );
        }
    }
}

#[test]
fn nested_repeater_focus_pins_each_ancestor_once_and_releases_the_previous_group() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let (first, _) = fixture(ItemsSource::from_values(0..2), 2, false);
    let second_source = Rc::new(FerroList::from_items(0..2));
    let (second, _) = fixture(second_source.clone().into(), 2, false);
    let groups = [StackPanel::new(), StackPanel::new()];
    groups[0].children().add(first.clone());
    groups[1].children().add(second.clone());
    let outer = ItemsRepeater::new();
    outer.set_layout(Some(WindowLayout::new(0, 2).upcast()));
    outer.set_items_source(Some(ItemsSource::from_items(
        groups
            .iter()
            .map(|group| Some(Control::boxed(group.clone()))),
    )));
    let root = TestRoot::with_child(&outer);
    root.execute_initial_layout_pass();
    let focused = first.try_get_element(0).unwrap();
    assert!(focused.focus());
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&focused).pin_count(),
        1
    );
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&groups[0]).pin_count(),
        1
    );
    first.clear_element_impl(&focused);
    first.clear_element_impl(&first.try_get_element(1).unwrap());
    outer.clear_element_impl(&groups[0].clone().upcast());
    assert_eq!(first.get_element_index(&focused), 0);
    assert_eq!(outer.get_element_index(&groups[0].clone().upcast()), 0);
    let second_focused = second.try_get_element(1).unwrap();
    assert!(second_focused.focus());
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&focused).pin_count(),
        0
    );
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&groups[0]).pin_count(),
        0
    );
    first.view_manager.prune_pinned_elements(&first);
    outer.view_manager.prune_pinned_elements(&outer);
    assert_eq!(first.get_element_index(&focused), -1);
    assert_eq!(outer.get_element_index(&groups[0].clone().upcast()), -1);
    second_source.remove_at(1);
    root.layout_manager().execute_layout_pass();
    assert_eq!(second.get_element_index(&second_focused), -1);
}

#[test]
fn flow_layout_clears_scrolled_edges_from_the_outside_in() {
    let _scope = test_scope();
    let (repeater, _) = fixture(ItemsSource::from_values(0..15), 15, false);
    repeater.set_layout(Some(StackLayout::new().upcast()));
    repeater.set_vertical_cache_length(0.0);
    repeater.set_horizontal_cache_length(0.0);
    let scroll = scroller(&repeater);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    let events = Events::attach(&repeater);
    scroll.set_offset(Vector::new(0.0, 100.0));
    settle(&root);
    assert!(
        events.cleared.borrow().starts_with(&[0, 1, 2, 3]),
        "{:?}",
        events.cleared.borrow()
    );
    events.cleared.borrow_mut().clear();
    scroll.set_offset(Vector::default());
    settle(&root);
    assert!(
        events.cleared.borrow().starts_with(&[14, 13, 12, 11]),
        "{:?}",
        events.cleared.borrow()
    );
}

#[test]
fn bringing_the_already_visible_last_item_into_view_preserves_offset() {
    let _scope = test_scope();
    let (repeater, _) = fixture(ItemsSource::from_values(0..100), 100, false);
    repeater.set_layout(Some(StackLayout::new().upcast()));
    repeater.set_vertical_cache_length(0.0);
    let scroll = scroller(&repeater);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    scroll.set_offset(Vector::new(0.0, 100_000.0));
    settle(&root);
    let offset = scroll.offset();
    assert!(offset.y > 1000.0);
    let last = repeater.get_or_create_element(99);
    settle(&root);
    last.bring_into_view();
    settle(&root);
    assert!((scroll.offset().y - offset.y).abs() < 1.0);
    assert_eq!(repeater.get_element_index(&last), 99);
}

#[test]
fn items_source_switches_through_null_and_distinct_lists_in_order() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(None);
    assert!(repeater.items_source_view().is_none());
    for first in [0, 5, 10] {
        if first == 10 {
            repeater.set_items_source(None);
            assert!(repeater.items_source_view().is_none());
        }
        repeater.set_items_source(Some(ItemsSource::from_values(first..first + 5)));
        let view = repeater.items_source_view().unwrap();
        assert_eq!(view.count(), 5);
        for offset in 0..5 {
            assert_eq!(
                *view.get_at(offset).unwrap().downcast_ref::<i32>().unwrap(),
                first + offset as i32
            );
        }
    }
    repeater.set_items_source(None);
    assert!(repeater.items_source_view().is_none());
}

#[test]
fn clearing_an_explicit_item_template_returns_to_default_text_blocks() {
    let _scope = test_scope();
    let (repeater, _) = fixture(
        ItemsSource::from_values((0..3).map(|index| format!("Item #{index}"))),
        3,
        false,
    );
    run(&repeater);
    assert!(repeater.try_get_element(0).unwrap().is::<Button>());
    repeater.set_item_template(None);
    run(&repeater);
    for index in 0..3 {
        let text = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(text.text(), Some(format!("Item #{index}")));
        assert_eq!(repeater.get_element_index(&text.upcast()), index);
    }
    assert!(repeater.try_get_element(20).is_none());
}

#[test]
fn source_reset_to_empty_recycles_a_focused_element() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    for non_virtualizing in [false, true] {
        let (repeater, _) = fixture(ItemsSource::from_values(0..10), 10, false);
        let layout: Ref<Layout> = if non_virtualizing {
            AllItemsLayout::new().upcast()
        } else {
            StackLayout::new().upcast()
        };
        repeater.set_layout(Some(layout));
        let root = TestRoot::with_child(&repeater);
        // The native fixture has no implicit Button theme to supply a width.
        // Match the fixed viewport of the source window so all ten rows can
        // be realized before focusing the middle one.
        root.set_width(100.0);
        root.set_height(200.0);
        root.execute_initial_layout_pass();
        let focused = repeater.try_get_element(4).unwrap();
        assert!(focused.focus());
        repeater.set_items_source(Some(ItemsSource::from_values(std::iter::empty::<i32>())));
        root.layout_manager().execute_layout_pass();
        assert!(repeater.realized_elements().is_empty());
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&focused).owner(),
            ElementOwner::ElementFactory
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&focused).pin_count(),
            0
        );
        for index in 0..10 {
            assert!(repeater.try_get_element(index).is_none());
        }
    }
}

fn validate_native_tab_order(repeater: &ItemsRepeater, root: &TestRoot) {
    let expected: Vec<_> = repeater
        .realized_elements()
        .into_iter()
        .map(|element| element.upcast::<InputElement>())
        .collect();
    assert!(!expected.is_empty());
    assert!(expected
        .last()
        .unwrap()
        .focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    for (direction, sequence) in [
        (NavigationDirection::Next, expected.clone()),
        (
            NavigationDirection::Previous,
            expected.iter().rev().skip(1).cloned().collect(),
        ),
    ] {
        for expected in sequence {
            let current = root.focus_manager().get_focused_element().unwrap();
            assert_eq!(
                KeyboardNavigationHandler::get_next(&current, direction),
                Some(expected.clone())
            );
            let mut event = KeyEventArgs::new();
            event.set_routed_event(Some(InputElement::key_down_event()));
            event.key = Key::Tab;
            event.key_modifiers = if direction == NavigationDirection::Previous {
                KeyModifiers::SHIFT
            } else {
                KeyModifiers::NONE
            };
            current.raise_event(&event);
            assert!(event.handled());
            assert_eq!(root.focus_manager().get_focused_element(), Some(expected));
        }
    }
}

#[test]
fn tab_navigation_follows_source_order_after_scroll_and_bulk_removal() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let source = Rc::new(FerroList::from_items(0..50));
    let repeater = scrolling_repeater(Some(source.clone().into()), 100.0, true);
    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Local);
    let scroll = scroller(&repeater);
    scroll.set_width(400.0);
    scroll.set_height(400.0);
    let root = TestRoot::with_child(&scroll);
    let navigation = KeyboardNavigationHandler::new();
    navigation.set_owner(&root.clone().upcast());
    root.execute_initial_layout_pass();
    settle(&root);
    validate_native_tab_order(&repeater, &root);
    scroll.set_offset(Vector::new(0.0, 2000.0));
    settle(&root);
    assert_eq!(scroll.offset().y, 2000.0);
    validate_native_tab_order(&repeater, &root);
    while source.count() > 21 {
        source.remove_at(21);
    }
    settle(&root);
    // The pool still owns children of this panel. Native focus traversal
    // must exclude them and continue using the survivors' source indices.
    assert!(repeater.children().count() > repeater.realized_elements().len());
    validate_native_tab_order(&repeater, &root);
}

#[test]
fn native_tab_navigation_honors_once_local_cycle_contained_and_none_modes() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let (repeater, _) = fixture(ItemsSource::from_values(0..3), 3, false);
    let before = Button::new();
    let after = Button::new();
    let panel = StackPanel::new();
    panel.children().add(before.clone());
    panel.children().add(repeater.clone());
    panel.children().add(after.clone());
    let root = TestRoot::with_child(&panel);
    root.execute_initial_layout_pass();
    let first = focus_target(&repeater, 0);
    let middle = focus_target(&repeater, 1);
    let last = focus_target(&repeater, 2);
    let before = before.upcast::<InputElement>();
    let after = after.upcast::<InputElement>();
    let next = NavigationDirection::Next;
    let previous = NavigationDirection::Previous;
    let find = |current: &Ref<InputElement>, direction| {
        assert!(current.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
        KeyboardNavigationHandler::get_next(current, direction)
    };

    assert_eq!(
        KeyboardNavigation::get_tab_navigation(&repeater),
        KeyboardNavigationMode::Once
    );
    assert_eq!(find(&middle, next), Some(after.clone()));
    assert_eq!(find(&middle, previous), Some(before.clone()));
    assert_eq!(find(&before, next), Some(middle.clone()));
    assert_eq!(find(&after, previous), Some(middle.clone()));

    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Local);
    assert_eq!(find(&last, next), Some(after.clone()));
    assert_eq!(find(&before, next), Some(first.clone()));
    assert_eq!(find(&middle, next), Some(last.clone()));
    assert_eq!(find(&middle, previous), Some(first.clone()));
    assert_eq!(find(&first, previous), Some(before.clone()));

    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Cycle);
    assert_eq!(find(&last, next), Some(first.clone()));
    assert_eq!(find(&first, previous), Some(last.clone()));

    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Contained);
    assert_eq!(find(&last, next), None);
    assert_eq!(find(&first, previous), None);
    assert_eq!(find(&middle, next), Some(last));

    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::None);
    assert_eq!(find(&before, next), Some(after.clone()));
    assert_eq!(find(&after, previous), Some(before));
}

#[test]
fn native_once_navigation_does_not_return_to_a_recycled_or_animating_active_item() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let clock = super::services_tests::TestClock::install();
    for animate in [false, true] {
        let source = Rc::new(FerroList::from_items([0]));
        let (repeater, _) = fixture(source.clone().into(), 1, false);
        let before = Button::new();
        let after = Button::new();
        let panel = StackPanel::new();
        panel.children().add(before.clone());
        panel.children().add(repeater.clone());
        panel.children().add(after.clone());
        let root = TestRoot::with_child(&panel);
        root.execute_initial_layout_pass();
        if animate {
            repeater.set_item_transition_provider(Some(
                LinedFlowLayoutItemCollectionTransitionProvider::new().upcast(),
            ));
        }
        let removed = repeater.try_get_element(0).unwrap();
        assert!(removed.focus());
        assert_eq!(
            KeyboardNavigation::get_tab_once_active_element(&repeater),
            Some(removed.clone().upcast())
        );
        source.remove_at(0);
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&removed).owner(),
            if animate {
                ElementOwner::Animator
            } else {
                ElementOwner::ElementFactory
            }
        );
        assert!(removed.visual_parent().is_some());
        assert!(before.focus());
        assert_eq!(
            KeyboardNavigationHandler::get_next(&before.upcast(), NavigationDirection::Next),
            Some(after.upcast()),
            "retained removed element, animate={animate}"
        );
        let start = if animate { 2000.0 } else { 0.0 };
        clock.pulse(start);
        clock.pulse(start + 1000.0);
    }
}

#[test]
fn nested_once_navigation_remembers_the_inner_target_without_duplicating_pins() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());
    let (first, _) = fixture(ItemsSource::from_values(0..2), 2, false);
    let second_source = Rc::new(FerroList::from_items(0..2));
    let (second, _) = fixture(second_source.clone().into(), 2, false);
    let groups = [StackPanel::new(), StackPanel::new()];
    groups[0].children().add(first.clone());
    groups[1].children().add(second.clone());
    let outer = ItemsRepeater::new();
    outer.set_layout(Some(WindowLayout::new(0, 2).upcast()));
    outer.set_items_source(Some(ItemsSource::from_items(
        groups
            .iter()
            .map(|group| Some(Control::boxed(group.clone()))),
    )));
    let before = Button::new();
    let after = Button::new();
    let panel = StackPanel::new();
    panel.children().add(before.clone());
    panel.children().add(outer.clone());
    panel.children().add(after);
    let root = TestRoot::with_child(&panel);
    root.execute_initial_layout_pass();
    let focused = second.try_get_element(1).unwrap();
    assert!(focused.focus());
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&focused).pin_count(),
        1
    );
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&groups[1]).pin_count(),
        1
    );
    assert_eq!(
        KeyboardNavigation::get_tab_once_active_element(&outer),
        Some(focused.clone().upcast())
    );
    assert_eq!(
        KeyboardNavigation::get_tab_once_active_element(&second),
        Some(focused.clone().upcast())
    );
    assert!(before.focus());
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&focused).pin_count(),
        0
    );
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&groups[1]).pin_count(),
        0
    );
    assert_eq!(
        KeyboardNavigationHandler::get_next(&before.clone().upcast(), NavigationDirection::Next),
        Some(focused.clone().upcast())
    );

    assert!(focused.focus());
    second_source.remove_at(1);
    root.layout_manager().execute_layout_pass();
    let remaining = second.try_get_element(0).unwrap();
    assert_eq!(
        KeyboardNavigation::get_tab_once_active_element(&outer),
        Some(remaining.clone().upcast())
    );
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&remaining).pin_count(),
        1
    );
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&groups[1]).pin_count(),
        1
    );
    second_source.remove_at(0);
    root.layout_manager().execute_layout_pass();
    assert!(KeyboardNavigation::get_tab_once_active_element(&outer).is_none());
    assert!(KeyboardNavigation::get_tab_once_active_element(&second).is_none());
    assert!(before.focus());
    assert_eq!(
        KeyboardNavigationHandler::get_next(&before.upcast(), NavigationDirection::Next),
        Some(first.try_get_element(0).unwrap().upcast())
    );
}

#[test]
fn nested_focus_releases_pins_before_direct_or_factory_owned_rows_are_detached() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());

    struct DetachingFactory {
        group: ferroui_base::WeakRef<Control>,
        observed_pins: Rc<RefCell<Vec<u32>>>,
        redirect: Option<ferroui_base::WeakRef<InputElement>>,
    }
    impl IElementFactory for DetachingFactory {
        fn get_element(
            &self,
            _args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            let button = Button::new();
            button.set_height(20.0);
            Ok(button.upcast())
        }
        fn recycle_element(
            &self,
            args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            self.observed_pins.borrow_mut().push(
                ItemsRepeater::get_virtualization_info(&self.group.upgrade().unwrap()).pin_count(),
            );
            args.parent()
                .unwrap()
                .cast::<ItemsRepeater>()
                .unwrap()
                .children()
                .remove(&args.element());
            if let Some(redirect) = self
                .redirect
                .as_ref()
                .and_then(ferroui_base::WeakRef::upgrade)
            {
                assert!(redirect.focus());
            }
            Ok(())
        }
    }

    for (use_factory, redirect) in [(false, false), (true, false), (true, true)] {
        let items: Vec<Option<BoxedValue>> = (0..3)
            .map(|index| {
                if use_factory {
                    Some(Rc::new(index) as BoxedValue)
                } else {
                    let item = Button::new();
                    item.set_height(20.0);
                    Some(Control::boxed(item))
                }
            })
            .collect();
        let source = Rc::new(FerroList::from_items(items));
        let inner = ItemsRepeater::new();
        inner.set_layout(Some(WindowLayout::new(0, 3).upcast()));
        let group = StackPanel::new();
        group.children().add(inner.clone());
        let after = Button::new();
        let observed_pins = Rc::new(RefCell::new(Vec::new()));
        if use_factory {
            inner.set_element_factory(Some(Rc::new(DetachingFactory {
                group: group.clone().upcast::<Control>().downgrade(),
                observed_pins: observed_pins.clone(),
                redirect: redirect.then(|| after.clone().upcast::<InputElement>().downgrade()),
            })));
        }
        inner.set_items_source(Some(source.clone().into()));
        let outer = ItemsRepeater::new();
        outer.set_layout(Some(WindowLayout::new(0, 1).upcast()));
        outer.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(
            group.clone(),
        ))])));
        let panel = StackPanel::new();
        panel.children().add(outer.clone());
        panel.children().add(after.clone());
        let root = TestRoot::with_child(&panel);
        root.execute_initial_layout_pass();
        let removed = inner.try_get_element(1).unwrap();
        assert!(removed.focus());
        // An independent pin makes a duplicate focus release observable.
        outer.pin_element(&group.clone().upcast());
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&group).pin_count(),
            2
        );
        source.remove_at(1);
        root.layout_manager().execute_layout_pass();
        assert!(removed.visual_parent().is_none());
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&removed).pin_count(),
            0
        );
        if redirect {
            assert_eq!(
                root.focus_manager().get_focused_element(),
                Some(after.upcast())
            );
            assert_eq!(
                ItemsRepeater::get_virtualization_info(&group).pin_count(),
                1
            );
        } else {
            let replacement = inner.try_get_element(1).unwrap();
            assert_eq!(
                root.focus_manager().get_focused_element(),
                Some(replacement.clone().upcast())
            );
            assert_eq!(
                ItemsRepeater::get_virtualization_info(&replacement).pin_count(),
                1
            );
            assert_eq!(
                ItemsRepeater::get_virtualization_info(&group).pin_count(),
                2
            );
            source.remove_at(1);
            root.layout_manager().execute_layout_pass();
            source.remove_at(0);
            root.layout_manager().execute_layout_pass();
            assert_eq!(
                ItemsRepeater::get_virtualization_info(&group).pin_count(),
                1
            );
        }
        if use_factory {
            assert_eq!(
                *observed_pins.borrow(),
                vec![1; if redirect { 1 } else { 3 }]
            );
        }
        outer.unpin_element(&group.upcast());
    }
}

#[test]
fn clearing_an_enclosing_row_releases_nested_focus_before_detach_or_pooling() {
    let _scope = test_scope();
    let _app = UnitTestApplication::start(TestServices::real_focus());

    struct EnclosingRowFactory {
        ancestor: ferroui_base::WeakRef<Control>,
        observed_pins: Rc<RefCell<Vec<u32>>>,
        detach: bool,
        redirect: Option<ferroui_base::WeakRef<InputElement>>,
    }
    impl IElementFactory for EnclosingRowFactory {
        fn get_element(
            &self,
            args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            Ok(args.data().as_ref().and_then(Control::from_boxed).unwrap())
        }
        fn recycle_element(
            &self,
            args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            // Only the independent pin may remain before a factory can cut
            // the visual path from the focused inner row to this ancestor.
            self.observed_pins.borrow_mut().push(
                ItemsRepeater::get_virtualization_info(&self.ancestor.upgrade().unwrap())
                    .pin_count(),
            );
            if self.detach {
                args.parent()
                    .unwrap()
                    .cast::<ItemsRepeater>()
                    .unwrap()
                    .children()
                    .remove(&args.element());
            }
            if let Some(redirect) = self
                .redirect
                .as_ref()
                .and_then(ferroui_base::WeakRef::upgrade)
            {
                assert!(redirect.focus());
            }
            Ok(())
        }
    }

    for (use_factory, detach, redirect) in [
        (false, true, false),
        (true, true, false),
        (true, false, false),
        (true, true, true),
        (true, false, true),
    ] {
        let (first, _) = fixture(ItemsSource::from_values(0..2), 2, false);
        let (second, _) = fixture(ItemsSource::from_values(0..2), 2, false);
        let groups = [StackPanel::new(), StackPanel::new()];
        groups[0].children().add(first.clone());
        groups[1].children().add(second.clone());
        let source = Rc::new(FerroList::from_items(
            groups
                .iter()
                .map(|group| Some(Control::boxed(group.clone()))),
        ));
        let middle = ItemsRepeater::new();
        middle.set_layout(Some(WindowLayout::new(0, 2).upcast()));
        let ancestor = StackPanel::new();
        ancestor.children().add(middle.clone());
        let after = Button::new();
        let observed_pins = Rc::new(RefCell::new(Vec::new()));
        if use_factory {
            middle.set_element_factory(Some(Rc::new(EnclosingRowFactory {
                ancestor: ancestor.clone().upcast::<Control>().downgrade(),
                observed_pins: observed_pins.clone(),
                detach,
                redirect: redirect.then(|| after.clone().upcast::<InputElement>().downgrade()),
            })));
        }
        middle.set_items_source(Some(source.clone().into()));
        let outer = ItemsRepeater::new();
        outer.set_layout(Some(WindowLayout::new(0, 1).upcast()));
        outer.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(
            ancestor.clone(),
        ))])));
        let panel = StackPanel::new();
        panel.children().add(outer.clone());
        panel.children().add(after.clone());
        let root = TestRoot::with_child(&panel);
        root.execute_initial_layout_pass();
        let focused = first.try_get_element(1).unwrap();
        let replacement = second.try_get_element(0).unwrap();
        assert!(focused.focus());
        outer.pin_element(&ancestor.clone().upcast());
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&focused).pin_count(),
            1
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&groups[0]).pin_count(),
            1
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&ancestor).pin_count(),
            2
        );

        source.remove_at(0);
        root.layout_manager().execute_layout_pass();
        assert_eq!(groups[0].visual_parent().is_none(), detach);
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&groups[0]).owner(),
            ElementOwner::ElementFactory
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&focused).pin_count(),
            0
        );
        assert_eq!(
            root.focus_manager().get_focused_element(),
            Some(if redirect {
                after.clone().upcast()
            } else {
                replacement.clone().upcast()
            })
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&replacement).pin_count(),
            if redirect { 0 } else { 1 }
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&ancestor).pin_count(),
            if redirect { 1 } else { 2 }
        );

        // Removing the final enclosing row has no replacement. Pooled rows
        // remain attached, so their stale focused leaf must be cleared too.
        source.remove_at(0);
        root.layout_manager().execute_layout_pass();
        assert_eq!(groups[1].visual_parent().is_none(), detach);
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&replacement).pin_count(),
            0
        );
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&ancestor).pin_count(),
            1
        );
        assert_eq!(
            root.focus_manager().get_focused_element(),
            redirect.then(|| after.clone().upcast())
        );
        if use_factory {
            assert_eq!(*observed_pins.borrow(), vec![1, 1]);
        }
        assert!(after.focus());
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&ancestor).pin_count(),
            1
        );
        outer.unpin_element(&ancestor.clone().upcast());
        assert_eq!(
            ItemsRepeater::get_virtualization_info(&ancestor).pin_count(),
            0
        );
    }
}

#[test]
fn disconnected_bring_into_view_preserves_realization_order_with_both_cache_lengths() {
    let _scope = test_scope();
    for cache in [0.0, 2.0] {
        let repeater = scrolling_repeater(None, 24.0, false);
        repeater.set_horizontal_cache_length(cache);
        repeater.set_vertical_cache_length(cache);
        let realized = Rc::new(RefCell::new(Vec::new()));
        let prepared = realized.clone();
        repeater.element_prepared(move |args| prepared.borrow_mut().push(args.index));
        let cleared = realized.clone();
        let weak = repeater.downgrade();
        repeater.element_clearing(move |args| {
            let index = weak.upgrade().unwrap().get_element_index(&args.element);
            cleared.borrow_mut().retain(|entry| *entry != index);
        });
        let scroll = scroller(&repeater);
        scroll.set_width(200.0);
        let root = TestRoot::with_child(&scroll);
        root.execute_initial_layout_pass();
        repeater.set_items_source(Some(ItemsSource::from_values(0..200)));
        settle(&root);
        let initial = if cache == 0.0 { 0..10 } else { 0..18 };
        assert_eq!(
            *realized.borrow(),
            initial.collect::<Vec<_>>(),
            "initial cache {cache}"
        );
        for (index, expected) in if cache == 0.0 {
            vec![
                (99, vec![99, 100, 98, 97, 96, 95, 94, 93, 92, 91, 90]),
                (199, (190..200).rev().collect()),
                (49, (49..59).chain([48]).collect()),
                (0, (0..10).collect()),
            ]
        } else {
            vec![
                (99, (99..110).chain((82..99).rev()).collect()),
                (199, (182..200).rev().collect()),
                (49, (49..67).chain((39..49).rev()).collect()),
                (0, (0..18).collect()),
            ]
        } {
            let target = repeater.get_or_create_element(index);
            root.layout_manager().execute_layout_pass();
            target.bring_into_view();
            settle(&root);
            assert_eq!(
                *realized.borrow(),
                expected,
                "index {index}, cache {cache}, offset {:?}",
                scroll.offset()
            );
            assert_eq!(repeater.get_element_index(&target), index);
        }
    }
}
