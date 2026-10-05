use super::popup_positioning::{CustomPopupPlacement, CustomPopupPlacementCallback, PopupAnchor};
use super::{PopupFlyoutBase, SelectionHandleType, TextSelectionHandle};
use crate::platform::{FeedbackAction, PlatformFeedbackExtensions};
use crate::presenters::TextPresenter;
use crate::text_box_text_input_method_client::TextInputMethodChange;
use crate::{Canvas, CanvasImpl, Control, ControlImpl, FlyoutShowMode, PanelImpl, PlacementMode, TextBox};
use ferroui_base::input::{
    ContextRequestedEventArgs, FocusChangedEventArgs, InputElement, InputElementImpl, KeyEventArgs,
    PointerPressedEventArgs, PointerType, ScrollGestureEventArgs, TappedEventArgs, VectorEventArgs,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::text_formatting::ShapedTextRun;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Point, Ref, Size, StyledElementImpl, Thickness, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// Some platforms do not allow the handles to move over each other, that
/// is, to reverse the bounds of the selection.
const SHOULD_WRAP_AROUND_SELECTION: bool = !cfg!(target_os = "android");

const CONTEXT_MENU_PADDING: f64 = 16.0;

/// The handlers the canvas has on its presenter and on the text box of the
/// presenter.
struct PresenterSubscriptions {
    key_down: RoutedEventHandlerToken,
    tapped: RoutedEventHandlerToken,
    pointer_pressed: RoutedEventHandlerToken,
    got_focus: RoutedEventHandlerToken,
    text_box: Option<(Rc<dyn IDisposable>, RoutedEventHandlerToken)>,
}

/// The canvas of the touch selection handles of a text presenter. It lives
/// in the text selector layer of the top level.
#[repr(C)]
pub struct TextSelectionHandleCanvas {
    base: Canvas,
    caret_handle: Ref<TextSelectionHandle>,
    handle1: Ref<TextSelectionHandle>,
    handle2: Ref<TextSelectionHandle>,
    // The presenter owns the canvas: the canvas refers to it (and to its
    // text box) weakly.
    presenter: RefCell<Option<WeakRef<TextPresenter>>>,
    text_box: RefCell<Option<WeakRef<TextBox>>>,
    subscriptions: RefCell<Option<PresenterSubscriptions>>,
    show_handle: Cell<bool>,
    show_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    layout_listener: PresenterVisualListener,
    saved_selection_start: Cell<Option<i32>>,
    save_selection_end: Cell<Option<i32>>,
    initial_handle_type: Cell<Option<SelectionHandleType>>,
    is_in_touch_mode: Cell<bool>,
}

ferro_class!(TextSelectionHandleCanvas: Canvas);
ferro_class_info!(TextSelectionHandleCanvas { new: TextSelectionHandleCanvas::new });

ferro_impl_classes!(
    TextSelectionHandleCanvas: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl,
    CanvasImpl
);

impl FerroObjectImpl for TextSelectionHandleCanvas {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.caret_handle.set_selection_handle_type(SelectionHandleType::Caret);
        this.handle1.set_selection_handle_type(SelectionHandleType::Start);
        this.handle2.set_selection_handle_type(SelectionHandleType::End);

        this.children().add(this.caret_handle.clone());
        this.children().add(this.handle1.clone());
        this.children().add(this.handle2.clone());

        let weak = this.to_ref().downgrade();

        // The sender of the handlers of upstream is the handle a handler is
        // attached to.
        let drag_started = |handle: &Ref<TextSelectionHandle>| {
            let (weak, sender) = (weak.clone(), handle.downgrade());
            handle.drag_started(move |_, _| {
                if let (Some(this), Some(sender)) = (weak.upgrade(), sender.upgrade()) {
                    this.handle_drag_started(&sender);
                }
            });
        };
        let drag_completed = |handle: &Ref<TextSelectionHandle>| {
            let weak = weak.clone();
            handle.drag_completed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.handle_drag_completed();
                }
            });
        };
        let selection_drag_delta = |handle: &Ref<TextSelectionHandle>| {
            let (weak, sender) = (weak.clone(), handle.downgrade());
            handle.drag_delta(move |_, e| {
                if let (Some(this), Some(sender)) = (weak.upgrade(), sender.upgrade()) {
                    this.selection_handle_drag_delta(&sender, e);
                }
            });
        };

        drag_started(&this.caret_handle);
        {
            let weak = weak.clone();
            this.caret_handle.drag_delta(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.caret_handle_drag_delta();
                }
            });
        }
        drag_completed(&this.caret_handle);
        selection_drag_delta(&this.handle1);
        drag_completed(&this.handle1);
        drag_started(&this.handle1);
        selection_drag_delta(&this.handle2);
        drag_completed(&this.handle2);
        drag_started(&this.handle2);

        this.handle1.set_top_left(Point::default());
        this.caret_handle.set_top_left(Point::default());
        this.handle2.set_top_left(Point::default());

        for handle in [&this.handle1, &this.caret_handle, &this.handle2] {
            let weak = weak.clone();
            handle.add_handler(InputElement::context_canceled_event(), move |_, _: &RoutedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.caret_context_canceled();
                }
            });
        }
        for handle in [&this.handle1, &this.caret_handle, &this.handle2] {
            let weak = weak.clone();
            handle.add_handler(
                InputElement::context_requested_event(),
                move |_, e: &ContextRequestedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.caret_context_requested(e);
                    }
                },
            );
        }

        this.set_is_visible(this.show_handles());

        this.set_clip_to_bounds(false);

        this.layout_listener.set_invalidated(move || {
            if let Some(this) = weak.upgrade() {
                this.layout_listener_invalidated();
            }
        });
    }
}

impl LayoutableImpl for TextSelectionHandleCanvas {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        if this.show_handles() {
            this.move_handles_to_selection();
        }

        Self::parent_measure_override(this, available_size)
    }
}

impl TextSelectionHandleCanvas {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Canvas::construct(),
            caret_handle: TextSelectionHandle::new(),
            handle1: TextSelectionHandle::new(),
            handle2: TextSelectionHandle::new(),
            presenter: RefCell::new(None),
            text_box: RefCell::new(None),
            subscriptions: RefCell::new(None),
            show_handle: Cell::new(false),
            show_disposable: RefCell::new(None),
            layout_listener: PresenterVisualListener::new(),
            saved_selection_start: Cell::new(None),
            save_selection_end: Cell::new(None),
            initial_handle_type: Cell::new(None),
            is_in_touch_mode: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn presenter(&self) -> Option<Ref<TextPresenter>> {
        self.presenter.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn text_box(&self) -> Option<Ref<TextBox>> {
        self.text_box.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub(crate) fn show_handles(&self) -> bool {
        self.show_handle.get()
    }

    pub(crate) fn set_show_handles(&self, value: bool) {
        self.show_handle.set(value);

        if !value {
            self.handle1.set_is_visible(false);
            self.handle2.set_is_visible(false);
            self.caret_handle.set_is_visible(false);
        }

        let has_text = self.presenter().and_then(|presenter| presenter.text()).is_some_and(|text| !text.is_empty());
        self.set_is_visible(has_text && value);
    }

    fn layout_listener_invalidated(&self) {
        self.invalidate_measure();
    }

    fn caret_context_canceled(&self) {
        self.close_flyout();
    }

    fn caret_context_requested(&self, e: &ContextRequestedEventArgs) {
        self.show_flyout();
        e.set_handled(true);

        if e.is_holding() {
            let element: &InputElement = self;
            element.perform_feedback(FeedbackAction::hold());
        }
    }

    fn handle_drag_started(&self, sender: &TextSelectionHandle) {
        let text_box = self.text_box();
        self.saved_selection_start.set(text_box.as_ref().map(|text_box| text_box.selection_start()));
        self.save_selection_end.set(text_box.as_ref().map(|text_box| text_box.selection_end()));
        self.initial_handle_type.set(Some(sender.selection_handle_type()));
        self.close_flyout();
    }

    fn close_flyout(&self) {
        if let Some(text_box) = self.text_box() {
            text_box.raise_event(&RoutedEventArgs::with_event(InputElement::context_canceled_event()));
        }
    }

    fn selection_handle_drag_delta(&self, handle: &Ref<TextSelectionHandle>, e: &VectorEventArgs) {
        self.drag_selection_handle(handle);
        e.set_handled(true);
    }

    fn caret_handle_drag_delta(&self) {
        if let (Some(presenter), Some(text_box)) = (self.presenter(), self.text_box()) {
            let indicator_position = self.get_search_point(&self.caret_handle);
            let point = self.to_presenter(indicator_position);
            let _change = self.begin_change();
            presenter.move_caret_to_point(point);
            let caret_index = presenter.caret_index();
            text_box.set_current_value(TextBox::caret_index_property(), caret_index);
            text_box.set_current_value(TextBox::selection_start_property(), caret_index);
            text_box.set_current_value(TextBox::selection_end_property(), caret_index);

            let caret_bound = presenter.get_cursor_rectangle();
            self.caret_handle.set_top_left(self.to_layer(caret_bound.bottom_left()));
        }
    }

    pub fn hide(&self) {
        self.set_show_handles(false);
        self.is_in_touch_mode.set(false);
    }

    pub(crate) fn show(&self, force_touch_mode: bool) {
        self.is_in_touch_mode.set(self.is_in_touch_mode.get() || force_touch_mode);
        if self.is_in_touch_mode.get() {
            self.set_show_handles(true);
            self.move_handles_to_selection();

            self.check_state_and_show_flyout();
        }
    }

    fn begin_change(&self) -> Option<TextInputMethodChange> {
        self.presenter().and_then(|presenter| presenter.current_im_client()).map(|client| client.begin_change())
    }

    fn handle_drag_completed(&self) {
        self.saved_selection_start.set(Some(-1));
        self.save_selection_end.set(Some(-1));

        if !self.handle1.is_dragging() {
            self.handle1.set_needs_indicator_update(true);
            self.handle1.set_selection_handle_type(SelectionHandleType::Start);
        }

        if !self.handle2.is_dragging() {
            self.handle2.set_needs_indicator_update(true);
            self.handle2.set_selection_handle_type(SelectionHandleType::End);
        }

        self.move_handles_to_selection();
        self.check_state_and_show_flyout();
    }

    fn check_state_and_show_flyout(&self) {
        let text_box = self.text_box();
        let selection_start = text_box.as_ref().map(|text_box| text_box.selection_start());
        let selection_end = text_box.as_ref().map(|text_box| text_box.selection_end());

        if selection_start != selection_end && !(self.handle1.is_dragging() || self.handle2.is_dragging()) {
            // Show flyout if there's a selection
            self.show_flyout();
        }
    }

    fn ensure_visible(&self) {
        if let Some(show_disposable) = self.show_disposable.take() {
            show_disposable.dispose();
        }

        let Some(presenter) = self.presenter() else {
            return;
        };

        if presenter.visual_root().is_some_and(|root| root.is::<InputElement>()) {
            let Some(bounds) = presenter.get_transformed_bounds() else {
                return;
            };

            let padding = self.text_box().map_or(Thickness::symmetric(4.0, 0.0), |text_box| text_box.padding());
            let clip = bounds.clip.inflate_thickness(padding);
            let is_occluded = |point: Point| !clip.contains(point);

            let is_selection_dragging = self.handle1.is_dragging() || self.handle2.is_dragging();

            let has_selection = presenter.selection_start() != presenter.selection_end() || is_selection_dragging;

            let show_handles = self.show_handles();
            self.handle1.set_is_visible(
                show_handles
                    && has_selection
                    && (self.handle1.is_dragging() || !is_occluded(self.handle1.indicator_position())),
            );
            self.handle2.set_is_visible(
                show_handles
                    && has_selection
                    && (self.handle2.is_dragging() || !is_occluded(self.handle2.indicator_position())),
            );
            self.caret_handle.set_is_visible(
                show_handles
                    && !has_selection
                    && (self.caret_handle.is_dragging() || !is_occluded(self.caret_handle.indicator_position())),
            );

            if self.show_handles() && !has_selection {
                let weak = self.to_ref().downgrade();
                let show_disposable = DispatcherTimer::run_once(
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.set_show_handles(false);
                            let show_disposable = this.show_disposable.borrow().clone();
                            if let Some(show_disposable) = show_disposable {
                                show_disposable.dispose();
                            }
                        }
                    },
                    Duration::from_secs(5),
                    DispatcherPriority::BACKGROUND,
                );
                *self.show_disposable.borrow_mut() = Some(show_disposable);
            }
        }
    }

    fn get_search_point(&self, handle: &TextSelectionHandle) -> Point {
        let Some(presenter) = self.presenter() else {
            return Point::default();
        };

        let caret_bounds = presenter.get_cursor_rectangle();
        let search_offset = caret_bounds.height / 2.0;
        let indicator = handle.indicator_position();
        indicator.with_y(indicator.y - search_offset)
    }

    fn drag_selection_handle(&self, handle: &Ref<TextSelectionHandle>) {
        let (Some(presenter), Some(textbox)) = (self.presenter(), self.text_box()) else {
            return;
        };

        self.close_flyout();

        let mut position = {
            let indicator_position = self.get_search_point(handle);
            let point = self.to_presenter(indicator_position);
            let hit = presenter.text_layout().hit_test_point(point);
            let character_hit = hit.character_hit();
            character_hit.first_character_index() + character_hit.trailing_length()
        };

        let other_handle = if *handle == self.handle1 { &self.handle2 } else { &self.handle1 };

        let _change = self.begin_change();

        // Some platforms do not allow handles to cause selection bounds to
        // reverse, i.e. allow handles to move over each other.
        if !SHOULD_WRAP_AROUND_SELECTION {
            if handle.selection_handle_type() == SelectionHandleType::Start {
                position = if position >= textbox.selection_end() { textbox.selection_end() - 1 } else { position };
                textbox.set_current_value(TextBox::selection_start_property(), position);
            } else {
                position =
                    if position <= textbox.selection_start() { textbox.selection_start() + 1 } else { position };
                textbox.set_current_value(TextBox::selection_end_property(), position);
            }
        } else {
            // For platforms that do, update the handle types for each and
            // adjust selection.
            let last_position =
                || textbox.text().map_or(0, |text| text.encode_utf16().count() as i32 - 1);
            // The saved bounds are those of the start of the drag: there is
            // no drag delta without a drag start. An absent bound is not
            // assigned.
            let set = |property, value: Option<i32>| {
                if let Some(value) = value {
                    textbox.set_current_value(property, value);
                }
            };

            if self.initial_handle_type.get() == Some(SelectionHandleType::Start) {
                // If handle was previously the selection end, set new
                // selection end to the previous selection start.
                let other_position = self.save_selection_end.get();
                let has_position_swapped = other_position.is_some_and(|other| position > other);

                if Some(position) == other_position {
                    position = if has_position_swapped {
                        i32::min(position + 1, last_position())
                    } else {
                        i32::max(position - 1, 0)
                    };
                }

                set(
                    TextBox::selection_start_property(),
                    if has_position_swapped { other_position } else { Some(position) },
                );
                set(
                    TextBox::selection_end_property(),
                    if has_position_swapped { Some(position) } else { other_position },
                );

                other_handle.set_selection_handle_type(if has_position_swapped {
                    SelectionHandleType::Start
                } else {
                    SelectionHandleType::End
                });
            } else {
                // If handle was previously the selection start, set new
                // selection start to the previous selection end.
                let other_position = self.saved_selection_start.get();
                let has_position_swapped = other_position.is_some_and(|other| position < other);

                if Some(position) == other_position {
                    position = if !has_position_swapped {
                        i32::min(position + 1, last_position())
                    } else {
                        i32::max(position - 1, 0)
                    };
                }

                set(
                    TextBox::selection_start_property(),
                    if has_position_swapped { Some(position) } else { other_position },
                );
                set(
                    TextBox::selection_end_property(),
                    if has_position_swapped { other_position } else { Some(position) },
                );

                other_handle.set_selection_handle_type(if has_position_swapped {
                    SelectionHandleType::End
                } else {
                    SelectionHandleType::Start
                });
            }
        }

        presenter.move_caret_to_text_position(position, false);
        let caret_bound = presenter.get_cursor_rectangle();
        handle.set_top_left(self.to_layer(caret_bound.bottom_left()));

        self.move_handles_to_selection();
    }

    fn to_layer(&self, point: Point) -> Point {
        let Some(presenter) = self.presenter() else {
            return point;
        };
        match presenter.visual_root() {
            Some(root) => presenter.translate_point(point, &root).unwrap_or(point),
            None => point,
        }
    }

    fn to_presenter(&self, point: Point) -> Point {
        let Some(presenter) = self.presenter() else {
            return point;
        };
        presenter.visual_root().and_then(|root| root.translate_point(point, &presenter)).unwrap_or(point)
    }

    fn to_text_box(&self, point: Point) -> Point {
        let Some(text_box) = self.text_box() else {
            return point;
        };
        text_box.visual_root().and_then(|root| root.translate_point(point, &text_box)).unwrap_or(point)
    }

    pub fn move_handles_to_selection(&self) {
        let Some(presenter) = self.presenter() else {
            return;
        };

        let selection_start = presenter.selection_start();
        let selection_end = presenter.selection_end();
        let has_selection = selection_start != selection_end;

        if !self.caret_handle.is_dragging() {
            let points = presenter.get_caret_points();

            self.caret_handle.set_top_left(self.to_layer(points.1));
        }

        if has_selection {
            let start = i32::min(selection_start, selection_end);
            let end = i32::max(selection_start, selection_end);

            let start_point = Self::get_position(&presenter, start, true);
            let end_point = Self::get_position(&presenter, end - 1, false);

            for handle in [&self.handle1, &self.handle2] {
                if !handle.is_dragging() {
                    let (position, is_rtl) =
                        if handle.selection_handle_type() == SelectionHandleType::Start { start_point } else { end_point };
                    if is_rtl != handle.is_rtl() {
                        handle.set_needs_indicator_update(true);
                    }
                    handle.set_is_rtl(is_rtl);
                    handle.set_top_left(self.to_layer(position));
                }
            }
        }

        self.ensure_visible();
    }

    /// The position of a handle for the character at `index` and whether the
    /// run of the character is right to left.
    fn get_position(presenter: &TextPresenter, index: i32, start: bool) -> (Point, bool) {
        let text_layout = presenter.text_layout();
        let rect = text_layout.hit_test_text_range(index, 1).first().copied().unwrap_or_default();

        let line_index = text_layout.get_line_index_from_character_index(index, false);
        let text_line = &text_layout.text_lines()[line_index as usize];
        let line_start = text_line.first_text_source_index();
        let character_line_index = i32::max(0, index - line_start);
        let mut is_left_to_right = true;
        let mut search_length = 0;

        for run in text_line.text_runs().iter() {
            is_left_to_right = run
                .as_any()
                .downcast_ref::<ShapedTextRun>()
                .is_none_or(|shaped| shaped.shaped_buffer().is_left_to_right());

            search_length += run.length();
            if search_length > character_line_index {
                break;
            }
        }

        let is_rtl = !is_left_to_right;
        let mut reversed = is_rtl;
        if !start {
            reversed = !reversed;
        }

        (if reversed { rect.bottom_right() } else { rect.bottom_left() }, is_rtl)
    }

    pub(crate) fn set_presenter(&self, text_presenter: Option<&Ref<TextPresenter>>) {
        let current = self.presenter();
        if current.as_ref() == text_presenter && (current.is_some() || self.subscriptions.borrow().is_none()) {
            return;
        }

        // A presenter that is gone took its handlers with it; the listener
        // and the handlers of the text box are released all the same.
        if current.is_some() || self.subscriptions.borrow().is_some() {
            self.layout_listener.detach();
            let subscriptions = self.subscriptions.take();
            if let Some(subscriptions) = subscriptions {
                if let Some(presenter) = &current {
                    presenter.remove_handler(InputElement::key_down_event(), subscriptions.key_down);
                    presenter.remove_handler(InputElement::tapped_event(), subscriptions.tapped);
                    presenter.remove_handler(InputElement::pointer_pressed_event(), subscriptions.pointer_pressed);
                    presenter.remove_handler(InputElement::got_focus_event(), subscriptions.got_focus);
                }

                if let Some((property_changed, scroll_gesture)) = subscriptions.text_box {
                    property_changed.dispose();
                    if let Some(text_box) = self.text_box() {
                        text_box.remove_handler(InputElement::scroll_gesture_event(), scroll_gesture);
                    }
                }
            }
            *self.text_box.borrow_mut() = None;

            *self.presenter.borrow_mut() = None;
        }

        *self.presenter.borrow_mut() = text_presenter.map(Ref::downgrade);
        if let Some(presenter) = text_presenter {
            self.layout_listener.attach(presenter);
            let weak = self.to_ref().downgrade();

            let key_down = {
                let weak = weak.clone();
                presenter.add_handler_with(
                    InputElement::key_down_event(),
                    move |_, _: &KeyEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.presenter_key_down();
                        }
                    },
                    Interactive::DEFAULT_ROUTES,
                    true,
                )
            };
            let tapped = {
                let weak = weak.clone();
                presenter.add_handler(InputElement::tapped_event(), move |_, e: &TappedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.presenter_tapped(e);
                    }
                })
            };
            let pointer_pressed = {
                let weak = weak.clone();
                presenter.add_handler_with(
                    InputElement::pointer_pressed_event(),
                    move |_, e: &PointerPressedEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.presenter_pressed(e);
                        }
                    },
                    Interactive::DEFAULT_ROUTES,
                    true,
                )
            };
            let got_focus = {
                let weak = weak.clone();
                presenter.add_handler_with(
                    InputElement::got_focus_event(),
                    move |_, _: &FocusChangedEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.presenter_focused();
                        }
                    },
                    Interactive::DEFAULT_ROUTES,
                    true,
                )
            };

            let text_box = presenter.find_ancestor_of_type::<TextBox>(false);
            *self.text_box.borrow_mut() = text_box.as_ref().map(Ref::downgrade);

            let text_box = text_box.map(|text_box| {
                let property_changed = {
                    let weak = weak.clone();
                    text_box.property_changed(move |e| {
                        if let Some(this) = weak.upgrade() {
                            this.text_box_property_changed(e);
                        }
                    })
                };
                let scroll_gesture = text_box.add_handler_with(
                    InputElement::scroll_gesture_event(),
                    move |_, _: &ScrollGestureEventArgs| {
                        if let Some(this) = weak.upgrade() {
                            this.text_box_scrolling();
                        }
                    },
                    Interactive::DEFAULT_ROUTES,
                    true,
                );
                (property_changed, scroll_gesture)
            });

            *self.subscriptions.borrow_mut() =
                Some(PresenterSubscriptions { key_down, tapped, pointer_pressed, got_focus, text_box });
        }
    }

    fn text_box_scrolling(&self) {
        self.close_flyout();
    }

    fn presenter_pressed(&self, e: &PointerPressedEventArgs) {
        self.is_in_touch_mode.set(e.pointer().type_() != PointerType::Mouse);
    }

    fn presenter_focused(&self) {
        if self.presenter().is_some_and(|presenter| presenter.selection_start() != presenter.selection_end()) {
            self.set_show_handles(true);
            self.ensure_visible();
        }
    }

    fn presenter_tapped(&self, e: &TappedEventArgs) {
        self.is_in_touch_mode.set(e.pointer().type_() != PointerType::Mouse);

        if self.is_in_touch_mode.get() {
            self.move_handles_to_selection();
        } else {
            self.set_show_handles(false);
            if let Some(show_disposable) = self.show_disposable.take() {
                show_disposable.dispose();
            }
        }
    }

    pub(crate) fn show_flyout(&self) -> bool {
        let Some(text_box) = self.text_box() else {
            return false;
        };
        let Some(flyout) = text_box.context_flyout().and_then(|flyout| flyout.cast::<PopupFlyoutBase>()) else {
            return false;
        };

        let line_height = text_box.line_height();
        let vertical_offset =
            (if line_height.is_nan() { text_box.font_size() } else { line_height }) + CONTEXT_MENU_PADDING;

        let topleft = if text_box.selection_start() != text_box.selection_end() {
            if self.handle1.is_effectively_visible() && self.handle2.is_effectively_visible() {
                let p1 = self.handle1.indicator_position();
                let p2 = self.handle2.indicator_position();

                Some(Point::new((p1.x + p2.x) / 2.0, f64::min(p1.y, p2.y)))
            } else if self.handle1.is_effectively_visible() {
                Some(self.handle1.indicator_position())
            } else if self.handle2.is_effectively_visible() {
                Some(self.handle2.indicator_position())
            } else {
                None
            }
        } else if self.caret_handle.is_effectively_visible() {
            Some(self.caret_handle.indicator_position())
        } else {
            None
        };

        let Some(topleft) = topleft else {
            return false;
        };

        let old_placement = flyout.placement();
        let old_callback = flyout.custom_popup_placement_callback();
        let old_show_mode = flyout.show_mode();
        let point = self.to_text_box(topleft);
        let point = point.with_y(f64::max(0.0, point.y));

        let place: CustomPopupPlacementCallback = Rc::new(move |parameters: &mut CustomPopupPlacement| {
            parameters.set_anchor(PopupAnchor::TOP_LEFT);
            let offset = parameters.offset;
            parameters.offset = offset.with_x(point.x).with_y(point.y - vertical_offset);
        });
        flyout.set_custom_popup_placement_callback(Some(place));
        flyout.set_placement(PlacementMode::Custom);
        flyout.set_show_mode(FlyoutShowMode::Transient);

        text_box.raise_event(&ContextRequestedEventArgs::new());

        flyout.set_placement(old_placement);
        flyout.set_custom_popup_placement_callback(old_callback);
        flyout.set_show_mode(old_show_mode);

        true
    }

    fn text_box_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TextPresenter::text_property().as_property() {
            self.set_show_handles(false);
            self.close_flyout();
        } else if e.property() == TextBox::selection_start_property().as_property()
            || e.property() == TextBox::selection_end_property().as_property()
            || e.property() == TextBox::caret_index_property().as_property()
        {
            self.move_handles_to_selection();
        }
    }

    fn presenter_key_down(&self) {
        self.set_show_handles(false);
        self.is_in_touch_mode.set(false);
    }

    #[cfg(test)]
    pub(crate) fn handles(&self) -> [Ref<TextSelectionHandle>; 3] {
        [self.caret_handle.clone(), self.handle1.clone(), self.handle2.clone()]
    }

    #[cfg(test)]
    pub(crate) fn is_in_touch_mode(&self) -> bool {
        self.is_in_touch_mode.get()
    }
}

/// Listener to layout changes for presenter.
struct PresenterVisualListener {
    presenter: RefCell<Option<WeakRef<TextPresenter>>>,
    invalidated: RefCell<Option<Rc<dyn Fn()>>>,
    disposables: RefCell<Option<Vec<Rc<dyn IDisposable>>>>,
}

impl PresenterVisualListener {
    fn new() -> Self {
        Self { presenter: RefCell::new(None), invalidated: RefCell::new(None), disposables: RefCell::new(None) }
    }

    /// Sets the one handler of the `Invalidated` event.
    fn set_invalidated(&self, handler: impl Fn() + 'static) {
        *self.invalidated.borrow_mut() = Some(Rc::new(handler));
    }

    fn attach(&self, presenter: &Ref<TextPresenter>) {
        if self.presenter.borrow().is_some() {
            panic!("Listener is already attached to a TextPresenter");
        }

        self.dispose_all();
        *self.presenter.borrow_mut() = Some(presenter.downgrade());

        let mut disposables = Vec::new();
        let mut current: Option<Ref<Visual>> = Some(presenter.clone().upcast());
        while let Some(visual) = current {
            self.attach_events(&visual, &mut disposables);
            current = visual.visual_parent();
        }
        *self.disposables.borrow_mut() = Some(disposables);
    }

    fn attach_events(&self, visual: &Ref<Visual>, disposables: &mut Vec<Rc<dyn IDisposable>>) {
        let invalidated = self.invalidated.borrow().clone();
        let on_invalidated = move || {
            if let Some(invalidated) = &invalidated {
                invalidated();
            }
        };

        if let Some(layoutable) = visual.cast::<Layoutable>() {
            let on_invalidated = on_invalidated.clone();
            disposables.push(layoutable.effective_viewport_changed(move |_| on_invalidated()));
        }
        if let Some(control) = visual.cast::<Control>() {
            let token = control.size_changed(move |_, _| on_invalidated());
            let control = control.downgrade();
            disposables.push(Disposable::create(move || {
                if let Some(control) = control.upgrade() {
                    control.remove_handler(Control::size_changed_event(), token);
                }
            }));
        }
    }

    fn dispose_all(&self) {
        let disposables = self.disposables.take();
        for disposable in disposables.into_iter().flatten() {
            disposable.dispose();
        }
    }

    fn detach(&self) {
        self.dispose_all();

        *self.presenter.borrow_mut() = None;
    }
}
