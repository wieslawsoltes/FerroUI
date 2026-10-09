use super::{Inline, InlineImpl, TextElementImpl};
use ferroui_base::data::BindingMode;
use ferroui_base::media::text_formatting::{TextCharacters, TextRun};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use std::rc::Rc;

/// A terminal element in text flow hierarchy - contains a uniformatted run
/// of unicode characters.
#[repr(C)]
pub struct Run {
    base: Inline,
}

ferro_class!(Run: Inline);
ferroui_base::ferro_class_info!(Run { new: Run::new });

ferroui_base::ferro_impl_classes!(Run: StyledElementImpl);
ferroui_base::ferro_impl_classes!(Run: TextElementImpl);

impl FerroObjectImpl for Run {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property().name() == "Text" {
            if let Some(host) = this.inline_host() {
                host.invalidate();
            }
        }
    }
}

impl InlineImpl for Run {
    fn build_text_run(this: &Self, text_runs: &mut Vec<Rc<dyn TextRun>>) {
        let text = this.text().unwrap_or_default();

        if text.is_empty() {
            return;
        }

        let text_run_properties = this.create_text_run_properties();

        let text_characters = TextCharacters::from_str(&text, text_run_properties);

        text_runs.push(Rc::new(text_characters));
    }

    fn append_text(this: &Self, string_builder: &mut String) {
        let text = this.text().unwrap_or_default();

        string_builder.push_str(&text);
    }
}

ferroui_base::ferro_properties! { impl Run {
    ferro_property!(
        /// Dependency property backing `Text`.
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register_with::<Run, _>(
                "Text",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );
} }

impl Run {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Inline::construct() }
    }

    /// Initializes an instance of the `Run` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Initializes an instance of the `Run` class specifying its text
    /// content.
    pub fn with_text(text: Option<&str>) -> Ref<Self> {
        let run = Self::new();
        run.set_text(text);
        run
    }

    /// The content spanned by this text pointer.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(str::to_owned))
    }
}
