//! An animated-visual runtime (#557): a Lottie (Bodymovin JSON) document is read once into a typed
//! model, and any frame of it is evaluated into a list of drawing commands in the space of the
//! composition. It knows no render backend: what it produces are paths, paints, transforms, clips
//! and mattes, which every backend draws on its GPU.

mod content;
mod evaluate;
mod icons;
mod model;
mod path;
mod playback;
mod player;
mod register_types;
mod ring;
#[cfg(test)]
mod render;
mod scene;
mod source;
mod value;

pub use content::ImageAsset;
pub use evaluate::{FrameCommand, FrameCommands, GradientKind, MaskMode, Paint, ShapeDraw, StrokeStyle, TextDraw};
pub use icons::{
    AnimatedAcceptVisualSource, AnimatedBackVisualSource, AnimatedChevronDownSmallVisualSource,
    AnimatedChevronRightDownSmallVisualSource, AnimatedChevronUpDownSmallVisualSource, AnimatedFindVisualSource,
    AnimatedGlobalNavigationButtonVisualSource, AnimatedSettingsVisualSource,
};
pub use model::{AnimatedVisual, AnimatedVisualError, MatteMode, Marker};
pub use path::{PathCommand, PathData};
pub use playback::AnimatedIconAnimationQueueBehavior;
pub use player::{AnimatedIcon, AnimatedIconSource, AnimatedVisualPlayer, PlayCompletion};
pub use register_types::{register_types, ASSEMBLY};
#[cfg(test)]
pub(crate) use render::VisualRenderer;
pub use scene::CompositionScene;
pub use source::{
    AnimatedVisualSource, IAnimatedVisualSource, IAnimatedVisualSource2, IDynamicAnimatedVisualSource, ImageLoader,
    LottieDocumentSource, LottieVisualSource,
};
