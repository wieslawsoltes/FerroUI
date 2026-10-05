use crate::media::IExperimentalAcrylicMaterial;
use std::rc::Rc;

/// Represents a mutable material which can return an immutable clone of
/// itself.
pub trait IMutableExperimentalAcrylicMaterial: IExperimentalAcrylicMaterial {
    /// Creates an immutable clone of the material.
    fn to_immutable(&self) -> Rc<dyn IExperimentalAcrylicMaterial>;
}
