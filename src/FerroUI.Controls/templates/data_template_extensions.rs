use super::IDataTemplate;
use crate::{Control, IGlobalDataTemplates};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref, StyledElement};
use std::rc::Rc;

impl Control {
    /// Find a data template that matches a piece of data.
    ///
    /// `primary` is an optional primary template that can display the data.
    /// Returns the data template or `None` if no matching data template was
    /// found.
    pub fn find_data_template(
        &self,
        data: Option<&BoxedValue>,
        primary: Option<&Rc<dyn IDataTemplate>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        if let Some(primary) = primary {
            if primary.match_(data) {
                return Some(primary.clone());
            }
        }

        let mut current_template_host: Option<Ref<StyledElement>> = Some(self.to_ref().upcast());

        while let Some(current) = current_template_host {
            if let Some(host_candidate) = current.downcast_ref::<Control>() {
                if host_candidate.is_data_templates_initialized() {
                    for dt in host_candidate.data_templates().snapshot().iter() {
                        if dt.match_(data) {
                            return Some(dt.clone());
                        }
                    }
                }
            }

            current_template_host = current.parent();
        }

        let global = FerroLocator::current().get_service::<dyn IGlobalDataTemplates>();

        if let Some(global) = global {
            if global.is_data_templates_initialized() {
                for dt in global.data_templates().snapshot().iter() {
                    if dt.match_(data) {
                        return Some(dt.clone());
                    }
                }
            }
        }

        None
    }
}
