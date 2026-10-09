use ferroui_base::{ferro_class, ferro_properties, Control, FerroProperty, StyledProperty};

pub struct TextBlock {
    base: Control,
}

ferro_class!(TextBlock: Control);

ferro_properties! {
    impl TextBlock {
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TextBlock, _>("Text", None)
        }
    }
}

pub mod documents {
    use ferroui_base::{ferro_class, Control};

    pub struct Run {
        base: Control,
    }

    ferro_class!(Run: Control);

    #[cfg(feature = "inlines")]
    mod gated {
        use super::Run;
        use ferroui_base::ferro_class;

        pub struct Span {
            base: Run,
        }

        ferro_class!(Span: Run);
    }
}
