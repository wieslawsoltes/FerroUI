//! `CombinedAdaptersBuildConventionOnlyViews`: the generated data template builds the view of a
//! view model found by the conventions, with the view model as its data context; the generated
//! view locator of ReactiveUI finds no view of ReactiveUI for it (the view does not state its view
//! model).

use crate::prelude::*;

pub mod view_models {
    #[derive(PartialEq)]
    pub struct PlainViewModel;
}

pub mod views {
    use crate::prelude::*;

    test_view!(PlainView);
}

#[static_view_locator(generate_view_locator = true, generate_data_template = true, generate_runtime_type_fallback_methods = false)]
#[derive(Default)]
pub struct CombinedViewLocator;

#[cfg(test)]
mod tests {
    use super::view_models::PlainViewModel;
    use super::CombinedViewLocator;
    use crate::support::{class_name, start, value};
    use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
    use ferroui_reactiveui::IViewLocator;
    use std::any::Any;
    use std::rc::Rc;

    #[test]
    fn combined_adapters_build_convention_only_views() {
        let _app = start();
        let locator = CombinedViewLocator;
        let view_model = value(PlainViewModel);

        let view = locator.build(&Some(view_model.clone())).expect("a view");
        assert_eq!(class_name(&view), "PlainView");
        assert!(view.data_context().is_some_and(|context| Rc::ptr_eq(&context, &view_model)));
        assert!(locator.match_(Some(&view_model)));

        let reactive: Rc<dyn Any> = Rc::new(PlainViewModel);
        assert!(locator.resolve_view(Some(reactive), None).is_none());
    }
}
