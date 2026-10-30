//! A sample-specific view builder adapter. Session sources remain the author's
//! XAML; the registered synchronous function derives candidate markup at prepare
//! time. It must be pure: arbitrary user side effects cannot be rolled back.
use ferroui_hot_reload::{
    ComponentReloadTarget, DocumentContract, PreparedReload, ReloadError, ReloadTarget,
};
use ferroui_markup_xaml::source_overlay::{with_sources, Sources};
use std::rc::Rc;

pub struct BuiltView {
    pub component: ComponentReloadTarget,
    pub uri: String,
    pub documents: Vec<String>,
}
impl ReloadTarget for BuiltView {
    fn documents(&self) -> Vec<String> {
        // This small sample owns all manifest documents. A derived include can
        // therefore never escape dependency invalidation for a registered file.
        self.documents.clone()
    }
    fn prepare(&self, sources: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
        let source = sources
            .get(&self.uri)
            .ok_or_else(|| ReloadError::Failed("Missing builder source".into()))?;
        let derived = crate::VIEW_BUILDER.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or_else(|| source.clone(), |build| build(source.clone()))
        });
        if DocumentContract::parse(source)? != DocumentContract::parse(&derived)? {
            return Err(ReloadError::RestartRequired(
                "View builder changed the component's compiled contract".into(),
            ));
        }
        let mut candidate = (*sources).clone();
        candidate.insert(self.uri.clone(), derived);
        let candidate = Rc::new(candidate);
        with_sources(candidate.clone(), || self.component.prepare(candidate))
    }
}
