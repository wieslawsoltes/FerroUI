//! `GeneratedReactiveUIAndDataTemplateAdaptersWorkAtRuntime`: the generated view locator of
//! ReactiveUI and data template, with a fallback hook and a match hook of the locator.

use crate::prelude::*;
use ferroui_controls::Border;

pub mod view_models {
    pub trait ViewModelBase {}

    #[derive(PartialEq)]
    pub struct DashboardViewModel;

    impl ViewModelBase for DashboardViewModel {}

    /// A view model that holds another (`ContextHost`).
    #[derive(PartialEq)]
    pub struct ContextHost {
        pub context: std::rc::Rc<DashboardViewModel>,
    }

    /// A view model of a generic type whose view states the instantiation it shows.
    #[derive(PartialEq)]
    pub struct GenericViewModel<T>(pub T);

    #[derive(PartialEq)]
    pub struct Item;
}

pub mod views {
    use crate::prelude::*;
    use super::view_models::{DashboardViewModel, GenericViewModel, Item};
    use ferroui_reactiveui_controls::IControlViewFor;
    use std::cell::RefCell;
    use std::rc::Rc;

    test_view!(DashboardScreen);

    ferroui_base::ferro_thread_local! {
        /// The view models of the dashboard screens, by the address of the screen (a field of the
        /// view upstream).
        static DASHBOARD_VIEW_MODELS: RefCell<Vec<(usize, Rc<DashboardViewModel>)>> = const { RefCell::new(Vec::new()) };
    }

    impl IControlViewFor for DashboardScreen {
        type ViewModel = DashboardViewModel;

        fn view_model_of(&self) -> Option<Rc<DashboardViewModel>> {
            let address = self as *const Self as usize;
            DASHBOARD_VIEW_MODELS.with(|view_models| view_models.borrow().iter().find(|(view, _)| *view == address).map(|(_, view_model)| view_model.clone()))
        }

        fn set_view_model_of(&self, view_model: Option<Rc<DashboardViewModel>>) {
            let address = self as *const Self as usize;
            DASHBOARD_VIEW_MODELS.with(|view_models| {
                let mut view_models = view_models.borrow_mut();
                view_models.retain(|(view, _)| *view != address);
                if let Some(view_model) = view_model {
                    view_models.push((address, view_model));
                }
            });
        }
    }

    test_view!(ClosedGenericScreen);

    impl IControlViewFor for ClosedGenericScreen {
        type ViewModel = GenericViewModel<Item>;

        fn view_model_of(&self) -> Option<Rc<GenericViewModel<Item>>> {
            None
        }

        fn set_view_model_of(&self, _view_model: Option<Rc<GenericViewModel<Item>>>) {}
    }
}

#[static_view_locator(
    generate_view_locator = true,
    generate_data_template = true,
    generate_runtime_type_fallback_methods = false,
    data_template_match_types = [view_models::ViewModelBase, view_models::ContextHost],
)]
#[derive(Default)]
pub struct IntegrationViewLocator;

impl IntegrationViewLocator {
    fn build_fallback_view(&self, param: Option<&BoxedValue>) -> Option<Ref<Control>> {
        param?.downcast_ref::<view_models::ContextHost>()?;
        let border = Border::new();
        let tag: BoxedValue = std::rc::Rc::new("context-fallback".to_string());
        border.set_tag(Some(tag));
        Some(border.upcast())
    }

    fn match_data_template(&self, data: Option<&BoxedValue>) -> bool {
        data.is_some_and(|data| data.is::<view_models::ContextHost>()) || Self::try_get_resolved_view_factory(data).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::view_models::{ContextHost, DashboardViewModel, GenericViewModel, Item};
    use super::views::{ClosedGenericScreen, DashboardScreen};
    use super::IntegrationViewLocator;
    use crate::support::{class_name, start, value};
    use ferroui_base::BoxedValue;
    use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
    use ferroui_controls::{Border, TextBlock};
    use ferroui_reactiveui::IViewLocator;
    use ferroui_reactiveui_controls::{view_control, IControlViewFor};
    use std::any::Any;
    use std::rc::Rc;

    #[test]
    fn generated_reactive_ui_and_data_template_adapters_work_at_runtime() {
        let _app = start();
        let locator = IntegrationViewLocator;
        let dashboard = Rc::new(DashboardViewModel);
        let dashboard_value: BoxedValue = dashboard.clone();
        let context_host = value(ContextHost { context: dashboard.clone() });

        // The locator is both adapters.
        let _: &dyn IDataTemplate = &locator;
        let _: &dyn IViewLocator = &locator;

        let view = locator.build(&Some(dashboard_value.clone())).expect("a view");
        assert_eq!(class_name(&view), "DashboardScreen");
        let screen = view.cast::<DashboardScreen>().expect("a dashboard screen");
        assert!(screen.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &dashboard)));

        let fallback = locator.build(&Some(context_host.clone())).expect("a view");
        let border = fallback.cast::<Border>().expect("a border");
        let tag = border.tag().expect("a tag");
        assert_eq!(tag.downcast_ref::<String>().map(String::as_str), Some("context-fallback"));

        assert!(locator.match_(Some(&dashboard_value)));
        assert!(locator.match_(Some(&context_host)));
        assert!(!locator.match_(Some(&value(1_i32))));
        assert!(!locator.match_(None));

        let instance: Rc<dyn Any> = dashboard.clone();
        let resolved = locator.resolve_view(Some(instance), None).expect("a view of ReactiveUI");
        let control = view_control(&*resolved).expect("the control of the view");
        let screen = control.cast::<DashboardScreen>().expect("a dashboard screen");
        assert!(screen.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &dashboard)));
        let resolved_view_model = resolved.view_model().expect("the view model of the view");
        assert!(Rc::ptr_eq(&resolved_view_model.downcast::<DashboardViewModel>().expect("a dashboard view model"), &dashboard));

        assert!(locator.resolve_view_default::<DashboardViewModel>().is_some());
        assert!(locator.resolve_view_for::<DashboardViewModel>(Some("other")).is_none());
        let generic = locator.resolve_view_default::<GenericViewModel<Item>>().expect("the view of the instantiation");
        assert!(view_control(&*generic).and_then(|control| control.cast::<ClosedGenericScreen>()).is_some());

        let invalid = locator.build(&None).expect("a view");
        let text = invalid.cast::<TextBlock>().expect("a text block");
        assert_eq!(text.text().as_deref(), Some("Invalid view model type."));
    }
}
