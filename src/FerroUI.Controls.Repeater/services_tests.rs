// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.
// Tests named validate_* and verify_* adapt the corresponding WinUI Repeater
// API tests. Other tests exercise Rust ownership and native-framework integration.

use super::build_tree_scheduler::BuildTreeScheduler;
use super::element_factory::{
    ElementFactory, ElementFactoryError, ElementFactoryGetArgs, ElementFactoryRecycleArgs,
    IElementFactory,
};
use super::item_collection_transition::{
    ItemCollectionTransition, ItemCollectionTransitionOperation as Operation,
    ItemCollectionTransitionTriggers as Triggers,
};
use super::item_collection_transition_completed_event_args::ItemCollectionTransitionCompletedEventArgs;
use super::item_collection_transition_provider::{
    ItemCollectionTransitionProvider, ItemCollectionTransitionProviderImpl,
};
use super::item_template_wrapper::{
    factory_from_template, IDataTemplateSelector, ItemTemplateWrapper,
};
use super::items_repeater_scroll_host::ItemsRepeaterScrollHost;
use super::lined_flow_layout_item_collection_transition_provider::LinedFlowLayoutItemCollectionTransitionProvider;
use super::phaser::{IDataTemplateComponent, Phaser};
use super::recycle_pool::RecyclePool;
use super::recycle_pool::{RecyclePoolImpl, RecyclePoolImplExt};
use super::recycling_element_factory::RecyclingElementFactory;
use super::repeater_test_hooks::RepeaterTestHooks;
use super::transition_manager::TransitionManager;
use super::virtualization_info::{ElementOwner, VirtualizationInfo};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::{Border, Button, Control, Panel, ScrollViewer, StackPanel, TextBlock};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::BindingPriority;
use ferroui_base::media::{ITransform, MatrixTransform};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroLocator, FerroObjectImpl, Matrix, Rect, Ref, Size,
    Vector, Visual,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(super) struct TestClock {
    time: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl TestClock {
    pub(super) fn install() -> Rc<Self> {
        let clock = Rc::new(Self {
            time: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        });
        FerroLocator::current_mutable()
            .bind::<dyn IGlobalClock>()
            .to_constant(clock.clone() as Rc<dyn IGlobalClock>);
        clock
    }
    pub(super) fn pulse(&self, milliseconds: f64) {
        self.time.on_next(TimeSpan::from_milliseconds(milliseconds));
        Dispatcher::ui_thread().run_jobs(None);
    }
}
impl IObservable<TimeSpan> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.time.subscribe(observer)
    }
}
impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
impl IGlobalClock for TestClock {}

#[repr(C)]
struct TestTransitionProvider {
    base: ItemCollectionTransitionProvider,
    animate: Cell<bool>,
    only_add: Cell<bool>,
    should_calls: Cell<usize>,
    started: RefCell<Vec<Rc<ItemCollectionTransition>>>,
    start: RefCell<Option<Rc<dyn Fn(Vec<Rc<ItemCollectionTransition>>)>>>,
}
ferro_class!(TestTransitionProvider: ItemCollectionTransitionProvider);
impl FerroObjectImpl for TestTransitionProvider {}
impl ItemCollectionTransitionProviderImpl for TestTransitionProvider {
    fn should_animate_core(this: &Self, transition: Rc<ItemCollectionTransition>) -> bool {
        this.should_calls.set(this.should_calls.get() + 1);
        this.animate.get() && (!this.only_add.get() || transition.operation() == Operation::Add)
    }
    fn start_transitions_core(this: &Self, transitions: Vec<Rc<ItemCollectionTransition>>) {
        this.started
            .borrow_mut()
            .extend(transitions.iter().cloned());
        let start = this.start.borrow().clone();
        if let Some(start) = start {
            start(transitions);
        }
    }
}
impl TestTransitionProvider {
    fn new(animate: bool) -> Ref<Self> {
        instantiate(Self {
            base: ItemCollectionTransitionProvider::construct(),
            animate: Cell::new(animate),
            only_add: Cell::new(false),
            should_calls: Cell::new(0),
            started: RefCell::new(Vec::new()),
            start: RefCell::new(None),
        })
    }
}

fn template(recycle: bool) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, |_, _| Some(Border::new().upcast()), recycle)
}
fn element() -> Ref<Control> {
    let element = Border::new();
    element.set_width(80.0);
    element.set_height(20.0);
    element.measure(Size::new(80.0, 20.0));
    element.arrange(Rect::new(0.0, 0.0, 80.0, 20.0));
    element.upcast()
}
fn add(
    provider: &Ref<TestTransitionProvider>,
    element: Ref<Control>,
) -> Rc<ItemCollectionTransition> {
    ItemCollectionTransition::new(
        provider.clone().upcast(),
        element,
        Operation::Add,
        Triggers::COLLECTION_CHANGE_ADD,
    )
}

#[test]
fn validate_elements_have_correct_keys() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    for _ in 0..3 {
        pool.put_element(Button::new().upcast(), "ButtonKey")
            .unwrap();
        pool.put_element(TextBlock::new().upcast(), "TextBlockKey")
            .unwrap();
        pool.put_element(StackPanel::new().upcast(), "StackPanelKey")
            .unwrap();
    }
    for _ in 0..3 {
        assert!(pool
            .try_get_element("ButtonKey")
            .unwrap()
            .unwrap()
            .is::<Button>());
        assert!(pool
            .try_get_element("TextBlockKey")
            .unwrap()
            .unwrap()
            .is::<TextBlock>());
        assert!(pool
            .try_get_element("StackPanelKey")
            .unwrap()
            .unwrap()
            .is::<StackPanel>());
    }
    assert!(pool.try_get_element("ButtonKey").unwrap().is_none());
    assert!(pool.try_get_element("TextBlockKey").unwrap().is_none());
    assert!(pool.try_get_element("StackPanelKey").unwrap().is_none());
    let empty_key = Control::new();
    pool.put_element(empty_key.clone(), "").unwrap();
    assert!(pool.try_get_element("ButtonKey").unwrap().is_none());
    assert_eq!(pool.try_get_element("").unwrap(), Some(empty_key));
    assert!(pool.try_get_element("").unwrap().is_none());
    assert_eq!(
        pool.put_element_with_owner(Control::new(), "key", Some(Control::new())),
        Err(ElementFactoryError::InvalidOwner)
    );
}

#[test]
fn validate_ownership_with_stack_panel() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    let owner = StackPanel::new();
    let child = Control::new();
    owner.children().add(child.clone());
    pool.put_element_with_owner(child.clone(), "key", Some(owner.clone().upcast()))
        .unwrap();
    assert_eq!(
        pool.try_get_element_with_owner("key", Some(owner.clone().upcast()))
            .unwrap(),
        Some(child.clone())
    );
    assert_eq!(owner.children().get(0), child);
}

#[test]
fn validate_recycled_element_owner_affinity() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    let one = Panel::new();
    let two = Panel::new();
    let first = Control::new();
    let second = Control::new();
    one.children().add(first.clone());
    two.children().add(second.clone());
    pool.put_element_with_owner(first.clone(), "key", Some(one.clone().upcast()))
        .unwrap();
    pool.put_element_with_owner(second.clone(), "key", Some(two.clone().upcast()))
        .unwrap();
    assert_eq!(
        pool.try_get_element_with_owner("key", Some(one.clone().upcast()))
            .unwrap(),
        Some(first)
    );
    assert_eq!(
        pool.try_get_element_with_owner("key", Some(two.clone().upcast()))
            .unwrap(),
        Some(second)
    );
    assert_eq!(one.children().count(), 1);
    assert_eq!(two.children().count(), 1);
}

#[test]
fn validate_child_removed_from_parent_when_owner_is_different() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    let one = Panel::new();
    let two = Panel::new();
    let child = Control::new();
    one.children().add(child.clone());
    pool.put_element_with_owner(child.clone(), "key", Some(one.clone().upcast()))
        .unwrap();
    assert_eq!(
        pool.try_get_element_with_owner("key", Some(two.clone().upcast()))
            .unwrap(),
        Some(child.clone())
    );
    assert!(child.visual_parent().is_none());
    assert_eq!(one.children().count(), 0);
}

#[test]
fn pool_uses_unowned_candidates_before_cross_owner_detachment() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    let owner = Panel::new();
    let requested = Panel::new();
    let owned = Control::new();
    let free = Control::new();
    owner.children().add(owned.clone());
    pool.put_element_with_owner(owned, "key", Some(owner.clone().upcast()))
        .unwrap();
    pool.put_element(free.clone(), "key").unwrap();
    assert_eq!(
        pool.try_get_element_with_owner("key", Some(requested.upcast()))
            .unwrap(),
        Some(free)
    );
    assert_eq!(owner.children().count(), 1);
}

#[test]
fn pool_retains_elements_but_does_not_retain_the_owner() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    let owner = Panel::new();
    let owner_weak = owner.downgrade();
    let child = Control::new();
    owner.children().add(child.clone());
    pool.put_element_with_owner(child.clone(), "key", Some(owner.clone().upcast()))
        .unwrap();
    drop(owner);
    assert!(owner_weak.upgrade().is_none());
    assert_eq!(pool.try_get_element("key").unwrap(), Some(child));
}

#[test]
fn pool_rejects_inconsistent_owner_children() {
    let _scope = test_scope();
    let pool = RecyclePool::new();
    let owner = Panel::new();
    pool.put_element_with_owner(Control::new(), "key", Some(owner.clone().upcast()))
        .unwrap();
    assert_eq!(
        pool.try_get_element("key"),
        Err(ElementFactoryError::MissingChild)
    );
}

#[test]
fn recycling_factory_single_template_uses_key_without_selection() {
    let _scope = test_scope();
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().set("one".into(), template(true));
    factory.select_template_key(|_| panic!("A one-template factory must not request selection."));
    let args = ElementFactoryGetArgs::default();
    let first = factory.get_element(&args).unwrap();
    assert_eq!(RecyclePool::get_reuse_key(&first), "one");
    factory
        .recycle_element(&ElementFactoryRecycleArgs::new(first.clone(), None))
        .unwrap();
    assert_eq!(factory.get_element(&args).unwrap(), first);
}

#[test]
fn recycling_factory_selects_template_with_data_and_owner() {
    let _scope = test_scope();
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().set("odd".into(), template(true));
    factory.templates().set("even".into(), template(true));
    let parent = Panel::new();
    let expected = parent.clone();
    factory.select_template_key(move |args| {
        assert_eq!(args.owner(), Some(expected.clone().upcast()));
        args.set_template_key(
            if *args.data_context().unwrap().downcast_ref::<i32>().unwrap() % 2 == 0 {
                "even"
            } else {
                "odd"
            },
        );
    });
    let element = factory
        .get_element(&ElementFactoryGetArgs::new(
            Some(Rc::new(3i32)),
            Some(parent.upcast()),
        ))
        .unwrap();
    assert_eq!(RecyclePool::get_reuse_key(&element), "odd");
}

#[test]
fn recycling_factory_validates_templates_keys_and_pool() {
    let _scope = test_scope();
    let factory = RecyclingElementFactory::new();
    let args = ElementFactoryGetArgs::default();
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::MissingTemplates)
    );
    factory.templates().set("".into(), template(true));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::EmptyTemplateKey)
    );
    factory.templates().clear();
    factory.templates().set("one".into(), template(true));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::MissingRecyclePool)
    );
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().set("two".into(), template(true));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::EmptyTemplateKey)
    );
    factory.select_template_key(|args| args.set_template_key("unknown"));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::UnknownTemplateKey("unknown".into()))
    );
}

#[test]
fn wrapper_shares_template_pool_and_reuses_native_recycling_roots() {
    let _scope = test_scope();
    let template = template(true);
    let pool = RecyclePool::new();
    RecyclePool::set_pool_instance(&template, Some(pool.clone()));
    let one = ItemTemplateWrapper::new(template.clone());
    let two = ItemTemplateWrapper::new(template.clone());
    let child = one
        .get_element(&ElementFactoryGetArgs::new(Some(Rc::new(1i32)), None))
        .unwrap();
    one.recycle_element(&ElementFactoryRecycleArgs::new(child.clone(), None))
        .unwrap();
    assert_eq!(pool.count(), 1);
    assert_eq!(RecyclePool::get_pool_instance(&template), Some(pool));
    assert_eq!(
        two.get_element(&ElementFactoryGetArgs::new(Some(Rc::new(2i32)), None))
            .unwrap(),
        child
    );
}

#[test]
fn native_template_that_replaces_a_root_detaches_the_old_root() {
    let _scope = test_scope();
    let wrapper = ItemTemplateWrapper::new(template(false));
    let owner = Panel::new();
    let args = ElementFactoryGetArgs::new(None, Some(owner.clone().upcast()));
    let first = wrapper.get_element(&args).unwrap();
    owner.children().add(first.clone());
    wrapper
        .recycle_element(&ElementFactoryRecycleArgs::new(
            first.clone(),
            Some(owner.clone().upcast()),
        ))
        .unwrap();
    let second = wrapper.get_element(&args).unwrap();
    assert_ne!(first, second);
    assert!(first.visual_parent().is_none());
    assert!(owner.children().is_empty());
}

struct FallbackSelector {
    template: Rc<dyn IDataTemplate>,
    calls: Cell<usize>,
}
impl IDataTemplateSelector for FallbackSelector {
    fn select_template(&self, _item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.calls.set(self.calls.get() + 1);
        None
    }
    fn select_template_with_container(
        &self,
        _item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        self.calls.set(self.calls.get() + 1);
        assert!(container.is_none());
        Some(self.template.clone())
    }
}

fn template_with_release_callback(callback: impl Fn() + 'static) -> Rc<dyn IDataTemplate> {
    struct ReleaseCallback(Box<dyn Fn()>);
    impl Drop for ReleaseCallback {
        fn drop(&mut self) {
            (self.0)();
        }
    }
    let release = ReleaseCallback(Box::new(callback));
    FuncDataTemplate::new(
        move |_| {
            let _ = &release;
            true
        },
        |_, _| Some(Border::new().upcast()),
        true,
    )
}

#[test]
fn template_replacement_releases_old_template_after_publishing_the_new_value() {
    let _scope = test_scope();
    let wrapper = ItemTemplateWrapper::new(template(true));
    let replacement = template(true);
    let expected = replacement.clone();
    let weak = Rc::downgrade(&wrapper);
    let released = Rc::new(Cell::new(false));
    let observed = released.clone();
    wrapper.set_template(Some(template_with_release_callback(move || {
        let wrapper = weak.upgrade().unwrap();
        assert!(Rc::ptr_eq(&wrapper.template().unwrap(), &expected));
        wrapper.set_template(None);
        observed.set(true);
    })));

    wrapper.set_template(Some(replacement));
    assert!(released.get());
    assert!(wrapper.template().is_none());
}

#[test]
fn selector_replacement_releases_old_selector_after_publishing_the_new_value() {
    let _scope = test_scope();
    let wrapper = ItemTemplateWrapper::new(template(true));
    wrapper.set_template(None);
    let replacement: Rc<dyn IDataTemplateSelector> = Rc::new(FallbackSelector {
        template: template(true),
        calls: Cell::new(0),
    });
    let expected = replacement.clone();
    let weak = Rc::downgrade(&wrapper);
    let released = Rc::new(Cell::new(false));
    let observed = released.clone();
    wrapper.set_template_selector(Some(Rc::new(FallbackSelector {
        template: template_with_release_callback(move || {
            let wrapper = weak.upgrade().unwrap();
            assert!(Rc::ptr_eq(&wrapper.template_selector().unwrap(), &expected));
            wrapper.set_template_selector(None);
            observed.set(true);
        }),
        calls: Cell::new(0),
    })));

    wrapper.set_template_selector(Some(replacement));
    assert!(released.get());
    assert!(wrapper.template_selector().is_none());
}

#[test]
fn selector_retries_container_overload_and_empty_content_has_zero_size() {
    let _scope = test_scope();
    let selector = Rc::new(FallbackSelector {
        template: FuncDataTemplate::new(|_| true, |_, _| None, true),
        calls: Cell::new(0),
    });
    let wrapper = ItemTemplateWrapper::with_selector(selector.clone());
    let element = wrapper
        .get_element(&ElementFactoryGetArgs::default())
        .unwrap();
    assert_eq!(selector.calls.get(), 2);
    element.measure(Size::new(100.0, 100.0));
    assert_eq!(element.desired_size(), Size::default());
    wrapper
        .recycle_element(&ElementFactoryRecycleArgs::new(element, None))
        .unwrap();
}

struct EphemeralSelector {
    last_template: RefCell<Option<std::rc::Weak<dyn IDataTemplate>>>,
}
impl IDataTemplateSelector for EphemeralSelector {
    fn select_template(&self, _item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        let template = template(true);
        *self.last_template.borrow_mut() = Some(Rc::downgrade(&template));
        Some(template)
    }
    fn select_template_with_container(
        &self,
        _item: Option<&BoxedValue>,
        _container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        panic!("A successful first selection must not invoke the second overload.")
    }
}

#[test]
fn selected_template_lives_with_the_realized_element_without_pool_cycles() {
    let _scope = test_scope();
    let selector = Rc::new(EphemeralSelector {
        last_template: RefCell::new(None),
    });
    let wrapper = ItemTemplateWrapper::with_selector(selector.clone());
    let element = wrapper
        .get_element(&ElementFactoryGetArgs::default())
        .unwrap();
    let weak_element = element.downgrade();
    let weak_template = selector.last_template.borrow().clone().unwrap();
    assert!(weak_template.upgrade().is_some());
    assert!(RecyclePool::get_origin_template(&element).is_some());
    wrapper
        .recycle_element(&ElementFactoryRecycleArgs::new(element, None))
        .unwrap();
    assert!(weak_template.upgrade().is_none());
    RecyclePool::prune_template_pools();
    assert!(weak_element.upgrade().is_none());
}

#[test]
fn direct_empty_template_is_detected_without_probing_incompatible_null_data() {
    let _scope = test_scope();
    let empty = ItemTemplateWrapper::new(FuncDataTemplate::new(|_| true, |_, _| None, true));
    assert!(empty.is_template_empty(None));
    let typed = ItemTemplateWrapper::new(FuncDataTemplate::for_type::<i32>(
        |_, _| Some(Control::new()),
        true,
    ));
    assert!(!typed.is_template_empty(None));
    assert!(!typed.is_template_empty(Some(&(Rc::new(1i32) as BoxedValue))));
}

#[test]
fn untyped_factory_conversion_preserves_object_and_template_identity() {
    let _scope = test_scope();
    let factory = RecyclingElementFactory::new();
    let boxed: BoxedValue = Rc::new(factory.clone());
    let adapted = factory_from_template(Some(&boxed)).unwrap().unwrap();
    assert_eq!(adapted.as_object(), Some(factory.upcast()));
    let boxed: BoxedValue = Rc::new(template(true));
    assert!(factory_from_template(Some(&boxed)).unwrap().is_some());
    let invalid: BoxedValue = Rc::new(42i32);
    assert!(matches!(
        factory_from_template(Some(&invalid)),
        Err(ElementFactoryError::InvalidTemplate)
    ));
    assert!(factory_from_template(None).unwrap().is_none());
    assert!(matches!(
        ElementFactory::new().get_element(&ElementFactoryGetArgs::default()),
        Err(ElementFactoryError::AbstractMethod("GetElementCore"))
    ));
}

#[test]
fn factory_args_have_shared_mutations_and_weak_parent_references() {
    let _scope = test_scope();
    let owner = Panel::new();
    let weak = owner.downgrade();
    let args = ElementFactoryGetArgs::new(None, Some(owner.clone().upcast()));
    let same = args.clone();
    same.set_data(Some(Rc::new(4i32)));
    assert_eq!(args.data().unwrap().downcast_ref::<i32>(), Some(&4));
    drop(owner);
    assert!(weak.upgrade().is_none());
    assert!(args.parent().is_none());
}

#[test]
fn virtualization_ownership_state_machine_preserves_pins_and_reset_keys() {
    let info = VirtualizationInfo::new();
    assert_eq!(info.owner(), ElementOwner::ElementFactory);
    assert_eq!(info.index(), -1);
    info.move_ownership_to_layout_from_element_factory(2, "item-two");
    assert!(info.is_realized());
    assert_eq!(info.add_pin(), 1);
    assert_eq!(info.add_pin(), 2);
    info.move_ownership_to_pinned_pool();
    assert!(info.is_realized());
    info.move_ownership_to_layout_from_pinned_pool();
    info.move_ownership_to_unique_id_reset_pool_from_layout();
    assert!(!info.is_realized());
    assert_eq!(info.pin_count(), 2);
    info.move_ownership_to_layout_from_unique_id_reset_pool();
    info.update_index(7);
    assert_eq!(info.unique_id(), "item-two");
    assert_eq!(info.remove_pin(), 1);
    assert_eq!(info.remove_pin(), 0);
    info.move_ownership_to_element_factory();
    assert_eq!(info.index(), -1);
    assert!(info.unique_id().is_empty());
    assert_eq!(info.arrange_bounds(), VirtualizationInfo::INVALID_RECT);
}

#[test]
fn virtualization_animator_ownership_clears_index_and_pins_from_reset_pool() {
    let info = VirtualizationInfo::new();
    info.move_ownership_to_layout_from_element_factory(3, "three");
    info.add_pin();
    info.move_ownership_to_unique_id_reset_pool_from_layout();
    info.move_ownership_to_animator();
    assert_eq!(info.owner(), ElementOwner::Animator);
    assert_eq!(info.index(), -1);
    assert_eq!(info.pin_count(), 0);
    assert!(!info.is_realized());
    info.move_ownership_to_element_factory();
}

#[test]
#[should_panic(expected = "cannot pin an unrealized")]
fn unrealized_elements_cannot_be_pinned() {
    VirtualizationInfo::new().add_pin();
}

#[test]
#[should_panic(expected = "more often than PinElement")]
fn unbalanced_unpin_is_rejected() {
    let info = VirtualizationInfo::new();
    info.move_ownership_to_layout_from_element_factory(0, "");
    info.remove_pin();
}

struct PhasedComponent {
    id: i32,
    log: Rc<RefCell<Vec<(i32, i32, i32)>>>,
    recycled: Cell<usize>,
    callback: RefCell<Option<Rc<dyn Fn(i32)>>>,
}
impl IDataTemplateComponent for PhasedComponent {
    fn process_bindings(&self, data: Option<BoxedValue>, index: i32, phase: i32) -> i32 {
        assert!(data.is_some());
        self.log.borrow_mut().push((self.id, phase, index));
        let callback = self.callback.borrow().clone();
        if let Some(callback) = callback {
            callback(phase);
        }
        if phase < 2 {
            phase + 1
        } else {
            -1
        }
    }
    fn recycle(&self) {
        self.recycled.set(self.recycled.get() + 1);
    }
}
fn phase_item(
    id: i32,
    log: &Rc<RefCell<Vec<(i32, i32, i32)>>>,
    bounds: Rect,
) -> (Ref<Control>, Rc<VirtualizationInfo>, Rc<PhasedComponent>) {
    let item = element();
    let info = VirtualizationInfo::new();
    info.move_ownership_to_layout_from_element_factory(id, "");
    info.set_arrange_bounds(bounds);
    let component = Rc::new(PhasedComponent {
        id,
        log: log.clone(),
        recycled: Cell::new(0),
        callback: RefCell::new(None),
    });
    Phaser::set_data_template_component(&item, Some(component.clone()));
    (item, info, component)
}
fn phaser() -> Rc<Phaser> {
    let items = Rc::new(vec![
        Rc::new(0i32) as BoxedValue,
        Rc::new(1i32) as BoxedValue,
        Rc::new(2i32) as BoxedValue,
    ]);
    Phaser::new(
        Rc::new(|| Rect::new(0.0, 0.0, 100.0, 100.0)),
        Rc::new(move |index| items.get(index as usize).cloned()),
    )
}

#[test]
fn later_phases_receive_the_item_of_a_source_that_boxes_on_every_read() {
    // A typed FerroUI list (`FerroList<T>`, `ObservableCollection<T>`) boxes a new value on
    // every `get_at`; nothing but the phasing state keeps that box alive.
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = Phaser::new(
        Rc::new(|| Rect::new(0.0, 0.0, 100.0, 100.0)),
        Rc::new(|index| Some(Rc::new(index) as BoxedValue)),
    );
    let log = Rc::new(RefCell::new(Vec::new()));
    let (item, info, _component) = phase_item(1, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    phaser.phase_element(&item, &info);
    clock.pulse(0.0);
    // `PhasedComponent::process_bindings` asserts that every phase has its data.
    assert_eq!(*log.borrow(), vec![(1, 0, 1), (1, 1, -1), (1, 2, -1)]);
    phaser.stop_phasing(&item, &info);
    assert!(info.data().is_none());
}

#[test]
fn unphased_elements_do_not_read_the_items_source_again() {
    let _scope = test_scope();
    let phaser = Phaser::new(
        Rc::new(Rect::default),
        Rc::new(|_| panic!("A template without a binding component must not read item data.")),
    );
    let item = element();
    let info = VirtualizationInfo::new();
    info.move_ownership_to_layout_from_element_factory(0, "");
    phaser.phase_element(&item, &info);
    assert_eq!(phaser.pending_count(), 0);
    assert_eq!(info.phase(), VirtualizationInfo::PHASE_NOT_SPECIFIED);
}

#[test]
fn phasing_initializes_synchronously_and_prioritizes_visible_elements() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (cached, cached_info, cached_component) =
        phase_item(0, &log, Rect::new(0.0, 200.0, 80.0, 20.0));
    let (visible, visible_info, visible_component) =
        phase_item(1, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    phaser.phase_element(&cached, &cached_info);
    phaser.phase_element(&visible, &visible_info);
    assert_eq!(*log.borrow(), vec![(0, 0, 0), (1, 0, 1)]);
    assert_eq!(phaser.pending_count(), 2);
    log.borrow_mut().clear();
    clock.pulse(0.0);
    assert_eq!(
        *log.borrow(),
        vec![(1, 1, -1), (1, 2, -1), (0, 1, -1), (0, 2, -1)]
    );
    assert_eq!(phaser.pending_count(), 0);
    assert_eq!(visible_info.phase(), 2);
    assert_eq!(cached_component.recycled.get(), 1);
    assert_eq!(visible_component.recycled.get(), 1);
}

#[test]
fn phasing_orders_visible_items_by_phase_and_realization_order() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (first, info1, _) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    let (second, info2, _) = phase_item(1, &log, Rect::new(0.0, 20.0, 80.0, 20.0));
    phaser.phase_element(&first, &info1);
    phaser.phase_element(&second, &info2);
    log.borrow_mut().clear();
    clock.pulse(0.0);
    assert_eq!(
        *log.borrow(),
        vec![(0, 1, -1), (1, 1, -1), (0, 2, -1), (1, 2, -1)]
    );
}

#[test]
fn staggered_phases_preserve_source_sort_and_tail_traversal() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (first, info1, _) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    let (second, info2, _) = phase_item(1, &log, Rect::new(0.0, 20.0, 80.0, 20.0));
    phaser.phase_element(&first, &info1);
    phaser.phase_element(&second, &info2);
    info2.set_phase(2);
    log.borrow_mut().clear();
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(1, 2, -1), (0, 1, -1), (0, 2, -1)]);
    assert_eq!(phaser.pending_count(), 0);
}

#[test]
fn phase_zero_initialization_does_not_enqueue_until_preparation_finishes() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (item, info, _) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    let data: BoxedValue = Rc::new(0i32);
    Phaser::initialize_phase_zero(&item, &info, Some(data.clone()), 0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0)]);
    assert_eq!(phaser.pending_count(), 0);
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0)]);
    phaser.phase_element(&item, &info);
    clock.pulse(16.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0), (0, 1, -1), (0, 2, -1)]);
}

#[test]
fn animator_owned_elements_finish_pending_binding_phases_before_final_recycling() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (item, info, _) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    phaser.phase_element(&item, &info);
    info.move_ownership_to_animator();
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0), (0, 1, -1), (0, 2, -1)]);
    assert_eq!(phaser.pending_count(), 0);
    phaser.stop_phasing(&item, &info);
    info.move_ownership_to_element_factory();
}

#[test]
fn stop_phasing_cancels_queued_work_before_recycling() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (item, info, _) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    phaser.phase_element(&item, &info);
    phaser.stop_phasing(&item, &info);
    info.move_ownership_to_element_factory();
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0)]);
    assert_eq!(info.phase(), VirtualizationInfo::PHASE_NOT_SPECIFIED);
    assert_eq!(phaser.pending_count(), 0);
}

#[test]
fn phasing_callback_can_recycle_its_own_element_without_rescheduling_it() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (item, info, component) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    let weak = Rc::downgrade(&phaser);
    let weak_item = item.downgrade();
    let callback_info = info.clone();
    *component.callback.borrow_mut() = Some(Rc::new(move |phase| {
        if phase == 1 {
            weak.upgrade()
                .unwrap()
                .stop_phasing(&weak_item.upgrade().unwrap(), &callback_info);
            callback_info.move_ownership_to_element_factory();
        }
    }));
    phaser.phase_element(&item, &info);
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0), (0, 1, -1)]);
    assert_eq!(phaser.pending_count(), 0);
}

#[test]
fn phase_zero_callback_cancellation_does_not_restore_stale_binding_state() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (item, info, component) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    let weak = Rc::downgrade(&phaser);
    let weak_item = item.downgrade();
    let callback_info = info.clone();
    *component.callback.borrow_mut() = Some(Rc::new(move |phase| {
        assert_eq!(phase, 0);
        weak.upgrade()
            .unwrap()
            .stop_phasing(&weak_item.upgrade().unwrap(), &callback_info);
        callback_info.move_ownership_to_element_factory();
    }));
    phaser.phase_element(&item, &info);
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0)]);
    assert_eq!(info.phase(), VirtualizationInfo::PHASE_NOT_SPECIFIED);
    assert!(info.data_template_component().is_none());
    assert_eq!(phaser.pending_count(), 0);
}

#[test]
#[should_panic(expected = "monotonically increasing")]
fn repeated_or_decreasing_phases_are_rejected() {
    Phaser::validate_phase_ordering(2, 2);
}

#[test]
fn scheduler_orders_work_and_defers_work_registered_during_a_frame() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let log = Rc::new(RefCell::new(Vec::new()));
    let a = log.clone();
    BuildTreeScheduler::register_work(2, move || {
        a.borrow_mut().push(2);
        let next = a.clone();
        BuildTreeScheduler::register_work(0, move || next.borrow_mut().push(0));
    });
    let b = log.clone();
    BuildTreeScheduler::register_work(1, move || b.borrow_mut().push(1));
    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![1, 2]);
    clock.pulse(16.0);
    assert_eq!(*log.borrow(), vec![1, 2, 0]);
    assert_eq!(BuildTreeScheduler::pending_work_count(), 0);
    assert_eq!(BuildTreeScheduler::budget_milliseconds(), 40.0);
}

#[test]
fn scheduler_cancels_and_releases_work_already_selected_for_a_frame() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let log = Rc::new(RefCell::new(Vec::new()));
    let resource = Rc::new(());
    let released = Rc::downgrade(&resource);
    let cancelled = BuildTreeScheduler::register_work(1, move || {
        drop(resource);
        panic!("Work cancelled during this frame must not run.");
    });
    let first = log.clone();
    BuildTreeScheduler::register_work(0, move || {
        cancelled.dispose();
        cancelled.dispose();
        assert!(released.upgrade().is_none());
        first.borrow_mut().push(0);
    });
    let last = log.clone();
    BuildTreeScheduler::register_work(2, move || last.borrow_mut().push(2));

    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![0, 2]);
    assert_eq!(BuildTreeScheduler::pending_work_count(), 0);
}

#[test]
fn recycling_during_another_callback_defers_replacement_phases_until_the_next_frame() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let phaser = phaser();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (original, original_info, _) = phase_item(0, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    let (replacement, replacement_info, _) =
        phase_item(1, &log, Rect::new(0.0, 0.0, 80.0, 20.0));
    phaser.phase_element(&original, &original_info);
    let recycle = phaser.clone();
    BuildTreeScheduler::register_work(0, move || {
        recycle.stop_phasing(&original, &original_info);
        original_info.move_ownership_to_element_factory();
        recycle.phase_element(&replacement, &replacement_info);
    });

    clock.pulse(0.0);
    assert_eq!(*log.borrow(), vec![(0, 0, 0), (1, 0, 1)]);
    assert_eq!(phaser.pending_count(), 1);
    assert_eq!(BuildTreeScheduler::pending_work_count(), 1);
    clock.pulse(16.0);
    assert_eq!(
        *log.borrow(),
        vec![(0, 0, 0), (1, 0, 1), (1, 1, -1), (1, 2, -1)]
    );
    assert_eq!(phaser.pending_count(), 0);
    assert_eq!(BuildTreeScheduler::pending_work_count(), 0);
}

#[test]
fn build_tree_completed_fires_only_after_all_deferred_frames_are_drained() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let completed = Rc::new(Cell::new(0));
    let count = completed.clone();
    let subscription = RepeaterTestHooks::build_tree_completed(move || count.set(count.get() + 1));
    BuildTreeScheduler::register_work(1, || {
        BuildTreeScheduler::register_work(2, || {});
    });
    assert_eq!(completed.get(), 0);
    clock.pulse(0.0);
    assert_eq!(completed.get(), 0);
    clock.pulse(16.0);
    assert_eq!(completed.get(), 1);
    subscription.dispose();
    BuildTreeScheduler::register_work(1, || {});
    clock.pulse(32.0);
    assert_eq!(completed.get(), 1);
}

#[test]
fn repeater_diagnostics_preserve_layout_and_element_identifiers() {
    let _scope = test_scope();
    let layout = super::layouts::StackLayout::new();
    let object = layout.upcast();
    RepeaterTestHooks::set_layout_id(&object, "rows");
    assert_eq!(RepeaterTestHooks::get_layout_id(&object), "rows");
    RepeaterTestHooks::set_element_factory_element_index(27);
    assert_eq!(RepeaterTestHooks::get_element_factory_element_index(), 27);
    RepeaterTestHooks::set_log_item_index(11);
    assert_eq!(RepeaterTestHooks::get_log_item_index(), 11);
}

#[test]
fn verify_transition_lifecycle_without_animation() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(false);
    let item = element();
    let completed = Rc::new(RefCell::new(Vec::<
        ItemCollectionTransitionCompletedEventArgs,
    >::new()));
    let collect = completed.clone();
    let subscription =
        provider.transition_completed(move |args| collect.borrow_mut().push(args.clone()));
    let transition = add(&provider, item.clone());
    provider.queue_transition(transition.clone());
    assert!(completed.borrow().is_empty());
    clock.pulse(0.0);
    assert_eq!(completed.borrow().len(), 1);
    assert!(!transition.has_started());
    assert_eq!(completed.borrow()[0].element(), item);
    let progress = transition.start();
    assert!(transition.has_started());
    assert!(Rc::ptr_eq(&progress, &transition.start()));
    assert!(Rc::ptr_eq(&progress.transition().unwrap(), &transition));
    assert_eq!(progress.element(), Some(item));
    progress.complete();
    assert_eq!(completed.borrow().len(), 2);
    subscription.dispose();
    progress.complete();
    assert_eq!(completed.borrow().len(), 2);
}

#[test]
fn verify_queue_transition_schedules_transitions_on_next_frame() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let owner = TestTransitionProvider::new(false);
    let provider = TestTransitionProvider::new(false);
    let transition = add(&owner, element());
    let completed = Rc::new(Cell::new(0));
    let count = completed.clone();
    provider.transition_completed(move |_| count.set(count.get() + 1));
    for animate in [false, true] {
        provider.animate.set(animate);
        provider.started.borrow_mut().clear();
        provider.should_calls.set(0);
        completed.set(0);
        provider.queue_transition(transition.clone());
        assert_eq!(completed.get(), 0);
        assert!(provider.started.borrow().is_empty());
        clock.pulse(if animate { 16.0 } else { 0.0 });
        assert_eq!(completed.get(), 1);
        assert_eq!(provider.should_calls.get(), 1);
        assert_eq!(provider.started.borrow().len(), usize::from(animate));
    }
}

#[test]
fn disabled_animations_do_not_consult_the_provider_and_still_complete() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    provider.set_animations_enabled(false);
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    provider.transition_completed(move |_| observed.set(observed.get() + 1));
    provider.queue_transition(add(&provider, element()));
    clock.pulse(0.0);
    assert_eq!(provider.should_calls.get(), 0);
    assert!(provider.started.borrow().is_empty());
    assert_eq!(count.get(), 1);
}

struct MotionPlatformSettings {
    base: ferroui_base::platform::DefaultPlatformSettings,
    enabled: Cell<bool>,
}
impl MotionPlatformSettings {
    fn new(enabled: bool) -> Rc<Self> {
        Rc::new(Self {
            base: ferroui_base::platform::DefaultPlatformSettings::new(),
            enabled: Cell::new(enabled),
        })
    }
}
impl ferroui_base::platform::IPlatformSettings for MotionPlatformSettings {
    fn get_tap_size(&self, kind: ferroui_base::input::PointerType) -> Size {
        self.base.get_tap_size(kind)
    }
    fn get_double_tap_size(&self, kind: ferroui_base::input::PointerType) -> Size {
        self.base.get_double_tap_size(kind)
    }
    fn get_double_tap_time(&self, kind: ferroui_base::input::PointerType) -> std::time::Duration {
        self.base.get_double_tap_time(kind)
    }
    fn hold_wait_duration(&self) -> std::time::Duration {
        self.base.hold_wait_duration()
    }
    fn hotkey_configuration(
        &self,
    ) -> Rc<ferroui_base::input::platform::PlatformHotkeyConfiguration> {
        self.base.hotkey_configuration()
    }
    fn preferred_application_language(&self) -> String {
        self.base.preferred_application_language()
    }
    fn animations_enabled(&self) -> bool {
        self.enabled.get()
    }
    fn get_color_values(&self) -> ferroui_base::platform::PlatformColorValues {
        self.base.get_color_values()
    }
    fn color_values_changed(
        &self,
        handler: Rc<dyn Fn(&ferroui_base::platform::PlatformColorValues)>,
    ) -> Rc<dyn IDisposable> {
        self.base.color_values_changed(handler)
    }
    fn preferred_application_language_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.preferred_application_language_changed(handler)
    }
}

#[test]
fn queued_transitions_observe_live_native_motion_settings_per_element() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    let item = element();
    let root = TestRoot::with_child(&item);
    let settings = MotionPlatformSettings::new(false);
    root.set_platform_settings(Some(settings.clone()));
    let completed = Rc::new(Cell::new(0));
    let observed = completed.clone();
    provider.transition_completed(move |_| observed.set(observed.get() + 1));

    provider.queue_transition(add(&provider, item.clone()));
    settings.enabled.set(true);
    provider.queue_transition(add(&provider, item));
    assert_eq!(
        completed.get(),
        0,
        "Even disabled transitions complete on a frame."
    );
    clock.pulse(0.0);
    assert_eq!(provider.should_calls.get(), 1);
    assert_eq!(provider.started.borrow().len(), 1);
    assert_eq!(completed.get(), 2);
}

#[test]
fn unattached_transitions_use_application_motion_preference_without_overriding_local_opt_out() {
    let _scope = test_scope();
    let settings = MotionPlatformSettings::new(false);
    let _app = ferroui_controls::testing::UnitTestApplication::start(
        ferroui_controls::testing::TestServices::default().with_platform_settings(settings.clone()),
    );
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    provider.queue_transition(add(&provider, element()));
    clock.pulse(0.0);
    assert_eq!(provider.should_calls.get(), 0);
    assert!(provider.started.borrow().is_empty());

    settings.enabled.set(true);
    provider.set_animations_enabled(false);
    provider.queue_transition(add(&provider, element()));
    clock.pulse(16.0);
    assert_eq!(provider.should_calls.get(), 0);
    provider.set_animations_enabled(true);
    provider.queue_transition(add(&provider, element()));
    clock.pulse(32.0);
    assert_eq!(provider.should_calls.get(), 1);
}

fn transition_repeater() -> (
    Ref<super::ItemsRepeater>,
    Rc<ferroui_base::collections::FerroList<i32>>,
) {
    let repeater = super::ItemsRepeater::new();
    let source = Rc::new(ferroui_base::collections::FerroList::from_items([0, 1]));
    repeater.set_items_source(Some(source.clone().into()));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| Some(element()),
        true,
    )));
    arrange_transition_repeater(&repeater);
    (repeater, source)
}

fn arrange_transition_repeater(repeater: &super::ItemsRepeater) {
    repeater.measure(Size::new(80.0, 100.0));
    repeater.arrange(Rect::new(0.0, 0.0, 80.0, repeater.desired_size().height));
}

#[test]
fn validate_item_collection_transition_provider() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let repeater = super::ItemsRepeater::new();
    let source = Rc::new(ferroui_base::collections::FerroList::from_items(0..10));
    repeater.set_items_source(Some(source.clone().into()));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let child = Border::new();
            child.set_height(50.0);
            Some(child.upcast())
        },
        true,
    )));
    arrange_transition_repeater(&repeater);
    let provider = TestTransitionProvider::new(true);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    let weak = repeater.downgrade();
    *provider.start.borrow_mut() = Some(Rc::new(move |transitions| {
        let repeater = weak.upgrade().unwrap();
        for transition in transitions {
            let progress = transition.start();
            observed.borrow_mut().push((
                repeater.get_element_index(&progress.element().unwrap()),
                transition.clone(),
            ));
            progress.complete();
        }
    }));
    repeater.set_item_transition_provider(Some(provider.clone().upcast()));
    for (frame, only_add) in [false, true].into_iter().enumerate() {
        provider.only_add.set(only_add);
        calls.borrow_mut().clear();
        source.insert(0, -1);
        source.remove_at(2);
        arrange_transition_repeater(&repeater);
        clock.pulse(frame as f64 * 16.0);
        let calls = calls.borrow();
        let additions: Vec<_> = calls
            .iter()
            .filter(|(_, t)| t.operation() == Operation::Add)
            .collect();
        let removals: Vec<_> = calls
            .iter()
            .filter(|(_, t)| t.operation() == Operation::Remove)
            .collect();
        let moves: Vec<_> = calls
            .iter()
            .filter(|(_, t)| t.operation() == Operation::Move)
            .collect();
        assert_eq!(additions.len(), 1);
        assert_eq!(additions[0].0, 0);
        assert_eq!(additions[0].1.triggers(), Triggers::COLLECTION_CHANGE_ADD);
        if only_add {
            assert!(removals.is_empty());
            assert!(moves.is_empty());
        } else {
            assert_eq!(removals.len(), 1);
            assert_eq!(removals[0].0, -1);
            assert_eq!(removals[0].1.triggers(), Triggers::COLLECTION_CHANGE_REMOVE);
            assert_eq!(moves.len(), 1);
            assert_eq!(moves[0].0, 1);
            assert_eq!(
                moves[0].1.triggers(),
                Triggers::COLLECTION_CHANGE_ADD | Triggers::COLLECTION_CHANGE_REMOVE
            );
            assert_eq!(moves[0].1.old_bounds().y, 0.0);
            assert_eq!(moves[0].1.new_bounds().y, 50.0);
        }
    }
}

#[test]
fn removal_completion_invalidates_arrange_and_does_not_recycle_a_reused_element_twice() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (repeater, source) = transition_repeater();
    let provider = TestTransitionProvider::new(true);
    *provider.start.borrow_mut() = Some(Rc::new(|transitions| {
        for transition in transitions {
            transition.start();
        }
    }));
    repeater.set_item_transition_provider(Some(provider.clone().upcast()));
    let removed = repeater.try_get_element(0).unwrap();
    source.remove_at(0);
    arrange_transition_repeater(&repeater);
    clock.pulse(0.0);
    let transition = provider
        .started
        .borrow()
        .iter()
        .find(|transition| transition.operation() == Operation::Remove)
        .unwrap()
        .clone();
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::Animator
    );
    assert!(repeater.is_arrange_valid());
    transition.start().complete();
    assert!(!repeater.is_arrange_valid());
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::ElementFactory
    );
    arrange_transition_repeater(&repeater);
    assert!(removed.bounds().x < -1000.0);

    source.add(2);
    arrange_transition_repeater(&repeater);
    assert_eq!(repeater.try_get_element(1), Some(removed.clone()));
    transition.start().complete();
    assert_eq!(repeater.try_get_element(1), Some(removed.clone()));
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::Layout
    );
    assert_eq!(
        *removed
            .data_context()
            .unwrap()
            .downcast_ref::<i32>()
            .unwrap(),
        2
    );

    // The old completion must also be harmless when the reused control is
    // Animator-owned again, rather than merely while it belongs to Layout.
    source.remove_at(1);
    arrange_transition_repeater(&repeater);
    clock.pulse(16.0);
    let next_removal = provider
        .started
        .borrow()
        .iter()
        .rev()
        .find(|candidate| {
            candidate.operation() == Operation::Remove && candidate.element() == removed
        })
        .unwrap()
        .clone();
    assert!(!Rc::ptr_eq(&transition, &next_removal));
    let notifications = Rc::new(Cell::new(0));
    let observed = notifications.clone();
    provider.transition_completed(move |_| observed.set(observed.get() + 1));
    transition.start().complete();
    transition.start().complete();
    assert_eq!(notifications.get(), 2);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::Animator
    );
    next_removal.start().complete();
    assert_eq!(notifications.get(), 3);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::ElementFactory
    );
}

#[test]
fn template_replacement_cancels_queued_transitions_without_affecting_shared_hosts() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (first, first_source) = transition_repeater();
    let (second, second_source) = transition_repeater();
    let provider = TestTransitionProvider::new(true);
    first.set_item_transition_provider(Some(provider.clone().upcast()));
    second.set_item_transition_provider(Some(provider.clone().upcast()));
    let completed = Rc::new(RefCell::new(Vec::new()));
    let observed = completed.clone();
    provider.transition_completed(move |args| {
        observed.borrow_mut().push(args.transition());
    });

    first_source.add(2);
    arrange_transition_repeater(&first);
    let old_elements = first.realized_elements();
    first_source.remove_at(0);
    arrange_transition_repeater(&first);
    let second_removed = second.try_get_element(0).unwrap();
    second_source.remove_at(0);
    arrange_transition_repeater(&second);
    first.set_data_template(Some(template(true)));
    assert!(completed.borrow().is_empty());
    assert!(provider.started.borrow().is_empty());
    for element in &old_elements {
        assert_eq!(
            super::ItemsRepeater::get_virtualization_info(element).owner(),
            ElementOwner::ElementFactory
        );
    }

    clock.pulse(0.0);
    let started = provider.started.borrow();
    assert!(started
        .iter()
        .all(|transition| !old_elements.contains(&transition.element())));
    assert!(started.iter().any(|transition| {
        transition.operation() == Operation::Remove && transition.element() == second_removed
    }));
    let completed = completed.borrow();
    for operation in [Operation::Add, Operation::Remove, Operation::Move] {
        assert!(completed.iter().any(|transition| {
            transition.operation() == operation
                && old_elements.contains(&transition.element())
                && !transition.has_started()
        }));
    }
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&second_removed).owner(),
        ElementOwner::ElementFactory
    );
    assert_eq!(second.realized_elements().len(), 1);
}

#[test]
fn shared_transition_provider_completes_only_the_repeaters_own_elements() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (first, source) = transition_repeater();
    let (second, _) = transition_repeater();
    let provider = TestTransitionProvider::new(true);
    // The unrelated host receives the completion first.
    second.set_item_transition_provider(Some(provider.clone().upcast()));
    first.set_item_transition_provider(Some(provider.clone().upcast()));
    let removed = first.try_get_element(0).unwrap();
    let second_first = second.try_get_element(0).unwrap();
    source.remove_at(0);
    arrange_transition_repeater(&first);
    clock.pulse(0.0);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::ElementFactory
    );
    assert_eq!(second.try_get_element(0), Some(second_first));
    assert_eq!(second.realized_elements().len(), 2);
}

#[test]
fn clearing_transition_provider_recycles_pending_removals_before_their_render_frame() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (first, first_source) = transition_repeater();
    let (second, second_source) = transition_repeater();
    let provider = TestTransitionProvider::new(true);
    first.set_item_transition_provider(Some(provider.clone().upcast()));
    second.set_item_transition_provider(Some(provider.clone().upcast()));
    let removed = first.try_get_element(0).unwrap();
    let retained = first.try_get_element(1).unwrap();
    let other_removed = second.try_get_element(0).unwrap();
    let cleared = Rc::new(Cell::new(0));
    let observed = cleared.clone();
    first.element_clearing(move |_| observed.set(observed.get() + 1));
    first_source.remove_at(0);
    second_source.remove_at(0);
    arrange_transition_repeater(&first);
    arrange_transition_repeater(&second);

    first.set_item_transition_provider(None);
    assert_eq!(cleared.get(), 1);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::ElementFactory
    );
    assert_eq!(first.try_get_element(0), Some(retained.clone()));
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&other_removed).owner(),
        ElementOwner::Animator
    );
    clock.pulse(0.0);
    assert_eq!(cleared.get(), 1);
    assert!(provider
        .started
        .borrow()
        .iter()
        .all(|transition| { transition.element() != removed && transition.element() != retained }));
    assert!(provider
        .started
        .borrow()
        .iter()
        .any(|transition| transition.element() == other_removed));
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&other_removed).owner(),
        ElementOwner::ElementFactory
    );
}

#[test]
fn replacing_transition_provider_stops_old_animations_and_preserves_new_removal_identity() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (first, first_source) = transition_repeater();
    let (second, second_source) = transition_repeater();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    first.set_item_transition_provider(Some(provider.clone().upcast()));
    second.set_item_transition_provider(Some(provider.clone().upcast()));
    let removed = first.try_get_element(0).unwrap();
    let retained = first.try_get_element(1).unwrap();
    let other_removed = second.try_get_element(0).unwrap();
    let cleared = Rc::new(Cell::new(0));
    let observed = cleared.clone();
    first.element_clearing(move |args| {
        assert!(args.element.render_transform().is_none());
        assert_eq!(args.element.opacity(), 1.0);
        observed.set(observed.get() + 1);
    });
    let completed = Rc::new(RefCell::new(Vec::new()));
    let observed = completed.clone();
    provider.transition_completed(move |args| observed.borrow_mut().push(args.transition()));
    first_source.remove_at(0);
    second_source.remove_at(0);
    arrange_transition_repeater(&first);
    arrange_transition_repeater(&second);
    clock.pulse(0.0);
    clock.pulse(50.0);
    assert_eq!(provider.active_animation_count(), 4);

    let replacement = TestTransitionProvider::new(true);
    *replacement.start.borrow_mut() = Some(Rc::new(|transitions| {
        for transition in transitions {
            transition.start();
        }
    }));
    first.set_item_transition_provider(Some(replacement.clone().upcast()));
    assert_eq!(cleared.get(), 1);
    assert_eq!(provider.active_animation_count(), 2);
    assert_eq!(first.try_get_element(0), Some(retained.clone()));
    assert!(removed.render_transform().is_none());
    assert!(retained.render_transform().is_none());
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::ElementFactory
    );
    let old_removal = completed
        .borrow()
        .iter()
        .find(|transition| {
            transition.operation() == Operation::Remove && transition.element() == removed
        })
        .unwrap()
        .clone();

    first_source.add(2);
    arrange_transition_repeater(&first);
    assert_eq!(first.try_get_element(1), Some(removed.clone()));
    first_source.remove_at(1);
    arrange_transition_repeater(&first);
    clock.pulse(60.0);
    let new_removal = replacement
        .started
        .borrow()
        .iter()
        .find(|transition| transition.operation() == Operation::Remove)
        .unwrap()
        .clone();
    let completion_count = completed.borrow().len();
    old_removal.start().complete();
    old_removal.start().complete();
    assert_eq!(completed.borrow().len(), completion_count + 2);
    assert_eq!(cleared.get(), 1);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::Animator
    );
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&other_removed).owner(),
        ElementOwner::Animator
    );
    new_removal.start().complete();
    assert_eq!(cleared.get(), 2);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&removed).owner(),
        ElementOwner::ElementFactory
    );
    clock.pulse(900.0);
    assert_eq!(provider.active_animation_count(), 0);
    assert_eq!(
        super::ItemsRepeater::get_virtualization_info(&other_removed).owner(),
        ElementOwner::ElementFactory
    );
}

#[test]
fn a_provider_change_from_cancellation_completion_supersedes_the_outer_change() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (repeater, source) = transition_repeater();
    let original = LinedFlowLayoutItemCollectionTransitionProvider::new();
    let superseded = TestTransitionProvider::new(true);
    let replacement = TestTransitionProvider::new(true);
    repeater.set_item_transition_provider(Some(original.clone().upcast()));
    let weak = repeater.downgrade();
    let newest = replacement.clone();
    original.transition_completed(move |args| {
        if args.transition().operation() == Operation::Remove {
            weak.upgrade()
                .unwrap()
                .set_item_transition_provider(Some(newest.clone().upcast()));
        }
    });
    source.remove_at(0);
    arrange_transition_repeater(&repeater);
    clock.pulse(0.0);
    repeater.set_item_transition_provider(Some(superseded.clone().upcast()));
    assert_eq!(
        repeater.item_transition_provider(),
        Some(replacement.clone().upcast())
    );
    source.add(2);
    arrange_transition_repeater(&repeater);
    clock.pulse(16.0);
    assert!(superseded.started.borrow().is_empty());
    assert!(replacement
        .started
        .borrow()
        .iter()
        .any(|transition| transition.operation() == Operation::Add));
    assert_eq!(original.active_animation_count(), 0);
}

#[test]
fn started_transitions_remain_available_until_the_full_keepalive_expires() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    let progress = Rc::new(RefCell::new(None));
    let captured = progress.clone();
    *provider.start.borrow_mut() = Some(Rc::new(move |transitions| {
        *captured.borrow_mut() = Some(transitions[0].start());
    }));
    provider.queue_transition(add(&provider, element()));
    clock.pulse(0.0);
    provider.started.borrow_mut().clear();
    let progress = progress.borrow().clone().unwrap();
    assert!(progress.transition().is_some());
    assert_eq!(provider.retained_batch_count(), 1);
    let timers = Dispatcher::timers_for_unit_tests();
    let timer = timers
        .iter()
        .find(|timer| timer.interval() == std::time::Duration::from_secs(5))
        .unwrap();
    Dispatcher::force_fire_timer_for_unit_tests(timer);
    assert_eq!(provider.retained_batch_count(), 0);
    assert!(progress.transition().is_none());
    progress.complete();
}

#[test]
fn cancellation_preserves_started_progress_keepalive_and_repeated_public_completion() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    let progress = Rc::new(RefCell::new(None));
    let captured = progress.clone();
    *provider.start.borrow_mut() = Some(Rc::new(move |transitions| {
        *captured.borrow_mut() = Some(transitions[0].start());
    }));
    let item = element();
    provider.queue_transition(add(&provider, item.clone()));
    clock.pulse(0.0);
    provider.started.borrow_mut().clear();
    let progress = progress.borrow().clone().unwrap();
    provider.cancel_transitions_for_element(&item);
    assert!(progress.transition().unwrap().is_cancelled());
    assert_eq!(provider.retained_batch_count(), 1);
    let completed = Rc::new(Cell::new(0));
    let observed = completed.clone();
    provider.transition_completed(move |_| observed.set(observed.get() + 1));
    progress.complete();
    progress.complete();
    assert_eq!(completed.get(), 2);
    let timer = Dispatcher::timers_for_unit_tests()
        .into_iter()
        .find(|timer| timer.interval() == std::time::Duration::from_secs(5))
        .unwrap();
    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert!(progress.transition().is_none());
    assert_eq!(provider.retained_batch_count(), 0);
    progress.complete();
    assert_eq!(completed.get(), 2);
}

#[test]
fn transition_callbacks_can_queue_a_new_independent_batch() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    let next = add(&provider, element());
    let weak = provider.downgrade();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    *provider.start.borrow_mut() = Some(Rc::new(move |_| {
        if count.get() == 0 {
            weak.upgrade().unwrap().queue_transition(next.clone());
        }
        count.set(count.get() + 1);
    }));
    provider.queue_transition(add(&provider, element()));
    clock.pulse(0.0);
    assert_eq!(calls.get(), 1);
    assert_eq!(provider.retained_batch_count(), 1);
    clock.pulse(16.0);
    assert_eq!(calls.get(), 2);
    assert_eq!(provider.retained_batch_count(), 0);
}

#[test]
fn transition_manager_records_trigger_combinations_and_recycles_only_removals() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(true);
    let recycled = Rc::new(RefCell::new(Vec::new()));
    let output = recycled.clone();
    let manager = TransitionManager::new(Rc::new(move |element| output.borrow_mut().push(element)));
    manager.on_transition_provider_changed(Some(provider.clone().upcast()));
    manager.on_items_source_changed(NotifyCollectionChangedAction::Add);
    let added = element();
    manager.on_element_prepared(&added);
    manager.on_items_source_changed(NotifyCollectionChangedAction::Remove);
    let removed = element();
    assert!(manager.clear_element(&removed));
    manager.on_element_bounds_changed(
        &added,
        Rect::new(0.0, 0.0, 80.0, 20.0),
        Rect::new(0.0, 20.0, 80.0, 20.0),
    );
    manager.on_owner_arranged();
    clock.pulse(0.0);
    let started = provider.started.borrow();
    assert_eq!(started.len(), 3);
    assert_eq!(started[0].triggers(), Triggers::COLLECTION_CHANGE_ADD);
    assert_eq!(started[1].triggers(), Triggers::COLLECTION_CHANGE_REMOVE);
    assert_eq!(
        started[2].triggers(),
        Triggers::COLLECTION_CHANGE_ADD | Triggers::COLLECTION_CHANGE_REMOVE
    );
    assert_eq!(*recycled.borrow(), vec![removed]);
}

#[test]
fn lined_transitions_run_full_remove_move_add_choreography_and_restore_styles() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    let base = provider
        .clone()
        .upcast::<ItemCollectionTransitionProvider>();
    let added = element();
    let removed = element();
    let moved = element();
    let original: Rc<dyn ITransform> =
        MatrixTransform::with_matrix(Matrix::create_translation(3.0, 4.0)).into();
    added.set_render_transform(Some(original.clone()));
    added.set_opacity(0.6);
    let added_transition = ItemCollectionTransition::new(
        base.clone(),
        added.clone(),
        Operation::Add,
        Triggers::COLLECTION_CHANGE_ADD,
    );
    let removed_transition = ItemCollectionTransition::new(
        base.clone(),
        removed.clone(),
        Operation::Remove,
        Triggers::COLLECTION_CHANGE_REMOVE,
    );
    let moved_transition = ItemCollectionTransition::with_bounds(
        base.clone(),
        moved.clone(),
        Triggers::COLLECTION_CHANGE_ADD | Triggers::COLLECTION_CHANGE_REMOVE,
        Rect::new(0.0, 0.0, 80.0, 20.0),
        Rect::new(0.0, 40.0, 80.0, 20.0),
    );
    let completed = Rc::new(RefCell::new(Vec::new()));
    let output = completed.clone();
    provider
        .transition_completed(move |args| output.borrow_mut().push(args.transition().operation()));
    // This direct provider call bypasses QueueTransition's normal batch
    // keepalive, so its caller owns the transition handles until completion.
    base.start_transitions(vec![
        added_transition.clone(),
        removed_transition.clone(),
        moved_transition.clone(),
    ]);
    assert_eq!(provider.active_animation_count(), 3);
    clock.pulse(0.0);
    clock.pulse(100.0);
    assert!(completed.borrow().is_empty());
    assert_eq!(added.render_transform().unwrap().value().m11, 0.0);
    clock.pulse(200.0);
    assert_eq!(*completed.borrow(), vec![Operation::Remove]);
    assert_eq!(provider.active_animation_count(), 2);
    clock.pulse(700.0);
    assert_eq!(
        *completed.borrow(),
        vec![Operation::Remove, Operation::Move]
    );
    let scale = added.render_transform().unwrap().value().m11;
    assert!(scale > 0.0 && scale < 1.0);
    clock.pulse(900.0);
    assert_eq!(
        *completed.borrow(),
        vec![Operation::Remove, Operation::Move, Operation::Add]
    );
    assert_eq!(provider.active_animation_count(), 0);
    assert_eq!(
        added.render_transform().unwrap().reference_id(),
        original.reference_id()
    );
    assert_eq!(added.opacity(), 0.6);
}

#[test]
fn initial_slide_in_animates_both_opacity_and_position_for_250_milliseconds() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    let item = element();
    let transition = ItemCollectionTransition::new(
        provider.clone().upcast(),
        item.clone(),
        Operation::Add,
        Triggers::LAYOUT_TRANSITION,
    );
    provider.start_transitions(vec![transition]);
    assert_eq!(item.opacity(), 0.0);
    assert_eq!(item.render_transform().unwrap().value().m32, 100.0);
    clock.pulse(0.0);
    clock.pulse(125.0);
    assert!(item.opacity() > 0.0 && item.opacity() < 1.0);
    let y = item.render_transform().unwrap().value().m32;
    assert!(y > 0.0 && y < 100.0);
    assert_eq!(provider.active_animation_count(), 1);
    clock.pulse(251.0);
    assert_eq!(item.opacity(), 1.0);
    assert!(item.render_transform().is_none());
    assert_eq!(provider.active_animation_count(), 0);
}

#[test]
fn replacing_an_animation_releases_its_transient_values_without_clobbering_new_binding_values() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    let item = element();
    let opacity = Rc::new(LightweightSubject::<f64>::new());
    let binding = item.bind(
        Visual::opacity_property(),
        opacity.clone(),
        BindingPriority::LocalValue,
    );
    opacity.on_next(0.8);
    let base = provider.clone().upcast();
    let transition = ItemCollectionTransition::new(
        base,
        item.clone(),
        Operation::Add,
        Triggers::LAYOUT_TRANSITION,
    );
    provider.start_transitions(vec![transition]);
    clock.pulse(0.0);
    clock.pulse(50.0);
    opacity.on_next(0.3);
    let replacement = ItemCollectionTransition::new(
        provider.clone().upcast(),
        item.clone(),
        Operation::Add,
        Triggers::COLLECTION_CHANGE_ADD,
    );
    provider.start_transitions(vec![replacement]);
    assert_eq!(provider.active_animation_count(), 1);
    clock.pulse(60.0);
    clock.pulse(230.0);
    assert_eq!(provider.active_animation_count(), 0);
    assert_eq!(item.opacity(), 0.3);
    binding.dispose();
}

#[test]
fn template_replacement_stops_native_animations_before_the_old_factory_reuses_controls() {
    let _scope = test_scope();
    let clock = TestClock::install();
    struct StyleCheckingFactory {
        pool: Ref<RecyclePool>,
        original_transform: Rc<dyn ITransform>,
        recycled: RefCell<Vec<Ref<Control>>>,
    }
    impl IElementFactory for StyleCheckingFactory {
        fn get_element(
            &self,
            args: &ElementFactoryGetArgs,
        ) -> Result<Ref<Control>, ElementFactoryError> {
            if let Some(element) = self.pool.try_get_element_with_owner("", args.parent())? {
                return Ok(element);
            }
            let element = element();
            element.set_render_transform(Some(self.original_transform.clone()));
            element.set_render_transform_origin(ferroui_base::RelativePoint::CENTER);
            element.set_opacity(0.3);
            Ok(element)
        }
        fn recycle_element(
            &self,
            args: &ElementFactoryRecycleArgs,
        ) -> Result<(), ElementFactoryError> {
            let element = args.element();
            assert_eq!(element.opacity(), 0.3);
            assert_eq!(
                element.render_transform().unwrap().reference_id(),
                self.original_transform.reference_id()
            );
            assert_eq!(
                element.render_transform_origin(),
                ferroui_base::RelativePoint::CENTER
            );
            self.recycled.borrow_mut().push(element.clone());
            self.pool.put_element_with_owner(element, "", args.parent())
        }
    }
    let original: Rc<dyn ITransform> =
        MatrixTransform::with_matrix(Matrix::create_translation(3.0, 4.0)).into();
    let old_factory = Rc::new(StyleCheckingFactory {
        pool: RecyclePool::new(),
        original_transform: original.clone(),
        recycled: RefCell::new(Vec::new()),
    });
    let first = super::ItemsRepeater::new();
    let first_source = Rc::new(ferroui_base::collections::FerroList::from_items([0, 1]));
    first.set_items_source(Some(first_source.clone().into()));
    first.set_element_factory(Some(old_factory.clone()));
    arrange_transition_repeater(&first);
    // StackLayout estimates its initial item size with a recycled probe.
    // Count only the template replacement performed below.
    old_factory.recycled.borrow_mut().clear();
    let old_elements = first.realized_elements();
    let removed = first.try_get_element(0).unwrap();
    let opacity = Rc::new(LightweightSubject::<f64>::new());
    let binding = removed.bind(
        Visual::opacity_property(),
        opacity.clone(),
        BindingPriority::LocalValue,
    );
    opacity.on_next(0.8);
    let (second, second_source) = transition_repeater();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    first.set_item_transition_provider(Some(provider.clone().upcast()));
    second.set_item_transition_provider(Some(provider.clone().upcast()));
    first_source.remove_at(0);
    second_source.remove_at(0);
    arrange_transition_repeater(&first);
    arrange_transition_repeater(&second);
    clock.pulse(0.0);
    clock.pulse(50.0);
    assert_eq!(provider.active_animation_count(), 4);
    assert_ne!(
        removed.render_transform().unwrap().reference_id(),
        original.reference_id()
    );
    opacity.on_next(0.3);
    let cleared = Rc::new(Cell::new(0));
    let observed = cleared.clone();
    let expected_transform = original.clone();
    first.element_clearing(move |args| {
        assert_eq!(args.element.opacity(), 0.3);
        assert_eq!(
            args.element.render_transform().unwrap().reference_id(),
            expected_transform.reference_id()
        );
        observed.set(observed.get() + 1);
    });

    first.set_data_template(Some(template(true)));
    assert_eq!(cleared.get(), 2);
    assert_eq!(old_factory.recycled.borrow().len(), 2);
    assert_eq!(provider.active_animation_count(), 2);
    assert_eq!(removed.opacity(), 0.3);

    // Reuse the exact controls in another native host while the shared
    // provider continues running the unrelated host's animations.
    let next = super::ItemsRepeater::new();
    next.set_items_source(Some(ferroui_controls::ItemsSource::from_values([2, 3])));
    next.set_element_factory(Some(old_factory.clone()));
    arrange_transition_repeater(&next);
    assert!(old_elements
        .iter()
        .all(|element| next.realized_elements().contains(element)));
    // The destination layout also performs an initial probe. Later animation
    // ticks must not add any recycling beyond this completed reuse operation.
    let recycle_count_after_reuse = old_factory.recycled.borrow().len();
    let replacement: Rc<dyn ITransform> =
        MatrixTransform::with_matrix(Matrix::create_translation(9.0, 10.0)).into();
    removed.set_render_transform(Some(replacement.clone()));
    opacity.on_next(0.45);
    clock.pulse(100.0);
    clock.pulse(900.0);
    assert_eq!(provider.active_animation_count(), 0);
    assert_eq!(removed.opacity(), 0.45);
    assert_eq!(
        removed.render_transform().unwrap().reference_id(),
        replacement.reference_id()
    );
    assert_eq!(
        old_factory.recycled.borrow().len(),
        recycle_count_after_reuse
    );
    assert_eq!(next.realized_elements().len(), 2);
    binding.dispose();
}

#[test]
fn reentrant_cancellation_prevents_later_direct_batch_transitions_from_starting() {
    let _scope = test_scope();
    let _clock = TestClock::install();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    let first = element();
    let second = element();
    let transition = |element: &Ref<Control>| {
        ItemCollectionTransition::new(
            provider.clone().upcast(),
            element.clone(),
            Operation::Add,
            Triggers::LAYOUT_TRANSITION,
        )
    };
    let old_first = transition(&first);
    let old_second = transition(&second);
    provider.start_transitions(vec![old_first.clone(), old_second.clone()]);
    assert_eq!(provider.active_animation_count(), 2);
    let replacement_first = transition(&first);
    let replacement_second = transition(&second);
    let weak = provider.downgrade();
    let completed_first = old_first.clone();
    let cancelled_first = first.clone();
    let cancelled_second = second.clone();
    provider.transition_completed(move |args| {
        if Rc::ptr_eq(&args.transition(), &completed_first) {
            let provider = weak.upgrade().unwrap();
            provider.cancel_transitions_for_element(&cancelled_first);
            provider.cancel_transitions_for_element(&cancelled_second);
        }
    });
    provider.start_transitions(vec![replacement_first.clone(), replacement_second.clone()]);
    assert_eq!(provider.active_animation_count(), 0);
    for transition in [&replacement_first, &replacement_second] {
        assert!(transition.is_cancelled());
        assert!(!transition.has_started());
    }
    for element in [&first, &second] {
        assert!(element.render_transform().is_none());
        assert_eq!(element.opacity(), 1.0);
    }
}

#[test]
fn reset_removals_and_layout_moves_are_completed_without_starting_animations() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let provider = LinedFlowLayoutItemCollectionTransitionProvider::new();
    let base = provider
        .clone()
        .upcast::<ItemCollectionTransitionProvider>();
    let add = ItemCollectionTransition::new(
        base.clone(),
        element(),
        Operation::Add,
        Triggers::COLLECTION_CHANGE_RESET,
    );
    let remove = ItemCollectionTransition::new(
        base.clone(),
        element(),
        Operation::Remove,
        Triggers::COLLECTION_CHANGE_RESET,
    );
    let movement = ItemCollectionTransition::with_bounds(
        base.clone(),
        element(),
        Triggers::LAYOUT_TRANSITION,
        Rect::default(),
        Rect::new(1.0, 0.0, 80.0, 20.0),
    );
    base.queue_transition(add.clone());
    base.queue_transition(remove.clone());
    base.queue_transition(movement.clone());
    clock.pulse(0.0);
    assert!(add.has_started());
    assert!(!remove.has_started());
    assert!(!movement.has_started());
    clock.pulse(16.0);
    clock.pulse(300.0);
    assert_eq!(provider.active_animation_count(), 0);
}

fn scroll_host_fixture() -> (
    Ref<TestRoot>,
    Ref<ItemsRepeaterScrollHost>,
    Ref<ScrollViewer>,
    Vec<Ref<Control>>,
) {
    let panel = StackPanel::new();
    panel.set_width(200.0);
    let items = (0..10)
        .map(|_| {
            let item = Border::new();
            item.set_width(200.0);
            item.set_height(100.0);
            panel.children().add(&item);
            item.upcast::<Control>()
        })
        .collect::<Vec<_>>();
    let viewer = ScrollViewer::new();
    viewer.set_template(Some(ferroui_controls::templates::FuncControlTemplate::for_type::<
        ScrollViewer,
    >(crate::test_support::create_template)));
    viewer.set_content(Some(Control::boxed(&panel)));
    viewer.set_horizontal_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Disabled);
    viewer.set_vertical_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Hidden);
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(200.0);
    host.set_height(300.0);
    host.set_scroll_viewer(Some(viewer.clone()));
    let root = TestRoot::with_child(&host);
    root.layout_manager().execute_initial_layout_pass();
    assert_eq!(viewer.viewport(), Size::new(200.0, 300.0));
    assert_eq!(viewer.extent(), Size::new(200.0, 1000.0));
    (root, host, viewer, items)
}

#[test]
fn verify_anchor_ratio_properties_round_trip() {
    let _scope = test_scope();
    let host = ItemsRepeaterScrollHost::new();
    assert_eq!(host.horizontal_anchor_ratio(), 0.0);
    assert_eq!(host.vertical_anchor_ratio(), 0.0);
    host.set_horizontal_anchor_ratio(0.5);
    host.set_vertical_anchor_ratio(1.0);
    assert_eq!(host.horizontal_anchor_ratio(), 0.5);
    assert_eq!(host.vertical_anchor_ratio(), 1.0);
    host.set_horizontal_anchor_ratio(f64::NAN);
    host.set_vertical_anchor_ratio(f64::NAN);
    assert!(host.horizontal_anchor_ratio().is_nan());
    assert!(host.vertical_anchor_ratio().is_nan());
}

#[test]
fn verify_scroll_viewer_property_replaces_child() {
    let _scope = test_scope();
    let host = ItemsRepeaterScrollHost::new();
    let first = ScrollViewer::new();
    let second = ScrollViewer::new();
    assert!(host.scroll_viewer().is_none());
    host.set_scroll_viewer(Some(first.clone()));
    assert_eq!(host.children().count(), 1);
    assert_eq!(host.scroll_viewer(), Some(first.clone()));
    host.set_scroll_viewer(Some(second.clone()));
    assert_eq!(host.children().count(), 1);
    assert_eq!(host.scroll_viewer(), Some(second));
    assert!(first.visual_parent().is_none());
    host.set_scroll_viewer(None);
    assert!(host.children().is_empty());
}

#[test]
fn verify_current_anchor_is_candidate_closest_to_viewport_edge() {
    let _scope = test_scope();
    let (_root, host, _viewer, items) = scroll_host_fixture();
    assert!(host.current_anchor().is_none());
    for item in &items {
        host.register_anchor_candidate(item);
    }
    for (ratio, index) in [(0.0, 0), (0.5, 1), (1.0, 2)] {
        host.set_horizontal_anchor_ratio(ratio);
        host.set_vertical_anchor_ratio(ratio);
        assert_eq!(host.current_anchor(), Some(items[index].clone()));
        assert_eq!(host.anchor_element(), host.current_anchor());
    }
    assert!(host.is_horizontally_scrollable());
    assert!(host.is_vertically_scrollable());
    assert!(ferroui_controls::as_scroll_anchor_provider(&host).is_some());
}

#[test]
fn legacy_scroll_host_uses_horizontal_ratio_for_the_vertical_edge() {
    let _scope = test_scope();
    let (_root, host, _viewer, items) = scroll_host_fixture();
    for item in &items {
        host.register_anchor_candidate(item);
    }
    host.set_horizontal_anchor_ratio(0.5);
    host.set_vertical_anchor_ratio(0.0);
    assert_eq!(host.current_anchor(), Some(items[1].clone()));
}

#[test]
fn verify_unregister_anchor_candidate_recomputes_anchor() {
    let _scope = test_scope();
    let (_root, host, _viewer, items) = scroll_host_fixture();
    host.set_horizontal_anchor_ratio(0.5);
    for item in items.iter().take(6) {
        host.register_anchor_candidate(item);
    }
    assert_eq!(host.current_anchor(), Some(items[1].clone()));
    host.unregister_anchor_candidate(&items[1]);
    assert_eq!(host.current_anchor(), Some(items[0].clone()));
    host.unregister_anchor_candidate(&Control::new());
    host.unregister_anchor_candidate(&items[1]);
    assert_eq!(host.current_anchor(), Some(items[0].clone()));
    host.unregister_anchor_candidate(&items[0]);
    assert_eq!(host.current_anchor(), Some(items[2].clone()));
    for item in &items {
        host.unregister_anchor_candidate(item);
    }
    assert!(host.current_anchor().is_none());
}

#[test]
fn verify_nan_anchor_ratio_yields_no_anchor() {
    let _scope = test_scope();
    let (_root, host, _viewer, items) = scroll_host_fixture();
    host.set_horizontal_anchor_ratio(f64::NAN);
    host.set_vertical_anchor_ratio(f64::NAN);
    for item in &items {
        host.register_anchor_candidate(item);
    }
    assert!(host.current_anchor().is_none());
}

#[test]
fn verify_anchor_and_relative_viewport_use_scroll_offset() {
    let _scope = test_scope();
    let (root, host, viewer, items) = scroll_host_fixture();
    assert_eq!(
        host.get_relative_viewport(&items[3]),
        Rect::new(0.0, -300.0, 200.0, 300.0)
    );
    viewer.set_offset(Vector::new(0.0, 200.0));
    // The viewport must be correct even before the child is arranged at the
    // new native offset, then remain correct after that arrange.
    assert_eq!(
        host.get_relative_viewport(&items[3]),
        Rect::new(0.0, -100.0, 200.0, 300.0)
    );
    root.layout_manager().execute_layout_pass();
    assert_eq!(
        host.get_relative_viewport(&items[1]),
        Rect::new(0.0, 100.0, 200.0, 300.0)
    );
    for item in &items {
        host.register_anchor_candidate(item);
    }
    assert_eq!(host.current_anchor(), Some(items[2].clone()));
}

#[test]
fn verify_scrolling_surface_without_scroll_viewer() {
    let _scope = test_scope();
    let host = ItemsRepeaterScrollHost::new();
    let item = Control::new();
    host.register_anchor_candidate(&item);
    assert!(host.current_anchor().is_none());
    assert_eq!(host.get_relative_viewport(&item), Rect::default());
    host.set_scroll_viewer(Some(ScrollViewer::new()));
    assert!(host.current_anchor().is_none());
}

#[test]
fn verify_scrolling_surface_events_can_be_subscribed_and_removed() {
    let _scope = test_scope();
    let (root, host, viewer, _items) = scroll_host_fixture();
    let viewport = Rc::new(Cell::new(0));
    let post = Rc::new(Cell::new(0));
    let configuration = Rc::new(Cell::new(0));
    let count = viewport.clone();
    let viewport_subscription = host.viewport_changed(move |_| count.set(count.get() + 1));
    let count = post.clone();
    let post_subscription = host.post_arrange(move || count.set(count.get() + 1));
    let count = configuration.clone();
    let configuration_subscription = host.configuration_changed(move || count.set(count.get() + 1));
    host.set_horizontal_anchor_ratio(0.5);
    viewer.set_offset(Vector::new(0.0, 150.0));
    host.invalidate_arrange();
    root.layout_manager().execute_layout_pass();
    assert!(viewport.get() > 0 && post.get() > 0 && configuration.get() > 0);
    let before = (viewport.get(), post.get(), configuration.get());
    viewport_subscription.dispose();
    post_subscription.dispose();
    configuration_subscription.dispose();
    host.set_horizontal_anchor_ratio(1.0);
    viewer.set_offset(Vector::new(0.0, 300.0));
    host.invalidate_arrange();
    root.layout_manager().execute_layout_pass();
    assert_eq!((viewport.get(), post.get(), configuration.get()), before);
}

#[test]
fn scroll_host_tracks_anchor_when_preceding_content_changes_size() {
    let _scope = test_scope();
    let (root, host, viewer, items) = scroll_host_fixture();
    viewer.set_offset(Vector::new(0.0, 200.0));
    root.layout_manager().execute_layout_pass();
    for item in &items {
        host.register_anchor_candidate(item);
    }
    assert_eq!(host.current_anchor(), Some(items[2].clone()));
    items[0].set_height(150.0);
    root.layout_manager().execute_layout_pass();
    assert_eq!(viewer.offset(), Vector::new(0.0, 250.0));
    assert_eq!(host.get_relative_viewport(&items[2]).y, 0.0);
}

#[test]
fn scroll_host_bring_into_view_waits_for_arrange_and_applies_alignment_and_offsets() {
    let _scope = test_scope();
    let (root, host, viewer, items) = scroll_host_fixture();
    host.start_bring_into_view(&items[5], 0.0, 0.5, 0.0, 10.0, false);
    assert!(host.has_pending_bring_into_view());
    assert_eq!(viewer.offset(), Vector::default());
    root.layout_manager().execute_layout_pass();
    assert_eq!(viewer.offset(), Vector::new(0.0, 410.0));
    assert!(!host.has_pending_bring_into_view());
    host.start_bring_into_view(&items[9], 0.0, 0.0, 0.0, 20.0, false);
    root.layout_manager().execute_layout_pass();
    assert_eq!(viewer.offset(), Vector::new(0.0, 700.0));
}

#[test]
fn animated_bring_into_view_uses_the_native_clock_and_finishes_at_the_extent() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (root, host, viewer, items) = scroll_host_fixture();
    host.start_bring_into_view(&items[9], 0.0, 0.0, 0.0, 0.0, true);
    root.layout_manager().execute_layout_pass();
    clock.pulse(0.0);
    assert_eq!(viewer.offset(), Vector::default());
    clock.pulse(125.0);
    assert!(viewer.offset().y > 0.0 && viewer.offset().y < 700.0);
    assert!(host.has_pending_bring_into_view());
    clock.pulse(251.0);
    assert_eq!(viewer.offset(), Vector::new(0.0, 700.0));
    assert!(!host.has_pending_bring_into_view());
}

#[test]
fn user_scroll_cancels_a_pending_animated_bring_into_view() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (root, host, viewer, items) = scroll_host_fixture();
    host.start_bring_into_view(&items[9], 0.0, 0.0, 0.0, 0.0, true);
    root.layout_manager().execute_layout_pass();
    clock.pulse(0.0);
    clock.pulse(100.0);
    viewer.set_offset(Vector::new(0.0, 120.0));
    assert!(!host.has_pending_bring_into_view());
    clock.pulse(251.0);
    assert_eq!(viewer.offset(), Vector::new(0.0, 120.0));
}

// Additional source API scenarios exercise the same service contracts through
// FerroUI templates, observable collections, and native metadata invocation.
#[test]
fn verify_recycling_element_factory_templates_setter() {
    let _scope = test_scope();
    let button: Rc<dyn IDataTemplate> =
        FuncDataTemplate::new(|_| true, |_, _| Some(Button::new().upcast()), true);
    let templates = super::ElementFactoryTemplates::new();
    templates.add("Only".into(), button.clone());
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.set_templates(Some(templates));
    assert_eq!(factory.templates().count(), 1);
    assert!(factory.templates().contains_key(&"Only".into()));
    assert!(factory
        .get_element(&ElementFactoryGetArgs::default())
        .unwrap()
        .is::<Button>());
    let empty_key = super::ElementFactoryTemplates::new();
    empty_key.add(String::new(), button);
    factory.set_templates(Some(empty_key));
    assert_eq!(
        factory.get_element(&ElementFactoryGetArgs::default()),
        Err(ElementFactoryError::EmptyTemplateKey)
    );
}

#[test]
fn verify_select_template_key_handler_removal() {
    let _scope = test_scope();
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory.templates().add(
        "Text".into(),
        FuncDataTemplate::new(|_| true, |_, _| Some(TextBlock::new().upcast()), true),
    );
    factory.templates().add(
        "Button".into(),
        FuncDataTemplate::new(|_| true, |_, _| Some(Button::new().upcast()), true),
    );
    let remaining_calls = Rc::new(RefCell::new(Vec::new()));
    let seen = remaining_calls.clone();
    factory.select_template_key(move |args| {
        seen.borrow_mut().push(args.template_key());
        args.set_template_key("Text");
        assert_eq!(args.template_key(), "Text");
    });
    let removed_calls = Rc::new(Cell::new(0));
    let count = removed_calls.clone();
    let subscription = factory.select_template_key(move |args| {
        count.set(count.get() + 1);
        args.set_template_key("Button");
    });
    let args = ElementFactoryGetArgs::new(Some(Rc::new(1i32)), Some(StackPanel::new().upcast()));
    assert!(factory.get_element(&args).unwrap().is::<Button>());
    assert_eq!(removed_calls.get(), 1);
    subscription.dispose();
    args.set_data(Some(Rc::new(2i32)));
    assert!(factory.get_element(&args).unwrap().is::<TextBlock>());
    assert_eq!(removed_calls.get(), 1);
    assert_eq!(*remaining_calls.borrow(), [String::new(), String::new()]);
}

struct ErrorSelector {
    error: Option<ElementFactoryError>,
    first_error: bool,
    fallback_calls: Cell<usize>,
}
impl IDataTemplateSelector for ErrorSelector {
    fn select_template(&self, _item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        None
    }
    fn select_template_with_container(
        &self,
        _item: Option<&BoxedValue>,
        _container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        None
    }
    fn try_select_template(
        &self,
        _item: Option<&BoxedValue>,
    ) -> Result<Option<Rc<dyn IDataTemplate>>, ElementFactoryError> {
        if self.first_error {
            Err(self.error.clone().unwrap())
        } else {
            Ok(None)
        }
    }
    fn try_select_template_with_container(
        &self,
        _item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Result<Option<Rc<dyn IDataTemplate>>, ElementFactoryError> {
        assert!(container.is_none());
        self.fallback_calls.set(self.fallback_calls.get() + 1);
        self.error.clone().map_or(Ok(None), Err)
    }
}

#[test]
fn verify_null_template_from_selector_fails_with_invalid_argument() {
    let _scope = test_scope();
    for error in [
        None,
        Some(ElementFactoryError::InvalidArgument(
            "Null container".into(),
        )),
    ] {
        let selector = Rc::new(ErrorSelector {
            error,
            first_error: false,
            fallback_calls: Cell::new(0),
        });
        let wrapper = ItemTemplateWrapper::with_selector(selector.clone());
        assert_eq!(
            wrapper.get_element(&ElementFactoryGetArgs::default()),
            Err(ElementFactoryError::MissingTemplate)
        );
        assert_eq!(selector.fallback_calls.get(), 1);
    }
}

#[test]
fn verify_null_template_gives_meaningfull_error() {
    let _scope = test_scope();
    let wrapper = ItemTemplateWrapper::with_selector(Rc::new(ErrorSelector {
        error: None,
        first_error: false,
        fallback_calls: Cell::new(0),
    }));
    let error = wrapper
        .get_element(&ElementFactoryGetArgs::default())
        .unwrap_err();
    assert_eq!(error.to_string(), "Null encountered as data template. That is not a valid value for a data template, and can not be used.");
}

#[test]
fn verify_selector_error_is_propagated() {
    let _scope = test_scope();
    let error = ElementFactoryError::TemplateSelection("Selector failure".into());
    for first_error in [false, true] {
        let selector = Rc::new(ErrorSelector {
            error: Some(error.clone()),
            first_error,
            fallback_calls: Cell::new(0),
        });
        let wrapper = ItemTemplateWrapper::with_selector(selector.clone());
        assert_eq!(
            wrapper.get_element(&ElementFactoryGetArgs::default()),
            Err(error.clone())
        );
        assert_eq!(selector.fallback_calls.get(), usize::from(!first_error));
    }
}

#[repr(C)]
struct CountingRecyclePool {
    base: RecyclePool,
    puts: RefCell<Vec<(Ref<Control>, String)>>,
    gets: Cell<usize>,
}
ferro_class!(CountingRecyclePool: RecyclePool);
impl FerroObjectImpl for CountingRecyclePool {}
impl RecyclePoolImpl for CountingRecyclePool {
    fn put_element_core(
        this: &Self,
        element: Ref<Control>,
        key: &str,
        owner: Option<Ref<Control>>,
    ) -> Result<(), ElementFactoryError> {
        this.puts.borrow_mut().push((element.clone(), key.into()));
        Self::parent_put_element_core(this, element, key, owner)
    }
    fn try_get_element_core(
        this: &Self,
        key: &str,
        owner: Option<Ref<Control>>,
    ) -> Result<Option<Ref<Control>>, ElementFactoryError> {
        this.gets.set(this.gets.get() + 1);
        Self::parent_try_get_element_core(this, key, owner)
    }
}
impl CountingRecyclePool {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: RecyclePool::construct(),
            puts: RefCell::new(Vec::new()),
            gets: Cell::new(0),
        })
    }
}

fn template_repeater_fixture(
    template: Rc<dyn IDataTemplate>,
) -> (
    Ref<super::ItemsRepeater>,
    ferroui_base::collections::FerroList<String>,
) {
    let data = ferroui_base::collections::FerroList::new();
    for index in 0..3 {
        data.add(format!("Item #{index}"));
    }
    let repeater = super::ItemsRepeater::new();
    repeater.set_layout(Some(super::StackLayout::new().upcast()));
    repeater.set_items_source(Some(ferroui_controls::ItemsSource::new(Rc::new(data.clone()))));
    repeater.set_item_template(Some(Rc::new(template)));
    measure_template_repeater(&repeater);
    (repeater, data)
}
fn measure_template_repeater(repeater: &super::ItemsRepeater) {
    repeater.measure(Size::new(400.0, 400.0));
    repeater.arrange(Rect::new(0.0, 0.0, 400.0, 400.0));
}

#[test]
fn verify_pool_instance_is_used_for_data_template_recycling() {
    let _scope = test_scope();
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
        |_| true,
        |data, _| {
            let block = TextBlock::new();
            block.set_height(20.0);
            block.set_text(
                data.as_ref()
                    .and_then(|data| data.downcast_ref::<String>())
                    .map(String::as_str),
            );
            Some(block.upcast())
        },
        true,
    );
    assert!(RecyclePool::get_pool_instance(&template).is_none());
    let pool = RecyclePool::new();
    RecyclePool::set_pool_instance(&template, Some(pool.clone()));
    assert_eq!(
        RecyclePool::get_pool_instance(&template),
        Some(pool.clone())
    );
    let (repeater, data) = template_repeater_fixture(template.clone());
    assert!(pool
        .try_get_element_with_owner("", Some(repeater.clone().upcast()))
        .unwrap()
        .is_none());
    let removed = repeater.try_get_element(2).unwrap();
    data.remove_at(2);
    measure_template_repeater(&repeater);
    assert_eq!(
        pool.try_get_element_with_owner("", Some(repeater.clone().upcast()))
            .unwrap(),
        Some(removed.clone())
    );
    assert!(pool
        .try_get_element_with_owner("", Some(repeater.clone().upcast()))
        .unwrap()
        .is_none());
    data.add("Item #3".into());
    measure_template_repeater(&repeater);
    let added = repeater.try_get_element(2).unwrap();
    assert_ne!(added, removed);
    assert_eq!(
        added.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("Item #3")
    );
    drop(repeater);
    RecyclePool::set_pool_instance(&template, None);
    assert!(RecyclePool::get_pool_instance(&template).is_none());
    RecyclePool::set_pool_instance(&template, Some(pool));
    RecyclePool::set_pool_instance(&template, None);
    assert!(RecyclePool::get_pool_instance(&template).is_none());
}

#[test]
fn verify_recycle_pool_overrides_are_called() {
    let _scope = test_scope();
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let element = Border::new();
            element.set_height(20.0);
            Some(element.upcast())
        },
        true,
    );
    let pool = CountingRecyclePool::new();
    RecyclePool::set_pool_instance(&template, Some(pool.clone().upcast()));
    let (repeater, data) = template_repeater_fixture(template);
    // The sizing probe can recycle before collection edits. Isolate the
    // removal under test without making that layout implementation observable.
    pool.puts.borrow_mut().clear();
    let removed = repeater.try_get_element(2).unwrap();
    data.remove_at(2);
    measure_template_repeater(&repeater);
    assert_eq!(*pool.puts.borrow(), [(removed, String::new())]);
    let previous_gets = pool.gets.get();
    data.add("Item #3".into());
    measure_template_repeater(&repeater);
    assert!(pool.gets.get() > previous_gets);
}

#[test]
fn metadata_builds_keyed_templates_and_invokes_factory_events_and_methods() {
    use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupTyped};
    let _scope = test_scope();
    crate::register_types();
    let dictionary_type = <super::ElementFactoryTemplatesMetadata as MarkupTyped>::MARKUP;
    let dictionary = (dictionary_type.constructors[0].invoke)(&[]).unwrap();
    let add = dictionary_type.find_methods("Add").next().unwrap();
    let text: Rc<dyn IDataTemplate> =
        FuncDataTemplate::new(|_| true, |_, _| Some(TextBlock::new().upcast()), true);
    let button: Rc<dyn IDataTemplate> =
        FuncDataTemplate::new(|_| true, |_, _| Some(Button::new().upcast()), true);
    (add.invoke)(&[
        dictionary.clone(),
        into_markup_value("Text".to_owned()),
        into_markup_value(text),
    ])
    .unwrap();
    (add.invoke)(&[
        dictionary.clone(),
        into_markup_value("Button".to_owned()),
        into_markup_value(button.clone()),
    ])
    .unwrap();
    assert!((add.invoke)(&[
        dictionary.clone(),
        into_markup_value("Button".to_owned()),
        into_markup_value(button)
    ])
    .is_err());
    let templates = dictionary
        .unwrap()
        .downcast_ref::<super::ElementFactoryTemplates>()
        .unwrap()
        .clone();
    let factory = RecyclingElementFactory::new();
    factory.set_templates(Some(templates));
    factory.set_recycle_pool(Some(RecyclePool::new()));
    let event = RecyclingElementFactory::TYPE
        .markup()
        .unwrap()
        .find_event("SelectTemplateKey")
        .unwrap();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let handler = MarkupDelegate::new(move |args| {
        count.set(count.get() + 1);
        assert!(args[0].is_some());
        args[1]
            .as_ref()
            .unwrap()
            .downcast_ref::<super::SelectTemplateEventArgs>()
            .unwrap()
            .set_template_key("Button");
        None
    });
    (event.add)(&[
        into_markup_value(factory.clone()),
        into_markup_value(handler),
    ])
    .unwrap();
    let get = ElementFactory::TYPE
        .markup()
        .unwrap()
        .find_methods("GetElement")
        .next()
        .unwrap();
    let element = (get.invoke)(&[
        into_markup_value(factory),
        into_markup_value(ElementFactoryGetArgs::default()),
    ])
    .unwrap()
    .unwrap();
    assert!(Control::from_boxed(&element).unwrap().is::<Button>());
    assert_eq!(calls.get(), 1);
}

#[test]
fn metadata_transition_methods_and_completion_events_preserve_handles() {
    use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupTyped};
    let _scope = test_scope();
    crate::register_types();
    let clock = TestClock::install();
    let provider = TestTransitionProvider::new(false);
    let transition = add(&provider, element());
    let completed = Rc::new(RefCell::new(None));
    let seen = completed.clone();
    let handler = MarkupDelegate::new(move |args| {
        *seen.borrow_mut() = Some(
            args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<ItemCollectionTransitionCompletedEventArgs>()
                .unwrap()
                .transition(),
        );
        None
    });
    let metadata = ItemCollectionTransitionProvider::TYPE.markup().unwrap();
    let base = provider
        .clone()
        .upcast::<ItemCollectionTransitionProvider>();
    (metadata.find_event("TransitionCompleted").unwrap().add)(&[
        into_markup_value(base.clone()),
        into_markup_value(handler),
    ])
    .unwrap();
    (metadata
        .find_methods("QueueTransition")
        .next()
        .unwrap()
        .invoke)(&[
        into_markup_value(base),
        into_markup_value(transition.clone()),
    ])
    .unwrap();
    assert!(completed.borrow().is_none());
    clock.pulse(0.0);
    assert!(Rc::ptr_eq(
        completed.borrow().as_ref().unwrap(),
        &transition
    ));
    let metadata = <ItemCollectionTransition as MarkupTyped>::MARKUP;
    let progress = (metadata.find_methods("Start").next().unwrap().invoke)(&[into_markup_value(
        transition.clone(),
    )])
    .unwrap()
    .unwrap();
    let progress = progress
        .downcast_ref::<Rc<super::ItemCollectionTransitionProgress>>()
        .unwrap();
    assert!(Rc::ptr_eq(&progress.transition().unwrap(), &transition));
}

#[test]
fn metadata_static_build_completion_subscription_is_disposable() {
    use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupTyped};
    let _scope = test_scope();
    crate::register_types();
    let clock = TestClock::install();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let handler = MarkupDelegate::new(move |_| {
        count.set(count.get() + 1);
        None
    });
    let subscribe = <RepeaterTestHooks as MarkupTyped>::MARKUP
        .find_methods("SubscribeBuildTreeCompleted")
        .next()
        .unwrap();
    let subscription = (subscribe.invoke)(&[into_markup_value(handler)])
        .unwrap()
        .unwrap();
    let subscription = subscription.downcast_ref::<Rc<dyn IDisposable>>().unwrap();
    BuildTreeScheduler::register_work(0, || {});
    clock.pulse(0.0);
    assert_eq!(calls.get(), 1);
    subscription.dispose();
    BuildTreeScheduler::register_work(0, || {});
    clock.pulse(16.0);
    assert_eq!(calls.get(), 1);
}

#[test]
fn scroll_animation_and_event_subscriptions_do_not_retain_a_dropped_host() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (root, host, viewer, items) = scroll_host_fixture();
    let weak = host.downgrade();
    let viewport = host.viewport_changed(|_| {});
    let configuration = host.configuration_changed(|| {});
    host.register_anchor_candidate(&items[0]);
    host.start_bring_into_view(&items[9], 0.0, 0.0, 0.0, 0.0, true);
    root.layout_manager().execute_layout_pass();
    clock.pulse(0.0);
    assert!(host.has_pending_bring_into_view());
    // Detach the visual subtree before releasing the test root: the native
    // layout manager retains registered viewport listeners and queued layout
    // work until detachment and the next layout pass.
    root.set_child(None::<Ref<Control>>);
    root.layout_manager().execute_layout_pass();
    drop(root);
    drop(host);
    assert!(weak.upgrade().is_none());
    clock.pulse(500.0);
    viewer.set_offset(Vector::new(0.0, 30.0));
    viewport.dispose();
    configuration.dispose();
}

struct PhaseBudgetDispatcher {
    now: Cell<i64>,
    signaled: ferroui_base::threading::DispatcherImplEvent,
    timer: ferroui_base::threading::DispatcherImplEvent,
}
struct PhaseBudgetSignal;
impl ferroui_base::threading::IDispatcherSignal for PhaseBudgetSignal {
    fn signal(&self) {}
}
impl ferroui_base::threading::IDispatcherImpl for PhaseBudgetDispatcher {
    fn current_thread_is_loop_thread(&self) -> bool {
        true
    }
    fn signal(&self) {}
    fn signal_handle(&self) -> std::sync::Arc<dyn ferroui_base::threading::IDispatcherSignal> {
        std::sync::Arc::new(PhaseBudgetSignal)
    }
    fn signaled(&self) -> &ferroui_base::threading::DispatcherImplEvent {
        &self.signaled
    }
    fn timer(&self) -> &ferroui_base::threading::DispatcherImplEvent {
        &self.timer
    }
    fn now(&self) -> i64 {
        self.now.get()
    }
    fn update_timer(&self, _due: Option<i64>) {}
}
impl PhaseBudgetDispatcher {
    fn install() -> Rc<Self> {
        let platform = Rc::new(Self {
            now: Cell::new(0),
            signaled: ferroui_base::threading::DispatcherImplEvent::new(),
            timer: ferroui_base::threading::DispatcherImplEvent::new(),
        });
        Dispatcher::initialize_ui_thread_dispatcher(platform.clone());
        platform
    }
}
struct FullPhasingComponent {
    phases: usize,
    calls: RefCell<Vec<i32>>,
    cost: Option<Rc<PhaseBudgetDispatcher>>,
    frame: Rc<Cell<usize>>,
    frames: RefCell<Vec<usize>>,
}
impl IDataTemplateComponent for FullPhasingComponent {
    fn process_bindings(&self, data: Option<BoxedValue>, index: i32, phase: i32) -> i32 {
        assert!(data.is_some());
        if phase > 0 {
            assert_eq!(index, -1);
            self.frames.borrow_mut().push(self.frame.get());
        }
        self.calls.borrow_mut().push(phase);
        if let Some(cost) = &self.cost {
            cost.now.set(cost.now.get() + 15);
        }
        if (phase as usize) + 1 == self.phases {
            -1
        } else {
            phase + 1
        }
    }
    fn recycle(&self) {}
}

#[test]
fn validate_slow_phases_are_spread_over_frames() {
    let _scope = test_scope();
    let dispatcher = PhaseBudgetDispatcher::install();
    let clock = TestClock::install();
    let data = (0..6)
        .map(|index| Rc::new(index) as BoxedValue)
        .collect::<Vec<_>>();
    let phaser = Phaser::new(
        Rc::new(|| Rect::new(0.0, 0.0, 400.0, 400.0)),
        Rc::new(move |index| data.get(index as usize).cloned()),
    );
    let frame = Rc::new(Cell::new(0));
    let components = (0..6)
        .map(|index| {
            let component = Rc::new(FullPhasingComponent {
                phases: 4,
                calls: RefCell::new(Vec::new()),
                cost: Some(dispatcher.clone()),
                frame: frame.clone(),
                frames: RefCell::new(Vec::new()),
            });
            let element = element();
            let info = VirtualizationInfo::new();
            info.move_ownership_to_layout_from_element_factory(index, "");
            info.set_arrange_bounds(Rect::new(0.0, index as f64 * 50.0, 100.0, 50.0));
            Phaser::set_data_template_component(&element, Some(component.clone()));
            phaser.phase_element(&element, &info);
            component
        })
        .collect::<Vec<_>>();
    assert_eq!(phaser.pending_count(), 6);
    for current in 1..=20 {
        frame.set(current);
        clock.pulse(current as f64 * 16.0);
        if current == 1 {
            assert!(
                phaser.pending_count() > 0,
                "Work beyond the 40 ms budget must remain for another frame."
            );
        }
        if phaser.pending_count() == 0 {
            break;
        }
    }
    assert_eq!(phaser.pending_count(), 0);
    let mut frames = std::collections::BTreeSet::new();
    for component in components {
        assert_eq!(*component.calls.borrow(), [0, 1, 2, 3]);
        frames.extend(component.frames.borrow().iter().copied());
    }
    assert!(frames.len() > 1);
    assert_eq!(BuildTreeScheduler::budget_milliseconds(), 40.0);
}

fn verify_phase_sequence(phase_count: usize) {
    let _scope = test_scope();
    let clock = TestClock::install();
    let data = (0..9)
        .map(|index| Rc::new(index) as BoxedValue)
        .collect::<Vec<_>>();
    let phaser = Phaser::new(
        Rc::new(|| Rect::new(0.0, 0.0, 400.0, 400.0)),
        Rc::new(move |index| data.get(index as usize).cloned()),
    );
    let completed = Rc::new(Cell::new(0));
    let observed = completed.clone();
    let subscription =
        RepeaterTestHooks::build_tree_completed(move || observed.set(observed.get() + 1));
    let components = (0..9)
        .map(|index| {
            let component = Rc::new(FullPhasingComponent {
                phases: phase_count,
                calls: RefCell::new(Vec::new()),
                cost: None,
                frame: Rc::new(Cell::new(0)),
                frames: RefCell::new(Vec::new()),
            });
            let element = element();
            let info = VirtualizationInfo::new();
            info.move_ownership_to_layout_from_element_factory(index, "");
            info.set_arrange_bounds(Rect::new(0.0, index as f64 * 100.0, 100.0, 100.0));
            Phaser::set_data_template_component(&element, Some(component.clone()));
            phaser.phase_element(&element, &info);
            component
        })
        .collect::<Vec<_>>();
    if phase_count == 1 {
        assert_eq!(phaser.pending_count(), 0);
        assert_eq!(BuildTreeScheduler::pending_work_count(), 0);
    } else {
        assert_eq!(phaser.pending_count(), 9);
    }
    clock.pulse(0.0);
    assert_eq!(phaser.pending_count(), 0);
    assert_eq!(completed.get(), usize::from(phase_count > 1));
    let expected = (0..phase_count as i32).collect::<Vec<_>>();
    assert_eq!(components.len(), 9);
    for component in components {
        assert_eq!(*component.calls.borrow(), expected);
    }
    subscription.dispose();
}

#[test]
fn validate_phase_invoke_and_ordering() {
    verify_phase_sequence(6);
}

#[test]
fn validate_x_bind_without_phasing() {
    verify_phase_sequence(1);
}

fn bound_text_template(text: Option<&'static str>, height: f64) -> Rc<dyn IDataTemplate> {
    use ferroui_base::data::ReflectionBinding;
    FuncDataTemplate::new(
        |_| true,
        move |_, _| {
            let block = TextBlock::new();
            block.set_height(height);
            block.set_width(50.0);
            if let Some(text) = text {
                block.set_text(Some(text));
            } else {
                block.bind_binding(
                    TextBlock::text_property().as_property(),
                    &*ReflectionBinding::empty(),
                );
            }
            Some(block.upcast())
        },
        true,
    )
}

fn host_repeater_fixture(
    items: ferroui_controls::ItemsSource,
    item_template: BoxedValue,
) -> (
    Ref<TestRoot>,
    Ref<ItemsRepeaterScrollHost>,
    Ref<ScrollViewer>,
    Ref<super::ItemsRepeater>,
) {
    let repeater = super::ItemsRepeater::new();
    repeater.set_layout(Some(super::StackLayout::new().upcast()));
    repeater.set_items_source(Some(items));
    repeater.set_item_template(Some(item_template));
    let viewer = ScrollViewer::new();
    viewer.set_template(Some(ferroui_controls::templates::FuncControlTemplate::for_type::<
        ScrollViewer,
    >(crate::test_support::create_template)));
    viewer.set_content(Some(Control::boxed(&repeater)));
    viewer.set_horizontal_scroll_bar_visibility(ferroui_controls::primitives::ScrollBarVisibility::Disabled);
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(400.0);
    host.set_height(400.0);
    host.set_scroll_viewer(Some(viewer.clone()));
    let root = TestRoot::with_child(host.clone().upcast::<Control>());
    root.layout_manager().execute_initial_layout_pass();
    (root, host, viewer, repeater)
}

fn keyed_factory() -> Ref<RecyclingElementFactory> {
    let factory = RecyclingElementFactory::new();
    factory.set_recycle_pool(Some(RecyclePool::new()));
    factory
}

#[test]
fn validate_recycling() {
    let _scope = test_scope();
    let factory = keyed_factory();
    factory
        .templates()
        .add("even".into(), bound_text_template(Some("even"), 20.0));
    factory
        .templates()
        .add("odd".into(), bound_text_template(Some("odd"), 20.0));
    factory.select_template_key(|args| {
        let data = args.data_context().unwrap();
        args.set_template_key(if data.downcast_ref::<i32>().unwrap() % 2 == 0 {
            "even"
        } else {
            "odd"
        });
    });
    let owner = super::ItemsRepeater::new();
    let args = ElementFactoryGetArgs::new(Some(Rc::new(0i32)), Some(owner.clone().upcast()));
    let even = factory.get_element(&args).unwrap();
    assert_eq!(
        even.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("even")
    );
    factory
        .recycle_element(&ElementFactoryRecycleArgs::new(
            even.clone(),
            Some(owner.clone().upcast()),
        ))
        .unwrap();
    args.set_data(Some(Rc::new(1i32)));
    let odd = factory.get_element(&args).unwrap();
    assert_ne!(odd, even);
    assert_eq!(
        odd.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("odd")
    );
    factory
        .recycle_element(&ElementFactoryRecycleArgs::new(
            odd.clone(),
            Some(owner.upcast()),
        ))
        .unwrap();
    args.set_data(Some(Rc::new(2i32)));
    let next_even = factory.get_element(&args).unwrap();
    assert_eq!(next_even, even);
    assert_eq!(
        next_even.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("even")
    );
    args.set_data(Some(Rc::new(3i32)));
    let next_odd = factory.get_element(&args).unwrap();
    assert_eq!(next_odd, odd);
    assert_eq!(
        next_odd.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("odd")
    );
}

#[test]
fn validate_binding_and_template_selection() {
    let _scope = test_scope();
    let factory = keyed_factory();
    factory
        .templates()
        .add("static".into(), bound_text_template(Some("static"), 20.0));
    factory
        .templates()
        .add("bind".into(), bound_text_template(None, 20.0));
    factory.select_template_key(|args| {
        args.set_template_key(
            if args.data_context().unwrap().downcast_ref::<i32>().unwrap() % 2 == 0 {
                "bind"
            } else {
                "static"
            },
        );
    });
    let (_root, _host, _viewer, repeater) =
        host_repeater_fixture(ferroui_controls::ItemsSource::from_values(0..10), Rc::new(factory));
    assert_eq!(repeater.children().count(), 10);
    for index in 0..10 {
        let element = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        if index % 2 == 0 {
            assert_eq!(element.text(), Some(index.to_string()));
            assert_eq!(
                element.data_context().unwrap().downcast_ref::<i32>(),
                Some(&index)
            );
        } else {
            assert_eq!(element.text().as_deref(), Some("static"));
        }
    }
}

#[repr(C)]
struct CustomRecyclingFactory {
    base: RecyclingElementFactory,
    selections: Cell<usize>,
    recycled: Cell<usize>,
}
ferro_class!(CustomRecyclingFactory: RecyclingElementFactory);
impl FerroObjectImpl for CustomRecyclingFactory {}
impl super::RecyclingElementFactoryImpl for CustomRecyclingFactory {
    fn on_select_template_key_core(
        this: &Self,
        _data: Option<BoxedValue>,
        _owner: Option<Ref<Control>>,
    ) -> Result<String, ElementFactoryError> {
        this.selections.set(this.selections.get() + 1);
        Ok("key".into())
    }
}
impl super::ElementFactoryImpl for CustomRecyclingFactory {
    fn get_element_core(
        this: &Self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        use super::ElementFactoryImplExt;
        let element = Self::parent_get_element_core(this, args)?;
        let text = RepeaterTestHooks::get_element_factory_element_index().to_string();
        element.cast::<TextBlock>().unwrap().set_text(Some(&text));
        Ok(element)
    }
    fn recycle_element_core(
        this: &Self,
        args: &ElementFactoryRecycleArgs,
    ) -> Result<(), ElementFactoryError> {
        use super::ElementFactoryImplExt;
        this.recycled.set(this.recycled.get() + 1);
        args.element()
            .cast::<TextBlock>()
            .unwrap()
            .set_text(Some("uninitialized"));
        Self::parent_recycle_element_core(this, args)
    }
}
impl CustomRecyclingFactory {
    fn new() -> Ref<Self> {
        let factory: Ref<Self> = instantiate(Self {
            base: RecyclingElementFactory::construct(),
            selections: Cell::new(0),
            recycled: Cell::new(0),
        });
        factory.templates().add(
            "key".into(),
            bound_text_template(Some("uninitialized"), 20.0),
        );
        factory.set_recycle_pool(Some(RecyclePool::new()));
        factory
    }
}

fn custom_recycling_scenario(single_template: bool) {
    let _scope = test_scope();
    let factory = CustomRecyclingFactory::new();
    if single_template {
        factory.select_template_key(|_| panic!("Selection must be skipped for a single template."));
    }
    let (root, _host, _viewer, repeater) = host_repeater_fixture(
        ferroui_controls::ItemsSource::from_values((0..10).map(|index| format!("Item #{index}"))),
        Rc::new(factory.clone()),
    );
    assert_eq!(repeater.children().count(), 10);
    for index in 0..10 {
        assert_eq!(
            repeater
                .try_get_element(index)
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
                .text(),
            Some(index.to_string())
        );
    }
    assert_eq!(factory.selections.get(), 0);
    let before = factory.recycled.get();
    repeater.set_items_source(None);
    root.layout_manager().execute_layout_pass();
    assert!(factory.recycled.get() >= before + 10);
}

#[test]
fn validate_custom_recycling_element_factory() {
    custom_recycling_scenario(false);
}

#[test]
fn validate_recycling_element_factory_with_single_template() {
    custom_recycling_scenario(true);
}

fn element_name_binding_scenario(nested: bool) {
    use ferroui_base::controls::{ChildNameScope, NameScope, NameScopeRef};
    use ferroui_base::data::ReflectionBinding;
    let _scope = test_scope();
    let panel = StackPanel::new();
    panel.set_width(200.0);
    let scope = NameScopeRef::new(NameScope::new());
    NameScope::set_name_scope(&panel, Some(scope.clone()));
    let sibling = TextBlock::new();
    sibling.set_name(Some("siblingTextBlock".to_owned()));
    sibling.set_text(Some("DataSource"));
    scope.register("siblingTextBlock", sibling.clone().upcast());
    panel.children().add(&sibling);
    let weak_scope = Rc::downgrade(&scope.0);
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
        |_| true,
        move |_, _| {
            let scope = NameScopeRef::new(ChildNameScope::new(NameScopeRef(
                weak_scope.upgrade().unwrap(),
            )));
            let block = TextBlock::new();
            block.bind_binding(
                TextBlock::text_property().as_property(),
                &*ReflectionBinding::empty(),
            );
            let tag = ReflectionBinding::new("Text");
            tag.set_element_name(Some("siblingTextBlock".into()));
            tag.set_name_scope(Some(Rc::downgrade(&scope.0)));
            block.bind_binding(Control::tag_property().as_property(), &*tag);
            let root = if nested {
                let border = Border::new();
                border.set_border_thickness(ferroui_base::Thickness::uniform(1.0));
                border.set_child(Some(block.upcast::<Control>()));
                border.upcast::<Control>()
            } else {
                block.upcast()
            };
            NameScope::set_name_scope(&root, Some(scope.clone()));
            scope.complete();
            Some(root)
        },
        true,
    );
    let repeater = super::ItemsRepeater::new();
    repeater.set_layout(Some(super::StackLayout::new().upcast()));
    repeater.set_items_source(Some(ferroui_controls::ItemsSource::from_values(
        (0..5).map(|index| index.to_string()),
    )));
    repeater.set_item_template(Some(Rc::new(template)));
    panel.children().add(&repeater);
    scope.complete();
    let root = TestRoot::with_child(panel.upcast::<Control>());
    root.layout_manager().execute_initial_layout_pass();
    assert_eq!(repeater.children().count(), 5);
    for index in 0..5 {
        let item = repeater.try_get_element(index).unwrap();
        let block = if nested {
            item.cast::<Border>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
        } else {
            item.cast::<TextBlock>().unwrap()
        };
        assert_eq!(block.text(), Some(index.to_string()));
        assert_eq!(
            block
                .tag()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("DataSource")
        );
    }
    // The template binding continues to observe its native named source.
    sibling.set_text(Some("Updated"));
    for item in repeater.realized_elements() {
        let block = if nested {
            item.cast::<Border>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
        } else {
            item.cast::<TextBlock>().unwrap()
        };
        assert_eq!(
            block
                .tag()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("Updated")
        );
    }
    repeater.set_items_source(None);
    root.layout_manager().execute_layout_pass();
}

#[test]
fn validate_data_template_with_element_name_binding() {
    element_name_binding_scenario(false);
}

#[test]
fn validate_data_template_with_nested_element_name_binding() {
    element_name_binding_scenario(true);
}

#[test]
fn validate_data_template_as_item_template() {
    let _scope = test_scope();
    let template = bound_text_template(None, 20.0);
    let (root, _host, _viewer, repeater) = host_repeater_fixture(
        ferroui_controls::ItemsSource::from_values((0..10).map(|index| index.to_string())),
        Rc::new(template),
    );
    assert_eq!(repeater.children().count(), 10);
    for index in 0..10 {
        assert_eq!(
            repeater
                .try_get_element(index)
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
                .text(),
            Some(index.to_string())
        );
    }
    repeater.set_items_source(None);
    root.layout_manager().execute_layout_pass();
    assert!(repeater.realized_elements().is_empty());
}

struct AlternatingSelector {
    even: Rc<dyn IDataTemplate>,
    odd: Rc<dyn IDataTemplate>,
    container_only: bool,
    fallbacks: Cell<usize>,
}
impl AlternatingSelector {
    fn select(&self, item: Option<&BoxedValue>) -> Rc<dyn IDataTemplate> {
        if item.unwrap().downcast_ref::<i32>().unwrap() % 2 == 0 {
            self.even.clone()
        } else {
            self.odd.clone()
        }
    }
}
impl IDataTemplateSelector for AlternatingSelector {
    fn select_template(&self, item: Option<&BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        if self.container_only {
            None
        } else {
            Some(self.select(item))
        }
    }
    fn select_template_with_container(
        &self,
        item: Option<&BoxedValue>,
        container: Option<&Ref<Control>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        assert!(container.is_none());
        self.fallbacks.set(self.fallbacks.get() + 1);
        Some(self.select(item))
    }
}

#[test]
fn validate_data_template_selector_as_item_template() {
    let _scope = test_scope();
    let selector: Rc<dyn IDataTemplateSelector> = Rc::new(AlternatingSelector {
        even: bound_text_template(None, 40.0),
        odd: bound_text_template(None, 30.0),
        container_only: false,
        fallbacks: Cell::new(0),
    });
    let (root, _host, _viewer, repeater) =
        host_repeater_fixture(ferroui_controls::ItemsSource::from_values(0..10), Rc::new(selector));
    assert_eq!(repeater.children().count(), 10);
    for index in 0..10 {
        let element = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(element.text(), Some(index.to_string()));
        assert_eq!(element.height(), if index % 2 == 0 { 40.0 } else { 30.0 });
    }
    repeater.set_items_source(None);
    root.layout_manager().execute_layout_pass();
}

#[test]
fn validate_correct_size_when_empty_data_template_in_selector() {
    let _scope = test_scope();
    let selector: Rc<dyn IDataTemplateSelector> = Rc::new(AlternatingSelector {
        even: FuncDataTemplate::new(|_| true, |_, _| None, true),
        odd: bound_text_template(None, 30.0),
        container_only: false,
        fallbacks: Cell::new(0),
    });
    let repeater = super::ItemsRepeater::new();
    repeater.set_layout(Some(super::StackLayout::new().upcast()));
    repeater.set_item_template(Some(Rc::new(selector)));
    repeater.set_items_source(Some(ferroui_controls::ItemsSource::from_values(0..10)));
    repeater.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    repeater.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
    let root = TestRoot::with_child(repeater.clone().upcast::<Control>());
    // A Window has a nonempty client area even when its first realized item
    // has an empty template. TestRoot otherwise arranges its desired size.
    root.set_width(300.0);
    root.set_height(300.0);
    root.layout_manager().execute_initial_layout_pass();
    assert_eq!(repeater.children().count(), 10);
    for index in 0..10 {
        assert_eq!(
            repeater.try_get_element(index).unwrap().height(),
            if index % 2 == 0 { 0.0 } else { 30.0 }
        );
    }
    assert_eq!(repeater.bounds().height, 150.0);
    repeater.set_items_source(None);
    root.layout_manager().execute_layout_pass();
}

#[test]
fn validate_reycling_element_factory_with_no_template() {
    let _scope = test_scope();
    let factory = keyed_factory();
    let args = ElementFactoryGetArgs::new(Some(Rc::new(0i32)), None);
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::MissingTemplates)
    );
}

#[repr(C)]
struct ButtonElementFactory {
    base: ElementFactory,
}
ferro_class!(ButtonElementFactory: ElementFactory);
impl FerroObjectImpl for ButtonElementFactory {}
impl super::ElementFactoryImpl for ButtonElementFactory {
    fn get_element_core(
        _this: &Self,
        args: &ElementFactoryGetArgs,
    ) -> Result<Ref<Control>, ElementFactoryError> {
        let data = args.data().unwrap();
        let index = data.downcast_ref::<i32>().unwrap();
        let button = Button::new();
        button.set_content(Some(Rc::new(index.to_string())));
        button.set_height(20.0);
        Ok(button.upcast())
    }
    fn recycle_element_core(
        _this: &Self,
        args: &ElementFactoryRecycleArgs,
    ) -> Result<(), ElementFactoryError> {
        assert!(args.element().is::<Button>());
        Ok(())
    }
}

#[test]
fn validate_custom_element_factory() {
    let _scope = test_scope();
    let factory: Ref<ButtonElementFactory> = instantiate(ButtonElementFactory {
        base: ElementFactory::construct(),
    });
    let (_root, _host, _viewer, repeater) =
        host_repeater_fixture(ferroui_controls::ItemsSource::from_values(0..10), Rc::new(factory));
    for index in 0..10 {
        let element = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<Button>()
            .unwrap();
        assert_eq!(
            element.content().unwrap().downcast_ref::<String>(),
            Some(&index.to_string())
        );
    }
}

#[test]
fn verify_select_template_layout_fallback() {
    let _scope = test_scope();
    let selector = Rc::new(AlternatingSelector {
        even: bound_text_template(None, 30.0),
        odd: bound_text_template(Some("static"), 30.0),
        container_only: true,
        fallbacks: Cell::new(0),
    });
    let handle: Rc<dyn IDataTemplateSelector> = selector.clone();
    let (_root, _host, _viewer, repeater) =
        host_repeater_fixture(ferroui_controls::ItemsSource::from_values(0..10), Rc::new(handle));
    assert_eq!(repeater.children().count(), 10);
    for index in 0..10 {
        let element = repeater
            .try_get_element(index)
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(
            element.text(),
            Some(if index % 2 == 0 {
                index.to_string()
            } else {
                "static".into()
            })
        );
        if index % 2 == 0 {
            assert_eq!(
                element.data_context().unwrap().downcast_ref::<i32>(),
                Some(&index)
            );
        }
    }
    assert!(selector.fallbacks.get() >= 10);
}

#[test]
fn verify_recycling_element_factory_template_key_validation() {
    let _scope = test_scope();
    let factory = keyed_factory();
    factory
        .templates()
        .add("Text".into(), bound_text_template(None, 20.0));
    factory.templates().add(
        "Button".into(),
        FuncDataTemplate::new(|_| true, |_, _| Some(Button::new().upcast()), true),
    );
    let owner = StackPanel::new();
    let args = ElementFactoryGetArgs::new(Some(Rc::new(1i32)), Some(owner.clone().upcast()));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::EmptyTemplateKey)
    );
    let key = Rc::new(RefCell::new("Missing".to_string()));
    let current_key = key.clone();
    let weak_owner = owner.downgrade();
    factory.select_template_key(move |args| {
        assert_eq!(
            args.owner(),
            weak_owner.upgrade().map(|owner| owner.upcast())
        );
        assert!(args.data_context().unwrap().downcast_ref::<i32>().is_some());
        args.set_template_key(current_key.borrow().clone());
    });
    args.set_data(Some(Rc::new(2i32)));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::UnknownTemplateKey("Missing".into()))
    );
    key.borrow_mut().clear();
    args.set_data(Some(Rc::new(3i32)));
    assert_eq!(
        factory.get_element(&args),
        Err(ElementFactoryError::EmptyTemplateKey)
    );
    *key.borrow_mut() = "Button".into();
    args.set_data(Some(Rc::new(4i32)));
    assert!(factory.get_element(&args).unwrap().is::<Button>());
    *key.borrow_mut() = "Text".into();
    args.set_data(Some(Rc::new(5i32)));
    assert!(factory.get_element(&args).unwrap().is::<TextBlock>());
}

#[test]
fn host_bring_releases_a_distant_repeater_anchor_and_allows_scrolling_back() {
    let _scope = test_scope();
    let clock = TestClock::install();
    let (root, host, viewer, repeater) = host_repeater_fixture(
        ferroui_controls::ItemsSource::from_values(0..1000),
        Rc::new(bound_text_template(None, 20.0)),
    );
    assert!(repeater.try_get_element(500).is_none());
    let target = repeater.get_or_create_element(500);
    assert_eq!(
        repeater.viewport_manager.made_anchor(),
        Some(target.clone())
    );
    host.start_bring_into_view(&target, 0.0, 0.0, 0.0, 0.0, true);
    root.layout_manager().execute_layout_pass();
    assert!(host.has_pending_bring_into_view());
    assert!(repeater.viewport_manager.made_anchor().is_none());
    clock.pulse(0.0);
    clock.pulse(100.0);
    root.layout_manager().execute_layout_pass();
    assert!(viewer.offset().y > 0.0 && viewer.offset().y < 10000.0);
    clock.pulse(251.0);
    root.layout_manager().execute_layout_pass();
    assert!(!host.has_pending_bring_into_view());
    assert!((viewer.offset().y - 10000.0).abs() < 1.0);
    assert!(repeater.try_get_element(500).is_some());
    viewer.set_offset(Vector::default());
    root.layout_manager().execute_layout_pass();
    assert_eq!(viewer.offset(), Vector::default());
    // Layout coordinates retain the origin established by the distant
    // anchor. The native scroll position and arranged row bounds are zero;
    // a layout-visible Y of -10000 is correct for an extent at -10000.
    assert!(
        (repeater.visible_window().y - repeater.viewport_manager.layout_extent().y).abs() < 1.0,
        "offset {:?}, viewport {:?}, layout extent {:?}",
        viewer.offset(),
        repeater.visible_window(),
        repeater.viewport_manager.layout_extent(),
    );
    let first = repeater.try_get_element(0).expect("first row is realized");
    assert!(first.bounds().y.abs() < 1.0);
    assert!(repeater.try_get_element(500).is_none());
    assert!(repeater.realized_elements().len() < 100);
}
