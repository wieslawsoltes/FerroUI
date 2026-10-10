//! Port of upstream's `CrossUI` directory of the render tests: a small
//! description of a scene that does not depend on the framework
//! (`cross_ui`), and its implementation over the framework
//! (`cross_ui_ferro`).

#[allow(clippy::module_inception)]
pub mod cross_ui;
pub mod cross_ui_ferro;

pub use cross_ui::*;
pub use cross_ui_ferro::*;
