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

        let text_characters = TextCharacters::from_str(NEW_LINE, text_run_properties);

        text_runs.push(Rc::new(text_characters));
    }

    fn append_text(_this: &Self, string_builder: &mut String) {
        string_builder.push_str(NEW_LINE);
    }
}

impl LineBreak {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Inline::construct() }
    }

    /// Creates a new `LineBreak` instance.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
