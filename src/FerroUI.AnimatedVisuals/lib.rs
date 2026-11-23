//! An animated-visual runtime (#557): a Lottie (Bodymovin JSON) document is read once into a typed
//! model, and any frame of it is evaluated into a list of drawing commands in the space of the
//! composition. It knows no render backend: what it produces are paths, paints, transforms, clips
//! and mattes, which every backend draws on its GPU.

mod evaluate;
mod model;
mod path;
mod playback;
mod player;
mod render;
mod source;
mod value;

pub use evaluate::{FrameCommand, FrameCommands, GradientKind, MaskMode, Paint, ShapeDraw, StrokeStyle};
pub use model::{AnimatedVisual, AnimatedVisualError, MatteMode, Marker};
pub use path::{PathCommand, PathData};
pub use playback::AnimatedIconAnimationQueueBehavior;
pub use player::{AnimatedIcon, AnimatedVisualPlayer, PlayCompletion};
pub use render::VisualRenderer;
pub use source::{IAnimatedVisualSource, IAnimatedVisualSource2, IDynamicAnimatedVisualSource, LottieVisualSource};
