/// The events of a native menu item, raised by the platform exporter of the
/// menu the item belongs to.
///
/// A native menu item is viewed through this contract with
/// `item.to_exporter_events_bridge()`.
pub trait INativeMenuItemExporterEventsImplBridge {
    fn raise_clicked(&self);
}
