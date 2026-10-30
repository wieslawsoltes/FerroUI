use super::{Inline, InlineImpl, TextElementImpl};
use ferroui_base::media::text_formatting::{TextCharacters, TextRun};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use std::rc::Rc;

/// The line terminator of the platform.
const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// `LineBreak` element that forces a line breaking.
#[repr(C)]
pub struct LineBreak {
    base: Inline,
}

ferro_class!(LineBreak: Inline);
ferroui_base::ferro_class_info!(LineBreak { new: LineBreak::new });
ferro_impl_classes!(LineBreak: FerroObjectImpl, StyledElementImpl, TextElementImpl);

impl InlineImpl for LineBreak {
    fn build_text_run(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        let text_run_properties = this.create_text_run_properties();

        let text_characters = TextCharacters::from_str(this.line_break_text(), text_run_properties);

        text_runs.push(Rc::new(text_characters));
    }

    fn append_text(this: &Self, string_builder: &mut String) {
        string_builder.push_str(this.line_break_text());
    }
}

/// The soft line break of a rich paragraph: one UTF-16 unit on every platform, the
/// vertical tab of RichEdit's text object model (`docs/porting/winui-rich-text.md`,
/// decision D10). The formatter breaks the line at it.
pub(crate) const RICH_LINE_BREAK: &str = "\u{000B}";

impl LineBreak {
    /// Deviation (DEVIATIONS.md, Text block): in a `Paragraph` of a rich document a line
    /// break is U+000B, one unit everywhere; elsewhere it is the platform's line
    /// terminator, as upstream.
    fn line_break_text(&self) -> &'static str {
        let mut parent = self.parent();
        while let Some(element) = parent {
            if element.is::<super::Paragraph>() {
                return RICH_LINE_BREAK;
            }
            if !element.is::<Inline>() {
                break;
            }
            parent = element.parent();
        }
        NEW_LINE
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Inline::construct() }
    }

    /// Creates a new `LineBreak` instance.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
