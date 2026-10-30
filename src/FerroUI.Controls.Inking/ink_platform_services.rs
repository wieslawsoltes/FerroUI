use super::{InkError, InkStroke};
use ferroui_base::Point;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// Persisted representation. The portable format preserves FerroUI telemetry;
/// the other two formats use a platform service with genuine ISF support.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkPersistenceFormat {
    GifWithEmbeddedIsf = 0,
    Isf = 1,
    #[default]
    Portable = 2,
}

pub type InkServiceFuture<T> = Pin<Box<dyn Future<Output = Result<T, InkError>>>>;

/// Optional OS interoperability. The native Windows implementation uses the
/// Windows ink engine for ISF, embedded ISF/GIF and the system ink clipboard.
/// Core drawing, collection editing and portable persistence require no service.
pub trait IInkPlatformServices {
    /// Capture the supplied strokes' values before returning the future.
    /// Stroke handles remain editable by their owners while native I/O awaits.
    fn save(
        &self,
        strokes: Vec<Rc<InkStroke>>,
        format: InkPersistenceFormat,
    ) -> InkServiceFuture<Vec<u8>>;
    fn load(&self, data: Vec<u8>) -> InkServiceFuture<Vec<Rc<InkStroke>>>;
    fn copy_to_clipboard(&self, strokes: &[Rc<InkStroke>]) -> Result<(), InkError>;
    fn can_paste(&self) -> Result<bool, InkError>;
    fn paste(&self, position: Point) -> Result<Vec<Rc<InkStroke>>, InkError>;
}
