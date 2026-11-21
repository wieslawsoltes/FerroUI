use crate::{primitives::RibbonIconPresenter, tests::run};
use ferroribbon_core::{
    RibbonString,
    model::{RibbonIcon, RibbonIconKind},
};
use ferroui_base::data::BindingPriority;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{
    FontWeight, Geometry, IBrush, IImage, PenLineCap, PenLineJoin, SolidColorBrush, Stretch,
    StretchDirection, TextAlignment,
};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::shapes::Path;
use ferroui_controls::{Border, Canvas, Control, Image, Panel, PathIcon, TextBlock, Viewbox};
use std::rc::Rc;

fn child(p: &RibbonIconPresenter) -> Ref<Control> {
    p.children().snapshot()[0].clone()
}
fn brush(argb: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(argb).into()
}
fn text_icon(p: &RibbonIconPresenter, text: &str) {
    p.set_icon(Some(Rc::new(text.to_owned())))
}

#[test]
fn classifier_matches_verbatim_csharp_all_bmp_and_adversarial_utf16() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/icon-classifier.json")).unwrap();
    for mode in ["single", "digit"] {
        let mut next = 0;
        for range in corpus[mode].as_array().unwrap() {
            let start = range[0].as_u64().unwrap() as u32;
            let end = range[1].as_u64().unwrap() as u32;
            assert_eq!(start, next);
            for unit in start..=end {
                let mut units = if mode == "single" {
                    vec![]
                } else {
                    "M-----".encode_utf16().collect()
                };
                units.push(unit as u16);
                assert_eq!(
                    RibbonIconPresenter::classify(&RibbonString::from_utf16(units)).raw(),
                    range[2].as_i64().unwrap() as i32,
                    "{mode}: {unit:x}"
                );
            }
            next = end + 1;
        }
        assert_eq!(next, 65536);
    }
    for case in corpus["cases"].as_array().unwrap() {
        let units = case["units"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>();
        assert_eq!(
            RibbonIconPresenter::classify(&RibbonString::from_utf16(units)).raw(),
            case["kind"].as_i64().unwrap() as i32,
            "{case}"
        );
    }
}
#[test]
fn presenter_defaults_rebuild_empty_and_unknown_match_source_order() {
    run(|| {
        let p = RibbonIconPresenter::new();
        assert_eq!(p.icon_size(), 16.);
        assert!(!p.has_icon());
        assert!(p.width().is_nan() && p.height().is_nan());
        assert!(!p.is_hit_test_visible());
        assert_eq!(p.horizontal_alignment(), HorizontalAlignment::Center);
        assert_eq!(p.vertical_alignment(), VerticalAlignment::Center);
        p.set_icon_size(20.);
        assert_eq!((p.width(), p.height()), (0., 0.));
        p.set_icon(Some(Rc::new(42_i32)));
        assert!(!p.has_icon());
        assert_eq!((p.width(), p.height()), (20., 20.));
        text_icon(&p, "");
        assert!(!p.has_icon());
        assert_eq!(p.width(), 20.);
        text_icon(&p, "%");
        assert!(p.has_icon());
        assert!(child(&p).cast::<TextBlock>().is_some());
        p.set_icon(None);
        assert!(!p.has_icon());
        assert!(p.children().is_empty());
        assert_eq!((p.width(), p.height()), (0., 0.));
    });
}
#[test]
fn text_rendering_preserves_utf16_length_size_foreground_and_rebuild() {
    run(|| {
        let p = RibbonIconPresenter::new();
        let red = brush(0xffff0000);
        let blue = brush(0xff0000ff);
        p.set_foreground(Some(red.clone()));
        text_icon(&p, "é");
        let text = child(&p).cast::<TextBlock>().unwrap();
        assert_eq!(text.text().as_deref(), Some("é"));
        assert_eq!(text.font_size(), 12.8);
        assert_eq!(text.font_weight(), FontWeight::SemiBold);
        assert_eq!(text.text_alignment(), TextAlignment::Center);
        assert_eq!(text.foreground(), Some(red.clone()));
        p.set_foreground(None);
        assert_eq!(text.foreground(), Some(red.clone())); // source intentionally leaves it set
        p.set_foreground(Some(blue.clone()));
        assert_eq!(text.foreground(), Some(blue));
        text_icon(&p, "ééé");
        let view = child(&p).cast::<Viewbox>().unwrap();
        assert_eq!(view.stretch(), Stretch::Uniform);
        assert_eq!(view.stretch_direction(), StretchDirection::DownOnly);
        assert_eq!(
            view.child()
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("ééé")
        );
        let icon = RibbonIcon::text_with_foreground(Some("red".into()), Some("#f00".into()));
        p.set_icon(Some(icon));
        let fixed = child(&p)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        let color = fixed
            .foreground()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color();
        assert_eq!(color.to_uint32(), 0xffff0000);
        p.set_foreground(Some(brush(0xff00ff00)));
        assert_eq!(
            fixed
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            color
        );
        // Explicit Text does not apply Glyph classification; a supplementary scalar
        // is two CLR units and stays directly hosted.
        p.set_icon(Some(RibbonIcon::text(Some("👩".into()))));
        assert!(child(&p).cast::<TextBlock>().is_some());
    });
}
#[test]
fn free_control_is_rehosted_by_identity_without_stealing_or_retained_old_parent() {
    run(|| {
        let p = RibbonIconPresenter::new();
        let icon = Border::new();
        let weak = icon.downgrade();
        let value: BoxedValue = Rc::new(icon.clone());
        p.set_icon(Some(value.clone()));
        let old = child(&p).cast::<Viewbox>().unwrap();
        assert_eq!(old.child().unwrap(), icon.clone().upcast::<Control>());
        assert!(icon.visual_parent().is_some() && icon.parent().is_some());
        p.set_icon_size(24.);
        assert!(old.child().is_none());
        let new = child(&p).cast::<Viewbox>().unwrap();
        assert_ne!(old, new);
        assert_eq!(new.child().unwrap(), icon.clone().upcast::<Control>());
        let copy = RibbonIconPresenter::new();
        copy.set_icon(Some(value));
        assert!(!copy.has_icon());
        assert_eq!(copy.width(), 16.);
        p.set_icon(None);
        assert!(new.child().is_none());
        assert!(icon.visual_parent().is_none() && icon.parent().is_none());
        copy.set_icon_size(20.);
        assert!(copy.has_icon()); // rebuild can now host the original
        copy.set_icon(None);
        drop(icon);
        assert!(weak.upgrade().is_none());
        let parent = Panel::new();
        let in_use = Border::new();
        parent.children().add(in_use.clone());
        p.set_icon(Some(Rc::new(in_use.clone())));
        assert!(!p.has_icon());
        assert_eq!(parent.children().snapshot()[0], in_use.upcast::<Control>());
    });
}
#[test]
fn path_icon_clones_geometry_and_preserves_local_vs_styled_foreground() {
    run(|| {
        let p = RibbonIconPresenter::new();
        let source = PathIcon::new();
        let parent = Panel::new();
        parent.children().add(source.clone());
        let geometry = Geometry::parse("M0,0 L10,10").unwrap();
        source.set_data(geometry.clone());
        let red = brush(0xffff0000);
        let blue = brush(0xff0000ff);
        p.set_foreground(Some(blue.clone()));
        let style = source.set_value_with_priority(
            TemplatedControl::foreground_property(),
            Some(red.clone()),
            BindingPriority::Style,
        );
        p.set_icon(Some(Rc::new(source.clone())));
        let clone = child(&p)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<PathIcon>()
            .unwrap();
        assert_ne!(clone, source);
        assert_eq!(clone.data(), Some(geometry.clone()));
        assert_eq!(clone.foreground(), Some(blue.clone()));
        assert_eq!(source.foreground(), Some(red.clone()));
        source.set_foreground(Some(red.clone()));
        p.set_icon_size(32.);
        let fixed = child(&p)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<PathIcon>()
            .unwrap();
        assert_eq!(fixed.foreground(), Some(red));
        p.set_foreground(Some(brush(0xff00ff00)));
        assert_ne!(fixed.foreground(), p.foreground());
        source.set_foreground(None);
        p.set_icon_size(48.);
        let null_fixed = child(&p)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<PathIcon>()
            .unwrap();
        assert!(null_fixed.foreground().is_none());
        assert_eq!(null_fixed.data(), Some(geometry));
        drop(style);
    });
}
#[test]
fn layered_paths_preserve_source_colors_tags_opacity_and_brush_identity() {
    run(|| {
        let p = RibbonIconPresenter::new();
        let blue = brush(0xff0000ff);
        p.set_foreground(Some(blue.clone()));
        text_icon(
            &p,
            "[viewbox=32;stroke=1.5;opacity=.4]M0,0 L8,8|{color=#f00}M0,0 L4,4|M0,0 L2,2|bogus",
        );
        let view = child(&p).cast::<Viewbox>().unwrap();
        let canvas = view.child().unwrap().cast::<Canvas>().unwrap();
        assert_eq!((canvas.width(), canvas.height()), (32., 32.));
        assert_eq!(view.stretch_direction(), StretchDirection::Both);
        let paths = canvas.children().snapshot();
        assert_eq!(paths.len(), 3);
        let stroke = paths[0].cast::<Path>().unwrap();
        let colored = paths[1].cast::<Path>().unwrap();
        let fill = paths[2].cast::<Path>().unwrap();
        assert_eq!(stroke.stroke_thickness(), 1.5);
        assert_eq!(stroke.stroke_line_cap(), PenLineCap::Round);
        assert_eq!(stroke.stroke_join(), PenLineJoin::Round);
        assert_eq!(stroke.opacity(), 0.4);
        assert!(stroke.fill().is_none());
        assert_eq!(stroke.stroke(), Some(blue.clone()));
        assert_eq!(
            colored
                .fill()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color()
                .to_uint32(),
            0xffff0000
        );
        assert!(colored.tag().is_none());
        assert!(stroke.tag().is_some() && fill.tag().is_some());
        assert!(!Rc::ptr_eq(&stroke.tag().unwrap(), &fill.tag().unwrap()));
        let green = brush(0xff00ff00);
        p.set_foreground(Some(green.clone()));
        assert_eq!(stroke.stroke(), Some(green.clone()));
        assert_eq!(fill.fill(), Some(green));
        assert_eq!(
            colored
                .fill()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color()
                .to_uint32(),
            0xffff0000
        );
        p.set_foreground(None);
        assert!(stroke.stroke().is_some());
    });
}
struct TestImage;
impl IImage for TestImage {
    fn size(&self) -> ferroui_base::Size {
        ferroui_base::Size::new(8., 8.)
    }
    fn draw(
        &self,
        _: &mut ferroui_base::media::DrawingContext<'_>,
        _: ferroui_base::Rect,
        _: ferroui_base::Rect,
    ) {
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
#[test]
fn native_image_sources_are_shared_and_missing_app_resources_leave_a_blank_icon() {
    run(|| {
        use crate::primitives::{RibbonDeferredImage, RibbonImageLoadError};

        let p = RibbonIconPresenter::new();
        let image: Rc<dyn IImage> = Rc::new(TestImage);
        p.set_icon(Some(Rc::new(image.clone())));
        let element = child(&p).cast::<Image>().unwrap();
        assert_eq!(element.source(), Some(image));
        assert_eq!(element.stretch(), Stretch::Uniform);
        assert_eq!(element.width(), 16.);
        p.set_icon(Some(Rc::new("Assets/cut.png".to_owned())));
        assert!(p.has_icon());
        let source = child(&p).cast::<Image>().unwrap().source().unwrap();
        let failed = source
            .as_object()
            .unwrap()
            .downcast_ref::<RibbonDeferredImage>()
            .unwrap();
        assert!(matches!(
            failed.load_result().unwrap().unwrap_err().as_ref(),
            RibbonImageLoadError::MissingApplicationBaseUri
        ));

        let unpaired_text = RibbonIcon::text(Some(RibbonString::from_utf16(vec![0xd800])));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                p.set_icon(Some(unpaired_text));
            }))
            .is_err()
        );
        assert!(!p.has_icon());
        assert_eq!(p.width(), 16.);
        assert!(p.children().is_empty());
        p.set_icon(None);
        // The source enum's unnamed values use the Text fallback, not default success.
        p.set_icon(Some(RibbonIcon::new(
            RibbonIconKind::from_raw(99),
            Some("X".into()),
        )));
        assert!(child(&p).cast::<TextBlock>().is_some());
    });
}

#[test]
fn cloned_icons_keep_local_foreground_ownership_under_animation() {
    run(|| {
        use crate::compat::FontIcon;
        use ferroui_base::reactive::{IObserver, LightweightSubject};
        let property = TemplatedControl::foreground_property();
        let local = brush(0xffff0000);
        let animated = brush(0xff00ff00);
        let inherited = brush(0xff0000ff);
        for is_font in [false, true] {
            for ownership in 0..5 {
                let source = if is_font {
                    FontIcon::new().upcast::<ferroui_controls::IconElement>()
                } else {
                    let icon = PathIcon::new();
                    icon.set_data(Geometry::parse("M0,0 L10,10").unwrap());
                    icon.upcast::<ferroui_controls::IconElement>()
                };
                let style = source
                    .set_value_with_priority(property, Some(local.clone()), BindingPriority::Style)
                    .unwrap();
                let bound = LightweightSubject::<Option<Rc<dyn IBrush>>>::new();
                let binding = match ownership {
                    0 => None, // style only
                    1 => {
                        source.set_foreground(Some(local.clone()));
                        None
                    }
                    2 => {
                        source.set_foreground(None);
                        None
                    }
                    3 | 4 => {
                        let handle = source.bind(
                            property,
                            Rc::new(bound.clone()),
                            BindingPriority::LocalValue,
                        );
                        if ownership == 4 {
                            bound.on_next(None);
                        }
                        Some(handle)
                    }
                    _ => unreachable!(),
                };
                let animation = source
                    .set_value_with_priority(
                        property,
                        Some(animated.clone()),
                        BindingPriority::Animation,
                    )
                    .unwrap();
                let presenter = RibbonIconPresenter::new();
                presenter.set_foreground(Some(inherited.clone()));
                presenter.set_icon(Some(Control::boxed(source.clone())));
                let first = if is_font {
                    child(&presenter)
                        .cast::<ferroui_controls::IconElement>()
                        .unwrap()
                } else {
                    child(&presenter)
                        .cast::<Viewbox>()
                        .unwrap()
                        .child()
                        .unwrap()
                        .cast::<ferroui_controls::IconElement>()
                        .unwrap()
                };
                assert!(!first.ptr_eq(&source));
                assert_eq!(
                    first.foreground(),
                    Some(if ownership == 0 {
                        inherited.clone()
                    } else {
                        animated.clone()
                    })
                );
                presenter.set_foreground(Some(local.clone()));
                assert_eq!(
                    first.foreground(),
                    Some(if ownership == 0 {
                        local.clone()
                    } else {
                        animated.clone()
                    })
                );
                // Clones are snapshots, not live subscriptions to the original icon.
                animation.dispose();
                if binding.is_some() {
                    bound.on_next(Some(local.clone()));
                }
                assert_eq!(
                    first.foreground(),
                    Some(if ownership == 0 {
                        local.clone()
                    } else {
                        animated.clone()
                    })
                );
                presenter.set_icon_size(24.);
                let second = if is_font {
                    child(&presenter)
                        .cast::<ferroui_controls::IconElement>()
                        .unwrap()
                } else {
                    child(&presenter)
                        .cast::<Viewbox>()
                        .unwrap()
                        .child()
                        .unwrap()
                        .cast::<ferroui_controls::IconElement>()
                        .unwrap()
                };
                assert_eq!(
                    second.foreground(),
                    if ownership == 2 {
                        None
                    } else {
                        Some(local.clone())
                    }
                );
                assert!(!second.ptr_eq(&first));
                if let Some(binding) = binding {
                    binding.dispose();
                }
                style.dispose();
            }
        }
    });
}
