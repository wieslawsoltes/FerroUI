//! The renderer seam of the top-level layer.
//!
//! Upstream every presentation source creates a compositing renderer over
//! the compositor of its platform implementation. Top-levels are written
//! against [`ITopLevelRenderer`] (the renderer contract plus the handful of
//! composition-target members a top-level drives), and the renderer of a
//! source is created by exactly one function, [`create_renderer`], which
//! resolves the [`IRendererFactory`] registered in the service locator.
//!
//! The compositing renderer plugs in by implementing [`ITopLevelRenderer`]
//! and registering a factory that constructs it from the three arguments
//! (which are the arguments of its upstream constructor).

use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::rendering::composition::{CompositingRenderer, CompositionTransparencyLevel, Compositor};
use ferroui_base::rendering::{IHitTester, IPresentationSource, IRenderer};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref, Visual};
use std::any::Any;
use std::rc::Rc;

/// Returns the current platform surfaces of a top-level; called by the
/// renderer whenever it (re)creates its render target.
pub type RenderSurfaces = Rc<dyn Fn() -> Vec<Rc<dyn IPlatformRenderSurface>>>;

/// The renderer of a top-level: the renderer contract and the members of
/// its composition target that the top-level layer sets.
pub trait ITopLevelRenderer: IRenderer {
    /// Sets the visual whose composition visual is the root of the
    /// renderer's composition target; `None` detaches the tree.
    fn set_root(&self, _root: Option<Ref<Visual>>) {}

    /// Sets the transparency level the composition target renders with.
    fn set_transparency_level(&self, _level: CompositionTransparencyLevel) {}

    /// Sets the platform-specific scene info passed to the render target.
    fn set_platform_specific_scene_info(&self, _info: Option<std::sync::Arc<dyn Any + Send + Sync>>) {}

    /// The hit tester backed by the renderer's scene, if it has one. When
    /// `None` the presentation source hit tests the visual tree directly.
    fn hit_tester(&self) -> Option<Rc<dyn IHitTester>> {
        None
    }

    /// Lets callers recover the concrete renderer type.
    fn as_any(&self) -> &dyn Any;
}

/// The compositing renderer is the renderer of a top-level, as upstream.
impl ITopLevelRenderer for CompositingRenderer {
    fn set_root(&self, root: Option<Ref<Visual>>) {
        CompositingRenderer::set_root(self, root)
    }

    fn set_transparency_level(&self, level: CompositionTransparencyLevel) {
        CompositingRenderer::set_transparency_level(self, level)
    }

    fn set_platform_specific_scene_info(&self, info: Option<std::sync::Arc<dyn Any + Send + Sync>>) {
        CompositingRenderer::set_platform_specific_scene_info(self, info)
    }

    fn hit_tester(&self) -> Option<Rc<dyn IHitTester>> {
        CompositingRenderer::hit_tester(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Creates the renderers of presentation sources. Registered in the service
/// locator as `dyn IRendererFactory`.
pub trait IRendererFactory {
    /// Creates the renderer of `root`.
    ///
    /// `compositor` is the compositor of the platform implementation of the
    /// top-level, if the platform has one; `surfaces` returns its current
    /// platform surfaces.
    fn create_renderer(
        &self,
        root: Rc<dyn IPresentationSource>,
        compositor: Option<Rc<Compositor>>,
        surfaces: RenderSurfaces,
    ) -> Rc<dyn ITopLevelRenderer>;
}

impl<F> IRendererFactory for F
where
    F: Fn(Rc<dyn IPresentationSource>, Option<Rc<Compositor>>, RenderSurfaces) -> Rc<dyn ITopLevelRenderer>,
{
    fn create_renderer(
        &self,
        root: Rc<dyn IPresentationSource>,
        compositor: Option<Rc<Compositor>>,
        surfaces: RenderSurfaces,
    ) -> Rc<dyn ITopLevelRenderer> {
        self(root, compositor, surfaces)
    }
}

/// Creates the renderer of a presentation source: THE renderer seam.
///
/// The [`IRendererFactory`] registered in the service locator creates the
/// renderer when there is one; otherwise a [`CompositingRenderer`] is
/// created over `compositor`.
///
/// # Panics
/// Panics when neither a factory nor a compositor is available.
pub fn create_renderer(
    root: Rc<dyn IPresentationSource>,
    compositor: Option<Rc<Compositor>>,
    surfaces: RenderSurfaces,
) -> Rc<dyn ITopLevelRenderer> {
    // A registered factory replaces the default renderer (tests, custom
    // rendering subsystems).
    if let Some(factory) = FerroLocator::current().get_service::<dyn IRendererFactory>() {
        return factory.create_renderer(root, compositor, surfaces);
    }
    // As upstream: a compositing renderer over the compositor of the
    // platform implementation.
    let Some(compositor) = compositor else {
        panic!("Could not create a renderer: the platform has no compositor and no renderer factory is registered. Was a rendering subsystem initialized?")
    };
    CompositingRenderer::new(&root, &compositor, surfaces)
}
