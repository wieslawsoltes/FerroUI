// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use super::{ElementOwner, ItemsRepeater, ItemsRepeaterScrollHost};
use ferroui_controls::as_scroll_anchor_provider;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::{Control, ScrollViewer};
use ferroui_base::layout::Layoutable;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Matrix, Point, Rect, Ref, Size, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Effective viewport, gradual cache growth, and scroll-anchor registration.
/// The state lives with the control; queued work holds only a weak reference.
pub(crate) struct ViewportManager {
    scroller: RefCell<Option<WeakRef<Visual>>>,
    visible: Cell<Rect>,
    native_viewport: Cell<Rect>,
    viewport_received: Cell<bool>,
    viewport_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    transform_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    attached: Cell<bool>,
    last_realization: Cell<Rect>,
    extent: Cell<Rect>,
    expected_shift: Cell<Point>,
    pending_shift: Cell<Point>,
    unshiftable_shift: Cell<Point>,
    horizontal_cache: Cell<f64>,
    vertical_cache: Cell<f64>,
    cache_pending: Cell<bool>,
    virtualizing: Cell<bool>,
    prepared: RefCell<Vec<WeakRef<Control>>>,
    arranged: RefCell<Vec<WeakRef<Control>>>,
    anchor: RefCell<Option<WeakRef<Control>>>,
    anchor_outside: Cell<bool>,
    anchor_release_pending: Cell<bool>,
    #[cfg(test)]
    test_viewport: Cell<Option<Rect>>,
}

impl Default for ViewportManager {
    fn default() -> Self {
        Self {
            scroller: RefCell::new(None),
            visible: Cell::new(Rect::default()),
            native_viewport: Cell::new(Rect::default()),
            viewport_received: Cell::new(false),
            viewport_subscription: RefCell::new(None),
            transform_subscription: RefCell::new(None),
            attached: Cell::new(false),
            last_realization: Cell::new(Rect::default()),
            extent: Cell::new(Rect::default()),
            expected_shift: Cell::new(Point::default()),
            pending_shift: Cell::new(Point::default()),
            unshiftable_shift: Cell::new(Point::default()),
            horizontal_cache: Cell::new(0.0),
            vertical_cache: Cell::new(0.0),
            cache_pending: Cell::new(false),
            virtualizing: Cell::new(true),
            prepared: RefCell::new(Vec::new()),
            arranged: RefCell::new(Vec::new()),
            anchor: RefCell::new(None),
            anchor_outside: Cell::new(false),
            anchor_release_pending: Cell::new(false),
            #[cfg(test)]
            test_viewport: Cell::new(None),
        }
    }
}

impl ViewportManager {
    fn scroller(&self) -> Option<Ref<Visual>> {
        self.scroller.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn has_viewport(&self) -> bool {
        self.viewport_received.get() || self.has_scroller() || self.attached.get()
    }

    fn has_scroller(&self) -> bool {
        #[cfg(test)]
        if self.test_viewport.get().is_some() {
            return true;
        }
        self.scroller().is_some()
    }

    pub fn made_anchor(&self) -> Option<Ref<Control>> {
        self.anchor.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn suggested_anchor(&self, owner: &ItemsRepeater) -> Option<Ref<Control>> {
        if let Some(anchor) = self.made_anchor() {
            return Some(anchor);
        }
        let scroller = self.scroller()?;
        let anchor = as_scroll_anchor_provider(&scroller)?.current_anchor()?;
        owner.immediate_child(&anchor.clone().upcast())
    }

    pub fn visible_window(&self) -> Rect {
        if !self.virtualizing.get() {
            return Rect::from_size(Size::INFINITY);
        }
        if !self.has_viewport() {
            // Deviation (winui-repeater/DEVIATIONS.md, WR-11).
            // Direct measurement of an unattached FerroUI control has no
            // effective viewport yet. A finite unbounded window realizes its
            // contents without using the infinity sentinel reserved for the
            // fixed-children layout-context adapter.
            return Rect::new(0.0, 0.0, f32::MAX as f64, f32::MAX as f64);
        }
        let visible = self.visible.get();
        if self.made_anchor().is_some() && self.anchor_outside.get() {
            return Rect::new(0.0, 0.0, visible.width, visible.height);
        }
        if !self.has_scroller() {
            return visible;
        }
        let extent = self.extent.get();
        let expected = self.expected_shift.get();
        let unshiftable = self.unshiftable_shift.get();
        Rect::new(
            visible.x + extent.x + expected.x + unshiftable.x,
            visible.y + extent.y + expected.y + unshiftable.y,
            visible.width,
            visible.height,
        )
    }

    pub fn realization_window(&self) -> Rect {
        let visible = self.visible_window();
        if !self.virtualizing.get() || !self.has_scroller() {
            return visible;
        }
        let x = self.horizontal_cache.get();
        let y = self.vertical_cache.get();
        Rect::new(
            visible.x - x,
            visible.y - y,
            visible.width + x * 2.0,
            visible.height + y * 2.0,
        )
    }

    pub fn attach(&self, owner: &ItemsRepeater) {
        self.attached.set(owner.is_attached_to_visual_tree());
        let mut visual = owner.visual_parent();
        let mut provider = None;
        while let Some(parent) = visual {
            if as_scroll_anchor_provider(&parent).is_some() {
                provider = Some(parent);
                break;
            }
            visual = parent.visual_parent();
        }
        if self.scroller() != provider {
            self.unregister_candidates(owner, None);
            self.prepared.borrow_mut().clear();
            self.arranged.borrow_mut().clear();
            if provider.is_some() {
                // Reparenting preserves layout-owned controls, so they need
                // not pass through ElementPrepared again. Queue them for the
                // new provider after their bounds have been arranged in its
                // coordinate space, just like newly realized controls.
                for child in owner.children().snapshot().iter() {
                    if ItemsRepeater::get_virtualization_info(child).owner() == ElementOwner::Layout
                    {
                        self.on_element_prepared(child);
                    }
                }
            }
        }
        *self.scroller.borrow_mut() = provider.as_ref().map(Ref::downgrade);
    }

    pub fn detach(&self, owner: &ItemsRepeater) {
        self.attached.set(false);
        self.unregister_candidates(owner, None);
        *self.scroller.borrow_mut() = None;
        self.prepared.borrow_mut().clear();
        self.arranged.borrow_mut().clear();
        *self.anchor.borrow_mut() = None;
        self.anchor_outside.set(false);
        self.anchor_release_pending.set(false);
        self.reset_cache();
        // As WinUI (`OnUnloaded`: `ResetScrollers` and the cache reset), the visible window,
        // the native viewport and the layout extent are kept: a repeater attached again after
        // scrolling realizes the items it had instead of starting from the origin.
        self.expected_shift.set(Point::default());
        self.pending_shift.set(Point::default());
        self.unshiftable_shift.set(Point::default());
    }

    pub fn on_layout_changed(&self, owner: &ItemsRepeater, virtualizing: bool) {
        self.virtualizing.set(virtualizing);
        if virtualizing && self.viewport_subscription.borrow().is_none() {
            // A newly virtualizing layout needs a fresh native viewport even
            // when the geometry is unchanged since the nonvirtualizing pass.
            let weak = owner.to_ref().downgrade();
            let subscription = owner.effective_viewport_changed(move |args| {
                if let Some(owner) = weak.upgrade() {
                    owner
                        .viewport_manager
                        .on_effective_viewport_changed(&owner, args.effective_viewport());
                }
            });
            *self.viewport_subscription.borrow_mut() = Some(subscription);
        } else if !virtualizing {
            let subscription = self.viewport_subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            self.viewport_received.set(false);
            self.visible.set(Rect::default());
            self.native_viewport.set(Rect::default());
        }
        self.extent.set(Rect::default());
        self.expected_shift.set(Point::default());
        self.pending_shift.set(Point::default());
        self.unshiftable_shift.set(Point::default());
        self.reset_cache();
    }

    pub fn reset_cache(&self) {
        self.horizontal_cache.set(0.0);
        self.vertical_cache.set(0.0);
        self.last_realization.set(Rect::default());
    }

    pub fn on_element_prepared(&self, element: &Ref<Control>) {
        let mut prepared = self.prepared.borrow_mut();
        if !prepared
            .iter()
            .any(|item| item.upgrade().as_ref() == Some(element))
        {
            prepared.push(element.downgrade());
        }
    }

    pub fn on_element_cleared(&self, element: &Ref<Control>) {
        self.prepared
            .borrow_mut()
            .retain(|item| item.upgrade().as_ref().is_some_and(|item| item != element));
        self.arranged
            .borrow_mut()
            .retain(|item| item.upgrade().as_ref().is_some_and(|item| item != element));
        if let Some(scroller) = self.scroller() {
            if let Some(provider) = as_scroll_anchor_provider(&scroller) {
                provider.unregister_anchor_candidate(element);
            }
        }
        if self.made_anchor().as_ref() == Some(element) {
            *self.anchor.borrow_mut() = None;
            self.anchor_outside.set(false);
            // The other prepared items become candidates again at the next measure, as when
            // the anchor is released after a bring-into-view.
            let prepared = self.prepared.borrow().clone();
            let mut arranged = self.arranged.borrow_mut();
            for element in prepared {
                if !arranged.iter().any(|item| item.ptr_eq(&element)) {
                    arranged.push(element);
                }
            }
        }
    }

    pub fn on_owner_measuring(&self, owner: &ItemsRepeater) {
        if self.scroller().is_none() {
            self.attach(owner);
        }
        let current = self.realization_window();
        let previous = self.last_realization.get();
        if (self.horizontal_cache.get() != 0.0 || self.vertical_cache.get() != 0.0)
            && (previous.width <= 0.0 || previous.height <= 0.0 || !previous.intersects(current))
        {
            self.reset_cache();
        }
        self.last_realization.set(self.realization_window());
        self.register_arranged_candidates();
    }

    fn register_arranged_candidates(&self) {
        let arranged = std::mem::take(&mut *self.arranged.borrow_mut());
        if let Some(scroller) = self.scroller() {
            if let Some(provider) = as_scroll_anchor_provider(&scroller) {
                let anchor = self.made_anchor();
                let mut deferred = Vec::new();
                for element in arranged.into_iter().filter_map(|element| element.upgrade()) {
                    if ItemsRepeater::get_virtualization_info(&element).owner()
                        != ElementOwner::Layout
                    {
                        continue;
                    }
                    if anchor.as_ref().is_none_or(|anchor| anchor == &element) {
                        provider.register_anchor_candidate(&element);
                    } else {
                        // Registered once the made anchor is released, not lost.
                        deferred.push(element.downgrade());
                    }
                }
                self.arranged.borrow_mut().extend(deferred);
            }
        }
    }

    pub fn layout_extent(&self) -> Rect {
        self.extent.get()
    }

    pub fn set_layout_extent(&self, extent: Rect) {
        let old = self.extent.replace(extent);
        let shift = self.expected_shift.get();
        let shift = Point::new(shift.x + old.x - extent.x, shift.y + old.y - extent.y);
        self.expected_shift.set(shift);
        self.pending_shift.set(shift);
        // Anchor registrations can change even when the extent does not.
        if let Some(scroller) = self
            .scroller()
            .and_then(|visual| visual.cast::<Layoutable>())
        {
            scroller.invalidate_arrange();
        }
    }

    pub fn on_owner_arranged(&self, owner: &ItemsRepeater) {
        let prepared = std::mem::take(&mut *self.prepared.borrow_mut());
        self.arranged.borrow_mut().extend(prepared);
        // Native scroll presenters select their anchor after child arrange.
        // With a zero-length cache there need not be another measure before
        // scrolling, so publish the now-valid candidates in this same pass.
        self.register_arranged_candidates();
        self.expected_shift.set(Point::default());
        let unshiftable = self.unshiftable_shift.get();
        if unshiftable != Point::default() {
            let offset = self
                .scroller()
                .and_then(|scroller| {
                    scroller
                        .cast::<ScrollViewer>()
                        .map(|scroller| scroller.offset())
                        .or_else(|| {
                            scroller
                                .cast::<ScrollContentPresenter>()
                                .map(|scroller| scroller.offset())
                        })
                        .or_else(|| {
                            scroller
                                .cast::<ItemsRepeaterScrollHost>()
                                .and_then(|host| host.scroll_viewer())
                                .map(|viewer| viewer.offset())
                        })
                })
                .unwrap_or_default();
            let visible = self.visible.get();
            if (unshiftable.x != 0.0 && (visible.x - offset.x).abs() < 1.0)
                || (unshiftable.y != 0.0 && (visible.y - offset.y).abs() < 1.0)
            {
                self.unshiftable_shift.set(Point::default());
                self.try_invalidate_measure(owner);
            }
        }
        if !self.virtualizing.get() || !self.has_scroller() {
            return;
        }
        let visible = self.visible.get();
        let max_x = owner.horizontal_cache_length() * visible.width / 2.0;
        let max_y = owner.vertical_cache_length() * visible.height / 2.0;
        if self.horizontal_cache.get() < max_x || self.vertical_cache.get() < max_y {
            // WinUI grows by 40 DIPs per side and schedules another layout
            // even when this step reaches the configured cache limit.
            self.horizontal_cache
                .set((self.horizontal_cache.get() + 40.0).min(max_x));
            self.vertical_cache
                .set((self.vertical_cache.get() + 40.0).min(max_y));
            if !self.cache_pending.replace(true) {
                let weak = owner.to_ref().downgrade();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(owner) = weak.upgrade() {
                            owner.viewport_manager.cache_pending.set(false);
                            if owner.viewport_manager.virtualizing.get() {
                                owner.viewport_manager.try_invalidate_measure(&owner);
                            }
                        }
                    },
                    DispatcherPriority::BACKGROUND,
                );
            }
        }
    }

    pub fn on_layout_updated(&self, owner: &ItemsRepeater) {
        let pending = self.pending_shift.replace(Point::default());
        // Deviation (winui-repeater/DEVIATIONS.md, WR-36): any nonzero shift, without WinUI's
        // 1-pixel tolerance.
        if self.virtualizing.get() && pending != Point::default() {
            let old = self.unshiftable_shift.get();
            self.unshiftable_shift
                .set(Point::new(old.x + pending.x, old.y + pending.y));
            self.expected_shift.set(Point::default());
            self.try_invalidate_measure(owner);
        }
        if self.anchor_release_pending.replace(false) {
            *self.anchor.borrow_mut() = None;
            if self.anchor_outside.replace(false) {
                self.try_invalidate_measure(owner);
            }
            // FerroUI scroll offsets are committed during layout. This is
            // the frame boundary corresponding to WinUI's Rendering event.
            if let Some(scroller) = self.scroller() {
                if let Some(provider) = as_scroll_anchor_provider(&scroller) {
                    for element in owner.realized_elements() {
                        if ItemsRepeater::get_virtualization_info(&element).is_held_by_layout() {
                            provider.register_anchor_candidate(&element);
                        }
                    }
                }
            }
        }
    }

    pub fn on_effective_viewport_changed(&self, owner: &ItemsRepeater, viewport: Rect) {
        if !self.virtualizing.get() {
            return;
        }
        self.viewport_received.set(true);
        self.native_viewport.set(viewport);
        let viewport = if viewport.x >= 10000.0 && viewport.y >= 10000.0 {
            Rect::default()
        } else if let Some(transform) = owner.render_transform() {
            // Native effective viewports include ancestor transforms and
            // deliberately exclude the receiving visual's own transform.
            // Repeater layouts consume item-space coordinates, including a
            // LayoutTransformControl's matrix installed on this visual.
            transform
                .value()
                .try_invert()
                .map_or(Rect::default(), |inverse| {
                    let origin = owner
                        .render_transform_origin()
                        .to_pixels(owner.bounds().size());
                    let offset = Matrix::create_translation(origin.x, origin.y);
                    viewport.transform_to_aabb(-offset * inverse * offset)
                })
        } else {
            viewport
        };
        let old = self.visible.replace(viewport);
        let pending = self.pending_shift.replace(Point::default());
        let unshiftable = self.unshiftable_shift.replace(Point::default());
        if viewport == Rect::default() {
            self.extent.set(Rect::default());
        }
        if (old.x - viewport.x).abs() > 0.01
            || (old.y - viewport.y).abs() > 0.01
            || (old.width - viewport.width).abs() > 0.01
            || (old.height - viewport.height).abs() > 0.01
            || pending != Point::default()
            || unshiftable != Point::default()
        {
            self.try_invalidate_measure(owner);
        }
    }

    pub fn refresh_viewport(&self, owner: &ItemsRepeater) {
        if self.viewport_received.get() {
            self.on_effective_viewport_changed(owner, self.native_viewport.get());
        }
    }

    pub fn on_render_transform_changed(&self, owner: &ItemsRepeater) {
        let previous = self.transform_subscription.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }
        if let Some(transform) = owner.render_transform() {
            if let Some(mutable) = transform.as_mutable_transform() {
                let weak = owner.to_ref().downgrade();
                let subscription = mutable.changed(Rc::new(move || {
                    if let Some(owner) = weak.upgrade() {
                        owner.viewport_manager.refresh_viewport(&owner);
                    }
                }));
                *self.transform_subscription.borrow_mut() = Some(subscription);
            }
        }
        self.refresh_viewport(owner);
    }

    pub fn make_anchor(&self, owner: &ItemsRepeater, element: &Ref<Control>, outside: bool) {
        self.anchor_release_pending.set(false);
        *self.anchor.borrow_mut() = Some(element.downgrade());
        self.anchor_outside.set(outside);
        self.unregister_candidates(owner, Some(element));
        if let Some(scroller) = self.scroller() {
            if let Some(provider) = as_scroll_anchor_provider(&scroller) {
                if element.is_arrange_valid() {
                    provider.register_anchor_candidate(element);
                }
            }
        }
    }

    pub fn on_bring_into_view(&self, owner: &ItemsRepeater, target: &Ref<Visual>) {
        if let Some(element) = owner.immediate_child(target) {
            self.unregister_candidates(owner, Some(&element));
            self.anchor_release_pending.set(true);
            owner.invalidate_arrange();
        }
    }

    fn try_invalidate_measure(&self, owner: &ItemsRepeater) {
        if self.visible.get() != Rect::default() {
            owner.invalidate_measure();
        }
    }

    fn unregister_candidates(&self, owner: &ItemsRepeater, except: Option<&Ref<Control>>) {
        if let Some(scroller) = self.scroller() {
            if let Some(provider) = as_scroll_anchor_provider(&scroller) {
                for child in owner.children().snapshot().iter() {
                    if except != Some(child) {
                        provider.unregister_anchor_candidate(child);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    pub fn set_test_viewport(&self, viewport: Rect) {
        self.test_viewport.set(Some(viewport));
        self.visible.set(viewport);
    }
}

impl Drop for ViewportManager {
    fn drop(&mut self) {
        for subscription in [
            self.viewport_subscription.get_mut().take(),
            self.transform_subscription.get_mut().take(),
        ]
        .into_iter()
        .flatten()
        {
            subscription.dispose();
        }
    }
}
