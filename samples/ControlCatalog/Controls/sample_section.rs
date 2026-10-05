//! Port of `Controls/SampleSection.cs`.

use super::CodeLanguage;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_properties, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::{HeaderedContentControl, TemplatedControlImpl};
use ferroui_controls::{ContentControlImpl, ControlImpl, ControlImplExt, SizeChangedEventArgs};

/// Framing options for a [`SampleSection`] stage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SampleStage {
    #[default]
    Padded = 0,
    Flush = 1,
    None = 2,
}

ferro_markup_enum!(SampleStage { Padded, Flush, None });

const PC_HAS_OPTIONS: &str = ":has-options";
const PC_HAS_CODE: &str = ":has-code";
const PC_NARROW: &str = ":narrow";
const PC_STAGE_FLUSH: &str = ":stage-flush";
const PC_STAGE_NONE: &str = ":stage-none";
const PC_FIXED_STAGE: &str = ":fixed-stage";

/// Below this width the options panel moves under the example instead of
/// beside it.
const NARROW_THRESHOLD: f64 = 620.0;

/// One example on a `SamplePage`: a card with a title (`Header`), a
/// description, the live example as content, and an optional options panel
/// that sits beside the example on wide layouts and below it on narrow
/// ones.
#[repr(C)]
pub struct SampleSection {
    base: HeaderedContentControl,
}

ferro_class!(SampleSection: HeaderedContentControl);
ferro_class_info!(SampleSection {
    new: SampleSection::new,
    markup: {
        attributes: [PseudoClasses(
            ":has-options",
            ":has-code",
            ":narrow",
            ":stage-flush",
            ":stage-none",
            ":fixed-stage"
        )],
    },
});
ferro_impl_classes!(
    SampleSection: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for SampleSection {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::options_property().as_property() {
            this.pseudo_classes().set(PC_HAS_OPTIONS, change.get_new_value::<Option<BoxedValue>>().is_some());
        } else if change.property() == Self::code_property().as_property() {
            let code = change.get_new_value::<Option<String>>();
            this.pseudo_classes().set(PC_HAS_CODE, code.is_some_and(|code| !code.trim().is_empty()));
        } else if change.property() == Self::stage_property().as_property() {
            let stage = change.get_new_value::<SampleStage>();
            this.pseudo_classes().set(PC_STAGE_FLUSH, stage == SampleStage::Flush);
            this.pseudo_classes().set(PC_STAGE_NONE, stage == SampleStage::None);
        } else if change.property() == Self::stage_height_property().as_property() {
            this.pseudo_classes().set(PC_FIXED_STAGE, !change.get_new_value::<f64>().is_nan());
        }
    }
}

impl ControlImpl for SampleSection {
    fn on_size_changed(this: &Self, e: &SizeChangedEventArgs) {
        Self::parent_on_size_changed(this, e);

        if e.width_changed() {
            this.pseudo_classes().set(PC_NARROW, e.new_size().width < NARROW_THRESHOLD);
        }
    }
}

ferro_properties! {
    impl SampleSection {
        pub fn description_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SampleSection, _>("Description", None)
        }

        pub fn options_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<SampleSection, _>("Options", None)
        }

        pub fn code_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SampleSection, _>("Code", None)
        }

        pub fn code_language_property() -> StyledProperty<CodeLanguage> {
            FerroProperty::register::<SampleSection, _>("CodeLanguage", CodeLanguage::Xaml)
        }

        pub fn stage_property() -> StyledProperty<SampleStage> {
            FerroProperty::register::<SampleSection, _>("Stage", SampleStage::Padded)
        }

        pub fn stage_min_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<SampleSection, _>("StageMinHeight", 72.0)
        }

        pub fn stage_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<SampleSection, _>("StageHeight", f64::NAN)
        }
    }
}

impl SampleSection {
    pub fn construct() -> Self {
        Self { base: HeaderedContentControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// What this example shows, in one sentence.
    pub fn description(&self) -> Option<String> {
        self.get_value(Self::description_property())
    }

    pub fn set_description(&self, value: Option<&str>) {
        self.set_value(Self::description_property(), value.map(str::to_string))
    }

    /// Controls that configure the example. Leave unset when the example
    /// has nothing to configure.
    pub fn options(&self) -> Option<BoxedValue> {
        self.get_value(Self::options_property())
    }

    pub fn set_options(&self, value: Option<BoxedValue>) {
        self.set_value(Self::options_property(), value)
    }

    /// A short snippet shown under the example, collapsed until the reader
    /// opens it. Keep it to the lines that matter rather than reproducing
    /// the whole sample.
    pub fn code(&self) -> Option<String> {
        self.get_value(Self::code_property())
    }

    pub fn set_code(&self, value: Option<&str>) {
        self.set_value(Self::code_property(), value.map(str::to_string))
    }

    /// The language `Code` is highlighted as. Defaults to XAML.
    pub fn code_language(&self) -> CodeLanguage {
        self.get_value(Self::code_language_property())
    }

    pub fn set_code_language(&self, value: CodeLanguage) {
        self.set_value(Self::code_language_property(), value)
    }

    /// How the example is framed. `Padded` is the default card stage; use
    /// `Flush` for content that paints its own edges and `None` for content
    /// that must sit directly on the card.
    pub fn stage(&self) -> SampleStage {
        self.get_value(Self::stage_property())
    }

    pub fn set_stage(&self, value: SampleStage) {
        self.set_value(Self::stage_property(), value)
    }

    /// Minimum height of the stage, for examples that need room to be
    /// understood.
    pub fn stage_min_height(&self) -> f64 {
        self.get_value(Self::stage_min_height_property())
    }

    pub fn set_stage_min_height(&self, value: f64) {
        self.set_value(Self::stage_min_height_property(), value)
    }

    /// Fixed stage height, for content with no natural height: a demo app,
    /// a carousel, a GL surface.
    pub fn stage_height(&self) -> f64 {
        self.get_value(Self::stage_height_property())
    }

    pub fn set_stage_height(&self, value: f64) {
        self.set_value(Self::stage_height_property(), value)
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use std::rc::Rc;

    #[test]
    fn pseudo_classes_follow_the_properties() {
        let section = SampleSection::new();
        assert!(!section.classes().contains(PC_HAS_CODE));

        section.set_code(Some("  \n "));
        assert!(!section.classes().contains(PC_HAS_CODE));
        section.set_code(Some("<Button />"));
        assert!(section.classes().contains(PC_HAS_CODE));

        section.set_options(Some(Rc::new(1_i32) as BoxedValue));
        assert!(section.classes().contains(PC_HAS_OPTIONS));
        section.set_options(None);
        assert!(!section.classes().contains(PC_HAS_OPTIONS));

        section.set_stage(SampleStage::Flush);
        assert!(section.classes().contains(PC_STAGE_FLUSH));
        section.set_stage(SampleStage::None);
        assert!(!section.classes().contains(PC_STAGE_FLUSH));
        assert!(section.classes().contains(PC_STAGE_NONE));

        section.set_stage_height(120.0);
        assert!(section.classes().contains(PC_FIXED_STAGE));
        section.set_stage_height(f64::NAN);
        assert!(!section.classes().contains(PC_FIXED_STAGE));
    }
}
