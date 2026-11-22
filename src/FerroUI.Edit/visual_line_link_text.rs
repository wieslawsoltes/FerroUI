use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::visual_line_element::{VisualLineElement, VisualLineElementData};
use crate::visual_line_error::VisualLineError;
use crate::visual_line_text::VisualLineText;
use crate::TextArea;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::input::{Cursor, KeyModifiers, PointerPressedEventArgs, StandardCursorType};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::{TextCharacters, TextRun, TextRunProperties};
use ferroui_base::media::{Color, IBrush, TextDecorationCollection, TextDecorations};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::ReadOnlyMemory;
use url::Url;
use ferroui_controls::TopLevel;
use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection};
use std::any::Any;
use std::rc::Rc;

#[derive(Clone)]
pub struct OpenUriRoutedEventArgs {
    base: RoutedEventArgs,
    pub uri: Url,
    pub target_name: Option<String>,
}
ferro_routed_event_args!(OpenUriRoutedEventArgs: RoutedEventArgs);
impl OpenUriRoutedEventArgs {
    pub fn new(uri: Url) -> Self {
        Self {
            base: RoutedEventArgs::with_event(&VisualLineLinkText::open_uri_event()),
            uri,
            target_name: None,
        }
    }
}
ferroui_base::ferro_thread_local! {
    static OPEN_URI_EVENT: RoutedEvent<OpenUriRoutedEventArgs> =
        *RoutedEvent::register::<TextArea, _>("OpenUri", RoutingStrategies::BUBBLE);
}

#[derive(Clone)]
pub struct LinkTextStyle {
    pub foreground: Option<Rc<dyn IBrush>>,
    pub background: Option<Rc<dyn IBrush>>,
    pub underline: bool,
}
impl Default for LinkTextStyle {
    fn default() -> Self {
        Self {
            foreground: Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                0, 0, 255,
            )))),
            background: None,
            underline: true,
        }
    }
}

/// Splittable text with native URI routing. Splitting retains URI, modifier
/// requirements, target and presentation so highlighting cannot disable a link.
#[derive(Clone)]
pub struct VisualLineLinkText {
    text: VisualLineText,
    split_group: Option<Rc<()>>,
    pub navigate_uri: Option<Url>,
    pub target_name: Option<String>,
    pub require_control_modifier_for_click: bool,
    pub style: LinkTextStyle,
}
impl VisualLineLinkText {
    pub fn new(length: usize, properties: &dyn TextRunProperties) -> Result<Self, VisualLineError> {
        Ok(Self {
            text: VisualLineText::new(length, properties)?,
            split_group: None,
            navigate_uri: None,
            target_name: None,
            require_control_modifier_for_click: true,
            style: LinkTextStyle::default(),
        })
    }
    pub fn open_uri_event() -> RoutedEvent<OpenUriRoutedEventArgs> {
        OPEN_URI_EVENT.with(|event| *event)
    }
    pub fn link_is_clickable(&self, modifiers: KeyModifiers) -> bool {
        self.navigate_uri.is_some()
            && (!self.require_control_modifier_for_click
                || modifiers.contains(crate::text_area::platform_keymap().command_modifiers))
    }

    pub(crate) fn underline_decorations(&self) -> Option<TextDecorationCollection> {
        self.style.underline.then(TextDecorations::underline)
    }

    pub(crate) fn is_same_split_link(&self, other: &Self) -> bool {
        self.split_group.as_ref().zip(other.split_group.as_ref())
            .is_some_and(|(left, right)| Rc::ptr_eq(left, right))
            && self.navigate_uri.is_some()
            && self.navigate_uri == other.navigate_uri
            && self.target_name == other.target_name
    }

    pub(crate) fn open_uri(view: &TextArea, uri: Url, target_name: Option<String>) {
        let mut args = OpenUriRoutedEventArgs::new(uri.clone());
        args.target_name = target_name;
        view.raise_event(&args);
        if !args.handled() {
            if let Some(top_level) = TopLevel::get_top_level(Some(&view.to_ref())) {
                let _ = Dispatcher::ui_thread().spawn_local(async move {
                    top_level.launcher().launch_uri_async(&uri).await;
                });
            }
        }
    }
}
impl VisualLineElement for VisualLineLinkText {
    fn data(&self) -> &VisualLineElementData {
        self.text.data()
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        self.text.data_mut()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_text_run(
        &self,
        column: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        let run = self.text.create_text_run(column, context)?;
        let mut properties = self.data().text_run_properties().clone();
        if !self.data().has_selection_foreground() {
            properties.set_foreground_brush(self.style.foreground.clone());
        }
        properties.set_background_brush(self.style.background.clone());
        if let Some(underline) = self.underline_decorations() {
            properties.set_text_decorations(Some(underline));
        }
        Ok(Rc::new(TextCharacters::new(
            run.text(),
            properties.snapshot(),
        )))
    }
    fn get_preceding_text(
        &self,
        limit: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<ReadOnlyMemory<u16>, VisualLineError> {
        self.text.get_preceding_text(limit, context)
    }
    fn can_split(&self) -> bool {
        true
    }
    fn split(
        &self,
        column: usize,
    ) -> Result<(Box<dyn VisualLineElement>, Box<dyn VisualLineElement>), VisualLineError> {
        let offset = self
            .data()
            .relative_text_offset()
            .checked_add(column.saturating_sub(self.data().visual_column()))
            .ok_or(VisualLineError::OffsetOverflow)?;
        let (left, right) = self.data().split_at(column, offset)?;
        let mut first = self.clone();
        let mut second = self.clone();
        let group = self.split_group.clone().unwrap_or_else(|| Rc::new(()));
        first.split_group = Some(group.clone());
        second.split_group = Some(group);
        first.text = VisualLineText::with_data(left);
        second.text = VisualLineText::with_data(right);
        Ok((Box::new(first), Box::new(second)))
    }
    fn get_visual_column(&self, offset: usize) -> usize {
        self.text.get_visual_column(offset)
    }
    fn get_relative_offset(&self, column: usize) -> usize {
        self.text.get_relative_offset(column)
    }
    fn get_next_caret_position(
        &self,
        column: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        self.text
            .get_next_caret_position(column, direction, mode, context)
    }
    fn is_whitespace(&self, column: usize, context: &dyn ITextRunConstructionContext) -> bool {
        self.text.is_whitespace(column, context)
    }
    fn is_document_offset_visible(&self, offset: usize) -> bool {
        self.text.is_document_offset_visible(offset)
    }
    fn query_cursor(&self, modifiers: KeyModifiers) -> Option<Rc<Cursor>> {
        self.link_is_clickable(modifiers)
            .then(|| Cursor::new(StandardCursorType::Hand))
    }
    fn on_pointer_pressed(&self, view: &TextArea, event: &PointerPressedEventArgs) {
        if event.handled() || !self.link_is_clickable(event.key_modifiers()) {
            return;
        }
        let Some(uri) = self.navigate_uri.clone() else {
            return;
        };
        Self::open_uri(view, uri, self.target_name.clone());
        event.set_handled(true);
    }
}
