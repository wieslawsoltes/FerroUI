use crate::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, TemplateAppliedEventArgs, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{ControlImpl, ItemsControl, ItemsControlImpl, Panel, SelectionMode, VirtualizingCarouselPanel};
use ferroui_base::animation::{IPageTransition, PageSlide, SlideAxis};
use ferroui_base::input::{InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, Vector,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An items control that displays its items as pages and can reveal adjacent
/// pages using `viewport_fraction`.
#[repr(C)]
pub struct Carousel {
    base: SelectingItemsControl,
    /// The template part named `PART_ScrollViewer`: an input element that
    /// is a scrollable.
    scroller: RefCell<Option<Ref<InputElement>>>,
    is_swiping: Cell<bool>,
}

ferro_class!(Carousel: SelectingItemsControl);
ferroui_base::ferro_class_info!(Carousel { new: Carousel::new });
ferro_impl_classes!(
    Carousel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    ControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for Carousel {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == SelectingItemsControl::selected_index_property().as_property() {
            this.sync_scroll_offset();
        }

        if change.property() == Self::is_swipe_enabled_property().as_property()
            || change.property() == Self::page_transition_property().as_property()
            || change.property() == Self::viewport_fraction_property().as_property()
            || change.property() == SelectingItemsControl::wrap_selection_property().as_property()
        {
            if let Some(panel) = this.carousel_panel() {
                if change.property() == Self::viewport_fraction_property().as_property()
                    && !panel.is_managing_interaction_offset()
                {
                    panel.sync_selection_offset(this.selected_index());
                }

                panel.refresh_gesture_recognizer();
                panel.invalidate_measure();
            }

            this.sync_scroll_offset();
        }
    }
}

impl InputElementImpl for Carousel {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        if e.handled() || this.item_count() == 0 {
            return;
        }

        let axis = if this.viewport_fraction() != 1.0 {
            Some(this.get_layout_axis())
        } else {
            this.get_transition_axis()
        };
        let is_vertical = axis == Some(SlideAxis::Vertical);
        let is_horizontal = axis == Some(SlideAxis::Horizontal);

        match e.key {
            Key::Left if !is_vertical => {
                this.previous();
                e.set_handled(true);
            }
            Key::Up if !is_horizontal => {
                this.previous();
                e.set_handled(true);
            }
            Key::Right if !is_vertical => {
                this.next();
                e.set_handled(true);
            }
            Key::Down if !is_horizontal => {
                this.next();
                e.set_handled(true);
            }
            Key::Home => {
                this.set_selected_index(0);
                e.set_handled(true);
            }
            Key::End => {
                this.set_selected_index(this.item_count() - 1);
                e.set_handled(true);
            }
            _ => {}
        }
    }
}

impl LayoutableImpl for Carousel {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);

        this.sync_scroll_offset();

        result
    }
}

impl TemplatedControlImpl for Carousel {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let scroller = e.name_scope().find("PART_ScrollViewer").map(|found| {
            match found.cast::<InputElement>().filter(|element| element.as_scrollable().is_some()) {
                Some(scroller) => scroller,
                None => panic!(
                    "Expected control 'PART_ScrollViewer' to be 'IScrollable' but it was '{}'.",
                    found.get_type().name()
                ),
            }
        });
        *this.scroller.borrow_mut() = scroller;

        if let Some(panel) = this.carousel_panel() {
            panel.refresh_gesture_recognizer();
        }
    }
}

ferroui_base::ferro_properties! { impl Carousel {
    ferro_property!(
        /// Defines the `PageTransition` property.
        pub fn page_transition_property() -> StyledProperty<Option<Rc<dyn IPageTransition>>> {
            FerroProperty::register::<Carousel, _>("PageTransition", None)
        }
    );

    ferro_property!(
        /// Defines the `IsSwipeEnabled` property.
        pub fn is_swipe_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Carousel, _>("IsSwipeEnabled", false)
        }
    );

    ferro_property!(
        /// Defines the `ViewportFraction` property.
        pub fn viewport_fraction_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Carousel, _>(
                "ViewportFraction",
                StyledPropertyOptions::new(1.0)
                    .coerce(|_, value: f64| if value.is_finite() && value > 0.0 { value } else { 1.0 }),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsSwiping` property.
        pub fn is_swiping_property() -> DirectProperty<Carousel, bool> {
            FerroProperty::register_direct::<Carousel, _>("IsSwiping", |o| o.is_swiping(), None, false)
        }
    );
} }

impl Carousel {
    /// Initializes static members of the [`Carousel`] class.
    fn static_constructor() {
        // The default value of the `ItemsPanel` property for a carousel.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(VirtualizingCarouselPanel::new().upcast::<Panel>()));

        SelectingItemsControl::selection_mode_property()
            .override_default_value::<Carousel>(SelectionMode::ALWAYS_SELECTED);
        ItemsControl::items_panel_property().override_default_value::<Carousel>(default_panel);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: SelectingItemsControl::construct(), scroller: RefCell::new(None), is_swiping: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the transition to use when moving between pages.
    pub fn page_transition(&self) -> Option<Rc<dyn IPageTransition>> {
        self.get_value(Self::page_transition_property())
    }

    pub fn set_page_transition(&self, value: Option<Rc<dyn IPageTransition>>) {
        self.set_value(Self::page_transition_property(), value)
    }

    /// Gets or sets whether swipe gestures are enabled for navigating
    /// between pages. When enabled, mouse pointer events are also accepted
    /// in addition to touch and pen.
    pub fn is_swipe_enabled(&self) -> bool {
        self.get_value(Self::is_swipe_enabled_property())
    }

    pub fn set_is_swipe_enabled(&self, value: bool) {
        self.set_value(Self::is_swipe_enabled_property(), value)
    }

    /// Gets or sets the fraction of the viewport occupied by each page. A
    /// value of 1 shows a single full page; values below 1 reveal adjacent
    /// pages.
    pub fn viewport_fraction(&self) -> f64 {
        self.get_value(Self::viewport_fraction_property())
    }

    pub fn set_viewport_fraction(&self, value: f64) {
        self.set_value(Self::viewport_fraction_property(), value)
    }

    /// Gets a value indicating whether a swipe gesture is currently in
    /// progress.
    pub fn is_swiping(&self) -> bool {
        self.is_swiping.get()
    }

    pub(crate) fn set_is_swiping(&self, value: bool) {
        self.set_and_raise_cell(Self::is_swiping_property(), &self.is_swiping, value);
    }

    /// Moves to the next item in the carousel.
    pub fn next(&self) {
        if self.item_count() == 0 {
            return;
        }

        if self.selected_index() < self.item_count() - 1 {
            self.set_selected_index(self.selected_index() + 1);
        } else if self.wrap_selection() {
            self.set_selected_index(0);
        }
    }

    /// Moves to the previous item in the carousel.
    pub fn previous(&self) {
        if self.item_count() == 0 {
            return;
        }

        if self.selected_index() > 0 {
            self.set_selected_index(self.selected_index() - 1);
        } else if self.wrap_selection() {
            self.set_selected_index(self.item_count() - 1);
        }
    }

    pub(crate) fn get_transition_axis(&self) -> Option<SlideAxis> {
        let transition = self.page_transition()?;

        if let Some(composite) = transition.as_composite_page_transition() {
            for t in composite.page_transitions() {
                if let Some(slide) = t.as_page_slide() {
                    return Some(slide.orientation());
                }
            }

            return None;
        }

        transition.as_page_slide().map(PageSlide::orientation)
    }

    pub(crate) fn get_layout_axis(&self) -> SlideAxis {
        self.get_transition_axis().unwrap_or(SlideAxis::Horizontal)
    }

    /// The items panel, when it is a carousel panel.
    fn carousel_panel(&self) -> Option<Ref<VirtualizingCarouselPanel>> {
        self.items_panel_root().and_then(|panel| panel.cast::<VirtualizingCarouselPanel>())
    }

    fn sync_scroll_offset(&self) {
        if let Some(panel) = self.carousel_panel() {
            if panel.is_managing_interaction_offset() {
                return;
            }

            panel.sync_selection_offset(self.selected_index());

            if self.viewport_fraction() != 1.0 {
                return;
            }
        }

        let scroller = self.scroller.borrow().clone();
        let Some(scroller) = scroller else {
            return;
        };

        if let Some(scrollable) = scroller.as_scrollable() {
            scrollable.set_offset(self.create_scroll_offset(self.selected_index()));
        }
    }

    fn create_scroll_offset(&self, index: i32) -> Vector {
        if self.viewport_fraction() != 1.0 && self.get_layout_axis() == SlideAxis::Vertical {
            return Vector::new(0.0, index as f64);
        }

        Vector::new(index as f64, 0.0)
    }
}
