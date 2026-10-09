use crate::metadata::TemplatePartAttribute;
use crate::presenters::ContentPresenter;
use crate::primitives::{
    as_logical_scrollable, ScrollBar, ScrollBarVisibility, SnapPointsAlignment, SnapPointsType,
    TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::{
    as_scroll_anchor_provider, register_scroll_anchor_provider, ContentControl, ContentControlImpl,
    ContentControlImplExt, Control, ControlImpl, IScrollAnchorProvider, ScrollChangedEventArgs,
};
use ferroui_base::input::{
    FocusChangedEventArgs, IScrollable, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{CompositeDisposable, IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AttachedProperty, DirectProperty,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A control which scrolls its content if the content is bigger than the
/// space available.
#[repr(C)]
pub struct ScrollViewer {
    base: ContentControl,
    child_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The logically scrolling child and the subscription to its scroll
    /// invalidated event.
    logical_scrollable: RefCell<Option<(Ref<Control>, Rc<dyn IDisposable>)>>,
    extent: Cell<Size>,
    viewport: Cell<Size>,
    old_extent: Cell<Size>,
    old_offset: Cell<Vector>,
    old_maximum: Cell<Vector>,
    old_viewport: Cell<Size>,
    large_change: Cell<Size>,
    small_change: Cell<Size>,
    is_expanded: Cell<bool>,
    scroll_bar_expand_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    horizontal_scroll_bar: RefCell<Option<Ref<ScrollBar>>>,
    vertical_scroll_bar: RefCell<Option<Ref<ScrollBar>>>,
}

ferro_class! {
    ScrollViewer: ContentControl, virtuals ScrollViewerImpl: ContentControlImpl {
        /// Called when a change in scrolling state is detected, such as a
        /// change in scroll position, extent, or viewport size.
        ///
        /// If you override this method, call the parent implementation to
        /// ensure that the `ScrollChanged` event is raised.
        fn on_scroll_changed(this, e: &ScrollChangedEventArgs);
    }
}
// The `markup:` part is declared here and not generated: the public methods of the class are
// declared for markup, the commands of the scroll buttons of the menu scroll viewer in the control themes
// (`Command="{Binding LineUp, RelativeSource={RelativeSource TemplatedParent}}"`).
ferroui_base::ferro_class_info!(ScrollViewer {
    new: ScrollViewer::new,
    markup: {
        properties: [
            BringIntoViewOnFocusChange: bool {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::bring_into_view_on_focus_change_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: bool| this.set_value(ScrollViewer::bring_into_view_on_focus_change_property(), value)
            },
            HorizontalScrollBarVisibility: crate::primitives::ScrollBarVisibility {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::horizontal_scroll_bar_visibility_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: crate::primitives::ScrollBarVisibility| this.set_value(ScrollViewer::horizontal_scroll_bar_visibility_property(), value)
            },
            VerticalScrollBarVisibility: crate::primitives::ScrollBarVisibility {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::vertical_scroll_bar_visibility_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: crate::primitives::ScrollBarVisibility| this.set_value(ScrollViewer::vertical_scroll_bar_visibility_property(), value)
            },
            HorizontalSnapPointsType: crate::primitives::SnapPointsType {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::horizontal_snap_points_type_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: crate::primitives::SnapPointsType| this.set_value(ScrollViewer::horizontal_snap_points_type_property(), value)
            },
            VerticalSnapPointsType: crate::primitives::SnapPointsType {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::vertical_snap_points_type_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: crate::primitives::SnapPointsType| this.set_value(ScrollViewer::vertical_snap_points_type_property(), value)
            },
            HorizontalSnapPointsAlignment: crate::primitives::SnapPointsAlignment {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::horizontal_snap_points_alignment_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: crate::primitives::SnapPointsAlignment| this.set_value(ScrollViewer::horizontal_snap_points_alignment_property(), value)
            },
            VerticalSnapPointsAlignment: crate::primitives::SnapPointsAlignment {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::vertical_snap_points_alignment_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: crate::primitives::SnapPointsAlignment| this.set_value(ScrollViewer::vertical_snap_points_alignment_property(), value)
            },
            AllowAutoHide: bool {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::allow_auto_hide_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: bool| this.set_value(ScrollViewer::allow_auto_hide_property(), value)
            },
            IsScrollChainingEnabled: bool {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::is_scroll_chaining_enabled_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: bool| this.set_value(ScrollViewer::is_scroll_chaining_enabled_property(), value)
            },
            IsScrollInertiaEnabled: bool {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::is_scroll_inertia_enabled_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: bool| this.set_value(ScrollViewer::is_scroll_inertia_enabled_property(), value)
            },
            IsDeferredScrollingEnabled: bool {
                get: |this: &ferroui_base::Ref<ScrollViewer>| this.get_value(ScrollViewer::is_deferred_scrolling_enabled_property()),
                set: |this: &ferroui_base::Ref<ScrollViewer>, value: bool| this.set_value(ScrollViewer::is_deferred_scrolling_enabled_property(), value)
            },
        ],
        methods: [
            fn LineUp() => ScrollViewer::line_up,
            fn LineDown() => ScrollViewer::line_down,
            fn LineLeft() => ScrollViewer::line_left,
            fn LineRight() => ScrollViewer::line_right,
            fn PageUp() => ScrollViewer::page_up,
            fn PageDown() => ScrollViewer::page_down,
            fn PageLeft() => ScrollViewer::page_left,
            fn PageRight() => ScrollViewer::page_right,
            fn ScrollToHome() => ScrollViewer::scroll_to_home,
            fn ScrollToEnd() => ScrollViewer::scroll_to_end,
        ],
        fields: [
            ScrollChangedEvent: ferroui_base::interactivity::RoutedEvent<crate::ScrollChangedEventArgs> => || *ScrollViewer::scroll_changed_event(),
        ],
        attributes: [
            TemplatePart("PART_HorizontalScrollBar", type(ferroui_base::Ref<crate::primitives::ScrollBar>)),
            TemplatePart("PART_VerticalScrollBar", type(ferroui_base::Ref<crate::primitives::ScrollBar>)),
        ],
    },
});

ferro_impl_classes!(ScrollViewer: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

impl ControlImpl for ScrollViewer {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ScrollViewerAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for ScrollViewer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        let _ = this.layout_updated(move || {
            if let Some(this) = weak.upgrade() {
                this.raise_scroll_changed();
            }
        });
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::offset_property().as_property() {
            this.calculated_properties_changed();
        } else if property == Self::extent_property().as_property()
            || property == Self::viewport_property().as_property()
        {
            this.coerce_value(Self::offset_property().as_property());
        }
    }
}

impl InputElementImpl for ScrollViewer {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        if !e.is_source(this) && this.bring_into_view_on_focus_change() {
            if let Some(c) = e.source().and_then(|source| source.cast::<Control>()) {
                c.bring_into_view();
            }
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if e.key == Key::PageUp {
            this.page_up();
            e.set_handled(true);
        } else if e.key == Key::PageDown {
            this.page_down();
            e.set_handled(true);
        } else if e.is_source(this) {
            let rtl = this.flow_direction() == FlowDirection::RightToLeft;
            if e.key == Key::Left {
                if rtl {
                    this.line_right();
                } else {
                    this.line_left();
                }
                e.set_handled(true);
            } else if e.key == Key::Right {
                if rtl {
                    this.line_left();
                } else {
                    this.line_right();
                }
                e.set_handled(true);
            } else if e.key == Key::Up {
                this.line_up();
                e.set_handled(true);
            } else if e.key == Key::Down {
                this.line_down();
                e.set_handled(true);
            }
        }
    }

    fn as_scrollable(this: &Self) -> Option<&dyn IScrollable> {
        Some(this)
    }
}

impl TemplatedControlImpl for ScrollViewer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let old_subscription = this.scroll_bar_expand_subscription.borrow_mut().take();
        if let Some(old_subscription) = old_subscription {
            old_subscription.dispose();
        }

        let subscription = this.subscribe_to_scroll_bars(e);
        *this.scroll_bar_expand_subscription.borrow_mut() = subscription;
    }
}

impl ContentControlImpl for ScrollViewer {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        let old_subscription = this.child_subscription.borrow_mut().take();
        if let Some(old_subscription) = old_subscription {
            old_subscription.dispose();
        }

        if Self::parent_register_content_presenter(this, presenter) {
            if let Some(presenter) = this.presenter() {
                let weak = this.to_ref().downgrade();
                let subscription =
                    FerroObjectExtensions::get_observable(fo(&presenter), ContentPresenter::child_property())
                        .subscribe_fn(move |child| {
                            if let Some(this) = weak.upgrade() {
                                this.child_changed(child);
                            }
                        });
                *this.child_subscription.borrow_mut() = Some(subscription);
            }
            return true;
        }

        false
    }
}

impl ScrollViewerImpl for ScrollViewer {
    fn on_scroll_changed(this: &Self, e: &ScrollChangedEventArgs) {
        this.raise_event(e);
    }
}

impl IScrollable for ScrollViewer {
    fn extent(&self) -> Size {
        ScrollViewer::extent(self)
    }

    fn offset(&self) -> Vector {
        ScrollViewer::offset(self)
    }

    fn set_offset(&self, value: Vector) {
        ScrollViewer::set_offset(self, value)
    }

    fn viewport(&self) -> Size {
        ScrollViewer::viewport(self)
    }

    fn can_horizontally_scroll(&self) -> bool {
        ScrollViewer::can_horizontally_scroll(self)
    }

    fn can_vertically_scroll(&self) -> bool {
        ScrollViewer::can_vertically_scroll(self)
    }
}

impl IScrollAnchorProvider for ScrollViewer {
    fn current_anchor(&self) -> Option<Ref<Control>> {
        ScrollViewer::current_anchor(self)
    }

    fn register_anchor_candidate(&self, element: &Ref<Control>) {
        ScrollViewer::register_anchor_candidate(self, element)
    }

    fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        ScrollViewer::unregister_anchor_candidate(self, element)
    }
}

impl ScrollViewer {
    /// The named parts expected in the control template, in addition to
    /// those of the base class.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_HorizontalScrollBar", <ScrollBar as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_VerticalScrollBar", <ScrollBar as StaticType>::TYPE),
    ];

    /// The default small (line) change of the scroll viewer.
    pub(crate) const DEFAULT_SMALL_CHANGE: f64 = 16.0;
}

ferroui_base::ferro_properties! { impl ScrollViewer {
    ferro_property!(
        /// Defines the `BringIntoViewOnFocusChange` property.
        pub fn bring_into_view_on_focus_change_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>("BringIntoViewOnFocusChange", true)
        }
    );

    ferro_property!(
        /// Defines the `Extent` property.
        pub fn extent_property() -> DirectProperty<ScrollViewer, Size> {
            FerroProperty::register_direct::<ScrollViewer, _>("Extent", |o| o.extent(), None, Size::default())
        }
    );

    ferro_property!(
        /// Defines the `Offset` property.
        pub fn offset_property() -> StyledProperty<Vector> {
            FerroProperty::register_with::<ScrollViewer, _>(
                "Offset",
                StyledPropertyOptions::new(Vector::default()).coerce(ScrollViewer::coerce_offset),
            )
        }
    );

    ferro_property!(
        /// Defines the `Viewport` property.
        pub fn viewport_property() -> DirectProperty<ScrollViewer, Size> {
            FerroProperty::register_direct::<ScrollViewer, _>("Viewport", |o| o.viewport(), None, Size::default())
        }
    );

    ferro_property!(
        /// Defines the `LargeChange` property.
        pub fn large_change_property() -> DirectProperty<ScrollViewer, Size> {
            FerroProperty::register_direct::<ScrollViewer, _>(
                "LargeChange",
                |o| o.large_change(),
                None,
                Size::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `SmallChange` property.
        pub fn small_change_property() -> DirectProperty<ScrollViewer, Size> {
            FerroProperty::register_direct::<ScrollViewer, _>(
                "SmallChange",
                |o| o.small_change(),
                None,
                Size::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `ScrollBarMaximum` property.
        pub fn scroll_bar_maximum_property() -> DirectProperty<ScrollViewer, Vector> {
            FerroProperty::register_direct::<ScrollViewer, _>(
                "ScrollBarMaximum",
                |o| o.scroll_bar_maximum(),
                None,
                Vector::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `HorizontalScrollBarVisibility` property.
        pub fn horizontal_scroll_bar_visibility_property() -> AttachedProperty<ScrollBarVisibility> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>(
                "HorizontalScrollBarVisibility",
                ScrollBarVisibility::Disabled,
            )
        }
    );

    ferro_property!(
        /// Defines the `HorizontalSnapPointsType` property.
        pub fn horizontal_snap_points_type_property() -> AttachedProperty<SnapPointsType> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>(
                "HorizontalSnapPointsType",
                SnapPointsType::None,
            )
        }
    );

    ferro_property!(
        /// Defines the `VerticalSnapPointsType` property.
        pub fn vertical_snap_points_type_property() -> AttachedProperty<SnapPointsType> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>("VerticalSnapPointsType", SnapPointsType::None)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalSnapPointsAlignment` property.
        pub fn horizontal_snap_points_alignment_property() -> AttachedProperty<SnapPointsAlignment> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>(
                "HorizontalSnapPointsAlignment",
                SnapPointsAlignment::Near,
            )
        }
    );

    ferro_property!(
        /// Defines the `VerticalSnapPointsAlignment` property.
        pub fn vertical_snap_points_alignment_property() -> AttachedProperty<SnapPointsAlignment> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>(
                "VerticalSnapPointsAlignment",
                SnapPointsAlignment::Near,
            )
        }
    );

    ferro_property!(
        /// Defines the `VerticalScrollBarVisibility` property.
        pub fn vertical_scroll_bar_visibility_property() -> AttachedProperty<ScrollBarVisibility> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>(
                "VerticalScrollBarVisibility",
                ScrollBarVisibility::Auto,
            )
        }
    );

    ferro_property!(
        /// Defines the `IsExpanded` property.
        pub fn is_expanded_property() -> DirectProperty<ScrollViewer, bool> {
            ScrollBar::is_expanded_property().add_owner::<ScrollViewer>(|o| o.is_expanded(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `AllowAutoHide` property.
        pub fn allow_auto_hide_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>("AllowAutoHide", true)
        }
    );

    ferro_property!(
        /// Defines the `IsScrollChainingEnabled` property.
        pub fn is_scroll_chaining_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>("IsScrollChainingEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsScrollInertiaEnabled` property.
        pub fn is_scroll_inertia_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>("IsScrollInertiaEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsDeferredScrollingEnabled` property.
        pub fn is_deferred_scrolling_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<ScrollViewer, Control, _>("IsDeferredScrollingEnabled", false)
        }
    );
} }

impl ScrollViewer {
    ferro_routed_event!(
        /// Defines the `ScrollChanged` event.
        pub fn scroll_changed_event() -> RoutedEvent<ScrollChangedEventArgs> {
            RoutedEvent::register::<ScrollViewer, _>("ScrollChanged", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        Self::scroll_changed_event();

        register_scroll_anchor_provider::<ScrollViewer>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            child_subscription: RefCell::new(None),
            logical_scrollable: RefCell::new(None),
            extent: Cell::new(Size::default()),
            viewport: Cell::new(Size::default()),
            old_extent: Cell::new(Size::default()),
            old_offset: Cell::new(Vector::default()),
            old_maximum: Cell::new(Vector::default()),
            old_viewport: Cell::new(Size::default()),
            large_change: Cell::new(Size::default()),
            small_change: Cell::new(Size::new(Self::DEFAULT_SMALL_CHANGE, Self::DEFAULT_SMALL_CHANGE)),
            is_expanded: Cell::new(false),
            scroll_bar_expand_subscription: RefCell::new(None),
            horizontal_scroll_bar: RefCell::new(None),
            vertical_scroll_bar: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the horizontal scroll bar from the applied template, if any.
    pub(crate) fn horizontal_scroll_bar(&self) -> Option<Ref<ScrollBar>> {
        self.horizontal_scroll_bar.borrow().clone()
    }

    /// Gets the vertical scroll bar from the applied template, if any.
    pub(crate) fn vertical_scroll_bar(&self) -> Option<Ref<ScrollBar>> {
        self.vertical_scroll_bar.borrow().clone()
    }

    /// Occurs when changes are detected to the scroll position, extent, or
    /// viewport size.
    pub fn scroll_changed(
        &self,
        handler: impl Fn(&Interactive, &ScrollChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::scroll_changed_event(), handler)
    }

    /// Gets a value that determines whether the scroll viewer uses a
    /// bring-into-view scroll behavior when an item in the view gets focus.
    ///
    /// True to use a behavior that brings focused items into view. False to
    /// use a behavior that focused items do not automatically scroll into
    /// view. The default is true.
    ///
    /// The value can either be set explicitly on a scroll viewer, or the
    /// attached `ScrollViewer.BringIntoViewOnFocusChange` property can be
    /// set on an element that hosts a scroll viewer.
    pub fn bring_into_view_on_focus_change(&self) -> bool {
        self.get_value(Self::bring_into_view_on_focus_change_property())
    }

    /// Sets a value that determines whether the scroll viewer uses a
    /// bring-into-view scroll behavior when an item in the view gets focus.
    pub fn set_bring_into_view_on_focus_change(&self, value: bool) {
        self.set_value(Self::bring_into_view_on_focus_change_property(), value)
    }

    /// Gets the extent of the scrollable content.
    pub fn extent(&self) -> Size {
        self.extent.get()
    }

    pub(crate) fn set_extent(&self, value: Size) {
        if self.set_and_raise_cell(Self::extent_property(), &self.extent, value) {
            self.calculated_properties_changed();
        }
    }

    /// Gets the current scroll offset.
    pub fn offset(&self) -> Vector {
        self.get_value(Self::offset_property())
    }

    /// Sets the current scroll offset.
    pub fn set_offset(&self, value: Vector) {
        self.set_value(Self::offset_property(), value)
    }

    /// Gets the size of the viewport on the scrollable content.
    pub fn viewport(&self) -> Size {
        self.viewport.get()
    }

    pub(crate) fn set_viewport(&self, value: Size) {
        if self.set_and_raise_cell(Self::viewport_property(), &self.viewport, value) {
            self.calculated_properties_changed();
        }
    }

    /// Gets the large (page) change value for the scroll viewer.
    pub fn large_change(&self) -> Size {
        self.large_change.get()
    }

    /// Gets the small (line) change value for the scroll viewer.
    pub fn small_change(&self) -> Size {
        self.small_change.get()
    }

    /// Gets the horizontal scrollbar visibility.
    pub fn horizontal_scroll_bar_visibility(&self) -> ScrollBarVisibility {
        self.get_value(Self::horizontal_scroll_bar_visibility_property())
    }

    /// Sets the horizontal scrollbar visibility.
    pub fn set_horizontal_scroll_bar_visibility(&self, value: ScrollBarVisibility) {
        self.set_value(Self::horizontal_scroll_bar_visibility_property(), value)
    }

    /// Gets the vertical scrollbar visibility.
    pub fn vertical_scroll_bar_visibility(&self) -> ScrollBarVisibility {
        self.get_value(Self::vertical_scroll_bar_visibility_property())
    }

    /// Sets the vertical scrollbar visibility.
    pub fn set_vertical_scroll_bar_visibility(&self, value: ScrollBarVisibility) {
        self.set_value(Self::vertical_scroll_bar_visibility_property(), value)
    }

    /// Gets a value indicating whether the viewer can scroll horizontally.
    pub fn can_horizontally_scroll(&self) -> bool {
        self.horizontal_scroll_bar_visibility() != ScrollBarVisibility::Disabled
    }

    /// Gets a value indicating whether the viewer can scroll vertically.
    pub fn can_vertically_scroll(&self) -> bool {
        self.vertical_scroll_bar_visibility() != ScrollBarVisibility::Disabled
    }

    /// The currently chosen anchor element to use for scroll anchoring.
    pub fn current_anchor(&self) -> Option<Ref<Control>> {
        let presenter = self.presenter()?;
        as_scroll_anchor_provider(&presenter)?.current_anchor()
    }

    /// Gets the maximum scrolling distance (which is the extent minus the
    /// viewport).
    pub fn scroll_bar_maximum(&self) -> Vector {
        let extent = self.extent.get();
        let viewport = self.viewport.get();
        Vector::new(
            Self::max(extent.width - viewport.width, 0.0),
            Self::max(extent.height - viewport.height, 0.0),
        )
    }

    /// Gets a value that indicates whether any scrollbar is expanded.
    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        self.set_and_raise_cell(ScrollBar::is_expanded_property(), &self.is_expanded, value);
    }

    /// Gets how scroll gesture reacts to the snap points along the
    /// horizontal axis.
    pub fn horizontal_snap_points_type(&self) -> SnapPointsType {
        self.get_value(Self::horizontal_snap_points_type_property())
    }

    /// Sets how scroll gesture reacts to the snap points along the
    /// horizontal axis.
    pub fn set_horizontal_snap_points_type(&self, value: SnapPointsType) {
        self.set_value(Self::horizontal_snap_points_type_property(), value)
    }

    /// Gets how scroll gesture reacts to the snap points along the vertical
    /// axis.
    pub fn vertical_snap_points_type(&self) -> SnapPointsType {
        self.get_value(Self::vertical_snap_points_type_property())
    }

    /// Sets how scroll gesture reacts to the snap points along the vertical
    /// axis.
    pub fn set_vertical_snap_points_type(&self, value: SnapPointsType) {
        self.set_value(Self::vertical_snap_points_type_property(), value)
    }

    /// Gets how the existing snap points are horizontally aligned versus the
    /// initial viewport.
    pub fn horizontal_snap_points_alignment(&self) -> SnapPointsAlignment {
        self.get_value(Self::horizontal_snap_points_alignment_property())
    }

    /// Sets how the existing snap points are horizontally aligned versus the
    /// initial viewport.
    pub fn set_horizontal_snap_points_alignment(&self, value: SnapPointsAlignment) {
        self.set_value(Self::horizontal_snap_points_alignment_property(), value)
    }

    /// Gets how the existing snap points are vertically aligned versus the
    /// initial viewport.
    pub fn vertical_snap_points_alignment(&self) -> SnapPointsAlignment {
        self.get_value(Self::vertical_snap_points_alignment_property())
    }

    /// Sets how the existing snap points are vertically aligned versus the
    /// initial viewport.
    pub fn set_vertical_snap_points_alignment(&self, value: SnapPointsAlignment) {
        self.set_value(Self::vertical_snap_points_alignment_property(), value)
    }

    /// Gets a value that indicates whether scrollbars can hide itself when
    /// user is not interacting with it.
    pub fn allow_auto_hide(&self) -> bool {
        self.get_value(Self::allow_auto_hide_property())
    }

    /// Sets a value that indicates whether scrollbars can hide itself when
    /// user is not interacting with it.
    pub fn set_allow_auto_hide(&self, value: bool) {
        self.set_value(Self::allow_auto_hide_property(), value)
    }

    /// Gets if scroll chaining is enabled. The default value is true.
    ///
    /// After a user hits a scroll limit on an element that has been nested
    /// within another scrollable element, you can specify whether that
    /// parent element should continue the scrolling operation begun in its
    /// child element. This is called scroll chaining.
    pub fn is_scroll_chaining_enabled(&self) -> bool {
        self.get_value(Self::is_scroll_chaining_enabled_property())
    }

    /// Sets if scroll chaining is enabled.
    pub fn set_is_scroll_chaining_enabled(&self, value: bool) {
        self.set_value(Self::is_scroll_chaining_enabled_property(), value)
    }

    /// Gets whether scroll gestures should include inertia in their behavior
    /// and value.
    pub fn is_scroll_inertia_enabled(&self) -> bool {
        self.get_value(Self::is_scroll_inertia_enabled_property())
    }

    /// Sets whether scroll gestures should include inertia in their behavior
    /// and value.
    pub fn set_is_scroll_inertia_enabled(&self, value: bool) {
        self.set_value(Self::is_scroll_inertia_enabled_property(), value)
    }

    /// Gets whether dragging of thumb elements should update the scroll
    /// viewer only when the user releases the mouse.
    pub fn is_deferred_scrolling_enabled(&self) -> bool {
        self.get_value(Self::is_deferred_scrolling_enabled_property())
    }

    /// Sets whether dragging of thumb elements should update the scroll
    /// viewer only when the user releases the mouse.
    pub fn set_is_deferred_scrolling_enabled(&self, value: bool) {
        self.set_value(Self::is_deferred_scrolling_enabled_property(), value)
    }

    /// Scrolls the content up one line.
    pub fn line_up(&self) {
        let offset = self.offset() - Vector::new(0.0, self.small_change.get().height);
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content down one line.
    pub fn line_down(&self) {
        let offset = self.offset() + Vector::new(0.0, self.small_change.get().height);
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content left one line.
    pub fn line_left(&self) {
        let offset = self.offset() - Vector::new(self.small_change.get().width, 0.0);
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content right one line.
    pub fn line_right(&self) {
        let offset = self.offset() + Vector::new(self.small_change.get().width, 0.0);
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content upward by one page.
    pub fn page_up(&self) {
        let offset = self.offset();
        let offset = offset.with_y((offset.y - self.viewport.get().height).max(0.0));
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content downward by one page.
    pub fn page_down(&self) {
        let offset = self.offset();
        let offset = offset.with_y((offset.y + self.viewport.get().height).min(self.scroll_bar_maximum().y));
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content left by one page.
    pub fn page_left(&self) {
        let offset = self.offset();
        let offset = offset.with_x((offset.x - self.viewport.get().width).max(0.0));
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls the content right by one page.
    pub fn page_right(&self) {
        let offset = self.offset();
        let offset = offset.with_x((offset.x + self.viewport.get().width).min(self.scroll_bar_maximum().x));
        self.set_current_value(Self::offset_property(), offset);
    }

    /// Scrolls to the top-left corner of the content.
    pub fn scroll_to_home(&self) {
        self.set_current_value(
            Self::offset_property(),
            Vector::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
    }

    /// Scrolls to the bottom-left corner of the content.
    pub fn scroll_to_end(&self) {
        self.set_current_value(Self::offset_property(), Vector::new(f64::NEG_INFINITY, f64::INFINITY));
    }

    /// Gets the value of the `BringIntoViewOnFocusChange` attached property
    /// of `control`.
    pub fn get_bring_into_view_on_focus_change(control: &Control) -> bool {
        control.get_value(Self::bring_into_view_on_focus_change_property())
    }

    /// Sets the value of the `BringIntoViewOnFocusChange` attached property
    /// on `control`.
    pub fn set_bring_into_view_on_focus_change_on(control: &Control, value: bool) {
        control.set_value(Self::bring_into_view_on_focus_change_property(), value);
    }

    /// Gets the value of the `HorizontalScrollBarVisibility` attached
    /// property of `control`.
    pub fn get_horizontal_scroll_bar_visibility(control: &Control) -> ScrollBarVisibility {
        control.get_value(Self::horizontal_scroll_bar_visibility_property())
    }

    /// Sets the value of the `HorizontalScrollBarVisibility` attached
    /// property on `control`.
    pub fn set_horizontal_scroll_bar_visibility_on(control: &Control, value: ScrollBarVisibility) {
        control.set_value(Self::horizontal_scroll_bar_visibility_property(), value);
    }

    /// Gets the value of the `HorizontalSnapPointsType` attached property of
    /// `control`.
    pub fn get_horizontal_snap_points_type(control: &Control) -> SnapPointsType {
        control.get_value(Self::horizontal_snap_points_type_property())
    }

    /// Sets the value of the `HorizontalSnapPointsType` attached property on
    /// `control`.
    pub fn set_horizontal_snap_points_type_on(control: &Control, value: SnapPointsType) {
        control.set_value(Self::horizontal_snap_points_type_property(), value);
    }

    /// Gets the value of the `VerticalSnapPointsType` attached property of
    /// `control`.
    pub fn get_vertical_snap_points_type(control: &Control) -> SnapPointsType {
        control.get_value(Self::vertical_snap_points_type_property())
    }

    /// Sets the value of the `VerticalSnapPointsType` attached property on
    /// `control`.
    pub fn set_vertical_snap_points_type_on(control: &Control, value: SnapPointsType) {
        control.set_value(Self::vertical_snap_points_type_property(), value);
    }

    /// Gets the value of the `HorizontalSnapPointsAlignment` attached
    /// property of `control`.
    pub fn get_horizontal_snap_points_alignment(control: &Control) -> SnapPointsAlignment {
        control.get_value(Self::horizontal_snap_points_alignment_property())
    }

    /// Sets the value of the `HorizontalSnapPointsAlignment` attached
    /// property on `control`.
    pub fn set_horizontal_snap_points_alignment_on(control: &Control, value: SnapPointsAlignment) {
        control.set_value(Self::horizontal_snap_points_alignment_property(), value);
    }

    /// Gets the value of the `VerticalSnapPointsAlignment` attached property
    /// of `control`.
    pub fn get_vertical_snap_points_alignment(control: &Control) -> SnapPointsAlignment {
        control.get_value(Self::vertical_snap_points_alignment_property())
    }

    /// Sets the value of the `VerticalSnapPointsAlignment` attached property
    /// on `control`.
    pub fn set_vertical_snap_points_alignment_on(control: &Control, value: SnapPointsAlignment) {
        control.set_value(Self::vertical_snap_points_alignment_property(), value);
    }

    /// Gets the value of the `VerticalScrollBarVisibility` attached property
    /// of `control`.
    pub fn get_vertical_scroll_bar_visibility(control: &Control) -> ScrollBarVisibility {
        control.get_value(Self::vertical_scroll_bar_visibility_property())
    }

    /// Sets the value of the `AllowAutoHide` attached property on `control`.
    pub fn set_allow_auto_hide_on(control: &Control, value: bool) {
        control.set_value(Self::allow_auto_hide_property(), value);
    }

    /// Gets the value of the `AllowAutoHide` attached property of `control`.
    pub fn get_allow_auto_hide(control: &Control) -> bool {
        control.get_value(Self::allow_auto_hide_property())
    }

    /// Sets the value of the `IsScrollChainingEnabled` attached property on
    /// `control`.
    ///
    /// After a user hits a scroll limit on an element that has been nested
    /// within another scrollable element, you can specify whether that
    /// parent element should continue the scrolling operation begun in its
    /// child element. This is called scroll chaining.
    pub fn set_is_scroll_chaining_enabled_on(control: &Control, value: bool) {
        control.set_value(Self::is_scroll_chaining_enabled_property(), value);
    }

    /// Gets the value of the `IsScrollChainingEnabled` attached property of
    /// `control`.
    ///
    /// After a user hits a scroll limit on an element that has been nested
    /// within another scrollable element, you can specify whether that
    /// parent element should continue the scrolling operation begun in its
    /// child element. This is called scroll chaining.
    pub fn get_is_scroll_chaining_enabled(control: &Control) -> bool {
        control.get_value(Self::is_scroll_chaining_enabled_property())
    }

    /// Sets the value of the `VerticalScrollBarVisibility` attached property
    /// on `control`.
    pub fn set_vertical_scroll_bar_visibility_on(control: &Control, value: ScrollBarVisibility) {
        control.set_value(Self::vertical_scroll_bar_visibility_property(), value);
    }

    /// Gets whether scroll gestures should include inertia in their behavior
    /// and value.
    pub fn get_is_scroll_inertia_enabled(control: &Control) -> bool {
        control.get_value(Self::is_scroll_inertia_enabled_property())
    }

    /// Sets whether scroll gestures should include inertia in their behavior
    /// and value.
    pub fn set_is_scroll_inertia_enabled_on(control: &Control, value: bool) {
        control.set_value(Self::is_scroll_inertia_enabled_property(), value);
    }

    /// Gets whether dragging of thumb elements should update the scroll
    /// viewer only when the user releases the mouse.
    pub fn get_is_deferred_scrolling_enabled(control: &Control) -> bool {
        control.get_value(Self::is_deferred_scrolling_enabled_property())
    }

    /// Sets whether dragging of thumb elements should update the scroll
    /// viewer only when the user releases the mouse.
    pub fn set_is_deferred_scrolling_enabled_on(control: &Control, value: bool) {
        control.set_value(Self::is_deferred_scrolling_enabled_property(), value);
    }

    /// Registers a control as a potential scroll anchor candidate.
    pub fn register_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(presenter) = self.presenter() {
            if let Some(provider) = as_scroll_anchor_provider(&presenter) {
                provider.register_anchor_candidate(element);
            }
        }
    }

    /// Unregisters a control as a potential scroll anchor candidate.
    pub fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        if let Some(presenter) = self.presenter() {
            if let Some(provider) = as_scroll_anchor_provider(&presenter) {
                provider.unregister_anchor_candidate(element);
            }
        }
    }

    /// Coerces an offset to the scrollable range of `sender`.
    pub(crate) fn coerce_offset(sender: &FerroObject, value: Vector) -> Vector {
        let extent = sender.get_direct_value(Self::extent_property());
        let viewport = sender.get_direct_value(Self::viewport_property());

        let max_x = (extent.width - viewport.width).max(0.0);
        let max_y = (extent.height - viewport.height).max(0.0);
        Vector::new(Self::clamp(value.x, 0.0, max_x), Self::clamp(value.y, 0.0, max_y))
    }

    fn clamp(value: f64, min: f64, max: f64) -> f64 {
        // A NaN offset must never survive. Offset is two-way coerced between
        // the scroll viewer and its presenter; because NaN != NaN, a NaN
        // offset never compares equal, so the coerce/raise cycle never
        // converges and recurses until it overflows the stack.
        if value.is_nan() {
            return min;
        }

        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    fn max(x: f64, y: f64) -> f64 {
        // The reference maximum propagates NaN, which is then replaced.
        if x.is_nan() || y.is_nan() {
            return 0.0;
        }
        x.max(y)
    }

    fn child_changed(&self, child: Option<Ref<Control>>) {
        let old = self.logical_scrollable.borrow_mut().take();
        if let Some((_, subscription)) = old {
            subscription.dispose();
        }

        if let Some(child) = child {
            if let Some(logical) = as_logical_scrollable(&child) {
                let weak = self.to_ref().downgrade();
                let subscription = logical.scroll_invalidated(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.calculated_properties_changed();
                    }
                }));
                *self.logical_scrollable.borrow_mut() = Some((child.clone(), subscription));
            }
        }

        self.calculated_properties_changed();
    }

    fn calculated_properties_changed(&self) {
        let new_maximum = self.scroll_bar_maximum();
        let old_maximum = self.old_maximum.get();
        if new_maximum != old_maximum {
            self.raise_direct_property_changed(Self::scroll_bar_maximum_property(), &old_maximum, &new_maximum);
            self.old_maximum.set(new_maximum);
        }

        let logical_scrollable = self
            .logical_scrollable
            .borrow()
            .as_ref()
            .map(|(child, _)| child.clone());
        let logical = logical_scrollable
            .as_ref()
            .and_then(|child| as_logical_scrollable(child))
            .filter(|logical| logical.is_logical_scroll_enabled());

        if let Some(logical) = logical {
            self.set_and_raise_cell(Self::small_change_property(), &self.small_change, logical.scroll_size());
            self.set_and_raise_cell(
                Self::large_change_property(),
                &self.large_change,
                logical.page_scroll_size(),
            );
        } else {
            self.set_and_raise_cell(
                Self::small_change_property(),
                &self.small_change,
                Size::new(Self::DEFAULT_SMALL_CHANGE, Self::DEFAULT_SMALL_CHANGE),
            );
            self.set_and_raise_cell(Self::large_change_property(), &self.large_change, self.viewport());
        }
    }

    fn subscribe_to_scroll_bars(&self, e: &TemplateAppliedEventArgs) -> Option<Rc<dyn IDisposable>> {
        let horizontal_scroll_bar = e.name_scope().find_as::<ScrollBar>("PART_HorizontalScrollBar");
        let vertical_scroll_bar = e.name_scope().find_as::<ScrollBar>("PART_VerticalScrollBar");

        *self.horizontal_scroll_bar.borrow_mut() = horizontal_scroll_bar.clone();
        *self.vertical_scroll_bar.borrow_mut() = vertical_scroll_bar.clone();

        let horizontal_expanded = horizontal_scroll_bar.map(|scroll_bar| {
            FerroObjectExtensions::get_observable(fo(&scroll_bar), ScrollBar::is_expanded_property())
        });
        let vertical_expanded = vertical_scroll_bar.map(|scroll_bar| {
            FerroObjectExtensions::get_observable(fo(&scroll_bar), ScrollBar::is_expanded_property())
        });

        let weak = self.to_ref().downgrade();
        let on_scroll_bar_expanded_changed = move |is_expanded: bool| {
            if let Some(this) = weak.upgrade() {
                this.set_is_expanded(is_expanded);
            }
        };

        match (horizontal_expanded, vertical_expanded) {
            (Some(horizontal_expanded), Some(vertical_expanded)) => {
                // The latest values of both scroll bars are combined once
                // each of them has produced a value.
                let latest: Rc<Cell<(Option<bool>, Option<bool>)>> = Rc::new(Cell::new((None, None)));
                let publish = Rc::new(move |latest: (Option<bool>, Option<bool>)| {
                    if let (Some(h), Some(v)) = latest {
                        on_scroll_bar_expanded_changed(h || v);
                    }
                });

                let horizontal = {
                    let latest = latest.clone();
                    let publish = publish.clone();
                    horizontal_expanded.subscribe_fn(move |h| {
                        latest.set((Some(h), latest.get().1));
                        publish(latest.get());
                    })
                };
                let vertical = vertical_expanded.subscribe_fn(move |v| {
                    latest.set((latest.get().0, Some(v)));
                    publish(latest.get());
                });

                let subscription: Rc<dyn IDisposable> =
                    Rc::new(CompositeDisposable::from_disposables([horizontal, vertical]));
                Some(subscription)
            }
            (Some(actual_expanded), None) | (None, Some(actual_expanded)) => {
                Some(actual_expanded.subscribe_fn(on_scroll_bar_expanded_changed))
            }
            (None, None) => None,
        }
    }

    fn raise_scroll_changed(&self) {
        let extent = self.extent();
        let offset = self.offset();
        let viewport = self.viewport();
        let old_extent = self.old_extent.get();
        let old_viewport = self.old_viewport.get();

        let extent_delta = Vector::new(extent.width - old_extent.width, extent.height - old_extent.height);
        let offset_delta = offset - self.old_offset.get();
        let viewport_delta = Vector::new(
            viewport.width - old_viewport.width,
            viewport.height - old_viewport.height,
        );

        if !extent_delta.nearly_equals(Vector::default())
            || !offset_delta.nearly_equals(Vector::default())
            || !viewport_delta.nearly_equals(Vector::default())
        {
            let e = ScrollChangedEventArgs::new(extent_delta, offset_delta, viewport_delta);
            self.on_scroll_changed(&e);

            self.old_extent.set(self.extent());
            self.old_offset.set(self.offset());
            self.old_viewport.set(self.viewport());
        }
    }
}

/// The object as its root class, for calls that would otherwise resolve to
/// a member of an intermediate class.
fn fo(object: &ferroui_base::FerroObject) -> &ferroui_base::FerroObject {
    object
}
