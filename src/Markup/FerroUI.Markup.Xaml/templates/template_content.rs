//! Port of `Templates/TemplateContent.cs`.

use crate::object_casts::rc_of;
use crate::xaml_il::runtime::DeferredContent;
use crate::XamlLoadException;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::templates::TemplateResult;
use ferroui_controls::Control;

/// Builds the content of templates.
pub struct TemplateContent;

impl TemplateContent {
    /// Builds template content whose result is a control.
    pub fn load(template_content: Option<&BoxedValue>) -> Option<TemplateResult<Ref<Control>>> {
        Self::load_as::<Ref<Control>>(template_content)
    }

    /// Builds template content (the value of a `Content` property marked as
    /// template content) whose result is a `T`: the result and the name
    /// scope its named elements were registered in.
    ///
    /// Returns `None` for no content, and for content that built nothing.
    ///
    /// The content is the deferred content object the runtime helpers
    /// create ([`DeferredContent`]). The delegate form of the managed
    /// original has no counterpart: a function is not an untyped value.
    ///
    /// # Panics
    /// Panics if the content is anything else, or if what it built is not a
    /// `T`.
    pub fn load_as<T: Clone + 'static>(template_content: Option<&BoxedValue>) -> Option<TemplateResult<T>> {
        crate::throw(Self::try_load_as(template_content))
    }

    /// [`load_as`](Self::load_as) without panics: unexpected content and a
    /// result of another type are errors. This is what untyped (metadata)
    /// callers use.
    pub fn try_load_as<T: Clone + 'static>(
        template_content: Option<&BoxedValue>,
    ) -> Result<Option<TemplateResult<T>>, XamlLoadException> {
        let Some(template_content) = template_content else { return Ok(None) };
        let Some(deferred) = rc_of::<DeferredContent>(template_content) else {
            return Err(XamlLoadException::with_message(format!(
                "Unexpected content {} (Parameter 'templateContent')",
                (**template_content).type_name()
            )));
        };

        let built = deferred.try_build_with(None)?;
        if built.result.is_none() {
            return Ok(None);
        }
        match from_markup_value::<T>(&built.result) {
            Some(result) => Ok(Some(TemplateResult::new(result, built.name_scope))),
            None => Err(XamlLoadException::with_message(format!(
                "Unable to cast object of type '{}' to type '{}'.",
                built.result.as_ref().map_or("null", |r| (**r).type_name()),
                std::any::type_name::<T>()
            ))),
        }
    }
}

ferro_markup_type!(static TemplateContent {
    methods: [
        static try fn Load(Option<BoxedValue>) -> Option<Ref<Control>> => |template_content: Option<BoxedValue>| {
            TemplateContent::try_load_as::<Ref<Control>>(template_content.as_ref())
                .map(|result| result.map(|result| result.result().clone()))
        },
    ],
});
