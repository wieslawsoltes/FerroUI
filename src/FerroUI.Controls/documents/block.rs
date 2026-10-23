use super::{TextElement, TextElementImpl};
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::media::{FlowDirection, TextAlignment};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Size,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, ferro_class,
};
use std::rc::Rc;

/// Determines how the height of a line is chosen when a paragraph specifies a line height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum LineStackingStrategy {
    /// A line accommodates its tallest run and is at least the requested height.
    #[default]
    MaxHeight = 0,
    /// Each line occupies the requested height, independently of its runs.
    BlockLineHeight = 1,
    /// The distance from one baseline to the next is the requested height.
    BaselineToBaseline = 2,
}

ferroui_base::ferro_markup_enum!(LineStackingStrategy {
    MaxHeight, BlockLineHeight, BaselineToBaseline
}, { namespace: "FerroUI.Controls.Documents" });

/// Base class for block-level text content. A block owns paragraph formatting;
/// character formatting continues to use the inherited [`TextElement`] properties.
#[repr(C)]
pub struct Block {
    base: TextElement,
}

ferro_class! {
    Block: TextElement, virtuals BlockImpl: TextElementImpl {
        /// Returns the text of this block, without a following paragraph separator.
        fn text(this) -> String;
        /// Appends formatter runs in content order.
        fn build_text_runs(this, text_runs: &mut Vec<Rc<dyn TextRun>>);
        /// Measures inline controls and reports whether a size changed.
        fn measure_embedded_controls(this, available_size: Size) -> bool;
    }
}

ferroui_base::ferro_class_info!(Block {
    markup: {
        properties: [HorizontalTextAlignment: TextAlignment {
            get: Block::horizontal_text_alignment, set: Block::set_horizontal_text_alignment
        }],
    },
});

ferroui_base::ferro_impl_classes!(Block: StyledElementImpl, TextElementImpl);

impl FerroObjectImpl for Block {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if matches!(
            change.property().name(),
            "TextAlignment" | "LineHeight" | "LineStackingStrategy" | "Margin" | "FlowDirection"
        ) {
            this.invalidate_format_change(change);
        }
    }
}

impl BlockImpl for Block {
    fn text(_this: &Self) -> String {
        String::new()
    }

    fn build_text_runs(_this: &Self, _text_runs: &mut Vec<Rc<dyn TextRun>>) {}

    fn measure_embedded_controls(_this: &Self, _available_size: Size) -> bool {
        false
    }
}

ferroui_base::ferro_properties! {
    impl Block {
        pub fn text_alignment_property() -> StyledProperty<TextAlignment> {
            crate::TextBlock::text_alignment_property().add_owner::<Block>()
        }

        pub fn line_height_property() -> StyledProperty<f64> {
            crate::TextBlock::line_height_property().add_owner::<Block>()
        }

        pub fn line_stacking_strategy_property() -> StyledProperty<LineStackingStrategy> {
            FerroProperty::register_with::<Block, _>("LineStackingStrategy",
                StyledPropertyOptions::new(LineStackingStrategy::MaxHeight).inherits(true))
        }

        pub fn margin_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<Block, _>("Margin",
                StyledPropertyOptions::new(Thickness::default()).validate(|value: &Thickness|
                    [value.left, value.top, value.right, value.bottom].iter().all(|v| v.is_finite() && *v >= 0.0)))
        }

        pub fn flow_direction_property() -> StyledProperty<FlowDirection> {
            ferroui_base::Visual::flow_direction_property().add_owner::<Block>()
        }
    }
}

impl Block {
    pub fn construct() -> Self {
        Self {
            base: TextElement::construct(),
        }
    }

    /// Length in UTF-16 code units, matching text formatter and document offsets.
    pub fn text_length(&self) -> usize {
        self.text().encode_utf16().count()
    }

    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value);
    }

    /// Alias of TextAlignment, sharing its backing property and inheritance.
    pub fn horizontal_text_alignment(&self) -> TextAlignment {
        self.text_alignment()
    }
    pub fn set_horizontal_text_alignment(&self, value: TextAlignment) {
        self.set_text_alignment(value);
    }

    /// `NaN` chooses the natural line height, as in the shared text formatter properties.
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }

    pub fn set_line_height(&self, value: f64) {
        self.set_value(
            Self::line_height_property(),
            if value == 0.0 { f64::NAN } else { value },
        );
    }

    pub fn line_stacking_strategy(&self) -> LineStackingStrategy {
        self.get_value(Self::line_stacking_strategy_property())
    }

    pub fn set_line_stacking_strategy(&self, value: LineStackingStrategy) {
        self.set_value(Self::line_stacking_strategy_property(), value);
    }

    pub fn margin(&self) -> Thickness {
        self.get_value(Self::margin_property())
    }

    pub fn set_margin(&self, value: Thickness) {
        self.set_value(Self::margin_property(), value);
    }

    pub fn flow_direction(&self) -> FlowDirection {
        self.get_value(Self::flow_direction_property())
    }

    pub fn set_flow_direction(&self, value: FlowDirection) {
        self.set_value(Self::flow_direction_property(), value);
    }
}
