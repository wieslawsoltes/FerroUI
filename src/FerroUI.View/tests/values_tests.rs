//! The short typed values: each is what the text form of the value parses to.

use crate::builder::IntoValue;
use crate::generated::builders::*;
use crate::{
    alt, argb, blur, ctrl, drop_shadow, linear_gradient, meta, rgb, rotate, scale, shadow, shift, skew, translate,
    BoxShadowOptions, IntoElement, KeyCode, KeyGestureModifiers,
};
use ferroui_base::input::KeyGesture;
use ferroui_base::media::immutable::ImmutableLinearGradientBrush;
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::{
    BoxShadows, Color, Colors, Effect, GradientStop, IBrush, IEffect, LinearGradientBrush,
};
use ferroui_base::{RelativePoint, RelativeUnit};
use std::rc::Rc;

#[test]
fn key_gestures_are_what_their_text_parses_to() {
    let forms = [
        ("Ctrl+S", ctrl(KeyCode::S)),
        ("Shift+F5", shift(KeyCode::F5)),
        ("Alt+X", alt(KeyCode::X)),
        ("Cmd+Q", meta(KeyCode::Q)),
        ("Ctrl+Shift+S", ctrl(KeyCode::S).shift()),
        ("Ctrl+Alt+Shift+A", alt(KeyCode::A).shift().ctrl()),
    ];
    for (text, typed) in forms {
        assert_eq!(typed, KeyGesture::parse(text).unwrap(), "{text}");
    }
    let alone: Option<KeyGesture> = KeyCode::F5.into_value();
    assert_eq!(alone, Some(KeyGesture::parse("F5").unwrap()));
}

#[test]
fn transforms_are_what_their_text_parses_to() {
    let forms = [
        ("translate(10px, 5px)", translate(10.0, 5.0)),
        ("rotate(45deg)", rotate(45.0)),
        ("scale(2, 3)", scale(2.0, 3.0)),
        ("skew(10deg, 20deg)", skew(10.0, 20.0)),
        ("translate(10px, 5px) rotate(45deg)", translate(10.0, 5.0).rotate(45.0)),
        ("scale(2, 3) skew(10deg, 20deg) translate(1px, 2px)", scale(2.0, 3.0).skew(10.0, 20.0).translate(1.0, 2.0)),
        ("rotate(30deg) scale(2, 2)", rotate(30.0).scale(2.0, 2.0)),
    ];
    for (text, typed) in forms {
        let parsed = TransformOperations::parse(text).unwrap();
        // Operations compare by identity, so the comparison is of what they are made of.
        let operations = typed.to_operations();
        assert_eq!(operations.operations().len(), parsed.operations().len(), "{text}");
        assert_eq!(operations.value(), parsed.value(), "{text}");
    }
}

#[test]
fn shadows_are_what_their_text_parses_to() {
    let forms = [
        ("5 5 10 Red", shadow(5.0, 5.0, 10.0, Colors::RED)),
        ("5 5 10 2 Red", shadow(5.0, 5.0, 10.0, Colors::RED).spread(2.0)),
        ("inset 0 1 2 Black", shadow(0.0, 1.0, 2.0, Colors::BLACK).inset()),
        ("inset 1 2 3 4 #1E293B", shadow(1.0, 2.0, 3.0, rgb(0x1E293B)).spread(4.0).inset()),
    ];
    for (text, typed) in forms {
        let value: BoxShadows = typed.into_value();
        let parsed = BoxShadows::parse(text).unwrap();
        assert_eq!((value.count(), value.get(0)), (parsed.count(), parsed.get(0)), "{text}");
    }
    let many: BoxShadows = [shadow(1.0, 1.0, 1.0, Colors::RED), shadow(2.0, 2.0, 2.0, Colors::BLUE)].into_value();
    let parsed = BoxShadows::parse("1 1 1 Red, 2 2 2 Blue").unwrap();
    assert_eq!(many.count(), 2);
    assert_eq!((many.get(0), many.get(1)), (parsed.get(0), parsed.get(1)));
}

fn effect(value: impl IntoValue<Option<Rc<dyn IEffect>>>) -> Rc<dyn IEffect> {
    value.into_value().unwrap()
}

#[test]
fn effects_are_what_their_text_parses_to() {
    let forms: [(&str, Rc<dyn IEffect>); 3] = [
        ("blur(5)", effect(blur(5.0))),
        ("drop-shadow(2 3)", effect(drop_shadow(2.0, 3.0, 0.0, Colors::BLACK))),
        ("drop-shadow(2 3 4 Red)", effect(drop_shadow(2.0, 3.0, 4.0, Colors::RED))),
    ];
    for (text, typed) in forms {
        let parsed = Effect::parse(text).unwrap();
        let immutable = parsed.as_immutable_effect().expect("an immutable effect");
        assert!(immutable.equals(Some(&*typed)), "{text}");
    }
}

fn brush(value: impl IntoValue<Option<Rc<dyn IBrush>>>) -> Rc<dyn IBrush> {
    value.into_value().unwrap()
}

#[test]
fn colors_are_what_their_text_parses_to_and_share_their_brush() {
    assert_eq!(rgb(0x1E293B), Color::parse("#1E293B").unwrap());
    assert_eq!(argb(0x801E293B), Color::parse("#801E293B").unwrap());
    const IN_A_CONSTANT: Color = rgb(0x102030);
    assert_eq!(IN_A_CONSTANT, Color::parse("#102030").unwrap());
    assert!(Rc::ptr_eq(&brush(rgb(0x1E293B)), &brush(rgb(0x1E293B))), "one brush per color");
    assert!(Rc::ptr_eq(&brush(rgb(0x1E293B)), &brush("#1E293B")), "the text form shares it too");
}

#[test]
#[should_panic(expected = "use argb")]
fn rgb_refuses_a_color_with_alpha() {
    let value = std::hint::black_box(0x801E_293B);
    let _ = rgb(value);
}

#[test]
fn a_linear_gradient_is_the_brush_written_with_setters() {
    let (start, end) =
        (RelativePoint::new(0.0, 0.5, RelativeUnit::Relative), RelativePoint::new(1.0, 0.5, RelativeUnit::Relative));
    let source = LinearGradientBrush::new();
    source.set_start_point(start);
    source.set_end_point(end);
    source.gradient_stops().add(GradientStop::with_color_and_offset(Colors::RED, 0.0));
    source.gradient_stops().add(GradientStop::with_color_and_offset(Colors::BLUE, 1.0));
    let expected = ImmutableLinearGradientBrush::from_brush(&source);

    let typed = brush(linear_gradient(&[(0.0, Colors::RED), (1.0, Colors::BLUE)]).horizontal());
    let typed = typed.as_any().downcast_ref::<ImmutableLinearGradientBrush>().expect("a linear gradient");
    assert_eq!(typed.start_point(), expected.start_point());
    assert_eq!(typed.end_point(), expected.end_point());
    let stops = |brush: &ImmutableLinearGradientBrush| -> Vec<(f64, Color)> {
        brush.gradient_stops().iter().map(|stop| (stop.offset(), stop.color())).collect()
    };
    assert_eq!(stops(typed), stops(&expected));
}

#[test]
fn one_gradient_is_one_brush() {
    let make = || linear_gradient(&[(0.0, Colors::RED), (1.0, Colors::BLUE)]).vertical();
    assert!(Rc::ptr_eq(&brush(make()), &brush(make())));
    assert!(!Rc::ptr_eq(&brush(make()), &brush(make().horizontal())));
}

#[test]
fn the_setters_take_the_short_forms() {
    let _: crate::Element = border()
        .background(rgb(0x1E293B))
        .box_shadow(shadow(0.0, 2.0, 4.0, argb(0x40000000)))
        .render_transform(translate(1.0, 2.0).rotate(5.0))
        .effect(blur(2.0))
        .into_element();
    let _: crate::Element = button().hot_key(ctrl(KeyCode::S)).background(linear_gradient(&[(0.0, Colors::RED)])).into_element();
    let _: crate::Element = crate::rsx! {
        Border { background: rgb(0x1E293B), box_shadow: shadow(0.0, 2.0, 4.0, Colors::BLACK),
            render_transform: rotate(5.0), effect: drop_shadow(1.0, 1.0, 2.0, Colors::BLACK),
            Button { hot_key: ctrl(KeyCode::S).shift(), "Save" }
        }
    };
}
