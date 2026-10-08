//! Port of `Rendering/SceneGraph/RenderDataStreamEffectTests.cs`.

use crate::media::effects::{EffectExtensions, ImmutableBlurEffect};
use crate::media::BoxShadows;
use crate::rendering::composition::drawing::RenderDataStream;
use crate::{Rect, RoundedRect};

#[test]
fn effect_inflates_child_bounds_by_padding() {
    let mut stream = RenderDataStream::new();
    stream.push_effect(Some(std::sync::Arc::new(ImmutableBlurEffect::new(5.0))), Rect::new(0.0, 0.0, 100.0, 100.0));
    stream.draw_rectangle(
        None,
        None,
        None,
        RoundedRect::from_rect(Rect::new(0.0, 0.0, 100.0, 100.0)),
        &BoxShadows::default(),
    );
    stream.pop();

    let padding = EffectExtensions::get_effect_output_padding(Some(&ImmutableBlurEffect::new(5.0)));
    assert_eq!(Some(Rect::new(0.0, 0.0, 100.0, 100.0).inflate_thickness(padding)), stream.calculate_bounds());
}

#[test]
fn empty_effect_scope_has_null_bounds() {
    let mut stream = RenderDataStream::new();
    stream.push_effect(Some(std::sync::Arc::new(ImmutableBlurEffect::new(5.0))), Rect::new(0.0, 0.0, 100.0, 100.0));
    stream.pop();
    assert_eq!(None, stream.calculate_bounds());
}
