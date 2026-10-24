// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Behavioral ports of Repeater/APITests/RepeaterTests.cs and ViewManagerTests.cs;
// additional tests cover Rust ownership and native FerroUI integration.

use super::*;
use ferroui_controls::ItemsChangedHandler;
use ferroui_controls::templates::FuncDataTemplate;
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::{
    Border, Button, Control, IItemsList, IKeyIndexMapping, ItemsChangedEventArgs, ItemsSource,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, NavigationDirection};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Rect, Ref, Size,
    Thickness,
};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

#[repr(C)]
struct WindowLayout {
    base: VirtualizingLayout,
    requests: RefCell<Vec<(i32, ElementRealizationOptions)>>,
    live: RefCell<Vec<(i32, Ref<Control>)>>,
    measure_hook: RefCell<Option<Rc<dyn Fn(&Ref<VirtualizingLayoutContext>)>>>,
    arrange_hook: RefCell<Option<Rc<dyn Fn()>>>,
    uninitialized: Cell<usize>,
}
ferro_class!(WindowLayout: VirtualizingLayout);
ferro_impl_classes!(WindowLayout: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for WindowLayout {
    fn uninitialize_for_context_core(this: &Self, _context: &Ref<VirtualizingLayoutContext>) {
        this.uninitialized.set(this.uninitialized.get() + 1);
    }
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        available: Size,
    ) -> Size {
        let hook = this.measure_hook.borrow().clone();
        if let Some(hook) = hook {
            hook(context);
        }
        let requests = this.requests.borrow().clone();
        let mut live = Vec::new();
        for (index, options) in requests {
            let element = context.get_or_create_element_at_with_options(index, options);
            element.measure(available);
            live.push((index, element));
        }
        *this.live.borrow_mut() = live;
        Size::new(100.0, context.item_count() as f64 * 20.0)
    }
    fn arrange_override(
        this: &Self,
        _context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        let live = this.live.borrow().clone();
        for (index, element) in live {
            element.arrange(Rect::new(0.0, index as f64 * 20.0, 100.0, 20.0));
        }
        let hook = this.arrange_hook.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
        final_size
    }
    fn on_items_changed_core(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        args: &ItemsChangedEventArgs<'_>,
    ) {
        if matches!(
            args.action,
            ferroui_base::collections::NotifyCollectionChangedAction::Replace
                | ferroui_base::collections::NotifyCollectionChangedAction::Reset
        ) {
            let live = this.live.borrow().clone();
            for (_, element) in live {
                let index = ItemsRepeater::get_virtualization_info(&element).index();
                if args.action == ferroui_base::collections::NotifyCollectionChangedAction::Reset
                    || (index >= args.old_starting_index
                        && index < args.old_starting_index + args.old_items.len() as i32)
                {
                    context.recycle_element(&element);
                }
            }
        }
    }
}
impl WindowLayout {
    fn new(indices: &[i32]) -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            requests: RefCell::new(
                indices
                    .iter()
                    .map(|i| (*i, ElementRealizationOptions::NONE))
                    .collect(),
            ),
            live: RefCell::new(Vec::new()),
            measure_hook: RefCell::new(None),
            arrange_hook: RefCell::new(None),
            uninitialized: Cell::new(0),
        })
    }
    fn set_indices(&self, indices: &[i32]) {
        *self.requests.borrow_mut() = indices
            .iter()
            .map(|i| (*i, ElementRealizationOptions::NONE))
            .collect();
        self.invalidate_measure();
    }
}
struct CountingFactory {
    pool: Ref<RecyclePool>,
    created: Cell<usize>,
    cleared: RefCell<Vec<Ref<Control>>>,
}
impl CountingFactory {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            pool: RecyclePool::new(),
            created: Cell::new(0),
            cleared: RefCell::new(Vec::new()),
        })
    }
}
impl IElementFactory for CountingFactory {
    fn get_element(
        &self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        if let Some(element) = self
            .pool
            .try_get_element_with_owner("item", args.parent())?
        {
            return Ok(element);
        }
        self.created.set(self.created.get() + 1);
        let element = Border::new();
        element.set_width(90.0);
        element.set_height(20.0);
        Ok(element.upcast())
    }
    fn recycle_element(&self, args: &ElementFactoryRecycleArgs) -> Result<(), ElementFactoryError> {
        self.cleared.borrow_mut().push(args.element());
        self.pool
            .put_element_with_owner(args.element(), "item", args.parent())
    }
}
fn fixture(
    count: i32,
    indices: &[i32],
) -> (Ref<ItemsRepeater>, Ref<WindowLayout>, Rc<CountingFactory>) {
    let repeater = ItemsRepeater::new();
    repeater.set_use_layout_rounding(false);
    let layout = WindowLayout::new(indices);
    let factory = CountingFactory::new();
    repeater.set_layout(Some(layout.clone().upcast()));
    repeater.set_element_factory(Some(factory.clone()));
    repeater.set_items_source(Some(ItemsSource::from_values(0..count)));
    (repeater, layout, factory)
}
fn layout(repeater: &ItemsRepeater) {
    repeater.measure(Size::new(100.0, 100.0));
    repeater.arrange(Rect::new(
        0.0,
        0.0,
        100.0,
        repeater.desired_size().height.max(100.0),
    ));
}
fn item_number(element: &Control) -> i32 {
    *element
        .data_context()
        .unwrap()
        .downcast_ref::<i32>()
        .unwrap()
}

#[test]
fn validate_default_properties_and_empty_source() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    assert!(repeater.items_source().is_none());
    assert!(repeater.items_source_view().is_none());
    assert!(repeater.item_template().is_none());
    assert!(repeater.layout().is_none());
    assert_eq!(repeater.horizontal_cache_length(), 2.0);
    assert_eq!(repeater.vertical_cache_length(), 2.0);
    repeater.measure(Size::new(100.0, 100.0));
    assert_eq!(repeater.desired_size(), Size::default());
    assert!(repeater.try_get_element(0).is_none());
}

#[test]
fn validate_element_prepared_and_clearing_observe_parent_data_and_index() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(4, &[1]);
    let events = Rc::new(RefCell::new(Vec::new()));
    let saved = events.clone();
    let weak = repeater.downgrade();
    repeater.element_prepared(move |e| {
        let owner = weak.upgrade().unwrap();
        assert_eq!(owner.get_element_index(&e.element), e.index);
        saved
            .borrow_mut()
            .push(("prepared", e.index, item_number(&e.element)));
    });
    let saved = events.clone();
    let weak = repeater.downgrade();
    repeater.element_clearing(move |e| {
        saved.borrow_mut().push((
            "clearing",
            weak.upgrade().unwrap().get_element_index(&e.element),
            item_number(&e.element),
        ));
    });
    layout(&repeater);
    let element = repeater.try_get_element(1).unwrap();
    window.set_indices(&[]);
    layout(&repeater);
    assert_eq!(&*events.borrow(), &[("prepared", 1, 1), ("clearing", 1, 1)]);
    assert_eq!(repeater.get_element_index(&element), -1);
    assert!(element.data_context().is_none());
    assert_eq!(factory.cleared.borrow().len(), 1);
    assert_eq!(element.previous_arrange().unwrap().size(), Size::default());
    assert!(element.bounds().y < -10000.0);
}

#[test]
fn validate_recycling_reuses_roots_and_updates_data_context() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(10, &[0, 1]);
    layout(&repeater);
    let old = repeater.realized_elements();
    window.set_indices(&[]);
    layout(&repeater);
    window.set_indices(&[8, 9]);
    layout(&repeater);
    assert_eq!(factory.created.get(), 2);
    for i in [8, 9] {
        let element = repeater.try_get_element(i).unwrap();
        assert!(old.contains(&element));
        assert_eq!(item_number(&element), i);
    }
    assert_eq!(repeater.children().count(), 2);
}

#[test]
fn validate_force_create_produces_two_roots_for_one_index() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(1, &[0]);
    window
        .requests
        .borrow_mut()
        .push((0, ElementRealizationOptions::FORCE_CREATE));
    layout(&repeater);
    assert_eq!(factory.created.get(), 2);
    let elements = repeater.realized_elements();
    assert_eq!(elements.len(), 2);
    assert_ne!(elements[0], elements[1]);
    assert_eq!(repeater.get_element_index(&elements[0]), 0);
    assert_eq!(repeater.get_element_index(&elements[1]), 0);
}

#[test]
fn validate_suppress_auto_recycle_requires_explicit_recycle() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(1, &[]);
    *window.requests.borrow_mut() = vec![(0, ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE)];
    layout(&repeater);
    let element = repeater.try_get_element(0).unwrap();
    window.set_indices(&[]);
    layout(&repeater);
    assert_eq!(repeater.try_get_element(0), Some(element.clone()));
    repeater.clear_element_impl(&element);
    assert!(repeater.try_get_element(0).is_none());
    assert_eq!(factory.cleared.borrow().len(), 1);
}

#[test]
fn validate_pinned_element_survives_virtualization_and_is_reused() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(2, &[0]);
    layout(&repeater);
    let element = repeater.try_get_element(0).unwrap();
    repeater.pin_element(&element);
    window.set_indices(&[]);
    layout(&repeater);
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&element).owner(),
        ElementOwner::PinnedPool
    );
    assert_eq!(repeater.try_get_element(0), Some(element.clone()));
    assert!(factory.cleared.borrow().is_empty());
    window.set_indices(&[0]);
    layout(&repeater);
    assert_eq!(repeater.try_get_element(0), Some(element.clone()));
    repeater.unpin_element(&element);
    window.set_indices(&[]);
    layout(&repeater);
    assert!(repeater.try_get_element(0).is_none());
}

#[test]
fn validate_unpin_prunes_an_offscreen_element_on_next_measure() {
    let _scope = test_scope();
    let (repeater, window, _) = fixture(1, &[0]);
    layout(&repeater);
    let element = repeater.try_get_element(0).unwrap();
    repeater.pin_element(&element);
    window.set_indices(&[]);
    layout(&repeater);
    repeater.unpin_element(&element);
    layout(&repeater);
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&element).owner(),
        ElementOwner::ElementFactory
    );
}

#[test]
fn validate_collection_insert_remove_replace_and_move_preserve_indices() {
    let _scope = test_scope();
    let (repeater, _, _) = fixture(0, &[0, 1, 2]);
    let source = Rc::new(FerroList::from_items([10, 20, 30]));
    repeater.set_items_source(Some(source.clone().into()));
    layout(&repeater);
    let a = repeater.try_get_element(0).unwrap();
    let b = repeater.try_get_element(1).unwrap();
    let c = repeater.try_get_element(2).unwrap();
    let changed = Rc::new(RefCell::new(Vec::new()));
    let saved = changed.clone();
    repeater.element_index_changed(move |e| saved.borrow_mut().push((e.old_index, e.new_index)));
    source.insert(1, 15);
    assert_eq!(repeater.get_element_index(&b), 2);
    assert_eq!(repeater.get_element_index(&c), 3);
    source.move_item(3, 0);
    assert_eq!(repeater.get_element_index(&c), 0);
    assert_eq!(repeater.get_element_index(&a), 1);
    assert_eq!(repeater.get_element_index(&b), 3);
    source.set(1, 11);
    assert_eq!(repeater.get_element_index(&a), -1);
    source.remove_at(0);
    assert_eq!(repeater.get_element_index(&c), -1);
    assert_eq!(repeater.get_element_index(&b), 2);
    assert!(!changed.borrow().is_empty());
}

#[test]
fn collection_remove_clears_pins() {
    let _scope = test_scope();
    let (repeater, _, _) = fixture(0, &[0]);
    let source = Rc::new(FerroList::from_items([10]));
    repeater.set_items_source(Some(source.clone().into()));
    layout(&repeater);
    let element = repeater.try_get_element(0).unwrap();
    repeater.pin_element(&element);
    source.remove_at(0);
    assert_eq!(repeater.get_element_index(&element), -1);
    assert_eq!(
        ItemsRepeater::get_virtualization_info(&element).pin_count(),
        0
    );
}

struct KeyedSource {
    keys: RefCell<Vec<String>>,
    changes: HandlerList<ItemsChangedHandler>,
}
impl KeyedSource {
    fn new(keys: &[&str]) -> Rc<Self> {
        Rc::new(Self {
            keys: RefCell::new(keys.iter().map(|s| s.to_string()).collect()),
            changes: HandlerList::new(),
        })
    }
    fn reset(&self, keys: &[&str]) {
        *self.keys.borrow_mut() = keys.iter().map(|s| s.to_string()).collect();
        for (_, handler) in self.changes.snapshot().iter() {
            handler(&ItemsChangedEventArgs::RESET);
        }
    }
}
impl IKeyIndexMapping for KeyedSource {
    fn key_from_index(&self, index: i32) -> String {
        self.keys.borrow()[index as usize].clone()
    }
    fn index_from_key(&self, key: &str) -> i32 {
        self.keys
            .borrow()
            .iter()
            .position(|s| s == key)
            .map_or(-1, |i| i as i32)
    }
}
impl IItemsList for KeyedSource {
    fn key_index_mapping(&self) -> Option<&dyn IKeyIndexMapping> {
        Some(self)
    }
    fn count(&self) -> usize {
        self.keys.borrow().len()
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        Some(Rc::new(self.keys.borrow()[index].clone()))
    }
    fn is_notifying(&self) -> bool {
        true
    }
    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.changes.add(handler))
    }
    fn remove_collection_changed(&self, token: u64) {
        self.changes.remove(token);
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn validate_stable_reset_reuses_by_key_and_clears_leftovers_after_arrange() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(0, &[0, 1, 2]);
    let source = KeyedSource::new(&["a", "b", "c"]);
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    layout(&repeater);
    let a = repeater.try_get_element(0).unwrap();
    let b = repeater.try_get_element(1).unwrap();
    let c = repeater.try_get_element(2).unwrap();
    source.reset(&["c", "a"]);
    assert!(repeater.realized_elements().is_empty());
    assert!(factory.cleared.borrow().is_empty());
    window.set_indices(&[0, 1]);
    repeater.measure(Size::new(100.0, 100.0));
    assert_eq!(repeater.try_get_element(0), Some(c));
    assert_eq!(repeater.try_get_element(1), Some(a));
    assert!(factory.cleared.borrow().is_empty());
    repeater.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(&*factory.cleared.borrow(), &[b]);
    assert_eq!(factory.created.get(), 3);
}

#[test]
fn stable_reset_preserves_layout_released_pins_by_key_and_clears_leftovers() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(0, &[]);
    let source = KeyedSource::new(&["keep", "remove"]);
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    *window.requests.borrow_mut() = vec![
        (0, ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE),
        (1, ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE),
    ];
    layout(&repeater);
    let retained = repeater.try_get_element(0).unwrap();
    let removed = repeater.try_get_element(1).unwrap();
    let retained_info = ItemsRepeater::get_virtualization_info(&retained);
    let removed_info = ItemsRepeater::get_virtualization_info(&removed);
    let retained_generation = retained_info.pin_generation();
    let removed_generation = removed_info.pin_generation();
    repeater.pin_element(&retained);
    repeater.pin_element(&removed);

    // Flow layouts suppress automatic recycling, then explicitly release
    // offscreen rows. Once pinned, these rows no longer belong to the layout.
    let released = [retained.clone(), removed.clone()];
    *window.measure_hook.borrow_mut() = Some(Rc::new(move |context| {
        for element in &released {
            context.recycle_element(element);
        }
    }));
    window.set_indices(&[]);
    layout(&repeater);
    window.measure_hook.borrow_mut().take();
    for info in [&retained_info, &removed_info] {
        assert_eq!(info.owner(), ElementOwner::PinnedPool);
        assert!(!info.auto_recycle_candidate());
        assert_eq!(info.pin_count(), 1);
    }
    assert!(window.live.borrow().is_empty());

    source.reset(&["new", "keep"]);
    assert!(repeater.realized_elements().is_empty());
    for info in [&retained_info, &removed_info] {
        assert_eq!(info.owner(), ElementOwner::UniqueIdResetPool);
        assert_eq!(info.pin_count(), 1);
    }
    assert_eq!(retained_info.pin_generation(), retained_generation);
    assert_eq!(removed_info.pin_generation(), removed_generation);
    assert!(factory.cleared.borrow().is_empty());

    window.set_indices(&[1]);
    repeater.measure(Size::new(100.0, 100.0));
    assert_eq!(repeater.try_get_element(1), Some(retained.clone()));
    assert_eq!(retained_info.owner(), ElementOwner::Layout);
    assert_eq!(retained_info.index(), 1);
    assert_eq!(retained_info.pin_count(), 1);
    assert_eq!(retained_info.pin_generation(), retained_generation);
    assert_eq!(removed_info.owner(), ElementOwner::UniqueIdResetPool);
    assert!(factory.cleared.borrow().is_empty());

    repeater.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(&*factory.cleared.borrow(), &[removed.clone()]);
    assert_eq!(removed_info.owner(), ElementOwner::ElementFactory);
    assert_eq!(removed_info.pin_count(), 0);
    assert_ne!(removed_info.pin_generation(), removed_generation);
    assert_eq!(factory.created.get(), 2);

    // Returning from the reset pool must also remove its old pinned-pool
    // membership: unpinning a row still requested by layout cannot clear it.
    repeater.unpin_element(&retained);
    layout(&repeater);
    assert_eq!(repeater.try_get_element(1), Some(retained));
    assert_eq!(&*factory.cleared.borrow(), &[removed]);
}

#[test]
fn source_change_and_drop_unsubscribe() {
    let _scope = test_scope();
    let first = Rc::new(FerroList::from_items([1, 2]));
    let second = Rc::new(FerroList::from_items([3]));
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(first.clone().into()));
    assert!(first.has_collection_changed_subscribers());
    repeater.set_items_source(Some(second.clone().into()));
    assert!(!first.has_collection_changed_subscribers());
    assert!(second.has_collection_changed_subscribers());
    let weak = repeater.downgrade();
    drop(repeater);
    assert!(weak.upgrade().is_none());
    assert!(!second.has_collection_changed_subscribers());
}

#[test]
fn validate_mapping_and_auto_recycling() {
    let _scope = test_scope();
    let (repeater, window, factory) = fixture(5, &[]);
    let forced =
        ElementRealizationOptions::FORCE_CREATE | ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE;
    *window.requests.borrow_mut() = vec![
        (0, ElementRealizationOptions::NONE),
        (0, ElementRealizationOptions::NONE),
        (1, forced),
        (1, forced),
    ];
    layout(&repeater);
    {
        let live = window.live.borrow();
        assert!(live[0].1.ptr_eq(&live[1].1));
        assert!(!live[2].1.ptr_eq(&live[3].1));
    }
    assert_eq!(factory.created.get(), 3);
    assert!(repeater.try_get_element(0).is_some());
    assert!(repeater.try_get_element(1).is_some());

    // Match the source's second measure without any element requests: normal
    // mapping is automatically recycled; both explicitly retained roots live.
    window.set_indices(&[]);
    layout(&repeater);
    assert!(repeater.try_get_element(0).is_none());
    assert!(repeater.try_get_element(1).is_some());
    assert_eq!(repeater.realized_elements().len(), 2);
    assert_eq!(factory.cleared.borrow().len(), 1);
}

#[test]
fn template_change_preserves_old_pool_ownership_until_reuse() {
    let _scope = test_scope();
    let (repeater, _, old_factory) = fixture(1, &[0]);
    layout(&repeater);
    let old = repeater.try_get_element(0).unwrap();
    let new_factory = CountingFactory::new();
    repeater.set_element_factory(Some(new_factory.clone()));
    assert!(old.visual_parent().is_some());
    layout(&repeater);
    assert_eq!(old.previous_arrange().unwrap().size(), Size::default());
    assert!(old.bounds().y < -10000.0);
    assert_ne!(repeater.try_get_element(0), Some(old.clone()));
    let (other, _, _) = fixture(1, &[0]);
    other.set_element_factory(Some(old_factory));
    layout(&other);
    assert_eq!(other.try_get_element(0), Some(old));
    assert_eq!(repeater.children().count(), 1);
}

#[test]
fn direct_control_items_preserve_data_context() {
    let _scope = test_scope();
    let item = Border::new();
    item.set_height(20.0);
    item.set_data_context(Some(Rc::new("existing".to_string())));
    let repeater = ItemsRepeater::new();
    repeater.set_items_source(Some(ItemsSource::from_items([Some(Control::boxed(
        item.clone(),
    ))])));
    layout(&repeater);
    assert_eq!(repeater.try_get_element(0), Some(item.clone().upcast()));
    assert_eq!(
        *item
            .data_context()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "existing"
    );
    repeater.set_items_source(None);
    assert_eq!(
        *item
            .data_context()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "existing"
    );
    assert!(item.visual_parent().is_none());
}

#[test]
fn empty_template_is_measured_as_zero_and_can_be_replaced() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_data_template(Some(FuncDataTemplate::new(|_| true, |_, _| None, false)));
    repeater.set_items_source(Some(ItemsSource::from_values([1, 2, 3])));
    repeater.measure(Size::new(100.0, 100.0));
    assert_eq!(repeater.desired_size(), Size::default());
    assert_eq!(repeater.children().count(), 0);
    repeater.set_element_factory(Some(CountingFactory::new()));
    layout(&repeater);
    assert_eq!(repeater.realized_elements().len(), 3);
}

#[test]
fn get_or_create_makes_out_of_range_anchor_and_rejects_invalid_index() {
    let _scope = test_scope();
    let (repeater, _, _) = fixture(10, &[0]);
    repeater
        .viewport_manager
        .set_test_viewport(Rect::new(0.0, 400.0, 100.0, 100.0));
    let target = repeater.get_or_create_element(9);
    assert_eq!(repeater.get_element_index(&target), 9);
    assert_eq!(repeater.viewport_manager.made_anchor(), Some(target));
    assert_eq!(repeater.visible_window(), Rect::new(0.0, 0.0, 100.0, 100.0));
    assert!(catch_unwind(AssertUnwindSafe(|| repeater.get_or_create_element(-1))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| repeater.get_or_create_element(10))).is_err());
}

#[test]
fn layout_reentrancy_guards_recover_after_caught_panics() {
    let _scope = test_scope();
    let (repeater, window, _) = fixture(1, &[0]);
    let weak = repeater.downgrade();
    *window.measure_hook.borrow_mut() = Some(Rc::new(move |_| {
        weak.upgrade().unwrap().get_or_create_element(0);
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| layout(&repeater))).is_err());
    *window.measure_hook.borrow_mut() = None;
    repeater.invalidate_measure();
    layout(&repeater);
    assert!(repeater.try_get_element(0).is_some());
}

#[test]
fn verify_repeater_apis_throw_when_called_during_layout() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let window = WindowLayout::new(&[0, 1, 2, 3, 4]);
    repeater.set_layout(Some(window.clone().upcast()));
    repeater.set_items_source(Some(ItemsSource::from_values(
        (0..5).map(|index| format!("Item #{index}")),
    )));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |data, _| {
            let text = ferroui_controls::TextBlock::new();
            text.set_height(20.0);
            text.set_text(
                data.as_ref()
                    .and_then(|data| data.downcast_ref::<String>())
                    .map(String::as_str),
            );
            Some(text.upcast())
        },
        true,
    )));
    let failures = Rc::new(RefCell::new(Vec::new()));
    let observed = failures.clone();
    let weak = repeater.downgrade();
    let attempted = Cell::new(false);
    *window.measure_hook.borrow_mut() = Some(Rc::new(move |_| {
        if attempted.replace(true) {
            return;
        }
        let repeater = weak.upgrade().unwrap();
        observed.borrow_mut().push((
            "GetOrCreateElement",
            catch_unwind(AssertUnwindSafe(|| repeater.get_or_create_element(0))).is_err(),
        ));
        observed.borrow_mut().push((
            "ItemsSource",
            catch_unwind(AssertUnwindSafe(|| {
                repeater.set_items_source(Some(ItemsSource::from_values(["Other".to_owned()])))
            }))
            .is_err(),
        ));
        observed.borrow_mut().push((
            "ItemTemplate",
            catch_unwind(AssertUnwindSafe(|| {
                repeater.set_data_template(Some(FuncDataTemplate::new(
                    |_| true,
                    |_, _| Some(Button::new().upcast()),
                    true,
                )))
            }))
            .is_err(),
        ));
    }));
    let weak = repeater.downgrade();
    let original = window.downgrade();
    let observed = failures.clone();
    let attempted = Cell::new(false);
    *window.arrange_hook.borrow_mut() = Some(Rc::new(move || {
        if attempted.replace(true) {
            return;
        }
        let repeater = weak.upgrade().unwrap();
        observed.borrow_mut().push((
            "Layout",
            catch_unwind(AssertUnwindSafe(|| {
                repeater.set_layout(Some(StackLayout::new().upcast()))
            }))
            .is_err(),
        ));
        // Native styled properties store the proposed value before notifying,
        // just as the pinned dependency-property callback does. Restore the
        // original public value; that callback must also reject reentrancy.
        observed.borrow_mut().push((
            "RestoreLayout",
            catch_unwind(AssertUnwindSafe(|| {
                repeater.set_layout(Some(original.upgrade().unwrap().upcast()))
            }))
            .is_err(),
        ));
    }));
    layout(&repeater);
    assert_eq!(failures.borrow().len(), 5);
    for (name, rejected) in failures.borrow().iter() {
        assert!(rejected, "{name} must reject mutation during layout");
    }
    assert_eq!(repeater.items_source_view().unwrap().count(), 5);
    assert_eq!(repeater.layout(), Some(window.clone().upcast()));
    assert_eq!(window.uninitialized.get(), 0);
    for index in 0..5 {
        let text = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<ferroui_controls::TextBlock>()
            .unwrap();
        assert_eq!(text.text(), Some(format!("Item #{index}")));
    }
}

#[test]
fn can_query_element_factory() {
    struct Source {
        inner: ItemsSource,
        gets: RefCell<Vec<usize>>,
    }
    impl IItemsList for Source {
        fn count(&self) -> usize {
            self.inner.count()
        }
        fn get_at(&self, index: usize) -> Option<BoxedValue> {
            self.gets.borrow_mut().push(index);
            self.inner.get_at(index)
        }
        fn is_notifying(&self) -> bool {
            true
        }
        fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
            self.inner.list().add_collection_changed(handler)
        }
        fn remove_collection_changed(&self, token: u64) {
            self.inner.list().remove_collection_changed(token);
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    struct Factory {
        element: Ref<Control>,
        gets: RefCell<Vec<(i32, Option<ferroui_base::WeakRef<Control>>)>>,
        recycles: RefCell<Vec<(Ref<Control>, Option<ferroui_base::WeakRef<Control>>)>>,
    }
    impl IElementFactory for Factory {
        fn get_element(
            &self,
            args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            self.gets.borrow_mut().push((
                *args.data().unwrap().downcast_ref::<i32>().unwrap(),
                args.parent().as_ref().map(Ref::downgrade),
            ));
            Ok(self.element.clone())
        }
        fn recycle_element(
            &self,
            args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            self.recycles
                .borrow_mut()
                .push((args.element(), args.parent().as_ref().map(Ref::downgrade)));
            Ok(())
        }
    }
    let _scope = test_scope();
    let items = Rc::new(FerroList::<i32>::new());
    let source = Rc::new(Source {
        inner: items.clone().into(),
        gets: RefCell::new(Vec::new()),
    });
    let factory = Rc::new(Factory {
        element: Border::new().upcast(),
        gets: RefCell::new(Vec::new()),
        recycles: RefCell::new(Vec::new()),
    });
    let repeater = ItemsRepeater::new();
    let window = WindowLayout::new(&[]);
    repeater.set_layout(Some(window.clone().upcast()));
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    repeater.set_element_factory(Some(factory.clone()));
    layout(&repeater);
    assert!(source.gets.borrow().is_empty());
    assert!(factory.gets.borrow().is_empty());
    assert!(factory.recycles.borrow().is_empty());
    items.add(1);
    window.set_indices(&[0]);
    layout(&repeater);
    assert_eq!(*source.gets.borrow(), vec![0]);
    assert_eq!(factory.gets.borrow().len(), 1);
    assert_eq!(factory.gets.borrow()[0].0, 1);
    assert_eq!(
        factory.gets.borrow()[0].1.as_ref().unwrap().upgrade(),
        Some(repeater.clone().upcast())
    );
    assert_eq!(repeater.try_get_element(0), Some(factory.element.clone()));
    assert!(factory.recycles.borrow().is_empty());
    source.gets.borrow_mut().clear();
    factory.gets.borrow_mut().clear();
    items.remove_at(0);
    window.set_indices(&[]);
    layout(&repeater);
    assert!(source.gets.borrow().is_empty());
    assert!(factory.gets.borrow().is_empty());
    assert_eq!(factory.recycles.borrow().len(), 1);
    assert_eq!(factory.recycles.borrow()[0].0, factory.element);
    assert_eq!(
        factory.recycles.borrow()[0].1.as_ref().unwrap().upgrade(),
        Some(repeater.upcast())
    );
    // Native ItemsSourceView queries the live IItemsList count. It does not
    // expose WinRT's cached GetSizeCore callback; data reads and factory
    // callback identity/ordering above are the portable observable contract.
}

#[test]
fn collection_mutation_during_measure_is_rejected() {
    let _scope = test_scope();
    let (repeater, window, _) = fixture(0, &[0]);
    let source = Rc::new(FerroList::from_items([1]));
    repeater.set_items_source(Some(source.clone().into()));
    *window.measure_hook.borrow_mut() = Some(Rc::new(move |_| {
        source.add(2);
    }));
    assert!(catch_unwind(AssertUnwindSafe(|| layout(&repeater))).is_err());
}

struct MillionItems(Cell<usize>);
impl IItemsList for MillionItems {
    fn count(&self) -> usize {
        1_000_000
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.0.set(self.0.get() + 1);
        Some(Rc::new(index as i32))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
#[test]
fn stack_virtualization_does_not_enumerate_a_million_item_source() {
    let _scope = test_scope();
    let source = Rc::new(MillionItems(Cell::new(0)));
    let repeater = ItemsRepeater::new();
    repeater.set_element_factory(Some(CountingFactory::new()));
    repeater.set_items_source(Some(ItemsSource::new(source.clone())));
    repeater
        .viewport_manager
        .set_test_viewport(Rect::new(0.0, 0.0, 100.0, 100.0));
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    layout(&repeater);
    assert!(repeater.realized_elements().len() <= 8);
    assert!(source.0.get() <= 10);
    repeater
        .viewport_manager
        .set_test_viewport(Rect::new(0.0, 1_000_000.0, 100.0, 100.0));
    repeater.invalidate_measure();
    layout(&repeater);
    assert!(repeater.realized_elements().len() <= 8);
    assert!(source.0.get() <= 25);
    assert!(repeater
        .realized_elements()
        .iter()
        .all(|e| repeater.get_element_index(e) > 49_990));
}

#[test]
fn effective_viewport_without_scroller_remains_finite_without_cache() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater
        .viewport_manager
        .on_effective_viewport_changed(&repeater, Rect::new(0.0, 25.0, 80.0, 60.0));
    assert_eq!(repeater.visible_window(), Rect::new(0.0, 25.0, 80.0, 60.0));
    assert_eq!(repeater.realization_window(), repeater.visible_window());
    repeater.viewport_manager.on_owner_arranged(&repeater);
    assert_eq!(repeater.realization_window(), repeater.visible_window());
}

#[test]
fn cache_grows_by_forty_per_side_and_resets_after_distant_scroll() {
    let _scope = test_scope();
    let (repeater, _, _) = fixture(0, &[]);
    repeater
        .viewport_manager
        .set_test_viewport(Rect::new(0.0, 0.0, 100.0, 100.0));
    layout(&repeater);
    assert_eq!(
        repeater.realization_window(),
        Rect::new(-40.0, -40.0, 180.0, 180.0)
    );
    Dispatcher::ui_thread().run_jobs(None);
    layout(&repeater);
    assert_eq!(
        repeater.realization_window(),
        Rect::new(-80.0, -80.0, 260.0, 260.0)
    );
    repeater
        .viewport_manager
        .set_test_viewport(Rect::new(0.0, 1000.0, 100.0, 100.0));
    repeater.invalidate_measure();
    repeater.measure(Size::new(100.0, 100.0));
    assert_eq!(
        repeater.realization_window(),
        Rect::new(0.0, 1000.0, 100.0, 100.0)
    );
}

#[test]
fn tab_order_follows_source_indices_after_out_of_order_realization() {
    use ferroui_base::input::{
        KeyboardNavigation, KeyboardNavigationHandler, KeyboardNavigationMode,
    };
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    KeyboardNavigation::set_tab_navigation(&repeater, KeyboardNavigationMode::Local);
    let buttons: Vec<_> = (0..3).map(|_| Button::new()).collect();
    repeater.set_items_source(Some(ItemsSource::from_items(
        buttons.iter().map(|b| Some(Control::boxed(b.clone()))),
    )));
    let window = WindowLayout::new(&[2, 0, 1]);
    repeater.set_layout(Some(window.upcast()));
    let _root = TestRoot::with_child(repeater.clone());
    layout(&repeater);
    let first: Ref<InputElement> = buttons[0].clone().upcast();
    assert_eq!(
        KeyboardNavigationHandler::get_next(&first, NavigationDirection::Next),
        Some(buttons[1].clone().upcast())
    );
}

#[test]
fn layout_panel_insets_the_layout_and_adapts_fixed_children() {
    let _scope = test_scope();
    let panel = LayoutPanel::new();
    panel.set_use_layout_rounding(false);
    panel.set_padding(Thickness::uniform(3.0));
    panel.set_border_thickness(Thickness::uniform(2.0));
    let stack = StackLayout::new();
    stack.set_spacing(4.0);
    panel.set_layout(Some(stack.upcast()));
    let first = Border::new();
    first.set_width(30.0);
    first.set_height(20.0);
    let second = Border::new();
    second.set_width(40.0);
    second.set_height(10.0);
    panel.children().add(&first);
    panel.children().add(&second);
    panel.measure(Size::new(100.0, 100.0));
    assert_eq!(panel.desired_size(), Size::new(50.0, 44.0));
    panel.arrange(Rect::new(0.0, 0.0, 50.0, 44.0));
    assert_eq!(first.previous_arrange().unwrap().y, 5.0);
    assert_eq!(second.previous_arrange().unwrap().y, 29.0);
    assert_eq!(first.previous_arrange().unwrap().x, 5.0);
    assert_eq!(panel.children().count(), 2);
}
