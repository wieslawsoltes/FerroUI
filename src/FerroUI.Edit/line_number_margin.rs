use crate::text_view_margin::MarginBinding;
use crate::{
    ITextViewConnect, ITextViewMargin, TextArea, TextDocument, TextEditor, TextViewMargin,
};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, PointerPressedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions};
use ferroui_base::media::{DrawingContext, FontFamily, FontStretch, FontStyle, FontWeight, IBrush, Typeface};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl,
    StyledProperty, Visual, VisualImpl, VisualImplExt, WeakRef,
};
use ferroui_controls::{Control, ControlImpl, TextBlock};
use std::cell::Cell;
use std::rc::Rc;

/// A native gutter that displays line numbers and selects the associated
/// editor's visual rows. Keep the handle returned by [`Self::as_margin`] when
/// assigning a view to a control hosted outside the editor.
#[repr(C)]
pub struct LineNumberMargin {
    base: Control,
    binding: MarginBinding,
    digit_advance: Cell<Option<f64>>,
    natural_width: Cell<Option<f64>>,
}

ferro_class!(LineNumberMargin: Control);
ferroui_base::ferro_class_info!(LineNumberMargin {
    new: LineNumberMargin::new
});
ferro_impl_classes!(LineNumberMargin: StyledElementImpl, InteractiveImpl, ControlImpl);

ferro_properties! { impl LineNumberMargin {
    pub fn min_width_in_digits_property() -> StyledProperty<u32> {
        FerroProperty::register::<Self, _>("MinWidthInDigits", 2)
    }
    pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        TextBlock::foreground_property().add_owner::<Self>()
    }
    pub fn font_family_property() -> StyledProperty<FontFamily> {
        TextEditor::font_family_property().add_owner::<Self>()
    }
    pub fn font_size_property() -> StyledProperty<f64> {
        TextEditor::font_size_property().add_owner::<Self>()
    }
    pub fn font_style_property() -> StyledProperty<FontStyle> {
        TextEditor::font_style_property().add_owner::<Self>()
    }
    pub fn font_weight_property() -> StyledProperty<FontWeight> {
        TextEditor::font_weight_property().add_owner::<Self>()
    }
    pub fn font_stretch_property() -> StyledProperty<FontStretch> {
        TextEditor::font_stretch_property().add_owner::<Self>()
    }
} }

impl LineNumberMargin {
    pub(crate) const RIGHT_PADDING: f64 = 3.0;
    const PADDING: f64 = 10.0 + Self::RIGHT_PADDING;

    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<Self>(true);
        Self::foreground_property().override_default_value::<Self>(Some(Rc::new(
            ImmutableSolidColorBrush::from_uint32(0xff858585),
        )));
        Visual::affects_render::<Self>(&[Self::foreground_property().as_property()]);
        Layoutable::affects_measure::<Self>(&[Self::min_width_in_digits_property().as_property()]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            binding: MarginBinding::default(),
            digit_advance: Cell::new(None),
            natural_width: Cell::new(None),
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
            .for_control(control.upcast(), || Rc::new(LineNumberMarginBehavior(weak)))
    }

    pub fn text_view(&self) -> Option<Ref<TextArea>> {
        self.binding.text_view()
    }

    pub fn min_width_in_digits(&self) -> u32 {
        self.get_value(Self::min_width_in_digits_property())
    }

    pub fn set_min_width_in_digits(&self, digits: u32) {
        self.set_value(Self::min_width_in_digits_property(), digits);
    }

    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, brush: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), brush);
    }

    pub(crate) fn preferred_width(&self, line_count: usize) -> f64 {
        let advance = self.digit_advance.get().unwrap_or_else(|| {
            let advance = self.number_layout("9", None).width().max(1.0);
            self.digit_advance.set(Some(advance));
            advance
        });
        let digits = (line_count.max(1).ilog10() + 1).max(self.min_width_in_digits());
        f64::from(digits) * advance + Self::PADDING
    }

    pub(crate) fn number_layout(
        &self,
        text: &str,
        foreground: Option<Rc<dyn IBrush>>,
    ) -> TextLayout {
        TextLayout::new(
            text,
            Typeface::with_style(
                self.get_value(Self::font_family_property()),
                self.get_value(Self::font_style_property()),
                self.get_value(Self::font_weight_property()),
                self.get_value(Self::font_stretch_property()),
            ),
            TextLayoutOptions {
                font_size: self.get_value(Self::font_size_property()),
                foreground,
                ..Default::default()
            },
        )
    }

    fn width_for_view(&self) -> f64 {
        let line_count = self
            .text_view()
            .and_then(|view| view.editor())
            .map_or(1, |editor| editor.document().line_count());
        self.preferred_width(line_count)
    }
}

impl FerroObjectImpl for LineNumberMargin {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if [
            Self::font_family_property().as_property(),
            Self::font_size_property().as_property(),
            Self::font_style_property().as_property(),
            Self::font_weight_property().as_property(),
            Self::font_stretch_property().as_property(),
        ].contains(&property) {
            this.digit_advance.set(None);
            this.invalidate_measure();
            this.invalidate_visual();
        }
    }
}

impl LayoutableImpl for LineNumberMargin {
    fn measure_override(this: &Self, _: Size) -> Size {
        let width = this.width_for_view();
        this.natural_width.set(Some(width));
        Size::new(width, 0.0)
    }
}

impl VisualImpl for LineNumberMargin {
    fn render(this: &Self, context: &mut DrawingContext) {
        Self::parent_render(this, context);
        if let Some(view) = this.text_view() {
            view.render_line_number_margin(this, context);
        }
    }
}

impl InputElementImpl for LineNumberMargin {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, event);
        if let Some(view) = this.text_view() {
            view.press_line_number_margin(event);
        }
    }
}

struct LineNumberMarginBehavior(WeakRef<LineNumberMargin>);

impl ITextViewConnect for LineNumberMarginBehavior {
    fn add_to_text_view(&self, _: &TextArea) {
        if let Some(control) = self.0.upgrade() {
            control.invalidate_measure();
        }
    }

    fn remove_from_text_view(&self, _: &TextArea) {
        if let Some(control) = self.0.upgrade() {
            control.invalidate_measure();
        }
    }
}

impl ITextViewMargin for LineNumberMarginBehavior {
    fn document_changed(&self, _: Option<&TextDocument>, _: Option<&TextDocument>) {
        if let Some(control) = self.0.upgrade() {
            control.invalidate_measure();
        }
    }

    fn viewport_changed(&self, _: &TextArea) {
        if let Some(control) = self.0.upgrade() {
            if control.natural_width.get() != Some(control.width_for_view()) {
                control.invalidate_measure();
            }
        }
    }
}
