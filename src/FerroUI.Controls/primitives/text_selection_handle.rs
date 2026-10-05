use super::{
    SelectionHandleType, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, Thumb, ThumbImpl,
    ThumbImplExt,
};
use crate::{Border, Canvas, ControlImpl, ControlImplExt, SizeChangedEventArgs};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, VectorEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs, RoutingStrategies};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref,
    StyledElementImpl, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_PRESSED: &str = ":pressed";
const PC_CARET: &str = ":caret";
const PC_START: &str = ":start";
const PC_END: &str = ":end";

/// A control that enables easy control over text selection using touch
/// based input.
#[repr(C)]
pub struct TextSelectionHandle {
    base: Thumb,
    selection_handle_type: Cell<SelectionHandleType>,
    is_rtl: Cell<bool>,
    is_dragging: Cell<bool>,
    needs_indicator_update: Cell<bool>,
    start_position: Cell<Point>,
    delta: Cell<Vector>,
    last_point: Cell<Option<Point>>,
    indicator: RefCell<Option<(Ref<Border>, Rc<dyn IDisposable>)>>,
    last_requested_position: Cell<Option<Point>>,
}

ferro_class!(TextSelectionHandle: Thumb);

ferro_class_info!(TextSelectionHandle {
    new: TextSelectionHandle::new,
    markup: {
        attributes: [TemplatePart("PART_Indicator", type(Ref<Border>))],
    },
});

ferro_impl_classes!(TextSelectionHandle: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl);

impl TemplatedControlImpl for TextSelectionHandle {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if let Some((_, layout_updated)) = this.indicator.take() {
            layout_updated.dispose();
        }

        let indicator = e.name_scope().get_as::<Border>("PART_Indicator");

        let weak = this.to_ref().downgrade();
        let layout_updated = indicator.layout_updated(move || {
            if let Some(this) = weak.upgrade() {
                this.indicator_layout_updated();
            }
        });
        *this.indicator.borrow_mut() = Some((indicator, layout_updated));

        this.update_handle_classes();
    }
}

impl ControlImpl for TextSelectionHandle {
    fn on_size_changed(this: &Self, e: &SizeChangedEventArgs) {
        Self::parent_on_size_changed(this, e);

        if let Some(last_requested_position) = this.last_requested_position.get() {
            this.set_top_left(last_requested_position);
            this.last_requested_position.set(None);
            this.needs_indicator_update.set(false);
        }
    }

    fn on_loaded(this: &Self, args: &RoutedEventArgs) {
        Self::parent_on_loaded(this, args);

        this.update_handle_classes();

        this.invalidate_visual();
    }
}

impl ThumbImpl for TextSelectionHandle {
    fn on_drag_started(this: &Self, e: &VectorEventArgs) {
        Self::parent_on_drag_started(this, e);

        this.start_position.set(this.get_top_left());
        this.delta.set(Vector::default());
        this.is_dragging.set(true);
    }

    fn on_drag_delta(this: &Self, e: &VectorEventArgs) {
        Self::parent_on_drag_delta(this, e);
        let new_delta = e.vector;

        if !e.handled() && (new_delta - this.delta.get()).length().abs() > 0.0 {
            this.delta.set(new_delta);
            let point = this.start_position.get() + new_delta;
            Canvas::set_top(this, point.y);
            Canvas::set_left(this, point.x);
        }
    }

    fn on_drag_completed(this: &Self, e: &VectorEventArgs) {
        this.is_dragging.set(false);
        this.start_position.set(Point::default());
        Self::parent_on_drag_completed(this, e);
    }
}

impl LayoutableImpl for TextSelectionHandle {
    fn arrange_core(this: &Self, final_rect: Rect) {
        this.update_handle_classes();

        Self::parent_arrange_core(this, final_rect);
    }
}

impl InputElementImpl for TextSelectionHandle {
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        if let Some(last_point) = this.last_point.get() {
            let mut ev = VectorEventArgs::new();
            ev.set_routed_event(Some(Thumb::drag_completed_event()));
            ev.vector = last_point.into();

            this.last_point.set(None);

            this.raise_event(&ev);
        }

        this.pseudo_classes().remove_pseudo(PC_PRESSED);

        Self::parent_on_pointer_capture_lost(this, e);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let this_element: Ref<InputElement> = this.to_ref().upcast();
        if e.pointer().captured() != Some(this_element.clone()) {
            return;
        }

        let root = this.visual_root();
        let mut ev = VectorEventArgs::new();

        match this.last_point.get() {
            None => {
                let last_point = e.get_position(root.as_deref());
                this.last_point.set(Some(last_point));
                e.pointer().capture(Some(&this_element));

                ev.set_routed_event(Some(Thumb::drag_started_event()));
                ev.vector = last_point.into();
            }
            Some(last_point) => {
                let vector = e.get_position(root.as_deref()) - last_point;

                ev.set_routed_event(Some(Thumb::drag_delta_event()));
                ev.vector = vector.into();
            }
        }

        this.raise_event(&ev);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        this.pseudo_classes().add_pseudo(PC_PRESSED);
        let this_element: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&this_element));
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.last_point.get().is_some() {
            e.set_handled(true);
            this.last_point.set(None);

            let mut ev = VectorEventArgs::new();
            ev.set_routed_event(Some(Thumb::drag_completed_event()));
            ev.vector = e.get_position(this.visual_root().as_deref()).into();

            this.raise_event(&ev);
        }

        this.pseudo_classes().remove_pseudo(PC_PRESSED);
        e.pointer().capture(None);
    }
}

impl TextSelectionHandle {
    fn static_constructor() {
        Thumb::drag_started_event().add_class_handler_with::<TextSelectionHandle>(
            |x, e| x.on_drag_started(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Thumb::drag_delta_event().add_class_handler_with::<TextSelectionHandle>(
            |x, e| x.on_drag_delta(e),
            RoutingStrategies::BUBBLE,
            false,
        );
        Thumb::drag_completed_event().add_class_handler_with::<TextSelectionHandle>(
            |x, e| x.on_drag_completed(e),
            RoutingStrategies::BUBBLE,
            false,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Thumb::construct(),
            selection_handle_type: Cell::new(SelectionHandleType::Caret),
            is_rtl: Cell::new(false),
            is_dragging: Cell::new(false),
            needs_indicator_update: Cell::new(false),
            start_position: Cell::new(Point::default()),
            delta: Cell::new(Vector::default()),
            last_point: Cell::new(None),
            indicator: RefCell::new(None),
            last_requested_position: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn selection_handle_type(&self) -> SelectionHandleType {
        self.selection_handle_type.get()
    }

    pub(crate) fn set_selection_handle_type(&self, value: SelectionHandleType) {
        self.selection_handle_type.set(value);
        self.update_handle_classes();
    }

    pub(crate) fn is_rtl(&self) -> bool {
        self.is_rtl.get()
    }

    pub(crate) fn set_is_rtl(&self, value: bool) {
        self.is_rtl.set(value);
        self.update_handle_classes();
    }

    pub(crate) fn indicator_position(&self) -> Point {
        let top_left = self.get_top_left();
        top_left.with_x(top_left.x + self.get_indicator_offset())
    }

    pub(crate) fn is_dragging(&self) -> bool {
        self.is_dragging.get()
    }

    // The getter of the upstream internal property; only the setter is used by the canvas.
    #[allow(dead_code)]
    pub(crate) fn needs_indicator_update(&self) -> bool {
        self.needs_indicator_update.get()
    }

    pub(crate) fn set_needs_indicator_update(&self, value: bool) {
        self.needs_indicator_update.set(value)
    }

    fn indicator(&self) -> Option<Ref<Border>> {
        self.indicator.borrow().as_ref().map(|(indicator, _)| indicator.clone())
    }

    fn indicator_layout_updated(&self) {
        if self.needs_indicator_update.get() {
            if let Some(last_requested_position) = self.last_requested_position.get() {
                self.set_top_left(last_requested_position);

                self.needs_indicator_update.set(!self.indicator().is_none_or(|indicator| indicator.is_arrange_valid()));
            }
        }
    }

    pub(crate) fn set_top_left(&self, point: Point) {
        if self.indicator.borrow().is_none() || self.needs_indicator_update.get() {
            self.last_requested_position.set(Some(point));
        }
        Canvas::set_top(self, point.y);
        Canvas::set_left(self, point.x - self.get_indicator_offset());
    }

    pub(crate) fn get_top_left(&self) -> Point {
        Point::new(Canvas::get_left(self), Canvas::get_top(self))
    }

    pub(crate) fn get_indicator_offset(&self) -> f64 {
        if let Some(indicator) = self.indicator() {
            return indicator.bounds().center().x;
        }

        match self.selection_handle_type.get() {
            SelectionHandleType::Caret => self.bounds().width / 2.0,
            SelectionHandleType::Start => {
                if self.is_rtl.get() {
                    0.0
                } else {
                    self.bounds().width
                }
            }
            SelectionHandleType::End => {
                if self.is_rtl.get() {
                    self.bounds().width
                } else {
                    0.0
                }
            }
        }
    }

    fn update_handle_classes(&self) {
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.remove_pseudo(PC_CARET);
        pseudo_classes.remove_pseudo(PC_START);
        pseudo_classes.remove_pseudo(PC_END);

        pseudo_classes.add_pseudo(match (self.selection_handle_type.get(), self.is_rtl.get()) {
            (SelectionHandleType::Caret, _) => PC_CARET,
            (SelectionHandleType::Start, false) | (SelectionHandleType::End, true) => PC_START,
            (SelectionHandleType::Start, true) | (SelectionHandleType::End, false) => PC_END,
        });
        self.invalidate_visual();
    }
}
