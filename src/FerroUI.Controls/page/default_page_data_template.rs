use super::{ContentPage, Page};
use crate::presenters::ContentPresenter;
use crate::templates::{IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam};
use crate::Control;
use ferroui_base::{BoxedValue, Ref};
use std::rc::Rc;

/// The data template that page hosts use by default: a page is presented as
/// itself, anything else as the content of a new content page.
pub(crate) struct DefaultPageDataTemplate;

impl DefaultPageDataTemplate {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    /// Detaches the control from any previous content presenter so that the
    /// new one can adopt it.
    fn detach_from_presenter(control: &Control) {
        let visual_parent = control.get_visual_parent().and_then(|parent| parent.cast::<ContentPresenter>());
        if let Some(visual_parent) = visual_parent {
            visual_parent.set_content(None);
        }
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for DefaultPageDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        let control = param.as_ref().and_then(Control::from_boxed);

        if let Some(control) = &control {
            if control.is::<Page>() {
                Self::detach_from_presenter(control);

                return Some(control.clone());
            }

            Self::detach_from_presenter(control);
        }

        let page = ContentPage::new();
        page.set_content(param.clone());
        Some(page.upcast())
    }
}

impl IDataTemplate for DefaultPageDataTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        data.is_some()
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for DefaultPageDataTemplate {
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        if let (Some(existing_page), Some(data)) =
            (existing.as_ref().and_then(|existing| existing.clone().cast::<ContentPage>()), data)
        {
            // Neither a page nor any other control.
            if Control::from_boxed(data).is_none() {
                existing_page.set_content(Some(data.clone()));
                return Some(existing_page.upcast());
            }
        }

        self.build(&data.cloned())
    }
}
