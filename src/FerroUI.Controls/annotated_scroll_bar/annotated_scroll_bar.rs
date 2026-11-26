//! WinUI's `AnnotatedScrollBar` (`controls/dev/AnnotatedScrollBar/AnnotatedScrollBar.cpp`, `.h`
//! and `.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::{
    AnnotatedScrollBarDetailLabelRequestedEventArgs, AnnotatedScrollBarLabel, AnnotatedScrollBarScrollingEventArgs,
    AnnotatedScrollBarScrollingEventKind,
};
use crate::presenters::ContentPresenter;
use crate::primitives::{
    AddScrollVelocityRequestedHandler, IScrollController, ScrollByRequestedHandler, ScrollControllerRequests,
    ScrollToRequestedHandler, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::IDataTemplate;
use crate::{Border, Control, ControlImpl, Panel, RepeatButton, ScrollingAnimationMode, ScrollingScrollOptions};
use crate::{ScrollingSnapPointsMode, ToolTip};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{
    InputElement, InputElementImpl, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutingStrategies};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl};
use ferroui_base::media::{ITransform, TranslateTransform};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, Size, StyledElementImpl,
    StyledProperty, Thickness, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const VERTICAL_THUMB_PART_NAME: &str = "PART_VerticalThumb";
const VERTICAL_THUMB_GHOST_PART_NAME: &str = "PART_VerticalThumbGhost";
const VERTICAL_DECREMENT_REPEAT_BUTTON_PART_NAME: &str = "PART_VerticalDecrementRepeatButton";
const VERTICAL_INCREMENT_REPEAT_BUTTON_PART_NAME: &str = "PART_VerticalIncrementRepeatButton";
const VERTICAL_GRID_PART_NAME: &str = "PART_VerticalGrid";
const LABELS_GRID_PART_NAME: &str = "PART_LabelsGrid";
const TOOL_TIP_RAIL_PART_NAME: &str = "PART_ToolTipRail";
const TOOLTIP_CONTENT_PRESENTER_PART_NAME: &str = "PART_TooltipContentPresenter";

/// `s_defaultViewportToSmallChangeRatio`.
const DEFAULT_VIEWPORT_TO_SMALL_CHANGE_RATIO: f64 = 8.0;
/// `s_velocityNeededPerPixel`.
const VELOCITY_NEEDED_PER_PIXEL: f64 = 3.688880455092886;
/// `s_smallChangeInertiaDecayRate`.
const SMALL_CHANGE_INERTIA_DECAY_RATE: f32 = 0.975;
/// `s_toolTipTouchOffset`.
const TOOL_TIP_TOUCH_OFFSET: f64 = 22.0;

/// A vertical scroll bar with labels (WinUI's `AnnotatedScrollBar`): a rail that shows the
/// `Labels` at the offsets they mark, a thumb at the current offset, an increment and a
/// decrement button, and a detail label under the pointer, whose content the
/// [`detail_label_requested`](Self::detail_label_requested) handlers give. A press or a drag on
/// the rail scrolls to the offset under the pointer, the buttons scroll by `SmallChange` (an
/// eighth of the viewport when 0); [`scrolling`](Self::scrolling) handlers can cancel each request.
///
/// The bar drives a scroll presenter through its [`scroll_controller`](Self::scroll_controller):
/// `scroll_view.scroll_presenter().set_vertical_scroll_controller(Some(bar.scroll_controller()))`.
///
/// The template is the control theme of the WinUI theme (`Controls/AnnotatedScrollBar.xaml`, from
/// WinUI's `AnnotatedScrollBar.xaml`); the Fluent and Simple themes have none
/// (docs/porting/winui-controls.md, section 5.1). Differences: section 10.13 of that document.
#[repr(C)]
pub struct AnnotatedScrollBar {
    base: TemplatedControl,
    labels: RefCell<Option<FerroList<AnnotatedScrollBarLabel>>>,
    labels_changed_token: Cell<Option<u64>>,
    controller: RefCell<Option<Rc<AnnotatedScrollBarController>>>,
    requests: ScrollControllerRequests,
    parts: RefCell<Parts>,
    part_subscriptions: RefCell<Vec<Subscription>>,
    thumb_translation: RefCell<Option<Ref<TranslateTransform>>>,
    viewport_size: Cell<f64>,
    maximum: Cell<f64>,
    minimum: Cell<f64>,
    value: Cell<f64>,
    last_offset: Cell<f64>,
    scroll_offset_to_label_offset_factor: Cell<f64>,
    captured: Cell<bool>,
    tracked_pointer_id: Cell<Option<i32>>,
    is_pressed: Cell<bool>,
    is_pointer_over: Cell<bool>,
    last_vertical_grid_pointer_moved_offset: Cell<f64>,
    detail_label_template_applied: Cell<bool>,
    can_scroll: Cell<bool>,
    is_scrolling_with_mouse: Cell<bool>,
    is_scrollable: Cell<bool>,
    operations_count: Cell<i32>,
    last_correlation_ids: Cell<(i32, i32, i32)>,
    label_sizes: RefCell<Vec<f64>>,
    labels_debounce: RefCell<Option<Subscription>>,
    scrolling: HandlerList<dyn Fn(&AnnotatedScrollBar, &AnnotatedScrollBarScrollingEventArgs)>,
    detail_label_requested: HandlerList<dyn Fn(&AnnotatedScrollBar, &AnnotatedScrollBarDetailLabelRequestedEventArgs)>,
}

#[derive(Default)]
struct Parts {
    vertical_thumb: Option<Ref<Border>>,
    vertical_thumb_ghost: Option<Ref<Border>>,
    vertical_decrement_repeat_button: Option<Ref<RepeatButton>>,
    vertical_increment_repeat_button: Option<Ref<RepeatButton>>,
    vertical_grid: Option<Ref<Panel>>,
    labels_grid: Option<Ref<Panel>>,
    tool_tip_rail: Option<Ref<Control>>,
    tooltip_content_presenter: Option<Ref<ContentPresenter>>,
}

ferro_class!(AnnotatedScrollBar: TemplatedControl);
ferroui_base::ferro_class_info!(AnnotatedScrollBar {
    new: AnnotatedScrollBar::new,
    markup: {
        namespace: "FerroUI.Controls",
        events: [
            Scrolling(Option<BoxedValue>, AnnotatedScrollBarScrollingEventArgs) =>
                |this: &Ref<AnnotatedScrollBar>, handler: MarkupDelegate| {
                this.scrolling(move |sender: &AnnotatedScrollBar, args: &AnnotatedScrollBarScrollingEventArgs| {
                    handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(args.clone())]);
                })
            },
            DetailLabelRequested(Option<BoxedValue>, AnnotatedScrollBarDetailLabelRequestedEventArgs) =>
                |this: &Ref<AnnotatedScrollBar>, handler: MarkupDelegate| {
                this.detail_label_requested(
                    move |sender: &AnnotatedScrollBar, args: &AnnotatedScrollBarDetailLabelRequestedEventArgs| {
                        handler.invoke(&[into_markup_value(Some(sender.to_ref())), into_markup_value(args.clone())]);
                    },
                )
            },
        ],
        attributes: [
            TemplatePart("PART_VerticalThumb", type(Ref<Border>)),
            TemplatePart("PART_VerticalThumbGhost", type(Ref<Border>)),
            TemplatePart("PART_VerticalDecrementRepeatButton", type(Ref<RepeatButton>)),
            TemplatePart("PART_VerticalIncrementRepeatButton", type(Ref<RepeatButton>)),
            TemplatePart("PART_VerticalGrid", type(Ref<Panel>)),
            TemplatePart("PART_LabelsGrid", type(Ref<Panel>)),
            TemplatePart("PART_ToolTipRail", type(Ref<Control>)),
            TemplatePart("PART_TooltipContentPresenter", type(Ref<ContentPresenter>)),
        ],
    },
});
ferro_impl_classes!(AnnotatedScrollBar: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl ControlImpl for AnnotatedScrollBar {}

impl FerroObjectImpl for AnnotatedScrollBar {
    /// `OnPropertyChanged` and `OnIsEnabledChanged`.
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::label_template_property().as_property() {
            this.queue_layout_labels(50);
        } else if property == Self::detail_label_template_property().as_property() {
            this.detail_label_template_applied.set(false);
        } else if property == Visual::bounds_property().as_property() {
            // `OnSizeChanged`.
            this.update_positions_of_anchored_elements();
        } else if property == InputElement::is_effectively_enabled_property().as_property() {
            if !this.is_effectively_enabled() {
                this.is_pressed.set(false);
                this.is_pointer_over.set(false);
                this.captured.set(false);
                this.reset_tracked_pointer_id();
            }
            this.update_can_scroll();
        }
    }
}

impl TemplatedControlImpl for AnnotatedScrollBar {
    /// `OnApplyTemplate`, `SetUpInteractionElements` and `HookInputEvents`.
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        this.part_subscriptions.borrow_mut().clear();
        let scope = args.name_scope();
        let parts = Parts {
            vertical_thumb: scope.find_as::<Border>(VERTICAL_THUMB_PART_NAME),
            vertical_thumb_ghost: scope.find_as::<Border>(VERTICAL_THUMB_GHOST_PART_NAME),
            vertical_decrement_repeat_button: scope.find_as::<RepeatButton>(VERTICAL_DECREMENT_REPEAT_BUTTON_PART_NAME),
            vertical_increment_repeat_button: scope.find_as::<RepeatButton>(VERTICAL_INCREMENT_REPEAT_BUTTON_PART_NAME),
            vertical_grid: scope.find_as::<Panel>(VERTICAL_GRID_PART_NAME),
            labels_grid: scope.find_as::<Panel>(LABELS_GRID_PART_NAME),
            tool_tip_rail: scope.find_as::<Control>(TOOL_TIP_RAIL_PART_NAME),
            tooltip_content_presenter: scope.find_as::<ContentPresenter>(TOOLTIP_CONTENT_PRESENTER_PART_NAME),
        };
        let mut parts = parts;
        if parts.tooltip_content_presenter.is_none() {
            // The tip of the rail is not in the visual tree: its presenter is the content of the tip.
            parts.tooltip_content_presenter = parts.tool_tip_rail.as_ref().and_then(tip_content_presenter);
        }
        // The thumb moves with a render transform: the presenter's offset changes no layout.
        let translation = parts.vertical_thumb.as_ref().map(|thumb| {
            let translation = TranslateTransform::new();
            let transform: Rc<dyn ITransform> = translation.clone().into();
            thumb.set_render_transform(Some(transform));
            translation
        });
        *this.thumb_translation.borrow_mut() = translation;
        this.label_sizes.borrow_mut().clear();
        *this.parts.borrow_mut() = parts;
        this.hook_input_events();
        this.update_positions_of_anchored_elements();
    }
}

ferroui_base::ferro_properties! { impl AnnotatedScrollBar {
    ferro_property!(
        /// Defines the `Labels` property: the labels of the rail. The list is created when it is
        /// first read.
        pub fn labels_property() -> DirectProperty<AnnotatedScrollBar, Option<FerroList<AnnotatedScrollBarLabel>>> {
            FerroProperty::register_direct::<AnnotatedScrollBar, _>(
                "Labels",
                |o| Some(o.labels()),
                Some(|o, v| o.set_labels(v)),
                None,
            )
        }
    );
    ferro_property!(
        /// Defines the `LabelTemplate` property: the template of each label (WinUI's
        /// `IElementFactory`; a data template or a template selector here).
        pub fn label_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<AnnotatedScrollBar, _>("LabelTemplate", None)
        }
    );
    ferro_property!(
        /// Defines the `DetailLabelTemplate` property: the template of the detail label.
        pub fn detail_label_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<AnnotatedScrollBar, _>("DetailLabelTemplate", None)
        }
    );
    ferro_property!(
        /// Defines the `SmallChange` property: the scroll of a button (default 0: an eighth of the
        /// viewport, set on the first click).
        pub fn small_change_property() -> StyledProperty<f64> {
            FerroProperty::register::<AnnotatedScrollBar, _>("SmallChange", 0.0)
        }
    );
} }

impl AnnotatedScrollBar {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            labels: RefCell::new(None),
            labels_changed_token: Cell::new(None),
            controller: RefCell::new(None),
            requests: ScrollControllerRequests::new(),
            parts: RefCell::new(Parts::default()),
            part_subscriptions: RefCell::new(Vec::new()),
            thumb_translation: RefCell::new(None),
            viewport_size: Cell::new(0.0),
            maximum: Cell::new(0.0),
            minimum: Cell::new(0.0),
            value: Cell::new(0.0),
            last_offset: Cell::new(0.0),
            scroll_offset_to_label_offset_factor: Cell::new(0.0),
            captured: Cell::new(false),
            tracked_pointer_id: Cell::new(None),
            is_pressed: Cell::new(false),
            is_pointer_over: Cell::new(false),
            last_vertical_grid_pointer_moved_offset: Cell::new(0.0),
            detail_label_template_applied: Cell::new(false),
            can_scroll: Cell::new(false),
            is_scrolling_with_mouse: Cell::new(false),
            is_scrollable: Cell::new(false),
            operations_count: Cell::new(0),
            last_correlation_ids: Cell::new((-1, -1, -1)),
            label_sizes: RefCell::new(Vec::new()),
            labels_debounce: RefCell::new(None),
            scrolling: HandlerList::new(),
            detail_label_requested: HandlerList::new(),
        }
    }

    /// Creates a bar without labels.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The labels of the rail (`Labels`); the list is created when it is first read.
    pub fn labels(&self) -> FerroList<AnnotatedScrollBarLabel> {
        if let Some(labels) = self.labels.borrow().clone() {
            return labels;
        }
        let labels = FerroList::new();
        *self.labels.borrow_mut() = Some(labels.clone());
        self.subscribe_to_labels(&labels);
        labels
    }

    /// Replaces the labels (`Labels`): the rail shows the new list.
    pub fn set_labels(&self, value: Option<FerroList<AnnotatedScrollBarLabel>>) {
        let old = self.labels.borrow().clone();
        if old == value {
            return;
        }
        if let (Some(old), Some(token)) = (&old, self.labels_changed_token.take()) {
            old.remove_collection_changed(token);
        }
        self.set_and_raise(Self::labels_property(), &self.labels, value.clone());
        if let Some(value) = &value {
            self.subscribe_to_labels(value);
        }
        self.on_labels_collection_changed();
    }

    fn subscribe_to_labels(&self, labels: &FerroList<AnnotatedScrollBarLabel>) {
        let weak = self.to_ref().downgrade();
        let token = labels.add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_labels_collection_changed();
            }
        }));
        self.labels_changed_token.set(Some(token));
    }

    /// The template of each label (`LabelTemplate`).
    pub fn label_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::label_template_property())
    }

    /// Sets the template of each label.
    pub fn set_label_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::label_template_property(), value);
    }

    /// The template of the detail label (`DetailLabelTemplate`).
    pub fn detail_label_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::detail_label_template_property())
    }

    /// Sets the template of the detail label.
    pub fn set_detail_label_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::detail_label_template_property(), value);
    }

    /// The scroll of a button (`SmallChange`).
    pub fn small_change(&self) -> f64 {
        self.get_value(Self::small_change_property())
    }

    /// Sets the scroll of a button.
    pub fn set_small_change(&self, value: f64) {
        self.set_value(Self::small_change_property(), value);
    }

    /// The scroll controller of the bar (`ScrollController`), for the vertical axis of a scroll
    /// presenter. The controller refers to the bar weakly; the same controller is returned each
    /// time.
    pub fn scroll_controller(&self) -> Rc<dyn IScrollController> {
        let controller = self
            .controller
            .borrow_mut()
            .get_or_insert_with(|| {
                Rc::new(AnnotatedScrollBarController {
                    bar: self.to_ref().downgrade(),
                })
            })
            .clone();
        controller
    }

    /// Adds a handler of `Scrolling`, raised before the bar requests a scroll; a handler cancels
    /// the request with [`AnnotatedScrollBarScrollingEventArgs::set_cancel`]. Dropping the
    /// returned guard removes the handler.
    pub fn scrolling(
        &self,
        handler: impl Fn(&AnnotatedScrollBar, &AnnotatedScrollBarScrollingEventArgs) + 'static,
    ) -> Subscription {
        let token = self.scrolling.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.scrolling.remove(token);
            }
        }))
    }

    /// Adds a handler of `DetailLabelRequested`, raised when the pointer moves over the rail: the
    /// handler sets the content of the detail label for the offset under the pointer. Without a
    /// handler the bar shows no detail label. Dropping the returned guard removes the handler.
    pub fn detail_label_requested(
        &self,
        handler: impl Fn(&AnnotatedScrollBar, &AnnotatedScrollBarDetailLabelRequestedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.detail_label_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.detail_label_requested.remove(token);
                if this.detail_label_requested.is_empty() {
                    this.set_tool_tip_is_open(false);
                }
            }
        }))
    }

    /// The factor from a scroll offset to an offset on the rail
    /// (`m_scrollOffsetToLabelOffsetFactor`).
    pub(crate) fn scroll_offset_to_label_offset_factor(&self) -> f64 {
        self.scroll_offset_to_label_offset_factor.get()
    }

    fn on_labels_collection_changed(&self) {
        if let Some(labels_grid) = self.parts.borrow().labels_grid.clone() {
            labels_grid.children().clear();
        }
        self.label_sizes.borrow_mut().clear();
        self.queue_layout_labels(50);
    }

    fn hook_input_events(&self) {
        let parts = self.parts.borrow();
        let weak = self.to_ref().downgrade();
        let mut subscriptions = Vec::new();
        if let Some(grid) = &parts.vertical_grid {
            subscriptions.push(grid.add_disposable_handler_with(
                InputElement::pointer_entered_event(),
                with_bar(&weak, |this, e: &PointerEventArgs| {
                    this.on_vertical_grid_pointer_entered(e)
                }),
                RoutingStrategies::DIRECT,
                false,
            ));
            subscriptions.push(grid.add_disposable_handler_with(
                InputElement::pointer_moved_event(),
                with_bar(&weak, |this, e: &PointerEventArgs| {
                    this.on_vertical_grid_pointer_moved(e)
                }),
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                false,
            ));
            subscriptions.push(grid.add_disposable_handler_with(
                InputElement::pointer_pressed_event(),
                with_bar(&weak, |this, e: &PointerPressedEventArgs| {
                    this.on_vertical_grid_pointer_pressed(e)
                }),
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                false,
            ));
            subscriptions.push(grid.add_disposable_handler_with(
                InputElement::pointer_exited_event(),
                with_bar(&weak, |this, e: &PointerEventArgs| {
                    this.on_vertical_grid_pointer_exited(e)
                }),
                RoutingStrategies::DIRECT,
                true,
            ));
            subscriptions.push(self.add_disposable_handler_with(
                InputElement::pointer_released_event(),
                with_bar(&weak, |this, e: &PointerReleasedEventArgs| {
                    this.on_vertical_grid_pointer_released(e)
                }),
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                true,
            ));
            subscriptions.push(self.add_disposable_handler_with(
                InputElement::pointer_capture_lost_event(),
                with_bar(&weak, |this, e: &PointerCaptureLostEventArgs| {
                    this.on_vertical_grid_pointer_capture_lost(e)
                }),
                RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                true,
            ));
        }
        let clicked = |button: &Option<Ref<RepeatButton>>, kind: AnnotatedScrollBarScrollingEventKind| {
            button.as_ref().map(|button| {
                button.add_disposable_handler_with(
                    crate::Button::click_event(),
                    with_bar(&weak, move |this, _: &ferroui_base::interactivity::RoutedEventArgs| {
                        this.scroll_to(0.0, kind)
                    }),
                    RoutingStrategies::BUBBLE,
                    false,
                )
            })
        };
        subscriptions.extend(clicked(
            &parts.vertical_increment_repeat_button,
            AnnotatedScrollBarScrollingEventKind::IncrementButton,
        ));
        subscriptions.extend(clicked(
            &parts.vertical_decrement_repeat_button,
            AnnotatedScrollBarScrollingEventKind::DecrementButton,
        ));
        if let Some(labels_grid) = &parts.labels_grid {
            // `OnLabelsGridSizeChanged`: the labels follow the height of the rail.
            subscriptions.push(labels_grid.add_disposable_handler_with(
                Control::size_changed_event(),
                with_bar(&weak, |this, _: &crate::SizeChangedEventArgs| {
                    this.update_positions_of_anchored_elements()
                }),
                RoutingStrategies::DIRECT,
                false,
            ));
        }
        *self.part_subscriptions.borrow_mut() = subscriptions;
    }

    /// `UpdatePositionsOfAnchoredElements`: the factor, the thumb and, after 500 ms, the labels.
    fn update_positions_of_anchored_elements(&self) {
        self.update_scroll_offset_to_label_offset_factor();
        self.update_thumb_position();
        self.queue_layout_labels(500);
    }

    /// The thumb at the offset of the presenter (WinUI moves it through the panning info of the
    /// controller, `PanningElementOffsetMultiplier`).
    fn update_thumb_position(&self) {
        if let Some(translation) = self.thumb_translation.borrow().as_ref() {
            let offset = self.last_offset.get() - self.minimum.get();
            translation.set_y((offset * self.scroll_offset_to_label_offset_factor.get()).max(0.0));
        }
    }

    /// `QueueLayoutLabels`: one layout of the labels after `millisecond_wait`.
    fn queue_layout_labels(&self, millisecond_wait: u64) {
        if self.labels_debounce.borrow().is_some() {
            return;
        }
        let weak = self.to_ref().downgrade();
        let timer = DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.labels_debounce.borrow_mut().take();
                    this.layout_labels();
                }
            },
            Duration::from_millis(millisecond_wait),
            DispatcherPriority::BACKGROUND,
        );
        *self.labels_debounce.borrow_mut() = Some(timer);
    }

    /// `LayoutLabels`.
    pub(crate) fn layout_labels(&self) {
        let Some(labels_grid) = self.parts.borrow().labels_grid.clone() else {
            return;
        };
        if labels_grid.children().count() == 0 {
            self.create_label_containers(&labels_grid);
        } else {
            self.update_label_containers_offsets(&labels_grid);
        }
        self.collapse_colliding_and_out_of_bounds_labels(&labels_grid);
    }

    /// `CreateLabelContainers`: a content presenter per label, right aligned, with the label
    /// template, measured for the collisions.
    fn create_label_containers(&self, labels_grid: &Ref<Panel>) {
        let template = self.label_template();
        let mut sizes = Vec::new();
        for label in self.labels().snapshot().iter() {
            let presenter = ContentPresenter::new();
            presenter.set_horizontal_alignment(HorizontalAlignment::Right);
            presenter.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
            let content: BoxedValue = Rc::new(label.clone());
            presenter.set_content(Some(content));
            if let Some(template) = &template {
                presenter.set_content_template(Some(template.clone()));
            }
            labels_grid.children().add(presenter.clone());
            presenter.measure(Size::INFINITY);
            sizes.push(presenter.desired_size().height);
            presenter.set_margin(Thickness::new(0.0, self.label_vertical_offset(label), 0.0, 0.0));
        }
        *self.label_sizes.borrow_mut() = sizes;
    }

    /// `UpdateLabelContainersOffsets`.
    fn update_label_containers_offsets(&self, labels_grid: &Ref<Panel>) {
        for child in labels_grid.children().snapshot().iter() {
            let Some(presenter) = child.cast::<ContentPresenter>() else {
                continue;
            };
            let label = presenter
                .content()
                .and_then(|content| content.downcast_ref::<AnnotatedScrollBarLabel>().cloned());
            if let Some(label) = label {
                presenter.set_margin(Thickness::new(0.0, self.label_vertical_offset(&label), 0.0, 0.0));
            }
        }
    }

    /// `CollapseCollidingAndOutOfBoundsLabels`: from the last label up, a label out of the rail
    /// or overlapping the label below it collapses; the first label wins over the label below it
    /// unless that one is the last.
    fn collapse_colliding_and_out_of_bounds_labels(&self, labels_grid: &Ref<Panel>) {
        let labels_grid_height = labels_grid.bounds().height;
        if self.labels().count() == 0 || labels_grid_height == 0.0 {
            return;
        }
        let children = labels_grid.children().snapshot();
        let sizes = self.label_sizes.borrow();
        let mut previous_label_top_position = -1.0;
        let mut previous_label_index = children.len();
        for current_label_index in (0..children.len()).rev() {
            let current_label = &children[current_label_index];
            let current_label_top_position = current_label.margin().top;
            let current_label_bottom_position =
                current_label_top_position + sizes.get(current_label_index).copied().unwrap_or(0.0);
            let is_label_out_of_bounds =
                current_label_top_position < 0.0 || current_label_bottom_position > labels_grid_height;
            let is_label_colliding =
                previous_label_top_position >= 0.0 && previous_label_top_position <= current_label_bottom_position;
            if is_label_out_of_bounds {
                current_label.set_is_visible(false);
            } else if is_label_colliding {
                let mut index_to_collapse = current_label_index;
                if current_label_index == 0 && previous_label_index < children.len() - 1 {
                    index_to_collapse = previous_label_index;
                }
                children[index_to_collapse].set_is_visible(false);
            } else {
                current_label.set_is_visible(true);
                previous_label_top_position = current_label_top_position;
                previous_label_index = current_label_index;
            }
        }
    }

    /// `GetLabelVerticalOffset`.
    fn label_vertical_offset(&self, label: &AnnotatedScrollBarLabel) -> f64 {
        label.scroll_offset() * self.scroll_offset_to_label_offset_factor.get()
    }

    /// `UpdateScrollOffsetToLabelOffsetFactor`: the rail without the thumb over the scrollable
    /// height.
    fn update_scroll_offset_to_label_offset_factor(&self) {
        let parts = self.parts.borrow();
        let mut factor = 1.0;
        if let (Some(labels_grid), Some(thumb)) = (&parts.labels_grid, &parts.vertical_thumb) {
            let scrollable_height = self.maximum.get() - self.minimum.get();
            let scrollable_height = if scrollable_height == 0.0 {
                1.0
            } else {
                scrollable_height
            };
            factor = (labels_grid.bounds().height - thumb.bounds().height) / scrollable_height;
        }
        self.scroll_offset_to_label_offset_factor.set(factor);
    }

    /// `EnsureSmallChangeValue`.
    fn ensure_small_change_value(&self) {
        if self.small_change() == 0.0 {
            self.set_small_change(self.viewport_size.get() / DEFAULT_VIEWPORT_TO_SMALL_CHANGE_RATIO);
        }
    }

    fn set_thumb_ghost_is_visible(&self, is_visible: bool) {
        if let Some(ghost) = &self.parts.borrow().vertical_thumb_ghost {
            ghost.set_is_visible(is_visible);
        }
    }

    /// `ShowHoverVisuals`.
    fn show_hover_visuals(&self, offset_from_top_tool_tip: f64, offset_from_top_thumb_ghost: f64) {
        self.show_tool_tip_at_offset(offset_from_top_tool_tip);
        self.show_thumb_ghost_at_offset(offset_from_top_thumb_ghost);
    }

    /// `HideHoverVisuals`.
    fn hide_hover_visuals(&self) {
        self.set_tool_tip_is_open(false);
        self.set_thumb_ghost_is_visible(false);
    }

    /// `SetToolTipIsOpen`: the detail label opens only with a `DetailLabelRequested` handler.
    fn set_tool_tip_is_open(&self, is_open: bool) {
        if is_open && self.detail_label_requested.is_empty() {
            return;
        }
        if let Some(rail) = &self.parts.borrow().tool_tip_rail {
            if ToolTip::get_is_open(rail) != is_open {
                ToolTip::set_is_open(rail, is_open);
            }
        }
    }

    /// `ShowToolTipAtOffset`: the content of the handlers in the detail label, above the point of
    /// the rail at `offset_from_top`, its right edge 2 px past the rail.
    fn show_tool_tip_at_offset(&self, offset_from_top: f64) {
        let Some(data) = self.raise_detail_label_requested(offset_from_top) else {
            return;
        };
        let (rail, presenter) = {
            let parts = self.parts.borrow();
            (parts.tool_tip_rail.clone(), parts.tooltip_content_presenter.clone())
        };
        let (Some(rail), Some(presenter)) = (rail, presenter) else {
            return;
        };
        presenter.set_content(Some(data));
        if !self.detail_label_template_applied.get() || presenter.content_template().is_none() {
            if let Some(template) = self.detail_label_template() {
                presenter.set_content_template(Some(template));
                self.detail_label_template_applied.set(true);
            }
        }
        presenter.measure(Size::INFINITY);
        let width = presenter.desired_size().width;
        // Placement Top centres the tip on the rail; WinUI's placement rectangle starts at
        // -width / 2 + 2 with no width, so the tip ends 2 px right of the rail.
        ToolTip::set_horizontal_offset(&rail, -width / 2.0 + 2.0);
        ToolTip::set_vertical_offset(&rail, offset_from_top);
        self.set_tool_tip_is_open(true);
    }

    /// `ShowThumbGhostAtOffset`.
    fn show_thumb_ghost_at_offset(&self, offset_from_top: f64) {
        if let Some(ghost) = &self.parts.borrow().vertical_thumb_ghost {
            let margin = ghost.margin();
            ghost.set_margin(Thickness::new(
                margin.left,
                offset_from_top,
                margin.right,
                margin.bottom,
            ));
        }
        self.set_thumb_ghost_is_visible(true);
    }

    /// `UpdateCanScroll`.
    fn update_can_scroll(&self) {
        self.can_scroll
            .set(self.is_scrollable.get() && self.is_effectively_enabled());
    }

    fn reset_tracked_pointer_id(&self) {
        self.tracked_pointer_id.set(None);
    }

    /// `IgnorePointerId`: the bar follows one pointer at a time.
    fn ignore_pointer_id(&self, args: &PointerEventArgs) -> bool {
        let pointer_id = args.pointer().id();
        match self.tracked_pointer_id.get() {
            None => {
                self.tracked_pointer_id.set(Some(pointer_id));
                false
            }
            Some(tracked) => tracked != pointer_id,
        }
    }

    fn vertical_grid(&self) -> Option<Ref<Panel>> {
        self.parts.borrow().vertical_grid.clone()
    }

    fn position_in_vertical_grid(&self, args: &PointerEventArgs) -> Point {
        let grid: Option<Ref<Visual>> = self.vertical_grid().map(|grid| grid.upcast());
        args.get_position(grid.as_deref())
    }

    fn on_vertical_grid_pointer_entered(&self, args: &PointerEventArgs) {
        if self.ignore_pointer_id(args) {
            return;
        }
        self.is_pointer_over.set(true);
        let offset_from_top = self.position_in_vertical_grid(args).y;
        let mut offset_from_top_tool_tip = offset_from_top;
        if args.pointer().type_() == PointerType::Touch {
            offset_from_top_tool_tip -= TOOL_TIP_TOUCH_OFFSET;
        }
        self.show_hover_visuals(offset_from_top_tool_tip, offset_from_top);
    }

    fn on_vertical_grid_pointer_moved(&self, args: &PointerEventArgs) {
        if self.ignore_pointer_id(args) {
            return;
        }
        self.is_pointer_over.set(true);
        let Some(grid) = self.vertical_grid() else { return };
        let offset_from_top = self
            .position_in_vertical_grid(args)
            .y
            .max(0.0)
            .min(grid.bounds().height);
        let is_open = self
            .parts
            .borrow()
            .tool_tip_rail
            .as_ref()
            .is_some_and(|rail| ToolTip::get_is_open(rail));
        if is_open && offset_from_top == self.last_vertical_grid_pointer_moved_offset.get() {
            return;
        }
        self.last_vertical_grid_pointer_moved_offset.set(offset_from_top);
        let mut offset_from_top_tool_tip = offset_from_top;
        if args.pointer().type_() == PointerType::Touch {
            offset_from_top_tool_tip -= TOOL_TIP_TOUCH_OFFSET;
        }
        self.show_hover_visuals(offset_from_top_tool_tip, offset_from_top);
        if self.is_pressed.get() || self.captured.get() {
            let new_scroll_offset = offset_from_top / self.scroll_offset_to_label_offset_factor.get();
            self.scroll_to(new_scroll_offset, AnnotatedScrollBarScrollingEventKind::Drag);
        }
    }

    fn on_vertical_grid_pointer_exited(&self, args: &PointerEventArgs) {
        if self.ignore_pointer_id(args) {
            return;
        }
        let is_touch = args.pointer().type_() == PointerType::Touch;
        if is_touch && !self.captured.get() {
            self.reset_tracked_pointer_id();
        }
        if self.is_out_of_vertical_grid_bounds(self.position_in_vertical_grid(args)) && !is_touch {
            self.is_pressed.set(false);
            self.is_pointer_over.set(false);
            if !self.captured.get() {
                self.reset_tracked_pointer_id();
            }
            self.hide_hover_visuals();
        }
    }

    fn on_vertical_grid_pointer_pressed(&self, args: &PointerPressedEventArgs) {
        if self.ignore_pointer_id(args) {
            return;
        }
        self.is_pressed.set(true);
        let new_scroll_offset =
            self.position_in_vertical_grid(args).y / self.scroll_offset_to_label_offset_factor.get();
        self.scroll_to(new_scroll_offset, AnnotatedScrollBarScrollingEventKind::Click);
        if let Some(grid) = self.vertical_grid() {
            let element: Ref<InputElement> = grid.upcast();
            args.pointer().capture(Some(&element));
            self.captured
                .set(args.pointer().captured().is_some_and(|captured| captured == element));
        }
        if args.pointer().type_() == PointerType::Mouse && !self.is_scrolling_with_mouse.get() {
            self.is_scrolling_with_mouse.set(true);
        }
        args.set_handled(true);
    }

    fn on_vertical_grid_pointer_released(&self, args: &PointerReleasedEventArgs) {
        if self.ignore_pointer_id(args) {
            return;
        }
        self.is_pressed.set(false);
        if self.captured.get() {
            self.captured.set(false);
            args.pointer().capture(None);
            match args.pointer().type_() {
                PointerType::Touch => {
                    self.is_pointer_over.set(false);
                    self.hide_hover_visuals();
                    self.reset_tracked_pointer_id();
                }
                PointerType::Mouse => self.is_scrolling_with_mouse.set(false),
                _ => {}
            }
        }
    }

    /// `ProcessVerticalGridPointerCanceled` for a lost capture.
    fn on_vertical_grid_pointer_capture_lost(&self, args: &PointerCaptureLostEventArgs) {
        if !self.captured.get() {
            return;
        }
        self.is_pressed.set(false);
        if !self.is_pointer_over.get() {
            self.hide_hover_visuals();
        }
        if args.pointer().type_() == PointerType::Mouse {
            self.is_scrolling_with_mouse.set(false);
        }
        self.captured.set(false);
        self.reset_tracked_pointer_id();
    }

    /// `IsOutOfVerticalGridBounds`, with WinUI's tolerance of 1 px.
    fn is_out_of_vertical_grid_bounds(&self, point: Point) -> bool {
        let Some(grid) = self.vertical_grid() else { return true };
        const TOLERANCE: f64 = 1.0;
        let bounds = grid.bounds();
        point.x < TOLERANCE
            || point.x > bounds.width - TOLERANCE
            || point.y < TOLERANCE
            || point.y > bounds.height - TOLERANCE
    }

    /// `ScrollTo`: the offset of a press or a drag, or a small change of a button (the increment
    /// button scrolls up); `Scrolling` can cancel it. A press or a drag requests a scroll to the
    /// offset; a button adds a velocity when animations are on and scrolls by the change otherwise.
    pub(crate) fn scroll_to(&self, offset: f64, kind: AnnotatedScrollBarScrollingEventKind) {
        let mut scroll_offset = offset;
        let mut change_amount = 0.0;
        match kind {
            AnnotatedScrollBarScrollingEventKind::IncrementButton => {
                self.ensure_small_change_value();
                change_amount = -self.small_change();
                scroll_offset = (self.value.get() + change_amount).max(self.minimum.get());
            }
            AnnotatedScrollBarScrollingEventKind::DecrementButton => {
                self.ensure_small_change_value();
                change_amount = self.small_change();
                scroll_offset = (self.value.get() + change_amount).min(self.maximum.get());
            }
            _ => {}
        }
        if self.raise_scrolling(scroll_offset, kind) {
            return;
        }
        match kind {
            AnnotatedScrollBarScrollingEventKind::Click | AnnotatedScrollBarScrollingEventKind::Drag => {
                self.raise_scroll_to_requested(scroll_offset);
            }
            AnnotatedScrollBarScrollingEventKind::IncrementButton
            | AnnotatedScrollBarScrollingEventKind::DecrementButton => {
                let animations_enabled = self
                    .get_platform_settings()
                    .is_none_or(|settings| settings.animations_enabled());
                if animations_enabled {
                    self.raise_add_scroll_velocity_requested(change_amount);
                } else {
                    self.raise_scroll_by_requested(change_amount);
                }
            }
        }
    }

    /// `RaiseScrolling`: whether a handler cancelled the scroll.
    fn raise_scrolling(&self, offset: f64, kind: AnnotatedScrollBarScrollingEventKind) -> bool {
        if self.scrolling.is_empty() {
            return false;
        }
        let args = AnnotatedScrollBarScrollingEventArgs::new(offset, kind);
        for (_, handler) in self.scrolling.snapshot().iter() {
            handler(self, &args);
        }
        args.cancel()
    }

    /// `RaiseDetailLabelRequested`: the content of the handlers for the offset under the pointer.
    pub(crate) fn raise_detail_label_requested(&self, offset_from_top: f64) -> Option<BoxedValue> {
        if self.detail_label_requested.is_empty() {
            return None;
        }
        let value = (offset_from_top / self.scroll_offset_to_label_offset_factor.get())
            .min(self.maximum.get() + self.viewport_size.get());
        let args = AnnotatedScrollBarDetailLabelRequestedEventArgs::new(value);
        for (_, handler) in self.detail_label_requested.snapshot().iter() {
            handler(self, &args);
        }
        args.content()
    }

    /// Counts a request the presenter accepted, once per correlation id.
    fn count_operation(&self, correlation_id: i32, slot: fn(&mut (i32, i32, i32)) -> &mut i32) {
        let mut ids = self.last_correlation_ids.get();
        let last = slot(&mut ids);
        if correlation_id != -1 && correlation_id != *last {
            *last = correlation_id;
            self.last_correlation_ids.set(ids);
            self.operations_count.set(self.operations_count.get() + 1);
        }
    }

    /// `RaiseScrollToRequested`: no animation, no snap points.
    fn raise_scroll_to_requested(&self, offset: f64) {
        let options = ScrollingScrollOptions::with_snap_points_mode(
            ScrollingAnimationMode::Disabled,
            ScrollingSnapPointsMode::Ignore,
        );
        let id = self.requests.raise_scroll_to(offset, options);
        self.count_operation(id, |ids| &mut ids.2);
    }

    /// `RaiseScrollByRequested`: no animation, no snap points.
    fn raise_scroll_by_requested(&self, offset_delta: f64) {
        let options = ScrollingScrollOptions::with_snap_points_mode(
            ScrollingAnimationMode::Disabled,
            ScrollingSnapPointsMode::Ignore,
        );
        let id = self.requests.raise_scroll_by(offset_delta, options);
        self.count_operation(id, |ids| &mut ids.1);
    }

    /// `RaiseAddScrollVelocityRequested`: the velocity that covers the change with the decay rate
    /// of a small change.
    fn raise_add_scroll_velocity_requested(&self, offset_delta: f64) {
        let offset_velocity = (offset_delta * VELOCITY_NEEDED_PER_PIXEL) as f32;
        let id = self
            .requests
            .raise_add_scroll_velocity(offset_velocity, Some(SMALL_CHANGE_INERTIA_DECAY_RATE));
        self.count_operation(id, |ids| &mut ids.0);
    }

    /// `SetIsScrollable`.
    fn set_is_scrollable(&self, is_scrollable: bool) {
        self.is_scrollable.set(is_scrollable);
        self.update_can_scroll();
    }

    /// `SetValues`: the range and the viewport of the presenter; the value follows the offset when
    /// no request of the bar is under way.
    fn set_values(&self, min_offset: f64, max_offset: f64, offset: f64, viewport_length: f64) {
        let max_offset = max_offset.max(min_offset);
        let viewport_length = viewport_length.max(0.0);
        let offset = offset.clamp(min_offset, max_offset);
        self.last_offset.set(offset);
        let range_changed = self.minimum.get() != min_offset
            || self.maximum.get() != max_offset
            || self.viewport_size.get() != viewport_length;
        self.minimum.set(min_offset);
        self.maximum.set(max_offset);
        self.viewport_size.set(viewport_length);
        if self.operations_count.get() == 0 {
            self.value.set(offset);
        }
        if range_changed {
            self.update_positions_of_anchored_elements();
        } else {
            self.update_thumb_position();
        }
    }

    /// `NotifyRequestedScrollCompleted`.
    fn notify_requested_scroll_completed(&self) {
        let count = (self.operations_count.get() - 1).max(0);
        self.operations_count.set(count);
        if count == 0 && self.value.get() != self.last_offset.get() {
            self.value.set(self.last_offset.get());
        }
    }
}

/// The content presenter of the tooltip of `rail` (`ToolTip.Tip`): the content of a `ToolTip` tip or
/// the tip itself.
fn tip_content_presenter(rail: &Ref<Control>) -> Option<Ref<ContentPresenter>> {
    let tip = ToolTip::get_tip(rail)?;
    let control = Control::from_boxed(&tip)?;
    if let Some(tool_tip) = control.cast::<ToolTip>() {
        return tool_tip.content().as_ref().and_then(Control::from_boxed)?.cast::<ContentPresenter>();
    }
    control.cast::<ContentPresenter>()
}

fn with_bar<A: ?Sized>(
    weak: &WeakRef<AnnotatedScrollBar>,
    f: impl Fn(&AnnotatedScrollBar, &A) + 'static,
) -> impl Fn(&ferroui_base::interactivity::Interactive, &A) + 'static {
    let weak = weak.clone();
    move |_, args| {
        if let Some(this) = weak.upgrade() {
            f(&this, args);
        }
    }
}

/// The [`IScrollController`] of an [`AnnotatedScrollBar`] (WinUI's bar implements the interface
/// itself; here a controller refers to the bar weakly, so a presenter does not keep it alive).
struct AnnotatedScrollBarController {
    bar: WeakRef<AnnotatedScrollBar>,
}

impl IScrollController for AnnotatedScrollBarController {
    fn can_scroll(&self) -> bool {
        self.bar.upgrade().is_some_and(|bar| bar.can_scroll.get())
    }

    fn is_scrolling_with_mouse(&self) -> bool {
        self.bar.upgrade().is_some_and(|bar| bar.is_scrolling_with_mouse.get())
    }

    fn set_is_scrollable(&self, is_scrollable: bool) {
        if let Some(bar) = self.bar.upgrade() {
            bar.set_is_scrollable(is_scrollable);
        }
    }

    fn set_values(&self, min_offset: f64, max_offset: f64, offset: f64, viewport_length: f64) {
        if let Some(bar) = self.bar.upgrade() {
            bar.set_values(min_offset, max_offset, offset, viewport_length);
        }
    }

    fn notify_requested_scroll_completed(&self, _correlation_id: i32) {
        if let Some(bar) = self.bar.upgrade() {
            bar.notify_requested_scroll_completed();
        }
    }

    fn scroll_to_requested(&self, handler: Rc<ScrollToRequestedHandler>) -> Subscription {
        match self.bar.upgrade() {
            Some(bar) => bar.requests.add_scroll_to(handler),
            None => Subscription::new(Disposable::empty()),
        }
    }

    fn scroll_by_requested(&self, handler: Rc<ScrollByRequestedHandler>) -> Subscription {
        match self.bar.upgrade() {
            Some(bar) => bar.requests.add_scroll_by(handler),
            None => Subscription::new(Disposable::empty()),
        }
    }

    fn add_scroll_velocity_requested(&self, handler: Rc<AddScrollVelocityRequestedHandler>) -> Subscription {
        match self.bar.upgrade() {
            Some(bar) => bar.requests.add_add_scroll_velocity(handler),
            None => Subscription::new(Disposable::empty()),
        }
    }
}
