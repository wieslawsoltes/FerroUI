use crate::{AnimationValue, KeyframeComponentEdit, RelativeUnit, TrackCurveChannel};

fn channel(name: &str) -> TrackCurveChannel {
    name.parse().unwrap()
}

#[test]
fn component_batch_requires_distinct_writable_fields() {
    let original = AnimationValue::RelativePoint { x: 0.25, y: 0.6, unit: RelativeUnit::Relative };
    let mut edits = [
        KeyframeComponentEdit { channel: channel("percent_x"), value: 45.0 },
        KeyframeComponentEdit { channel: channel("percent_y"), value: 35.0 },
    ];
    assert_eq!(
        TrackCurveChannel::replace_key_components(&original, &edits).unwrap(),
        AnimationValue::RelativePoint { x: 0.45, y: 0.35, unit: RelativeUnit::Relative },
    );
    edits[1].channel = channel("percent_x");
    assert!(TrackCurveChannel::replace_key_components(&original, &edits).unwrap_err().0.contains("only once"));
    edits[1].channel = channel("absolute_y");
    assert!(TrackCurveChannel::replace_key_components(&original, &edits).is_err());
    assert!(TrackCurveChannel::replace_key_components(&original, &[]).is_err());
}

#[test]
fn relative_and_color_handles_edit_in_graph_units() {
    let original = AnimationValue::RelativePoint {
        x: 0.25,
        y: 0.6,
        unit: RelativeUnit::Relative,
    };
    let x = channel("percent_x");
    assert_eq!(x.authored_key_value(&original), Some(25.0));
    assert_eq!(
        x.replace_key_component(&original, 43.0).unwrap(),
        AnimationValue::RelativePoint {
            x: 0.43,
            y: 0.6,
            unit: RelativeUnit::Relative
        },
    );
    assert!(channel("absolute_x")
        .replace_key_component(&original, 43.0)
        .is_err());
    let color = AnimationValue::from_argb8([128, 30, 70, 80]);
    let red = channel("red");
    assert!(red.replace_key_component(&color, 1.2).is_err());
    assert_eq!(
        red.replace_key_component(&color, 0.5).unwrap().argb8(),
        Some([128, 128, 70, 80])
    );
    assert!(red.replace_key_component(&color, f64::NAN).is_err());
}

#[cfg(feature = "native-values")]
#[test]
fn native_composite_handles_keep_sibling_layers_and_geometry() {
    use crate::{
        AnimationBoxShadows, AnimationBrush, AnimationGradient, AnimationGradientStop,
        AnimationTransformList, GradientShape,
    };
    let gradient = AnimationGradient::from_stops(
        GradientShape::defaults("RadialGradientBrush").unwrap(),
        vec![AnimationGradientStop {
            offset: 0.24,
            argb: [255, 30, 48, 96],
        }],
    );
    let brush = AnimationValue::Brush(AnimationBrush::gradient(gradient));
    let updated = channel("gradient/radius_y/percent")
        .replace_key_component(&brush, 65.0)
        .unwrap();
    assert_eq!(
        channel("gradient/radius_x/percent").authored_key_value(&updated),
        Some(50.0)
    );
    assert_eq!(
        channel("gradient/radius_y/percent").authored_key_value(&updated),
        Some(65.0)
    );
    let updated = channel("gradient_stop/1/green")
        .replace_key_component(&updated, 0.5)
        .unwrap();
    assert_eq!(
        channel("gradient_stop/1/green").authored_key_value(&updated),
        Some(128.0 / 255.0)
    );
    assert_eq!(
        channel("gradient_stop/1/stop_offset").authored_key_value(&updated),
        Some(0.24)
    );
    assert_eq!(
        channel("gradient_stop/2/stop_offset").authored_key_value(&updated),
        None
    );

    let shadow = AnimationValue::BoxShadows(
        AnimationBoxShadows::parse("3 4 8 1 #A0314050, inset 1 2 3 0 #90453386").unwrap(),
    );
    let updated = channel("box_shadow/2/offset_x")
        .replace_key_component(&shadow, 6.0)
        .unwrap();
    assert_eq!(
        channel("box_shadow/1/offset_x").authored_key_value(&updated),
        Some(3.0)
    );
    assert_eq!(
        channel("box_shadow/2/offset_x").authored_key_value(&updated),
        Some(6.0)
    );
    assert!(updated.to_xaml().contains("inset"));

    let transform = AnimationValue::TransformList(
        AnimationTransformList::parse("translate(8px, 4px) rotate(30deg)").unwrap(),
    );
    assert!(channel("transform_matrix/m11")
        .replace_key_component(&transform, 0.0)
        .is_err());
    let angle = channel("transform_operation/2/rotate/angle");
    let updated = angle.replace_key_component(&transform, 60.0).unwrap();
    assert!((angle.authored_key_value(&updated).unwrap() - 60.0).abs() < 1e-12);
    assert_eq!(
        channel("transform_operation/1/translate/absolute_y").authored_key_value(&updated),
        Some(4.0)
    );
}
