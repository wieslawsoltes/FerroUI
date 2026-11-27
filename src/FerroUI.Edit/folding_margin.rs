use crate::text_view_margin::MarginBinding;
use crate::{
    ITextViewConnect, ITextViewMargin, TextArea, TextDocument, TextEditor, TextViewMargin,
};
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, PointerEventArgs, PointerPressedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::{DrawingContext, IBrush};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, Point, Rect,
    Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl, WeakRef,
};
use ferroui_controls::{Control, ControlImpl, TopLevel};
use ferroui_edit_core::folding::{FoldingManager, FoldingSubscription};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const POINT_TO_DIP: f64 = 4.0 / 3.0;

/// A native gutter that displays and toggles folds in an associated text view.
/// Keep the handle returned by [`Self::as_margin`] when assigning an explicit
/// view to a control hosted outside the editor.
#[repr(C)]
pub struct FoldingMargin {
    base: Control,
    binding: MarginBinding,
    manager: RefCell<Option<(Rc<FoldingManager>, FoldingSubscription)>>,
    pointer: Cell<Option<Point>>,
}

ferro_class!(FoldingMargin: Control);
ferroui_base::ferro_class_info!(FoldingMargin {
    new: FoldingMargin::new
});
ferro_impl_classes!(FoldingMargin: FerroObjectImpl, StyledElementImpl, InteractiveImpl, ControlImpl);

ferro_properties! { impl FoldingMargin {
    pub fn font_size_property() -> StyledProperty<f64> {
        TextEditor::font_size_property().add_owner::<Self>()
    }
    pub fn folding_marker_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextEditor::folding_marker_brush_property().add_owner::<Self>()
    }
    pub fn folding_marker_background_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextEditor::folding_marker_background_brush_property().add_owner::<Self>()
    }
    pub fn selected_folding_marker_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextEditor::selected_folding_marker_brush_property().add_owner::<Self>()
    }
    pub fn selected_folding_marker_background_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextEditor::selected_folding_marker_background_brush_property().add_owner::<Self>()
    }
} }

impl FoldingMargin {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<Self>(true);
        Layoutable::affects_measure::<Self>(&[Self::font_size_property().as_property()]);
        Visual::affects_render::<Self>(&[
            Self::folding_marker_brush_property().as_property(),
            Self::folding_marker_background_brush_property().as_property(),
            Self::selected_folding_marker_brush_property().as_property(),
            Self::selected_folding_marker_background_brush_property().as_property(),
        ]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            binding: MarginBinding::default(),
            manager: RefCell::new(None),
            pointer: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Returns the shared view binding for this control.
    pub fn as_margin(&self) -> TextViewMargin {
        let control = self.to_ref();
        let weak = control.downgrade();
        self.binding
            .for_control(control.upcast(), || Rc::new(FoldingMarginBehavior(weak)))
    }

    pub fn text_view(&self) -> Option<Ref<TextArea>> {
        self.binding.text_view()
    }

    /// Sets a manager for this margin. A supplied manager must use the view's
    /// current document. Clearing it follows the editor's installed manager.
    pub fn set_folding_manager(&self, manager: Option<Rc<FoldingManager>>) {
        self.manager.replace(manager.map(|manager| {
            let weak = self.to_ref().downgrade();
            let subscription = manager.changed(move || {
                if let Some(control) = weak.upgrade() {
                    control.invalidate_visual();
                }
            });
            (manager, subscription)
        }));
        self.update_pointer(None);
        self.invalidate_visual();
    }

    pub fn folding_manager(&self) -> Option<Rc<FoldingManager>> {
        let editor = self.text_view()?.editor()?;
        self.manager_for_editor(&editor)
    }

    pub(crate) fn manager_for_editor(&self, editor: &TextEditor) -> Option<Rc<FoldingManager>> {
        if let Some((manager, _)) = self.manager.borrow().as_ref() {
            return (manager.document() == &editor.document()).then(|| manager.clone());
        }
        editor.folding_manager()
    }

    pub(crate) fn pixel_size(&self) -> f64 {
        TopLevel::get_top_level(Some(self)).map_or(1.0, |top| 1.0 / top.render_scaling())
    }

    pub(crate) fn preferred_width(&self) -> f64 {
        let size = self.get_value(Self::font_size_property()) * POINT_TO_DIP;
        round_to_odd_pixel(size, self.pixel_size())
    }

    pub(crate) fn local_bounds(&self) -> Rect {
        Rect::from_size(self.bounds().size())
    }

    pub(crate) fn pointer(&self) -> Option<Point> {
        self.pointer.get()
    }

    pub(crate) fn update_pointer(&self, point: Option<Point>) {
        let point = point.filter(|&point| self.local_bounds().contains_exclusive(point));
        if self.pointer.replace(point) != point {
            self.invalidate_visual();
        }
    }
}

pub(crate) fn round_to_odd_pixel(value: f64, pixel: f64) -> f64 {
    ((value - pixel) / (2.0 * pixel)).round() * 2.0 * pixel + pixel
}

impl LayoutableImpl for FoldingMargin {
    fn measure_override(this: &Self, _: Size) -> Size {
        Size::new(this.preferred_width(), 0.0)
    }
}

impl VisualImpl for FoldingMargin {
    fn render(this: &Self, context: &mut DrawingContext) {
        if let Some(view) = this.text_view() {
            view.render_folding_margin(this, context);
        }
    }
}

impl InputElementImpl for FoldingMargin {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, event);
        if let Some(view) = this.text_view() {
            view.press_folding_margin(this, event, event.get_position(Some(this)));
        }
    }

    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, event);
        if !event.handled() {
            this.update_pointer(Some(event.get_position(Some(this))));
        }
    }

    fn on_pointer_exited(this: &Self, event: &PointerEventArgs) {
        this.update_pointer(None);
        Self::parent_on_pointer_exited(this, event);
    }
}

struct FoldingMarginBehavior(WeakRef<FoldingMargin>);

impl ITextViewConnect for FoldingMarginBehavior {
    fn remove_from_text_view(&self, _: &TextArea) {
        if let Some(control) = self.0.upgrade() {
            control.update_pointer(None);
        }
    }
}

impl ITextViewMargin for FoldingMarginBehavior {
    fn document_changed(&self, _: Option<&TextDocument>, _: Option<&TextDocument>) {
        if let Some(control) = self.0.upgrade() {
            control.update_pointer(None);
            control.invalidate_visual();
        }
    }
}
