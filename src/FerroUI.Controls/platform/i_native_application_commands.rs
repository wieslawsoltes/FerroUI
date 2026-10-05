/// Application-level commands implemented by the native platform.
pub trait INativeApplicationCommands {
    fn show_app(&self);
    fn hide_app(&self);
    fn show_all(&self);
    fn hide_others(&self);
}
