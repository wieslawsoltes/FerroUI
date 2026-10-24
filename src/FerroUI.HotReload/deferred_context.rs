use crate::{PreparedReload, ReloadError};
use ferroui_base::{BoxedValue, Ref, StyledElement};
use ferroui_markup_xaml::xaml_il::runtime::{
    DeferredContentRebase, DeferredContentReloadScope,
};
use std::rc::Rc;
use ferroui_markup_xaml::reload_resources::{DynamicResourceRebase, DynamicResourceReloadScope};

pub(crate) fn prepare_dynamic(
    scope: &DynamicResourceReloadScope,
    candidate: &Ref<StyledElement>,
    root: &Ref<StyledElement>,
) -> Box<dyn PreparedReload> {
    Box::new(DynamicChange(scope.prepare_rebase(candidate, root)))
}
struct DynamicChange(DynamicResourceRebase);
impl PreparedReload for DynamicChange {
    fn commit(&mut self) -> Result<(), ReloadError> {
        self.0.commit();
        Ok(())
    }
    fn rollback(&mut self) {
        self.0.rollback();
    }
}

pub(crate) fn prepare(
    scope: &DeferredContentReloadScope,
    candidate: &Ref<StyledElement>,
    root: &Ref<StyledElement>,
    resources: bool,
    styles: bool,
) -> Box<dyn PreparedReload> {
    let mut mappings: Vec<(BoxedValue, BoxedValue)> =
        vec![(Rc::new(candidate.clone()), Rc::new(root.clone()))];
    if resources {
        mappings.push((Rc::new(candidate.resources()), Rc::new(root.resources())));
    }
    if styles {
        mappings.push((Rc::new(candidate.styles()), Rc::new(root.styles())));
        mappings.push((Rc::new(candidate.styles().resources()), Rc::new(root.styles().resources())));
    }
    Box::new(Change(scope.prepare_rebase(&mappings)))
}
struct Change(DeferredContentRebase);
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        self.0.commit();
        Ok(())
    }
    fn rollback(&mut self) {
        self.0.rollback();
    }
}
