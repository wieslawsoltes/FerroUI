use crate::TranslationBindingSource;
use ferroui_protranslate_core::{Culture, FormatArgument, TranslationError};

/// UI-thread-local convenience entry point shared by native XAML consumers.
/// Applications that need isolation can attach a source to a subtree instead.
pub struct TranslationService;
ferroui_base::ferro_static_type!(TranslationService);

impl TranslationService {
    pub fn source() -> TranslationBindingSource {
        TranslationBindingSource::default_source()
    }
    pub fn use_source(source: TranslationBindingSource) {
        TranslationBindingSource::use_default_source(source);
    }
    pub fn use_service(service: ferroui_protranslate_core::TranslationService) {
        let source = Self::source();
        if source.is_disposed() {
            Self::use_source(TranslationBindingSource::new(service));
        } else {
            source.set_service(service);
        }
    }
    pub fn culture() -> Culture {
        Self::source().culture()
    }
    pub fn formatting_culture() -> Culture {
        Self::source().formatting_culture()
    }
    pub fn set_culture(culture: Culture) -> bool {
        Self::source().set_culture(culture)
    }
    pub fn translate(key: &str) -> Result<String, TranslationError> {
        Self::source().translate(key, &[])
    }
    pub fn t(key: &str) -> Result<String, TranslationError> {
        Self::translate(key)
    }
    pub fn format(key: &str, arguments: &[FormatArgument]) -> Result<String, TranslationError> {
        Self::source().translate(key, arguments)
    }
}

ferroui_base::ferro_markup_type!(static TranslationService {
    type_info: TranslationService,
    static_properties: [
        Source: TranslationBindingSource { get: TranslationService::source },
        Culture: String {
            get: || TranslationService::culture().name().to_owned(),
            try_set: |name: String| Culture::parse(&name).map(|culture| { TranslationService::set_culture(culture); })
        },
        FormattingCulture: String { get: || TranslationService::formatting_culture().name().to_owned() },
    ],
    methods: [
        static try fn T(String) -> String => |key: String| TranslationService::t(&key),
        static try fn Translate(String) -> String => |key: String| TranslationService::translate(&key),
    ],
});
