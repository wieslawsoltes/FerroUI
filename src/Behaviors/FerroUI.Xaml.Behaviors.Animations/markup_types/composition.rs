//! The markup metadata of the static animation catalogs: their `Set*` functions, which markup
//! assigns as attached values (`<Border AttentionAnimations.Bounce="900" />`), as the reference's
//! markup assigns its static setters.

use crate::{
    AttentionAnimations, EntranceAnimations, ExitAnimations, FadeAnimation, FramerMotionAnimations, RotateAnimation,
    ScaleAnimation, SlidingAnimation, SpecialAnimations,
};
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::Ref;
use ferroui_controls::Control;

ferro_markup_type!(static AttentionAnimations {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetBounce(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_bounce(&element, milliseconds),
        static fn SetFlash(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_flash(&element, milliseconds),
        static fn SetPulse(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_pulse(&element, milliseconds),
        static fn SetRubberBand(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_rubber_band(&element, milliseconds),
        static fn SetShakeX(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_shake_x(&element, milliseconds),
        static fn SetShakeY(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_shake_y(&element, milliseconds),
        static fn SetHeadShake(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_head_shake(&element, milliseconds),
        static fn SetSwing(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_swing(&element, milliseconds),
        static fn SetTada(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_tada(&element, milliseconds),
        static fn SetWobble(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_wobble(&element, milliseconds),
        static fn SetJello(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_jello(&element, milliseconds),
        static fn SetHeartBeat(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| AttentionAnimations::set_heart_beat(&element, milliseconds),
    ],
});

ferro_markup_type!(static EntranceAnimations {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetBackInDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_back_in_down(&element, milliseconds),
        static fn SetBackInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_back_in_left(&element, milliseconds),
        static fn SetBackInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_back_in_right(&element, milliseconds),
        static fn SetBackInUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_back_in_up(&element, milliseconds),
        static fn SetBounceIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_bounce_in(&element, milliseconds),
        static fn SetBounceInDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_bounce_in_down(&element, milliseconds),
        static fn SetBounceInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_bounce_in_left(&element, milliseconds),
        static fn SetBounceInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_bounce_in_right(&element, milliseconds),
        static fn SetBounceInUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_bounce_in_up(&element, milliseconds),
        static fn SetFadeIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in(&element, milliseconds),
        static fn SetFadeInDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_down(&element, milliseconds),
        static fn SetFadeInDownBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_down_big(&element, milliseconds),
        static fn SetFadeInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_left(&element, milliseconds),
        static fn SetFadeInLeftBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_left_big(&element, milliseconds),
        static fn SetFadeInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_right(&element, milliseconds),
        static fn SetFadeInRightBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_right_big(&element, milliseconds),
        static fn SetFadeInUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_up(&element, milliseconds),
        static fn SetFadeInUpBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_up_big(&element, milliseconds),
        static fn SetFadeInTopLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_top_left(&element, milliseconds),
        static fn SetFadeInTopRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_top_right(&element, milliseconds),
        static fn SetFadeInBottomLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_bottom_left(&element, milliseconds),
        static fn SetFadeInBottomRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_bottom_right(&element, milliseconds),
        static fn SetFadeInZoom(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_fade_in_zoom(&element, milliseconds),
        static fn SetFlipInX(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_flip_in_x(&element, milliseconds),
        static fn SetFlipInY(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_flip_in_y(&element, milliseconds),
        static fn SetLightSpeedInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_light_speed_in_left(&element, milliseconds),
        static fn SetLightSpeedInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_light_speed_in_right(&element, milliseconds),
        static fn SetRotateIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_rotate_in(&element, milliseconds),
        static fn SetRotateInDownLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_rotate_in_down_left(&element, milliseconds),
        static fn SetRotateInDownRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_rotate_in_down_right(&element, milliseconds),
        static fn SetRotateInUpLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_rotate_in_up_left(&element, milliseconds),
        static fn SetRotateInUpRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_rotate_in_up_right(&element, milliseconds),
        static fn SetSlideInDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_slide_in_down(&element, milliseconds),
        static fn SetSlideInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_slide_in_left(&element, milliseconds),
        static fn SetSlideInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_slide_in_right(&element, milliseconds),
        static fn SetSlideInUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_slide_in_up(&element, milliseconds),
        static fn SetZoomIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_zoom_in(&element, milliseconds),
        static fn SetZoomInDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_zoom_in_down(&element, milliseconds),
        static fn SetZoomInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_zoom_in_left(&element, milliseconds),
        static fn SetZoomInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_zoom_in_right(&element, milliseconds),
        static fn SetZoomInUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_zoom_in_up(&element, milliseconds),
        static fn SetJackInTheBox(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_jack_in_the_box(&element, milliseconds),
        static fn SetRollIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| EntranceAnimations::set_roll_in(&element, milliseconds),
    ],
});

ferro_markup_type!(static ExitAnimations {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetBackOutDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_back_out_down(&element, milliseconds),
        static fn SetBackOutLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_back_out_left(&element, milliseconds),
        static fn SetBackOutRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_back_out_right(&element, milliseconds),
        static fn SetBackOutUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_back_out_up(&element, milliseconds),
        static fn SetBounceOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_bounce_out(&element, milliseconds),
        static fn SetBounceOutDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_bounce_out_down(&element, milliseconds),
        static fn SetBounceOutLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_bounce_out_left(&element, milliseconds),
        static fn SetBounceOutRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_bounce_out_right(&element, milliseconds),
        static fn SetBounceOutUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_bounce_out_up(&element, milliseconds),
        static fn SetFadeOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out(&element, milliseconds),
        static fn SetFadeOutDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_down(&element, milliseconds),
        static fn SetFadeOutDownBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_down_big(&element, milliseconds),
        static fn SetFadeOutLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_left(&element, milliseconds),
        static fn SetFadeOutLeftBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_left_big(&element, milliseconds),
        static fn SetFadeOutRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_right(&element, milliseconds),
        static fn SetFadeOutRightBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_right_big(&element, milliseconds),
        static fn SetFadeOutUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_up(&element, milliseconds),
        static fn SetFadeOutUpBig(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_up_big(&element, milliseconds),
        static fn SetFadeOutTopLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_top_left(&element, milliseconds),
        static fn SetFadeOutTopRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_top_right(&element, milliseconds),
        static fn SetFadeOutBottomLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_bottom_left(&element, milliseconds),
        static fn SetFadeOutBottomRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_fade_out_bottom_right(&element, milliseconds),
        static fn SetFlipOutX(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_flip_out_x(&element, milliseconds),
        static fn SetFlipOutY(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_flip_out_y(&element, milliseconds),
        static fn SetLightSpeedOutLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_light_speed_out_left(&element, milliseconds),
        static fn SetLightSpeedOutRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_light_speed_out_right(&element, milliseconds),
        static fn SetRotateOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_rotate_out(&element, milliseconds),
        static fn SetRotateOutDownLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_rotate_out_down_left(&element, milliseconds),
        static fn SetRotateOutDownRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_rotate_out_down_right(&element, milliseconds),
        static fn SetRotateOutUpLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_rotate_out_up_left(&element, milliseconds),
        static fn SetRotateOutUpRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_rotate_out_up_right(&element, milliseconds),
        static fn SetSlideOutDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_slide_out_down(&element, milliseconds),
        static fn SetSlideOutLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_slide_out_left(&element, milliseconds),
        static fn SetSlideOutRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_slide_out_right(&element, milliseconds),
        static fn SetSlideOutUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_slide_out_up(&element, milliseconds),
        static fn SetZoomOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_zoom_out(&element, milliseconds),
        static fn SetZoomOutDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_zoom_out_down(&element, milliseconds),
        static fn SetZoomOutLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_zoom_out_left(&element, milliseconds),
        static fn SetZoomOutRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_zoom_out_right(&element, milliseconds),
        static fn SetZoomOutUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_zoom_out_up(&element, milliseconds),
        static fn SetHinge(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_hinge(&element, milliseconds),
        static fn SetRollOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ExitAnimations::set_roll_out(&element, milliseconds),
    ],
});

ferro_markup_type!(static FadeAnimation {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetFadeIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FadeAnimation::set_fade_in(&element, milliseconds),
        static fn SetFadeOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FadeAnimation::set_fade_out(&element, milliseconds),
        static fn SetCustomFade(Ref<Control>, f64, f64, f64) =>
            |element: Ref<Control>, from_opacity: f64, to_opacity: f64, milliseconds: f64| FadeAnimation::set_custom_fade(&element, from_opacity, to_opacity, milliseconds),
    ],
});

ferro_markup_type!(static FramerMotionAnimations {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetFadeIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_fade_in(&element, milliseconds),
        static fn SetFadeInUp(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_fade_in_up(&element, milliseconds),
        static fn SetFadeInDown(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_fade_in_down(&element, milliseconds),
        static fn SetFadeInLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_fade_in_left(&element, milliseconds),
        static fn SetFadeInRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_fade_in_right(&element, milliseconds),
        static fn SetSlideInFromLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_slide_in_from_left(&element, milliseconds),
        static fn SetSlideInFromRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_slide_in_from_right(&element, milliseconds),
        static fn SetSlideInFromTop(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_slide_in_from_top(&element, milliseconds),
        static fn SetSlideInFromBottom(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_slide_in_from_bottom(&element, milliseconds),
        static fn SetPopIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_pop_in(&element, milliseconds),
        static fn SetPopOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_pop_out(&element, milliseconds),
        static fn SetSpringIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_spring_in(&element, milliseconds),
        static fn SetSpringOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_spring_out(&element, milliseconds),
        static fn SetRotateIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_rotate_in(&element, milliseconds),
        static fn SetRotateOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_rotate_out(&element, milliseconds),
        static fn SetScaleIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_scale_in(&element, milliseconds),
        static fn SetScaleOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| FramerMotionAnimations::set_scale_out(&element, milliseconds),
    ],
});

ferro_markup_type!(static RotateAnimation {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetRotateClockwise(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| RotateAnimation::set_rotate_clockwise(&element, milliseconds),
        static fn SetRotateCounterClockwise(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| RotateAnimation::set_rotate_counter_clockwise(&element, milliseconds),
        static fn SetRotateIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| RotateAnimation::set_rotate_in(&element, milliseconds),
        static fn SetRotateOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| RotateAnimation::set_rotate_out(&element, milliseconds),
        static fn SetFlip(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| RotateAnimation::set_flip(&element, milliseconds),
        static fn SetSwing(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| RotateAnimation::set_swing(&element, milliseconds),
        static fn SetCustomRotate(Ref<Control>, f64, f64, f64) =>
            |element: Ref<Control>, from_angle: f64, to_angle: f64, milliseconds: f64| RotateAnimation::set_custom_rotate(&element, from_angle, to_angle, milliseconds),
    ],
});

ferro_markup_type!(static ScaleAnimation {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetScaleIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ScaleAnimation::set_scale_in(&element, milliseconds),
        static fn SetScaleOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ScaleAnimation::set_scale_out(&element, milliseconds),
        static fn SetZoomIn(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ScaleAnimation::set_zoom_in(&element, milliseconds),
        static fn SetZoomOut(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ScaleAnimation::set_zoom_out(&element, milliseconds),
        static fn SetBounce(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| ScaleAnimation::set_bounce(&element, milliseconds),
        static fn SetCustomScale(Ref<Control>, f64, f64, f64, f64, f64) =>
            |element: Ref<Control>, from_scale_x: f64, from_scale_y: f64, to_scale_x: f64, to_scale_y: f64, milliseconds: f64| ScaleAnimation::set_custom_scale(&element, from_scale_x, from_scale_y, to_scale_x, to_scale_y, milliseconds),
    ],
});

ferro_markup_type!(static SlidingAnimation {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetLeft(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| SlidingAnimation::set_left(&element, milliseconds),
        static fn SetRight(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| SlidingAnimation::set_right(&element, milliseconds),
        static fn SetTop(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| SlidingAnimation::set_top(&element, milliseconds),
        static fn SetBottom(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| SlidingAnimation::set_bottom(&element, milliseconds),
    ],
});

ferro_markup_type!(static SpecialAnimations {
    namespace: "FerroUI.Xaml.Interactions.Custom",
    methods: [
        static fn SetFlip(Ref<Control>, f64) =>
            |element: Ref<Control>, milliseconds: f64| SpecialAnimations::set_flip(&element, milliseconds),
    ],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <AttentionAnimations as MarkupTyped>::MARKUP,
    <EntranceAnimations as MarkupTyped>::MARKUP,
    <ExitAnimations as MarkupTyped>::MARKUP,
    <FadeAnimation as MarkupTyped>::MARKUP,
    <FramerMotionAnimations as MarkupTyped>::MARKUP,
    <RotateAnimation as MarkupTyped>::MARKUP,
    <ScaleAnimation as MarkupTyped>::MARKUP,
    <SlidingAnimation as MarkupTyped>::MARKUP,
    <SpecialAnimations as MarkupTyped>::MARKUP,
];
