use super::*;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::{Point, RelativeUnit};
use ferroui_controls::{Control, Window};
use serde_json::Value;
use std::{cell::Cell, rc::Rc};

#[test]
fn unchanged_font_icon_mirroring_states_preserve_cache_origin_and_replacement() {
    crate::tests::run(|| {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/font-icon-mirroring.json"
        ))
        .unwrap();
        for case in fixture["corpus"]["cases"].as_array().unwrap() {
            let icon = FontIcon::new();
            assert!(!icon.mirrored_when_right_to_left());
            assert_eq!(icon.render_transform_origin(), RelativePoint::TOP_LEFT);
            let external = ScaleTransform::with_scale(3., 1.);
            let mut first_cache: Option<Ref<ScaleTransform>> = None;
            for expected in case.as_array().unwrap() {
                let operation = expected["operation"].as_str().unwrap();
                match operation {
                    "snapshot" => (),
                    "flag-on" => icon.set_mirrored_when_right_to_left(true),
                    "flag-off" => icon.set_mirrored_when_right_to_left(false),
                    "rtl" => icon.set_flow_direction(FlowDirection::RightToLeft),
                    "ltr" => icon.set_flow_direction(FlowDirection::LeftToRight),
                    "external" => icon.set_render_transform(Some(external.clone().into())),
                    "clear" => icon.set_render_transform(None),
                    "origin-corner" => icon.set_render_transform_origin(RelativePoint::new(
                        0.25,
                        0.75,
                        RelativeUnit::Relative,
                    )),
                    "arm-off" | "arm-ltr" | "arm-external" => {
                        let operation = operation.to_owned();
                        let fired = Cell::new(false);
                        let weak = icon.downgrade();
                        let external = external.clone();
                        icon.add_property_changed_handler(Rc::new(move |change| {
                            if change.property()
                                == Visual::render_transform_property().as_property()
                                && !fired.replace(true)
                            {
                                let icon = weak.upgrade().unwrap();
                                match operation.as_str() {
                                    "arm-off" => icon.set_mirrored_when_right_to_left(false),
                                    "arm-ltr" => {
                                        icon.set_flow_direction(FlowDirection::LeftToRight)
                                    }
                                    "arm-external" => {
                                        icon.set_render_transform(Some(external.clone().into()))
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        }));
                    }
                    other => panic!("Unknown source operation {other}"),
                }
                let cache = icon.mirror_scale.borrow().clone();
                if let Some(cache) = &cache {
                    if let Some(first) = &first_cache {
                        assert!(cache.ptr_eq(first));
                    } else {
                        first_cache = Some(cache.clone());
                    }
                }
                let active = icon.render_transform();
                let active_kind = active.as_ref().map_or("none", |value| {
                    if value.as_object().unwrap().to_ref().ptr_eq(&external) {
                        "external"
                    } else {
                        assert!(
                            value
                                .as_object()
                                .unwrap()
                                .to_ref()
                                .ptr_eq(cache.as_ref().unwrap())
                        );
                        "cached"
                    }
                });
                let origin = icon.render_transform_origin();
                assert_eq!(
                    icon.flow_direction() == FlowDirection::RightToLeft,
                    expected["rtl"].as_bool().unwrap(),
                    "{expected}"
                );
                assert_eq!(
                    icon.mirrored_when_right_to_left(),
                    expected["flag"].as_bool().unwrap(),
                    "{expected}"
                );
                assert_eq!(
                    active_kind,
                    expected["active"].as_str().unwrap(),
                    "{expected}"
                );
                assert_eq!(
                    cache.is_some(),
                    expected["cached"].as_bool().unwrap(),
                    "{expected}"
                );
                assert_eq!(
                    cache.map(|s| s.scale_x()).map(f64::to_bits),
                    expected["cacheScale"].as_f64().map(f64::to_bits),
                    "{expected}"
                );
                assert_eq!(
                    active.map(|s| s.value().m11).map(f64::to_bits),
                    expected["activeScale"].as_f64().map(f64::to_bits),
                    "{expected}"
                );
                assert_eq!(
                    [origin.point.x.to_bits(), origin.point.y.to_bits()],
                    [
                        expected["origin"][0].as_f64().unwrap().to_bits(),
                        expected["origin"][1].as_f64().unwrap().to_bits()
                    ],
                    "{expected}"
                );
            }
        }
    });
}

#[test]
fn mirror_scale_mutations_update_native_geometry_and_release_with_the_icon() {
    crate::tests::run(|| {
        let window = Window::new();
        let icon = FontIcon::new();
        icon.set_use_layout_rounding(false);
        icon.set_width(30.5);
        icon.set_height(22.25);
        icon.set_glyph(Some("➔".into()));
        icon.set_flow_direction(FlowDirection::RightToLeft);
        window.set_content(Some(Control::boxed(icon.clone())));
        window.show();
        window.layout_manager().execute_layout_pass();
        let before = icon.translate_point(Point::new(4., 5.), &window).unwrap();
        icon.set_mirrored_when_right_to_left(true);
        let transform = icon.mirror_scale.borrow().clone().unwrap();
        let first = icon.translate_point(Point::new(4., 5.), &window).unwrap();
        assert_eq!(first.x - before.x, icon.bounds().width - 8.);
        assert_eq!(first.y, before.y);
        icon.set_mirrored_when_right_to_left(false);
        assert_eq!(
            icon.translate_point(Point::new(4., 5.), &window).unwrap(),
            before
        );
        icon.set_mirrored_when_right_to_left(true);
        assert_eq!(
            icon.translate_point(Point::new(4., 5.), &window).unwrap(),
            first
        );
        assert!(transform.ptr_eq(icon.mirror_scale.borrow().as_ref().unwrap()));
        let weak_scale = transform.downgrade();
        let weak_icon = icon.downgrade();
        window.set_content(None);
        window.close();
        drop(transform);
        drop(icon);
        drop(window);
        assert!(weak_icon.upgrade().is_none());
        assert!(weak_scale.upgrade().is_none());
    });
}
