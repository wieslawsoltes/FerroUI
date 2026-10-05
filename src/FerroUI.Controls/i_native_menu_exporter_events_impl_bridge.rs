/// The events of a native menu, raised by the platform exporter of the menu.
///
/// A native menu is viewed through this contract with
/// `menu.to_exporter_events_bridge()`.
pub trait INativeMenuExporterEventsImplBridge {
    fn raise_needs_update(&self);
    fn raise_opening(&self);
    fn raise_closed(&self);
}
