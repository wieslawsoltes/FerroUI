//! WinUI's `FlipView` (`FlipView_Partial.cpp`) on `Carousel`: the navigation buttons and when
//! they show. Paging, keyboard and swipes are those of `Carousel`.

use ferroui_base::animation::{IPageTransition, SlideAxis};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, PointerEventArgs, PointerType};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs,
    Ref, StyledElementImpl, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, TemplateAppliedEventArgs, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use ferroui_controls::{Button, Carousel, ControlImpl, ItemsControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// `FLIP_VIEW_BUTTONS_SHOW_DURATION_MS`: how long the buttons stay after the pointer last moved.
const BUTTONS_SHOW_DURATION: Duration = Duration::from_millis(3000);

/// The four buttons of the template, with the handlers of their clicks.
#[derive(Default)]
struct NavigationButtons {
    previous_horizontal: Option<(Ref<Button>, RoutedEventHandlerToken)>,
    next_horizontal: Option<(Ref<Button>, RoutedEventHandlerToken)>,
    previous_vertical: Option<(Ref<Button>, RoutedEventHandlerToken)>,
    next_vertical: Option<(Ref<Button>, RoutedEventHandlerToken)>,
}

impl NavigationButtons {
    fn all(&self) -> [&Option<(Ref<Button>, RoutedEventHandlerToken)>; 4] {
        [&self.previous_horizontal, &self.next_horizontal, &self.previous_vertical, &self.next_vertical]
    }
}

/// Shows one item at a time, with buttons to the previous and the next one.
#[repr(C)]
pub struct FlipView {
    base: Carousel,
    buttons: RefCell<NavigationButtons>,
    buttons_shown: Cell<bool>,
    fade_out_timer: RefCell<Option<Rc<DispatcherTimer>>>,
}

ferro_class!(FlipView: Carousel);
ferroui_base::ferro_class_info!(FlipView { new: FlipView::new });
ferro_impl_classes!(
    FlipView: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl
);

impl FerroObjectImpl for FlipView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == SelectingItemsControl::selected_index_property().as_property()
            || property == Carousel::page_transition_property().as_property()
            || property == SelectingItemsControl::wrap_selection_property().as_property()
        {
            this.update_buttons();
        }
    }
}

impl VisualImpl for FlipView {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.hide_buttons();
    }
}

impl InputElementImpl for FlipView {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        // As WinUI: touch pages by swiping and does not bring the buttons up.
        if e.pointer().pointer_type() == PointerType::Touch {
            this.hide_buttons();
        } else {
            this.show_buttons();
        }
    }
}

impl TemplatedControlImpl for FlipView {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let old = this.buttons.take();
        for (button, token) in old.all().into_iter().flatten() {
            button.remove_handler(Button::click_event(), *token);
        }

        let wire = |name: &str, forward: bool| {
            let button = e.name_scope().find_as::<Button>(name)?;
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    if forward {
                        this.next();
                    } else {
                        this.previous();
                    }
                    // A click is pointer activity: the buttons stay for their full duration.
                    this.show_buttons();
                }
            });
            Some((button, token))
        };
        *this.buttons.borrow_mut() = NavigationButtons {
            previous_horizontal: wire("PreviousButtonHorizontal", false),
            next_horizontal: wire("NextButtonHorizontal", true),
            previous_vertical: wire("PreviousButtonVertical", false),
            next_vertical: wire("NextButtonVertical", true),
        };
        this.update_buttons();
    }
}

impl FlipView {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Carousel::construct(),
            buttons: RefCell::new(NavigationButtons::default()),
            buttons_shown: Cell::new(false),
            fade_out_timer: RefCell::new(None),
        }
    }

    /// Creates an empty flip view.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the items page vertically, from the axis of the page transition.
    fn is_vertical(&self) -> bool {
        let Some(transition) = self.page_transition() else { return false };
        if let Some(slide) = transition.as_page_slide() {
            return slide.orientation() == SlideAxis::Vertical;
        }
        transition.as_composite_page_transition().is_some_and(|composite| {
            composite
                .page_transitions()
                .iter()
                .find_map(|transition| transition.as_page_slide())
                .is_some_and(|slide| slide.orientation() == SlideAxis::Vertical)
        })
    }

    /// Shows the buttons and restarts the time they stay.
    pub(crate) fn show_buttons(&self) {
        self.buttons_shown.set(true);
        self.update_buttons();
        let timer = self.fade_out_timer.borrow_mut().get_or_insert_with(|| self.create_timer()).clone();
        timer.stop();
        timer.start();
    }

    fn create_timer(&self) -> Rc<DispatcherTimer> {
        let timer = DispatcherTimer::new();
        timer.set_interval(BUTTONS_SHOW_DURATION);
        let weak = self.to_ref().downgrade();
        let subscription = timer.tick(move |timer| {
            let Some(this) = weak.upgrade() else {
                timer.stop();
                return;
            };
            // The buttons stay while the pointer is on one of them.
            if !this.is_pointer_over_button() {
                this.hide_buttons();
            }
        });
        // The subscription lasts as long as the timer, which the control owns.
        std::mem::forget(subscription);
        timer
    }

    pub(crate) fn hide_buttons(&self) {
        if let Some(timer) = self.fade_out_timer.borrow().as_ref() {
            timer.stop();
        }
        self.buttons_shown.set(false);
        self.update_buttons();
    }

    fn is_pointer_over_button(&self) -> bool {
        self.buttons
            .borrow()
            .all()
            .into_iter()
            .flatten()
            .any(|(button, _)| button.is_visible() && button.is_pointer_over())
    }

    /// Shows the buttons of the axis of the items, without the one at the edge they cannot go
    /// past. A selection that wraps has no edge.
    fn update_buttons(&self) {
        let shown = self.buttons_shown.get() && self.item_count() > 1;
        let vertical = self.is_vertical();
        let wraps = self.wrap_selection();
        let index = self.selected_index();
        let has_previous = shown && (wraps || index > 0);
        let has_next = shown && (wraps || index < self.item_count() - 1);
        let buttons = self.buttons.borrow();
        let set = |button: &Option<(Ref<Button>, RoutedEventHandlerToken)>, visible: bool| {
            if let Some((button, _)) = button {
                button.set_is_visible(visible);
            }
        };
        set(&buttons.previous_horizontal, has_previous && !vertical);
        set(&buttons.next_horizontal, has_next && !vertical);
        set(&buttons.previous_vertical, has_previous && vertical);
        set(&buttons.next_vertical, has_next && vertical);
    }
}
