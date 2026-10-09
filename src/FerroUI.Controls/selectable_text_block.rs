use crate::documents::InlineCollection;
use crate::text_block::InlinesTextSource;
use crate::utils::{ClipboardHelper, PrimarySelectionHelper, StringUtils};
use crate::{Application, ControlImpl, TextBlock, TextBlockImpl, TextBlockImplExt, TextBox, TopLevel};
use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use std::any::Any;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs, KeyGesture,
    KeyModifiers, MouseButton, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::{
    FormattedTextSource, GenericTextParagraphProperties, GenericTextRunProperties, ITextSource, TextLayout,
    TextRunProperties,
};
use ferroui_base::media::{BaselineAlignment, DrawingContext, IBrush, Typeface};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::{ReadOnlyMemory, ValueSpan};
use ferroui_base::{
    ferro_class, ferro_property, ferro_routed_event, instantiate, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Matrix, PixelRect, Point, Ref, StyledElementImpl,
    StyledProperty, StyledPropertyMetadata, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A control that displays a block of formatted text whose content can be
/// selected and copied.
#[repr(C)]
pub struct SelectableTextBlock {
    base: TextBlock,
    can_copy: Cell<bool>,
    word_selection_start: Cell<i32>,
    selection_at_pointer_press: Cell<(i32, i32)>,
    inlines_invalidated: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The UTF-16 form of the text the last layout was created for, so that
    /// a new layout of unchanged text (a new selection, a new constraint)
    /// does not transcode it again.
    utf16_text: RefCell<Option<(String, ReadOnlyMemory<u16>)>>,
}

ferro_class!(SelectableTextBlock: TextBlock);
ferroui_base::ferro_class_info!(SelectableTextBlock { new: SelectableTextBlock::new });

ferroui_base::ferro_impl_classes!(SelectableTextBlock: StyledElementImpl);
ferroui_base::ferro_impl_classes!(SelectableTextBlock: VisualImpl);
ferroui_base::ferro_impl_classes!(SelectableTextBlock: LayoutableImpl);
ferroui_base::ferro_impl_classes!(SelectableTextBlock: InteractiveImpl);
ferroui_base::ferro_impl_classes!(SelectableTextBlock: ControlImpl);

impl FerroObjectImpl for SelectableTextBlock {
    fn constructed(this: &Self) {
        // The static constructor runs before the instance constructors: the
        // base constructor already raises property changes handled here.
        Self::parent_constructed(this);
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == TextBlock::inlines_property().as_property() {
            let (old_inlines, new_inlines) = change.get_old_and_new_value::<Option<InlineCollection>>();

            if old_inlines.is_some() {
                let subscription = this.inlines_invalidated.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
            }

            if let Some(new_inlines) = new_inlines {
                let weak = this.to_ref().downgrade();
                let subscription = new_inlines.invalidated(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_text_or_inlines_changed();
                    }
                });
                *this.inlines_invalidated.borrow_mut() = Some(subscription);
            }

            this.on_text_or_inlines_changed();
        } else if change.property() == TextBlock::text_property().as_property() {
            this.on_text_or_inlines_changed();
        } else if change.property() == Self::selection_start_property().as_property()
            || change.property() == Self::selection_end_property().as_property()
        {
            this.raise_direct_property_changed(Self::selected_text_property(), &String::new(), &String::new());
            this.update_command_states();
            this.invalidate_text_layout();
        } else if change.property() == Self::selection_foreground_brush_property().as_property() {
            this.invalidate_text_layout();
        }
    }
}

impl InputElementImpl for SelectableTextBlock {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);

        this.update_command_states();
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        if !this.context_flyout().is_some_and(|flyout| flyout.is_open())
            && !this.context_menu().is_some_and(|menu| menu.is_open())
        {
            this.clear_selection();
        }

        this.update_command_states();
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);

        let mut handled = false;
        let keymap = Application::current()
            .expect("the application")
            .platform_settings()
            .expect("the platform settings of the application")
            .hotkey_configuration();

        let matches = |gestures: &[KeyGesture]| gestures.iter().any(|g| g.matches(Some(e)));

        if matches(&keymap.copy) {
            this.copy();
            handled = true;
        } else if matches(&keymap.select_all) {
            this.select_all();
            handled = true;
        }

        e.set_handled(handled);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        this.selection_at_pointer_press.set(this.get_selection_range());

        let text = this.content_text();
        let click_info = e.get_current_point(Some(this));

        if let Some(text) = text.filter(|_| click_info.properties.is_left_button_pressed) {
            let text: Vec<u16> = text.encode_utf16().collect();
            let padding = this.padding();

            let point = e.get_position(Some(this)) - Point::new(padding.left, padding.top);

            let click_to_select = e.key_modifiers().contains(KeyModifiers::SHIFT);

            let old_index = this.selection_start();

            let hit = this.text_layout().hit_test_point(point);
            let index = hit.text_position();

            match e.click_count() {
                1 => {
                    if click_to_select {
                        let word_selection_start = this.word_selection_start.get();

                        if word_selection_start >= 0 {
                            let previous_word = StringUtils::previous_word(&text, index);

                            if index > word_selection_start {
                                this.set_current_value(Self::selection_end_property(), StringUtils::next_word(&text, index));
                            }

                            if index < word_selection_start || previous_word == word_selection_start {
                                this.set_current_value(Self::selection_start_property(), previous_word);
                            }
                        } else {
                            this.set_current_value(Self::selection_start_property(), old_index.min(index));
                            this.set_current_value(Self::selection_end_property(), old_index.max(index));
                        }
                    } else {
                        this.set_current_value(Self::selection_start_property(), index);
                        this.set_current_value(Self::selection_end_property(), index);
                        this.word_selection_start.set(-1);
                    }
                }
                2 => {
                    if !StringUtils::is_start_of_word(&text, index) {
                        this.set_current_value(Self::selection_start_property(), StringUtils::previous_word(&text, index));
                    }

                    this.word_selection_start.set(this.selection_start());

                    if !StringUtils::is_end_of_word(&text, index) {
                        this.set_current_value(Self::selection_end_property(), StringUtils::next_word(&text, index));
                    }
                }
                3 => {
                    this.word_selection_start.set(-1);

                    this.select_all();
                }
                _ => {}
            }
        }

        let element: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&element));
        e.set_handled(true);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);

        // The selection should not change during a pointer move if the user right clicks.
        if this.is_captured_by(e) && e.get_current_point(Some(this)).properties.is_left_button_pressed {
            let text = this.content_text();
            let padding = this.padding();

            let point = e.get_position(Some(this)) - Point::new(padding.left, padding.top);

            let hit = this.text_layout().hit_test_point(point);
            let text_position = hit.text_position();

            let word_selection_start = this.word_selection_start.get();

            match text {
                Some(text) if word_selection_start >= 0 => {
                    let text: Vec<u16> = text.encode_utf16().collect();
                    let distance = text_position - word_selection_start;

                    if distance <= 0 {
                        this.set_current_value(Self::selection_start_property(), StringUtils::previous_word(&text, text_position));
                    }

                    if distance >= 0 {
                        if this.selection_start() != word_selection_start {
                            this.set_current_value(Self::selection_start_property(), word_selection_start);
                        }

                        this.set_current_value(Self::selection_end_property(), StringUtils::next_word(&text, text_position));
                    }
                }
                _ => {
                    this.set_current_value(Self::selection_end_property(), text_position);
                }
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if !this.is_captured_by(e) {
            return;
        }

        if e.initial_press_mouse_button() == MouseButton::Right {
            let padding = this.padding();

            let point = e.get_position(Some(this)) - Point::new(padding.left, padding.top);

            let hit = this.text_layout().hit_test_point(point);

            // A point below the text hits one past its end, so clamp it as the text box does for its caret.
            let caret_index = TextBox::coerce_caret_index(this, hit.text_position());

            // See if the mouse clicked inside the current selection:
            // if it did not, the selection moves to where the user clicked.
            let first_selection = this.selection_start().min(this.selection_end());
            let last_selection = this.selection_start().max(this.selection_end());
            let did_click_in_selection = this.selection_start() != this.selection_end()
                && caret_index >= first_selection
                && caret_index <= last_selection;
            if !did_click_in_selection {
                this.set_current_value(Self::selection_start_property(), caret_index);
                this.set_current_value(Self::selection_end_property(), caret_index);
            }
        }

        let selection = this.get_selection_range();
        if e.initial_press_mouse_button() == MouseButton::Left
            && selection.0 != selection.1
            && selection != this.selection_at_pointer_press.get()
        {
            // The pointer gesture changed the selection, publish it to the primary selection.
            let target = this.to_ref();
            drop(PrimarySelectionHelper::publish_text_async(this, move || Some(target.get_selection())));
        }

        e.pointer().capture(None);
    }
}

impl TextBlockImpl for SelectableTextBlock {
    fn create_text_layout(this: &Self, text: Option<&str>) -> Rc<TextLayout> {
        let typeface =
            Typeface::with_style(this.font_family(), this.font_style(), this.font_weight(), this.font_stretch());

        let default_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            typeface.clone(),
            this.font_size(),
            this.text_decorations(),
            this.foreground(),
            None,
            BaselineAlignment::Baseline,
            None,
            this.font_features(),
        ));

        let paragraph_properties = GenericTextParagraphProperties::with_all(
            this.flow_direction(),
            this.text_alignment(),
            true,
            false,
            default_properties.clone(),
            this.text_wrapping(),
            this.line_height(),
            0.0,
            this.letter_spacing(),
        );
        paragraph_properties.set_line_spacing(this.line_spacing());

        let mut text_style_overrides: Option<Vec<ValueSpan<Rc<dyn TextRunProperties>>>> = None;
        let selection_start = this.selection_start();
        let selection_end = this.selection_end();
        let start = selection_start.min(selection_end);
        let length = selection_start.max(selection_end) - start;

        if length > 0 {
            if let Some(selection_foreground_brush) = this.selection_foreground_brush() {
                let selection_properties = |typeface: Typeface, font_features| -> Rc<dyn TextRunProperties> {
                    Rc::new(GenericTextRunProperties::with_all(
                        typeface,
                        this.font_size(),
                        None,
                        Some(selection_foreground_brush.clone()),
                        None,
                        BaselineAlignment::Baseline,
                        None,
                        font_features,
                    ))
                };

                if let Some(text_runs) = this.text_runs() {
                    // Apply the selection foreground without changing the original text formatting:
                    // each run keeps its own typeface and font features and only the foreground
                    // brush is overridden.
                    let mut accumulated_length = 0;

                    for text_run in text_runs.iter() {
                        let run_length = text_run.text().len() as i32;

                        if accumulated_length + run_length <= start || accumulated_length >= start + length {
                            accumulated_length += run_length;
                            continue;
                        }

                        let overlap_start = start.max(accumulated_length);
                        let overlap_end = (start + length).min(accumulated_length + run_length);
                        let overlap_length = overlap_end - overlap_start;

                        let run_properties = text_run.properties();

                        text_style_overrides.get_or_insert_with(Vec::new).push(ValueSpan::new(
                            overlap_start,
                            overlap_length,
                            selection_properties(
                                run_properties.map_or_else(|| typeface.clone(), |p| p.typeface().clone()),
                                match run_properties.and_then(|p| p.font_features()) {
                                    Some(font_features) => Some(font_features.clone()),
                                    None => this.font_features(),
                                },
                            ),
                        ));

                        accumulated_length += run_length;
                    }
                } else {
                    text_style_overrides = Some(vec![ValueSpan::new(
                        start,
                        length,
                        selection_properties(typeface.clone(), this.font_features()),
                    )]);
                }
            }
        }

        let text_style_overrides: Option<Rc<[ValueSpan<Rc<dyn TextRunProperties>>]>> =
            text_style_overrides.map(Rc::from);

        let text_source: Rc<dyn ITextSource> = if this.has_complex_content() {
            this.ensure_text_runs();

            Rc::new(InlinesTextSource::new(this.text_runs().unwrap_or_default(), text_style_overrides))
        } else {
            Rc::new(FormattedTextSource::new(
                this.utf16_text(text.unwrap_or("")),
                default_properties,
                text_style_overrides,
            ))
        };

        let max_size = this.get_max_size_from_constraint();

        Rc::new(TextLayout::from_text_source(
            text_source,
            Rc::new(paragraph_properties),
            Some(this.text_trimming()),
            max_size.width,
            max_size.height,
            this.max_lines(),
            None,
        ))
    }

    fn render_text_layout(this: &Self, context: &mut DrawingContext, origin: Point) {
        let selection_start = this.selection_start();
        let selection_end = this.selection_end();

        if selection_start != selection_end {
            if let Some(selection_brush) = this.selection_brush() {
                let start = selection_start.min(selection_end);
                let length = selection_start.max(selection_end) - start;

                let rects = this.text_layout().hit_test_text_range(start, length);

                let state = context.push_transform(Matrix::create_translation(origin.x, origin.y));

                for rect in rects {
                    context.fill_rectangle(&selection_brush, PixelRect::from_rect(rect, 1.0).to_rect(1.0), 0.0);
                }

                context.pop(state);
            }
        }

        Self::parent_render_text_layout(this, context, origin);
    }
}

ferroui_base::ferro_properties! { impl SelectableTextBlock {
    ferro_property!(
        /// Defines the `SelectionStart` property.
        pub fn selection_start_property() -> StyledProperty<i32> {
            TextBox::selection_start_property()
                .add_owner_with::<SelectableTextBlock>(StyledPropertyMetadata::new(None).with_coerce(TextBox::coerce_caret_index))
        }
    );

    ferro_property!(
        /// Defines the `SelectionEnd` property.
        pub fn selection_end_property() -> StyledProperty<i32> {
            TextBox::selection_end_property()
                .add_owner_with::<SelectableTextBlock>(StyledPropertyMetadata::new(None).with_coerce(TextBox::coerce_caret_index))
        }
    );

    ferro_property!(
        /// Defines the `SelectedText` property.
        pub fn selected_text_property() -> DirectProperty<SelectableTextBlock, String> {
            FerroProperty::register_direct::<SelectableTextBlock, _>(
                "SelectedText",
                |o| o.selected_text(),
                None,
                String::new(),
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectionBrush` property.
        pub fn selection_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextBox::selection_brush_property().add_owner::<SelectableTextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `SelectionForegroundBrush` property.
        pub fn selection_foreground_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            TextBox::selection_foreground_brush_property().add_owner::<SelectableTextBlock>()
        }
    );

    ferro_property!(
        /// Defines the `CanCopy` property.
        pub fn can_copy_property() -> DirectProperty<SelectableTextBlock, bool> {
            TextBox::can_copy_property().add_owner::<SelectableTextBlock>(|o| o.can_copy(), None, None)
        }
    );
} }

impl SelectableTextBlock {
    ferro_routed_event!(
        /// Defines the `CopyingToClipboard` event.
        pub fn copying_to_clipboard_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<SelectableTextBlock, _>("CopyingToClipboard", RoutingStrategies::BUBBLE)
        }
    );

    fn static_constructor() {
        InputElement::focusable_property().override_default_value::<SelectableTextBlock>(true);
        Visual::affects_render::<SelectableTextBlock>(&[
            Self::selection_start_property().as_property(),
            Self::selection_end_property().as_property(),
            Self::selection_brush_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TextBlock::construct(),
            can_copy: Cell::new(false),
            word_selection_start: Cell::new(-1),
            selection_at_pointer_press: Cell::new((0, 0)),
            inlines_invalidated: RefCell::new(None),
            utf16_text: RefCell::new(None),
        }
    }

    /// Initializes a new instance of the `SelectableTextBlock` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the selection is about to be copied to the clipboard;
    /// marking the event handled cancels the copy.
    pub fn copying_to_clipboard(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::copying_to_clipboard_event(), handler)
    }

    /// The brush that highlights selected text.
    pub fn selection_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_brush_property())
    }

    pub fn set_selection_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_brush_property(), value)
    }

    /// The brush that is used for the foreground of selected text.
    pub fn selection_foreground_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::selection_foreground_brush_property())
    }

    pub fn set_selection_foreground_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::selection_foreground_brush_property(), value)
    }

    /// The character index of the beginning of the current selection.
    pub fn selection_start(&self) -> i32 {
        self.get_value(Self::selection_start_property())
    }

    pub fn set_selection_start(&self, value: i32) {
        self.set_value(Self::selection_start_property(), value)
    }

    /// The character index of the end of the current selection.
    pub fn selection_end(&self) -> i32 {
        self.get_value(Self::selection_end_property())
    }

    pub fn set_selection_end(&self, value: i32) {
        self.set_value(Self::selection_end_property(), value)
    }

    /// The content of the current selection.
    pub fn selected_text(&self) -> String {
        self.get_selection()
    }

    /// Whether the copy command can be executed.
    pub fn can_copy(&self) -> bool {
        self.can_copy.get()
    }

    fn set_can_copy(&self, value: bool) {
        self.set_and_raise_cell(Self::can_copy_property(), &self.can_copy, value);
    }

    /// Copies the current selection to the clipboard.
    ///
    /// Like the asynchronous method of the reference, everything up to the
    /// clipboard call runs before this returns. A failure platform
    /// clipboards are known for (see
    /// `ClipboardHelper::is_expected_clipboard_exception`) is logged; any
    /// other failure, and a panic of the clipboard, is raised on the
    /// dispatcher.
    pub fn copy(&self) {
        if !self.can_copy.get() {
            return;
        }

        let text = self.get_selection();

        if text.is_empty() {
            return;
        }

        let event_args = RoutedEventArgs::with_event(Self::copying_to_clipboard_event());

        self.raise_event(&event_args);

        if !event_args.handled() {
            let Some(clipboard) = TopLevel::get_top_level(Some(self)).and_then(|top_level| top_level.clipboard())
            else {
                return;
            };

            let this = self.to_ref();

            ClipboardHelper::start(async move {
                match clipboard.set_text_async(Some(&text)).await {
                    Ok(()) => {}
                    Err(ex) if ClipboardHelper::is_expected_clipboard_exception(&ex) => {
                        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                            let source: &dyn Any = &this;
                            logger.log_with_values(Some(source), "Failed to write text to clipboard: {Error}", &[&ex]);
                        }
                    }
                    Err(ex) => return Err(ex),
                }

                Ok(())
            });
        }
    }

    /// Selects all text of the text block.
    pub fn select_all(&self) {
        let text = self.content_text();

        self.set_current_value(Self::selection_start_property(), 0);
        self.set_current_value(Self::selection_end_property(), text.as_deref().map_or(0, utf16_length));
    }

    /// Clears the current selection.
    pub fn clear_selection(&self) {
        self.set_current_value(Self::selection_end_property(), self.selection_start());
    }

    /// The text the selection indices refer to: the text of the inlines when
    /// there are any, the `Text` otherwise.
    fn content_text(&self) -> Option<String> {
        if self.has_complex_content() {
            self.inlines().and_then(|inlines| inlines.text())
        } else {
            self.text()
        }
    }

    /// Whether the pointer of a pointer event is captured by this control.
    fn is_captured_by(&self, e: &PointerEventArgs) -> bool {
        e.pointer().captured().is_some_and(|captured| captured == self.to_ref())
    }

    fn get_selection_range(&self) -> (i32, i32) {
        let selection_start = self.selection_start();
        let selection_end = self.selection_end();

        (selection_start.min(selection_end), selection_start.max(selection_end))
    }

    fn on_text_or_inlines_changed(&self) {
        self.coerce_value(Self::selection_start_property().as_property());
        self.coerce_value(Self::selection_end_property().as_property());
        self.raise_direct_property_changed(Self::selected_text_property(), &String::new(), &String::new());
        self.update_command_states();
    }

    fn update_command_states(&self) {
        self.set_can_copy(self.has_selection());
    }

    /// Reports the same emptiness conditions as `get_selection`, without
    /// building the selected string.
    fn has_selection(&self) -> bool {
        let selection_start = self.selection_start();
        let selection_end = self.selection_end();
        let start = selection_start.min(selection_end);
        let end = selection_start.max(selection_end);

        if start == end {
            return false;
        }

        let text_length = self.content_text().as_deref().map_or(0, utf16_length);

        text_length > 0 && end <= text_length
    }

    fn get_selection(&self) -> String {
        let Some(text) = self.content_text() else {
            return String::new();
        };

        let text_length = utf16_length(&text);

        if text_length == 0 {
            return String::new();
        }

        let selection_start = self.selection_start();
        let selection_end = self.selection_end();
        let start = selection_start.min(selection_end);
        let end = selection_start.max(selection_end);

        if start == end || text_length < end {
            return String::new();
        }

        let length = (end - start).max(0);

        // The indices are UTF-16 code units; a selection that splits a
        // surrogate pair yields a replacement character for the lone half.
        let selected_text: Vec<u16> = text.encode_utf16().skip(start.max(0) as usize).take(length as usize).collect();

        String::from_utf16_lossy(&selected_text)
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

/// The length of `text` in UTF-16 code units.
fn utf16_length(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}
