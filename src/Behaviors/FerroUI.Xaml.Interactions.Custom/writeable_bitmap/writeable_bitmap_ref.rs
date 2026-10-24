//! Not from the reference: the handle of a writeable bitmap that the `Bitmap` properties of the
//! writeable bitmap behaviors, triggers and actions hold.
//!
//! Deviation (DEVIATIONS.md, Behaviors): a property value is cloned and compares with `==`; a
//! [`WriteableBitmap`] of the framework is neither, so the properties typed `WriteableBitmap` in
//! the reference hold this shared handle, which compares by identity as the reference compares
//! its object references.

use ferroui_base::media::imaging::WriteableBitmap;
use ferroui_base::media::IImage;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// A writeable bitmap held by a property.
#[derive(Clone)]
pub struct WriteableBitmapRef(pub Rc<WriteableBitmap>);

impl WriteableBitmapRef {
    /// The handle of `bitmap`.
    pub fn new(bitmap: WriteableBitmap) -> Self {
        Self(Rc::new(bitmap))
    }

    /// The shared bitmap.
    pub fn get(&self) -> &Rc<WriteableBitmap> {
        &self.0
    }

    /// The bitmap as an image (the source of an image control).
    pub fn as_image(&self) -> Rc<dyn IImage> {
        self.0.clone()
    }
}

impl Deref for WriteableBitmapRef {
    type Target = WriteableBitmap;

    fn deref(&self) -> &WriteableBitmap {
        &self.0
    }
}

impl PartialEq for WriteableBitmapRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl From<Rc<WriteableBitmap>> for WriteableBitmapRef {
    fn from(value: Rc<WriteableBitmap>) -> Self {
        Self(value)
    }
}

impl fmt::Debug for WriteableBitmapRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WriteableBitmapRef({:p})", Rc::as_ptr(&self.0))
    }
}
