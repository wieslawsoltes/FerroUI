//! The OpenGL pages of the catalog (namespace `ControlCatalog.Pages.OpenGl`):
//! one module per upstream file.

use crate::markup::XamlClass;
use ferroui_base::TypeInfo;

mod gl_page_knobs;

pub use gl_page_knobs::GlPageKnobs;

/// The classes of the files under `Pages/OpenGl/`.
pub(crate) const TYPES: &[&TypeInfo] = &[GlPageKnobs::TYPE];

pub(crate) const CLASSES: &[&XamlClass] = &[&GlPageKnobs::XAML_CLASS];
