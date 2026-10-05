use std::io;

/// The platform implementation of a window icon.
pub trait IWindowIconImpl {
    /// Writes the icon to a stream.
    fn save(&self, output_stream: &mut dyn io::Write) -> io::Result<()>;
}
