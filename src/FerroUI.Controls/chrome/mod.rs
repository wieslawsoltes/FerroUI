//! Drawn (client-side) window decorations.

mod drawn_window_decoration_parts;
mod i_window_drawn_decorations_template;
mod resize_grip_layer;
mod title_bar_decorations;
mod window_decoration_properties;
mod window_drawn_decorations;
mod window_drawn_decorations_content;

pub(crate) use drawn_window_decoration_parts::DrawnWindowDecorationParts;
pub use i_window_drawn_decorations_template::IWindowDrawnDecorationsTemplate;
pub(crate) use resize_grip_layer::ResizeGripLayer;
pub use title_bar_decorations::TitleBarDecorations;
pub use window_decoration_properties::WindowDecorationProperties;
pub use window_drawn_decorations::WindowDrawnDecorations;
pub use window_drawn_decorations_content::WindowDrawnDecorationsContent;
