use crate::generators::{ItemContainerGenerator, RecycleKey};
use crate::i_scroll_anchor_provider::as_scroll_anchor_provider;
use crate::items_source::ItemsChangedEventArgs;
use crate::primitives::{
    register_scroll_snap_points_info, IScrollSnapPointsInfo, SnapPointsAlignment, SnapPointsChangedHandler,
};
use crate::utils::{RealizedStackElements, VirtualizingSnapPointsList};
use crate::{
    Control, ControlImpl, ItemsControl, ItemsSourceView, PanelImpl, StackPanel, VirtualizingPanel,
    VirtualizingPanelImpl, VirtualizingPanelImplExt,
};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::{InputElement, InputElementImpl, KeyboardNavigation, NavigationDirection};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{EffectiveViewportChangedEventArgs, LayoutableImpl, Orientation};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AttachedProperty, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Arranges and virtualizes content on a single line that is oriented either
/// horizontally or vertically.
#[repr(C)]
pub struct VirtualizingStackPanel {
    base: VirtualizingPanel,
    scroll_to_index: Cell<i32>,
    scroll_to_element: RefCell<Option<Ref<Control>>>,
    is_in_layout: Cell<bool>,
    is_waiting_for_viewport_update: Cell<bool>,
    last_estimated_element_size_u: Cell<f64>,
    measure_elements: RefCell<Option<Rc<RealizedStackElements>>>,
    realized_elements: RefCell<Option<Rc<RealizedStackElements>>>,
    /// The ancestor that implements the scroll anchor provider contract.
    scroll_anchor_provider: RefCell<Option<WeakRef<Visual>>>,
    viewport: Cell<Rect>,
    recycle_pool: RefCell<HashMap<RecycleKey, Vec<Ref<Control>>>>,
    focused_element: RefCell<Option<Ref<Control>>>,
    focused_index: Cell<i32>,
    realizing_element: RefCell<Option<Ref<Control>>>,
    realizing_index: Cell<i32>,
    buffer_factor: Cell<f64>,

    has_reached_start: Cell<bool>,
    has_reached_end: Cell<bool>,
    last_measured_extended_viewport: Cell<Rect>,
    last_known_extended_viewport: Cell<Rect>,

    /// The subscription to the property changes of the items control.
    items_control_property_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(VirtualizingStackPanel: VirtualizingPanel);
ferroui_base::ferro_class_info!(VirtualizingStackPanel { new: VirtualizingStackPanel::new });
ferro_impl_classes!(
    VirtualizingStackPanel: StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

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

/// Clears the in-layout flag when a layout override is left.
struct InLayoutGuard<'a>(&'a Cell<bool>);

impl Drop for InLayoutGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl FerroObjectImpl for VirtualizingStackPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.buffer_factor.set(this.cache_length().max(0.0));

        let weak = this.to_ref().downgrade();
        let _ = this.effective_viewport_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_effective_viewport_changed(e);
            }
        });
    }
}

impl LayoutableImpl for VirtualizingStackPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let items = this.items();
        let item_count = items.count() as i32;

        if item_count == 0 {
            return Size::default();
        }

        let orientation = this.orientation();

        // If we're bringing an item into view, ignore any layout passes until
        // we receive a new effective viewport.
        if this.is_waiting_for_viewport_update.get() {
            return this.estimate_desired_size(orientation, item_count);
        }

        this.is_in_layout.set(true);
        let _in_layout = InLayoutGuard(&this.is_in_layout);

        let realized_elements = this.realized_elements.borrow().clone();
        if let Some(realized_elements) = &realized_elements {
            realized_elements.validate_start_u(this.orientation());
        }
        let realized_elements = match realized_elements {
            Some(realized_elements) => realized_elements,
            None => {
                let realized_elements = Rc::new(RealizedStackElements::new());
                *this.realized_elements.borrow_mut() = Some(realized_elements.clone());
                realized_elements
            }
        };
        let measure_elements = this.measure_elements.borrow().clone();
        let measure_elements = match measure_elements {
            Some(measure_elements) => measure_elements,
            None => {
                let measure_elements = Rc::new(RealizedStackElements::new());
                *this.measure_elements.borrow_mut() = Some(measure_elements.clone());
                measure_elements
            }
        };

        // We need to set the last estimated element size before calling
        // `calculate_desired_size`.
        let _ = this.estimate_element_size_u();

        // We handle horizontal and vertical layouts here so X and Y are
        // abstracted to:
        // - Horizontal layouts: U = horizontal, V = vertical
        // - Vertical layouts: U = vertical, V = horizontal
        let mut viewport = this.calculate_measure_viewport(orientation, item_count, &realized_elements);

        // If the viewport is disjunct then we can recycle everything.
        if viewport.viewport_is_disjunct {
            realized_elements.recycle_all_elements(|element, index| this.recycle_element(element, index));
        }

        // Do the measure, creating/recycling elements as necessary to fill
        // the viewport. Don't write to the realized elements yet, only the
        // measure elements.
        this.realize_elements(&items, available_size, &mut viewport, &realized_elements, &measure_elements);

        // Now swap the measure elements and realized elements collection.
        *this.measure_elements.borrow_mut() = Some(realized_elements.clone());
        *this.realized_elements.borrow_mut() = Some(measure_elements);
        realized_elements.reset_for_reuse();

        // If there is a focused element is outside the visible viewport (i.e.
        // the focused element is non-null), ensure it's measured.
        let focused_element = this.focused_element.borrow().clone();
        if let Some(focused_element) = focused_element {
            focused_element.measure(available_size);
        }

        this.calculate_desired_size(orientation, item_count, &viewport)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let Some(realized_elements) = this.realized_elements.borrow().clone() else {
            return Size::default();
        };

        this.is_in_layout.set(true);

        {
            let _in_layout = InLayoutGuard(&this.is_in_layout);
            let orientation = this.orientation();
            let mut u = realized_elements.start_u();

            // Collection changes before the realized range make its exact
            // position unstable until the next measure. `scroll_into_view`
            // can intentionally defer that measure while waiting for an
            // updated viewport, so use the same position estimate used when
            // realizing an element instead of arranging the range at NaN.
            if u.is_nan() {
                u = this.get_or_estimate_element_u(realized_elements.first_index());
            }

            let provider_visual = this.scroll_anchor_provider();
            let provider = provider_visual.as_ref().and_then(|visual| as_scroll_anchor_provider(visual));
            let this_visual: Option<Ref<Visual>> = provider.map(|_| this.to_ref().upcast());

            let mut i = 0;
            while i < realized_elements.count() {
                if let Some(e) = realized_elements.element_at(i) {
                    let size_u = realized_elements.size_u_at(i);
                    let rect = if orientation == Orientation::Horizontal {
                        Rect::new(u, 0.0, size_u, final_size.height)
                    } else {
                        Rect::new(0.0, u, final_size.width, size_u)
                    };

                    e.arrange(rect);

                    if let Some(provider) = provider {
                        if e.is_visible() && this.viewport.get().intersects(rect) {
                            // An anchor candidate must be a visual descendant
                            // of the provider: an element of this panel is.
                            if e.visual_parent() == this_visual {
                                provider.register_anchor_candidate(&e);
                            } else if let Some(logger) = Logger::try_get(LogEventLevel::Verbose, LogArea::LAYOUT) {
                                // Element might have been removed/reparented
                                // during virtualization; ignore but log for
                                // diagnostics.
                                logger.log_with_values(
                                    Some(this as &dyn Any),
                                    "RegisterAnchorCandidate ignored for {Element}: not a descendant of ScrollAnchorProvider. {Message}",
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

            // Ensure that the focused element is in the correct position.
            let focused_element = this.focused_element.borrow().clone();
            let focused_index = this.focused_index.get();
            if let Some(focused_element) = focused_element.filter(|_| focused_index >= 0) {
                let realized_end_u = u;
                let size_u = if orientation == Orientation::Horizontal {
                    focused_element.desired_size().width
                } else {
                    focused_element.desired_size().height
                };

                u = this.get_or_estimate_element_u(focused_index);

                // The focused element's position is estimated as it's outside
                // the realized range. A bad estimate must not place it over
                // the realized elements in the viewport: an element before
                // the realized range always ends at or before its start, and
                // one after it always starts at or after its end.
                if realized_elements.count() > 0 && !realized_elements.start_u().is_nan() {
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

                focused_element.arrange(rect);
            }
        }

        let event = if this.orientation() == Orientation::Horizontal {
            Self::horizontal_snap_points_changed_event()
        } else {
            Self::vertical_snap_points_changed_event()
        };
        this.raise_event(&RoutedEventArgs::with_event(event));

        final_size
    }
}

impl VisualImpl for VirtualizingStackPanel {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let mut provider = None;
        let mut current = this.visual_parent();
        while let Some(visual) = current {
            if as_scroll_anchor_provider(&visual).is_some() {
                provider = Some(visual.downgrade());
                break;
            }
            current = visual.visual_parent();
        }
        *this.scroll_anchor_provider.borrow_mut() = provider;
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        *this.scroll_anchor_provider.borrow_mut() = None;
    }
}

impl VirtualizingPanelImpl for VirtualizingStackPanel {
    fn on_items_changed(this: &Self, _items: &Rc<ItemsSourceView>, e: &ItemsChangedEventArgs<'_>) {
        this.invalidate_measure();

        // Always update special elements.
        this.update_special_elements_on_items_changed(e);

        let Some(realized_elements) = this.realized_elements.borrow().clone() else {
            return;
        };

        let update_element_index =
            |element: &Ref<Control>, old_index: i32, new_index: i32| this.update_element_index(element, old_index, new_index);
        let recycle_element_on_item_removed = |element: &Ref<Control>| this.recycle_element_on_item_removed(element);

        let mut action = e.action;
        if action == NotifyCollectionChangedAction::Move && e.old_starting_index < 0 {
            action = NotifyCollectionChangedAction::Reset;
        }

        match action {
            NotifyCollectionChangedAction::Add => {
                realized_elements.items_inserted(e.new_starting_index, e.new_items.len() as i32, update_element_index);
            }
            NotifyCollectionChangedAction::Remove => {
                realized_elements.items_removed(
                    e.old_starting_index,
                    e.old_items.len() as i32,
                    update_element_index,
                    recycle_element_on_item_removed,
                );
            }
            NotifyCollectionChangedAction::Replace => {
                realized_elements.items_replaced(
                    e.old_starting_index,
                    e.old_items.len() as i32,
                    recycle_element_on_item_removed,
                );
            }
            NotifyCollectionChangedAction::Move => {
                realized_elements.items_removed(
                    e.old_starting_index,
                    e.old_items.len() as i32,
                    update_element_index,
                    recycle_element_on_item_removed,
                );
                let mut insert_index = e.new_starting_index;

                if e.new_starting_index > e.old_starting_index {
                    insert_index -= e.old_items.len() as i32 - 1;
                }

                realized_elements.items_inserted(insert_index, e.new_items.len() as i32, update_element_index);
            }
            NotifyCollectionChangedAction::Reset => {
                realized_elements.items_reset(recycle_element_on_item_removed);
            }
        }
    }

    fn on_items_control_changed(this: &Self, old_value: Option<&Ref<ItemsControl>>) {
        Self::parent_on_items_control_changed(this, old_value);

        if old_value.is_some() {
            if let Some(subscription) = this.items_control_property_changed.take() {
                subscription.dispose();
            }
        }
        let items_control = this.items_control();
        if let Some(items_control) = &items_control {
            let weak = this.to_ref().downgrade();
            let subscription = items_control.property_changed(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_items_control_property_changed(e);
                }
            });
            *this.items_control_property_changed.borrow_mut() = Some(subscription);
        }

        let realized_elements = this.realized_elements.borrow().clone();
        if let Some(realized_elements) = realized_elements {
            realized_elements.reset_for_reuse();
        }
        let measure_elements = this.measure_elements.borrow().clone();
        if let Some(measure_elements) = measure_elements {
            measure_elements.reset_for_reuse();
        }
        if items_control.is_some() && this.focused_element.borrow().is_some() {
            this.recycle_focused_element();
        }
        if items_control.is_some() && this.scroll_to_element.borrow().is_some() {
            this.recycle_scroll_to_element();
        }
        if items_control.is_none() {
            *this.focused_element.borrow_mut() = None;
            *this.scroll_to_element.borrow_mut() = None;
        }
        this.focused_index.set(-1);
        this.scroll_to_index.set(-1);
    }

    fn get_control_in_direction(
        this: &Self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let count = this.items().count() as i32;
        let from_control = from.and_then(|from| from.cast::<Control>());

        if count == 0
            || (from_control.is_none()
                && direction != NavigationDirection::First
                && direction != NavigationDirection::Last)
        {
            return None;
        }

        let horiz = this.orientation() == Orientation::Horizontal;
        let from_index = match &from_control {
            Some(from_control) => this.index_from_container(from_control),
            None => -1,
        };
        let mut to_index = from_index;

        match direction {
            NavigationDirection::First => to_index = 0,
            NavigationDirection::Last => to_index = count - 1,
            NavigationDirection::Next => to_index += 1,
            NavigationDirection::Previous => to_index -= 1,
            NavigationDirection::Left => {
                if horiz {
                    to_index -= 1;
                }
            }
            NavigationDirection::Right => {
                if horiz {
                    to_index += 1;
                }
            }
            NavigationDirection::Up => {
                if !horiz {
                    to_index -= 1;
                }
            }
            NavigationDirection::Down => {
                if !horiz {
                    to_index += 1;
                }
            }
            _ => return None,
        }

        if from_index == to_index {
            return from.cloned();
        }

        if wrap {
            if to_index < 0 {
                to_index = count - 1;
            } else if to_index >= count {
                to_index = 0;
            }
        }

        this.scroll_into_view(to_index).map(|element| element.upcast())
    }

    fn get_realized_containers(this: &Self) -> Option<Vec<Ref<Control>>> {
        let realized_elements = this.realized_elements.borrow().clone()?;
        let elements = realized_elements.elements();
        Some(elements.iter().flatten().cloned().collect())
    }

    fn container_from_index(this: &Self, index: i32) -> Option<Ref<Control>> {
        let items = this.items();

        if index < 0 || index as usize >= items.count() {
            return None;
        }
        if this.scroll_to_index.get() == index {
            return this.scroll_to_element.borrow().clone();
        }
        if this.focused_index.get() == index {
            return this.focused_element.borrow().clone();
        }
        if index == this.realizing_index.get() {
            return this.realizing_element.borrow().clone();
        }
        if let Some(realized) = this.get_realized_element(index) {
            return Some(realized);
        }
        if let Some(c) = items.get_at(index as usize).as_ref().and_then(Control::from_boxed) {
            if c.get_value(Self::recycle_key_property()) == Some(Self::item_is_its_own_container()) {
                return Some(c);
            }
        }
        None
    }

    fn index_from_container(this: &Self, container: &Ref<Control>) -> i32 {
        if this.scroll_to_element.borrow().as_ref() == Some(container) {
            return this.scroll_to_index.get();
        }
        if this.focused_element.borrow().as_ref() == Some(container) {
            return this.focused_index.get();
        }
        if this.realizing_element.borrow().as_ref() == Some(container) {
            return this.realizing_index.get();
        }
        let realized_elements = this.realized_elements.borrow().clone();
        realized_elements.map_or(-1, |realized_elements| realized_elements.get_index(container))
    }

    fn scroll_into_view(this: &Self, index: i32) -> Option<Ref<Control>> {
        let items = this.items();

        if this.is_in_layout.get()
            || index < 0
            || index as usize >= items.count()
            || this.realized_elements.borrow().is_none()
            || !this.is_effectively_visible()
        {
            return None;
        }

        if let Some(element) = this.get_realized_element(index) {
            element.bring_into_view();
            return Some(element);
        } else if let Some(root) = this.get_layout_root() {
            // Create and measure the element to be brought into view. Store
            // it in a field so that it can be re-used in the layout pass.
            let generator = this.generator();
            let scroll_to_element = this.get_or_create_element(&items, &generator, index);

            scroll_to_element.measure(Size::INFINITY);

            // Get the expected position of the element and put it in place.
            let anchor_u = this.get_or_estimate_element_u(index);
            let desired_size = scroll_to_element.desired_size();
            let rect = if this.orientation() == Orientation::Horizontal {
                Rect::new(anchor_u, 0.0, desired_size.width, desired_size.height)
            } else {
                Rect::new(0.0, anchor_u, desired_size.width, desired_size.height)
            };
            scroll_to_element.arrange(rect);

            // Store the element and index so that they can be used in the
            // layout pass.
            *this.scroll_to_element.borrow_mut() = Some(scroll_to_element.clone());
            this.scroll_to_index.set(index);

            // If the item being brought into view was added since the last
            // layout pass then our bounds won't be updated, so any containing
            // scroll viewers will not have an updated extent. Do a layout
            // pass to ensure that the containing scroll viewers will be able
            // to scroll the new item into view.
            if !this.bounds().contains_rect(rect) && !this.viewport.get().contains_rect(rect) {
                this.is_waiting_for_viewport_update.set(true);
                root.layout_manager().execute_layout_pass();
                this.is_waiting_for_viewport_update.set(false);
            }

            // Try to bring the item into view.
            scroll_to_element.bring_into_view();

            // If the viewport does not contain the item to scroll to, set
            // the waiting flag: this should cause the following chain of
            // events:
            // - Measure is first done with the old viewport (which will be a
            //   no-op, see `measure_override`)
            // - The viewport is then updated by the layout system which
            //   invalidates our measure
            // - Measure is then done with the new viewport.
            this.is_waiting_for_viewport_update.set(!this.viewport.get().contains_rect(rect));
            root.layout_manager().execute_layout_pass();

            // If for some reason the layout system didn't give us a new
            // viewport during the layout, we need to do another layout pass
            // as the one that took place was a no-op.
            if this.is_waiting_for_viewport_update.get() {
                this.is_waiting_for_viewport_update.set(false);
                this.invalidate_measure();
                root.layout_manager().execute_layout_pass();
            }

            // During the previous bring into view, the scroll width extent
            // might have been out of date if elements have different widths.
            // Because of that, the scroll viewer might not scroll to the
            // correct offset. After the previous bring into view, Y offset
            // should be correct and an extra layout pass has been executed,
            // hence the width extent should be correct now, and we can try to
            // scroll again.
            scroll_to_element.bring_into_view();

            // A layout pass normally adopts the temporary element into the
            // realized range and clears the scroll-to element. If it cannot
            // (for example, because the containing pane has no usable
            // width), the element remains an internal child. Recycle only
            // the temporary element created by this call so it cannot remain
            // visible and unindexed.
            if this.scroll_to_element.borrow().as_ref() == Some(&scroll_to_element) {
                let scroll_to_index = this.scroll_to_index.get();
                *this.scroll_to_element.borrow_mut() = None;
                this.scroll_to_index.set(-1);
                this.recycle_element(&scroll_to_element, scroll_to_index);
                return None;
            }

            return Some(scroll_to_element);
        }

        None
    }
}

ferroui_base::ferro_properties! { impl VirtualizingStackPanel, also [
    VirtualizingStackPanel::cache_length_property,
    VirtualizingStackPanel::recycle_key_property,
] {
    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            StackPanel::orientation_property().add_owner::<VirtualizingStackPanel>()
        }
    );

    ferro_property!(
        /// Defines the `AreHorizontalSnapPointsRegular` property.
        pub fn are_horizontal_snap_points_regular_property() -> StyledProperty<bool> {
            FerroProperty::register::<VirtualizingStackPanel, _>("AreHorizontalSnapPointsRegular", false)
        }
    );

    ferro_property!(
        /// Defines the `AreVerticalSnapPointsRegular` property.
        pub fn are_vertical_snap_points_regular_property() -> StyledProperty<bool> {
            FerroProperty::register::<VirtualizingStackPanel, _>("AreVerticalSnapPointsRegular", false)
        }
    );
} }

impl VirtualizingStackPanel {
    ferro_routed_event!(
        /// Defines the `HorizontalSnapPointsChanged` event.
        pub fn horizontal_snap_points_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<VirtualizingStackPanel, _>("HorizontalSnapPointsChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `VerticalSnapPointsChanged` event.
        pub fn vertical_snap_points_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<VirtualizingStackPanel, _>("VerticalSnapPointsChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_property!(for VirtualizingStackPanel;
        /// Defines the `CacheLength` property.
        pub fn cache_length_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<VirtualizingStackPanel, _>(
                "CacheLength",
                StyledPropertyOptions::new(0.0).validate(|v| *v >= 0.0 && *v <= 2.0),
            )
        }
    );

    ferro_property!(for VirtualizingStackPanel;
        fn recycle_key_property() -> AttachedProperty<Option<RecycleKey>> {
            FerroProperty::register_attached::<VirtualizingStackPanel, Control, _>("RecycleKey", None)
        }
    );

    /// The recycle key that marks an item which is its own container.
    fn item_is_its_own_container() -> RecycleKey {
        thread_local! {
            static KEY: RecycleKey = RecycleKey::new_unique();
        }
        KEY.with(|key| *key)
    }

    fn static_constructor() {
        register_scroll_snap_points_info::<VirtualizingStackPanel>();
        Self::cache_length_property()
            .changed()
            .add_class_handler::<VirtualizingStackPanel>(|x, e| x.on_cache_length_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: VirtualizingPanel::construct(),
            scroll_to_index: Cell::new(-1),
            scroll_to_element: RefCell::new(None),
            is_in_layout: Cell::new(false),
            is_waiting_for_viewport_update: Cell::new(false),
            last_estimated_element_size_u: Cell::new(25.0),
            measure_elements: RefCell::new(None),
            realized_elements: RefCell::new(None),
            scroll_anchor_provider: RefCell::new(None),
            viewport: Cell::new(Rect::default()),
            recycle_pool: RefCell::new(HashMap::new()),
            focused_element: RefCell::new(None),
            focused_index: Cell::new(-1),
            realizing_element: RefCell::new(None),
            realizing_index: Cell::new(-1),
            buffer_factor: Cell::new(0.0),
            has_reached_start: Cell::new(false),
            has_reached_end: Cell::new(false),
            last_measured_extended_viewport: Cell::new(Rect::default()),
            last_known_extended_viewport: Cell::new(Rect::default()),
            items_control_property_changed: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the axis along which items are laid out.
    ///
    /// One of the enumeration values that specifies the axis along which
    /// items are laid out. The default is vertical.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// Occurs when the measurements for horizontal snap points change.
    pub fn horizontal_snap_points_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::horizontal_snap_points_changed_event(), handler)
    }

    /// Occurs when the measurements for vertical snap points change.
    pub fn vertical_snap_points_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::vertical_snap_points_changed_event(), handler)
    }

    /// Gets or sets whether the horizontal snap points for the
    /// [`VirtualizingStackPanel`] are equidistant from each other.
    pub fn are_horizontal_snap_points_regular(&self) -> bool {
        self.get_value(Self::are_horizontal_snap_points_regular_property())
    }

    pub fn set_are_horizontal_snap_points_regular(&self, value: bool) {
        self.set_value(Self::are_horizontal_snap_points_regular_property(), value)
    }

    /// Gets or sets whether the vertical snap points for the
    /// [`VirtualizingStackPanel`] are equidistant from each other.
    pub fn are_vertical_snap_points_regular(&self) -> bool {
        self.get_value(Self::are_vertical_snap_points_regular_property())
    }

    pub fn set_are_vertical_snap_points_regular(&self, value: bool) {
        self.set_value(Self::are_vertical_snap_points_regular_property(), value)
    }

    /// Gets or sets the cache length.
    ///
    /// The factor determines how much additional space to maintain above and
    /// below the viewport. A value of 0.5 means half the viewport size will
    /// be buffered on each side (up-down or left-right). This uses more
    /// memory as more UI elements are realized, but greatly reduces the
    /// number of measure-arrange cycles which can cause heavy memory
    /// pressure depending on the complexity of the item layouts.
    pub fn cache_length(&self) -> f64 {
        self.get_value(Self::cache_length_property())
    }

    pub fn set_cache_length(&self, value: f64) {
        self.set_value(Self::cache_length_property(), value)
    }

    /// Gets the index of the first realized element, or -1 if no elements
    /// are realized.
    pub fn first_realized_index(&self) -> i32 {
        let realized_elements = self.realized_elements.borrow();
        realized_elements.as_ref().map_or(-1, |realized_elements| realized_elements.first_index())
    }

    /// Gets the index of the last realized element, or -1 if no elements are
    /// realized.
    pub fn last_realized_index(&self) -> i32 {
        let realized_elements = self.realized_elements.borrow();
        realized_elements.as_ref().map_or(-1, |realized_elements| realized_elements.last_index())
    }

    /// Returns the viewport that contains any visible elements.
    #[allow(dead_code)]
    pub(crate) fn view_port(&self) -> Rect {
        self.viewport.get()
    }

    /// Returns the extended viewport that contains any visible elements and
    /// the additional elements for fast scrolling (viewport * cache length *
    /// 2).
    #[allow(dead_code)]
    pub(crate) fn last_measured_extended_view_port(&self) -> Rect {
        self.last_measured_extended_viewport.get()
    }

    /// The realized elements, with an empty slot for each item of the
    /// realized range that has no element yet.
    #[allow(dead_code)]
    pub(crate) fn get_realized_elements(&self) -> Vec<Option<Ref<Control>>> {
        let realized_elements = self.realized_elements.borrow();
        realized_elements.as_ref().map_or_else(Vec::new, |realized_elements| realized_elements.elements().clone())
    }

    /// The ancestor implementing the scroll anchor provider contract.
    #[inline]
    fn scroll_anchor_provider(&self) -> Option<Ref<Visual>> {
        self.scroll_anchor_provider.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The generator of the items control. Panics if the panel does not
    /// belong to an items control.
    #[inline]
    fn generator(&self) -> Rc<ItemContainerGenerator> {
        match self.item_container_generator() {
            Some(generator) => generator,
            None => panic!("The VirtualizingPanel does not belong to an ItemsControl."),
        }
    }

    fn update_special_elements_on_items_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        let new_count = e.new_items.len() as i32;
        let old_count = e.old_items.len() as i32;

        let mut action = e.action;
        if action == NotifyCollectionChangedAction::Move && e.old_starting_index < 0 {
            action = NotifyCollectionChangedAction::Reset;
        }

        match action {
            NotifyCollectionChangedAction::Add => {
                let focused_element = self.focused_element.borrow().clone();
                if let Some(focused_element) = focused_element {
                    if e.new_starting_index <= self.focused_index.get() {
                        let old_index = self.focused_index.get();
                        self.focused_index.set(old_index + new_count);
                        self.update_element_index(&focused_element, old_index, self.focused_index.get());
                    }
                }
                if self.scroll_to_element.borrow().is_some() && e.new_starting_index <= self.scroll_to_index.get() {
                    self.scroll_to_index.set(self.scroll_to_index.get() + new_count);
                }
            }
            NotifyCollectionChangedAction::Remove => {
                let focused_element = self.focused_element.borrow().clone();
                if let Some(focused_element) = focused_element {
                    let focused_index = self.focused_index.get();
                    if e.old_starting_index <= focused_index && focused_index < e.old_starting_index + old_count {
                        self.recycle_focused_element();
                    } else if e.old_starting_index < focused_index {
                        let old_index = focused_index;
                        self.focused_index.set(focused_index - old_count);
                        self.update_element_index(&focused_element, old_index, self.focused_index.get());
                    }
                }
                if self.scroll_to_element.borrow().is_some() {
                    let scroll_to_index = self.scroll_to_index.get();
                    if e.old_starting_index <= scroll_to_index && scroll_to_index < e.old_starting_index + old_count {
                        self.recycle_scroll_to_element();
                    } else if e.old_starting_index < scroll_to_index {
                        self.scroll_to_index.set(scroll_to_index - old_count);
                    }
                }
            }
            NotifyCollectionChangedAction::Replace => {
                let focused_index = self.focused_index.get();
                if self.focused_element.borrow().is_some()
                    && e.old_starting_index <= focused_index
                    && focused_index < e.old_starting_index + old_count
                {
                    self.recycle_focused_element();
                }
                let scroll_to_index = self.scroll_to_index.get();
                if self.scroll_to_element.borrow().is_some()
                    && e.old_starting_index <= scroll_to_index
                    && scroll_to_index < e.old_starting_index + old_count
                {
                    self.recycle_scroll_to_element();
                }
            }
            NotifyCollectionChangedAction::Move => {
                let focused_element = self.focused_element.borrow().clone();
                if let Some(focused_element) = focused_element {
                    let focused_index = self.focused_index.get();
                    if e.old_starting_index <= focused_index && focused_index < e.old_starting_index + old_count {
                        let old_index = focused_index;
                        self.focused_index.set(e.new_starting_index + (focused_index - e.old_starting_index));
                        self.update_element_index(&focused_element, old_index, self.focused_index.get());
                    } else {
                        let mut new_focused_index = focused_index;

                        if e.old_starting_index < focused_index {
                            new_focused_index -= old_count;
                        }

                        if e.new_starting_index <= new_focused_index {
                            new_focused_index += new_count;
                        }

                        if new_focused_index != focused_index {
                            let old_index = focused_index;
                            self.focused_index.set(new_focused_index);
                            self.update_element_index(&focused_element, old_index, new_focused_index);
                        }
                    }
                }

                if self.scroll_to_element.borrow().is_some() {
                    let scroll_to_index = self.scroll_to_index.get();
                    if e.old_starting_index <= scroll_to_index && scroll_to_index < e.old_starting_index + old_count {
                        self.scroll_to_index.set(e.new_starting_index + (scroll_to_index - e.old_starting_index));
                    } else {
                        let mut new_scroll_to_index = scroll_to_index;

                        if e.old_starting_index < scroll_to_index {
                            new_scroll_to_index -= old_count;
                        }

                        if e.new_starting_index <= new_scroll_to_index {
                            new_scroll_to_index += new_count;
                        }

                        self.scroll_to_index.set(new_scroll_to_index);
                    }
                }
            }
            NotifyCollectionChangedAction::Reset => {
                if self.focused_element.borrow().is_some() {
                    self.recycle_focused_element();
                }
                if self.scroll_to_element.borrow().is_some() {
                    self.recycle_scroll_to_element();
                }
            }
        }
    }

    fn calculate_measure_viewport(
        &self,
        orientation: Orientation,
        item_count: i32,
        realized_elements: &RealizedStackElements,
    ) -> MeasureViewport {
        // Use the extended viewport for calculations.
        let viewport = self.last_measured_extended_viewport.get();

        // Get the viewport in the orientation direction.
        let viewport_start = if orientation == Orientation::Horizontal { viewport.x } else { viewport.y };
        let viewport_end = if orientation == Orientation::Horizontal { viewport.right() } else { viewport.bottom() };

        // Get or estimate the anchor element from which to start realization.
        // If we are scrolling to an element, use that as the anchor element.
        // Otherwise, estimate the anchor element based on the current
        // viewport.
        let anchor_index;
        let anchor_u;

        let scroll_to_bounds = if self.scroll_to_index.get() >= 0 {
            self.scroll_to_element.borrow().as_ref().map(|element| element.bounds())
        } else {
            None
        };

        if let Some(bounds) = scroll_to_bounds {
            anchor_index = self.scroll_to_index.get();
            anchor_u = if orientation == Orientation::Horizontal { bounds.left() } else { bounds.top() };
        } else {
            (anchor_index, anchor_u) =
                self.get_or_estimate_anchor_element_for_viewport(viewport_start, viewport_end, item_count);
        }

        // Check if the anchor element is not within the currently realized
        // elements.
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

    fn calculate_desired_size(&self, orientation: Orientation, item_count: i32, viewport: &MeasureViewport) -> Size {
        let mut size_u = 0.0;
        let size_v = viewport.measured_v;

        if viewport.last_index >= 0 {
            let remaining = item_count - viewport.last_index - 1;
            size_u = viewport.realized_end_u + (remaining as f64 * self.last_estimated_element_size_u.get());
        }

        if orientation == Orientation::Horizontal {
            Size::new(size_u, size_v)
        } else {
            Size::new(size_v, size_u)
        }
    }

    fn estimate_desired_size(&self, orientation: Orientation, item_count: i32) -> Size {
        let scroll_to_bounds = if self.scroll_to_index.get() >= 0 {
            self.scroll_to_element.borrow().as_ref().map(|element| element.bounds())
        } else {
            None
        };

        if let Some(bounds) = scroll_to_bounds {
            // We have an element to scroll to, so we can estimate the desired
            // size based on the element's position and the remaining
            // elements.
            let remaining = item_count - self.scroll_to_index.get() - 1;
            let u = if orientation == Orientation::Horizontal { bounds.right() } else { bounds.bottom() };
            let size_u = u + (remaining as f64 * self.last_estimated_element_size_u.get());
            return if orientation == Orientation::Horizontal {
                Size::new(size_u, self.desired_size().height)
            } else {
                Size::new(self.desired_size().width, size_u)
            };
        }

        self.desired_size()
    }

    fn estimate_element_size_u(&self) -> f64 {
        let realized_elements = self.realized_elements.borrow();
        let Some(realized_elements) = realized_elements.as_ref() else {
            return self.last_estimated_element_size_u.get();
        };

        let orientation = self.orientation();
        let mut total = 0.0;
        let mut divisor = 0.0;

        // Average the desired size of the realized, measured elements.
        for element in realized_elements.elements().iter() {
            let Some(element) = element else { continue };
            if !element.is_measure_valid() {
                continue;
            }
            let size_u = if orientation == Orientation::Horizontal {
                element.desired_size().width
            } else {
                element.desired_size().height
            };
            total += size_u;
            divisor += 1.0;
        }

        // Check we have enough information on which to base our estimate.
        if divisor == 0.0 || total == 0.0 {
            return self.last_estimated_element_size_u.get();
        }

        // Store and return the estimate.
        let estimate = total / divisor;
        self.last_estimated_element_size_u.set(estimate);
        estimate
    }

    /// Returns the index and the position of the anchor element.
    fn get_or_estimate_anchor_element_for_viewport(
        &self,
        viewport_start_u: f64,
        viewport_end_u: f64,
        item_count: i32,
    ) -> (i32, f64) {
        // We have no elements, or we're at the start of the viewport.
        if item_count <= 0 || MathUtilities::is_zero(viewport_start_u) {
            return (0, 0.0);
        }

        let horizontal = self.orientation() == Orientation::Horizontal;
        let viewport = self.viewport.get();
        let viewport_end = if horizontal { viewport.right() } else { viewport.bottom() };

        if !self.has_reached_end.get()
            && MathUtilities::greater_than_or_close(
                viewport_end,
                if horizontal { self.bounds().width } else { self.bounds().height },
            )
        {
            return (item_count - 1, viewport_end - self.estimate_element_size_u());
        }

        // If we have realised elements and a valid start position then try
        // to use this information to get the anchor element.
        {
            let realized_elements = self.realized_elements.borrow();
            if let Some(realized_elements) = realized_elements.as_ref() {
                let mut u = realized_elements.start_u();
                if !u.is_nan() {
                    let elements = realized_elements.elements();

                    for (i, element) in elements.iter().enumerate() {
                        let Some(element) = element else { continue };

                        let size_u =
                            if horizontal { element.desired_size().width } else { element.desired_size().height };
                        let end_u = u + size_u;

                        if end_u > viewport_start_u && u < viewport_end_u {
                            return (realized_elements.first_index() + i as i32, u);
                        }

                        u = end_u;
                    }
                }
            }
        }

        // We don't have any realized elements in the requested viewport, or
        // can't rely on the start position being valid. Estimate the index
        // using only the estimated element size.
        let estimated_size = self.estimate_element_size_u();

        // Estimate the element at the start of the viewport.
        let start_index = ((viewport_start_u / estimated_size) as i32).min(item_count - 1);
        (start_index, start_index as f64 * estimated_size)
    }

    fn get_or_estimate_element_u(&self, index: i32) -> f64 {
        let realized_elements = self.realized_elements.borrow().clone();

        // Return the position of the existing element if realized.
        let u = realized_elements.as_ref().map_or(f64::NAN, |realized| realized.get_element_u(index));

        if !u.is_nan() {
            return u;
        }

        // Estimate the element size.
        let estimated_size = self.estimate_element_size_u();

        // If we have a valid start position, use it to anchor estimates
        // relative to the realized range.
        if let Some(realized) = realized_elements.as_ref().filter(|realized| !realized.start_u().is_nan()) {
            let first = realized.first_index();
            let last = realized.last_index();

            if index < first {
                // Interpolate between the known panel origin and the first
                // realized item. Using the realized items' average size to
                // extrapolate backwards can place the target before the
                // panel origin.
                return realized.start_u() * index as f64 / first as f64;
            }

            if index > last {
                let sizes = realized.size_u();
                let mut realized_span = 0.0;

                for size_u in sizes.iter() {
                    realized_span += if size_u.is_nan() { estimated_size } else { *size_u };
                }

                return realized.start_u() + realized_span + ((index - last - 1) as f64 * estimated_size);
            }
        }

        index as f64 * estimated_size
    }

    fn realize_elements(
        &self,
        items: &ItemsSourceView,
        available_size: Size,
        viewport: &mut MeasureViewport,
        realized_elements: &RealizedStackElements,
        measure_elements: &RealizedStackElements,
    ) {
        let item_count = items.count() as i32;
        debug_assert!(item_count > 0);

        let generator = self.generator();
        let mut index = viewport.anchor_index;
        let horizontal = self.orientation() == Orientation::Horizontal;
        let mut u = viewport.anchor_u;
        let viewport_end = if horizontal { self.viewport.get().right() } else { self.viewport.get().bottom() };

        let anchor_at_end = !self.has_reached_end.get()
            && index == item_count - 1
            && !MathUtilities::is_zero(viewport.viewport_u_start)
            && MathUtilities::greater_than_or_close(
                viewport_end,
                if horizontal { self.bounds().width } else { self.bounds().height },
            );

        // Reset boundary flags.
        self.has_reached_start.set(false);
        self.has_reached_end.set(false);

        // If the anchor element is at the beginning of, or before, the start
        // of the viewport then we can recycle all elements before it.
        if u <= viewport.anchor_u {
            realized_elements
                .recycle_elements_before(viewport.anchor_index, |element, index| self.recycle_element(element, index));
        }

        // Start at the anchor element and move forwards, realizing elements.
        loop {
            self.realizing_index.set(index);
            let e = self.get_or_create_element(items, &generator, index);
            *self.realizing_element.borrow_mut() = Some(e.clone());

            e.measure(available_size);

            let desired_size = e.desired_size();
            let size_u = if horizontal { desired_size.width } else { desired_size.height };
            let size_v = if horizontal { desired_size.height } else { desired_size.width };

            if anchor_at_end && index == viewport.anchor_index {
                u = viewport_end - size_u;
                viewport.anchor_u = u;
            }

            measure_elements.add(index, e, u, size_u);
            viewport.measured_v = viewport.measured_v.max(size_v);

            u += size_u;
            index += 1;
            self.realizing_index.set(-1);
            *self.realizing_element.borrow_mut() = None;

            if !(u < viewport.viewport_u_end && index < item_count) {
                break;
            }
        }

        // Check if we reached the end of the collection.
        self.has_reached_end.set(index >= item_count);

        // Store the last index and end U position for the desired size
        // calculation.
        viewport.last_index = index - 1;
        viewport.realized_end_u = u;

        // We can now recycle elements after the last element.
        realized_elements
            .recycle_elements_after(viewport.last_index, |element, index| self.recycle_element(element, index));

        // Next move backwards from the anchor element, realizing elements.
        index = viewport.anchor_index - 1;
        u = viewport.anchor_u;

        while u > viewport.viewport_u_start && index >= 0 {
            let e = self.get_or_create_element(items, &generator, index);

            e.measure(available_size);
            let desired_size = e.desired_size();
            let size_u = if horizontal { desired_size.width } else { desired_size.height };
            let size_v = if horizontal { desired_size.height } else { desired_size.width };
            u -= size_u;

            measure_elements.add(index, e, u, size_u);
            viewport.measured_v = viewport.measured_v.max(size_v);
            index -= 1;
        }

        // Check if we reached the start of the collection.
        self.has_reached_start.set(index < 0);

        // We can now recycle elements before the first element.
        realized_elements.recycle_elements_before(index + 1, |element, index| self.recycle_element(element, index));
    }

    fn get_or_create_element(
        &self,
        items: &ItemsSourceView,
        generator: &ItemContainerGenerator,
        index: i32,
    ) -> Ref<Control> {
        if let Some(realized) = self
            .get_realized_element(index)
            .or_else(|| Self::get_realized_special_element(index, &self.focused_index, &self.focused_element))
            .or_else(|| Self::get_realized_special_element(index, &self.scroll_to_index, &self.scroll_to_element))
        {
            return realized;
        }

        let item = items.get_at(index as usize);
        let (needs_container, recycle_key) = generator.needs_container(&item, index);

        if needs_container {
            match self.get_recycled_element(generator, &item, index, recycle_key) {
                Some(recycled) => recycled,
                None => self.create_element(generator, &item, index, recycle_key),
            }
        } else {
            self.get_item_as_own_container(generator, &item, index)
        }
    }

    #[inline]
    fn get_realized_element(&self, index: i32) -> Option<Ref<Control>> {
        let realized_elements = self.realized_elements.borrow();
        realized_elements.as_ref().and_then(|realized_elements| realized_elements.get_element(index))
    }

    #[inline]
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

    fn get_item_as_own_container(
        &self,
        generator: &ItemContainerGenerator,
        item: &Option<BoxedValue>,
        index: i32,
    ) -> Ref<Control> {
        let control_item = match item.as_ref().and_then(Control::from_boxed) {
            Some(control_item) => control_item,
            None => panic!("An item that is its own container must be a control."),
        };

        if !control_item.is_set(Self::recycle_key_property().as_property()) {
            generator.prepare_item_container(&control_item, item, index);
            self.add_internal_child(&control_item);
            control_item.set_value(Self::recycle_key_property(), Some(Self::item_is_its_own_container()));
            generator.item_container_prepared(&control_item, item, index);
        }

        control_item.set_current_value(Visual::is_visible_property(), true);
        control_item
    }

    fn get_recycled_element(
        &self,
        generator: &ItemContainerGenerator,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Option<Ref<Control>> {
        let recycle_key = recycle_key?;

        let recycled = self.recycle_pool.borrow_mut().get_mut(&recycle_key).and_then(Vec::pop)?;
        ferroui_base::perf_count!(ContainersReused);

        recycled.set_current_value(Visual::is_visible_property(), true);
        generator.prepare_item_container(&recycled, item, index);
        self.add_internal_child(&recycled);
        generator.item_container_prepared(&recycled, item, index);
        Some(recycled)
    }

    fn create_element(
        &self,
        generator: &ItemContainerGenerator,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        ferroui_base::perf_count!(ContainersCreated);
        let container = generator.create_container(item, index, recycle_key);

        container.set_value(Self::recycle_key_property(), recycle_key);
        generator.prepare_item_container(&container, item, index);
        self.add_internal_child(&container);
        generator.item_container_prepared(&container, item, index);

        container
    }

    fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(provider_visual) = self.scroll_anchor_provider() {
            if let Some(provider) = as_scroll_anchor_provider(&provider_visual) {
                provider.unregister_anchor_candidate(element);
            }
        }
    }

    fn recycle_element(&self, element: &Ref<Control>, index: i32) {
        debug_assert!(self.items_control().is_some());

        self.unregister_anchor_candidate(element);

        let recycle_key = element.get_value(Self::recycle_key_property());

        match recycle_key {
            None => {
                self.generator().clear_item_container(element);
                self.remove_internal_child(element);
            }
            Some(recycle_key) if recycle_key == Self::item_is_its_own_container() => {
                element.set_current_value(Visual::is_visible_property(), false);
            }
            Some(recycle_key) => {
                let tab_once_active_element =
                    self.items_control().and_then(|items_control| KeyboardNavigation::get_tab_once_active_element(&items_control));

                if tab_once_active_element.is_some_and(|active| active == *element) {
                    *self.focused_element.borrow_mut() = Some(element.clone());
                    self.focused_index.set(index);
                } else {
                    ferroui_base::perf_count!(ContainersRecycled);
                    self.generator().clear_item_container(element);
                    self.push_to_recycle_pool(recycle_key, element);
                    element.set_current_value(Visual::is_visible_property(), false);
                    self.remove_internal_child(element);
                }
            }
        }
    }

    fn recycle_element_on_item_removed(&self, element: &Ref<Control>) {
        self.unregister_anchor_candidate(element);

        let recycle_key = element.get_value(Self::recycle_key_property());

        match recycle_key {
            None => {
                self.generator().clear_item_container(element);
                self.remove_internal_child(element);
            }
            Some(recycle_key) if recycle_key == Self::item_is_its_own_container() => {
                self.remove_internal_child(element);
            }
            Some(recycle_key) => {
                self.generator().clear_item_container(element);
                self.push_to_recycle_pool(recycle_key, element);
                element.set_current_value(Visual::is_visible_property(), false);
                self.remove_internal_child(element);
            }
        }
    }

    fn recycle_focused_element(&self) {
        let focused_element = self.focused_element.borrow().clone();
        if let Some(focused_element) = focused_element {
            self.recycle_element_on_item_removed(&focused_element);
        }
        *self.focused_element.borrow_mut() = None;
        self.focused_index.set(-1);
    }

    fn recycle_scroll_to_element(&self) {
        let scroll_to_element = self.scroll_to_element.borrow().clone();
        if let Some(scroll_to_element) = scroll_to_element {
            self.recycle_element_on_item_removed(&scroll_to_element);
        }
        *self.scroll_to_element.borrow_mut() = None;
        self.scroll_to_index.set(-1);
    }

    fn push_to_recycle_pool(&self, recycle_key: RecycleKey, element: &Ref<Control>) {
        self.recycle_pool.borrow_mut().entry(recycle_key).or_default().push(element.clone());
    }

    fn update_element_index(&self, element: &Ref<Control>, old_index: i32, new_index: i32) {
        self.generator().item_container_index_changed(element, old_index, new_index);
    }

    fn calculate_extended_viewport(&self, vertical: bool, _viewport_size: f64, buffer_size: f64) -> Rect {
        let viewport = self.viewport.get();
        let bounds = self.bounds();

        let mut extended_viewport_start = if vertical {
            (viewport.top() - buffer_size).max(0.0)
        } else {
            (viewport.left() - buffer_size).max(0.0)
        };

        let mut extended_viewport_end = if vertical {
            bounds.height.min(viewport.bottom() + buffer_size)
        } else {
            bounds.width.min(viewport.right() + buffer_size)
        };

        // If we are at the start of the list, append 2 * cache length
        // additional items. If we are at the end of the list, prepend 2 *
        // cache length additional items - this way we always maintain "2 *
        // cache length * element" items.
        if vertical {
            let space_above = viewport.top() - buffer_size;
            let space_below = bounds.height - (viewport.bottom() + buffer_size);

            if space_above < 0.0 && space_below >= 0.0 {
                extended_viewport_end = bounds.height.min(extended_viewport_end + space_above.abs());
            }
            if space_above >= 0.0 && space_below < 0.0 {
                extended_viewport_start = (extended_viewport_start - space_below.abs()).max(0.0);
            }
        } else {
            let space_left = viewport.left() - buffer_size;
            let space_right = bounds.width - (viewport.right() + buffer_size);

            if space_left < 0.0 && space_right >= 0.0 {
                extended_viewport_end = bounds.width.min(extended_viewport_end + space_left.abs());
            }
            if space_left >= 0.0 && space_right < 0.0 {
                extended_viewport_start = (extended_viewport_start - space_right.abs()).max(0.0);
            }
        }

        if vertical {
            Rect::new(viewport.x, extended_viewport_start, viewport.width, extended_viewport_end - extended_viewport_start)
        } else {
            Rect::new(extended_viewport_start, viewport.y, extended_viewport_end - extended_viewport_start, viewport.height)
        }
    }

    fn on_effective_viewport_changed(&self, e: &EffectiveViewportChangedEventArgs) {
        let vertical = self.orientation() == Orientation::Vertical;
        let old_viewport = self.viewport.get();
        let old_extended_viewport = self.last_measured_extended_viewport.get();
        let old_viewport_start = if vertical { old_viewport.top() } else { old_viewport.left() };
        let old_viewport_end = if vertical { old_viewport.bottom() } else { old_viewport.right() };
        let old_extended_viewport_start =
            if vertical { old_extended_viewport.top() } else { old_extended_viewport.left() };
        let old_extended_viewport_end =
            if vertical { old_extended_viewport.bottom() } else { old_extended_viewport.right() };

        // Update current viewport.
        let viewport = e.effective_viewport().intersect(Rect::from_size(self.bounds().size()));
        self.viewport.set(viewport);
        self.is_waiting_for_viewport_update.set(false);

        // Calculate buffer sizes based on viewport dimensions.
        let viewport_size = if vertical { viewport.height } else { viewport.width };
        let buffer_size = viewport_size * self.buffer_factor.get();

        let extended_viewport = self.calculate_extended_viewport(vertical, viewport_size, buffer_size);

        // Determine if we need a new measure.
        let new_viewport_start = if vertical { viewport.top() } else { viewport.left() };
        let new_viewport_end = if vertical { viewport.bottom() } else { viewport.right() };
        let new_extended_viewport_start = if vertical { extended_viewport.top() } else { extended_viewport.left() };
        let new_extended_viewport_end = if vertical { extended_viewport.bottom() } else { extended_viewport.right() };

        let mut needs_measure = false;

        // Case 1: Viewport has changed significantly.
        if !MathUtilities::are_close(old_viewport_start, new_viewport_start)
            || !MathUtilities::are_close(old_viewport_end, new_viewport_end)
        {
            // Case 1a: The new viewport exceeds the old extended viewport.
            if new_viewport_start < old_extended_viewport_start || new_viewport_end > old_extended_viewport_end {
                needs_measure = true;
            }
            // Case 1b: The extended viewport has changed significantly.
            else if !MathUtilities::are_close(old_extended_viewport_start, new_extended_viewport_start)
                || !MathUtilities::are_close(old_extended_viewport_end, new_extended_viewport_end)
            {
                // Check if we're about to scroll into an area where we don't
                // have realized elements. This would be the case if we're
                // near the edge of our current extended viewport.
                let mut nearing_edge = false;

                if self.realized_elements.borrow().is_some() {
                    // If scrolling up/left and nearing the top/left edge of
                    // realized elements.
                    if new_viewport_start < old_viewport_start
                        && new_viewport_start - new_extended_viewport_start < buffer_size
                    {
                        // Edge case: We're at item 0 with excess measurement
                        // space. Skip re-measuring since we're at the list
                        // start and it won't change the result. This prevents
                        // redundant measure-arrange cycles when at list
                        // beginning.
                        nearing_edge = !self.has_reached_start.get();
                    }

                    // If scrolling down/right and nearing the bottom/right
                    // edge of realized elements.
                    if new_viewport_end > old_viewport_end
                        && new_extended_viewport_end - new_viewport_end < buffer_size
                    {
                        // Edge case: We're at the last item with excess
                        // measurement space. Skip re-measuring since we're at
                        // the list end and it won't change the result. This
                        // prevents redundant measure-arrange cycles when at
                        // list beginning.
                        nearing_edge = !self.has_reached_end.get();
                    }
                } else {
                    nearing_edge = true;
                }

                needs_measure = nearing_edge;
            }
        }

        // Supplementary check: detect viewport growth after a previous
        // shrink. The main comparison (cases 1a/1b) uses the last measured
        // extended viewport which only updates on measure. When the viewport
        // shrinks (e.g. a popup during filtering), that viewport stays
        // stale-large, masking subsequent growth. Compare against the last
        // known extended viewport (always updated) to catch this case.
        if !needs_measure {
            let last_known = self.last_known_extended_viewport.get();
            let last_known_start = if vertical { last_known.top() } else { last_known.left() };
            let last_known_end = if vertical { last_known.bottom() } else { last_known.right() };
            if new_viewport_start < last_known_start || new_viewport_end > last_known_end {
                needs_measure = true;
            }
        }

        self.last_known_extended_viewport.set(extended_viewport);

        if needs_measure {
            // Only update the measure viewport when triggering a measure.
            // This keeps the wider realization range available for
            // externally-triggered measures (e.g. from `on_items_changed`),
            // ensuring enough items are realized.
            self.last_measured_extended_viewport.set(extended_viewport);
            self.invalidate_measure();
        }
    }

    fn on_items_control_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let focused_element = self.focused_element.borrow().clone();
        let Some(focused_element) = focused_element else { return };

        if e.property() == KeyboardNavigation::tab_once_active_element_property().as_property()
            && e.get_old_value::<Option<Ref<InputElement>>>().flatten().is_some_and(|old| old == focused_element)
        {
            // The tab once active element has moved away from the focused
            // element so we can recycle it.
            self.recycle_element(&focused_element, self.focused_index.get());
            *self.focused_element.borrow_mut() = None;
            self.focused_index.set(-1);
        }
    }

    fn on_cache_length_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<f64>();
        self.buffer_factor.set(new_value);

        // Force a recalculation of the extended viewport on the next layout
        // pass.
        self.invalidate_measure();
    }

    /// Returns the set of distances between irregular snap points for a
    /// specified orientation and alignment.
    pub fn get_irregular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> Vec<f64> {
        let Some(realized_elements) = self.realized_elements.borrow().clone() else {
            return Vec::new();
        };

        let count = self
            .items_control()
            .and_then(|items_control| items_control.items_source())
            .map_or(0, |items_source| items_source.count() as i32);

        VirtualizingSnapPointsList::new(
            realized_elements,
            count,
            orientation,
            self.orientation(),
            snap_points_alignment,
            self.estimate_element_size_u(),
        )
        .iter()
        .collect()
    }

    /// Gets the distance between regular snap points for a specified
    /// orientation and alignment, and the offset of the first snap point.
    ///
    /// Panics if the snap points of the panel's orientation are not regular.
    pub fn get_regular_snap_points(
        &self,
        _orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> (f64, f64) {
        let mut offset = 0.0;
        let first_realized_child = {
            let realized_elements = self.realized_elements.borrow();
            realized_elements.as_ref().and_then(|realized_elements| realized_elements.element_at(0))
        };

        let Some(first_realized_child) = first_realized_child else {
            return (0.0, offset);
        };

        let bounds = first_realized_child.bounds();

        let snap_point = match self.orientation() {
            Orientation::Horizontal => {
                if !self.are_horizontal_snap_points_regular() {
                    panic!("The horizontal snap points are not regular.");
                }

                match snap_points_alignment {
                    SnapPointsAlignment::Near => offset = 0.0,
                    SnapPointsAlignment::Center => offset = (bounds.right() - bounds.left()) / 2.0,
                    SnapPointsAlignment::Far => offset = bounds.width,
                }
                bounds.width
            }
            Orientation::Vertical => {
                if !self.are_vertical_snap_points_regular() {
                    panic!("The vertical snap points are not regular.");
                }

                match snap_points_alignment {
                    SnapPointsAlignment::Near => offset = 0.0,
                    SnapPointsAlignment::Center => offset = (bounds.bottom() - bounds.top()) / 2.0,
                    SnapPointsAlignment::Far => offset = bounds.height,
                }
                bounds.height
            }
        };

        (snap_point, offset)
    }
}

impl IScrollSnapPointsInfo for VirtualizingStackPanel {
    fn are_horizontal_snap_points_regular(&self) -> bool {
        VirtualizingStackPanel::are_horizontal_snap_points_regular(self)
    }

    fn set_are_horizontal_snap_points_regular(&self, value: bool) {
        VirtualizingStackPanel::set_are_horizontal_snap_points_regular(self, value)
    }

    fn are_vertical_snap_points_regular(&self) -> bool {
        VirtualizingStackPanel::are_vertical_snap_points_regular(self)
    }

    fn set_are_vertical_snap_points_regular(&self, value: bool) {
        VirtualizingStackPanel::set_are_vertical_snap_points_regular(self, value)
    }

    fn get_irregular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> Vec<f64> {
        VirtualizingStackPanel::get_irregular_snap_points(self, orientation, snap_points_alignment)
    }

    fn get_regular_snap_points(
        &self,
        orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
    ) -> (f64, f64) {
        VirtualizingStackPanel::get_regular_snap_points(self, orientation, snap_points_alignment)
    }

    fn horizontal_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable> {
        self.add_disposable_handler(Self::horizontal_snap_points_changed_event(), move |sender, e| handler(sender, e))
    }

    fn vertical_snap_points_changed(&self, handler: SnapPointsChangedHandler) -> Rc<dyn IDisposable> {
        self.add_disposable_handler(Self::vertical_snap_points_changed_event(), move |sender, e| handler(sender, e))
    }
}
