//! Port of `Pages/TextFormatterPage.xaml.cs`: the class of the document
//! `Pages/TextFormatterPage.xaml`.

use ferroui_base::layout::LayoutableImplExt;
use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::{
    DrawableTextRun, GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink, ITextSource,
    TextCharacters, TextFormatter, TextLine, TextParagraphProperties, TextRun, TextRunProperties,
    DEFAULT_TEXT_SOURCE_LENGTH,
};
use ferroui_base::media::{BaselineAlignment, Brushes, DrawingContext, IBrush, Typeface};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref, Size,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ContentControlImpl, Control, ControlImpl, TextBlock, UserControl};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct TextFormatterPage {
    base: UserControl,
    text_line: RefCell<Option<Rc<dyn TextLine>>>,
}

ferro_class!(TextFormatterPage: UserControl);
ferro_impl_classes!(
    TextFormatterPage: FerroObjectImpl,
    StyledElementImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(TextFormatterPage { new: TextFormatterPage::new });
xaml_class!(TextFormatterPage, "/Pages/TextFormatterPage.xaml");

impl VisualImpl for TextFormatterPage {
    fn render(this: &Self, context: &mut DrawingContext) {
        let text_line = this.text_line.borrow().clone();
        if let Some(text_line) = text_line {
            text_line.draw(context, Point::default());
        }
    }
}

impl LayoutableImpl for TextFormatterPage {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let default_run_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
            Typeface::default(),
            GenericTextRunProperties::DEFAULT_FONT_RENDERING_EM_SIZE,
            None,
            Some(Brushes::black() as Rc<dyn IBrush>),
            None,
            BaselineAlignment::Center,
            None,
            None,
        ));
        let paragraph_properties: Rc<dyn TextParagraphProperties> =
            Rc::new(GenericTextParagraphProperties::new(default_run_properties.clone()));

        let text_block = TextBlock::new();
        text_block.set_text(Some("ClickMe"));
        let control = Button::new();
        control.set_content(Some(Control::boxed(text_block)));

        this.set_content(Some(Control::boxed(control.clone())));

        let text_source = CustomTextSource::new(control.clone().upcast(), default_run_properties);

        control.measure(Size::INFINITY);

        let text_line =
            <dyn TextFormatter>::current().format_line(&text_source, 0, f64::INFINITY, &paragraph_properties, None);
        *this.text_line.borrow_mut() = text_line;

        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut current_x = 0.0;

        let text_runs: Vec<Rc<dyn TextRun>> =
            this.text_line.borrow().as_ref().map(|text_line| text_line.text_runs().to_vec()).unwrap_or_default();
        for text_run in text_runs {
            if let Some(control_run) = text_run.downcast_ref::<ControlRun>() {
                control_run
                    .control()
                    .arrange(Rect::from_position_size(Point::new(current_x, 0.0), DrawableTextRun::size(control_run)));
            }

            if let Some(drawable_text_run) = text_run.as_drawable() {
                current_x += drawable_text_run.size().width;
            }
        }

        final_size
    }
}

impl TextFormatterPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), text_line: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

struct CustomTextSource {
    control: Ref<Control>,
    default_properties: Rc<dyn TextRunProperties>,
    text: &'static str,
}

impl CustomTextSource {
    fn new(control: Ref<Control>, default_properties: Rc<dyn TextRunProperties>) -> Self {
        Self { control, default_properties, text: "<-Hello World->" }
    }

    /// `_text.Length`: the length of the text in UTF-16 code units.
    fn text_length(&self) -> i32 {
        self.text.encode_utf16().count() as i32
    }
}

impl ITextSource for CustomTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index >= self.text_length() * 2 + DEFAULT_TEXT_SOURCE_LENGTH {
            return None;
        }

        if text_source_index == self.text_length() {
            return Some(Rc::new(ControlRun::new(self.control.clone(), self.default_properties.clone())));
        }

        Some(Rc::new(TextCharacters::from_str(self.text, self.default_properties.clone())))
    }
}

struct ControlRun {
    control: Ref<Control>,
    properties: Rc<dyn TextRunProperties>,
}

impl ControlRun {
    fn new(control: Ref<Control>, properties: Rc<dyn TextRunProperties>) -> Self {
        Self { control, properties }
    }

    fn control(&self) -> &Ref<Control> {
        &self.control
    }
}

impl TextRun for ControlRun {
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

impl DrawableTextRun for ControlRun {
    fn size(&self) -> Size {
        self.control.desired_size()
    }

    fn baseline(&self) -> f64 {
        0.0
    }

    fn draw(&self, _drawing_context: &mut dyn ITextDrawingSink, _origin: Point) {
        // noop
    }
}
