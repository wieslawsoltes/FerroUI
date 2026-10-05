use crate::data::core::IndexerBindingExpression;
use crate::data::{BindingBase, BindingExpressionBase, BindingMode};
use crate::{FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// A binding to a registered property of another object: what the binding
/// indexer of [`FerroObject`] ([`FerroObject::indexer`]) produces.
#[derive(Clone)]
pub struct IndexerBinding {
    source: Ref<FerroObject>,
    /// The property of the source being bound.
    pub property: &'static FerroProperty,
    mode: BindingMode,
}

impl IndexerBinding {
    pub fn new(source: Ref<FerroObject>, property: &'static FerroProperty, mode: BindingMode) -> Self {
        Self { source, property, mode }
    }

    pub fn source(&self) -> &Ref<FerroObject> {
        &self.source
    }

    pub fn mode(&self) -> BindingMode {
        self.mode
    }
}

impl BindingBase for IndexerBinding {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        IndexerBindingExpression::new(&self.source, self.property, target, target_property, self.mode)
    }
}
