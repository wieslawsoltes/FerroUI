use super::{IRecyclingDataTemplate, ITemplateWithParam, ITreeDataTemplate, ITypedDataTemplate};
use crate::Control;
use ferroui_base::{diagnostics::RsxSourceInfo, BoxedValue, Ref};
use std::panic::Location;

/// Provenance reported by a Rust template constructed with editor diagnostics enabled.
#[derive(Clone, Copy, Debug)]
pub struct RustDataTemplateSource {
    /// The caller that constructed the template handle.
    pub constructor: &'static Location<'static>,
    /// The compiled source of the RSX invocation observed when rendering an item.
    pub rendered_body: Option<RsxSourceInfo>,
}

impl RustDataTemplateSource {
    /// The compiled evidence for the constructor's file, from a control or an
    /// observed template body in that file.
    pub fn verified_constructor(&self, owner: Option<RsxSourceInfo>) -> Option<RsxSourceInfo> {
        [owner, self.rendered_body].into_iter().flatten().find_map(|proof| {
            let path = RsxSourceInfo::resolve_file(self.constructor.file(), proof.manifest)?;
            let same = path == RsxSourceInfo::resolve_file(proof.file, proof.manifest)?;
            (same && proof.compiled.revision.is_some()).then_some(proof)
        })
    }
}

/// Interface representing a template used to build a control for a piece of
/// data.
pub trait IDataTemplate: ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> {
    /// Checks to see if this data template matches the specified data.
    fn match_(&self, data: Option<&BoxedValue>) -> bool;

    /// The template viewed as a recycling data template, if it is one.
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        None
    }

    /// The template viewed as a typed data template, if it is one.
    fn as_typed_data_template(&self) -> Option<&dyn ITypedDataTemplate> {
        None
    }

    /// The template viewed as a tree data template, if it is one.
    fn as_tree_data_template(&self) -> Option<&dyn ITreeDataTemplate> {
        None
    }

    /// Source evidence for the actual template handle selected by the control.
    fn rust_editor_source(&self) -> Option<RustDataTemplateSource> {
        None
    }

    /// The implementing value, for casting to its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` for an implementation that does not expose itself.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

impl PartialEq for dyn IDataTemplate {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
