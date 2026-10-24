//! Port of `Diagnostics/Controls/FilterTextBox.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions,
    TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, TextBox, TextBoxImpl};

/// The text box of the filters of the developer tools: a text box styled as
/// one (class `filter-text-box`), with the toggles of the options of the
/// filter (match case, match whole word, use a regular expression) in its
/// inner right content (`FilterTextBox.xaml`).
#[repr(C)]
pub struct FilterTextBox {
    base: TextBox,
}

ferro_class!(FilterTextBox: TextBox);
ferro_class_info!(FilterTextBox { new: FilterTextBox::new });
ferro_impl_classes!(
    FilterTextBox: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    TextBoxImpl
);

impl FerroObjectImpl for FilterTextBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.classes().add("filter-text-box");
    }
}

impl StyledElementImpl for FilterTextBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TextBox as StaticType>::TYPE
    }
}

ferro_properties! { impl FilterTextBox {
    ferro_property!(
        /// Defines the `UseRegexFilter` property.
        pub fn use_regex_filter_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<FilterTextBox, _>(
                "UseRegexFilter",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `UseCaseSensitiveFilter` property.
        pub fn use_case_sensitive_filter_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<FilterTextBox, _>(
                "UseCaseSensitiveFilter",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `UseWholeWordFilter` property.
        pub fn use_whole_word_filter_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<FilterTextBox, _>(
                "UseWholeWordFilter",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );
} }

impl FilterTextBox {
    pub fn construct() -> Self {
        Self { base: TextBox::construct() }
    }

    /// C# `new FilterTextBox()`.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn use_regex_filter(&self) -> bool {
        self.get_value(Self::use_regex_filter_property())
    }

    pub fn set_use_regex_filter(&self, value: bool) {
        self.set_value(Self::use_regex_filter_property(), value)
    }

    pub fn use_case_sensitive_filter(&self) -> bool {
        self.get_value(Self::use_case_sensitive_filter_property())
    }

    pub fn set_use_case_sensitive_filter(&self, value: bool) {
        self.set_value(Self::use_case_sensitive_filter_property(), value)
    }

    pub fn use_whole_word_filter(&self) -> bool {
        self.get_value(Self::use_whole_word_filter_property())
    }

    pub fn set_use_whole_word_filter(&self, value: bool) {
        self.set_value(Self::use_whole_word_filter_property(), value)
    }
}
