use crate::documents::TextElement;
use crate::{Border, Control, ControlImpl, TextBlock, TextBox, TextBoxTextInputMethodClient};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, VerticalAlignment};
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::text_formatting::{
    GenericTextRunProperties, LogicalDirection, TextLayout, TextLayoutOptions, TextRunCache, TextRunProperties,
};
use ferroui_base::media::{
    BaselineAlignment, BrushExtensions, Brushes, CharacterHit, Color, DrawingContext, FlowDirection, FontFamily,
    FontFeatureCollection, FontStretch, FontStyle, FontWeight, IBrush, IImmutableBrush, IPen, TextAlignment,
    TextDecorations, TextWrapping, Typeface,
};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::{HandlerList, MathUtilities, ReadOnlyMemory, ValueSpan};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, AttachedProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, PixelRect, Point, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, StyledPropertyMetadata, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The control that displays the text of a text box: the text, the
/// selection, the pre-edit text of an input method and the caret.
///
/// Every text position (`CaretIndex`, `SelectionStart`, `SelectionEnd`, the
/// positions of [`CharacterHit`]) is a UTF-16 code unit index.
#[repr(C)]
pub struct TextPresenter {
    base: Control,
    caret_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    /// The subscription of `caret_timer_tick` to the tick of `caret_timer`.
    caret_timer_tick: RefCell<Option<Rc<dyn IDisposable>>>,
    caret_blink: Cell<bool>,
    // Dropped by `invalidate_text_layout` and `invalidate_text_layout_keep_cache`
    // when a property that affects the text changes, by `measure_override`
    // against a new constraint, and by `arrange_override` when the final
    // width differs from the measured one.
    text_layout: RefCell<Option<Rc<TextLayout>>>,
    text_run_cache: RefCell<Option<Rc<TextRunCache>>>,
    constraint: Cell<Size>,

    last_character_hit: Cell<CharacterHit>,
    caret_bounds: Cell<Rect>,
    caret_bounds_dirty: Cell<bool>,
    pending_caret_text_position: Cell<Option<i32>>,
    navigation_position: Cell<Point>,
    previous_offset: Cell<Option<Point>>,
    // Not ported: `_layer` (waits for `TextSelectorLayer`).
    caret_bounds_changed: HandlerList<dyn Fn()>,
    current_im_client: RefCell<Option<Weak<TextBoxTextInputMethodClient>>>,
    /// The UTF-16 form of `Text` (the outer `None`: not transcoded since the
    /// text changed; the inner one: the text is null).
    utf16_text: RefCell<Option<Option<ReadOnlyMemory<u16>>>>,
}

ferro_class! {
    TextPresenter: Control, virtuals TextPresenterImpl: ControlImpl {
        /// Creates the `TextLayout` used to render the text.
        fn create_text_layout(this) -> Rc<TextLayout>;
        /// Drops the text layout and the shaped runs it was built from.
        fn invalidate_text_layout(this);
    }
}
ferroui_base::ferro_class_info!(TextPresenter { new: TextPresenter::new });

impl InteractiveImpl for TextPresenter {}
impl InputElementImpl for TextPresenter {}
impl ControlImpl for TextPresenter {}
impl StyledElementImpl for TextPresenter {}

impl FerroObjectImpl for TextPresenter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();

        if property == Self::text_property().as_property() {
            *this.utf16_text.borrow_mut() = None;
        }

        Self::parent_on_property_changed(this, change);

        if property == Self::caret_index_property().as_property() {
            this.move_caret_to_text_position(change.get_new_value::<i32>(), false);
        }

        if (property == Self::text_property().as_property() || property == Self::caret_index_property().as_property())
            && this.preedit_text().is_some_and(|preedit_text| !preedit_text.is_empty())
        {
            this.set_current_value(Self::preedit_text_property(), None);
        }

        if property == Self::caret_blink_interval_property().as_property() {
            this.reset_caret_timer();
        }

        match property.name() {
            // Properties that affect shaping: invalidate the run cache + layout.
            "PreeditText" | "Foreground" | "FontSize" | "FontStyle" | "FontWeight" | "FontFamily" | "FontStretch"
            | "Text" | "LetterSpacing" | "PasswordChar" | "RevealPassword" | "FlowDirection"
            | "SelectionForegroundBrush" | "ShowSelectionHighlight" => {
                this.invalidate_text_layout();
            }
            // Properties that do not affect shaping: preserve the run cache.
            "TextAlignment" | "TextWrapping" | "LineHeight" => {
                this.invalidate_text_layout_keep_cache();
            }
            "SelectionStart" | "SelectionEnd" => {
                if this.selection_foreground_brush().is_some() {
                    this.invalidate_text_layout();
                } else {
                    this.invalidate_text_layout_keep_cache();
                }
            }
            _ => {}
        }

        // After the invalidation above, so the caret is computed against a
        // layout that already contains the new preedit text; the stale layout
        // does not cover the preedit-shifted caret index.
        if property == Self::preedit_text_property().as_property() {
            let preedit_text = change.get_new_value::<Option<String>>();
            this.on_preedit_changed(preedit_text.as_deref(), this.preedit_text_cursor_position());
        }

        if property == Self::preedit_text_cursor_position_property().as_property() {
            let preedit_text = this.preedit_text();
            this.on_preedit_changed(preedit_text.as_deref(), this.preedit_text_cursor_position());
        }
    }
}

impl VisualImpl for TextPresenter {
    /// Renders the `TextPresenter` to a drawing context.
    ///
    /// This member is sealed.
    fn render(this: &Self, context: &mut DrawingContext) {
        let selection_start = this.selection_start();
        let selection_end = this.selection_end();
        let selection_brush = this.selection_brush();

        if let Some(selection_brush) = selection_brush {
            if this.show_selection_highlight() && selection_start != selection_end {
                let start = selection_start.min(selection_end);
                let length = selection_start.max(selection_end) - start;

                let rects = this.text_layout().hit_test_text_range(start, length);

                for rect in rects {
                    context.fill_rectangle(&selection_brush, PixelRect::from_rect(rect, 1.0).to_rect(1.0), 0.0);
                }
            }
        }

        if let Some(root) = this.visual_root() {
            let offset = this.translate_point(this.bounds().position(), &root);

            if this.previous_offset.get() != offset {
                this.previous_offset.set(offset);
            }
        }

        this.render_internal(context);

        if selection_start != selection_end || !this.caret_blink.get() {
            return;
        }

        let caret_brush: Rc<dyn IImmutableBrush> = match this.caret_brush() {
            Some(caret_brush) => BrushExtensions::to_immutable(&caret_brush),
            None => {
                let background_color = this
                    .background()
                    .and_then(|background| background.as_solid_color_brush().map(|brush| brush.color()));

                match background_color {
                    Some(background_color) => {
                        let red = !background_color.r;
                        let green = !background_color.g;
                        let blue = !background_color.b;

                        Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(red, green, blue)))
                    }
                    None => Brushes::black(),
                }
            }
        };

        let (p1, p2) = this.get_caret_points();

        let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(Some(caret_brush), 1.0));

        context.draw_line(&pen, p1, p2);
    }

    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.reset_caret_timer();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        // Not ported: `RemoveTextSelectionCanvas()` (waits for `TextSelectionHandleCanvas` / `TextSelectorLayer`).

        let caret_timer = this.caret_timer.borrow().clone();

        if let Some(caret_timer) = caret_timer {
            caret_timer.stop();
            this.unsubscribe_caret_timer_tick();
        }
    }
}

impl LayoutableImpl for TextPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.constraint.set(available_size);

        this.dispose_text_layout();

        this.invalidate_arrange();

        let text_layout = this.text_layout();

        // The text width used here is matching that TextBlock uses to
        // measure the text.
        let size = Size::new(text_layout.width_including_trailing_whitespace(), text_layout.height());

        this.ensure_caret_bounds();

        size
    }

    fn arrange_override(this: &Self, mut final_size: Size) -> Size {
        let final_width = final_size.width;

        let text_width = this.text_layout().width_including_trailing_whitespace().ceil();

        if final_size.width < text_width {
            final_size = final_size.with_width(text_width);
        }

        // Check if the constraint has changed since the last measure, if so
        // recalculate the text layout according to the new size.
        // NOTE: It is important to check this against the actual final size
        // (excluding the trailing whitespace) to avoid text layout overflow.
        if !MathUtilities::are_close(this.constraint.get().width, final_width) {
            this.constraint.set(Size::new(final_width.ceil(), f64::INFINITY));

            this.dispose_text_layout();
        }

        final_size
    }
}

impl TextPresenterImpl for TextPresenter {
    fn create_text_layout(this: &Self) -> Rc<TextLayout> {
        let caret_index = this.caret_index();
        let preedit_text = this.preedit_text().filter(|preedit_text| !preedit_text.is_empty());
        let preedit_text = preedit_text.map(|preedit_text| ReadOnlyMemory::<u16>::from_str(&preedit_text));
        let text = Self::get_combined_text(this.utf16_text(), caret_index, preedit_text.as_ref());
        let typeface =
            Typeface::with_style(this.font_family(), this.font_style(), this.font_weight(), this.font_stretch());
        let selection_start = this.selection_start();
        let selection_end = this.selection_end();
        let start = selection_start.min(selection_end);
        let length = selection_start.max(selection_end) - start;

        let mut text_style_overrides: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> = None;

        let foreground = this.foreground();

        if let Some(preedit_text) = &preedit_text {
            let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
                typeface.clone(),
                this.font_size(),
                Some(TextDecorations::underline()),
                foreground,
                None,
                BaselineAlignment::Baseline,
                None,
                this.font_features(),
            ));

            let preedit_highlight = ValueSpan::new(caret_index, preedit_text.len() as i32, properties);

            text_style_overrides = Some(Rc::new([preedit_highlight]));
        } else if this.show_selection_highlight() && length > 0 {
            if let Some(selection_foreground_brush) = this.selection_foreground_brush() {
                let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
                    typeface.clone(),
                    this.font_size(),
                    None,
                    Some(selection_foreground_brush),
                    None,
                    BaselineAlignment::Baseline,
                    None,
                    this.font_features(),
                ));

                text_style_overrides = Some(Rc::new([ValueSpan::new(start, length, properties)]));
            }
        }

        let password_char = this.password_char();

        if password_char != '\0' && !this.reveal_password() {
            let length = text.as_ref().map_or(0, ReadOnlyMemory::len);
            let mut units = [0u16; 2];
            let units = password_char.encode_utf16(&mut units);
            let mut password = Vec::with_capacity(length * units.len());

            for _ in 0..length {
                password.extend_from_slice(units);
            }

            this.create_text_layout_internal(
                this.constraint.get(),
                Some(ReadOnlyMemory::from_vec(password)),
                typeface,
                text_style_overrides,
            )
        } else {
            this.create_text_layout_internal(this.constraint.get(), text, typeface, text_style_overrides)
        }
    }

    fn invalidate_text_layout(this: &Self) {
        let text_run_cache = this.text_run_cache.borrow().clone();

        if let Some(text_run_cache) = text_run_cache {
            text_run_cache.invalidate();
        }

        this.dispose_text_layout();

        this.invalidate_visual();
        this.invalidate_measure();
    }
}

ferroui_base::ferro_properties! { impl TextPresenter {
    ferro_property!(
        /// Defines the `ShowSelectionHighlight` property.
        pub fn show_selection_highlight_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextPresenter, _>("ShowSelectionHighlight", true)
        }
    );

    ferro_property!(
        /// Defines the `CaretIndex` property.
        pub fn caret_index_property() -> StyledProperty<i32> {
            TextBox::caret_index_property()
                .add_owner_with::<TextPresenter>(StyledPropertyMetadata::new(None).with_coerce(TextBox::coerce_caret_index))
        }
    );

    ferro_property!(
        /// Defines the `RevealPassword` property.
        pub fn reveal_password_property() -> StyledProperty<bool> {
            FerroProperty::register::<TextPresenter, _>("RevealPassword", false)
        }
    );

    ferro_property!(
        /// Defines the `PasswordChar` property.
        pub fn password_char_property() -> StyledProperty<char> {
            FerroProperty::register::<TextPresenter, _>("PasswordChar", '\0')
        }
    );

    ferro_property!(
        /// Defines the `SelectionBrush` property.
        pub fn selection_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextPresenter, _>("SelectionBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `SelectionForegroundBrush` property.
        pub fn selection_foreground_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextPresenter, _>("SelectionForegroundBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `CaretBrush` property.
        pub fn caret_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<TextPresenter, _>("CaretBrush", None)
        }
    );

    ferro_property!(
        /// Defines the `CaretBlinkInterval` property.
        pub fn caret_blink_interval_property() -> StyledProperty<Duration> {
            TextBox::caret_blink_interval_property().add_owner::<TextPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `SelectionStart` property.
        pub fn selection_start_property() -> StyledProperty<i32> {
            TextBox::selection_start_property()
                .add_owner_with::<TextPresenter>(StyledPropertyMetadata::new(None).with_coerce(TextBox::coerce_caret_index))
        }
    );

    ferro_property!(
        /// Defines the `SelectionEnd` property.
        pub fn selection_end_property() -> StyledProperty<i32> {
            TextBox::selection_end_property()
                .add_owner_with::<TextPresenter>(StyledPropertyMetadata::new(None).with_coerce(TextBox::coerce_caret_index))
        }
    );

    ferro_property!(
        /// Defines the `Text` property.
        pub fn text_property() -> StyledProperty<Option<String>> {
            TextBlock::text_property()
                .add_owner_with::<TextPresenter>(StyledPropertyMetadata::new(Some(Some(String::new()))))
        }
    );

    ferro_property!(
        /// Defines the `PreeditText` property.
        pub fn preedit_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TextPresenter, _>("PreeditText", None)
        }
    );

    ferro_property!(
        /// Defines the `PreeditTextCursorPosition` property.
        pub fn preedit_text_cursor_position_property() -> StyledProperty<Option<i32>> {
            FerroProperty::register::<TextPresenter, _>("PreeditTextCursorPosition", None)
        }
    );

    ferro_property!(
        /// Defines the `TextAlignment` property.
        pub fn text_alignment_property() -> AttachedProperty<TextAlignment> {
            TextBlock::text_alignment_property().add_owner::<TextPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `TextWrapping` property.
        pub fn text_wrapping_property() -> AttachedProperty<TextWrapping> {
            TextBlock::text_wrapping_property().add_owner::<TextPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `LineHeight` property.
        pub fn line_height_property() -> AttachedProperty<f64> {
            TextBlock::line_height_property().add_owner::<TextPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `LetterSpacing` property.
        pub fn letter_spacing_property() -> AttachedProperty<f64> {
            TextElement::letter_spacing_property().add_owner::<TextPresenter>()
        }
    );

    ferro_property!(
        /// Defines the `Background` property.
        pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            Border::background_property().add_owner::<TextPresenter>()
        }
    );
} }

impl TextPresenter {
    fn static_constructor() {
        Visual::affects_render::<TextPresenter>(&[
            Self::caret_brush_property().as_property(),
            Self::selection_brush_property().as_property(),
            Self::selection_foreground_brush_property().as_property(),
            TextElement::foreground_property().as_property(),
            Self::show_selection_highlight_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            caret_timer: RefCell::new(None),
            caret_timer_tick: RefCell::new(None),
            caret_blink: Cell::new(false),
            text_layout: RefCell::new(None),
            text_run_cache: RefCell::new(None),
            constraint: Cell::new(Size::default()),
            last_character_hit: Cell::new(CharacterHit::default()),
            caret_bounds: Cell::new(Rect::default()),
            caret_bounds_dirty: Cell::new(false),
            pending_caret_text_position: Cell::new(None),
            navigation_position: Cell::new(Point::default()),
            previous_offset: Cell::new(None),
            caret_bounds_changed: HandlerList::new(),
            current_im_client: RefCell::new(None),
            utf16_text: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the `TextPresenter` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs when the bounds of the caret change.
    pub fn caret_bounds_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.caret_bounds_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.caret_bounds_changed.remove(token);
            }
        })
    }

    /// A brush used to paint the control's background.
    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }

    /// Whether the presenter shows a selection highlight.
    pub fn show_selection_highlight(&self) -> bool {
        self.get_value(Self::show_selection_highlight_property())
    }

    pub fn set_show_selection_highlight(&self, value: bool) {
        self.set_value(Self::show_selection_highlight_property(), value)
    }

    /// The text.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }

    /// The text an input method is composing, displayed at the caret.
    pub fn preedit_text(&self) -> Option<String> {
        self.get_value(Self::preedit_text_property())
    }

    pub fn set_preedit_text(&self, value: Option<&str>) {
        self.set_value(Self::preedit_text_property(), value.map(str::to_owned))
    }

    /// The position of the cursor within the pre-edit text.
    pub fn preedit_text_cursor_position(&self) -> Option<i32> {
        self.get_value(Self::preedit_text_cursor_position_property())
    }

    pub fn set_preedit_text_cursor_position(&self, value: Option<i32>) {
        self.set_value(Self::preedit_text_cursor_position_property(), value)
    }

    /// The font family.
    pub fn font_family(&self) -> FontFamily {
        TextElement::get_font_family(self)
    }

    pub fn set_font_family(&self, value: FontFamily) {
        TextElement::set_font_family_on(self, value)
    }

    /// The font features.
    pub fn font_features(&self) -> Option<FontFeatureCollection> {
        TextElement::get_font_features(self)
    }

    pub fn set_font_features(&self, value: Option<FontFeatureCollection>) {
        TextElement::set_font_features_on(self, value)
    }

    /// The font size.
    pub fn font_size(&self) -> f64 {
        TextElement::get_font_size(self)
    }

    pub fn set_font_size(&self, value: f64) {
        TextElement::set_font_size_on(self, value)
    }

    /// The font style.
    pub fn font_style(&self) -> FontStyle {
        TextElement::get_font_style(self)
    }

    pub fn set_font_style(&self, value: FontStyle) {
        TextElement::set_font_style_on(self, value)
    }

    /// The font weight.
    pub fn font_weight(&self) -> FontWeight {
        TextElement::get_font_weight(self)
    }

    pub fn set_font_weight(&self, value: FontWeight) {
        TextElement::set_font_weight_on(self, value)
    }

    /// The font stretch.
    pub fn font_stretch(&self) -> FontStretch {
        TextElement::get_font_stretch(self)
    }

    pub fn set_font_stretch(&self, value: FontStretch) {
        TextElement::set_font_stretch_on(self, value)
    }

    /// A brush used to paint the text.
    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        TextElement::get_foreground(self)
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        TextElement::set_foreground_on(self, value)
    }

    /// The control's text wrapping mode.
    pub fn text_wrapping(&self) -> TextWrapping {
        self.get_value(Self::text_wrapping_property())
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_value(Self::text_wrapping_property(), value)
    }

    /// The line height. By default, this is set to NaN, which determines the
    /// appropriate height automatically.
    pub fn line_height(&self) -> f64 {
        self.get_value(Self::line_height_property())
    }

    pub fn set_line_height(&self, value: f64) {
        self.set_value(Self::line_height_property(), value)
    }

    /// The letter spacing.
    pub fn letter_spacing(&self) -> f64 {
        self.get_value(Self::letter_spacing_property())
    }

    pub fn set_letter_spacing(&self, value: f64) {
        self.set_value(Self::letter_spacing_property(), value)
    }

    /// The text alignment.
    pub fn text_alignment(&self) -> TextAlignment {
        self.get_value(Self::text_alignment_property())
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_value(Self::text_alignment_property(), value)
    }

    /// The `TextLayout` used to render the text.
    pub fn text_layout(&self) -> Rc<TextLayout> {
        if let Some(text_layout) = self.text_layout.borrow().as_ref() {
            return text_layout.clone();
        }

        let text_layout = self.create_text_layout();
        *self.text_layout.borrow_mut() = Some(text_layout.clone());

        // The caret is measured against the new layout by
        // `ensure_caret_bounds`, which the measure pass runs once the layout
        // is stored. Doing it here would let a `caret_bounds_changed` handler
        // invalidate the layout this getter is returning.
        self.caret_bounds_dirty.set(true);

        text_layout
    }

    pub fn caret_index(&self) -> i32 {
        self.get_value(Self::caret_index_property())
    }

    pub fn set_caret_index(&self, value: i32) {
        self.set_value(Self::caret_index_property(), value)
    }

    /// The character displayed in place of every character of the text
    /// (`'\0'`: none).
    pub fn password_char(&self) -> char {
        self.get_value(Self::password_char_property())
    }

    pub fn set_password_char(&self, value: char) {
        self.set_value(Self::password_char_property(), value)
    }

    pub fn reveal_password(&self) -> bool {
        self.get_value(Self::reveal_password_property())
    }

    pub fn set_reveal_password(&self, value: bool) {
        self.set_value(Self::reveal_password_property(), value)
    }

    pub fn selection_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_brush_property())
    }

    pub fn set_selection_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_brush_property(), value)
    }

    pub fn selection_foreground_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_foreground_brush_property())
    }

    pub fn set_selection_foreground_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_foreground_brush_property(), value)
    }

    pub fn caret_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::caret_brush_property())
    }

    pub fn set_caret_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::caret_brush_property(), value)
    }

    /// The caret blink rate.
    pub fn caret_blink_interval(&self) -> Duration {
        self.get_value(Self::caret_blink_interval_property())
    }

    pub fn set_caret_blink_interval(&self, value: Duration) {
        self.set_value(Self::caret_blink_interval_property(), value)
    }

    pub fn selection_start(&self) -> i32 {
        self.get_value(Self::selection_start_property())
    }

    pub fn set_selection_start(&self, value: i32) {
        self.set_value(Self::selection_start_property(), value)
    }

    pub fn selection_end(&self) -> i32 {
        self.get_value(Self::selection_end_property())
    }

    pub fn set_selection_end(&self, value: i32) {
        self.set_value(Self::selection_end_property(), value)
    }

    // `TextSelectionHandleCanvas` (the property) is not ported: it waits for
    // the touch selection handles, which live in an overlay layer of the top
    // level.

    /// The input method client that currently edits the text of the
    /// presenter. The client refers to the presenter, so the presenter
    /// refers to it weakly.
    pub fn current_im_client(&self) -> Option<Rc<TextBoxTextInputMethodClient>> {
        self.current_im_client.borrow().as_ref().and_then(Weak::upgrade)
    }

    pub fn set_current_im_client(&self, value: Option<Rc<TextBoxTextInputMethodClient>>) {
        *self.current_im_client.borrow_mut() = value.as_ref().map(Rc::downgrade);
    }


    /// The UTF-16 form of `Text`, transcoded once per text.
    fn utf16_text(&self) -> Option<ReadOnlyMemory<u16>> {
        if let Some(utf16_text) = self.utf16_text.borrow().as_ref() {
            return utf16_text.clone();
        }

        let utf16_text = self.text().map(|text| ReadOnlyMemory::<u16>::from_str(&text));
        *self.utf16_text.borrow_mut() = Some(utf16_text.clone());
        utf16_text
    }

    /// Creates the `TextLayout` used to render the text.
    ///
    /// * `constraint` — the constraint of the text.
    /// * `text` — the text to format.
    fn create_text_layout_internal(
        &self,
        constraint: Size,
        text: Option<ReadOnlyMemory<u16>>,
        typeface: Typeface,
        text_style_overrides: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>>,
    ) -> Rc<TextLayout> {
        let max_width = if MathUtilities::is_zero(constraint.width) { f64::INFINITY } else { constraint.width };
        let max_height = if MathUtilities::is_zero(constraint.height) { f64::INFINITY } else { constraint.height };

        let text_run_cache = self.text_run_cache.borrow_mut().get_or_insert_with(|| Rc::new(TextRunCache::new())).clone();

        Rc::new(TextLayout::from_utf16(
            text.unwrap_or_default(),
            typeface,
            TextLayoutOptions {
                font_size: self.font_size(),
                foreground: self.foreground(),
                text_alignment: self.text_alignment(),
                text_wrapping: self.text_wrapping(),
                text_trimming: None,
                text_decorations: None,
                flow_direction: self.flow_direction(),
                max_width,
                max_height,
                line_height: self.line_height(),
                letter_spacing: self.letter_spacing(),
                max_lines: 0,
                font_features: self.font_features(),
                text_style_overrides,
                text_run_cache: Some(text_run_cache),
            },
        ))
    }

    /// Renders the `TextPresenter` to a drawing context.
    fn render_internal(&self, context: &mut DrawingContext) {
        if let Some(background) = self.background() {
            context.fill_rectangle(&background, Rect::from_size(self.bounds().size()), 0.0);
        }

        let mut top = 0.0;
        let left = 0.0;

        let text_layout = self.text_layout();
        let text_height = text_layout.height();
        let bounds_height = self.bounds().height;

        if bounds_height < text_height {
            match self.vertical_alignment() {
                VerticalAlignment::Center => top += (bounds_height - text_height) / 2.0,
                VerticalAlignment::Bottom => top += bounds_height - text_height,
                _ => {}
            }
        }

        text_layout.draw(context, Point::new(left, top));
    }

    /// The end points of the line the caret is drawn as.
    pub(crate) fn get_caret_points(&self) -> (Point, Point) {
        let last_character_hit = self.last_character_hit.get();
        let caret_index = last_character_hit.first_character_index() + last_character_hit.trailing_length();
        let text_layout = self.text_layout();
        let line_index =
            text_layout.get_line_index_from_character_index(caret_index, last_character_hit.trailing_length() > 0);
        let text_line = &text_layout.text_lines()[line_index as usize];

        let caret_bounds = self.caret_bounds.get();

        let mut x = caret_bounds.x.floor() + 0.5;
        let y = caret_bounds.y.floor() + 0.5;
        let b = caret_bounds.bottom().ceil() - 0.5;

        if caret_bounds.x > 0.0 && caret_bounds.x >= text_line.width_including_trailing_whitespace() {
            x -= 1.0;
        }

        (Point::new(x, y), Point::new(x, b))
    }

    pub fn show_caret(&self) {
        self.ensure_caret_timer();
        // Not ported: `EnsureTextSelectionLayer()` (waits for `TextSelectionHandleCanvas` / `TextSelectorLayer`).
        self.caret_blink.set(true);
        self.start_caret_timer();
        self.invalidate_visual();
    }

    pub fn hide_caret(&self) {
        self.caret_blink.set(false);
        self.stop_caret_timer();
        self.invalidate_visual();
    }

    pub(crate) fn caret_changed(&self) {
        if self.visual_parent().is_none() {
            return;
        }

        self.ensure_caret_timer();

        let is_enabled = self.caret_timer.borrow().as_ref().is_some_and(|caret_timer| caret_timer.is_enabled());

        if is_enabled {
            self.caret_blink.set(true);
            self.stop_caret_timer();
            self.start_caret_timer();
            self.invalidate_visual();
        } else {
            self.start_caret_timer();
            self.invalidate_visual();
            self.stop_caret_timer();
        }

        if self.is_measure_valid() {
            self.bring_into_view_rect(self.caret_bounds.get());
        } else {
            // The measure is currently invalid so there's no point trying to
            // bring the current char into view until a measure has been
            // carried out as the scroll viewer extents may not be up-to-date.
            let weak = self.to_ref().downgrade();

            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.bring_into_view_rect(this.caret_bounds.get());
                    }
                },
                // `AfterRender`, which the dispatcher does not export by name.
                DispatcherPriority::from_value(DispatcherPriority::UI_THREAD_RENDER.value() + 1),
            );
        }
    }

    /// The text with the pre-edit text inserted at the caret.
    fn get_combined_text(
        text: Option<ReadOnlyMemory<u16>>,
        caret_index: i32,
        preedit_text: Option<&ReadOnlyMemory<u16>>,
    ) -> Option<ReadOnlyMemory<u16>> {
        let Some(preedit_text) = preedit_text.filter(|preedit_text| !preedit_text.is_empty()) else {
            return text;
        };

        let Some(text) = text.filter(|text| !text.is_empty()) else {
            return Some(preedit_text.clone());
        };

        let text = text.span();
        // A caret outside of the text is a programmer error, as in the
        // reference (the property coerces what is set, a text set afterwards
        // does not move it).
        let caret_index = caret_index as usize;
        let mut combined = Vec::with_capacity(text.len() + preedit_text.len());

        combined.extend_from_slice(&text[..caret_index]);
        combined.extend_from_slice(preedit_text.span());
        combined.extend_from_slice(&text[caret_index..]);

        Some(ReadOnlyMemory::from_vec(combined))
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

    fn caret_timer_tick(&self) {
        self.caret_blink.set(!self.caret_blink.get());

        self.invalidate_visual();
    }

    fn start_caret_timer(&self) {
        let caret_timer = self.caret_timer.borrow().clone();

        if let Some(caret_timer) = caret_timer {
            caret_timer.start();
        }
    }

    fn stop_caret_timer(&self) {
        let caret_timer = self.caret_timer.borrow().clone();

        if let Some(caret_timer) = caret_timer {
            caret_timer.stop();
        }
    }

    fn unsubscribe_caret_timer_tick(&self) {
        let subscription = self.caret_timer_tick.borrow_mut().take();

        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    /// Normalizes a text position into a caret-valid [`CharacterHit`] via the
    /// line's caret hit walkers, so positions on cluster or run boundaries
    /// (the end of a preedit shaped with a fallback font, a surrogate pair)
    /// resolve to a hit the layout can measure.
    fn get_caret_character_hit(text_layout: &TextLayout, text_position: i32, trailing_edge: bool) -> CharacterHit {
        let line_index = text_layout.get_line_index_from_character_index(text_position, trailing_edge);
        let text_line = &text_layout.text_lines()[line_index as usize];

        let mut character_hit = text_line.get_previous_caret_character_hit(CharacterHit::new(text_position));

        let next_caret_character_hit = text_line.get_next_caret_character_hit(character_hit);

        if next_caret_character_hit.first_character_index() <= text_position {
            character_hit = next_caret_character_hit;
        }

        if text_position == character_hit.first_character_index() + character_hit.trailing_length() {
            return character_hit;
        }

        if trailing_edge {
            character_hit
        } else {
            CharacterHit::new(character_hit.first_character_index())
        }
    }

    pub fn move_caret_to_text_position(&self, text_position: i32, trailing_edge: bool) {
        let text_layout = self.text_layout();

        self.update_caret(Self::get_caret_character_hit(&text_layout, text_position, trailing_edge), true);

        self.navigation_position.set(self.caret_bounds.get().position());

        self.caret_changed();
    }

    pub fn move_caret_to_point(&self, point: Point) {
        let hit = self.text_layout().hit_test_point(point);

        self.update_caret(hit.character_hit(), true);

        self.navigation_position.set(self.caret_bounds.get().position());

        self.caret_changed();
    }

    pub fn move_caret_vertical(&self, direction: LogicalDirection) {
        let text_layout = self.text_layout();

        let mut line_index = text_layout.get_line_index_from_character_index(
            self.caret_index(),
            self.last_character_hit.get().trailing_length() > 0,
        );

        if line_index < 0 {
            return;
        }

        let Point { x: current_x, y: mut current_y } = self.navigation_position.get();

        if direction == LogicalDirection::Forward {
            if line_index + 1 > text_layout.text_lines().len() as i32 - 1 {
                return;
            }

            let text_line = &text_layout.text_lines()[line_index as usize];

            current_y += text_line.height();
        } else {
            if line_index - 1 < 0 {
                return;
            }

            line_index -= 1;

            let text_line = &text_layout.text_lines()[line_index as usize];

            current_y -= text_line.height();
        }

        let navigation_position = self.navigation_position.get();

        self.move_caret_to_point(Point::new(current_x, current_y));

        self.navigation_position.set(navigation_position.with_y(self.caret_bounds.get().y));

        self.caret_changed();
    }

    fn ensure_caret_timer(&self) {
        if self.caret_timer.borrow().is_none() {
            self.reset_caret_timer();
        }
    }

    fn reset_caret_timer(&self) {
        let mut is_enabled = false;

        let caret_timer = self.caret_timer.borrow_mut().take();

        if let Some(caret_timer) = caret_timer {
            self.unsubscribe_caret_timer_tick();

            if caret_timer.is_enabled() {
                caret_timer.stop();
                is_enabled = true;
            }
        }

        let caret_blink_interval = self.caret_blink_interval();

        if !caret_blink_interval.is_zero() {
            let caret_timer = DispatcherTimer::new();
            caret_timer.set_interval(caret_blink_interval);

            // The handler is created once per timer: a tick allocates nothing.
            let weak = self.to_ref().downgrade();
            let subscription = caret_timer.tick(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.caret_timer_tick();
                }
            });

            *self.caret_timer_tick.borrow_mut() = Some(subscription);
            *self.caret_timer.borrow_mut() = Some(caret_timer.clone());

            if is_enabled {
                caret_timer.start();
            }
        }
    }

    pub fn get_next_character_hit(&self, direction: LogicalDirection) -> CharacterHit {
        let Some(text) = self.utf16_text() else {
            return CharacterHit::default();
        };
        let text_length = text.len() as i32;

        let mut character_hit = self.last_character_hit.get();
        let mut caret_index = character_hit.first_character_index() + character_hit.trailing_length();

        let text_layout = self.text_layout();
        let text_lines = text_layout.text_lines();

        let mut line_index = text_layout.get_line_index_from_character_index(caret_index, false);

        if line_index < 0 {
            return CharacterHit::default();
        }

        if direction == LogicalDirection::Forward {
            while line_index < text_lines.len() as i32 {
                let text_line = &text_lines[line_index as usize];

                character_hit = text_line.get_next_caret_character_hit(character_hit);

                caret_index = character_hit.first_character_index() + character_hit.trailing_length();

                if text_line.trailing_whitespace_length() > 0
                    && caret_index == text_line.first_text_source_index() + text_line.length()
                {
                    character_hit = CharacterHit::new(caret_index);
                }

                if caret_index >= text_length {
                    character_hit = CharacterHit::new(text_length);

                    break;
                }

                if caret_index - text_line.new_line_length() == text_line.first_text_source_index() + text_line.length()
                {
                    break;
                }

                if caret_index <= self.caret_index() {
                    line_index += 1;

                    continue;
                }

                break;
            }
        } else {
            while line_index >= 0 {
                let text_line = &text_lines[line_index as usize];

                character_hit = text_line.get_previous_caret_character_hit(character_hit);

                caret_index = character_hit.first_character_index() + character_hit.trailing_length();

                if caret_index >= self.caret_index() {
                    line_index -= 1;

                    continue;
                }

                break;
            }
        }

        character_hit
    }

    pub fn move_caret_horizontal(&self, mut direction: LogicalDirection) {
        if self.flow_direction() == FlowDirection::RightToLeft {
            direction = if direction == LogicalDirection::Forward {
                LogicalDirection::Backward
            } else {
                LogicalDirection::Forward
            };
        }

        let character_hit = self.get_next_character_hit(direction);

        self.update_caret(character_hit, true);

        self.navigation_position.set(self.caret_bounds.get().position());

        self.caret_changed();
    }

    pub(crate) fn update_caret(&self, character_hit: CharacterHit, notify: bool) {
        self.last_character_hit.set(character_hit);
        self.pending_caret_text_position.set(None);
        self.caret_bounds_dirty.set(true);

        self.ensure_caret_bounds();

        if notify {
            self.set_current_value(
                Self::caret_index_property(),
                character_hit.first_character_index() + character_hit.trailing_length(),
            );
        }
    }

    /// Measures the caret against the current layout and reports a move.
    /// Reads the layout field rather than the property, so it never builds
    /// one: with no layout the caret stays dirty until the measure pass that
    /// builds it calls back here.
    fn ensure_caret_bounds(&self) {
        if !self.caret_bounds_dirty.get() {
            return;
        }

        let Some(text_layout) = self.text_layout.borrow().clone() else {
            return;
        };

        self.caret_bounds_dirty.set(false);

        if let Some(pending_position) = self.pending_caret_text_position.take() {
            self.last_character_hit.set(Self::get_caret_character_hit(&text_layout, pending_position, false));
        }

        let character_hit = self.last_character_hit.get();
        let caret_index = character_hit.first_character_index() + character_hit.trailing_length();

        let line_index =
            text_layout.get_line_index_from_character_index(caret_index, character_hit.trailing_length() > 0);
        let text_lines = text_layout.text_lines();
        let text_line = &text_lines[line_index as usize];
        let distance_x = text_line.get_distance_from_character_hit(character_hit);

        let mut distance_y = 0.0;

        for text_line in &text_lines[..line_index as usize] {
            distance_y += text_line.height();
        }

        let caret_bounds = Rect::new(distance_x, distance_y, 0.0, text_line.height());

        if caret_bounds != self.caret_bounds.get() {
            self.caret_bounds.set(caret_bounds);

            // A handler may invalidate the layout from here. Nothing below
            // reads it, and the invalidation schedules the measure pass that
            // rebuilds it.
            for (_, handler) in self.caret_bounds_changed.snapshot().iter() {
                handler();
            }
        }
    }

    #[allow(dead_code)] // called by the input method client of the text box
    pub(crate) fn get_cursor_rectangle(&self) -> Rect {
        self.ensure_caret_bounds();

        self.caret_bounds.get()
    }

    fn on_preedit_changed(&self, preedit_text: Option<&str>, cursor_position: Option<i32>) {
        match preedit_text.filter(|preedit_text| !preedit_text.is_empty()) {
            None => {
                self.pending_caret_text_position.set(Some(self.caret_index()));
                self.caret_bounds_dirty.set(true);

                self.ensure_caret_bounds();
            }
            Some(preedit_text) => {
                let preedit_length = preedit_text.encode_utf16().count() as i32;

                let cursor_pos = match cursor_position {
                    Some(cursor_position) if cursor_position >= 0 && cursor_position <= preedit_length => {
                        cursor_position
                    }
                    _ => preedit_length,
                };

                self.pending_caret_text_position.set(Some(self.caret_index() + cursor_pos));
                self.caret_bounds_dirty.set(true);

                self.invalidate_measure();
                self.caret_changed();
            }
        }
    }
}
