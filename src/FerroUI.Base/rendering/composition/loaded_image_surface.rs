use super::server::{ServerLoadedImageSurface, ServerObjectId};
use super::{CompositionObject, Compositor, ICompositionObject, ICompositionSurface};
use crate::media::imaging::Bitmap;
use crate::Size;
use std::any::Any;
use std::io::Read;
use std::rc::Rc;

/// The outcome of loading a [`LoadedImageSurface`] (WinUI
/// `Microsoft.UI.Xaml.Media.LoadedImageSourceLoadStatus`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LoadedImageSourceLoadStatus {
    /// The image was loaded.
    #[default]
    Success,
    /// The image could not be read from its source.
    NetworkError,
    /// The data is not an image the platform can decode.
    InvalidFormat,
    /// Another error.
    Other,
}

/// A composition surface whose content is an image (WinUI
/// `Microsoft.UI.Xaml.Media.LoadedImageSurface`), here backed by a
/// [`Bitmap`].
///
/// The surface holds a reference to the platform bitmap; the server-side
/// surface holds another one until it is disposed.
///
/// Deviation (winui-composition.md): WinUI loads the image asynchronously
/// from a URI or a stream and raises `LoadCompleted`; here a surface is made
/// from a bitmap, or loaded from a stream at once, and the outcome is
/// [`load_status`](Self::load_status). There is no `LoadCompleted` event and
/// no decode size.
pub struct LoadedImageSurface {
    object: CompositionObject,
    bitmap: Option<Bitmap>,
    load_status: LoadedImageSourceLoadStatus,
}

impl LoadedImageSurface {
    fn create(compositor: &Rc<Compositor>, bitmap: Option<Bitmap>, status: LoadedImageSourceLoadStatus) -> Rc<Self> {
        let server_bitmap = bitmap.as_ref().map(|bitmap| bitmap.platform_impl().clone_ref());
        let server = compositor.create_server_object(move |_, _| ServerLoadedImageSurface::new(server_bitmap));
        Rc::new(LoadedImageSurface {
            object: CompositionObject::new(compositor, Some(server)),
            bitmap,
            load_status: status,
        })
    }

    /// A surface that shows `bitmap`. The surface takes another reference to
    /// the platform bitmap: disposing `bitmap` afterwards does not affect it.
    pub fn from_bitmap(compositor: &Rc<Compositor>, bitmap: &Bitmap) -> Rc<LoadedImageSurface> {
        Self::create(compositor, Some(Bitmap::from_ref(bitmap.platform_impl())), LoadedImageSourceLoadStatus::Success)
    }

    /// `StartLoadFromStream`: a surface with the image decoded from
    /// `stream`. The image is decoded at once; a stream that cannot be read
    /// or decoded gives an empty surface whose
    /// [`load_status`](Self::load_status) says why.
    pub fn start_load_from_stream(compositor: &Rc<Compositor>, stream: &mut dyn Read) -> Rc<LoadedImageSurface> {
        match Bitmap::from_stream(stream) {
            Ok(bitmap) => Self::create(compositor, Some(bitmap), LoadedImageSourceLoadStatus::Success),
            Err(error) => {
                let status = match error.kind() {
                    std::io::ErrorKind::InvalidData | std::io::ErrorKind::Unsupported => {
                        LoadedImageSourceLoadStatus::InvalidFormat
                    }
                    std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::PermissionDenied
                    | std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::TimedOut => LoadedImageSourceLoadStatus::NetworkError,
                    _ => LoadedImageSourceLoadStatus::Other,
                };
                Self::create(compositor, None, status)
            }
        }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side surface.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    /// The outcome of the load.
    pub fn load_status(&self) -> LoadedImageSourceLoadStatus {
        self.load_status
    }

    /// The bitmap the surface shows, `None` when the image did not load or
    /// the surface is disposed.
    pub fn bitmap(&self) -> Option<&Bitmap> {
        self.bitmap.as_ref().filter(|bitmap| bitmap.platform_impl().is_alive())
    }

    /// The size of the image in device independent pixels (`NaturalSize`):
    /// what a surface brush lays out. Empty when the image did not load.
    pub fn natural_size(&self) -> Size {
        self.bitmap().map(|bitmap| bitmap.size()).unwrap_or_default()
    }

    /// The size of the decoded image in device independent pixels
    /// (`DecodedSize`): the natural size, as the image is decoded at its
    /// full size.
    pub fn decoded_size(&self) -> Size {
        self.natural_size()
    }

    /// The size of the decoded image in pixels (`DecodedPhysicalSize`).
    pub fn decoded_physical_size(&self) -> Size {
        self.bitmap()
            .map(|bitmap| {
                let size = bitmap.pixel_size();
                Size::new(size.width as f64, size.height as f64)
            })
            .unwrap_or_default()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    /// Releases the surface: its server side releases the bitmap with the
    /// next batch.
    pub fn dispose(&self) {
        if let Some(bitmap) = &self.bitmap {
            bitmap.dispose();
        }
        self.object.dispose();
    }
}

impl ICompositionObject for LoadedImageSurface {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl ICompositionSurface for LoadedImageSurface {}
