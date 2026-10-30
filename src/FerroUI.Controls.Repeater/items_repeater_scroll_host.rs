// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::animation_manager::RepeaterAnimationProxy;
use super::{ItemsRepeater, VirtualizationInfo};
use ferroui_controls::{
    register_scroll_anchor_provider, ContentControl, Control, ControlImpl, IScrollAnchorProvider,
    Panel, PanelImpl, ScrollViewer,
};
use ferroui_base::animation::easings::{Easing, SplineEasing};
use ferroui_base::animation::{Animation, Cue, FillMode, IClock, IGlobalClock, KeyFrame, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationTokenSource;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroLocator,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    LocatorExtensions, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Vector, Visual,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct CandidateInfo {
    element: WeakRef<Control>,
    relative_bounds: Option<Rect>,
}

#[derive(Clone)]
struct BringIntoViewState {
    id: u64,
    target: WeakRef<Control>,
    alignment: Point,
    offset: Vector,
    animate: bool,
    change_view_called: bool,
    change_view_offset: Vector,
    pins: Rc<BringIntoViewPins>,
}

struct BringIntoViewPin {
    owner: WeakRef<ItemsRepeater>,
    element: WeakRef<Control>,
    info: Rc<VirtualizationInfo>,
    generation: u64,
}

/// One shared lease per operation, independent of the temporary layout anchor.
/// Cloning a pending request must not add or release another pin contribution.
struct BringIntoViewPins {
    pins: Vec<BringIntoViewPin>,
    released: Cell<bool>,
}

impl BringIntoViewPins {
    fn acquire(element: &Ref<Control>) -> Rc<Self> {
        let mut pins = Vec::new();
        let mut child = element.clone().upcast::<Visual>();
        while let Some(parent) = child.visual_parent() {
            if let (Some(owner), Some(element)) =
                (parent.cast::<ItemsRepeater>(), child.cast::<Control>())
            {
                if let Some(info) = ItemsRepeater::try_get_virtualization_info(&element)
                    .filter(|info| info.is_realized())
                {
                    info.add_pin();
                    let generation = info.pin_generation();
                    pins.push(BringIntoViewPin {
                        owner: owner.downgrade(),
                        element: element.downgrade(),
                        info,
                        generation,
                    });
                }
            }
            child = parent;
        }
        Rc::new(Self {
            pins,
            released: Cell::new(false),
        })
    }

    fn is_valid(&self) -> bool {
        self.pins.iter().all(|pin| {
            pin.info.pin_generation() == pin.generation
                && pin
                    .owner
                    .upgrade()
                    .zip(pin.element.upgrade())
                    .is_some_and(|(owner, element)| {
                        element.visual_parent().as_ref() == Some(&owner.upcast::<Visual>())
                    })
        })
    }

    fn release(&self) {
        if self.released.replace(true) {
            return;
        }
        for pin in &self.pins {
            if pin.info.release_scoped_pin(pin.generation) {
                if let Some(owner) = pin.owner.upgrade() {
                    owner.invalidate_measure();
                }
            }
        }
    }
}

impl Drop for BringIntoViewPins {
    fn drop(&mut self) {
        self.release();
    }
}

struct ScrollAnimation {
    id: u64,
    _proxy: Ref<RepeaterAnimationProxy>,
    cancellation: CancellationTokenSource,
}

/// Compatibility host for a repeater inside a native ScrollViewer. Its
/// candidate selection retains the source host's vertical-edge convention:
/// HorizontalAnchorRatio controls that edge; VerticalAnchorRatio round-trips
/// for compatibility. Ordinary FerroUI scrolling does not require this host.
#[repr(C)]
pub struct ItemsRepeaterScrollHost {
    base: Panel,
    candidates: RefCell<Vec<CandidateInfo>>,
    anchor: RefCell<Option<WeakRef<Control>>>,
    anchor_bounds: Cell<Option<Rect>>,
    anchor_dirty: Cell<bool>,
    viewer_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    layout_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    pending_bring: RefCell<Option<BringIntoViewState>>,
    next_bring_id: Cell<u64>,
    animation: RefCell<Option<ScrollAnimation>>,
    changing_offset: Cell<bool>,
    viewport_changed: HandlerList<dyn Fn(bool)>,
    post_arrange: HandlerList<dyn Fn()>,
    configuration_changed: HandlerList<dyn Fn()>,
}

ferro_class!(ItemsRepeaterScrollHost: Panel);
ferro_class_info!(ItemsRepeaterScrollHost {
    new: ItemsRepeaterScrollHost::new,
    markup: {
        content: ScrollViewer,
        properties: [
            CurrentAnchor: Option<Ref<Control>> { get: |this: &Ref<ItemsRepeaterScrollHost>| this.current_anchor() },
            AnchorElement: Option<Ref<Control>> { get: |this: &Ref<ItemsRepeaterScrollHost>| this.anchor_element() },
            IsHorizontallyScrollable: bool { get: |this: &Ref<ItemsRepeaterScrollHost>| this.is_horizontally_scrollable() },
            IsVerticallyScrollable: bool { get: |this: &Ref<ItemsRepeaterScrollHost>| this.is_vertically_scrollable() },
        ],
        methods: [
            fn RegisterAnchorCandidate(Ref<Control>) =>
                |this: &Ref<ItemsRepeaterScrollHost>, element: Ref<Control>| this.register_anchor_candidate(&element),
            fn UnregisterAnchorCandidate(Ref<Control>) =>
                |this: &Ref<ItemsRepeaterScrollHost>, element: Ref<Control>| this.unregister_anchor_candidate(&element),
            fn GetRelativeViewport(Ref<Control>) -> Rect =>
                |this: &Ref<ItemsRepeaterScrollHost>, element: Ref<Control>| this.get_relative_viewport(&element),
            fn StartBringIntoView(Ref<Control>, f64, f64, f64, f64, bool) =>
                |this: &Ref<ItemsRepeaterScrollHost>, element: Ref<Control>, alignment_x: f64, alignment_y: f64, offset_x: f64, offset_y: f64, animate: bool|
                    this.start_bring_into_view(&element, alignment_x, alignment_y, offset_x, offset_y, animate),
        ],
        events: [
            ViewportChanged(Option<Ref<ItemsRepeaterScrollHost>>, bool) =>
                |this: &Ref<ItemsRepeaterScrollHost>, handler: ferroui_base::metadata::MarkupDelegate| {
                    let sender = this.downgrade();
                    this.viewport_changed(move |is_final| {
                        handler.invoke(&[
                            ferroui_base::metadata::into_markup_value(sender.upgrade()),
                            ferroui_base::metadata::into_markup_value(is_final),
                        ]);
                    })
                },
            PostArrange(Option<Ref<ItemsRepeaterScrollHost>>) =>
                |this: &Ref<ItemsRepeaterScrollHost>, handler: ferroui_base::metadata::MarkupDelegate| {
                    let sender = this.downgrade();
                    this.post_arrange(move || {
                        handler.invoke(&[ferroui_base::metadata::into_markup_value(sender.upgrade())]);
                    })
                },
            ConfigurationChanged(Option<Ref<ItemsRepeaterScrollHost>>) =>
                |this: &Ref<ItemsRepeaterScrollHost>, handler: ferroui_base::metadata::MarkupDelegate| {
                    let sender = this.downgrade();
                    this.configuration_changed(move || {
                        handler.invoke(&[ferroui_base::metadata::into_markup_value(sender.upgrade())]);
                    })
                },
        ],
    },
});
ferro_impl_classes!(ItemsRepeaterScrollHost:
    StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl
);

impl VisualImpl for ItemsRepeaterScrollHost {
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.cancel_scroll_animation();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}

impl FerroObjectImpl for ItemsRepeaterScrollHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let subscription = this.layout_updated(move || {
            if let Some(this) = weak.upgrade() {
                this.on_layout_updated();
            }
        });
        *this.layout_subscription.borrow_mut() = Some(subscription);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::scroll_viewer_property().as_property() {
            this.replace_scroll_viewer(change.get_new_value::<Option<Ref<ScrollViewer>>>());
        } else if change.property() == Self::horizontal_anchor_ratio_property().as_property()
            || change.property() == Self::vertical_anchor_ratio_property().as_property()
        {
            this.invalidate_anchor();
            this.raise_configuration_changed();
            this.invalidate_arrange();
        }
    }
}

impl LayoutableImpl for ItemsRepeaterScrollHost {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if let Some(viewer) = this.scroll_viewer() {
            viewer.measure(available_size);
            viewer.desired_size()
        } else {
            Size::default()
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(viewer) = this.scroll_viewer() {
            let previous_anchor = if this.pending_bring.borrow().is_none() {
                this.current_anchor().zip(this.anchor_bounds.get())
            } else {
                None
            };
            viewer.arrange(Rect::from_size(final_size));
            if this
                .pending_bring
                .borrow()
                .as_ref()
                .is_some_and(|bring| !bring.change_view_called)
            {
                this.apply_pending_change_view(&viewer);
            } else if let Some((anchor, bounds)) = previous_anchor {
                this.track_element(&anchor, bounds, &viewer);
            }
            this.invalidate_anchor();
        }
        for (_, handler) in this.post_arrange.snapshot().iter() {
            handler();
        }
        final_size
    }
}

ferro_properties! {
    impl ItemsRepeaterScrollHost {
        pub fn scroll_viewer_property() -> StyledProperty<Option<Ref<ScrollViewer>>> {
            FerroProperty::register::<ItemsRepeaterScrollHost, _>("ScrollViewer", None)
        }
        pub fn horizontal_anchor_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ItemsRepeaterScrollHost, _>("HorizontalAnchorRatio", 0.0)
        }
        pub fn vertical_anchor_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ItemsRepeaterScrollHost, _>("VerticalAnchorRatio", 0.0)
        }
    }
}

impl ItemsRepeaterScrollHost {
    fn static_constructor() {
        register_scroll_anchor_provider::<Self>();
    }
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            candidates: RefCell::new(Vec::new()),
            anchor: RefCell::new(None),
            anchor_bounds: Cell::new(None),
            anchor_dirty: Cell::new(true),
            viewer_subscription: RefCell::new(None),
            layout_subscription: RefCell::new(None),
            pending_bring: RefCell::new(None),
            next_bring_id: Cell::new(0),
            animation: RefCell::new(None),
            changing_offset: Cell::new(false),
            viewport_changed: HandlerList::new(),
            post_arrange: HandlerList::new(),
            configuration_changed: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        let children = self.children();
        if children.is_empty() {
            None
        } else {
            children.get(0).cast()
        }
    }
    pub fn set_scroll_viewer(&self, viewer: Option<Ref<ScrollViewer>>) {
        self.set_value(Self::scroll_viewer_property(), viewer);
    }
    pub fn horizontal_anchor_ratio(&self) -> f64 {
        self.get_value(Self::horizontal_anchor_ratio_property())
    }
    pub fn set_horizontal_anchor_ratio(&self, ratio: f64) {
        self.set_value(Self::horizontal_anchor_ratio_property(), ratio);
    }
    pub fn vertical_anchor_ratio(&self) -> f64 {
        self.get_value(Self::vertical_anchor_ratio_property())
    }
    pub fn set_vertical_anchor_ratio(&self, ratio: f64) {
        self.set_value(Self::vertical_anchor_ratio_property(), ratio);
    }
    pub fn is_horizontally_scrollable(&self) -> bool {
        true
    }
    pub fn is_vertically_scrollable(&self) -> bool {
        true
    }
    pub fn current_anchor(&self) -> Option<Ref<Control>> {
        self.ensure_anchor();
        self.anchor.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub fn anchor_element(&self) -> Option<Ref<Control>> {
        self.current_anchor()
    }
    pub fn register_anchor_candidate(&self, element: &Ref<Control>) {
        if self.scroll_viewer().is_none()
            || (self.horizontal_anchor_ratio().is_nan() && self.vertical_anchor_ratio().is_nan())
        {
            return;
        }
        let mut candidates = self.candidates.borrow_mut();
        if !candidates
            .iter()
            .any(|candidate| candidate.element.upgrade().as_ref() == Some(element))
        {
            candidates.push(CandidateInfo {
                element: element.downgrade(),
                relative_bounds: None,
            });
            self.anchor_dirty.set(true);
        }
    }
    pub fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        let mut candidates = self.candidates.borrow_mut();
        let old_count = candidates.len();
        candidates.retain(|candidate| {
            candidate
                .element
                .upgrade()
                .as_ref()
                .is_some_and(|candidate| candidate != element)
        });
        if candidates.len() != old_count {
            self.anchor_dirty.set(true);
        }
    }
    pub fn get_relative_viewport(&self, element: &Ref<Control>) -> Rect {
        let Some(viewer) = self.scroll_viewer() else {
            return Rect::default();
        };
        let Some(content) = Self::content_root(&viewer) else {
            return Rect::default();
        };
        let Some(position) = element.translate_point(Point::default(), &content) else {
            return Rect::default();
        };
        // Native scroll offsets are synchronous, but child arrange can lag the
        // offset property. Work in content coordinates and subtract the latest
        // offset, avoiding the WinUI-only pending viewport shift bookkeeping.
        let offset = self
            .pending_bring
            .borrow()
            .as_ref()
            .filter(|bring| bring.change_view_called)
            .map_or_else(|| viewer.offset(), |bring| bring.change_view_offset);
        Rect::new(
            offset.x - position.x,
            offset.y - position.y,
            viewer.viewport().width,
            viewer.viewport().height,
        )
    }
    pub fn start_bring_into_view(
        &self,
        element: &Ref<Control>,
        alignment_x: f64,
        alignment_y: f64,
        offset_x: f64,
        offset_y: f64,
        animate: bool,
    ) {
        self.cancel_scroll_animation();
        let id = self.next_bring_id.get().wrapping_add(1);
        self.next_bring_id.set(id);
        *self.pending_bring.borrow_mut() = Some(BringIntoViewState {
            id,
            target: element.downgrade(),
            alignment: Point::new(alignment_x, alignment_y),
            offset: Vector::new(offset_x, offset_y),
            animate,
            change_view_called: false,
            change_view_offset: Vector::default(),
            pins: BringIntoViewPins::acquire(element),
        });
        self.invalidate_arrange();
    }
    pub fn has_pending_bring_into_view(&self) -> bool {
        self.pending_bring
            .borrow()
            .as_ref()
            .is_some_and(|bring| bring.target.upgrade().is_some())
    }
    pub fn viewport_changed(&self, handler: impl Fn(bool) + 'static) -> Rc<dyn IDisposable> {
        let token = self.viewport_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.viewport_changed.remove(token);
            }
        })
    }
    pub fn post_arrange(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.post_arrange.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.post_arrange.remove(token);
            }
        })
    }
    pub fn configuration_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.configuration_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.configuration_changed.remove(token);
            }
        })
    }
    fn raise_viewport_changed(&self, is_final: bool) {
        for (_, handler) in self.viewport_changed.snapshot().iter() {
            handler(is_final);
        }
    }
    fn raise_configuration_changed(&self) {
        for (_, handler) in self.configuration_changed.snapshot().iter() {
            handler();
        }
    }
    fn content_root(viewer: &ScrollViewer) -> Option<Ref<Control>> {
        viewer
            .presenter()
            .and_then(|presenter| presenter.child())
            .or_else(|| viewer.content().as_ref().and_then(Control::from_boxed))
    }
    fn bounds_relative_to(element: &Control, content: &Control) -> Option<Rect> {
        element
            .transform_to_visual(content)
            .map(|transform| Rect::from_size(element.bounds().size()).transform_to_aabb(transform))
    }
    fn invalidate_anchor(&self) {
        self.anchor_dirty.set(true);
        for candidate in self.candidates.borrow_mut().iter_mut() {
            candidate.relative_bounds = None;
        }
    }
    fn ensure_anchor(&self) {
        if !self.anchor_dirty.replace(false) {
            return;
        }
        *self.anchor.borrow_mut() = None;
        self.anchor_bounds.set(None);
        let Some(viewer) = self.scroll_viewer() else {
            return;
        };
        let Some(content) = Self::content_root(&viewer) else {
            return;
        };
        let ratio = self.horizontal_anchor_ratio();
        let offset = self
            .pending_bring
            .borrow()
            .as_ref()
            .filter(|bring| !bring.animate && bring.change_view_called)
            .map_or_else(|| viewer.offset().y, |bring| bring.change_view_offset.y);
        let viewport_edge = offset + ratio * viewer.viewport().height;
        let mut closest = f64::MAX;
        let mut anchor = None;
        // The first registered candidate wins a distance tie, as upstream.
        for candidate in self.candidates.borrow_mut().iter_mut() {
            let Some(element) = candidate.element.upgrade() else {
                continue;
            };
            if candidate.relative_bounds.is_none() {
                candidate.relative_bounds = Self::bounds_relative_to(&element, &content);
            }
            let Some(bounds) = candidate.relative_bounds else {
                continue;
            };
            let distance = (bounds.y + ratio * bounds.height - viewport_edge).abs();
            if distance < closest {
                closest = distance;
                anchor = Some((element.downgrade(), bounds));
            }
        }
        if let Some((element, bounds)) = anchor {
            *self.anchor.borrow_mut() = Some(element);
            self.anchor_bounds.set(Some(bounds));
        }
    }
    fn replace_scroll_viewer(&self, viewer: Option<Ref<ScrollViewer>>) {
        let initial_bring = if self.scroll_viewer().is_none() {
            self.pending_bring.borrow_mut().take()
        } else {
            None
        };
        self.cancel_scroll_animation();
        let previous = self.viewer_subscription.borrow_mut().take();
        if let Some(previous) = previous {
            previous.dispose();
        }
        self.pending_bring.borrow_mut().take();
        self.candidates.borrow_mut().clear();
        *self.anchor.borrow_mut() = None;
        self.anchor_bounds.set(None);
        self.anchor_dirty.set(true);
        self.children().clear();
        if let Some(viewer) = viewer {
            self.children().add(&viewer);
            let weak = self.to_ref().downgrade();
            let subscription = viewer.property_changed(move |change| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if change.property() == ScrollViewer::offset_property().as_property() {
                    if !this.changing_offset.get() {
                        this.cancel_scroll_animation();
                        this.pending_bring.borrow_mut().take();
                    }
                    this.invalidate_anchor();
                    this.raise_viewport_changed(!this.has_pending_bring_into_view());
                } else if change.property() == ContentControl::content_property().as_property() {
                    this.cancel_scroll_animation();
                    this.invalidate_anchor();
                } else if change.property() == ScrollViewer::viewport_property().as_property()
                    || change.property() == ScrollViewer::extent_property().as_property()
                {
                    this.invalidate_anchor();
                    this.raise_viewport_changed(true);
                } else if change.property()
                    == ScrollViewer::horizontal_scroll_bar_visibility_property().as_property()
                    || change.property()
                        == ScrollViewer::vertical_scroll_bar_visibility_property().as_property()
                {
                    this.raise_configuration_changed();
                }
            });
            *self.viewer_subscription.borrow_mut() = Some(subscription);
        }
        *self.pending_bring.borrow_mut() = initial_bring;
        self.raise_configuration_changed();
        self.invalidate_measure();
    }
    fn set_offset(&self, viewer: &ScrollViewer, offset: Vector) {
        struct Reset<'a>(&'a Cell<bool>, bool);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
        let previous = self.changing_offset.replace(true);
        let _reset = Reset(&self.changing_offset, previous);
        viewer.set_current_value(ScrollViewer::offset_property(), offset);
    }
    fn track_element(&self, element: &Control, previous: Rect, viewer: &ScrollViewer) {
        let Some(content) = Self::content_root(viewer) else {
            return;
        };
        let Some(bounds) = Self::bounds_relative_to(element, &content) else {
            return;
        };
        if self
            .anchor
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
            .as_ref()
            == Some(&element.to_ref())
        {
            self.anchor_bounds.set(Some(bounds));
        }
        let ratio = self.horizontal_anchor_ratio();
        let delta = bounds.y + ratio * bounds.height - previous.y - ratio * previous.height;
        let offset = viewer.offset();
        let target = (offset.y + delta).clamp(
            0.0,
            (viewer.extent().height - viewer.viewport().height).max(0.0),
        );
        if (target - offset.y).abs() > 1.0 {
            self.set_offset(viewer, Vector::new(offset.x, target));
        } else if delta.abs() > 1.0 {
            self.raise_viewport_changed(true);
        }
    }
    fn on_layout_updated(&self) {
        let Some(viewer) = self.scroll_viewer() else {
            return;
        };
        let bring = self.pending_bring.borrow().clone();
        if bring
            .as_ref()
            .is_some_and(|bring| !self.bring_target_is_current(bring))
        {
            self.cancel_scroll_animation();
        }
        if self
            .pending_bring
            .borrow()
            .as_ref()
            .is_some_and(|bring| !bring.change_view_called)
        {
            self.apply_pending_change_view(&viewer);
        } else if self.pending_bring.borrow().is_none() {
            // A change inside a bounded ScrollViewer need not invalidate the
            // host's own arrange. Compare the retained content-space anchor
            // after the native layout manager has arranged that subtree.
            let anchor = self.anchor.borrow().as_ref().and_then(WeakRef::upgrade);
            if let Some((anchor, previous)) = anchor.zip(self.anchor_bounds.get()) {
                self.track_element(&anchor, previous, &viewer);
            }
        }
        self.invalidate_anchor();
        self.ensure_anchor();
    }
    fn apply_pending_change_view(&self, viewer: &Ref<ScrollViewer>) {
        let Some(mut bring) = self.pending_bring.borrow().clone() else {
            return;
        };
        if !self.bring_target_is_current(&bring) {
            self.cancel_scroll_animation();
            return;
        }
        let Some(target) = bring.target.upgrade() else {
            self.pending_bring.borrow_mut().take();
            return;
        };
        let Some(content) = Self::content_root(viewer) else {
            return;
        };
        // Native layout can queue the host before a virtualized descendant.
        // Capture the destination only once the target and its content-space
        // ancestors have completed layout; LayoutUpdated retries this request.
        let content_visual = content.clone().upcast::<Visual>();
        let mut ancestor = Some(target.clone().upcast::<Visual>());
        while let Some(visual) = ancestor {
            if visual.cast::<Layoutable>().is_some_and(|layoutable| {
                !layoutable.is_measure_valid() || !layoutable.is_arrange_valid()
            }) {
                return;
            }
            if visual == content_visual {
                break;
            }
            ancestor = visual.visual_parent();
        }
        let Some(bounds) = Self::bounds_relative_to(&target, &content) else {
            return;
        };
        let viewport = viewer.viewport();
        let extent = viewer.extent();
        let clamp = |value: f64, maximum: f64| value.max(0.0).min(maximum.max(0.0));
        let destination = Vector::new(
            clamp(
                bounds.x + bring.offset.x - (viewport.width - bounds.width) * bring.alignment.x,
                extent.width - viewport.width,
            ),
            clamp(
                bounds.y + bring.offset.y - (viewport.height - bounds.height) * bring.alignment.y,
                extent.height - viewport.height,
            ),
        );
        bring.change_view_offset = destination;
        bring.change_view_called = true;
        *self.pending_bring.borrow_mut() = Some(bring.clone());
        // The target has completed its first arrange and the destination is
        // captured. Release its temporary layout anchor at the native frame
        // boundary; the operation's scoped pins retain target identity while
        // intermediate animation viewports realize a different set of items.
        self.notify_bring_finished(&bring);
        let clock = viewer.clock().or_else(|| {
            FerroLocator::current()
                .get_service::<dyn IGlobalClock>()
                .map(|clock| clock as Rc<dyn IClock>)
        });
        if bring.animate && viewer.offset() != destination && clock.is_some() {
            self.animate_offset(
                viewer,
                &bring,
                clock.expect("A rendering clock is available."),
            );
        } else {
            self.set_offset(viewer, destination);
            self.complete_bring(bring.id);
        }
    }
    fn animate_offset(
        &self,
        viewer: &Ref<ScrollViewer>,
        bring: &BringIntoViewState,
        clock: Rc<dyn IClock>,
    ) {
        let start = viewer.offset();
        let target = bring.change_view_offset;
        let id = bring.id;
        let proxy = RepeaterAnimationProxy::new();
        let cancellation = CancellationTokenSource::new();
        let token = cancellation.token();
        let weak = self.to_ref().downgrade();
        let weak_viewer = viewer.downgrade();
        proxy.set_changed(Some(Rc::new(move |progress| {
            if let (Some(this), Some(viewer)) = (weak.upgrade(), weak_viewer.upgrade()) {
                let bring = this
                    .pending_bring
                    .borrow()
                    .clone()
                    .filter(|bring| bring.id == id);
                let Some(bring) = bring else {
                    return;
                };
                if !this.bring_target_is_current(&bring) {
                    this.cancel_scroll_animation();
                    return;
                }
                if this
                    .animation
                    .borrow()
                    .as_ref()
                    .is_some_and(|animation| animation.id == id)
                {
                    this.set_offset(&viewer, start + (target - start) * progress.clamp(0.0, 1.0));
                }
            }
        })));
        *self.animation.borrow_mut() = Some(ScrollAnimation {
            id,
            _proxy: proxy.clone(),
            cancellation,
        });
        let animation = Animation::new();
        // ChangeView's duration is platform-owned in WinUI. The native host
        // supplies a 250 ms scrolling transition on the framework clock.
        animation.set_duration(TimeSpan::from_milliseconds(250.0));
        animation.set_easing(Easing::new(SplineEasing::with_points(0.25, 0.1, 0.25, 1.0)));
        animation.set_fill_mode(FillMode::Both);
        animation.children().add(KeyFrame::with_cue(
            Cue::new(0.0),
            [Setter::new(RepeaterAnimationProxy::progress_property(), 0.0) as _],
        ));
        animation.children().add(KeyFrame::with_cue(
            Cue::new(1.0),
            [Setter::new(RepeaterAnimationProxy::progress_property(), 1.0) as _],
        ));
        let task = animation.run_async_with_clock(&proxy, Some(clock), token);
        let weak = self.to_ref().downgrade();
        task.on_completed(move || {
            if let Some(this) = weak.upgrade() {
                this.complete_bring(id);
            }
        });
    }
    fn complete_bring(&self, id: u64) {
        let bring = self
            .pending_bring
            .borrow()
            .clone()
            .filter(|bring| bring.id == id);
        let Some(bring) = bring else {
            return;
        };
        if !self.bring_target_is_current(&bring) {
            self.cancel_scroll_animation();
            return;
        }
        // Native Animation invokes completion before applying its final fill.
        // Commit the destination while the operation still owns the offset;
        // disposing its private animation target must not strand the previous
        // sampled frame when rendering skipped the exact final timestamp.
        if let Some(viewer) = self.scroll_viewer() {
            self.set_offset(&viewer, bring.change_view_offset);
        }
        if !self
            .pending_bring
            .borrow()
            .as_ref()
            .is_some_and(|bring| bring.id == id)
        {
            return;
        }
        self.pending_bring.borrow_mut().take();
        let finished = self.animation.borrow_mut().take();
        drop(finished);
        bring.pins.release();
        self.invalidate_anchor();
        self.notify_bring_finished(&bring);
        self.raise_viewport_changed(true);
    }
    fn cancel_scroll_animation(&self) {
        // Remove the pending identity before cancellation invokes completion.
        let bring = self.pending_bring.borrow_mut().take();
        let animation = self.animation.borrow_mut().take();
        if let Some(bring) = &bring {
            bring.pins.release();
        }
        if let Some(animation) = animation {
            animation.cancellation.cancel();
        }
        if let Some(bring) = bring {
            self.notify_bring_finished(&bring);
        }
    }
    fn notify_bring_finished(&self, bring: &BringIntoViewState) {
        if !self.bring_target_is_current(bring) {
            return;
        }
        let Some(target) = bring.target.upgrade() else {
            return;
        };
        let target = target.upcast::<Visual>();
        let host = self.to_ref().upcast::<Visual>();
        let mut ancestor = target.visual_parent();
        while let Some(parent) = ancestor {
            if parent == host {
                break;
            }
            if let Some(repeater) = parent.cast::<super::ItemsRepeater>() {
                let child = repeater.immediate_child(&target);
                if child.is_some() && child == repeater.viewport_manager.made_anchor() {
                    // Host scrolling bypasses the routed request. Notify the
                    // matching repeater anchor after its target was arranged;
                    // subsequent animation frames use the captured offsets.
                    repeater
                        .viewport_manager
                        .on_bring_into_view(&repeater, &target);
                }
            }
            ancestor = parent.visual_parent();
        }
    }

    fn bring_target_is_current(&self, bring: &BringIntoViewState) -> bool {
        if !bring.pins.is_valid() {
            return false;
        }
        let host = self.to_ref().upcast::<Visual>();
        let mut ancestor = bring
            .target
            .upgrade()
            .map(|target| target.upcast::<Visual>());
        while let Some(visual) = ancestor {
            if visual == host {
                return true;
            }
            ancestor = visual.visual_parent();
        }
        false
    }
}

impl IScrollAnchorProvider for ItemsRepeaterScrollHost {
    fn current_anchor(&self) -> Option<Ref<Control>> {
        Self::current_anchor(self)
    }
    fn register_anchor_candidate(&self, element: &Ref<Control>) {
        Self::register_anchor_candidate(self, element);
    }
    fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        Self::unregister_anchor_candidate(self, element);
    }
}

impl Drop for ItemsRepeaterScrollHost {
    fn drop(&mut self) {
        self.pending_bring.get_mut().take();
        if let Some(animation) = self.animation.get_mut().take() {
            animation.cancellation.cancel();
        }
        if let Some(subscription) = self.viewer_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.layout_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
