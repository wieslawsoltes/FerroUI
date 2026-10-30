//! Original native docking delegates and contracts.
mod dock_operation_handler;
pub use dock_operation_handler::DockOperationHandler;

mod i_drag_offset_calculator;
pub use i_drag_offset_calculator::IDragOffsetCalculator;
mod i_external_dock_surface;
pub use i_external_dock_surface::IExternalDockSurface;
