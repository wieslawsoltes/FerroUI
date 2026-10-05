use crate::presenters::ScrollContentPresenter;
use crate::primitives::{register_logical_scrollable, ILogicalScrollable};
use crate::utils::TimeUtils;
use crate::{Control, ControlImpl, Controls, ListBoxItem, Panel, PanelImpl};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::{
    IScrollable, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, NavigationDirection, ScrollGestureEndedEventArgs,
    TappedEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt, VerticalAlignment};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{DateTime, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, AnyValue, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    Vector, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What a [`DateTimePickerPanel`] displays, in date or time units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DateTimePickerPanelType {
    #[default]
    Year = 0,
    Month = 1,
    Day = 2,
    Hour = 3,
    Minute = 4,
    /// AM or PM.
    TimePeriod = 5,
    Second = 6,
}

impl DateTimePickerPanelType {
    /// The name of the member, as upstream `ToString()` gives it.
    pub const fn name(self) -> &'static str {
        match self {
            DateTimePickerPanelType::Year => "Year",
            DateTimePickerPanelType::Month => "Month",
            DateTimePickerPanelType::Day => "Day",
            DateTimePickerPanelType::Hour => "Hour",
            DateTimePickerPanelType::Minute => "Minute",
            DateTimePickerPanelType::TimePeriod => "TimePeriod",
            DateTimePickerPanelType::Second => "Second",
        }
    }

    /// The style class of the items of a panel of this type.
    const fn item_class(self) -> &'static str {
        match self {
            DateTimePickerPanelType::Year => "YearItem",
            DateTimePickerPanelType::Month => "MonthItem",
            DateTimePickerPanelType::Day => "DayItem",
            DateTimePickerPanelType::Hour => "HourItem",
            DateTimePickerPanelType::Minute => "MinuteItem",
            DateTimePickerPanelType::TimePeriod => "TimePeriodItem",
            DateTimePickerPanelType::Second => "SecondItem",
        }
    }
}

/// The looping, virtualizing panel that shows the values of one unit of a
/// date or time picker.
#[repr(C)]
pub struct DateTimePickerPanel {
    base: Panel,

    // Backing fields for properties
    minimum_value: Cell<i32>,
    maximum_value: Cell<i32>,
    selected_value: Cell<i32>,
    increment: Cell<i32>,

    // Helper fields
    selected_index: Cell<i32>,
    total_items: Cell<i32>,
    num_items_above_below_selected: Cell<i32>,
    range: Cell<i32>,
    extent_one: Cell<f64>,
    extent: Cell<Size>,
    offset: Cell<Vector>,
    has_init: Cell<bool>,
    suppress_update_offset: Cell<bool>,
    parent_scroller: RefCell<Option<WeakRef<ScrollContentPresenter>>>,
    /// The handler of the scroll gesture ended event of the parent scroller,
    /// while it is added.
    scroll_gesture_ended_handler: Cell<Option<RoutedEventHandlerToken>>,

    format_date: Cell<DateTime>,
    scroll_invalidated: Rc<HandlerList<dyn Fn()>>,
    selection_changed: HandlerList<dyn Fn()>,
}

ferro_class!(DateTimePickerPanel: Panel);

ferro_class_info!(DateTimePickerPanel {
    new: DateTimePickerPanel::new,
    markup: {
        properties: [
            MinimumValue: i32 { get: DateTimePickerPanel::minimum_value, set: DateTimePickerPanel::set_minimum_value },
            MaximumValue: i32 { get: DateTimePickerPanel::maximum_value, set: DateTimePickerPanel::set_maximum_value },
            SelectedValue: i32 { get: DateTimePickerPanel::selected_value, set: DateTimePickerPanel::set_selected_value },
            Increment: i32 { get: DateTimePickerPanel::increment, set: DateTimePickerPanel::set_increment },
            Offset: Vector { get: DateTimePickerPanel::offset, set: DateTimePickerPanel::set_offset },
            CanHorizontallyScroll: bool {
                get: DateTimePickerPanel::can_horizontally_scroll,
                set: DateTimePickerPanel::set_can_horizontally_scroll
            },
            CanVerticallyScroll: bool {
                get: DateTimePickerPanel::can_vertically_scroll,
                set: DateTimePickerPanel::set_can_vertically_scroll
            },
            IsLogicalScrollEnabled: bool { get: DateTimePickerPanel::is_logical_scroll_enabled },
            ScrollSize: Size { get: DateTimePickerPanel::scroll_size },
            PageScrollSize: Size { get: DateTimePickerPanel::page_scroll_size },
            Extent: Size { get: DateTimePickerPanel::extent },
            Viewport: Size { get: DateTimePickerPanel::viewport },
        ],
    },
});

ferro_impl_classes!(DateTimePickerPanel: StyledElementImpl, InteractiveImpl, ControlImpl, PanelImpl);

impl FerroObjectImpl for DateTimePickerPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::tapped_event(),
            move |_, e: &TappedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_item_tapped(e);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::item_height_property().as_property() {
            let item_height = this.item_height();
            for child in this.children().snapshot().iter() {
                child.set_height(item_height);
            }

            this.update_helper_info();
            this.has_init.set(false);
        }
    }
}

impl VisualImpl for DateTimePickerPanel {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let parent_scroller = this.get_visual_parent().and_then(|parent| parent.cast::<ScrollContentPresenter>());
        if let Some(parent_scroller) = &parent_scroller {
            let weak = this.to_ref().downgrade();
            let token = parent_scroller.add_handler(
                InputElement::scroll_gesture_ended_event(),
                move |_, e: &ScrollGestureEndedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.on_scroll_gesture_ended(e);
                    }
                },
            );
            this.scroll_gesture_ended_handler.set(Some(token));
        }
        *this.parent_scroller.borrow_mut() = parent_scroller.as_ref().map(Ref::downgrade);
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let parent_scroller = this.parent_scroller.take().and_then(|weak| weak.upgrade());
        let token = this.scroll_gesture_ended_handler.take();
        if let (Some(parent_scroller), Some(token)) = (parent_scroller, token) {
            parent_scroller.remove_handler(InputElement::scroll_gesture_ended_event(), token);
        }
    }
}

impl LayoutableImpl for DateTimePickerPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if available_size.width.is_infinite() || available_size.height.is_infinite() {
            panic!("Panel must have finite height");
        }

        if !this.has_init.get() {
            this.update_helper_info();
        }

        let item_height = this.item_height();
        let init_y = (available_size.height / 2.0) - (item_height / 2.0);
        this.num_items_above_below_selected.set((init_y / item_height).ceil() as i32 + 1);

        let children = this.children();

        this.create_or_destroy_items(&children);

        for child in children.snapshot().iter() {
            child.measure(available_size);
        }

        if !this.has_init.get() {
            this.update_items();
            this.raise_scroll_invalidated();
            this.has_init.set(true);
        }

        available_size
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children();
        if children.count() == 0 {
            return Self::parent_arrange_override(this, final_size);
        }

        let item_hgt = this.item_height();
        let offset_y = this.offset.get().y;
        let mut init_y = (final_size.height / 2.0) - (item_hgt / 2.0);

        if this.should_loop() {
            let extent_one = this.extent_one.get();
            let current_set = (offset_y / extent_one).trunc();
            init_y += (extent_one * current_set)
                + f64::from(this.selected_index.get() - this.num_items_above_below_selected.get()) * item_hgt;

            for child in children.snapshot().iter() {
                let rc = Rect::new(0.0, init_y - offset_y, final_size.width, item_hgt);
                child.arrange(rc);
                init_y += item_hgt;
            }
        } else {
            let first = f64::from((this.selected_index.get() - this.num_items_above_below_selected.get()).max(0));
            for child in children.snapshot().iter() {
                let rc = Rect::new(0.0, (init_y + first * item_hgt) - offset_y, final_size.width, item_hgt);
                child.arrange(rc);
                init_y += item_hgt;
            }
        }

        final_size
    }
}

impl InputElementImpl for DateTimePickerPanel {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        match e.key {
            Key::Up => {
                this.scroll_up(1);
                e.set_handled(true);
            }
            Key::Down => {
                this.scroll_down(1);
                e.set_handled(true);
            }
            Key::PageUp => {
                this.scroll_up(4);
                e.set_handled(true);
            }
            Key::PageDown => {
                this.scroll_down(4);
                e.set_handled(true);
            }
            _ => {}
        }
        Self::parent_on_key_down(this, e);
    }
}

impl IScrollable for DateTimePickerPanel {
    fn extent(&self) -> Size {
        DateTimePickerPanel::extent(self)
    }

    fn offset(&self) -> Vector {
        DateTimePickerPanel::offset(self)
    }

    fn set_offset(&self, value: Vector) {
        DateTimePickerPanel::set_offset(self, value)
    }

    fn viewport(&self) -> Size {
        DateTimePickerPanel::viewport(self)
    }

    fn can_horizontally_scroll(&self) -> bool {
        DateTimePickerPanel::can_horizontally_scroll(self)
    }

    fn can_vertically_scroll(&self) -> bool {
        DateTimePickerPanel::can_vertically_scroll(self)
    }
}

impl ILogicalScrollable for DateTimePickerPanel {
    fn set_can_horizontally_scroll(&self, value: bool) {
        DateTimePickerPanel::set_can_horizontally_scroll(self, value)
    }

    fn set_can_vertically_scroll(&self, value: bool) {
        DateTimePickerPanel::set_can_vertically_scroll(self, value)
    }

    fn is_logical_scroll_enabled(&self) -> bool {
        DateTimePickerPanel::is_logical_scroll_enabled(self)
    }

    fn scroll_size(&self) -> Size {
        DateTimePickerPanel::scroll_size(self)
    }

    fn page_scroll_size(&self) -> Size {
        DateTimePickerPanel::page_scroll_size(self)
    }

    fn scroll_invalidated(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.scroll_invalidated.add(handler);
        let handlers = Rc::downgrade(&self.scroll_invalidated);
        Disposable::create(move || {
            if let Some(handlers) = handlers.upgrade() {
                handlers.remove(token);
            }
        })
    }

    fn bring_into_view(&self, target: &Ref<Control>, target_rect: Rect) -> bool {
        DateTimePickerPanel::bring_into_view(self, target, target_rect)
    }

    fn get_control_in_direction(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        DateTimePickerPanel::get_control_in_direction(self, direction, from)
    }

    fn raise_scroll_invalidated(&self) {
        DateTimePickerPanel::raise_scroll_invalidated(self)
    }
}

ferro_properties! {
    impl DateTimePickerPanel {
        /// Defines the `ItemHeight` property.
        pub fn item_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<DateTimePickerPanel, _>("ItemHeight", 40.0)
        }

        /// Defines the `PanelType` property.
        pub fn panel_type_property() -> StyledProperty<DateTimePickerPanelType> {
            FerroProperty::register::<DateTimePickerPanel, _>("PanelType", DateTimePickerPanelType::Year)
        }

        /// Defines the `ItemFormat` property.
        pub fn item_format_property() -> StyledProperty<String> {
            FerroProperty::register::<DateTimePickerPanel, _>("ItemFormat", "yyyy".to_string())
        }

        /// Defines the `ShouldLoop` property.
        pub fn should_loop_property() -> StyledProperty<bool> {
            FerroProperty::register::<DateTimePickerPanel, _>("ShouldLoop", false)
        }
    }
}

impl DateTimePickerPanel {
    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<DateTimePickerPanel>(true);
        Panel::background_property()
            .override_default_value::<DateTimePickerPanel>(Some(Brushes::transparent() as Rc<dyn IBrush>));
        Layoutable::affects_measure::<DateTimePickerPanel>(&[Self::item_height_property().as_property()]);
        register_logical_scrollable::<DateTimePickerPanel>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            minimum_value: Cell::new(1),
            maximum_value: Cell::new(2),
            selected_value: Cell::new(1),
            increment: Cell::new(1),
            selected_index: Cell::new(0),
            total_items: Cell::new(0),
            num_items_above_below_selected: Cell::new(0),
            range: Cell::new(0),
            extent_one: Cell::new(0.0),
            extent: Cell::new(Size::default()),
            offset: Cell::new(Vector::default()),
            has_init: Cell::new(false),
            suppress_update_offset: Cell::new(false),
            parent_scroller: RefCell::new(None),
            scroll_gesture_ended_handler: Cell::new(None),
            format_date: Cell::new(DateTime::now()),
            scroll_invalidated: Rc::new(HandlerList::new()),
            selection_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// What this panel displays, in date or time units.
    pub fn panel_type(&self) -> DateTimePickerPanelType {
        self.get_value(Self::panel_type_property())
    }

    pub fn set_panel_type(&self, value: DateTimePickerPanelType) {
        self.set_value(Self::panel_type_property(), value)
    }

    /// The height of each item.
    pub fn item_height(&self) -> f64 {
        self.get_value(Self::item_height_property())
    }

    pub fn set_item_height(&self, value: f64) {
        self.set_value(Self::item_height_property(), value)
    }

    /// The string format for the items, using the standard date and time or
    /// time span formatting. The format must match the panel type.
    pub fn item_format(&self) -> String {
        self.get_value(Self::item_format_property())
    }

    pub fn set_item_format(&self, value: &str) {
        self.set_value(Self::item_format_property(), value.to_string())
    }

    /// Whether the panel should loop.
    pub fn should_loop(&self) -> bool {
        self.get_value(Self::should_loop_property())
    }

    pub fn set_should_loop(&self, value: bool) {
        self.set_value(Self::should_loop_property(), value)
    }

    /// The minimum value.
    pub fn minimum_value(&self) -> i32 {
        self.minimum_value.get()
    }

    /// Sets the minimum value. Panics when it is greater than the maximum.
    pub fn set_minimum_value(&self, value: i32) {
        if value > self.maximum_value() {
            panic!("Minimum cannot be greater than Maximum");
        }
        self.minimum_value.set(value);
        self.update_helper_info();
        let sel = self.coerce_selected(self.selected_value());
        if sel != self.selected_value() {
            self.set_selected_value(sel);
        }
        self.update_items();
        self.invalidate_arrange();
        self.raise_scroll_invalidated();
    }

    /// The maximum value.
    pub fn maximum_value(&self) -> i32 {
        self.maximum_value.get()
    }

    /// Sets the maximum value. Panics when it is less than the minimum.
    pub fn set_maximum_value(&self, value: i32) {
        if value < self.minimum_value() {
            panic!("Maximum cannot be less than Minimum");
        }
        self.maximum_value.set(value);
        self.update_helper_info();
        let sel = self.coerce_selected(self.selected_value());
        if sel != self.selected_value() {
            self.set_selected_value(sel);
        }
        self.update_items();
        self.invalidate_arrange();
        self.raise_scroll_invalidated();
    }

    /// The selected value.
    pub fn selected_value(&self) -> i32 {
        self.selected_value.get()
    }

    /// Sets the selected value. Panics when it is outside of the range of
    /// the minimum and maximum values.
    pub fn set_selected_value(&self, value: i32) {
        if value > self.maximum_value() || value < self.minimum_value() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'value')");
        }

        let sel = self.coerce_selected(value);
        self.selected_value.set(sel);
        self.selected_index.set((value - self.minimum_value()) / self.increment());

        if !self.should_loop() {
            self.create_or_destroy_items(&self.children());
        }

        if !self.suppress_update_offset.get() {
            let item_height = self.item_height();
            let selected_index = f64::from(self.selected_index.get());
            self.offset.set(Vector::new(
                0.0,
                if self.should_loop() {
                    selected_index * item_height + (self.extent_one.get() * 50.0)
                } else {
                    selected_index * item_height
                },
            ));
        }

        self.update_items();
        self.invalidate_arrange();
        self.raise_scroll_invalidated();

        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler();
        }
    }

    /// The increment.
    pub fn increment(&self) -> i32 {
        self.increment.get()
    }

    /// Sets the increment. Panics when it is not positive or greater than
    /// the range of the values.
    pub fn set_increment(&self, value: i32) {
        if value <= 0 || value > self.range.get() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'value')");
        }
        self.increment.set(value);
        self.update_helper_info();
        let sel = self.coerce_selected(self.selected_value());
        if sel != self.selected_value() {
            self.set_selected_value(sel);
        }
        self.update_items();
        self.invalidate_arrange();
        self.raise_scroll_invalidated();
    }

    /// Used to help format the date (if applicable): to display, for
    /// example, the day of week, the month and the year are needed as
    /// context; this is that context.
    pub(crate) fn format_date(&self) -> DateTime {
        self.format_date.get()
    }

    pub(crate) fn set_format_date(&self, value: DateTime) {
        self.format_date.set(value)
    }

    /// The current scroll offset.
    pub fn offset(&self) -> Vector {
        self.offset.get()
    }

    /// Sets the scroll offset: moves the items and selects the value at the
    /// offset.
    pub fn set_offset(&self, value: Vector) {
        let old = self.offset.get();
        self.offset.set(value);
        let dy = value.y - old.y;
        let children = self.children();

        if dy > 0.0 {
            // Scroll Down
            let snapshot = children.snapshot();
            let mut num_counts_to_move = 0;
            for child in snapshot.iter() {
                if child.bounds().bottom() - dy < 0.0 {
                    num_counts_to_move += 1;
                } else {
                    break;
                }
            }
            self.move_children(&children, 0, num_counts_to_move, snapshot.len().checked_sub(1));

            let scroll_height = self.extent.get().height - self.viewport().height;
            if self.should_loop() && value.y >= scroll_height - self.extent_one.get() {
                self.offset.set(Vector::new(0.0, value.y - (self.extent_one.get() * 50.0)));
            }
        } else if dy < 0.0 {
            // Scroll Up
            let snapshot = children.snapshot();
            let bounds_height = self.bounds().height;
            let mut num_counts_to_move = 0;
            for child in snapshot.iter().rev() {
                if child.bounds().y - dy > bounds_height {
                    num_counts_to_move += 1;
                } else {
                    break;
                }
            }
            self.move_children(&children, snapshot.len() - num_counts_to_move, num_counts_to_move, Some(0));
            if self.should_loop() && value.y < self.extent_one.get() {
                self.offset.set(Vector::new(0.0, value.y + (self.extent_one.get() * 50.0)));
            }
        }

        // Setting selection will handle all invalidation
        let new_sel = (self.offset.get().y / self.item_height()) % f64::from(self.total_items.get());
        self.suppress_update_offset.set(true);
        self.set_selected_value(new_sel as i32 * self.increment() + self.minimum_value());
        self.suppress_update_offset.set(false);
    }

    /// Moves a range of the children. A move of no items, or to the index
    /// the items already have, changes nothing in the list but is still a
    /// change of the children upstream, which invalidates the measure of the
    /// panel; `new_index` is `None` for the index before the first item
    /// (scrolling down a panel without items), where the move inserts
    /// upstream and fails.
    fn move_children(&self, children: &Controls, old_index: usize, count: usize, new_index: Option<usize>) {
        match new_index {
            None => panic!(
                "Index was out of range. Must be non-negative and less than or equal to the size of the collection. (Parameter 'index')"
            ),
            Some(new_index) if count != 0 && old_index != new_index => {
                children.move_range(old_index, count, new_index);
            }
            _ => self.invalidate_measure_on_children_changed(),
        }
    }

    pub fn can_horizontally_scroll(&self) -> bool {
        false
    }

    pub fn set_can_horizontally_scroll(&self, _value: bool) {}

    pub fn can_vertically_scroll(&self) -> bool {
        true
    }

    pub fn set_can_vertically_scroll(&self, _value: bool) {}

    pub fn is_logical_scroll_enabled(&self) -> bool {
        true
    }

    pub fn scroll_size(&self) -> Size {
        Size::new(0.0, self.item_height())
    }

    pub fn page_scroll_size(&self) -> Size {
        Size::new(0.0, self.item_height() * 4.0)
    }

    pub fn extent(&self) -> Size {
        self.extent.get()
    }

    pub fn viewport(&self) -> Size {
        self.bounds().size()
    }

    /// Raised when the selected value is set. Disposing the returned handle
    /// removes the handler.
    pub fn selection_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.selection_changed.remove(token);
            }
        })
    }

    /// Refreshes the content of the visible items.
    pub fn refresh_items(&self) {
        self.update_items();
    }

    /// Scrolls up the specified number of items.
    pub fn scroll_up(&self, num_items: i32) {
        let new_y = (self.offset.get().y - (f64::from(num_items) * self.item_height())).max(0.0);
        self.set_offset(Vector::new(0.0, new_y));
    }

    /// Scrolls down the specified number of items.
    pub fn scroll_down(&self, num_items: i32) {
        let scroll_height = (self.extent.get().height - self.item_height()).max(0.0);
        let new_y = (self.offset.get().y + (f64::from(num_items) * self.item_height())).min(scroll_height);
        self.set_offset(Vector::new(0.0, new_y));
    }

    /// Updates helper fields used in various calculations.
    fn update_helper_info(&self) {
        self.range.set(self.maximum_value.get() - self.minimum_value.get() + 1);
        self.total_items.set((f64::from(self.range.get()) / f64::from(self.increment.get())).ceil() as i32);

        let item_hgt = self.item_height();
        let should_loop = self.should_loop();
        let total_items = f64::from(self.total_items.get());
        // If looping, measure 100x as many items as we actually have
        self.extent.set(Size::new(
            0.0,
            if should_loop { total_items * item_hgt * 100.0 } else { total_items * item_hgt },
        ));

        // Height of 1 "set" of items
        self.extent_one.set(total_items * item_hgt);
        let selected_index = f64::from(self.selected_index.get());
        self.offset.set(Vector::new(
            0.0,
            if should_loop {
                self.extent_one.get() * 50.0 + selected_index * item_hgt
            } else {
                selected_index * item_hgt
            },
        ));
    }

    /// Ensures enough containers are visible in the viewport.
    fn create_or_destroy_items(&self, children: &Controls) {
        let num_items_above_below_selected = self.num_items_above_below_selected.get();
        let mut total_items_in_viewport = num_items_above_below_selected * 2 + 1;

        if !self.should_loop() {
            let selected_index = self.selected_index.get();
            let total_items = self.total_items.get();

            let mut num_item_above_select = num_items_above_below_selected;
            if selected_index - num_items_above_below_selected < 0 {
                num_item_above_select = selected_index;
            }
            let mut num_item_below_select = num_items_above_below_selected;
            if selected_index + num_items_above_below_selected >= total_items {
                num_item_below_select = total_items - selected_index - 1;
            }

            total_items_in_viewport = num_item_below_select + num_item_above_select + 1;
        }

        while (children.count() as i64) < i64::from(total_items_in_viewport) {
            let item = ListBoxItem::new();
            item.set_height(self.item_height());
            item.classes().add(self.panel_type().item_class());
            item.set_vertical_content_alignment(VerticalAlignment::Center);
            item.set_focusable(false);
            children.add(item);
        }
        if (children.count() as i64) > i64::from(total_items_in_viewport) {
            // A negative number of items leaves no item: the range of the
            // removal starts before the first one upstream, which fails.
            let total_items_in_viewport = usize::try_from(total_items_in_viewport)
                .unwrap_or_else(|_| panic!("Index was out of range. Must be non-negative."));
            let num_to_remove = children.count() - total_items_in_viewport;
            children.remove_range(children.count() - num_to_remove, num_to_remove);
        }
    }

    /// Updates item content based on the current selection and the panel
    /// type.
    fn update_items(&self) {
        let children = self.children().snapshot();
        let min = self.minimum_value();
        let panel_type = self.panel_type();
        let selected = self.selected_value();
        let max = self.maximum_value();
        let increment = self.increment();

        let mut first;
        if self.should_loop() {
            let total_items = self.total_items.get();
            first = (self.selected_index.get() - self.num_items_above_below_selected.get()) % total_items;
            first = if first < 0 { min + (first + total_items) * increment } else { min + first * increment };
        } else {
            first = min + (self.selected_index.get() - self.num_items_above_below_selected.get()).max(0) * increment;
        }

        for child in children.iter() {
            let item = child.cast::<ListBoxItem>().expect("the children of the panel are list box items");
            item.set_content(Some(Rc::new(self.format_content(first, panel_type)) as BoxedValue));
            item.set_tag(Some(Rc::new(first) as BoxedValue));
            item.set_is_selected(first == selected);
            first += increment;
            if first > max {
                first = min;
            }
        }
    }

    fn format_content(&self, value: i32, panel_type: DateTimePickerPanelType) -> String {
        match panel_type {
            DateTimePickerPanelType::Year => DateTime::new(value, 1, 1).to_string_with(&self.item_format()),
            DateTimePickerPanelType::Month => {
                DateTime::new(self.format_date().year(), value, 1).to_string_with(&self.item_format())
            }
            DateTimePickerPanelType::Day => {
                let format_date = self.format_date();
                DateTime::new(format_date.year(), format_date.month(), value).to_string_with(&self.item_format())
            }
            DateTimePickerPanelType::Hour => TimeSpan::from_hms(value, 0, 0).to_string_format(&self.item_format()),
            DateTimePickerPanelType::Minute => TimeSpan::from_hms(0, value, 0).to_string_format(&self.item_format()),
            DateTimePickerPanelType::Second => TimeSpan::from_hms(0, 0, value).to_string_format(&self.item_format()),
            DateTimePickerPanelType::TimePeriod => {
                if value == self.minimum_value() {
                    TimeUtils::get_am_designator()
                } else {
                    TimeUtils::get_pm_designator()
                }
            }
        }
    }

    /// Ensures the selected value is within the bounds and follows the
    /// current increment.
    fn coerce_selected(&self, new_value: i32) -> i32 {
        let minimum = self.minimum_value();
        let maximum = self.maximum_value();
        if new_value < minimum {
            return minimum;
        }
        if new_value > maximum {
            return maximum;
        }

        let increment = self.increment();
        if new_value % increment != 0 {
            // The candidates are the multiples of the increment among the
            // `maximum + 1` values that start at the minimum (the second
            // argument of the upstream range is a count), and the result is
            // the index of the nearest candidate times the increment.
            let count = i64::from(maximum) + 1;
            if count < 0 {
                panic!("Specified argument was out of the range of valid values. (Parameter 'count')");
            }
            let distance = |candidate: i64| (candidate - i64::from(new_value)).abs();
            let mut nearest: Option<(i32, i64)> = None;
            let candidates = (i64::from(minimum)..i64::from(minimum) + count).filter(|i| i % i64::from(increment) == 0);
            for (index, candidate) in candidates.enumerate() {
                nearest = match nearest {
                    Some((_, current)) if distance(current) <= distance(candidate) => nearest,
                    _ => Some((index as i32, candidate)),
                };
            }
            let (index, _) = nearest.unwrap_or_else(|| panic!("Sequence contains no elements"));
            return index * increment;
        }
        new_value
    }

    fn on_item_tapped(&self, e: &TappedEventArgs) {
        let Some(source) = e.source().and_then(|source| source.cast::<Visual>()) else {
            return;
        };
        let Some(list_box_item) = Self::get_item_from_source(source) else {
            return;
        };
        let Some(tag) = list_box_item.tag() else {
            return;
        };
        let tag: &dyn AnyValue = &*tag;
        if let Some(tag) = tag.downcast_ref::<i32>() {
            self.set_selected_value(*tag);
            e.set_handled(true);
        }
    }

    /// Helper to get the list box item from the source of a pointer event.
    fn get_item_from_source(src: Ref<Visual>) -> Option<Ref<ListBoxItem>> {
        let mut item = Some(src);
        while let Some(current) = item {
            if let Some(list_box_item) = current.cast::<ListBoxItem>() {
                return Some(list_box_item);
            }
            item = current.visual_parent();
        }
        None
    }

    pub fn bring_into_view(&self, _target: &Ref<Control>, _target_rect: Rect) -> bool {
        false
    }

    pub fn get_control_in_direction(
        &self,
        _direction: NavigationDirection,
        _from: Option<&Ref<Control>>,
    ) -> Option<Ref<Control>> {
        None
    }

    /// Raises the `ScrollInvalidated` event.
    pub fn raise_scroll_invalidated(&self) {
        for (_, handler) in self.scroll_invalidated.snapshot().iter() {
            handler();
        }
    }

    fn on_scroll_gesture_ended(&self, _e: &ScrollGestureEndedEventArgs) {
        let offset = self.offset.get();
        let item_height = self.item_height();
        let snap_y = (offset.y / item_height).round_ties_even() * item_height;

        if snap_y != offset.y {
            self.set_offset(Vector::new(offset.x, snap_y));
        }
    }
}
