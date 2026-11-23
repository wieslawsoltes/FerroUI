use crate::{AnimationEffect, AnimationValue, CurveStep, TrackCurveChannel};
use ferroui_base::media::{CustomEffect, CustomEffectRegistry, EffectParameterValue};
use ferroui_base::{Point, Rect, Size, Vector};

const PARAMETERS: [&str; 8] = [
    "is-ready", "tap-count", "limit", "center", "offset", "size", "area", "label",
];

#[derive(Clone, Debug, PartialEq)]
struct EditingEffect([EffectParameterValue; PARAMETERS.len()]);

impl Default for EditingEffect {
    fn default() -> Self {
        Self([
            false.into(),
            4.into(),
            0.5.into(),
            Point::new(3.0, 4.0).into(),
            Vector::new(-5.0, 6.0).into(),
            Size::new(7.0, 8.0).into(),
            Rect::new(10.0, 20.0, 30.0, 40.0).into(),
            EffectParameterValue::Text("Ready".into()),
        ])
    }
}

impl CustomEffect for EditingEffect {
    const NAME: &'static str = "editing-test";

    fn for_each_parameter(&self, visit: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        for (name, value) in PARAMETERS.into_iter().zip(&self.0) {
            visit(name, value.clone());
        }
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        let Some(index) = PARAMETERS.iter().position(|item| *item == name) else {
            return false;
        };
        if name == "limit" && value.as_float().is_some_and(|value| value > 1.0) {
            // Some application setters adjust other parameters when they validate a value.
            self.0[1] = 10.into();
        }
        self.0[index] = value;
        true
    }
}

fn key(source: &str) -> AnimationValue {
    CustomEffectRegistry::register::<EditingEffect>().unwrap();
    AnimationValue::Effect(AnimationEffect::parse_native(source).unwrap())
}

fn channel(name: &str) -> TrackCurveChannel {
    format!("effect_parameter/editing-test/{name}").parse().unwrap()
}

#[test]
fn edits_registered_components_in_native_units_and_preserves_other_arguments() {
    let source = " Editing_Test ( label='Live 🧪, pending'; tap_count = 2, Tap-Count = 4; area = 10, 20, 30, 40 ) ";
    let value = key(source);
    let count = channel("tap-count/value");
    let ready = channel("is-ready/value");
    let width = channel("area/width");
    let metadata = TrackCurveChannel::key_components(&[count.clone(), ready.clone(), width.clone()], &value);
    assert_eq!(metadata[0].step, Some(CurveStep::Integer));
    assert_eq!(metadata[1].step, Some(CurveStep::Boolean));
    assert_eq!(metadata[2].step, None);
    let updated = count.replace_key_component(&value, -5.0).unwrap();
    assert_eq!(updated.to_xaml(), source.replace("Count = 4", "Count = -5"));
    let updated = ready.replace_key_component(&updated, 1.0).unwrap();
    assert!(updated.to_xaml().ends_with(", is-ready=true ) "));
    let updated = width.replace_key_component(&updated, 52.0).unwrap();
    assert_eq!(channel("area/height").authored_key_value(&updated), Some(40.0));
    assert!(updated.to_xaml().contains("area = 10,20,52,40"));
    assert_eq!(count.authored_key_value(&updated), Some(-5.0));
    assert_eq!(ready.authored_key_value(&updated), Some(1.0));
    assert_eq!(width.authored_key_value(&updated), Some(52.0));
    for (field, sibling, original) in [
        ("center/x", "center/y", 4.0),
        ("offset/y", "offset/x", -5.0),
        ("size/height", "size/width", 7.0),
    ] {
        let updated = channel(field).replace_key_component(&updated, 22.0).unwrap();
        assert_eq!(channel(field).authored_key_value(&updated), Some(22.0));
        assert_eq!(channel(sibling).authored_key_value(&updated), Some(original));
    }
    assert_eq!(count.replace_key_component(&value, 4.0).unwrap().to_xaml(), source);
}

#[test]
fn rejects_wrong_types_effects_ranges_and_linked_parameter_changes() {
    let value = key("editing-test()");
    for (field, next) in [
        ("tap-count/value", 0.5),
        ("tap-count/value", 2147483648.0),
        ("is-ready/value", 2.0),
        ("center/height", 1.0),
        ("label/value", 1.0),
        ("limit/value", 2.0),
        ("limit/value", f64::NAN),
    ] {
        assert!(channel(field).replace_key_component(&value, next).is_err(), "{field}");
    }
    let wrong: TrackCurveChannel = "effect_parameter/spotlight/strength/value".parse().unwrap();
    assert!(wrong.replace_key_component(&value, 1.0).is_err());
    let color = key("Spotlight ( Color = #8021356C; strength = 0.5 )");
    let green: TrackCurveChannel = "effect_parameter/spotlight/color/green".parse().unwrap();
    let changed = green.replace_key_component(&color, 0.8).unwrap();
    assert_eq!(changed.to_xaml().to_ascii_lowercase(), "spotlight ( color = #8021cc6c; strength = 0.5 )");
    assert_eq!(green.authored_key_value(&changed), Some(0.8));
}
