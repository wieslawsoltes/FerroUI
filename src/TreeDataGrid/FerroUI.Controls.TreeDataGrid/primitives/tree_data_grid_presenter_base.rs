//! `Primitives/TreeDataGridPresenterBase.cs`: the virtualization shared by the presenters of the
//! control: viewport estimation, the anchor element, realization in both directions from the
//! anchor, recycling, the focused element retained outside the viewport, bringing an item into view
//! and the updates of the realized elements on collection changes.
//!
//! The reference's `TreeDataGridPresenterBase<TItem> : Border` is an abstract generic class. In
//! Rust the state it keeps is [`TreeDataGridPresenterBase`], which every presenter class
//! (`ferro_class!` classes derived from `Border`) embeds, and its abstract and virtual members are
//! the members of [`TreeDataGridPresenterHooks`], which every presenter implements. The base
//! algorithms are the methods of [`TreeDataGridPresenterBase`] that take the presenter as its hooks;
//! the presenters call them from their layout and tree attachment overrides.

use super::{IFinalMeasureSelector, RealizedStackElements, TreeDataGridElementFactory};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::layout::{EffectiveViewportChangedEventArgs, Orientation};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{FerroObject, ObjectType, Rect, Ref, Size, StyledElement, Upcast, Visual, WeakRef};
use ferroui_controls::{as_scroll_anchor_provider, Control};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The viewport of a presenter that has not received an effective viewport yet.
pub(crate) const INVALID_VIEWPORT: Rect = Rect::new(f64::INFINITY, f64::INFINITY, 0.0, 0.0);

/// A change of the items of a presenter: what the base needs of the reference's
/// `NotifyCollectionChangedEventArgs`, whatever the type of the items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresenterItemsChange {
    pub action: NotifyCollectionChangedAction,
    pub new_starting_index: i32,
    pub new_count: i32,
    pub old_starting_index: i32,
    pub old_count: i32,
}

impl PresenterItemsChange {
    /// The reset of the items (`CollectionExtensions.ResetEvent`).
    pub fn reset() -> Self {
        Self {
            action: NotifyCollectionChangedAction::Reset,
            new_starting_index: -1,
            new_count: 0,
            old_starting_index: -1,
            old_count: 0,
        }
    }

    /// The change described by the arguments of a collection change.
    pub fn from_args<T>(e: &NotifyCollectionChangedEventArgs<'_, T>) -> Self {
        Self {
            action: e.action,
            new_starting_index: e.new_starting_index,
            new_count: e.new_items.len() as i32,
            old_starting_index: e.old_starting_index,
            old_count: e.old_items.len() as i32,
        }
    }
}

/// The abstract and virtual members of the reference's presenter base, implemented by each
/// presenter class. The defaults are the base implementations.
pub trait TreeDataGridPresenterHooks: ObjectType + Upcast<Control> + 'static {
    /// The type of the items the presenter shows.
    type Item: Clone + 'static;

    /// The state of the base.
    fn presenter_base(&self) -> &TreeDataGridPresenterBase;

    /// The presenter as a control.
    #[inline]
    fn presenter_control(&self) -> &Control {
        Upcast::<Control>::upcast(self)
    }

    /// The axis along which the items are laid out.
    fn orientation(&self) -> Orientation;

    /// The number of items, or `None` when the presenter has no items (`Items is null`).
    fn item_count(&self) -> Option<usize>;

    /// The item at `index`.
    fn item_at(&self, index: usize) -> Self::Item;

    /// Subscribes to the changes of the items; `None` when the items do not notify.
    fn subscribe_to_items(&self, handler: Rc<dyn Fn(&PresenterItemsChange)>) -> Option<u64>;

    /// Removes a subscription of [`subscribe_to_items`](Self::subscribe_to_items).
    fn unsubscribe_from_items(&self, token: u64);

    /// Prepares `element` to show `item`, at `index` (`RealizeElement`).
    fn realize_element(&self, element: &Ref<Control>, item: &Self::Item, index: i32);

    /// The index of a realized element changed (`UpdateElementIndex`).
    fn update_element_index(&self, element: &Ref<Control>, old_index: i32, new_index: i32);

    /// Clears `element` before it is recycled (`UnrealizeElement`).
    fn unrealize_element(&self, element: &Ref<Control>);

    /// Clears `element` before it is recycled because its item was removed
    /// (`UnrealizeElementOnItemRemoved`).
    fn unrealize_element_on_item_removed(&self, element: &Ref<Control>) {
        self.unrealize_element(element);
    }

    /// Releases what a recycled element retained when it is no longer needed for a quick reuse
    /// (`FinalizeRecycledElement`).
    fn finalize_recycled_element(&self, _element: &Ref<Control>) {}

    /// Arranges `element` (`ArrangeElement`); returns the rect it was arranged in.
    fn arrange_element(&self, _index: i32, element: &Ref<Control>, rect: Rect) -> Rect {
        element.arrange(rect);
        rect
    }

    /// Measures `element` (`MeasureElement`); returns its size.
    fn measure_element(&self, _index: i32, element: &Ref<Control>, available_size: Size) -> Size {
        TreeDataGridPresenterBase::default_measure_element(element, available_size)
    }

    /// Gets the initial constraint for the first pass of the two-pass measure
    /// (`GetInitialConstraint`).
    ///
    /// The measure pass is split into two parts:
    ///
    /// - The initial pass is used to determine the "natural" size of the elements. In this pass,
    ///   infinity can be used as the measure constraint if the element has no other constraints on
    ///   its size.
    /// - The final pass is made once the "natural" sizes of the elements are known and any layout
    ///   logic has been run. This pass is needed because controls should not be arranged with a size
    ///   less than that passed as the constraint during the measure pass. This pass is only run if
    ///   [`needs_final_measure_pass`](Self::needs_final_measure_pass) returns true.
    fn get_initial_constraint(&self, _element: &Ref<Control>, _index: i32, available_size: Size) -> Size {
        available_size
    }

    /// Called when the initial pass of the two-pass measure has been completed, in order to
    /// determine whether a final measure pass is necessary (`NeedsFinalMeasurePass`).
    /// `first_index` is the index of the first element in `elements`.
    fn needs_final_measure_pass(&self, _first_index: i32, _elements: &[Option<Ref<Control>>]) -> bool {
        false
    }

    /// Gets the final constraint for the second pass of the two-pass measure
    /// (`GetFinalConstraint`).
    fn get_final_constraint(&self, element: &Ref<Control>, _index: i32, _available_size: Size) -> Size {
        element.desired_size()
    }

    /// Gets the element that shows `item` from the element factory (`GetElementFromFactory`).
    fn get_element_from_factory(&self, item: &Self::Item, _index: i32) -> Ref<Control> {
        self.presenter_base().get_element_from_factory_for(item as &dyn Any, self.presenter_control())
    }

    /// The index and position of the element at `position` (`GetElementAt`).
    fn get_element_at(&self, _position: f64) -> (i32, f64) {
        (-1, -1.0)
    }

    /// The position of the element at `index` (`GetElementPosition`).
    fn get_element_position(&self, _index: i32) -> f64 {
        -1.0
    }

    /// The estimated size of all the items on the primary axis (`CalculateSizeU`).
    fn calculate_size_u(&self, _available_size: Size) -> f64 {
        match self.item_count() {
            // Return the estimated size of all items based on the elements currently realized.
            Some(count) => self.estimate_element_size_u() * count as f64,
            None => 0.0,
        }
    }

    /// The anchor element from which realization starts (`GetOrEstimateAnchorElementForViewport`).
    fn get_or_estimate_anchor_element_for_viewport(
        &self,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
    ) -> (i32, f64) {
        self.presenter_base().default_get_or_estimate_anchor_element_for_viewport(
            viewport_start,
            viewport_end,
            item_count,
        )
    }

    /// The estimated size of an element on the primary axis (`EstimateElementSizeU`).
    fn estimate_element_size_u(&self) -> f64 {
        self.presenter_base().default_estimate_element_size_u()
    }

    /// The viewport of the parent presenter, if the presenter is nested in one
    /// (`GetParentPresenterViewPort`).
    fn get_parent_presenter_viewport(&self) -> Option<Rect> {
        None
    }

    /// The viewport realization fills, from the effective viewport (`GetMeasureViewport`).
    fn get_measure_viewport(&self, viewport: Rect) -> Rect {
        viewport
    }

    /// Whether a change of the effective viewport needs a new measure
    /// (`NeedsMeasureForViewportChange`).
    fn needs_measure_for_viewport_change(&self, _measure_viewport: Rect, _viewport: Rect) -> bool {
        true
    }

    /// Called when the effective viewport of the presenter changes (`OnEffectiveViewportChanged`).
    fn on_effective_viewport_changed(&self, e: &EffectiveViewportChangedEventArgs) {
        self.presenter_base().default_on_effective_viewport_changed(self, e);
    }

    /// The presenter as a selector of the elements of the final measure pass
    /// (`this is IFinalMeasureSelector`).
    fn as_final_measure_selector(&self) -> Option<&dyn IFinalMeasureSelector> {
        None
    }

    /// Whether the diagnostic trace of the presenter is written (the rows presenter's).
    fn trace_enabled(&self) -> bool {
        false
    }
}

/// The measure state of the realization pass.
#[derive(Clone, Copy, Default)]
struct MeasureViewport {
    anchor_index: i32,
    anchor_u: f64,
    viewport_u_start: f64,
    viewport_u_end: f64,
    measured_v: f64,
    realized_end_u: f64,
    last_index: i32,
    viewport_is_disjunct: bool,
}

/// Restores the tree membership flags of the base, and clears the in-layout flag when asked, when
/// a scope is left (the reference's `try`/`finally`).
struct MembershipScope<'a> {
    base: &'a TreeDataGridPresenterBase,
    previous_visual: bool,
    previous_logical: bool,
    clears_in_layout: bool,
}

impl<'a> MembershipScope<'a> {
    fn new(base: &'a TreeDataGridPresenterBase, visual: bool, logical: bool, clears_in_layout: bool) -> Self {
        let scope = Self {
            base,
            previous_visual: base.preserve_recycled_element_visual_tree_membership.get(),
            previous_logical: base.preserve_recycled_element_logical_tree_membership.get(),
            clears_in_layout,
        };
        base.preserve_recycled_element_visual_tree_membership.set(visual);
        base.preserve_recycled_element_logical_tree_membership.set(logical);
        scope
    }
}

impl Drop for MembershipScope<'_> {
    fn drop(&mut self) {
        self.base.preserve_recycled_element_visual_tree_membership.set(self.previous_visual);
        self.base.preserve_recycled_element_logical_tree_membership.set(self.previous_logical);
        if self.clears_in_layout {
            self.base.is_in_layout.set(false);
        }
    }
}

/// Clears the in-layout flag when a layout override is left.
struct InLayoutGuard<'a>(&'a Cell<bool>);

impl Drop for InLayoutGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A special element of the presenter: the focused element kept outside the realized range, or the
/// element being brought into view.
#[derive(Clone, Copy)]
enum Special {
    Focused,
    ScrollTo,
}

/// The state of the reference's `TreeDataGridPresenterBase<TItem>`.
pub struct TreeDataGridPresenterBase {
    element_factory: RefCell<Option<Rc<TreeDataGridElementFactory>>>,
    scroll_to_index: Cell<i32>,
    scroll_to_element: RefCell<Option<Ref<Control>>>,
    is_in_layout: Cell<bool>,
    is_waiting_for_viewport_update: Cell<bool>,
    last_measure_viewport: Cell<Rect>,
    items_subscription: Cell<Option<u64>>,
    measure_elements: RefCell<Option<Rc<RealizedStackElements>>>,
    realized_elements: RefCell<Option<Rc<RealizedStackElements>>>,
    /// The ancestor that implements the scroll anchor provider contract.
    scroll_anchor_provider: RefCell<Option<WeakRef<Visual>>>,
    last_estimated_element_size_u: Cell<f64>,
    focused_element: RefCell<Option<Ref<Control>>>,
    focused_index: Cell<i32>,
    /// The `LostFocus` handler of the retained focused element.
    focused_element_lost_focus: Cell<Option<RoutedEventHandlerToken>>,
    preserve_recycled_element_logical_tree_membership: Cell<bool>,
    preserve_recycled_element_visual_tree_membership: Cell<bool>,
    retained_recycled_element_count: Cell<i32>,
    /// Cached state for fast reattachment (fixes for tab switching performance).
    cached_viewport: Cell<Rect>,
    viewport: Cell<Rect>,
    /// The subscription to the effective viewport changes, held while attached to the visual tree:
    /// subscribing adds a reference to the presenter in the layout manager.
    effective_viewport_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl Default for TreeDataGridPresenterBase {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeDataGridPresenterBase {
    pub fn new() -> Self {
        Self {
            element_factory: RefCell::new(None),
            scroll_to_index: Cell::new(-1),
            scroll_to_element: RefCell::new(None),
            is_in_layout: Cell::new(false),
            is_waiting_for_viewport_update: Cell::new(false),
            last_measure_viewport: Cell::new(INVALID_VIEWPORT),
            items_subscription: Cell::new(None),
            measure_elements: RefCell::new(None),
            realized_elements: RefCell::new(None),
            scroll_anchor_provider: RefCell::new(None),
            last_estimated_element_size_u: Cell::new(25.0),
            focused_element: RefCell::new(None),
            focused_index: Cell::new(-1),
            focused_element_lost_focus: Cell::new(None),
            preserve_recycled_element_logical_tree_membership: Cell::new(false),
            preserve_recycled_element_visual_tree_membership: Cell::new(false),
            retained_recycled_element_count: Cell::new(0),
            cached_viewport: Cell::new(Rect::default()),
            viewport: Cell::new(INVALID_VIEWPORT),
            effective_viewport_subscription: RefCell::new(None),
        }
    }

    /// Writes a diagnostic trace line of the presenter (`Trace`, in debug builds only).
    fn trace<H: TreeDataGridPresenterHooks>(&self, hooks: &H, message: impl FnOnce() -> String) {
        if cfg!(debug_assertions) && hooks.trace_enabled() && crate::TreeDataGridDiagnostics::enable_tracing() {
            eprintln!("[{}] {}", hooks.presenter_control().get_type().name(), message());
        }
    }

    // --- properties -----------------------------------------------------------------------------

    /// The element factory (`ElementFactory`).
    pub fn element_factory(&self) -> Option<Rc<TreeDataGridElementFactory>> {
        self.element_factory.borrow().clone()
    }

    /// The field of the element factory, for the `ElementFactory` property of a presenter.
    pub(crate) fn element_factory_field(&self) -> &RefCell<Option<Rc<TreeDataGridElementFactory>>> {
        &self.element_factory
    }

    /// The realized elements, with an empty slot for each item of the realized range that has no
    /// element yet (`RealizedElements`).
    pub fn realized_elements(&self) -> Vec<Option<Ref<Control>>> {
        self.realized_elements.borrow().as_ref().map_or_else(Vec::new, |realized| realized.elements_snapshot())
    }

    /// The number of realized elements, empty slots included.
    pub fn realized_element_count(&self) -> usize {
        self.realized_elements.borrow().as_ref().map_or(0, |realized| realized.count())
    }

    /// Whether the presenter is in its measure or arrange pass (`IsInLayout`).
    pub fn is_in_layout(&self) -> bool {
        self.is_in_layout.get()
    }

    /// Whether the presenter is in its measure or arrange pass (`IsLayoutInProgress`).
    pub fn is_layout_in_progress(&self) -> bool {
        self.is_in_layout.get()
    }

    /// The effective viewport of the presenter, intersected with its bounds (`Viewport`).
    pub fn viewport(&self) -> Rect {
        self.viewport.get()
    }

    /// Gets the position of the first realized element on the primary axis (`StartU`).
    pub fn start_u(&self) -> f64 {
        self.realized_elements.borrow().as_ref().map_or(0.0, |realized| realized.start_u())
    }

    /// The index of the first realized element (`FirstIndex`).
    pub fn first_index(&self) -> i32 {
        self.realized_elements.borrow().as_ref().map_or(0, |realized| realized.first_index())
    }

    fn realized(&self) -> Option<Rc<RealizedStackElements>> {
        self.realized_elements.borrow().clone()
    }

    // --- public members -------------------------------------------------------------------------

    /// Brings the item at `index` into view, realizing its element if needed (`BringIntoView`).
    /// `rect` is the area of the element to bring into view. Returns the element, or `None` when
    /// the item cannot be brought into view.
    pub fn bring_into_view<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        index: i32,
        rect: Option<Rect>,
    ) -> Option<Ref<Control>> {
        let control = hooks.presenter_control();
        let item_count = hooks.item_count()?;

        if self.is_in_layout.get()
            || index < 0
            || index as usize >= item_count
            || self.realized_elements.borrow().is_none()
            || !control.is_effectively_visible()
        {
            return None;
        }

        let bring = |element: &Control| match rect {
            Some(rect) => element.bring_into_view_rect(rect),
            None => element.bring_into_view(),
        };

        if let Some(element) = self.get_realized_element(index) {
            bring(&element);
            return Some(element);
        } else if control.is_attached_to_visual_tree() {
            // Create and measure the element to be brought into view. Store it in a field so that
            // it can be re-used in the layout pass.
            let scroll_to_element = self.get_or_create_element(hooks, index);
            scroll_to_element.measure(Size::INFINITY);

            // Get the expected position of the element and put it in place.
            let anchor_u = self
                .realized()
                .map_or(0.0, |realized| realized.get_or_estimate_element_u(index, &self.last_estimated_element_size_u));
            let desired_size = scroll_to_element.desired_size();
            let element_rect = if hooks.orientation() == Orientation::Horizontal {
                Rect::new(anchor_u, 0.0, desired_size.width, desired_size.height)
            } else {
                Rect::new(0.0, anchor_u, desired_size.width, desired_size.height)
            };
            scroll_to_element.arrange(element_rect);

            // Store the element and index so that they can be used in the layout pass.
            *self.scroll_to_element.borrow_mut() = Some(scroll_to_element.clone());
            self.scroll_to_index.set(index);

            // If the item being brought into view was added since the last layout pass then our
            // bounds won't be updated, so any containing scroll viewers will not have an updated
            // extent. Do a layout pass to ensure that the containing scroll viewers will be able to
            // scroll the new item into view.
            if !control.bounds().contains_rect(element_rect) && !self.viewport.get().contains_rect(element_rect) {
                self.is_waiting_for_viewport_update.set(true);
                control.update_layout();
                self.is_waiting_for_viewport_update.set(false);
            }

            // Try to bring the item into view and do a layout pass.
            bring(&scroll_to_element);

            // If the viewport does not contain the item to scroll to, set the waiting flag: this
            // should cause the following chain of events:
            // - Measure is first done with the old viewport (which will be a no-op, see
            //   `measure_override`)
            // - The viewport is then updated by the layout system which invalidates our measure
            // - Measure is then done with the new viewport.
            self.is_waiting_for_viewport_update.set(!self.viewport.get().contains_rect(element_rect));
            control.update_layout();

            // If for some reason the layout system didn't give us a new viewport during the
            // layout, we need to do another layout pass as the one that took place was a no-op.
            if self.is_waiting_for_viewport_update.get() {
                self.is_waiting_for_viewport_update.set(false);
                control.invalidate_measure();
                control.update_layout();
            }

            // The first request can use an out-of-date cross-axis extent when elements have
            // different sizes. The intervening layout has now updated that extent, so retry the
            // same request with the final geometry.
            bring(&scroll_to_element);

            *self.scroll_to_element.borrow_mut() = None;
            self.scroll_to_index.set(-1);
            return Some(scroll_to_element);
        }

        None
    }

    /// The realized elements (`GetRealizedElements`).
    pub fn get_realized_elements(&self) -> Vec<Ref<Control>> {
        self.realized_elements.borrow().as_ref().map_or_else(Vec::new, |realized| {
            realized.elements().iter().flatten().cloned().collect()
        })
    }

    /// The realized element of the item at `index`, if any (`TryGetElement`).
    pub fn try_get_element(&self, index: i32) -> Option<Ref<Control>> {
        self.get_realized_element(index)
    }

    /// Recycles all the realized elements (`RecycleAllElements`).
    pub fn recycle_all_elements<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        preserve_visual_tree_membership: bool,
        preserve_logical_tree_membership: bool,
    ) {
        let _scope =
            MembershipScope::new(self, preserve_visual_tree_membership, preserve_logical_tree_membership, false);

        if let Some(realized) = self.realized() {
            realized.recycle_all_elements(|element, index| self.recycle_element(hooks, element, index));
        }
    }

    /// Recycles all the realized elements as if their items were removed
    /// (`RecycleAllElementsOnItemRemoved`).
    pub fn recycle_all_elements_on_item_removed<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        preserve_visual_tree_membership: bool,
        preserve_logical_tree_membership: bool,
    ) {
        let _scope =
            MembershipScope::new(self, preserve_visual_tree_membership, preserve_logical_tree_membership, false);

        if let Some(realized) = self.realized().filter(|realized| realized.count() > 0) {
            realized.items_removed(
                realized.first_index(),
                realized.count() as i32,
                |element, old_index, new_index| hooks.update_element_index(element, old_index, new_index),
                |element| self.recycle_element_on_item_removed(hooks, element),
            );
        }
    }

    // --- base implementations of the hooks ------------------------------------------------------

    /// The base `MeasureElement`.
    ///
    /// Optimization: skips the measure if the element was already measured with the same
    /// constraint and its measure is still valid. This significantly improves reattachment
    /// performance.
    pub fn default_measure_element(element: &Control, available_size: Size) -> Size {
        if element.previous_measure() == Some(available_size)
            && element.is_measure_valid()
            && element.desired_size() != Size::default()
        {
            return element.desired_size();
        }

        element.measure(available_size);
        element.desired_size()
    }

    /// The base `GetElementFromFactory(object data, int index, Control parent)`: the element of the
    /// element factory for `data`, preferring one recycled from `parent`.
    pub fn get_element_from_factory_for(&self, data: &dyn Any, parent: &Control) -> Ref<Control> {
        let factory = self.element_factory().expect("The presenter has no element factory.");
        factory.get_or_create_element(data, parent)
    }

    /// The base `GetOrEstimateAnchorElementForViewport`.
    pub fn default_get_or_estimate_anchor_element_for_viewport(
        &self,
        viewport_start: f64,
        viewport_end: f64,
        item_count: i32,
    ) -> (i32, f64) {
        let realized = self.realized().expect("the realized elements");
        realized.get_or_estimate_anchor_element_for_viewport(
            viewport_start,
            viewport_end,
            item_count,
            &self.last_estimated_element_size_u,
        )
    }

    /// The base `EstimateElementSizeU`.
    pub fn default_estimate_element_size_u(&self) -> f64 {
        let Some(realized) = self.realized() else {
            return self.last_estimated_element_size_u.get();
        };

        let result = realized.estimate_element_size_u();
        if result >= 0.0 {
            self.last_estimated_element_size_u.set(result);
        }
        self.last_estimated_element_size_u.get()
    }

    /// The base `OnEffectiveViewportChanged`.
    pub fn default_on_effective_viewport_changed<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        e: &EffectiveViewportChangedEventArgs,
    ) {
        let control = hooks.presenter_control();
        self.trace(hooks, || {
            format!(
                "OnEffectiveViewportChanged: OldViewport={:?}, NewEffectiveViewport={:?}, BoundsSize={:?}",
                self.viewport.get(),
                e.effective_viewport(),
                control.bounds().size()
            )
        });

        let vertical = hooks.orientation() == Orientation::Vertical;
        let old_viewport = self.viewport.get();
        let old_viewport_was_invalid = old_viewport == INVALID_VIEWPORT;
        let old_viewport_start = if vertical { old_viewport.top() } else { old_viewport.left() };
        let old_viewport_end = if vertical { old_viewport.bottom() } else { old_viewport.right() };

        // We sometimes get sent a viewport of 0,0 because the EffectiveViewportChanged event is
        // being raised when the parent control hasn't yet been arranged. This is a bug in the
        // framework, but we can work around it by forcing the measure to estimate the viewport.
        let effective_viewport = e.effective_viewport();
        let viewport = if effective_viewport.size() == Size::default() {
            INVALID_VIEWPORT
        } else {
            Self::intersect(effective_viewport, Rect::from_size(control.bounds().size()))
        };
        self.viewport.set(viewport);

        // Cache the viewport size for use when estimating viewport on reattachment.
        if viewport != INVALID_VIEWPORT && viewport.size() != Size::default() {
            self.cached_viewport.set(viewport);
        }

        self.is_waiting_for_viewport_update.set(false);

        let new_viewport_start = if vertical { viewport.top() } else { viewport.left() };
        let new_viewport_end = if vertical { viewport.bottom() } else { viewport.right() };
        if (!MathUtilities::are_close(old_viewport_start, new_viewport_start)
            || !MathUtilities::are_close(old_viewport_end, new_viewport_end))
            && (old_viewport_was_invalid
                || hooks.needs_measure_for_viewport_change(self.last_measure_viewport.get(), viewport))
        {
            control.invalidate_measure();
        }
    }

    /// Whether the realized elements cover `viewport` on the primary axis
    /// (`IsViewportCoveredByRealizedElements`).
    pub fn is_viewport_covered_by_realized_elements(&self, orientation: Orientation, viewport: Rect) -> bool {
        let viewport_start = if orientation == Orientation::Vertical { viewport.top() } else { viewport.left() };
        let viewport_end = if orientation == Orientation::Vertical { viewport.bottom() } else { viewport.right() };

        viewport_start.is_finite()
            && viewport_end.is_finite()
            && self.realized().and_then(|realized| realized.try_get_stable_range()).is_some_and(
                |(realized_start, realized_end)| {
                    !MathUtilities::greater_than(realized_start, viewport_start)
                        && !MathUtilities::greater_than(viewport_end, realized_end)
                },
            )
    }

    // --- layout ---------------------------------------------------------------------------------

    /// The base `MeasureOverride`.
    pub fn measure_override<H: TreeDataGridPresenterHooks>(&self, hooks: &H, available_size: Size) -> Size {
        let item_count = hooks.item_count().unwrap_or(0);

        if item_count == 0 {
            self.trim_unrealized_children(hooks);
            return Size::default();
        }

        let orientation = hooks.orientation();

        // If we're bringing an item into view, ignore any layout passes until we receive a new
        // effective viewport.
        if self.is_waiting_for_viewport_update.get() {
            return self.estimate_desired_size(hooks, orientation, item_count as i32);
        }

        self.is_in_layout.set(true);
        let _scope = MembershipScope::new(self, true, true, true);

        let trace_realized_elements = hooks.trace_enabled();
        let realized_elements = self.realized_elements.borrow().clone();
        let realized_elements = match realized_elements {
            Some(realized_elements) => realized_elements,
            None => {
                let realized_elements = Rc::new(RealizedStackElements::new(trace_realized_elements));
                *self.realized_elements.borrow_mut() = Some(realized_elements.clone());
                realized_elements
            }
        };
        let measure_elements = self.measure_elements.borrow().clone();
        let measure_elements = match measure_elements {
            Some(measure_elements) => measure_elements,
            None => {
                let measure_elements = Rc::new(RealizedStackElements::new(trace_realized_elements));
                *self.measure_elements.borrow_mut() = Some(measure_elements.clone());
                measure_elements
            }
        };

        // We handle horizontal and vertical layouts here so X and Y are abstracted to:
        // - Horizontal layouts: U = horizontal, V = vertical
        // - Vertical layouts: U = vertical, V = horizontal
        let mut viewport = self.calculate_measure_viewport(hooks, item_count as i32, available_size, &realized_elements);

        // If the viewport is disjunct then we can recycle everything.
        if viewport.viewport_is_disjunct {
            realized_elements.recycle_all_elements(|element, index| self.recycle_element(hooks, element, index));
        }

        // Do the measure, creating/recycling elements as necessary to fill the viewport. Don't
        // write to the realized elements yet, only the measure elements.
        self.realize_elements(hooks, item_count as i32, available_size, &mut viewport, &realized_elements, &measure_elements);

        // Run the final measure pass if necessary.
        let measured = measure_elements.elements_snapshot();
        if hooks.needs_final_measure_pass(measure_elements.first_index(), &measured) {
            let first_index = measure_elements.first_index();

            for (i, e) in measured.iter().enumerate() {
                let Some(e) = e else { continue };
                let previous = e.previous_measure().expect("the element was measured");

                let index = first_index + i as i32;
                let constraint = hooks.get_final_constraint(e, index, available_size);

                let needs_final_measure = match hooks.as_final_measure_selector() {
                    Some(selector) => selector.needs_final_measure(e, index),
                    None => Self::has_infinity(previous),
                };

                if needs_final_measure && previous != constraint {
                    e.measure(constraint);
                    let desired_size = e.desired_size();
                    viewport.measured_v = viewport.measured_v.max(if orientation == Orientation::Horizontal {
                        desired_size.height
                    } else {
                        desired_size.width
                    });
                }
            }
        }

        // Now swap the measure elements and realized elements collection.
        *self.measure_elements.borrow_mut() = Some(realized_elements.clone());
        *self.realized_elements.borrow_mut() = Some(measure_elements);
        realized_elements.reset_for_reuse();

        // A focused element is retained when it leaves the realized range. Keep its measurement
        // current so that it can be positioned safely outside the viewport and re-used without
        // losing keyboard focus.
        let focused_element = self.focused_element.borrow().clone();
        let focused_index = self.focused_index.get();
        if let Some(focused_element) = focused_element.filter(|_| focused_index >= 0) {
            let constraint = hooks.get_initial_constraint(&focused_element, focused_index, available_size);
            hooks.measure_element(focused_index, &focused_element, constraint);
        }

        self.trim_unrealized_children(hooks);

        self.calculate_desired_size(hooks, orientation, item_count as i32, &viewport)
    }

    /// The base `ArrangeOverride`.
    pub fn arrange_override<H: TreeDataGridPresenterHooks>(&self, hooks: &H, final_size: Size) -> Size {
        let Some(realized_elements) = self.realized() else {
            return final_size;
        };

        self.is_in_layout.set(true);
        let _in_layout = InLayoutGuard(&self.is_in_layout);

        let control = hooks.presenter_control();
        let orientation = hooks.orientation();
        let mut u = realized_elements
            .get_or_estimate_element_u(realized_elements.first_index(), &self.last_estimated_element_size_u);
        let viewport = self.viewport.get();

        let provider_visual = self.scroll_anchor_provider();
        let provider = provider_visual.as_ref().and_then(|visual| as_scroll_anchor_provider(visual));
        let this_visual: Option<Ref<Visual>> = provider.map(|_| control.to_ref().upcast());

        let first_index = realized_elements.first_index();
        let mut i = 0;
        while i < realized_elements.count() {
            if let Some(e) = realized_elements.element_at(i) {
                let size_u = realized_elements.size_u_at(i);
                let rect = if orientation == Orientation::Horizontal {
                    Rect::new(u, 0.0, size_u, final_size.height)
                } else {
                    Rect::new(0.0, u, final_size.width, size_u)
                };
                let rect = hooks.arrange_element(i as i32 + first_index, &e, rect);

                if let Some(provider) = provider {
                    if e.is_visible() && viewport != INVALID_VIEWPORT && viewport.intersects(rect) {
                        // The element may have been reparented during virtualization. It is no
                        // longer a valid anchor candidate in that case.
                        if e.visual_parent() == this_visual {
                            provider.register_anchor_candidate(&e);
                        } else if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::LAYOUT) {
                            logger.log_with_values(
                                Some(hooks as &dyn Any),
                                "RegisterAnchorCandidate ignored for {Element}: {Message}",
                                &[
                                    &e.get_type().name(),
                                    &"An anchor control must be a visual descendent of the scroll anchor provider.",
                                ],
                            );
                        }
                    }
                }

                u += if orientation == Orientation::Horizontal { rect.width } else { rect.height };
            }

            i += 1;
        }

        // Keep a retained focused element outside the realized range. Position estimates for
        // non-uniform items are approximate, so clamp them to the nearest realized boundary to
        // prevent a retained element from rendering over live rows or cells.
        let focused_element = self.focused_element.borrow().clone();
        let focused_index = self.focused_index.get();
        if let Some(focused_element) = focused_element.filter(|_| focused_index >= 0) {
            let realized_end_u = u;
            let size_u = if orientation == Orientation::Horizontal {
                focused_element.desired_size().width
            } else {
                focused_element.desired_size().height
            };

            u = realized_elements.get_or_estimate_element_u(focused_index, &self.last_estimated_element_size_u);

            if realized_elements.count() > 0 {
                if focused_index < realized_elements.first_index() {
                    u = u.min(realized_elements.start_u() - size_u);
                } else if focused_index > realized_elements.last_index() {
                    u = u.max(realized_end_u);
                }
            }

            let rect = if orientation == Orientation::Horizontal {
                Rect::new(u, 0.0, size_u, final_size.height)
            } else {
                Rect::new(0.0, u, final_size.width, size_u)
            };
            hooks.arrange_element(focused_index, &focused_element, rect);
        }

        final_size
    }

    // --- tree attachment ------------------------------------------------------------------------

    /// The base `OnAttachedToVisualTree`, after the parent class's.
    pub fn on_attached_to_visual_tree<H: TreeDataGridPresenterHooks>(&self, hooks: &H) {
        let control = hooks.presenter_control();

        let mut provider = None;
        let mut current = control.visual_parent();
        while let Some(visual) = current {
            if as_scroll_anchor_provider(&visual).is_some() {
                provider = Some(visual.downgrade());
                break;
            }
            current = visual.visual_parent();
        }
        *self.scroll_anchor_provider.borrow_mut() = provider;

        self.trace(hooks, || {
            format!(
                "OnAttachedToVisualTree: Viewport={:?}, IsInvalid={}, ItemCount={}, CachedViewport={:?}",
                self.viewport.get(),
                self.viewport.get() == INVALID_VIEWPORT,
                hooks.item_count().unwrap_or(0),
                self.cached_viewport.get()
            )
        });

        // Subscribing to this event adds a reference to the presenter in the layout manager, so
        // this must be unsubscribed to avoid memory leaks.
        let weak: WeakRef<H> = FerroObject::ref_of(hooks).downgrade();
        let subscription = control.effective_viewport_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_effective_viewport_changed(e);
            }
        });
        if let Some(previous) = self.effective_viewport_subscription.borrow_mut().replace(subscription) {
            previous.dispose();
        }

        self.subscribe_to_item_changes(hooks);
    }

    /// The base `OnDetachedFromVisualTree`, after the parent class's.
    pub fn on_detached_from_visual_tree<H: TreeDataGridPresenterHooks>(&self, hooks: &H) {
        self.trace(hooks, || {
            format!(
                "OnDetachedFromVisualTree: recycling {} elements, Viewport={:?}, CachedViewport={:?}",
                self.realized_element_count(),
                self.viewport.get(),
                self.cached_viewport.get()
            )
        });

        let subscription = self.effective_viewport_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        self.unsubscribe_from_item_changes(hooks);

        // The presenter is already leaving the visual tree, so keep recycled controls parented
        // logically until the next measure can reuse or trim them.
        self.recycle_all_elements(hooks, false, true);
        *self.scroll_anchor_provider.borrow_mut() = None;
    }

    /// The base `OnDetachedFromLogicalTree`, after the parent class's.
    pub fn on_detached_from_logical_tree<H: TreeDataGridPresenterHooks>(&self, hooks: &H) {
        self.recycle_all_elements(hooks, false, false);
    }

    // --- items ----------------------------------------------------------------------------------

    /// Subscribes to the changes of the presenter's items, if they notify and the presenter is not
    /// subscribed yet (`SubscribeToItemChanges`).
    pub fn subscribe_to_item_changes<H: TreeDataGridPresenterHooks>(&self, hooks: &H) {
        if self.items_subscription.get().is_some() {
            return;
        }

        let weak: WeakRef<H> = FerroObject::ref_of(hooks).downgrade();
        let token = hooks.subscribe_to_items(Rc::new(move |change: &PresenterItemsChange| {
            if let Some(this) = weak.upgrade() {
                this.presenter_base().on_items_collection_changed(&*this, change);
            }
        }));
        self.items_subscription.set(token);
    }

    /// Removes the subscription to the changes of the presenter's items
    /// (`UnsubscribeFromItemChanges`). Called before the items are replaced, while the presenter
    /// still holds the items it subscribed to.
    pub fn unsubscribe_from_item_changes<H: TreeDataGridPresenterHooks>(&self, hooks: &H) {
        if let Some(token) = self.items_subscription.take() {
            hooks.unsubscribe_from_items(token);
        }
    }

    /// Handles a change of the items (`OnItemsCollectionChanged`); a presenter also calls it with
    /// a reset when its items are replaced.
    pub fn on_items_collection_changed<H: TreeDataGridPresenterHooks>(&self, hooks: &H, e: &PresenterItemsChange) {
        hooks.presenter_control().invalidate_measure();

        let Some(realized) = self.realized() else {
            return;
        };

        let _scope = MembershipScope::new(self, true, true, false);
        let update_element_index =
            |element: &Ref<Control>, old_index: i32, new_index: i32| hooks.update_element_index(element, old_index, new_index);
        let recycle_element_on_item_removed = |element: &Ref<Control>| self.recycle_element_on_item_removed(hooks, element);

        match e.action {
            NotifyCollectionChangedAction::Add => {
                self.update_special_element_for_insert(hooks, Special::Focused, e.new_starting_index, e.new_count);
                self.update_special_element_for_insert(hooks, Special::ScrollTo, e.new_starting_index, e.new_count);
                realized.items_inserted(
                    e.new_starting_index,
                    e.new_count,
                    update_element_index,
                    recycle_element_on_item_removed,
                );
            }
            NotifyCollectionChangedAction::Remove => {
                realized.items_removed(
                    e.old_starting_index,
                    e.old_count,
                    update_element_index,
                    recycle_element_on_item_removed,
                );
                self.update_special_element_for_remove(hooks, Special::Focused, e.old_starting_index, e.old_count);
                self.update_special_element_for_remove(hooks, Special::ScrollTo, e.old_starting_index, e.old_count);
            }
            NotifyCollectionChangedAction::Replace => {
                realized.items_replaced(e.old_starting_index, e.old_count, recycle_element_on_item_removed);
                self.recycle_special_element_in_range(hooks, Special::Focused, e.old_starting_index, e.old_count);
                self.recycle_special_element_in_range(hooks, Special::ScrollTo, e.old_starting_index, e.old_count);
            }
            NotifyCollectionChangedAction::Move => {
                realized.items_moved(
                    e.old_starting_index,
                    e.new_starting_index,
                    e.old_count,
                    update_element_index,
                    recycle_element_on_item_removed,
                );
                self.update_special_element_for_move(
                    hooks,
                    Special::Focused,
                    e.old_starting_index,
                    e.new_starting_index,
                    e.old_count,
                );
                self.update_special_element_for_move(
                    hooks,
                    Special::ScrollTo,
                    e.old_starting_index,
                    e.new_starting_index,
                    e.old_count,
                );
            }
            NotifyCollectionChangedAction::Reset => {
                realized.items_reset(recycle_element_on_item_removed);
                self.recycle_special_element(hooks, Special::Focused);
                self.recycle_special_element(hooks, Special::ScrollTo);
            }
        }
    }

    // --- realization ----------------------------------------------------------------------------

    fn realize_elements<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        item_count: i32,
        available_size: Size,
        viewport: &mut MeasureViewport,
        realized_elements: &RealizedStackElements,
        measure_elements: &RealizedStackElements,
    ) {
        debug_assert!(item_count > 0);

        let mut index = viewport.anchor_index;
        let horizontal = hooks.orientation() == Orientation::Horizontal;
        let mut u = viewport.anchor_u;

        // If the anchor element is at the beginning of, or before, the start of the viewport then
        // we can recycle all elements before it.
        if u <= viewport.anchor_u {
            realized_elements
                .recycle_elements_before(viewport.anchor_index, |element, index| self.recycle_element(hooks, element, index));
        }

        // Start at the anchor element and move forwards, realizing elements.
        loop {
            let e = self.get_or_create_element(hooks, index);
            let constraint = hooks.get_initial_constraint(&e, index, available_size);
            let slot = hooks.measure_element(index, &e, constraint);

            let size_u = if horizontal { slot.width } else { slot.height };
            let size_v = if horizontal { slot.height } else { slot.width };

            measure_elements.add(index, e, u, size_u);
            viewport.measured_v = viewport.measured_v.max(size_v);

            u += size_u;
            index += 1;

            if !(u < viewport.viewport_u_end && index < item_count) {
                break;
            }
        }

        // Store the last index and end U position for the desired size calculation.
        viewport.last_index = index - 1;
        viewport.realized_end_u = u;

        // We can now recycle elements after the last element.
        realized_elements
            .recycle_elements_after(viewport.last_index, |element, index| self.recycle_element(hooks, element, index));

        // Next move backwards from the anchor element, realizing elements.
        index = viewport.anchor_index - 1;
        u = viewport.anchor_u;

        while u > viewport.viewport_u_start && index >= 0 {
            let e = self.get_or_create_element(hooks, index);
            let constraint = hooks.get_initial_constraint(&e, index, available_size);
            let slot = hooks.measure_element(index, &e, constraint);

            let size_u = if horizontal { slot.width } else { slot.height };
            let size_v = if horizontal { slot.height } else { slot.width };
            u -= size_u;

            measure_elements.add(index, e, u, size_u);
            viewport.measured_v = viewport.measured_v.max(size_v);
            index -= 1;
        }

        // We can now recycle elements before the first element.
        realized_elements.recycle_elements_before(index + 1, |element, index| self.recycle_element(hooks, element, index));

        self.trace(hooks, || {
            format!(
                "RealizeElements complete: realized {} elements from {} to {}, viewportU=[{:.1}, {:.1}]",
                measure_elements.count(),
                measure_elements.first_index(),
                viewport.last_index,
                viewport.viewport_u_start,
                viewport.viewport_u_end
            )
        });
    }

    fn calculate_desired_size<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        orientation: Orientation,
        item_count: i32,
        viewport: &MeasureViewport,
    ) -> Size {
        let mut size_u = 0.0;
        let size_v = viewport.measured_v;

        if viewport.last_index >= 0 {
            let remaining = item_count - viewport.last_index - 1;
            size_u = viewport.realized_end_u + (remaining as f64 * hooks.estimate_element_size_u());
        }

        if orientation == Orientation::Horizontal {
            Size::new(size_u, size_v)
        } else {
            Size::new(size_v, size_u)
        }
    }

    fn estimate_desired_size<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        orientation: Orientation,
        item_count: i32,
    ) -> Size {
        let control = hooks.presenter_control();
        let scroll_to_bounds = if self.scroll_to_index.get() >= 0 {
            self.scroll_to_element.borrow().as_ref().map(|element| element.bounds())
        } else {
            None
        };

        if let Some(bounds) = scroll_to_bounds {
            let remaining = item_count - self.scroll_to_index.get() - 1;
            let realized_end_u = if orientation == Orientation::Horizontal { bounds.right() } else { bounds.bottom() };
            let size_u = realized_end_u + (remaining as f64 * self.last_estimated_element_size_u.get());

            return if orientation == Orientation::Horizontal {
                Size::new(size_u, control.desired_size().height)
            } else {
                Size::new(control.desired_size().width, size_u)
            };
        }

        control.desired_size()
    }

    fn calculate_measure_viewport<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        item_count: i32,
        available_size: Size,
        realized_elements: &RealizedStackElements,
    ) -> MeasureViewport {
        // If the control has not yet been laid out then the effective viewport won't have been
        // set. Try to work it out from an ancestor control.
        let viewport_is_invalid = self.viewport.get() == INVALID_VIEWPORT;
        let viewport = if !viewport_is_invalid { self.viewport.get() } else { self.estimate_viewport(hooks, available_size) };
        let viewport = hooks.get_measure_viewport(viewport);
        self.last_measure_viewport.set(viewport);

        self.trace(hooks, || {
            format!(
                "CalculateMeasureViewport: ViewportIsInvalid={viewport_is_invalid}, Viewport={:?}, EstimatedViewport={viewport:?}, AvailableSize={available_size:?}, ItemCount={item_count}",
                self.viewport.get()
            )
        });

        // Get the viewport in the orientation direction.
        let horizontal = hooks.orientation() == Orientation::Horizontal;
        let viewport_start = if horizontal { viewport.x } else { viewport.y };
        let viewport_end = if horizontal { viewport.right() } else { viewport.bottom() };

        // Get or estimate the anchor element from which to start realization. If we are scrolling
        // to an element, use that as the anchor so that it is consumed by the realization pass
        // instead of being left as a disjoint visible child.
        let scroll_to_bounds = if self.scroll_to_index.get() >= 0 {
            self.scroll_to_element.borrow().as_ref().map(|element| element.bounds())
        } else {
            None
        };

        let (anchor_index, anchor_u) = match scroll_to_bounds {
            Some(bounds) => (self.scroll_to_index.get(), if horizontal { bounds.left() } else { bounds.top() }),
            None => hooks.get_or_estimate_anchor_element_for_viewport(viewport_start, viewport_end, item_count),
        };

        // Check if the anchor element is not within the currently realized elements.
        let disjunct = anchor_index < realized_elements.first_index() || anchor_index > realized_elements.last_index();

        MeasureViewport {
            anchor_index,
            anchor_u,
            viewport_u_start: viewport_start,
            viewport_u_end: viewport_end,
            viewport_is_disjunct: disjunct,
            ..MeasureViewport::default()
        }
    }

    fn get_or_create_element<H: TreeDataGridPresenterHooks>(&self, hooks: &H, index: i32) -> Ref<Control> {
        if let Some(element) = self
            .get_realized_element(index)
            .or_else(|| Self::get_realized_special_element(index, &self.focused_index, &self.focused_element))
            .or_else(|| Self::get_realized_special_element(index, &self.scroll_to_index, &self.scroll_to_element))
        {
            return element;
        }

        self.get_recycled_or_create_element(hooks, index)
    }

    fn get_realized_element(&self, index: i32) -> Option<Ref<Control>> {
        self.realized_elements.borrow().as_ref().and_then(|realized| realized.get_element(index))
    }

    fn get_realized_special_element(
        index: i32,
        special_index: &Cell<i32>,
        special_element: &RefCell<Option<Ref<Control>>>,
    ) -> Option<Ref<Control>> {
        if special_index.get() == index {
            let result = special_element.take();
            debug_assert!(result.is_some());
            special_index.set(-1);
            return result;
        }

        None
    }

    fn get_recycled_or_create_element<H: TreeDataGridPresenterHooks>(&self, hooks: &H, index: i32) -> Ref<Control> {
        let control = hooks.presenter_control();
        let this_element: Ref<StyledElement> = control.to_ref().upcast();
        let this_visual: Ref<Visual> = control.to_ref().upcast();

        let item = hooks.item_at(index as usize);
        let e = hooks.get_element_from_factory(&item, index);
        let was_retained = !e.is_visible()
            && (e.parent().is_some_and(|parent| parent.ptr_eq(&this_element))
                || e.visual_parent().is_some_and(|parent| parent.ptr_eq(&this_visual)));
        e.set_current_value(Visual::is_visible_property(), true);

        if was_retained && self.retained_recycled_element_count.get() > 0 {
            self.retained_recycled_element_count.set(self.retained_recycled_element_count.get() - 1);
        }

        hooks.realize_element(&e, &item, index);

        if e.parent().is_none() {
            e.set_parent(this_element);
            control.logical_children().add(e.clone().upcast());
        }

        if e.visual_parent().is_none() {
            control.visual_children().add(e.clone().upcast());
        }

        e
    }

    fn estimate_viewport<H: TreeDataGridPresenterHooks>(&self, hooks: &H, available_size: Size) -> Rect {
        if let Some(parent_viewport) = hooks.get_parent_presenter_viewport().filter(|v| *v != INVALID_VIEWPORT) {
            return parent_viewport;
        }

        // A parent presenter carries the current scroll offset and is therefore a safer estimate
        // than a cached size. Use the cache only when no parent viewport exists.
        if self.cached_viewport.get() != Rect::default() {
            self.trace(hooks, || format!("EstimateViewport: Using cached viewport {:?}", self.cached_viewport.get()));
            return self.cached_viewport.get();
        }

        let control = hooks.presenter_control();
        let mut c = control.visual_parent();

        if c.is_none() {
            return Rect::default();
        }

        while let Some(visual) = c {
            if visual.bounds() != Rect::default() {
                if let Some(transform) = visual.transform_to_visual(control) {
                    let r = Rect::new(0.0, 0.0, visual.bounds().width, visual.bounds().height).transform_to_aabb(transform);
                    return Self::intersect(r, Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY));
                }
            }

            c = visual.visual_parent();
        }

        Rect::new(
            0.0,
            0.0,
            if available_size.width.is_finite() { available_size.width } else { 0.0 },
            if available_size.height.is_finite() { available_size.height } else { 0.0 },
        )
    }

    // --- recycling ------------------------------------------------------------------------------

    fn scroll_anchor_provider(&self) -> Option<Ref<Visual>> {
        self.scroll_anchor_provider.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(provider_visual) = self.scroll_anchor_provider() {
            if let Some(provider) = as_scroll_anchor_provider(&provider_visual) {
                provider.unregister_anchor_candidate(element);
            }
        }
    }

    fn recycle_element<H: TreeDataGridPresenterHooks>(&self, hooks: &H, element: &Ref<Control>, index: i32) {
        self.unregister_anchor_candidate(element);

        if element.is_keyboard_focus_within() {
            self.unsubscribe_focused_element_lost_focus();
            *self.focused_element.borrow_mut() = Some(element.clone());
            self.focused_index.set(index);

            let weak: WeakRef<H> = FerroObject::ref_of(hooks).downgrade();
            let token = element.add_handler(InputElement::lost_focus_event(), move |sender, _e| {
                if let (Some(this), Some(sender)) = (weak.upgrade(), sender.to_ref().cast::<Control>()) {
                    this.presenter_base().on_unrealized_focused_element_lost_focus(&*this, sender);
                }
            });
            self.focused_element_lost_focus.set(Some(token));
        } else {
            hooks.unrealize_element(element);
            element.set_current_value(Visual::is_visible_property(), false);
            self.detach_element(hooks, element);
            if let Some(factory) = self.element_factory() {
                factory.recycle_element(element);
            }
        }
    }

    fn recycle_element_on_item_removed<H: TreeDataGridPresenterHooks>(&self, hooks: &H, element: &Ref<Control>) {
        self.unregister_anchor_candidate(element);

        if self.focused_element.borrow().as_ref() == Some(element) {
            self.unsubscribe_focused_element_lost_focus();
            *self.focused_element.borrow_mut() = None;
            self.focused_index.set(-1);
        }

        hooks.unrealize_element_on_item_removed(element);
        element.set_current_value(Visual::is_visible_property(), false);
        self.detach_element(hooks, element);
        if let Some(factory) = self.element_factory() {
            factory.recycle_element(element);
        }
    }

    fn unsubscribe_focused_element_lost_focus(&self) {
        if let Some(token) = self.focused_element_lost_focus.take() {
            let focused_element = self.focused_element.borrow().clone();
            if let Some(focused_element) = focused_element {
                focused_element.remove_handler(InputElement::lost_focus_event(), token);
            }
        }
    }

    fn detach_element<H: TreeDataGridPresenterHooks>(&self, hooks: &H, element: &Ref<Control>) {
        let control = hooks.presenter_control();
        let preserve_visual = self.preserve_recycled_element_visual_tree_membership.get();
        let preserve_logical = self.preserve_recycled_element_logical_tree_membership.get();

        if preserve_visual || preserve_logical {
            self.retained_recycled_element_count.set(self.retained_recycled_element_count.get() + 1);
        }

        if !preserve_visual {
            let this_visual: Ref<Visual> = control.to_ref().upcast();
            if element.visual_parent().is_some_and(|parent| parent.ptr_eq(&this_visual)) {
                control.visual_children().remove(&element.clone().upcast());
            }
        }

        if !preserve_logical {
            let this_element: Ref<StyledElement> = control.to_ref().upcast();
            control.logical_children().remove(&element.clone().upcast());

            if element.parent().is_some_and(|parent| parent.ptr_eq(&this_element)) {
                element.set_parent(None);
            }
        }
    }

    fn trim_unrealized_children<H: TreeDataGridPresenterHooks>(&self, hooks: &H) {
        if self.retained_recycled_element_count.get() == 0 {
            return;
        }

        let control = hooks.presenter_control();
        let this_element: Ref<StyledElement> = control.to_ref().upcast();
        let item_count = hooks.item_count().unwrap_or(0);

        for child in control.logical_children().snapshot().iter() {
            if let Some(child) = child.cast::<Control>().filter(|child| !child.is_visible()) {
                hooks.finalize_recycled_element(&child);
            }
        }

        // Keep normally recycled controls parented and hidden so that a later layout pass can
        // reuse them without reattaching their logical trees and reapplying styles. Only trim the
        // pool when it can no longer correspond to the source collection.
        let children = control.visual_children();
        let logical_children = control.logical_children();
        let trim_visual_children = children.count() > item_count;
        let trim_logical_children = logical_children.count() > item_count;

        if !trim_visual_children && !trim_logical_children {
            return;
        }

        if trim_visual_children {
            let mut i = children.count();
            while i > 0 {
                i -= 1;
                if i < children.count() && !children.get(i).is_visible() {
                    children.remove_at(i);
                }
            }
        }

        if trim_logical_children {
            let mut i = logical_children.count();
            while i > 0 {
                i -= 1;
                if i >= logical_children.count() {
                    continue;
                }

                let child = logical_children.get(i);

                if child.downcast_ref::<Visual>().is_some_and(|visual| !visual.is_visible()) {
                    logical_children.remove_at(i);

                    if child.parent().is_some_and(|parent| parent.ptr_eq(&this_element)) {
                        child.set_parent(None);
                    }
                }
            }
        }

        let retained = logical_children
            .snapshot()
            .iter()
            .filter_map(|child| child.cast::<Control>())
            .filter(|child| !child.is_visible() && child.parent().is_some_and(|parent| parent.ptr_eq(&this_element)))
            .count();
        self.retained_recycled_element_count.set(retained as i32);
    }

    fn special_fields(&self, special: Special) -> (&RefCell<Option<Ref<Control>>>, &Cell<i32>) {
        match special {
            Special::Focused => (&self.focused_element, &self.focused_index),
            Special::ScrollTo => (&self.scroll_to_element, &self.scroll_to_index),
        }
    }

    fn update_special_element_for_insert<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        special: Special,
        index: i32,
        count: i32,
    ) {
        let (element, element_index) = self.special_fields(special);
        let element = element.borrow().clone();
        if let Some(element) = element {
            if element_index.get() >= index {
                self.update_special_element_index(hooks, &element, element_index, element_index.get() + count);
            }
        }
    }

    fn update_special_element_for_remove<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        special: Special,
        index: i32,
        count: i32,
    ) {
        let (element, element_index) = self.special_fields(special);
        let Some(element) = element.borrow().clone() else {
            return;
        };

        if element_index.get() >= index && element_index.get() < index + count {
            self.recycle_special_element(hooks, special);
        } else if element_index.get() >= index + count {
            self.update_special_element_index(hooks, &element, element_index, element_index.get() - count);
        }
    }

    fn update_special_element_for_move<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        special: Special,
        old_index: i32,
        new_index: i32,
        count: i32,
    ) {
        let (element, element_index) = self.special_fields(special);
        let Some(element) = element.borrow().clone() else {
            return;
        };

        let current = element_index.get();
        let mut updated_index = current;

        if current >= old_index && current < old_index + count {
            updated_index = new_index + current - old_index;
        } else if old_index < new_index && current >= old_index + count && current < new_index + count {
            updated_index -= count;
        } else if old_index > new_index && current >= new_index && current < old_index {
            updated_index += count;
        }

        self.update_special_element_index(hooks, &element, element_index, updated_index);
    }

    fn recycle_special_element_in_range<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        special: Special,
        index: i32,
        count: i32,
    ) {
        let (element, element_index) = self.special_fields(special);
        if element.borrow().is_some() && element_index.get() >= index && element_index.get() < index + count {
            self.recycle_special_element(hooks, special);
        }
    }

    fn recycle_special_element<H: TreeDataGridPresenterHooks>(&self, hooks: &H, special: Special) {
        let (element, element_index) = self.special_fields(special);
        let value = element.borrow().clone();
        if let Some(value) = value {
            self.recycle_element_on_item_removed(hooks, &value);
        }

        *element.borrow_mut() = None;
        element_index.set(-1);
    }

    fn update_special_element_index<H: TreeDataGridPresenterHooks>(
        &self,
        hooks: &H,
        element: &Ref<Control>,
        element_index: &Cell<i32>,
        new_index: i32,
    ) {
        if element_index.get() != new_index {
            let old_index = element_index.get();
            element_index.set(new_index);
            hooks.update_element_index(element, old_index, new_index);
        }
    }

    fn on_unrealized_focused_element_lost_focus<H: TreeDataGridPresenterHooks>(&self, hooks: &H, sender: Ref<Control>) {
        let focused_element = self.focused_element.borrow().clone();
        let Some(focused_element) = focused_element.filter(|focused| *focused == sender) else {
            return;
        };

        self.unsubscribe_focused_element_lost_focus();
        let focused_index = self.focused_index.get();
        *self.focused_element.borrow_mut() = None;
        self.focused_index.set(-1);
        self.recycle_element(hooks, &focused_element, focused_index);
    }

    fn has_infinity(s: Size) -> bool {
        s.width.is_infinite() || s.height.is_infinite()
    }

    /// The intersection of two rects; the empty rect at the origin when they do not intersect.
    ///
    /// Works around the intersection of the framework, which does not give the intersection of an
    /// empty rect on the edge of the other.
    fn intersect(a: Rect, b: Rect) -> Rect {
        let new_left = if a.x > b.x { a.x } else { b.x };
        let new_top = if a.y > b.y { a.y } else { b.y };
        let new_right = if a.right() < b.right() { a.right() } else { b.right() };
        let new_bottom = if a.bottom() < b.bottom() { a.bottom() } else { b.bottom() };

        if new_right >= new_left && new_bottom >= new_top {
            Rect::new(new_left, new_top, new_right - new_left, new_bottom - new_top)
        } else {
            Rect::default()
        }
    }
}

/// The columns of a presentation compare by identity, so that they can be held in property values
/// (the `Columns` and `Items` properties of the presenters and the row).
impl PartialEq for dyn crate::models::IColumns {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// The rows of a presentation compare by identity, so that they can be held in property values
/// (the `Items` and `Rows` properties of the presenters and the row).
impl PartialEq for dyn crate::models::ITreeDataGridRows {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
