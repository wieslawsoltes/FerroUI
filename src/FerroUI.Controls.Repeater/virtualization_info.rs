// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::phaser::IDataTemplateComponent;
use ferroui_base::{BoxedValue, Rect};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The current owner of a repeater-generated element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElementOwner {
    ElementFactory,
    Layout,
    PinnedPool,
    UniqueIdResetPool,
    Animator,
}

/// The ownership and layout state of one realized element. The template component
/// is held weakly; the phased item is held strongly while phasing lasts (see `data`).
pub struct VirtualizationInfo {
    owner: Cell<ElementOwner>,
    index: Cell<i32>,
    pin_counter: Cell<u32>,
    pin_generation: Cell<u64>,
    unique_id: RefCell<String>,
    arrange_bounds: Cell<Rect>,
    phase: Cell<i32>,
    phase_generation: Cell<u64>,
    // Deviation (winui-repeater/DEVIATIONS.md, WR-32): WinUI holds the item weakly, because a
    // WinRT source returns the same object on every call; a typed FerroUI list boxes a new
    // value on every `get_at`, which nothing else would keep alive for the later phases.
    // `stop_phasing` clears it, so a recycled element does not root the source.
    data: RefCell<Option<BoxedValue>>,
    component: RefCell<Option<Weak<dyn IDataTemplateComponent>>>,
    must_clear_data_context: Cell<bool>,
    keep_alive: Cell<bool>,
    auto_recycle_candidate: Cell<bool>,
}

impl PartialEq for VirtualizationInfo {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Default for VirtualizationInfo {
    fn default() -> Self {
        Self {
            owner: Cell::new(ElementOwner::ElementFactory),
            index: Cell::new(-1),
            pin_counter: Cell::new(0),
            pin_generation: Cell::new(0),
            unique_id: RefCell::new(String::new()),
            arrange_bounds: Cell::new(Self::INVALID_RECT),
            phase: Cell::new(Self::PHASE_NOT_SPECIFIED),
            phase_generation: Cell::new(0),
            data: RefCell::new(None),
            component: RefCell::new(None),
            must_clear_data_context: Cell::new(false),
            keep_alive: Cell::new(false),
            auto_recycle_candidate: Cell::new(false),
        }
    }
}
impl VirtualizationInfo {
    pub const PHASE_NOT_SPECIFIED: i32 = i32::MIN;
    pub const PHASE_REACHED_END: i32 = -1;
    pub const INVALID_RECT: Rect = Rect::new(-1.0, -1.0, -1.0, -1.0);
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    pub fn owner(&self) -> ElementOwner {
        self.owner.get()
    }
    pub fn index(&self) -> i32 {
        self.index.get()
    }
    pub fn unique_id(&self) -> String {
        self.unique_id.borrow().clone()
    }
    pub fn is_pinned(&self) -> bool {
        self.pin_counter.get() > 0
    }
    pub fn pin_count(&self) -> u32 {
        self.pin_counter.get()
    }
    pub(crate) fn pin_generation(&self) -> u64 {
        self.pin_generation.get()
    }
    /// Release a scoped operation's contribution only while it still belongs
    /// to this realization. Stable-key reset ownership retains that identity.
    pub(crate) fn release_scoped_pin(&self, generation: u64) -> bool {
        if self.pin_generation.get() != generation || !self.is_pinned() {
            return false;
        }
        let count = self.pin_counter.get() - 1;
        self.pin_counter.set(count);
        count == 0
    }
    fn reset_pins(&self) {
        self.pin_counter.set(0);
        self.pin_generation
            .set(self.pin_generation.get().wrapping_add(1));
    }
    pub fn is_held_by_layout(&self) -> bool {
        self.owner() == ElementOwner::Layout
    }
    pub fn is_in_pinned_pool(&self) -> bool {
        self.owner() == ElementOwner::PinnedPool
    }
    pub fn is_in_unique_id_reset_pool(&self) -> bool {
        self.owner() == ElementOwner::UniqueIdResetPool
    }
    pub fn is_realized(&self) -> bool {
        self.is_held_by_layout() || self.is_in_pinned_pool()
    }
    pub fn phase(&self) -> i32 {
        self.phase.get()
    }
    pub(crate) fn phase_generation(&self) -> u64 {
        self.phase_generation.get()
    }
    pub fn set_phase(&self, phase: i32) {
        self.phase.set(phase);
    }
    pub fn data(&self) -> Option<BoxedValue> {
        self.data.borrow().clone()
    }
    pub fn data_template_component(&self) -> Option<Rc<dyn IDataTemplateComponent>> {
        self.component.borrow().as_ref().and_then(Weak::upgrade)
    }
    pub fn update_phasing_info(
        &self,
        phase: i32,
        data: Option<BoxedValue>,
        component: Option<Rc<dyn IDataTemplateComponent>>,
    ) {
        self.phase.set(phase);
        self.phase_generation
            .set(self.phase_generation.get().wrapping_add(1));
        *self.data.borrow_mut() = data;
        *self.component.borrow_mut() = component.as_ref().map(Rc::downgrade);
    }
    pub fn must_clear_data_context(&self) -> bool {
        self.must_clear_data_context.get()
    }
    pub fn set_must_clear_data_context(&self, value: bool) {
        self.must_clear_data_context.set(value);
    }
    pub fn keep_alive(&self) -> bool {
        self.keep_alive.get()
    }
    pub fn set_keep_alive(&self, value: bool) {
        self.keep_alive.set(value);
    }
    pub fn auto_recycle_candidate(&self) -> bool {
        self.auto_recycle_candidate.get()
    }
    pub fn set_auto_recycle_candidate(&self, value: bool) {
        self.auto_recycle_candidate.set(value);
    }
    pub fn arrange_bounds(&self) -> Rect {
        self.arrange_bounds.get()
    }
    pub fn set_arrange_bounds(&self, value: Rect) {
        self.arrange_bounds.set(value);
    }
    pub fn move_ownership_to_layout_from_element_factory(
        &self,
        index: i32,
        unique_id: impl Into<String>,
    ) {
        assert_eq!(self.owner(), ElementOwner::ElementFactory);
        self.owner.set(ElementOwner::Layout);
        self.index.set(index);
        *self.unique_id.borrow_mut() = unique_id.into();
    }
    pub fn move_ownership_to_layout_from_unique_id_reset_pool(&self) {
        assert_eq!(self.owner(), ElementOwner::UniqueIdResetPool);
        self.owner.set(ElementOwner::Layout);
    }
    pub fn move_ownership_to_layout_from_pinned_pool(&self) {
        assert_eq!(self.owner(), ElementOwner::PinnedPool);
        // As WinUI's `MUX_ASSERT`, a check of debug builds: a handler can release the last pin
        // during measure, before the pool is pruned again.
        debug_assert!(self.is_pinned());
        self.owner.set(ElementOwner::Layout);
    }
    pub fn move_ownership_to_element_factory(&self) {
        assert_ne!(self.owner(), ElementOwner::ElementFactory);
        self.owner.set(ElementOwner::ElementFactory);
        self.reset_pins();
        self.index.set(-1);
        self.unique_id.borrow_mut().clear();
        self.arrange_bounds.set(Self::INVALID_RECT);
    }
    pub fn move_ownership_to_unique_id_reset_pool_from_layout(&self) {
        assert_eq!(self.owner(), ElementOwner::Layout);
        self.owner.set(ElementOwner::UniqueIdResetPool);
    }
    pub(crate) fn move_ownership_to_unique_id_reset_pool_from_pinned_pool(&self) {
        assert_eq!(self.owner(), ElementOwner::PinnedPool);
        self.owner.set(ElementOwner::UniqueIdResetPool);
    }
    pub fn move_ownership_to_animator(&self) {
        assert!(matches!(
            self.owner(),
            ElementOwner::Layout | ElementOwner::UniqueIdResetPool
        ));
        self.owner.set(ElementOwner::Animator);
        self.index.set(-1);
        self.reset_pins();
    }
    pub fn move_ownership_to_pinned_pool(&self) {
        assert_eq!(self.owner(), ElementOwner::Layout);
        self.owner.set(ElementOwner::PinnedPool);
    }
    pub fn add_pin(&self) -> u32 {
        assert!(self.is_realized(), "You cannot pin an unrealized element.");
        let count = self
            .pin_counter
            .get()
            .checked_add(1)
            .expect("Element pin count overflowed.");
        self.pin_counter.set(count);
        count
    }
    pub fn remove_pin(&self) -> u32 {
        assert!(
            self.is_realized(),
            "You cannot unpin an unrealized element."
        );
        assert!(
            self.is_pinned(),
            "UnpinElement was called more often than PinElement."
        );
        let count = self.pin_counter.get() - 1;
        self.pin_counter.set(count);
        count
    }
    /// Releases a pin of an element in the unique-id reset pool, which keeps its pins across a
    /// keyed reset (WR-29) although it is not realized.
    pub(crate) fn remove_reset_pool_pin(&self) -> u32 {
        assert!(self.is_in_unique_id_reset_pool());
        let count = self.pin_counter.get().saturating_sub(1);
        self.pin_counter.set(count);
        count
    }
    pub fn update_index(&self, index: i32) {
        assert!(self.is_realized());
        self.index.set(index);
    }
}
