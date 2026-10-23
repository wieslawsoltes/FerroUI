use super::generated::{RedirectVisualHooks, RedirectVisualProps};
use super::server::ServerRedirectVisual;
use super::visual::CompositionVisualKind;
use super::{CompositionVisual, CompositionVisualSurface, Compositor, ICompositionObject};
use std::ops::Deref;
use std::rc::Rc;

impl RedirectVisualHooks for CompositionVisual {}

/// A visual that shows the subtree of another visual in its own place
/// (WinUI `Microsoft.UI.Composition.RedirectVisual`): the source keeps
/// rendering where it is, and the redirect visual draws the content and the
/// children of the source again, in its own coordinates, under its own
/// children.
///
/// The source is drawn as a [`CompositionVisualSurface`] draws it: its
/// content and children with their transforms, clips and opacities, the
/// visibility, opacity, transform and clip of the source itself ignored.
/// A redirect visual follows the changes of the source subtree while the
/// source is attached to a composition target. A redirect visual inside the
/// subtree of its own source draws nothing (it would contain itself).
///
/// Like a sprite visual, a redirect visual does not clip to its bounds when
/// it is created (`clip_to_bounds` is `false`).
#[derive(Clone)]
pub struct RedirectVisual(Rc<CompositionVisual>);

impl Deref for RedirectVisual {
    type Target = Rc<CompositionVisual>;

    fn deref(&self) -> &Rc<CompositionVisual> {
        &self.0
    }
}

impl RedirectVisual {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> RedirectVisual {
        let visual = CompositionVisual::create(
            compositor,
            CompositionVisualKind::Redirect(RedirectVisualProps::new()),
            || Box::new(ServerRedirectVisual::new()),
        );
        // A WinUI visual has no implicit clip.
        visual.set_clip_to_bounds(false);
        let this = RedirectVisual(visual);
        // The hidden visual surface that renders the source and follows its
        // changes on the server.
        let surface = CompositionVisualSurface::new(compositor);
        this.props().set_source_surface(&*this.0, Some(surface as Rc<dyn ICompositionObject>));
        this
    }

    pub(crate) fn with_source(compositor: &Rc<Compositor>, source: Option<Rc<CompositionVisual>>) -> RedirectVisual {
        let visual = Self::new(compositor);
        visual.set_source(source);
        visual
    }

    /// The handle of `visual` as a redirect visual, if it is one.
    pub fn from_visual(visual: &Rc<CompositionVisual>) -> Option<RedirectVisual> {
        matches!(visual.kind, CompositionVisualKind::Redirect(_)).then(|| RedirectVisual(visual.clone()))
    }

    /// The visual as a composition visual.
    pub fn visual(&self) -> &Rc<CompositionVisual> {
        &self.0
    }

    fn props(&self) -> &RedirectVisualProps {
        match &self.0.kind {
            CompositionVisualKind::Redirect(props) => props,
            _ => unreachable!("a redirect visual handle wraps a redirect visual"),
        }
    }

    /// The hidden surface that renders the source (not a WinUI member).
    pub(crate) fn source_surface(&self) -> Option<Rc<CompositionVisualSurface>> {
        self.props()
            .source_surface()
            .and_then(|surface| surface.into_any_rc().downcast::<CompositionVisualSurface>().ok())
    }

    /// The visual whose subtree the redirect visual shows.
    pub fn source(&self) -> Option<Rc<CompositionVisual>> {
        self.props().source().and_then(|visual| visual.into_any_rc().downcast::<CompositionVisual>().ok())
    }

    pub fn set_source(&self, value: Option<Rc<CompositionVisual>>) {
        if let Some(surface) = self.source_surface() {
            surface.set_source_visual(value.clone());
        }
        self.props().set_source(&*self.0, value.map(|visual| visual as Rc<dyn ICompositionObject>))
    }
}
