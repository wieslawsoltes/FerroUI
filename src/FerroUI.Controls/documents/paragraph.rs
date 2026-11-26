use super::{
    Block, BlockImpl, IInlineHost, Inline, InlineCollection, LineSpacingRule, MarkerAlignment,
    MarkerStyle, MarkerType, TextElementImpl, TextElementImplExt, TextTabStops, MAX_LIST_LEVEL,
};
use crate::Control;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::metadata::IAddChild;
use ferroui_base::{
    ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, IntoRef, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions,
};
use std::rc::Rc;

/// A paragraph of structured inline content, including styled spans and embedded controls.
#[repr(C)]
pub struct Paragraph {
    base: Block,
    inlines: InlineCollection,
}

ferro_class!(Paragraph: Block);
ferroui_base::ferro_class_info!(Paragraph {
    new: Paragraph::new,
    interfaces: [
        Rc<dyn IAddChild<Ref<Inline>>> => Paragraph::as_add_child_of_inline,
        Rc<dyn IAddChild<Ref<Control>>> => Paragraph::as_add_child_of_control,
        Rc<dyn IAddChild<String>> => Paragraph::as_add_child_of_string,
    ],
    markup: {
        content: Inlines,
        properties: [Inlines: InlineCollection { get: Paragraph::inlines }],
    },
});

ferroui_base::ferro_impl_classes!(Paragraph: StyledElementImpl);

impl FerroObjectImpl for Paragraph {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.inlines.set_logical_children(Some(this));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::text_indent_property().as_property()
            || change.property() == Self::line_spacing_rule_property().as_property()
            || change.property() == Self::line_spacing_property().as_property()
            || change.property() == Self::left_indent_property().as_property()
            || change.property() == Self::right_indent_property().as_property()
            || change.property() == Self::tab_stops_property().as_property()
            || change.property() == Self::default_tab_stop_property().as_property()
            || change.property() == Self::list_type_property().as_property()
            || change.property() == Self::list_style_property().as_property()
            || change.property() == Self::list_start_property().as_property()
            || change.property() == Self::list_tab_property().as_property()
            || change.property() == Self::list_level_index_property().as_property()
            || change.property() == Self::list_alignment_property().as_property()
        {
            this.invalidate_format_change(change);
        }
    }
}

impl TextElementImpl for Paragraph {
    fn on_inline_host_changed(
        this: &Self,
        old_value: Option<&Rc<dyn IInlineHost>>,
        new_value: Option<&Rc<dyn IInlineHost>>,
    ) {
        Self::parent_on_inline_host_changed(this, old_value, new_value);
        this.inlines.set_inline_host(new_value.cloned());
    }
}

impl BlockImpl for Paragraph {
    fn text(this: &Self) -> String {
        this.inlines.text().unwrap_or_default()
    }

    fn build_text_runs(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        for inline in this.inlines.snapshot().iter() {
            inline.build_text_run(text_runs);
        }
    }

    fn measure_embedded_controls(this: &Self, available_size: Size) -> bool {
        let mut changed = false;
        for inline in this.inlines.snapshot().iter() {
            changed |= inline.measure_embedded_controls(available_size);
        }
        changed
    }
}

ferroui_base::ferro_properties! {
    impl Paragraph {
        pub fn tab_stops_property() -> StyledProperty<TextTabStops> {
            FerroProperty::register::<Paragraph, _>("TabStops", TextTabStops::default())
        }

        pub fn default_tab_stop_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("DefaultTabStop",
                StyledPropertyOptions::new(48.0)
                    .validate(|value: &f64| value.is_finite() && *value > 0.0))
        }

        pub fn text_indent_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("TextIndent",
                StyledPropertyOptions::new(0.0).validate(|value: &f64| value.is_finite()))
        }

        pub fn line_spacing_rule_property() -> StyledProperty<LineSpacingRule> {
            FerroProperty::register_with::<Paragraph, _>("LineSpacingRule",
                StyledPropertyOptions::new(LineSpacingRule::Undefined).inherits(true)
                    .validate(|value: &LineSpacingRule| *value != LineSpacingRule::Percent))
        }

        pub fn line_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("LineSpacing",
                StyledPropertyOptions::new(1.0).inherits(true)
                    .validate(|value: &f64| value.is_finite() && *value >= 0.0))
        }

        pub fn left_indent_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("LeftIndent",
                StyledPropertyOptions::new(0.0).validate(|value: &f64| value.is_finite()))
        }

        pub fn right_indent_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("RightIndent",
                StyledPropertyOptions::new(0.0).validate(|value: &f64| value.is_finite()))
        }

        pub fn list_type_property() -> StyledProperty<MarkerType> {
            FerroProperty::register::<Paragraph, _>("ListType", MarkerType::None)
        }

        pub fn list_style_property() -> StyledProperty<MarkerStyle> {
            FerroProperty::register::<Paragraph, _>("ListStyle", MarkerStyle::Parenthesis)
        }

        pub fn list_start_property() -> StyledProperty<i32> {
            FerroProperty::register::<Paragraph, _>("ListStart", 1)
        }

        pub fn list_tab_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Paragraph, _>("ListTab",
                StyledPropertyOptions::new(0.0).validate(|value: &f64| value.is_finite() && *value >= 0.0))
        }

        pub fn list_level_index_property() -> StyledProperty<i32> {
            FerroProperty::register_with::<Paragraph, _>("ListLevelIndex",
                StyledPropertyOptions::new(0).validate(|value: &i32| (0..=MAX_LIST_LEVEL).contains(value)))
        }

        pub fn list_alignment_property() -> StyledProperty<MarkerAlignment> {
            FerroProperty::register::<Paragraph, _>("ListAlignment", MarkerAlignment::Left)
        }
    }
}

impl Paragraph {
    pub fn construct() -> Self {
        Self {
            base: Block::construct(),
            inlines: InlineCollection::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_text(text: &str) -> Ref<Self> {
        let paragraph = Self::new();
        paragraph.inlines.add_text(text);
        paragraph
    }

    pub fn inlines(&self) -> InlineCollection {
        self.inlines.clone()
    }

    pub fn add_child(&self, child: impl IntoRef<Inline>) {
        self.inlines.add(child);
    }

    pub fn add_child_text(&self, text: &str) {
        self.inlines.add_text(text);
    }

    pub fn add_child_control(&self, child: impl IntoRef<Control>) {
        self.inlines.add_control(child);
    }

    /// Immutable explicit stops, in layout units from the paragraph's leading page edge.
    pub fn tab_stops(&self) -> TextTabStops {
        self.get_value(Self::tab_stops_property())
    }
    pub fn set_tab_stops(&self, value: TextTabStops) {
        self.set_value(Self::tab_stops_property(), value);
    }

    /// Spacing of implicit stops after the explicit configuration, in layout units.
    pub fn default_tab_stop(&self) -> f64 {
        self.get_value(Self::default_tab_stop_property())
    }
    /// Assigns an explicit interval in DIPs. When unassigned, the layout host
    /// supplies the interval: four paragraph ems in RichTextBlock, or the editable
    /// document's default (initially 48 DIPs / 36 points) in RichEditBox.
    pub fn set_default_tab_stop(&self, value: f64) {
        self.set_value(Self::default_tab_stop_property(), value);
    }

    pub fn text_indent(&self) -> f64 {
        self.get_value(Self::text_indent_property())
    }

    pub fn set_text_indent(&self, value: f64) {
        self.set_value(Self::text_indent_property(), value);
    }

    pub fn line_spacing_rule(&self) -> LineSpacingRule {
        self.get_value(Self::line_spacing_rule_property())
    }

    pub fn set_line_spacing_rule(&self, value: LineSpacingRule) {
        self.set_value(Self::line_spacing_rule_property(), value);
    }

    pub fn line_spacing(&self) -> f64 {
        self.get_value(Self::line_spacing_property())
    }

    pub fn set_line_spacing(&self, value: f64) {
        self.set_value(Self::line_spacing_property(), value);
    }

    pub fn left_indent(&self) -> f64 {
        self.get_value(Self::left_indent_property())
    }

    pub fn set_left_indent(&self, value: f64) {
        self.set_value(Self::left_indent_property(), value);
    }

    pub fn right_indent(&self) -> f64 {
        self.get_value(Self::right_indent_property())
    }

    pub fn set_right_indent(&self, value: f64) {
        self.set_value(Self::right_indent_property(), value);
    }

    /// The kind of the paragraph's list marker; `MarkerType::None` is no list.
    pub fn list_type(&self) -> MarkerType {
        self.get_value(Self::list_type_property())
    }

    pub fn set_list_type(&self, value: MarkerType) {
        self.set_value(Self::list_type_property(), value);
    }

    /// The punctuation around the paragraph's list number.
    pub fn list_style(&self) -> MarkerStyle {
        self.get_value(Self::list_style_property())
    }

    pub fn set_list_style(&self, value: MarkerStyle) {
        self.set_value(Self::list_style_property(), value);
    }

    /// The number of the first paragraph of a numbered sequence (for `UnicodeSequence`, the
    /// first code point).
    pub fn list_start(&self) -> i32 {
        self.get_value(Self::list_start_property())
    }

    pub fn set_list_start(&self, value: i32) {
        self.set_value(Self::list_start_property(), value);
    }

    /// The space for the marker before the text and the indent of each nested level, in
    /// layout units; 0 is two ems of the paragraph's font. A marker wider than the space
    /// widens it for its paragraph.
    pub fn list_tab(&self) -> f64 {
        self.get_value(Self::list_tab_property())
    }

    pub fn set_list_tab(&self, value: f64) {
        self.set_value(Self::list_tab_property(), value);
    }

    /// The 1-based list level, up to 9; 0 lays out and numbers a list paragraph at level 1.
    pub fn list_level_index(&self) -> i32 {
        self.get_value(Self::list_level_index_property())
    }

    pub fn set_list_level_index(&self, value: i32) {
        self.set_value(Self::list_level_index_property(), value);
    }

    /// Where the marker sits in its space.
    pub fn list_alignment(&self) -> MarkerAlignment {
        self.get_value(Self::list_alignment_property())
    }

    pub fn set_list_alignment(&self, value: MarkerAlignment) {
        self.set_value(Self::list_alignment_property(), value);
    }

    pub fn as_add_child_of_inline(this: Ref<Self>) -> Rc<dyn IAddChild<Ref<Inline>>> {
        Rc::new(ParagraphAddChild(this))
    }

    pub fn as_add_child_of_control(this: Ref<Self>) -> Rc<dyn IAddChild<Ref<Control>>> {
        Rc::new(ParagraphAddChild(this))
    }

    pub fn as_add_child_of_string(this: Ref<Self>) -> Rc<dyn IAddChild<String>> {
        Rc::new(ParagraphAddChild(this))
    }
}

struct ParagraphAddChild(Ref<Paragraph>);

impl IAddChild<Ref<Inline>> for ParagraphAddChild {
    fn add_child(&self, value: Ref<Inline>) {
        self.0.add_child(value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const Paragraph as usize
    }
}

impl IAddChild<Ref<Control>> for ParagraphAddChild {
    fn add_child(&self, value: Ref<Control>) {
        self.0.add_child_control(value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const Paragraph as usize
    }
}

impl IAddChild<String> for ParagraphAddChild {
    fn add_child(&self, value: String) {
        self.0.add_child_text(&value);
    }
    fn reference_id(&self) -> usize {
        &*self.0 as *const Paragraph as usize
    }
}
