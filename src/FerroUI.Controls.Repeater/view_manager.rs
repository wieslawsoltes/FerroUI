// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::unique_id_element_pool::UniqueIdElementPool;
use super::{
    ElementFactoryGetArgs, ElementFactoryRecycleArgs, ElementOwner, ElementRealizationOptions,
    ItemsRepeater, ItemsRepeaterElementClearingEventArgs,
    ItemsRepeaterElementIndexChangedEventArgs, ItemsRepeaterElementPreparedEventArgs, Phaser,
};
use ferroui_controls::{Control, ItemsChangedEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::{
    FocusManager, InputElement, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode,
    NavigationMethod,
};
use ferroui_base::{Ref, Visual, WeakRef};
use rustc_hash::FxHashMap;
use std::cell::{Cell, RefCell};

/// Element ownership and source-index bookkeeping for an ItemsRepeater.
/// Index lookups use a hash table of weak handles; source notifications touch
/// only realized elements, never all source items.
#[derive(Default)]
pub(crate) struct ViewManager {
    by_index: RefCell<FxHashMap<i32, Vec<WeakRef<Control>>>>,
    reset_pool: UniqueIdElementPool,
    stable_reset_pending: Cell<bool>,
    pinned: RefCell<Vec<Ref<Control>>>,
    focused: RefCell<Option<WeakRef<Control>>>,
}

impl ViewManager {
    fn track(&self, index: i32, element: &Ref<Control>) {
        let mut map = self.by_index.borrow_mut();
        let entries = map.entry(index).or_default();
        if !entries
            .iter()
            .any(|entry| entry.upgrade().as_ref() == Some(element))
        {
            entries.push(element.downgrade());
        }
    }

    fn untrack(&self, index: i32, element: &Ref<Control>) {
        let mut map = self.by_index.borrow_mut();
        if let Some(entries) = map.get_mut(&index) {
            entries.retain(|entry| {
                entry
                    .upgrade()
                    .as_ref()
                    .is_some_and(|entry| entry != element)
            });
            if entries.is_empty() {
                map.remove(&index);
            }
        }
    }

    pub fn try_get_element(&self, index: i32, layout_only: bool) -> Option<Ref<Control>> {
        let candidates = self
            .by_index
            .borrow()
            .get(&index)
            .cloned()
            .unwrap_or_default();
        candidates
            .into_iter()
            .filter_map(|entry| entry.upgrade())
            .find(|element| {
                let info = ItemsRepeater::get_virtualization_info(element);
                info.index() == index
                    && if layout_only {
                        info.is_held_by_layout()
                    } else {
                        info.is_realized()
                    }
            })
    }

    pub fn get_element(
        &self,
        owner: &ItemsRepeater,
        index: i32,
        options: ElementRealizationOptions,
    ) -> Ref<Control> {
        let items = owner
            .items_source_view()
            .expect("ItemsSource doesn't have a value.");
        assert!(
            index >= 0 && (index as usize) < items.count(),
            "Argument index is invalid."
        );
        let mut element = if options.contains(ElementRealizationOptions::FORCE_CREATE) {
            None
        } else {
            self.try_get_element(index, true)
        };
        if element.is_none() {
            element = owner.viewport_manager.made_anchor().filter(|anchor| {
                let info = ItemsRepeater::get_virtualization_info(anchor);
                info.index() == index && info.is_realized()
            });
        }
        if element.is_none() && self.stable_reset_pending.get() && items.has_key_index_mapping() {
            element = self.reset_pool.remove(&items.key_from_index(index));
            if let Some(element) = &element {
                let info = ItemsRepeater::get_virtualization_info(element);
                info.move_ownership_to_layout_from_unique_id_reset_pool();
                self.update_element_index(owner, element, index);
            }
        }
        if element.is_none() {
            let pinned = self
                .pinned
                .borrow()
                .iter()
                .find(|element| {
                    // A handler can release the last pin during measure, before the pool is
                    // pruned again; such an element is no longer the pool's to hand out.
                    let info = ItemsRepeater::get_virtualization_info(element);
                    info.index() == index && info.is_pinned()
                })
                .cloned();
            if let Some(pinned) = pinned {
                element = Some(pinned);
            }
        }
        if let Some(element) = &element {
            let info = ItemsRepeater::get_virtualization_info(element);
            if info.is_in_pinned_pool() {
                self.pinned.borrow_mut().retain(|pinned| pinned != element);
                info.move_ownership_to_layout_from_pinned_pool();
            }
        }
        let element = element.unwrap_or_else(|| self.get_element_from_factory(owner, index));
        let info = ItemsRepeater::get_virtualization_info(&element);
        if options.contains(ElementRealizationOptions::SUPPRESS_AUTO_RECYCLE) {
            info.set_auto_recycle_candidate(false);
        } else {
            info.set_auto_recycle_candidate(true);
            info.set_keep_alive(true);
        }
        assert_eq!(info.owner(), ElementOwner::Layout);
        self.track(index, &element);
        super::diagnostics::element_lifecycle(owner, index, "realized", info.owner());
        element
    }

    fn get_element_from_factory(&self, owner: &ItemsRepeater, index: i32) -> Ref<Control> {
        let items = owner
            .items_source_view()
            .expect("ItemsSource doesn't have a value.");
        let data = items.get_at(index as usize);
        let data_control = data.as_ref().and_then(Control::from_boxed);
        let factory = owner.item_template_shim();
        let element = if factory.is_none() && data_control.is_some() {
            data_control.clone().expect("A control item was found.")
        } else {
            let factory = factory.unwrap_or_else(|| owner.ensure_default_factory());
            let args = ElementFactoryGetArgs::new(data.clone(), Some(owner.to_ref().upcast()));
            super::RepeaterTestHooks::set_element_factory_element_index(index);
            factory
                .get_element(&args)
                .unwrap_or_else(|error| panic!("{error}"))
        };
        let info = ItemsRepeater::get_virtualization_info(&element);
        assert_eq!(
            info.owner(),
            ElementOwner::ElementFactory,
            "An element factory returned a control which is already realized."
        );
        let key = if items.has_key_index_mapping() {
            items.key_from_index(index)
        } else {
            String::new()
        };
        info.move_ownership_to_layout_from_element_factory(index, key);
        info.set_must_clear_data_context(false);
        if data_control.as_ref() != Some(&element) {
            // Phase zero precedes DataContext, as in WinUI, so an x:Bind
            // component does not run every phase in a DataContext handler.
            let phased = Phaser::get_data_template_component(&element).is_some();
            if phased {
                Phaser::initialize_phase_zero(&element, &info, data.clone(), index);
            }
            if !phased {
                let context = data_control
                    .as_ref()
                    .and_then(|control| control.data_context())
                    .or_else(|| data.clone());
                element.set_data_context(context);
                info.set_must_clear_data_context(true);
            }
        }
        if let Some(parent) = element.visual_parent() {
            assert!(
                parent.ptr_eq(&owner.to_ref().upcast::<Visual>()),
                "An element factory returned an element owned by another parent."
            );
        } else {
            owner.children().add(&element);
        }
        self.track(index, &element);
        owner.viewport_manager.on_element_prepared(&element);
        owner.transition_manager().on_element_prepared(&element);
        owner.raise_element_prepared(&ItemsRepeaterElementPreparedEventArgs {
            element: element.clone(),
            index,
        });
        if data_control.as_ref() != Some(&element) {
            // ElementPrepared can install a phased binding component. Queue
            // deferred work only after that callback has returned.
            owner.phaser().phase_element(&element, &info);
        }
        element
    }

    pub fn clear_element(
        &self,
        owner: &ItemsRepeater,
        element: &Ref<Control>,
        collection_change: bool,
    ) {
        let info = ItemsRepeater::get_virtualization_info(element);
        if matches!(
            info.owner(),
            ElementOwner::ElementFactory | ElementOwner::Animator
        ) {
            return;
        }
        if info.is_in_unique_id_reset_pool() && self.stable_reset_pending.get() {
            return;
        }
        let index = info.index();
        if self.stable_reset_pending.get()
            && (info.is_held_by_layout() || info.is_in_pinned_pool())
        {
            self.reset_pool.add(info.unique_id(), element.clone());
            if info.is_in_pinned_pool() {
                self.pinned.borrow_mut().retain(|pinned| pinned != element);
                info.move_ownership_to_unique_id_reset_pool_from_pinned_pool();
            } else {
                info.move_ownership_to_unique_id_reset_pool_from_layout();
            }
            self.untrack(index, element);
        } else if !info.is_in_pinned_pool() && owner.transition_manager().clear_element(element) {
            self.untrack(index, element);
            self.pinned.borrow_mut().retain(|pinned| pinned != element);
            info.move_ownership_to_animator();
            Self::forget_tab_once_active_element(element);
            let navigation = Self::focus_navigation(owner);
            if self.release_focus_from_cleared_element(element) {
                self.move_focus_from_cleared_index(owner, element, index, navigation);
            }
        } else if !collection_change && info.is_pinned() {
            if !info.is_in_pinned_pool() {
                self.pinned.borrow_mut().push(element.clone());
                info.move_ownership_to_pinned_pool();
            }
        } else {
            self.clear_element_to_element_factory(owner, element);
            return;
        }
        super::diagnostics::element_lifecycle(owner, index, "retained", info.owner());
    }

    pub fn clear_element_to_element_factory(&self, owner: &ItemsRepeater, element: &Ref<Control>) {
        let info = ItemsRepeater::get_virtualization_info(element);
        if info.owner() == ElementOwner::ElementFactory {
            return;
        }
        owner.transition_manager().on_element_clearing(element);
        let index = info.index();
        // The event observes the original DataContext and source index.
        owner.raise_element_clearing(&ItemsRepeaterElementClearingEventArgs {
            element: element.clone(),
        });
        self.untrack(index, element);
        self.pinned.borrow_mut().retain(|pinned| pinned != element);
        owner.viewport_manager.on_element_cleared(element);
        owner.phaser().stop_phasing(element, &info);
        info.move_ownership_to_element_factory();
        Self::forget_tab_once_active_element(element);
        // Recycling factories and direct-control removal may detach the row.
        // Release its propagated focus pins while its ancestors are reachable,
        // but preserve event/recycle ordering before choosing replacement focus.
        let navigation = Self::focus_navigation(owner);
        let was_focused = self.release_focus_from_cleared_element(element);
        if info.must_clear_data_context() {
            element.set_data_context(None);
            info.set_must_clear_data_context(false);
        }
        if let Some(factory) = owner.item_template_shim() {
            factory
                .recycle_element(&ElementFactoryRecycleArgs::new(
                    element.clone(),
                    Some(owner.to_ref().upcast()),
                ))
                .unwrap_or_else(|error| panic!("{error}"));
        } else {
            owner.children().remove(element);
        }
        if was_focused {
            self.move_focus_from_cleared_index(owner, element, index, navigation);
        }
        super::diagnostics::element_lifecycle(owner, index, "recycled", info.owner());
    }

    pub fn prune_pinned_elements(&self, owner: &ItemsRepeater) {
        let expired: Vec<_> = self
            .pinned
            .borrow()
            .iter()
            .filter(|element| !ItemsRepeater::get_virtualization_info(element).is_pinned())
            .cloned()
            .collect();
        for element in expired {
            self.clear_element_to_element_factory(owner, &element);
        }
    }

    pub fn on_layout_changing(&self, owner: &ItemsRepeater) {
        if owner
            .items_source_view()
            .is_some_and(|items| items.has_key_index_mapping())
        {
            self.stable_reset_pending.set(true);
        }
    }

    pub fn on_owner_arranged(&self, owner: &ItemsRepeater) {
        if self.stable_reset_pending.replace(false) {
            let leftovers = self.reset_pool.take_all();
            for element in leftovers {
                self.clear_element(owner, &element, true);
            }
        }
        super::diagnostics::owner_arranged(owner);
    }

    pub fn on_template_changing(&self) {
        // Stable keys preserve identity only while the factory is unchanged.
        // Disable reuse before the layout receives Reset; clear_all drains
        // the retained pool after that callback releases layout references.
        self.stable_reset_pending.set(false);
    }

    pub fn clear_all(&self, owner: &ItemsRepeater) {
        self.stable_reset_pending.set(false);
        let _ = self.reset_pool.take_all();
        let children = owner.children().snapshot();
        for element in children.iter() {
            if ItemsRepeater::get_virtualization_info(element).owner()
                != ElementOwner::ElementFactory
            {
                self.clear_element_to_element_factory(owner, element);
            }
        }
        self.by_index.borrow_mut().clear();
        self.pinned.borrow_mut().clear();
    }

    fn update_element_index(&self, owner: &ItemsRepeater, element: &Ref<Control>, index: i32) {
        let info = ItemsRepeater::get_virtualization_info(element);
        let old_index = info.index();
        if old_index == index {
            self.track(index, element);
            return;
        }
        self.untrack(old_index, element);
        info.update_index(index);
        self.track(index, element);
        owner.raise_element_index_changed(&ItemsRepeaterElementIndexChangedEventArgs {
            element: element.clone(),
            old_index,
            new_index: index,
        });
    }

    pub fn on_items_source_changed(&self, owner: &ItemsRepeater, args: &ItemsChangedEventArgs<'_>) {
        use NotifyCollectionChangedAction::*;
        // No action needs the elements in source order, so they are not sorted.
        let elements = owner.realized_elements_unsorted();
        match args.action {
            Add => {
                // An insertion after every realized element (an append past the realized range)
                // changes no index, as WinUI visits only the pinned pool then.
                if elements.iter().all(|element| {
                    ItemsRepeater::get_virtualization_info(element).index() < args.new_starting_index
                }) {
                    return;
                }
                for element in elements {
                    let index = ItemsRepeater::get_virtualization_info(&element).index();
                    if index >= args.new_starting_index {
                        self.update_element_index(
                            owner,
                            &element,
                            index + args.new_items.len() as i32,
                        );
                    }
                }
            }
            Remove | Replace => {
                let start = args.old_starting_index;
                let count = args.old_items.len() as i32;
                let added = if args.action == Replace {
                    args.new_items.len() as i32
                } else {
                    0
                };
                if args.action == Replace {
                    assert_eq!(
                        start, args.new_starting_index,
                        "Replace requires matching old and new indices."
                    );
                    assert!(
                        count > 0 && added > 0,
                        "Replace requires nonempty old and new item ranges."
                    );
                }
                for element in elements {
                    let info = ItemsRepeater::get_virtualization_info(&element);
                    let index = info.index();
                    if index >= start
                        && index < start + count
                        // Deviation (winui-repeater/DEVIATIONS.md, WR-29): pinned-pool rows are
                        // retired whatever the old layout flag says.
                        && (info.is_in_pinned_pool()
                            || (args.action == Remove && info.auto_recycle_candidate()))
                    {
                        // A layout may suppress auto-recycling while it owns
                        // the element, then release it into the pinned pool.
                        // That layout no longer tracks this row and cannot
                        // retire it when its source item is removed/replaced.
                        owner.clear_element_impl(&element);
                    } else if index >= start + count {
                        self.update_element_index(owner, &element, index + added - count);
                    }
                }
            }
            // Deviation (winui-repeater/DEVIATIONS.md, WR-34): WinUI has no Move case.
            Move => {
                // FerroUI lists report Move directly. Preserve element identity
                // and adjust the intervening range without creating controls.
                let old = args.old_starting_index;
                let new = args.new_starting_index;
                let count = args.old_items.len() as i32;
                for element in elements {
                    let index = ItemsRepeater::get_virtualization_info(&element).index();
                    let target = if index >= old && index < old + count {
                        new + index - old
                    } else if old < new && index >= old + count && index < new + count {
                        index - count
                    } else if new < old && index >= new && index < old {
                        index + count
                    } else {
                        index
                    };
                    if target != index {
                        self.update_element_index(owner, &element, target);
                    }
                }
            }
            Reset => {
                if self.stable_reset_pending.get() {
                    return;
                }
                self.stable_reset_pending.set(
                    owner
                        .items_source_view()
                        .is_some_and(|items| items.has_key_index_mapping()),
                );
                for element in elements {
                    let info = ItemsRepeater::get_virtualization_info(&element);
                    if info.auto_recycle_candidate() || info.is_in_pinned_pool() {
                        owner.clear_element_impl(&element);
                    }
                }
            }
        }
    }

    pub fn update_pin(element: &Ref<Control>, add: bool) {
        let mut child = element.clone().upcast::<Visual>();
        while let Some(parent) = child.visual_parent() {
            if let (Some(repeater), Some(control)) =
                (parent.cast::<ItemsRepeater>(), child.cast::<Control>())
            {
                let info = ItemsRepeater::get_virtualization_info(&control);
                if info.is_realized() {
                    if add {
                        info.add_pin();
                    } else if info.is_pinned() && info.remove_pin() == 0 {
                        repeater.invalidate_measure();
                    }
                } else if !add && info.is_in_unique_id_reset_pool() && info.is_pinned() {
                    // A keyed reset keeps a row's pins in the reset pool (WR-29); the pin is
                    // released there too, or the row stays pinned once it is reused.
                    info.remove_reset_pool_pin();
                }
            }
            child = parent;
        }
    }

    pub fn update_focus(&self, owner: &ItemsRepeater, focused: Option<&Ref<InputElement>>) {
        // Only the nearest repeater owns the focus notification. UpdatePin
        // already propagates its pin through every containing repeater; also
        // tracking focus in outer hosts would pin the same ancestor twice.
        let next = focused.and_then(|focused| {
            let mut child = focused.clone().upcast::<Visual>();
            while let Some(parent) = child.visual_parent() {
                if let Some(repeater) = parent.cast::<ItemsRepeater>() {
                    if repeater.ptr_eq(&owner.to_ref()) {
                        return child.cast::<Control>().filter(|element| {
                            ItemsRepeater::get_virtualization_info(element).is_realized()
                        });
                    }
                    return None;
                }
                child = parent;
            }
            None
        });
        // Native Once navigation returns to the most recently focused
        // descendant. Local, Cycle and Contained navigation must enter in
        // source order instead of jumping to this remembered target.
        if KeyboardNavigation::get_tab_navigation(owner) != KeyboardNavigationMode::Once {
            KeyboardNavigation::set_tab_once_active_element(owner, None);
        } else if focused.is_some_and(|focused| {
            owner
                .immediate_child(&focused.clone().upcast())
                .is_some_and(|element| {
                    ItemsRepeater::get_virtualization_info(&element).is_realized()
                })
        }) {
            // Once history includes descendants of nested repeaters. It is
            // independent of nearest-owner pin tracking, and also changes
            // when focus moves within the same item without changing pins.
            KeyboardNavigation::set_tab_once_active_element(owner, focused.cloned());
        }
        let previous = self.focused.borrow().as_ref().and_then(WeakRef::upgrade);
        if previous == next {
            return;
        }
        *self.focused.borrow_mut() = next.as_ref().map(Ref::downgrade);
        if let Some(previous) = previous {
            Self::update_pin(&previous, false);
        }
        if let Some(next) = next {
            Self::update_pin(&next, true);
        }
    }

    fn forget_tab_once_active_element(element: &Ref<Control>) {
        let element = element.clone().upcast::<Visual>();
        let mut parent = element.visual_parent();
        while let Some(ancestor) = parent {
            if let Some(repeater) = ancestor.cast::<ItemsRepeater>() {
                if let Some(active) = KeyboardNavigation::get_tab_once_active_element(&repeater) {
                    let active = active.upcast::<Visual>();
                    if element.ptr_eq(&active) || element.is_visual_ancestor_of(&active) {
                        // An outer Once group may remember this same nested
                        // leaf. Clear every such record before focus fallback
                        // selects a replacement; pooled and animating rows
                        // remain attached but no longer participate in focus.
                        KeyboardNavigation::set_tab_once_active_element(&repeater, None);
                    }
                }
            }
            parent = ancestor.visual_parent();
        }
    }

    fn release_focus_from_cleared_element(&self, element: &Ref<Control>) -> bool {
        if self.release_tracked_focus(element) {
            return true;
        }
        // A focused descendant of a nested repeater is tracked only by that
        // nearest repeater. Clearing an enclosing row must release its pin
        // there, while the visual path to every containing repeater remains
        // intact. Native detach notifications arrive after that path is cut.
        let Some(focused) = FocusManager::get_focus_manager(element)
            .and_then(|manager| manager.get_focused_element())
        else {
            return false;
        };
        let mut child = focused.upcast::<Visual>();
        if !element.clone().upcast::<Visual>().ptr_eq(&child)
            && !element.is_visual_ancestor_of(&child)
        {
            return false;
        }
        while let Some(parent) = child.visual_parent() {
            if let Some(repeater) = parent.cast::<ItemsRepeater>() {
                if let Some(row) = child.cast::<Control>() {
                    repeater.view_manager.release_tracked_focus(&row);
                }
                break;
            }
            child = parent;
        }
        true
    }

    fn release_tracked_focus(&self, element: &Ref<Control>) -> bool {
        if self
            .focused
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .as_ref()
            != Some(element)
        {
            return false;
        }
        // Clear tracking first so native focus notifications caused by
        // detachment or a factory callback cannot release this pin twice.
        *self.focused.borrow_mut() = None;
        // A terminal ownership change already reset the removed row's pin
        // counter. A nested focused row and the other containing realized
        // rows still hold their propagated focus pin.
        Self::update_pin(element, false);
        true
    }

    /// How the focused element got its focus, as far as it shows: `Tab` for a keyboard focus
    /// (`:focus-visible`), otherwise `Unspecified`. WinUI passes the previous element's
    /// `FocusState` on to the replacement (`ViewManager::EnsureFocusedElement`).
    fn focus_navigation(owner: &ItemsRepeater) -> NavigationMethod {
        let keyboard = FocusManager::get_focus_manager(owner)
            .and_then(|manager| manager.get_focused_element())
            .is_some_and(|focused| focused.classes().contains(":focus-visible"));
        if keyboard {
            NavigationMethod::Tab
        } else {
            NavigationMethod::Unspecified
        }
    }

    // Deviation (winui-repeater/DEVIATIONS.md, WR-35): the wider search for the replacement.
    fn move_focus_from_cleared_index(
        &self,
        owner: &ItemsRepeater,
        element: &Ref<Control>,
        index: i32,
        navigation: NavigationMethod,
    ) {
        // A recycling callback may have moved focus deliberately. Only choose
        // a fallback when focus is still in the removed row or was cleared by
        // its detachment; never replace a newly selected target.
        if let Some(focused) =
            FocusManager::get_focus_manager(owner).and_then(|manager| manager.get_focused_element())
        {
            let focused = focused.upcast::<Visual>();
            if !element.clone().upcast::<Visual>().ptr_eq(&focused)
                && !element.is_visual_ancestor_of(&focused)
            {
                return;
            }
        }
        let mut candidates = owner.realized_elements();
        candidates.retain(|candidate| {
            candidate != element
                && ItemsRepeater::get_virtualization_info(candidate).is_held_by_layout()
        });
        candidates.sort_by_key(|candidate| {
            let candidate_index = ItemsRepeater::get_virtualization_info(candidate).index();
            (
                candidate_index < index,
                (candidate_index - index).unsigned_abs(),
            )
        });
        for candidate in candidates {
            let focusable = ItemsRepeater::focusable_descendants(&candidate);
            let focus = if owner.get_element_index(&candidate) < index {
                focusable.last()
            } else {
                focusable.first()
            };
            if let Some(focus) = focus {
                if focus.focus_with(navigation, KeyModifiers::NONE) {
                    return;
                }
            }
        }
        // A factory can keep the removed subtree attached for reuse. With no
        // surviving focus target, native detachment will not clear its focus.
        // Preserve any redirect made by a focus callback during the search.
        if let Some(manager) = FocusManager::get_focus_manager(owner) {
            if let Some(focused) = manager.get_focused_element() {
                let focused = focused.upcast::<Visual>();
                if element.clone().upcast::<Visual>().ptr_eq(&focused)
                    || element.is_visual_ancestor_of(&focused)
                {
                    manager.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
                }
            }
        }
    }
}
