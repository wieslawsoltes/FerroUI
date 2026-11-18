//! WinUI's storyboards (`Microsoft.UI.Xaml.Media.Animation`; #443, VSM-3 of
//! docs/porting/winui-calculator-gaps.md, section 5.8): `Storyboard`, the animations from, to or
//! by a value, the key frame animations and their key frames, and the easing functions. A
//! storyboard runs on its own or as the storyboard of a `VisualState`.

mod animations;
mod easing_functions;
mod key_frames;
mod property_path;
mod storyboard;
mod timeline;

#[cfg(test)]
mod storyboard_tests;

pub use animations::{ColorAnimation, DoubleAnimation, PointAnimation};
pub use easing_functions::{
    BackEase, BounceEase, CircleEase, CubicEase, EasingFunctionBase, EasingMode, ElasticEase, ExponentialEase,
    PowerEase, QuadraticEase, QuarticEase, QuinticEase, SineEase,
};
pub use key_frames::{
    ColorAnimationUsingKeyFrames, ColorKeyFrame, DiscreteColorKeyFrame, DiscreteDoubleKeyFrame, DiscreteObjectKeyFrame,
    DiscretePointKeyFrame, DoubleAnimationUsingKeyFrames, DoubleKeyFrame, EasingColorKeyFrame, EasingDoubleKeyFrame,
    EasingPointKeyFrame, LinearColorKeyFrame, LinearDoubleKeyFrame, LinearPointKeyFrame, ObjectAnimationUsingKeyFrames,
    ObjectKeyFrame, PointAnimationUsingKeyFrames, PointKeyFrame, SplineColorKeyFrame, SplineDoubleKeyFrame,
    SplinePointKeyFrame,
};
pub use storyboard::Storyboard;
pub use timeline::Timeline;
