use super::{TextElement, TextElementImpl};
use crate::Control;
use ferroui_base::media::text_formatting::{GenericTextRunProperties, TextRun, TextRunProperties};
use ferroui_base::media::{BaselineAlignment, IBrush, TextDecorationCollection, Typeface};
use ferroui_base::{
    ferro_class, ferro_property, AttachedProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use std::rc::Rc;

/// `Inline` is the base class for inline text elements.
///
/// This class is abstract.
#[repr(C)]
pub struct Inline {
    base: TextElement,
}

ferro_class! {
    Inline: TextElement, virtuals InlineImpl: TextElementImpl {
        /// Appends the text runs of the inline to `text_runs`.
        #[doc(hidden)]
        fn build_text_run(this, text_runs: &mut Vec<Rc<dyn TextRun>>);
        /// Measures the controls embedded in the inline for a block of
        /// `block_size`. Returns true when the size of one of them changed.
        #[doc(hidden)]
        fn measure_embedded_controls(this, block_size: Size) -> bool;
        /// Appends the text of the inline to `string_builder`.
        #[doc(hidden)]
        fn append_text(this, string_builder: &mut String);
    }
}

impl StyledElementImpl for Inline {}
impl TextElementImpl for Inline {}

impl FerroObjectImpl for Inline {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        match change.property().name() {
            "TextDecorations" | "BaselineAlignment" => {
                if let Some(host) = this.inline_host() {
                    host.invalidate();
                }
            }
            _ => {}
        }
    }
}

impl InlineImpl for Inline {
    fn build_text_run(_this: &Self, _text_runs: &mut Vec<Rc<dyn TextRun>>) {
        panic!("Inline is abstract.")
    }

    fn measure_embedded_controls(_this: &Self, _block_size: Size) -> bool {
        false
    }

    fn append_text(_this: &Self, _string_builder: &mut String) {
        panic!("Inline is abstract.")
    }
}

ferroui_base::ferro_properties! { impl Inline {
    ferro_property!(
        /// `AttachedProperty` for the `TextDecorations` property.
        pub fn text_decorations_property() -> AttachedProperty<Option<TextDecorationCollection>> {
            FerroProperty::register_attached_with::<Inline, Inline, _>(
                "TextDecorations",
                StyledPropertyOptions::new(None).inherits(true),
            )
        }
    );

    ferro_property!(
        /// `StyledProperty` for the `BaselineAlignment` property.
        pub fn baseline_alignment_property() -> StyledProperty<BaselineAlignment> {
            FerroProperty::register::<Inline, _>("BaselineAlignment", BaselineAlignment::Baseline)
        }
    );
} }

impl Inline {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TextElement::construct() }
    }

    /// The `TextDecorations` property specifies decorations that are added
    /// to the text of an element.
    pub fn text_decorations(&self) -> Option<TextDecorationCollection> {
        self.get_value(Self::text_decorations_property())
    }

    pub fn set_text_decorations(&self, value: Option<TextDecorationCollection>) {
        self.set_value(Self::text_decorations_property(), value)
    }

    /// Describes how the baseline for a text-based element is positioned on
    /// the vertical axis, relative to the established baseline for text.
    pub fn baseline_alignment(&self) -> BaselineAlignment {
        self.get_value(Self::baseline_alignment_property())
    }

    pub fn set_baseline_alignment(&self, value: BaselineAlignment) {
        self.set_value(Self::baseline_alignment_property(), value)
    }

    /// Gets the value of the attached `TextDecorations` property on a
    /// control.
    pub fn get_text_decorations(control: &Control) -> Option<TextDecorationCollection> {
        control.get_value(Self::text_decorations_property())
    }

    /// Sets the value of the attached `TextDecorations` property on a
    /// control.
    pub fn set_text_decorations_on(control: &Control, value: Option<TextDecorationCollection>) {
        control.set_value(Self::text_decorations_property(), value)
    }

    /// Creates the run properties of the text of the inline.
    pub fn create_text_run_properties(&self) -> Rc<dyn TextRunProperties> {
        let parent_or_self_background = self.background().or_else(|| self.find_parent_background());

        let typeface =
            Typeface::with_style(self.font_family(), self.font_style(), self.font_weight(), self.font_stretch());

        Rc::new(GenericTextRunProperties::with_all(
            typeface,
            self.font_size(),
            self.text_decorations(),
            self.foreground(),
            parent_or_self_background,
            self.baseline_alignment(),
            None,
            self.font_features(),
        ))
    }

    fn find_parent_background(&self) -> Option<Rc<dyn IBrush>> {
        let mut parent = self.parent();

        while let Some(inline) = parent.and_then(|parent| parent.cast::<Inline>()) {
            if let Some(background) = inline.background() {
                return Some(background);
            }

            parent = inline.parent();
        }

        None
    }
}
