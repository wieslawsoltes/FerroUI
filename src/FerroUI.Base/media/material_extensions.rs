use crate::media::IExperimentalAcrylicMaterial;
use std::rc::Rc;

/// Extension methods for material classes.
pub struct MaterialExtensions;

impl MaterialExtensions {
    /// Converts a material to an immutable material: the result of
    /// [`IMutableExperimentalAcrylicMaterial::to_immutable`](crate::media::IMutableExperimentalAcrylicMaterial::to_immutable)
    /// if the material is mutable, otherwise the material itself.
    pub fn to_immutable(material: &Rc<dyn IExperimentalAcrylicMaterial>) -> Rc<dyn IExperimentalAcrylicMaterial> {
        match material.as_mutable_material() {
            Some(mutable) => mutable.to_immutable(),
            None => material.clone(),
        }
    }
}
