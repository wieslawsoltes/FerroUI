//! Port of `ViewLocator.cs` of the ReactiveUI demo: the generated data template and view locator
//! of ReactiveUI, with a fallback hook for a view model wrapped in a [`ContextHost`].

use crate::view_models::reactive::ContextHost;
use crate::view_models::ViewModelBase;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{ContentControl, Control};
use ferroui_static_view_locator::static_view_locator;

/// The locator of the page of ReactiveUI.
#[static_view_locator(
    generate_view_locator = true,
    generate_data_template = true,
    generate_runtime_type_fallback_methods = false,
    data_template_match_types = [ViewModelBase, ContextHost],
)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ReactiveViewLocator;

impl ReactiveViewLocator {
    /// The view of a [`ContextHost`]: a content control whose content is the view model the host
    /// holds, which this locator shows in turn (`BuildFallbackView`, declared by the locator; the
    /// generator leaves out its default).
    fn build_fallback_view(&self, param: Option<&BoxedValue>) -> Option<Ref<Control>> {
        let host = param?.downcast_ref::<ContextHost>()?;
        let content_control = ContentControl::new();
        content_control.set_data_context(param.cloned());
        content_control.set_content(Some(host.context().clone()));
        Some(content_control.upcast())
    }

    /// A [`ContextHost`] matches, though it has no view of its own (`MatchDataTemplate`, declared by
    /// the locator): the generated match accepts only view models with a view.
    fn match_data_template(&self, data: Option<&BoxedValue>) -> bool {
        data.is_some_and(|data| data.is::<ContextHost>()) || Self::try_get_resolved_view_factory(data).is_some()
    }
}
