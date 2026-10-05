//! The brushes of the composition renderer: the server-side counterparts of
//! the mutable tile brushes.

mod server_simple_content_brush;
mod server_simple_image_brush;

pub use server_simple_content_brush::ServerCompositionSimpleContentBrush;
pub use server_simple_image_brush::ServerCompositionSimpleImageBrush;
