//! The effects whose GPU programs are made ahead of their first draw (#437,
//! not from upstream).
//!
//! A GPU backend makes the program (the pipeline) of a kind of draw the
//! first time it draws it. On a cold shader cache that takes tens of
//! milliseconds a program: the first frame that draws an acrylic surface
//! (a blur, two blend layers, a cross-fade and the noise) stalled 160 to 455
//! ms on macOS with Skia Graphite. A theme registers here the effect graphs
//! it draws (the WinUI theme the acrylic of `AcrylicBrush`); after a frame,
//! a composition target hands the ones it has not prewarmed yet to its
//! render target (`IRenderTarget::prewarm`), which draws them offscreen
//! with the drawing context of its frames, so that their programs are made
//! before a frame needs them (docs/porting/gpu-rendering.md, "Pipelines made
//! ahead of their first draw").

use super::{CompositionEffectFactoryError, EffectGraph, GraphicsEffect};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A registered effect: its name, its graph and the generation of the list
/// that added it.
struct PrewarmEntry {
    name: String,
    graph: Arc<EffectGraph>,
    generation: u64,
}

/// A list of effects to prewarm. Each registration of a new name makes a
/// new generation; a composition target remembers the generation it
/// prewarmed and asks for the effects added since.
pub(crate) struct EffectPrewarmList {
    entries: Mutex<Vec<PrewarmEntry>>,
    /// The generation of the latest entry, read without the lock by the
    /// frames that have nothing to prewarm.
    generation: AtomicU64,
}

impl EffectPrewarmList {
    pub(crate) const fn new() -> Self {
        Self { entries: Mutex::new(Vec::new()), generation: AtomicU64::new(0) }
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, Vec<PrewarmEntry>> {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Adds `effect` under `name`, loaded as an effect factory loads it. A
    /// name that is registered already keeps its effect. Fails as the
    /// factory of the effect would ([`CompositionEffectFactoryError`]).
    pub(crate) fn register(
        &self,
        name: &str,
        effect: &GraphicsEffect,
        animatable_properties: &[&str],
    ) -> Result<(), CompositionEffectFactoryError> {
        let graph = EffectGraph::load(effect, animatable_properties).map_err(|(_, error)| error)?;
        self.register_graph(name, Arc::new(graph));
        Ok(())
    }

    /// Adds a graph loaded already under `name`. A name that is registered
    /// already keeps its graph.
    pub(crate) fn register_graph(&self, name: &str, graph: Arc<EffectGraph>) {
        let mut entries = self.entries();
        if entries.iter().any(|entry| entry.name == name) {
            return;
        }
        let generation = self.generation.load(Ordering::Acquire) + 1;
        entries.push(PrewarmEntry { name: name.to_owned(), graph, generation });
        self.generation.store(generation, Ordering::Release);
    }

    /// The generation of the latest registration; 0 while nothing is
    /// registered.
    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// The graphs registered after `generation`, in the order of their
    /// registration, and the generation they bring the caller to.
    pub(crate) fn since(&self, generation: u64) -> (u64, Vec<Arc<EffectGraph>>) {
        let entries = self.entries();
        let graphs = entries.iter().filter(|entry| entry.generation > generation).map(|entry| entry.graph.clone()).collect();
        (self.generation.load(Ordering::Acquire).max(generation), graphs)
    }

    /// The registered names, in the order of their registration.
    pub(crate) fn names(&self) -> Vec<String> {
        self.entries().iter().map(|entry| entry.name.clone()).collect()
    }
}

/// The effects of the process: registered by the UI thread (a theme, a
/// brush type), read by the threads that render.
static EFFECT_PREWARM: EffectPrewarmList = EffectPrewarmList::new();

/// Registers `effect` (with its animatable properties, as for
/// [`Compositor::create_effect_factory_with_properties`](crate::rendering::composition::Compositor::create_effect_factory_with_properties))
/// under `name` for the prewarm of the GPU programs: after a frame, each
/// composition target that has not drawn it yet draws it once offscreen,
/// with the values of the description, at its scaling, so that the GPU
/// backend makes its programs before the first frame that shows it.
///
/// The source parameters of the effect are drawn as an image (they are
/// bound to brushes only when the effect is used). A name that is
/// registered already keeps its effect. Fails as the factory of the effect
/// would.
pub fn register_effect_prewarm(
    name: &str,
    effect: impl Into<GraphicsEffect>,
    animatable_properties: &[&str],
) -> Result<(), CompositionEffectFactoryError> {
    EFFECT_PREWARM.register(name, &effect.into(), animatable_properties)
}

/// Registers a loaded graph under `name` for the prewarm: the graphs of the
/// custom effects (#455), loaded once per type, which the prewarm draws
/// around an image as the render walk draws them around the content of a
/// visual. A name that is registered already keeps its graph.
pub(crate) fn register_effect_graph_prewarm(name: &str, graph: Arc<EffectGraph>) {
    EFFECT_PREWARM.register_graph(name, graph);
}

/// The names of the effects registered with [`register_effect_prewarm`], in
/// the order of their registration.
pub fn effect_prewarm_names() -> Vec<String> {
    EFFECT_PREWARM.names()
}

/// The generation of the latest registration of the process; see
/// [`EffectPrewarmList::generation`].
pub(crate) fn effect_prewarm_generation() -> u64 {
    EFFECT_PREWARM.generation()
}

/// The graphs of the process registered after `generation`; see
/// [`EffectPrewarmList::since`].
pub(crate) fn effect_prewarm_since(generation: u64) -> (u64, Vec<Arc<EffectGraph>>) {
    EFFECT_PREWARM.since(generation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Color;
    use crate::rendering::composition::effects::{
        ColorSourceEffect, CompositionEffectSourceParameter, GaussianBlurEffect, OpacityEffect,
    };

    fn blur() -> GraphicsEffect {
        GaussianBlurEffect {
            name: "Blur".into(),
            blur_amount: 30.0,
            source: Some(CompositionEffectSourceParameter::new("Backdrop").into()),
            ..Default::default()
        }
        .into()
    }

    fn fill() -> GraphicsEffect {
        ColorSourceEffect { name: "Fill".into(), color: Color::from_rgb(10, 20, 30) }.into()
    }

    #[test]
    fn each_registration_of_a_new_name_is_a_generation_of_its_own() {
        let list = EffectPrewarmList::new();
        assert_eq!(0, list.generation());
        assert_eq!((0, 0), {
            let (generation, graphs) = list.since(0);
            (generation, graphs.len())
        });

        list.register("blur", &blur(), &["Blur.BlurAmount"]).unwrap();
        list.register("fill", &fill(), &[]).unwrap();
        // A name that is registered already keeps its effect.
        list.register("blur", &fill(), &[]).unwrap();
        assert_eq!(2, list.generation());
        assert_eq!(vec!["blur".to_owned(), "fill".to_owned()], list.names());

        let (generation, graphs) = list.since(0);
        assert_eq!(2, generation);
        assert_eq!(2, graphs.len());
        assert!(matches!(
            graphs[0].root.as_ref().map(|root| &root.kind),
            Some(crate::rendering::composition::effects::EffectNodeKind::GaussianBlur { .. })
        ));
        assert_eq!(vec![("Blur.BlurAmount".to_owned(), super::super::EffectPropertyValue::Scalar(30.0))], graphs[0].animatable);

        // Only what was added since the generation a target prewarmed.
        let (generation, graphs) = list.since(1);
        assert_eq!((2, 1), (generation, graphs.len()));
        let (generation, graphs) = list.since(2);
        assert_eq!((2, 0), (generation, graphs.len()));
    }

    #[test]
    fn an_effect_a_factory_cannot_load_is_not_registered() {
        let list = EffectPrewarmList::new();
        // An opacity without a source.
        let effect: GraphicsEffect = OpacityEffect { name: "Opacity".into(), opacity: 0.5, source: None }.into();
        assert!(list.register("opacity", &effect, &[]).is_err());
        // An animatable property the effect does not have.
        assert!(list.register("fill", &fill(), &["Fill.Nothing"]).is_err());
        assert_eq!(0, list.generation());
        assert!(list.names().is_empty());
    }
}
