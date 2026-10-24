use crate::{ITextViewConnect, TextArea, VisualLineError};
use ferroui_base::media::DrawingContext;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

/// Native editor drawing order, from the background to the blinking caret.
/// Renderers draw immediately before the content of their selected layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KnownLayer {
    #[default]
    Background,
    Selection,
    Text,
    Caret,
}

/// Draws noninteractive editor decorations using the native drawing context.
///
/// Collection membership supplies the view connection hooks. Retain a weak
/// view reference from those hooks when the renderer needs later invalidation.
/// Changing renderer state requires the host's layer invalidation method.
pub trait IBackgroundRenderer: ITextViewConnect {
    fn layer(&self) -> KnownLayer;
    fn draw(
        &self,
        text_view: &TextArea,
        context: &mut DrawingContext,
    ) -> Result<(), VisualLineError>;
}

/// Shared renderer identity for native observable collections. Equality never
/// invokes application code and does not depend on mutable renderer settings.
#[derive(Clone)]
pub struct BackgroundRendererRef(pub Rc<dyn IBackgroundRenderer>);
impl BackgroundRendererRef {
    pub fn new(renderer: impl IBackgroundRenderer + 'static) -> Self {
        Self(Rc::new(renderer))
    }
}
impl PartialEq for BackgroundRendererRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for BackgroundRendererRef {}
impl fmt::Debug for BackgroundRendererRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BackgroundRendererRef")
            .field(&Rc::as_ptr(&self.0))
            .finish()
    }
}
impl Deref for BackgroundRendererRef {
    type Target = dyn IBackgroundRenderer;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl<T: IBackgroundRenderer + 'static> From<Rc<T>> for BackgroundRendererRef {
    fn from(value: Rc<T>) -> Self {
        Self(value)
    }
}
impl From<Rc<dyn IBackgroundRenderer>> for BackgroundRendererRef {
    fn from(value: Rc<dyn IBackgroundRenderer>) -> Self {
        Self(value)
    }
}
