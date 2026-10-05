use crate::documents::{EmbeddedControlRun, IInlineHost, Inline, InlineCollection, TextElement};
use crate::{Border, Control, ControlImpl, Decorator};
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, LayoutableImpl, LayoutableImplExt, VerticalAlignment};
use ferroui_base::media::text_formatting::{
    FormattedTextSource, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, TextCharacters,
    TextEndOfParagraph, TextLayout, TextLineImpl, TextRun, TextRunCache, TextRunProperties,
};
use ferroui_base::media::{
    DrawingContext, FlowDirection, FontFamily, FontFeatureCollection, FontStretch, FontStyle, FontWeight, IBrush, TextAlignment,
    TextDecorationCollection, TextTrimming, TextWrapping, Typeface,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{ReadOnlyMemory, ValueSpan};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, AttachedProperty, DirectProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, Thickness, Visual, VisualImpl, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// A control that displays a block of text.
#[repr(C)]
pub struct TextBlock {
    base: Control,
    text_layout: RefCell<Option<Rc<TextLayout>>>,
    text_run_cache: RefCell<Option<Rc<TextRunCache>>>,
    constraint: Cell<Size>,
    text_runs: RefCell<Option<Rc<Vec<Rc<dyn TextRun>>>>>,
    inlines: RefCell<Option<InlineCollection>>,
    inlines_invalidated: RefCell<Option<Rc<dyn IDisposable>>>,
    inline_host: OnceCell<Rc<dyn IInlineHost>>,
    clear_text_internal: Cell<bool>,
    /// The (resolved) text alignment the cached layout was created with.
    layout_text_alignment: Cell<TextAlignment>,
    /// The UTF-16 form of the text the last layout was created for, so that a
    /// new layout of unchanged text (a new constraint, an arrange) does not
    /// transcode it again.
    utf16_text: RefCell<Option<(String, ReadOnlyMemory<u16>)>>,
}

ferro_class! {
    TextBlock: Control, virtuals TextBlockImpl: ControlImpl {
        /// Renders the control (the overridable part of the sealed `render`).
        #[doc(hidden)]
        fn render_core(this, context: &mut DrawingContext);
        /// Renders the text layout to a drawing context.
        fn render_text_layout(this, context: &mut DrawingContext, origin: Point);
        /// Creates the `TextLayout` used to render the text.
        fn create_text_layout(this, text: Option<&str>) -> Rc<TextLayout>;
    }
}
ferroui_base::ferro_class_info!(TextBlock { new: TextBlock::new });

impl InteractiveImpl for TextBlock {}
impl InputElementImpl for TextBlock {}
impl ControlImpl for TextBlock {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::TextBlockAutomationPeer::new(this).upcast()
    }
}
impl StyledElementImpl for TextBlock {}

impl FerroObjectImpl for TextBlock {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let inlines = InlineCollection::new();
        inlines.set_logical_children(Some(this));
        inlines.set_inline_host(Some(this.as_inline_host()));
        this.set_inlines(Some(inlines));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::text_property().as_property()
            && this.has_complex_content()
            && !this.clear_text_internal.get()
        {
            if let Some(inlines) = this.inlines() {
                inlines.clear();
            }
        }

        match change.property().name() {
            // Properties that affect shaping: invalidate the run cache + layout.
            "FontSize" | "FontWeight" | "FontStyle" | "FontFamily" | "FontStretch" | "FlowDirection"
            | "LetterSpacing" | "Text" | "TextDecorations" | "FontFeatures" | "Foreground" => {
                this.invalidate_text_layout();
            }
            // Properties that do not affect shaping: preserve the run cache.
            "TextWrapping" | "TextTrimming" | "TextAlignment" | "Padding" | "LineHeight" | "LineSpacing"
            | "MaxLines" => {
                this.invalidate_text_layout_keep_cache();
            }
            "Inlines" => {
                let (old_value, new_value) = change.get_old_and_new_value::<Option<InlineCollection>>();
                this.on_inlines_changed(old_value, new_value);
                this.invalidate_text_layout();
            }
            _ => {}
        }
    }
}

impl VisualImpl for TextBlock {
    /// Renders the `TextBlock` to a drawing context.
    ///
    /// This member is sealed: subclasses override `render_text_layout`.
    fn render(this: &Self, context: &mut DrawingContext) {
        this.render_core(context);
    }

    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
}

impl LayoutableImpl for TextBlock {
    fn on_measure_invalidated(this: &Self) {
        this.dispose_text_layout();

        Self::parent_on_measure_invalidated(this);
    }

    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut padding = this.padding();

        if this.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(this);
            padding = LayoutHelper::round_layout_thickness(padding, scale);
        }

        let deflated_size = available_size.deflate(padding);

        if this.constraint.get() != deflated_size {
            // Reset the text layout when the constraint is not matching.
            this.dispose_text_layout();
            this.constraint.set(deflated_size);

            // Force arrange so text will be properly aligned.
            this.invalidate_arrange();
        }

        this.ensure_text_runs();

        if this.measure_embedded_controls(deflated_size) {
            // A line snapshots its metrics when it is formatted, so an
            // existing layout still reports the width and height the child
            // had before it was measured again.
            this.dispose_text_layout();
        }

        // This implicitly recreates the text layout with a new constraint if
        // we previously reset it.
        let text_layout = this.text_layout();

        // The text width used here is matching that the text presenter uses
        // to measure the text.
        Size::new(text_layout.width_including_trailing_whitespace(), text_layout.height()).inflate(padding)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut padding = this.padding();

        if this.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(this);
            padding = LayoutHelper::round_layout_thickness(padding, scale);
        }

        let available_size = final_size.deflate(padding);

        // The reference disposes the layout here and creates it again for the
        // arranged size, reusing the shaped runs of the run cache (a layout
        // created during measure is left aligned). A layout that was created
        // for the very same constraint and alignment (and has not been
        // invalidated since) is that layout already, so it is kept: nothing
        // is laid out again when the arranged size is the measured one.
        if this.constraint.get() != available_size
            || this.layout_text_alignment.get() != this.resolve_text_alignment(this.text_alignment())
        {
            this.dispose_text_layout();
            this.constraint.set(available_size);
        }

        // This implicitly recreates the text layout with a new constraint.
        let text_layout = this.text_layout();

        if this.has_complex_content() {
            // Clear visual children before complex run arrangement.
            this.visual_children().clear();

            let mut current_y = padding.top;

            for text_line in text_layout.text_lines() {
                let mut current_x = padding.left + text_line.start();

                for run in text_line.text_runs().iter() {
                    if let Some(drawable) = run.as_drawable() {
                        if let Some(control_run) = run.downcast_ref::<EmbeddedControlRun>() {
                            let control = control_run.control();

                            // Add again to prevent clipping.
                            this.visual_children().add(control.clone().upcast());

                            let offset_y = TextLineImpl::get_baseline_offset(&**text_line, drawable);

                            control.arrange(Rect::from_position_size(
                                Point::new(current_x, current_y + offset_y),
                                control.desired_size(),
                            ));
                        }

                        current_x += drawable.size().width;
                    }
                }

                current_y += text_line.height();
            }
        }

        final_size
    }
}

impl TextBlockImpl for TextBlock {
    fn render_core(this: &Self, context: &mut DrawingContext) {
        if let Some(background) = this.background() {
            context.fill_rectangle(&background, Rect::from_size(this.bounds().size()), 0.0);
        }

        let mut padding = this.padding();
        if this.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(this);
            padding = LayoutHelper::round_layout_thickness(padding, scale);
        }

        let mut top = padding.top;
        let text_height = this.text_layout().height();
        let bounds_height = this.bounds().height;

        if bounds_height < text_height {
            match this.vertical_alignment() {
                VerticalAlignment::Center => top += (bounds_height - text_height) / 2.0,
                VerticalAlignment::Bottom => top += bounds_height - text_height,
                _ => {}
            }
        }

        this.render_text_layout(context, Point::new(padding.left, top));
    }

    fn render_text_layout(this: &Self, context: &mut DrawingContext, origin: Point) {
        this.text_layout().draw(context, origin);
    }

    fn create_text_layout(this: &Self, text: Option<&str>) -> Rc<TextLayout> {
        let typeface =
            Typeface::with_style(this.font_family(), this.font_style(), this.font_weight(), this.font_stretch());

        let default_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            typeface,
            this.font_size(),
            this.text_decorations(),
            this.foreground(),
            None,
            Default::default(),
            None,
            this.font_features(),
        ));

        let paragraph_properties = GenericTextParagraphProperties::with_all(
            this.flow_direction(),
            if this.is_measure_valid() { this.text_alignment() } else { TextAlignment::Left },
            true,
            false,
            default_properties.clone(),
            this.text_wrapping(),
            this.line_height(),
            0.0,
            this.letter_spacing(),
        );
        paragraph_properties.set_line_spacing(this.line_spacing());

        let text_source: Rc<dyn ITextSource> = if this.has_complex_content() {
            this.ensure_text_runs();

            let text_runs = this.text_runs.borrow().clone().unwrap_or_default();

            Rc::new(InlinesTextSource::new(text_runs, None))
        } else {
            Rc::new(SimpleTextSource::new(this.utf16_text(text.unwrap_or("")), default_properties))
        };

        let max_size = this.get_max_size_from_constraint();

        let text_run_cache = this.text_run_cache.borrow_mut().get_or_insert_with(|| Rc::new(TextRunCache::new())).clone();

        Rc::new(TextLayout::from_text_source(
            text_source,
            Rc::new(paragraph_properties),
            Some(this.text_trimming()),
            max_size.width,
            max_size.height,
            this.max_lines(),
            Some(text_run_cache),
        ))
    }
}

/// The handle through which the inlines of a text block reach it.
struct TextBlockInlineHost(WeakRef<TextBlock>);

impl IInlineHost for TextBlockInlineHost {
    fn invalidate(&self) {
        if let Some(text_block) = self.0.upgrade() {
            text_block.invalidate_text_layout();
        }
    }

    fn with_visual_children(&self, f: &mut dyn FnMut(&FerroList<Ref<Visual>>)) {
        if let Some(text_block) = self.0.upgrade() {
            f(text_block.visual_children());
        }
    }

    fn as_text_block(&self) -> Option<Ref<TextBlock>> {
        self.0.upgrade()
    }
}

ferroui_base::ferro_properties! { impl TextBlock {
    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `Padding` property.
        pub fn padding_property() -> StyledProperty<Thickness> {
            Decorator::padding_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `FontFamily` property.
        pub fn font_family_property() -> StyledProperty<FontFamily> {
            TextElement::font_family_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `FontSize` property.
        pub fn font_size_property() -> StyledProperty<f64> {
            TextElement::font_size_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `FontStyle` property.
        pub fn font_style_property() -> StyledProperty<FontStyle> {
            TextElement::font_style_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `FontWeight` property.
        pub fn font_weight_property() -> StyledProperty<FontWeight> {
            TextElement::font_weight_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `FontStretch` property.
        pub fn font_stretch_property() -> StyledProperty<FontStretch> {
            TextElement::font_stretch_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `Foreground` property.
        pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextElement::foreground_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `BaselineOffset` attached property.
        pub fn baseline_offset_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "BaselineOffset",
                StyledPropertyOptions::new(0.0).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `LineHeight` property.
        pub fn line_height_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "LineHeight",
                StyledPropertyOptions::new(f64::NAN).validate(Self::is_valid_line_height).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `LineSpacing` property.
        pub fn line_spacing_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "LineSpacing",
                StyledPropertyOptions::new(0.0).validate(Self::is_valid_line_spacing).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `LetterSpacing` property.
        pub fn letter_spacing_property() -> StyledProperty<f64> {
            TextElement::letter_spacing_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `MaxLines` property.
        pub fn max_lines_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "MaxLines",
                StyledPropertyOptions::new(0).validate(Self::is_valid_max_lines).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TextBlock, _>("Text", None)
        }
    );

    ferro_property!(
        /// Defines the `TextAlignment` property.
        pub fn text_alignment_property() -> AttachedProperty<TextAlignment> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "TextAlignment",
                StyledPropertyOptions::new(TextAlignment::Start).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `TextWrapping` property.
        pub fn text_wrapping_property() -> AttachedProperty<TextWrapping> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "TextWrapping",
                StyledPropertyOptions::new(TextWrapping::default()).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `TextTrimming` property.
        pub fn text_trimming_property() -> AttachedProperty<Rc<dyn TextTrimming>> {
            FerroProperty::register_attached_with::<TextBlock, Control, _>(
                "TextTrimming",
                StyledPropertyOptions::new(<dyn TextTrimming>::none()).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `TextDecorations` property.
        pub fn text_decorations_property() -> StyledProperty<Option<TextDecorationCollection>> {
            Inline::text_decorations_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `FontFeatures` property.
        pub fn font_features_property() -> StyledProperty<Option<FontFeatureCollection>> {
            TextElement::font_features_property().add_owner::<TextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `Inlines` property.
        pub fn inlines_property() -> DirectProperty<TextBlock, Option<InlineCollection>> {
            FerroProperty::register_direct::<TextBlock, _>(
                "Inlines",
                |o| o.inlines(),
                Some(|o: &TextBlock, v| o.set_inlines(v)),
                None,
            )
        }
    );
} }

impl TextBlock {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<TextBlock>(true);

        Visual::affects_render::<TextBlock>(&[
            Self::background_property().as_property(),
            Self::foreground_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            text_layout: RefCell::new(None),
            text_run_cache: RefCell::new(None),
            constraint: Cell::new(Size::new(f64::NAN, f64::NAN)),
            text_runs: RefCell::new(None),
            inlines: RefCell::new(None),
            inlines_invalidated: RefCell::new(None),
            inline_host: OnceCell::new(),
            clear_text_internal: Cell::new(false),
            layout_text_alignment: Cell::new(TextAlignment::Left),
            utf16_text: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the `TextBlock` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The `TextLayout` used to render the text.
    ///
    /// The layout is cached until a property it depends on, the measure of
    /// the control or its constraint changes.
    pub fn text_layout(&self) -> Rc<TextLayout> {
        if let Some(text_layout) = self.text_layout.borrow().as_ref() {
            return text_layout.clone();
        }

        // The alignment `create_text_layout` uses: text is left aligned
        // until the control has been measured.
        let text_alignment = if self.is_measure_valid() { self.text_alignment() } else { TextAlignment::Left };
        self.layout_text_alignment.set(self.resolve_text_alignment(text_alignment));

        let text_layout = self.create_text_layout_core();
        *self.text_layout.borrow_mut() = Some(text_layout.clone());
        text_layout
    }

    /// The physical alignment of a logical one for the flow direction of the
    /// control.
    fn resolve_text_alignment(&self, text_alignment: TextAlignment) -> TextAlignment {
        let left_to_right = self.flow_direction() == FlowDirection::LeftToRight;

        match text_alignment {
            TextAlignment::Start if left_to_right => TextAlignment::Left,
            TextAlignment::Start => TextAlignment::Right,
            TextAlignment::End if left_to_right => TextAlignment::Right,
            TextAlignment::End => TextAlignment::Left,
            other => other,
        }
    }

    fn create_text_layout_core(&self) -> Rc<TextLayout> {
        // A layout can be created outside of a measure pass - a render pass
        // that runs before a queued measure, or a read of this property - so
        // it cannot rely on the measure pass having brought the complex
        // content up to date.
        self.ensure_text_runs();
        self.measure_embedded_controls(self.get_max_size_from_constraint());

        let text = self.text();
        self.create_text_layout(text.as_deref())
    }

    /// The padding to place around the `Text`.
    pub fn padding(&self) -> Thickness {
        self.get_value(Self::padding_property())
    }

    pub fn set_padding(&self, value: Thickness) {
        self.set_value(Self::padding_property(), value)
    }

    /// A brush used to paint the control's background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// The text.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }

    /// The font family used to draw the control's text.
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }

    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value)
    }

    /// The size of the control's text in points.
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }

    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value)
    }

    /// The font style used to draw the control's text.
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }

    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value)
    }

    /// The font weight used to draw the control's text.
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }

    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value)
    }

    /// The font stretch used to draw the control's text.
    pub fn font_stretch(&self) -> FontStretch {
        self.get_value(Self::font_stretch_property())
    }

    pub fn set_font_stretch(&self, value: FontStretch) {
        self.set_value(Self::font_stretch_property(), value)
    }

    /// The brush used to draw the control's text and other foreground
    /// elements.
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value)
    }

    /// The height of each line of content.
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }

    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value)
    }

    /// The extra distance between each line of content.
    pub fn line_spacing(&self) -> f64 {
        self.get_value(Self::line_spacing_property())
    }

    pub fn set_line_spacing(&self, value: f64) {
        self.set_value(Self::line_spacing_property(), value)
    }

    /// The letter spacing.
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
    }

    pub fn set_letter_spacing(&self, value: f64) {
        self.set_value(Self::letter_spacing_property(), value)
    }

    /// The maximum number of text lines.
    pub fn max_lines(&self) -> i32 {
        self.get_value(Self::max_lines_property())
    }

    pub fn set_max_lines(&self, value: i32) {
        self.set_value(Self::max_lines_property(), value)
    }

    /// The control's text wrapping mode.
    pub fn text_wrapping(&self) -> TextWrapping {
        self.get_value(Self::text_wrapping_property())
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_value(Self::text_wrapping_property(), value)
    }

    /// The control's text trimming mode.
    pub fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        self.get_value(Self::text_trimming_property())
    }

    pub fn set_text_trimming(&self, value: Rc<dyn TextTrimming>) {
        self.set_value(Self::text_trimming_property(), value)
    }

    /// The text alignment.
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value)
    }

    /// The text decorations.
    pub fn text_decorations(&self) -> Option<TextDecorationCollection> {
        self.get_value(Self::text_decorations_property())
    }

    pub fn set_text_decorations(&self, value: Option<TextDecorationCollection>) {
        self.set_value(Self::text_decorations_property(), value)
    }

    /// The font features.
    pub fn font_features(&self) -> Option<FontFeatureCollection> {
        self.get_value(Self::font_features_property())
    }

    pub fn set_font_features(&self, value: Option<FontFeatureCollection>) {
        self.set_value(Self::font_features_property(), value)
    }

    /// The inlines.
    pub fn inlines(&self) -> Option<InlineCollection> {
        self.inlines.borrow().clone()
    }

    pub fn set_inlines(&self, value: Option<InlineCollection>) {
        self.set_and_raise(Self::inlines_property(), &self.inlines, value);
    }

    /// Whether the text block displays inlines rather than its `Text`.
    pub fn has_complex_content(&self) -> bool {
        self.inlines.borrow().as_ref().is_some_and(|inlines| inlines.count() > 0)
    }

    /// The constraint of the text layout as a maximum size: an unset
    /// constraint is zero.
    pub fn get_max_size_from_constraint(&self) -> Size {
        let constraint = self.constraint.get();
        let max_width = if constraint.width.is_nan() { 0.0 } else { constraint.width };
        let max_height = if constraint.height.is_nan() { 0.0 } else { constraint.height };
        Size::new(max_width, max_height)
    }

    /// The constraint the text layout is created for.
    pub fn constraint(&self) -> Size {
        self.constraint.get()
    }

    /// The text runs of the inlines, when they have been built.
    pub fn text_runs(&self) -> Option<Rc<Vec<Rc<dyn TextRun>>>> {
        self.text_runs.borrow().clone()
    }

    /// The baseline offset.
    pub fn baseline_offset(&self) -> f64 {
        self.get_value(Self::baseline_offset_property())
    }

    pub fn set_baseline_offset(&self, value: f64) {
        self.set_value(Self::baseline_offset_property(), value)
    }

    /// Reads the attached `BaselineOffset` property from the given control.
    pub fn get_baseline_offset(control: &Control) -> f64 {
        control.get_value(Self::baseline_offset_property())
    }

    /// Writes the attached `BaselineOffset` property to the given control.
    pub fn set_baseline_offset_on(control: &Control, value: f64) {
        control.set_value(Self::baseline_offset_property(), value)
    }

    /// Reads the attached `TextAlignment` property from the given control.
    pub fn get_text_alignment(control: &Control) -> TextAlignment {
        control.get_value(Self::text_alignment_property())
    }

    /// Writes the attached `TextAlignment` property to the given control.
    pub fn set_text_alignment_on(control: &Control, alignment: TextAlignment) {
        control.set_value(Self::text_alignment_property(), alignment)
    }

    /// Reads the attached `TextWrapping` property from the given control.
    pub fn get_text_wrapping(control: &Control) -> TextWrapping {
        control.get_value(Self::text_wrapping_property())
    }

    /// Writes the attached `TextWrapping` property to the given control.
    pub fn set_text_wrapping_on(control: &Control, wrapping: TextWrapping) {
        control.set_value(Self::text_wrapping_property(), wrapping)
    }

    /// Reads the attached `TextTrimming` property from the given control.
    pub fn get_text_trimming(control: &Control) -> Rc<dyn TextTrimming> {
        control.get_value(Self::text_trimming_property())
    }

    /// Writes the attached `TextTrimming` property to the given control.
    pub fn set_text_trimming_on(control: &Control, trimming: Rc<dyn TextTrimming>) {
        control.set_value(Self::text_trimming_property(), trimming)
    }

    /// Reads the attached `LineHeight` property from the given control.
    pub fn get_line_height(control: &Control) -> f64 {
        control.get_value(Self::line_height_property())
    }

    /// Writes the attached `LineHeight` property to the given control.
    pub fn set_line_height_on(control: &Control, height: f64) {
        control.set_value(Self::line_height_property(), height)
    }

    /// Reads the attached `LetterSpacing` property from the given control.
    pub fn get_letter_spacing(control: &Control) -> f64 {
        control.get_value(Self::letter_spacing_property())
    }

    /// Writes the attached `LetterSpacing` property to the given control.
    pub fn set_letter_spacing_on(control: &Control, letter_spacing: f64) {
        control.set_value(Self::letter_spacing_property(), letter_spacing)
    }

    /// Reads the attached `MaxLines` property from the given control.
    pub fn get_max_lines(control: &Control) -> i32 {
        control.get_value(Self::max_lines_property())
    }

    /// Writes the attached `MaxLines` property to the given control.
    pub fn set_max_lines_on(control: &Control, max_lines: i32) {
        control.set_value(Self::max_lines_property(), max_lines)
    }

    /// Clears the text without clearing the inlines.
    pub(crate) fn clear_text_internal(&self) {
        struct Reset<'a>(&'a Cell<bool>);

        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        self.clear_text_internal.set(true);
        let _reset = Reset(&self.clear_text_internal);
        self.set_current_value(Self::text_property(), None);
    }

    /// Invalidates the text layout and the shaped runs it was built from.
    pub fn invalidate_text_layout(&self) {
        if let Some(text_run_cache) = self.text_run_cache.borrow().as_ref() {
            text_run_cache.invalidate();
        }
        *self.text_runs.borrow_mut() = None;
        self.dispose_text_layout();
        self.invalidate_visual();
        self.invalidate_measure();
    }

    fn invalidate_text_layout_keep_cache(&self) {
        self.dispose_text_layout();
        self.invalidate_visual();
        self.invalidate_measure();
    }

    fn dispose_text_layout(&self) {
        let text_layout = self.text_layout.borrow_mut().take();
        if let Some(text_layout) = text_layout {
            text_layout.dispose();
        }
    }

    /// Builds the text runs of the inlines, unless they are up to date.
    pub fn ensure_text_runs(&self) {
        if self.text_runs.borrow().is_some() || !self.has_complex_content() {
            return;
        }

        let mut text_runs: Vec<Rc<dyn TextRun>> = Vec::new();

        if let Some(inlines) = self.inlines() {
            for inline in inlines.snapshot().iter() {
                inline.build_text_run(&mut text_runs);
            }
        }

        *self.text_runs.borrow_mut() = Some(Rc::new(text_runs));
    }

    fn measure_embedded_controls(&self, constraint: Size) -> bool {
        if !self.has_complex_content() {
            return false;
        }

        let mut resized = false;

        if let Some(inlines) = self.inlines() {
            for inline in inlines.snapshot().iter() {
                resized |= inline.measure_embedded_controls(constraint);
            }
        }

        resized
    }

    fn is_valid_max_lines(max_lines: &i32) -> bool {
        *max_lines >= 0
    }

    fn is_valid_line_height(line_height: &f64) -> bool {
        line_height.is_nan() || *line_height > 0.0
    }

    fn is_valid_line_spacing(line_spacing: &f64) -> bool {
        !line_spacing.is_nan() && !line_spacing.is_infinite()
    }

    fn on_inlines_changed(&self, old_value: Option<InlineCollection>, new_value: Option<InlineCollection>) {
        self.visual_children().clear();

        if let Some(old_value) = old_value {
            old_value.set_logical_children(None);
            old_value.set_inline_host(None);
            if let Some(subscription) = self.inlines_invalidated.borrow_mut().take() {
                subscription.dispose();
            }
        }

        if let Some(new_value) = new_value {
            new_value.set_logical_children(Some(self));
            new_value.set_inline_host(Some(self.as_inline_host()));
            let weak = self.to_ref().downgrade();
            *self.inlines_invalidated.borrow_mut() = Some(new_value.invalidated(move || {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_text_layout();
                }
            }));
        }
    }

    /// The handle through which inlines reach this text block.
    pub(crate) fn as_inline_host(&self) -> Rc<dyn IInlineHost> {
        self.inline_host.get_or_init(|| Rc::new(TextBlockInlineHost(self.to_ref().downgrade()))).clone()
    }

    /// The UTF-16 form of `text`, transcoded only when it differs from the
    /// text of the previous layout.
    fn utf16_text(&self, text: &str) -> ReadOnlyMemory<u16> {
        if let Some((cached_text, utf16)) = self.utf16_text.borrow().as_ref() {
            if cached_text == text {
                return utf16.clone();
            }
        }

        let utf16 = ReadOnlyMemory::<u16>::from_str(text);
        *self.utf16_text.borrow_mut() = Some((text.to_owned(), utf16.clone()));
        utf16
    }
}

/// A text source of one string with one set of run properties.
#[derive(Clone)]
pub struct SimpleTextSource {
    text: ReadOnlyMemory<u16>,
    default_properties: Rc<dyn TextRunProperties>,
}

impl SimpleTextSource {
    pub fn new(text: ReadOnlyMemory<u16>, default_properties: Rc<dyn TextRunProperties>) -> Self {
        Self { text, default_properties }
    }
}

impl ITextSource for SimpleTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index > self.text.len() as i32 {
            return Some(Rc::new(TextEndOfParagraph::new()));
        }

        let run_text = self.text.slice_from(text_source_index.max(0) as usize);

        if run_text.is_empty() {
            return Some(Rc::new(TextEndOfParagraph::new()));
        }

        Some(Rc::new(TextCharacters::new(run_text, self.default_properties.clone())))
    }
}

/// A text source of the text runs of inlines, with optional run property
/// overrides for ranges of the text.
#[derive(Clone)]
pub struct InlinesTextSource {
    text_runs: Rc<Vec<Rc<dyn TextRun>>>,
    text_modifier: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
}

impl InlinesTextSource {
    pub fn new(
        text_runs: Rc<Vec<Rc<dyn TextRun>>>,
        text_modifier: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
    ) -> Self {
        Self { text_runs, text_modifier }
    }

    /// The text runs of the source.
    pub fn text_runs(&self) -> &Rc<Vec<Rc<dyn TextRun>>> {
        &self.text_runs
    }
}

impl ITextSource for InlinesTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let mut current_position = 0;

        for text_run in self.text_runs.iter() {
            if text_run.length() == 0 {
                continue;
            }

            if text_source_index >= current_position + text_run.length() {
                current_position += text_run.length();

                continue;
            }

            if let Some(text_characters) = text_run.downcast_ref::<TextCharacters>() {
                let skip = (text_source_index - current_position).max(0) as usize;

                let text = text_run.text().slice_from(skip);

                let text_style_run = FormattedTextSource::create_text_style_run(
                    text.span(),
                    text_source_index,
                    text_characters.run_properties(),
                    self.text_modifier.as_deref(),
                );

                return Some(Rc::new(TextCharacters::new(
                    text.slice(0, text_style_run.length() as usize),
                    text_style_run.value().clone(),
                )));
            }

            return Some(text_run.clone());
        }

        Some(Rc::new(TextEndOfParagraph::new()))
    }
}
