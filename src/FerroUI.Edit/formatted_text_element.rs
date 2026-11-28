use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::visual_line_element::{VisualLineElement, VisualLineElementData};
use crate::visual_line_error::VisualLineError;
use ferroui_base::media::text_formatting::{
    DrawableTextRun, FormattedTextSource, GenericTextParagraphProperties, ITextDrawingSink,
    TextLayout, TextLine, TextRun, TextRunProperties,
};
use ferroui_base::media::{FormattedText, IBrush, IPen};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{Point, Rect, Size};
use std::any::Any;
use std::cell::OnceCell;
use std::rc::Rc;

/// Native content measured and drawn by an inline text run.
#[derive(Clone)]
pub enum FormattedTextContent {
    FormattedText(Rc<FormattedText>),
    Layout(Rc<TextLayout>),
    Line(Rc<dyn TextLine>),
    Drawable(Rc<dyn DrawableTextRun>),
}
impl FormattedTextContent {
    fn size(&self) -> Size {
        match self {
            Self::FormattedText(text) => {
                Size::new(text.width_including_trailing_whitespace(), text.height())
            }
            Self::Layout(layout) => Size::new(
                layout.width_including_trailing_whitespace(),
                layout.height(),
            ),
            Self::Line(line) => {
                Size::new(line.width_including_trailing_whitespace(), line.height())
            }
            Self::Drawable(run) => run.size(),
        }
    }
    fn baseline(&self) -> f64 {
        match self {
            Self::FormattedText(text) => text.baseline(),
            Self::Layout(layout) => layout.baseline(),
            Self::Line(line) => line.baseline(),
            Self::Drawable(run) => run.baseline(),
        }
    }
    fn draw(&self, sink: &mut dyn ITextDrawingSink, origin: Point) {
        match self {
            Self::FormattedText(text) => text.draw(sink, origin),
            Self::Layout(layout) => layout.draw(sink, origin),
            Self::Line(line) => line.draw(sink, origin),
            Self::Drawable(run) => run.draw(sink, origin),
        }
    }
}

/// An atomic inline element with its own measured and drawn native text content.
pub struct FormattedTextElement {
    pub(crate) data: VisualLineElementData,
    text: Option<String>,
    content: Option<FormattedTextContent>,
    prepared: OnceCell<Rc<TextLayout>>,
    display_columns: usize,
}
impl FormattedTextElement {
    pub fn new(
        text: impl Into<String>,
        document_length: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        Ok(Self {
            data: VisualLineElementData::new(1, document_length, properties)?,
            text: Some(text.into()),
            content: None,
            prepared: OnceCell::new(),
            display_columns: 1,
        })
    }
    pub fn from_content(
        content: FormattedTextContent,
        document_length: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        Ok(Self {
            data: VisualLineElementData::new(1, document_length, properties)?,
            text: None,
            content: Some(content),
            prepared: OnceCell::new(),
            display_columns: 1,
        })
    }
    /// Displays a configured native text object as one atomic visual column.
    /// After changing a shared text object, call `TextArea::redraw` to remeasure it.
    pub fn from_formatted_text(
        text: Rc<FormattedText>,
        document_length: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        // Deviation (DEVIATIONS.md, Editor visual lines): send the prepared text
        // straight to the run. The pinned factory tries to format an unset string.
        Self::from_content(
            FormattedTextContent::FormattedText(text),
            document_length,
            properties,
        )
    }
    pub fn from_text_line(
        line: Rc<dyn TextLine>,
        document_length: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        Self::from_content(
            FormattedTextContent::Line(line),
            document_length,
            properties,
        )
    }
    pub fn from_text_layout(
        layout: Rc<TextLayout>,
        document_length: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<Self, VisualLineError> {
        Self::from_content(
            FormattedTextContent::Layout(layout),
            document_length,
            properties,
        )
    }
    pub fn set_display_column_length(&mut self, value: usize) {
        self.display_columns = value;
    }
    pub fn prepare_text(text: &str, properties: Rc<dyn TextRunProperties>) -> Rc<TextLayout> {
        let source = Rc::new(FormattedTextSource::new(
            ReadOnlyMemory::from_str(text),
            properties.clone(),
            None,
        ));
        Rc::new(TextLayout::from_text_source(
            source,
            Rc::new(GenericTextParagraphProperties::new(properties)),
            None,
            f64::INFINITY,
            f64::INFINITY,
            0,
            None,
        ))
    }
    pub fn content(&self) -> FormattedTextContent {
        self.content.clone().unwrap_or_else(|| {
            FormattedTextContent::Layout(
                self.prepared
                    .get_or_init(|| {
                        Self::prepare_text(
                            self.text.as_deref().unwrap_or_default(),
                            self.data.text_run_properties().snapshot(),
                        )
                    })
                    .clone(),
            )
        })
    }
    pub fn create_formatted_run(&self) -> FormattedTextRun {
        FormattedTextRun::new(self.content(), self.data.text_run_properties().snapshot())
    }
}
impl VisualLineElement for FormattedTextElement {
    fn data(&self) -> &VisualLineElementData {
        &self.data
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        self.prepared.take();
        &mut self.data
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn display_column_length(&self) -> usize {
        self.display_columns
    }
    fn create_text_run(
        &self,
        column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        self.data.checked_visual_index(column)?;
        Ok(Rc::new(self.create_formatted_run()))
    }
}

/// A native drawable run, with optional background, border and horizontal padding.
pub struct FormattedTextRun {
    content: FormattedTextContent,
    properties: Rc<dyn TextRunProperties>,
    horizontal_padding: f64,
    background: Option<Rc<dyn IBrush>>,
    border: Option<Rc<dyn IPen>>,
    corner_radius: f64,
    width_override: Option<f64>,
    height_override: Option<f64>,
}
impl FormattedTextRun {
    pub fn new(content: FormattedTextContent, properties: Rc<dyn TextRunProperties>) -> Self {
        Self {
            content,
            properties,
            horizontal_padding: 0.0,
            background: None,
            border: None,
            corner_radius: 0.0,
            width_override: None,
            height_override: None,
        }
    }
    pub fn with_box(
        mut self,
        padding: f64,
        background: Option<Rc<dyn IBrush>>,
        border: Option<Rc<dyn IPen>>,
    ) -> Self {
        self.horizontal_padding = padding.max(0.0);
        self.background = background;
        self.border = border;
        self
    }
    /// Sets the box corner radius in layout units.
    pub fn with_corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius.max(0.0);
        self
    }
    pub fn with_width(mut self, width: f64) -> Self {
        self.width_override = Some(width);
        self
    }
    pub fn with_size(mut self, size: Size) -> Self {
        self.width_override = Some(size.width);
        self.height_override = Some(size.height);
        self
    }
    pub fn content(&self) -> &FormattedTextContent {
        &self.content
    }
}
impl TextRun for FormattedTextRun {
    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        Some(&self.properties)
    }
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
impl DrawableTextRun for FormattedTextRun {
    fn size(&self) -> Size {
        let size = self.content.size();
        Size::new(
            self.width_override
                .unwrap_or(size.width + self.horizontal_padding),
            self.height_override.unwrap_or(size.height),
        )
    }
    fn baseline(&self) -> f64 {
        self.content.baseline()
    }
    fn draw(&self, sink: &mut dyn ITextDrawingSink, origin: Point) {
        if self.background.is_some() || self.border.is_some() {
            let size = self.size();
            let rect = Rect::new(origin.x, origin.y, size.width, size.height);
            if self.corner_radius > 0.0 {
                sink.draw_rounded_rectangle(
                    self.background.as_ref(),
                    self.border.as_ref(),
                    rect,
                    self.corner_radius,
                );
            } else {
                sink.draw_rectangle(self.background.as_ref(), self.border.as_ref(), rect);
            }
        }
        self.content.draw(
            sink,
            Point::new(origin.x + self.horizontal_padding / 2.0, origin.y),
        );
    }
}
