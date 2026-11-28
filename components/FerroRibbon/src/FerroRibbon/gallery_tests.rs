use super::super::{IRibbonItem, RibbonObjectItemsExt};
use super::*;
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_base::{
    Rect, Size,
    media::{Color, SolidColorBrush},
};
use ferroui_controls::Window;
fn run(test: impl FnOnce() + Send + 'static) {
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    ferroui_headless::HeadlessUnitTestSession::start_new(|| {
        ferroui_controls::AppBuilder::configure::<Application>()
            .use_headless(ferroui_headless::FerroHeadlessPlatformOptions::default())
    })
    .dispatch(
        move || {
            crate::register_types();
            let app = Application::current().unwrap();
            app.styles()
                .add(ferroui_themes_simple::SimpleTheme::new().as_style());
            let value =
                crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Gallery.xaml")
                    .unwrap()
                    .unwrap();
            let dictionary = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ferroui_base::controls::ResourceDictionary>()
                .unwrap();
            app.resources().merged_dictionaries().add(dictionary.into());
            test();
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}
fn mount(g: &Ref<RibbonGallery>) -> Ref<Window> {
    let w = Window::new();
    w.set_content(Some(Control::boxed(g.clone())));
    w.show();
    w.measure(Size::new(900., 600.));
    w.arrange(Rect::new(0., 0., 900., 600.));
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn item(label: &str, value: &str) -> Ref<RibbonGalleryItem> {
    let item = RibbonGalleryItem::new();
    item.set_label(Some(label.into()));
    item.set_item_value(Some(Rc::new(value.to_owned())));
    item.set_preview_text(Some("AaBb".into()));
    item
}
fn ribbon_item(g: &RibbonGallery) -> Rc<dyn IRibbonItem> {
    ValueTypes::try_cast(
        &Control::boxed(g.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .clone()
}
#[test]
fn source_defaults_item_properties_collection_identity_and_template_parts() {
    run(|| {
        let g = RibbonGallery::new();
        assert_eq!(g.size(), RibbonItemSize::Large);
        assert_eq!(g.item_width(), 72.);
        assert_eq!(g.item_height(), 62.);
        assert_eq!(g.rows(), 1);
        assert_eq!(g.min_columns(), 3);
        assert_eq!(g.max_columns(), 6);
        assert_eq!(g.drop_down_columns(), 0);
        assert_eq!(g.max_drop_down_height(), 460.);
        assert!(!g.is_filter_enabled());
        assert!(!g.is_drop_down_only());
        assert!(g.show_item_labels());
        assert!(g.items() == g.items());
        assert!(
            g.effective_items() == super::super::RibbonComboBox::new().effective_items(),
            "shared Array.Empty<object> identity"
        );
        assert!(g.footer_items() == g.footer_items());
        let value = RibbonGalleryItem::new();
        assert_eq!(value.preview_font_size(), 14.);
        assert_eq!(value.preview_font_weight(), FontWeight::Normal);
        assert_eq!(value.preview_font_style(), FontStyle::Normal);
        assert!(value.label().is_none());
        assert!(value.preview_background().is_none());
        g.items().add_native(Some(Rc::new(value) as BoxedValue));
        let w = mount(&g);
        let parts = g.parts.borrow();
        assert!(
            parts
                .host
                .as_ref()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&g.inline)
        );
        assert!(
            parts.more.is_some()
                && parts.up.is_some()
                && parts.down.is_some()
                && parts.dropdown.is_some()
        );
        drop(parts);
        w.close();
    });
}

#[test]
fn loaded_custom_gallery_container_uses_source_selection_border_fallback() {
    run(|| {
        let g = RibbonGallery::new();
        let b = Button::new();
        let red: Rc<dyn IBrush> =
            SolidColorBrush::with_color(Color::from_argb(255, 255, 0, 0)).into();
        let expected: Rc<dyn IBrush> =
            RibbonTheme::get_brush(Some(&b), Some("RibbonGalleryItemSelectedBorderBrush"))
                .unwrap()
                .unwrap()
                .into();
        b.set_border_brush(Some(red.clone()));
        b.set_tag(Some(Rc::new("A".to_owned())));
        g.set_selected_item(Some(Rc::new("A".to_owned())));
        g.apply_selection(&b);
        assert!(
            Rc::ptr_eq(&b.border_brush().unwrap(), &red),
            "unloaded source container leaves custom border alone"
        );
        let w = Window::new();
        w.set_content(Some(Control::boxed(b.clone())));
        w.show();
        w.measure(Size::new(300., 200.));
        w.arrange(Rect::new(0., 0., 300., 200.));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(b.is_loaded());
        g.apply_selection(&b);
        assert_eq!(
            b.border_brush()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            expected.as_solid_color_brush().unwrap().color(),
            "loaded custom style without selection states must use the source fallback brush"
        );
        w.close();
        w.set_content(None);
    });
}

#[test]
fn original_gallery_selection_fallback_matrix_preserves_repeat_brush_identity() {
    run(|| {
        use ferroui_controls::templates::FuncControlTemplate;
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery-selection.json"))
                .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        for case in cases {
            let g = RibbonGallery::new();
            let b = Button::new();
            let selected = case["selected"].as_bool().unwrap();
            let yes = case["selectedSupported"].as_bool().unwrap();
            let no = case["unselectedSupported"].as_bool().unwrap();
            let initial_color = Color::from_argb(255, 137, 18, 52);
            b.set_border_brush(Some(SolidColorBrush::with_color(initial_color).into()));
            b.set_tag(Some(Rc::new("A".to_owned())));
            b.set_template(Some(FuncControlTemplate::new(move |_, _| {
                let root = Border::new();
                RibbonGallery::set_supports_selected_state(&root, yes);
                RibbonGallery::set_supports_unselected_state(&root, no);
                root.upcast()
            })));
            b.apply_template();
            let w = Window::new();
            if case["loaded"].as_bool().unwrap() {
                w.set_content(Some(Control::boxed(b.clone())));
                w.show();
                w.measure(Size::new(300., 200.));
                w.arrange(Rect::new(0., 0., 300., 200.));
                Dispatcher::ui_thread().run_jobs(None);
            }
            assert_eq!(b.is_loaded(), case["loaded"].as_bool().unwrap());
            g.set_selected_item(Some(Rc::new(
                if selected { "A" } else { "other" }.to_owned(),
            )));
            g.apply_selection(&b);
            let first = b.border_brush().unwrap();
            g.apply_selection(&b);
            let second = b.border_brush().unwrap();
            let expected = match case["border"].as_str().unwrap() {
                "initial" => initial_color,
                "selected" => {
                    let brush: Rc<dyn IBrush> = RibbonTheme::get_brush(
                        Some(&b),
                        Some("RibbonGalleryItemSelectedBorderBrush"),
                    )
                    .unwrap()
                    .unwrap()
                    .into();
                    brush.as_solid_color_brush().unwrap().color()
                }
                "transparent" => ferroui_base::media::Colors::TRANSPARENT,
                value => panic!("unexpected source border {value}"),
            };
            assert_eq!(
                second.as_solid_color_brush().unwrap().color(),
                expected,
                "{case}"
            );
            assert_eq!(
                std::ptr::eq(first.as_object().unwrap(), second.as_object().unwrap()),
                case["sameOnRepeat"].as_bool().unwrap(),
                "{case}"
            );
            assert_eq!(b.pseudo_classes().contains(":ribbon-selected"), selected);
            w.close();
            w.set_content(None);
        }
        assert_eq!(cases.len(), 16);
    });
}

#[test]
fn template_owned_gallery_state_preserves_failed_requests_and_resets_on_replacement() {
    run(|| {
        use ferroui_controls::templates::FuncControlTemplate;
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery-selection.json"))
                .unwrap();
        let sequences = source["sequences"].as_array().unwrap();
        for sequence in sequences {
            let g = RibbonGallery::new();
            let b = Button::new();
            b.set_tag(Some(Rc::new("A".to_owned())));
            let initial = Color::from_argb(255, 137, 18, 52);
            b.set_border_brush(Some(SolidColorBrush::with_color(initial).into()));
            let yes = sequence["selectedSupported"].as_bool().unwrap();
            let no = sequence["unselectedSupported"].as_bool().unwrap();
            let template = move || {
                FuncControlTemplate::new(move |_, _| {
                    let root = Border::new();
                    RibbonGallery::set_supports_selected_state(&root, yes);
                    RibbonGallery::set_supports_unselected_state(&root, no);
                    root.upcast()
                })
            };
            b.set_template(Some(template()));
            b.apply_template();
            let w = Window::new();
            if sequence["loaded"].as_bool().unwrap() {
                w.set_content(Some(Control::boxed(b.clone())));
                w.show();
                w.measure(Size::new(300., 200.));
                w.arrange(Rect::new(0., 0., 300., 200.));
                Dispatcher::ui_thread().run_jobs(None);
            }
            for step in sequence["steps"].as_array().unwrap() {
                match step["action"].as_str().unwrap() {
                    "created" => {}
                    "template" => {
                        b.set_template(Some(template()));
                        b.apply_template();
                    }
                    action => {
                        g.set_selected_item(Some(Rc::new(
                            if action == "selected" { "A" } else { "other" }.to_owned(),
                        )));
                        g.apply_selection(&b);
                    }
                }
                let root = b.visual_children().get(0);
                let classes = root.pseudo_classes();
                let selected = classes.contains(":ribbon-gallery-selected");
                let unselected = classes.contains(":ribbon-gallery-unselected");
                assert!(!(selected && unselected));
                let actual = if selected {
                    Some("Selected")
                } else if unselected {
                    Some("Unselected")
                } else {
                    None
                };
                assert_eq!(
                    serde_json::json!(actual),
                    step["state"],
                    "{sequence} {step}"
                );
                let expected = match step["border"].as_str().unwrap() {
                    "initial" => initial,
                    "selected" => {
                        let p: Rc<dyn IBrush> = RibbonTheme::get_brush(
                            Some(&b),
                            Some("RibbonGalleryItemSelectedBorderBrush"),
                        )
                        .unwrap()
                        .unwrap()
                        .into();
                        p.as_solid_color_brush().unwrap().color()
                    }
                    "transparent" => ferroui_base::media::Colors::TRANSPARENT,
                    other => panic!("unexpected border {other}"),
                };
                assert_eq!(
                    b.border_brush()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color(),
                    expected,
                    "{sequence} {step}"
                );
            }
            w.close();
            w.set_content(None);
        }
        assert_eq!(sequences.len(), 8);
    });
}

#[test]
fn gallery_state_activation_retains_its_root_through_reentrant_template_replacement() {
    run(|| {
        use ferroui_controls::templates::FuncControlTemplate;
        use std::cell::Cell;
        let g = RibbonGallery::new();
        let b = Button::new();
        let initial = Color::from_argb(255, 137, 18, 52);
        b.set_border_brush(Some(SolidColorBrush::with_color(initial).into()));
        b.set_tag(Some(Rc::new("A".to_owned())));
        b.set_template(Some(FuncControlTemplate::new(|_, _| {
            let root = Border::new();
            RibbonGallery::set_supports_selected_state(&root, true);
            root.upcast()
        })));
        let w = Window::new();
        w.set_content(Some(Control::boxed(b.clone())));
        w.show();
        w.measure(Size::new(300., 200.));
        w.arrange(Rect::new(0., 0., 300., 200.));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(b.is_loaded());
        let root = b.visual_children().get(0);
        let old = root.downgrade();
        let classes = root.classes();
        let weak = b.downgrade();
        let entered = Rc::new(Cell::new(false));
        let out = entered.clone();
        let old_during = old.clone();
        let token = classes.add_collection_changed(Rc::new(move |_| {
            if old_during
                .upgrade()
                .is_some_and(|root| root.pseudo_classes().contains(":ribbon-gallery-selected"))
                && !out.replace(true)
            {
                let b = weak.upgrade().unwrap();
                b.set_template(Some(FuncControlTemplate::new(|_, _| {
                    Border::new().upcast()
                })));
                b.apply_template();
                assert!(
                    old_during.upgrade().is_some(),
                    "active state must retain its original root through reentry"
                );
            }
        }));
        drop(root);
        g.set_selected_item(Some(Rc::new("A".to_owned())));
        g.apply_selection(&b);
        assert!(entered.get());
        let replacement = b.visual_children().get(0);
        assert!(
            !replacement
                .pseudo_classes()
                .contains(":ribbon-gallery-selected")
        );
        assert_eq!(
            b.border_brush()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            initial,
            "original successful state attempt must not fall back using replacement state declarations"
        );
        classes.remove_collection_changed(token);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            old.upgrade().is_none(),
            "finished activation must release the detached root"
        );
        w.close();
        w.set_content(None);
    });
}
#[test]
fn source_inline_layout_columns_rows_scroll_and_collapsed_presentation() {
    run(|| {
        let g = RibbonGallery::new();
        g.set_max_columns(5);
        g.set_min_columns(2);
        g.set_rows(2);
        g.set_item_width(71.25);
        g.set_item_height(62.);
        for i in 0..31 {
            g.items().add_native(Some(Rc::new(format!("Item {i}"))));
        }
        let w = mount(&g);
        g.apply_layout(RibbonItemLayout::new(RibbonItemSize::Large, g.metrics()));
        assert_eq!(g.columns(), 5);
        assert!(g.is_inline());
        let expected = ferroui_base::utilities::math_utilities::min(
            62.,
            (g.metrics().unwrap().group_content_height() - 4. - 2.) / 2.,
        );
        assert_eq!(g.inline.item_height(), expected);
        g.scroll_rows(100);
        assert_eq!(g.first_row(), 5);
        g.scroll_rows(-1);
        assert_eq!(g.first_row(), 4);
        g.apply_layout(RibbonItemLayout::new(RibbonItemSize::Medium, g.metrics()));
        assert_eq!(g.columns(), 2);
        g.scroll_rows(i32::MIN);
        assert_eq!(g.first_row(), 0);
        g.apply_layout(RibbonItemLayout::new(RibbonItemSize::Small, g.metrics()));
        assert!(!g.is_inline());
        assert!(g.parts.borrow().dropdown.as_ref().unwrap().is_visible());
        let mut layout = RibbonItemLayout::new(RibbonItemSize::Large, g.metrics());
        layout.is_simplified = true;
        g.apply_layout(layout);
        assert!(!g.is_inline());
        g.set_min_columns(-10);
        g.set_max_columns(0);
        assert_eq!(g.columns(), 1);
        w.close();
    });
}
#[test]
fn original_element_items_survive_rebuilds_without_popup_or_copy_stealing_them() {
    run(|| {
        let g = RibbonGallery::new();
        let element = Border::new();
        element.set_width(20.);
        element.set_height(20.);
        g.items().add_native(Some(Control::boxed(element.clone())));
        let w = mount(&g);
        let first = element.parent().unwrap();
        assert!(first.cast::<Viewbox>().is_some());
        g.items().add_native(Some(Rc::new("Text item".to_owned())));
        g.set_show_item_labels(false);
        let second = element.parent().unwrap();
        assert!(second.cast::<Viewbox>().is_some());
        assert!(!first.ptr_eq(&second));
        assert!(first.cast::<Viewbox>().unwrap().child().is_none());
        assert!(
            Rc::ptr_eq(
                &first.cast::<Viewbox>().unwrap().tag().unwrap(),
                &second.cast::<Viewbox>().unwrap().tag().unwrap()
            ),
            "source static ElementHostTag identity survives rebuild"
        );
        g.open_drop_down();
        assert!(g.is_drop_down_open());
        assert!(element.parent().unwrap().ptr_eq(&second));
        let copy = ribbon_item(&g)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonGallery>()
            .unwrap();
        let cw = mount(&copy);
        assert!(element.parent().unwrap().ptr_eq(&second));
        g.close_drop_down();
        g.items().clear();
        assert!(element.parent().is_none());
        cw.close();
        w.close();
    });
}
#[test]
fn model_content_previews_accept_core_and_native_strings_without_formatting_other_values() {
    run(|| {
        use ferroribbon_core::{RibbonString, model::*};
        struct UnregisteredContent;
        let g = RibbonGallery::new();
        g.set_show_item_labels(false);
        let cases = [
            (
                Some(RibbonObject::value(RibbonString::from("Normal"))),
                Some("Normal"),
            ),
            (
                crate::metadata::model_objects::from_native(Some(Rc::new("Heading".to_owned()))),
                Some("Heading"),
            ),
            (
                Some(RibbonObject::value(RibbonString::from(
                    "A\u{301}😀 العربية",
                ))),
                Some("A\u{301}😀 العربية"),
            ),
            (Some(RibbonObject::value(RibbonString::from(""))), None),
            (Some(RibbonObject::value(42_i32)), None),
            (
                Some(RibbonObject::reference(Rc::new(UnregisteredContent))),
                None,
            ),
            (None, None),
        ];
        for (content, _) in &cases {
            let model = RibbonGalleryItemModel::new(None, Some("Hidden label".into()));
            model.set_content(content.clone());
            g.items().add_native(Some(Rc::new(model)));
        }
        let window = mount(&g);
        for (i, (_, expected)) in cases.iter().enumerate() {
            let button = g.inline.children().get(i).cast::<Button>().unwrap();
            let grid = Control::from_boxed(&button.content().unwrap())
                .unwrap()
                .cast::<Grid>()
                .unwrap();
            assert_eq!(grid.children().count(), 1, "labels remain hidden");
            let border = grid.children().get(0).cast::<Border>().unwrap();
            let text = border.child().and_then(|c| c.cast::<TextBlock>());
            assert_eq!(
                text.and_then(|t| t.text()).as_deref(),
                *expected,
                "preview {i}"
            );
        }
        window.close();
    });
}
#[test]
fn item_previews_keep_native_text_brush_font_and_value_identity() {
    run(|| {
        let g = RibbonGallery::new();
        let x = item("Heading", "chosen");
        let background: Rc<dyn IBrush> =
            SolidColorBrush::with_color(Color::from_argb(255, 20, 30, 40)).into();
        x.set_preview_background(Some(background.clone()));
        x.set_preview_font_size(23.5);
        x.set_preview_font_weight(FontWeight::Bold);
        x.set_preview_font_style(FontStyle::Italic);
        g.items().add_native(Some(Rc::new(x.clone()) as BoxedValue));
        let w = mount(&g);
        let button = g.inline.children().get(0).cast::<Button>().unwrap();
        let grid = Control::from_boxed(&button.content().unwrap())
            .unwrap()
            .cast::<Grid>()
            .unwrap();
        let border = grid.children().get(0).cast::<Border>().unwrap();
        assert_eq!(border.background(), Some(background));
        let text = border.child().unwrap().cast::<TextBlock>().unwrap();
        assert_eq!(text.text().as_deref(), Some("AaBb"));
        assert_eq!(text.font_size(), 23.5);
        assert_eq!(text.font_weight(), FontWeight::Bold);
        assert_eq!(text.font_style(), FontStyle::Italic);
        g.pick(Some(Rc::new(x.clone()) as BoxedValue));
        assert!(object_equals(&g.selected_item(), &x.value()));
        w.close();
    });
}

#[test]
fn application_text_style_cannot_change_source_gallery_preview_clipping() {
    run(|| {
        use ferroui_base::{
            Visual,
            styling::{Selectors, Setter, Style},
        };
        let app = Application::current().unwrap();
        app.styles().add(Style::with_setters(
            Selectors::of_type::<TextBlock>(),
            [
                Setter::new_untyped(
                    TextBlock::text_trimming_property().as_property(),
                    Rc::new(<dyn TextTrimming>::word_ellipsis()),
                ),
                Setter::new(Visual::clip_to_bounds_property(), false),
            ],
        ));
        let gallery = RibbonGallery::new();
        gallery.set_min_columns(1);
        gallery.set_max_columns(1);
        gallery.set_item_width(48.);
        let choice = item("Label uses character ellipsis", "choice");
        let preview = "A long preview A\u{301}😀 العربية אבג";
        choice.set_preview_text(Some(preview.into()));
        gallery.items().add_native(Some(Rc::new(choice)));
        let ordinary = TextBlock::new();
        ordinary.set_text(Some("Ordinary application text"));
        let host = StackPanel::new();
        host.children().add(ordinary.clone());
        host.children().add(gallery.clone());
        let window = Window::new();
        window.set_content(Some(Control::boxed(host)));
        window.show();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(ordinary.text_trimming().to_string(), "WordEllipsis");
        assert!(
            !ordinary.clip_to_bounds(),
            "the application style must apply"
        );
        let texts = gallery
            .get_visual_descendants()
            .filter_map(|c| c.cast::<TextBlock>())
            .collect::<Vec<_>>();
        let text = texts
            .iter()
            .find(|t| t.text().as_deref() == Some(preview))
            .unwrap();
        assert_eq!(
            (text.text_trimming().to_string(), text.clip_to_bounds()),
            ("None".into(), true),
            "source CreatePreview explicitly sets Clip, overriding application trimming",
        );
        assert!(
            text.text_layout()
                .text_lines()
                .iter()
                .all(|line| !line.has_collapsed())
        );
        let label = texts
            .iter()
            .find(|t| t.text().as_deref() == Some("Label uses character ellipsis"))
            .unwrap();
        assert_eq!(label.text_trimming().to_string(), "CharacterEllipsis");
        window.close();
        window.set_content(None);
    });
}

#[test]
fn preview_construction_matches_original_method_cases() {
    run(|| {
        use ferroui_base::styling::{IStyle, Selectors, Setter, Style};
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery-preview.json"))
                .unwrap();
        let _owner = RibbonGallery::new();
        let app = Application::current().unwrap();
        for case in reference["cases"].as_array().unwrap() {
            let input = &case["input"];
            let output = &case["output"];
            let trimming =
                <dyn TextTrimming>::parse(case["applicationTrimming"].as_str().unwrap()).unwrap();
            let style: Rc<dyn IStyle> = Style::with_setters(
                Selectors::of_type::<TextBlock>(),
                [Setter::new_untyped(
                    TextBlock::text_trimming_property().as_property(),
                    Rc::new(trimming),
                )],
            )
            .into();
            app.styles().add(style.clone());
            let background: Option<Rc<dyn IBrush>> = input["Background"]
                .as_bool()
                .unwrap()
                .then(|| SolidColorBrush::with_color(Color::from_argb(255, 1, 2, 3)).into());
            let foreground: Option<Rc<dyn IBrush>> = input["Foreground"]
                .as_bool()
                .unwrap()
                .then(|| SolidColorBrush::with_color(Color::from_argb(255, 4, 5, 6)).into());
            let icon: Option<BoxedValue> = input["Icon"]
                .as_str()
                .map(|s| Rc::new(s.to_owned()) as BoxedValue);
            let preview = create_preview(
                input["Text"].as_str().map(str::to_owned),
                icon.clone(),
                background.clone(),
                foreground.clone(),
                input["Family"].as_str().map(FontFamily::new),
                input["Size"].as_f64().unwrap(),
                FontWeight(input["Weight"].as_i64().unwrap() as i32),
                match input["Style"].as_i64().unwrap() {
                    0 => FontStyle::Normal,
                    2 => FontStyle::Italic,
                    _ => panic!("unexpected source font style"),
                },
            );
            let border = preview.cast::<Border>().unwrap();
            let window = Window::new();
            window.set_width(140.);
            window.set_height(80.);
            window.set_content(Some(Control::boxed(preview)));
            window.show();
            window.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let copied = match (border.background(), &background) {
                (None, None) => true,
                (Some(actual), Some(expected)) => Rc::ptr_eq(&actual, expected),
                _ => false,
            };
            assert_eq!(copied, output["backgroundCopied"].as_bool().unwrap());
            let margin = border.margin();
            for (actual, expected) in [margin.left, margin.top, margin.right, margin.bottom]
                .iter()
                .zip(output["margin"].as_array().unwrap())
            {
                assert_eq!(*actual, expected.as_f64().unwrap());
            }
            let radius = border.corner_radius();
            for (actual, expected) in [
                radius.top_left,
                radius.top_right,
                radius.bottom_right,
                radius.bottom_left,
            ]
            .iter()
            .zip(output["radius"].as_array().unwrap())
            {
                assert_eq!(*actual, expected.as_f64().unwrap());
            }
            match output["kind"].as_str().unwrap() {
                "text" => {
                    let text = border.child().unwrap().cast::<TextBlock>().unwrap();
                    assert_eq!(text.text().as_deref(), output["text"].as_str());
                    assert_eq!(output["trimming"], "Clip");
                    assert_eq!(text.text_trimming().to_string(), "None");
                    assert!(text.clip_to_bounds());
                    assert_eq!(text.font_size(), output["fontSize"].as_f64().unwrap());
                    assert_eq!(
                        text.font_weight().value() as i64,
                        output["weight"].as_i64().unwrap()
                    );
                    assert_eq!(
                        text.font_style().to_string(),
                        output["style"].as_str().unwrap()
                    );
                    // The seam reports only a family assigned by CreatePreview;
                    // an inherited/default platform family is not a local input.
                    assert_eq!(
                        text.get_local_value(TextBlock::font_family_property())
                            .map(|f| f.to_string())
                            .as_deref(),
                        output["family"].as_str()
                    );
                    if let Some(expected) = &foreground {
                        assert!(Rc::ptr_eq(&text.foreground().unwrap(), expected));
                    }
                }
                "icon" => {
                    let presenter = border
                        .child()
                        .unwrap()
                        .cast::<RibbonIconPresenter>()
                        .unwrap();
                    assert!(Rc::ptr_eq(
                        &presenter.icon().unwrap(),
                        icon.as_ref().unwrap()
                    ));
                    assert_eq!(presenter.icon_size(), output["iconSize"].as_f64().unwrap());
                    if let Some(expected) = &foreground {
                        assert!(Rc::ptr_eq(&presenter.foreground().unwrap(), expected));
                    } else {
                        let key = output["themeKey"].as_str().unwrap();
                        assert_eq!(key, "RibbonIconBrush");
                        let expected: Rc<dyn IBrush> =
                            RibbonTheme::get_brush(Some(&presenter), Some(key))
                                .unwrap()
                                .unwrap()
                                .into();
                        assert_eq!(
                            presenter
                                .foreground()
                                .unwrap()
                                .as_solid_color_brush()
                                .unwrap()
                                .color(),
                            expected.as_solid_color_brush().unwrap().color()
                        );
                    }
                }
                "empty" => assert!(border.child().is_none()),
                _ => panic!("unexpected source preview kind"),
            }
            window.close();
            window.set_content(None);
            app.styles().remove(style);
        }
    });
}
struct Command {
    trace: Rc<RefCell<Vec<String>>>,
    name: &'static str,
}
impl ICommand for Command {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        self.trace
            .borrow_mut()
            .push(format!("{}:can:{}", self.name, display(p)));
        true
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        self.trace
            .borrow_mut()
            .push(format!("{}:execute:{}", self.name, display(p)));
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
fn display(p: Option<&BoxedValue>) -> String {
    p.map(|p| ValueTypes::to_display_string(Some(p)))
        .unwrap_or("null".into())
}
#[test]
fn source_preview_pick_notification_command_order_and_null_preview_suppression() {
    run(|| {
        let g = RibbonGallery::new();
        let trace = Rc::new(RefCell::new(vec![]));
        g.set_command(Some(Rc::new(Command {
            trace: trace.clone(),
            name: "pick",
        })));
        g.set_preview_command(Some(Rc::new(Command {
            trace: trace.clone(),
            name: "preview",
        })));
        let log = trace.clone();
        g.add_item_preview(Some(MarkupDelegate::new(move |a| {
            let e = a[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonGalleryItemEventArgs>>()
                .unwrap();
            log.borrow_mut()
                .push(format!("event:preview:{}", display(e.value().as_ref())));
            None
        })));
        let log = trace.clone();
        g.add_item_click(Some(MarkupDelegate::new(move |a| {
            let e = a[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonGalleryItemEventArgs>>()
                .unwrap();
            log.borrow_mut()
                .push(format!("event:click:{}", display(e.value().as_ref())));
            None
        })));
        let log = trace.clone();
        let subscription = g.property_changed(move |e| {
            if e.property() == RibbonGallery::selected_item_property().as_property() {
                log.borrow_mut().push("selected".into());
            }
        });
        let x = item("Heading", "chosen");
        trace.borrow_mut().clear();
        g.preview(None);
        assert!(trace.borrow().is_empty());
        g.preview(Some(Rc::new(x.clone()) as BoxedValue));
        g.pick(Some(Rc::new(x) as BoxedValue));
        assert_eq!(
            *trace.borrow(),
            vec![
                "event:preview:chosen",
                "preview:can:chosen",
                "preview:execute:chosen",
                "event:preview:null",
                "preview:can:null",
                "preview:execute:null",
                "selected",
                "event:click:chosen",
                "pick:can:chosen",
                "pick:execute:chosen"
            ]
        );
        subscription.dispose();
    });
}
#[test]
fn loaded_source_observation_detaches_and_reloads_shared_collection() {
    run(|| {
        let g = RibbonGallery::new();
        let source = RibbonObjectItems::new_managed();
        source.add_native(Some(Rc::new("One".to_owned())));
        g.set_items_source(Some(Rc::new(source.clone())));
        assert!(g.observed.borrow().is_none());
        let w = mount(&g);
        assert!(g.observed.borrow().is_some());
        assert_eq!(g.inline.children().count(), 1);
        source.add_native(Some(Rc::new("Two".to_owned())));
        assert_eq!(g.inline.children().count(), 2);
        w.set_content(None);
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(g.observed.borrow().is_none());
        source.add_native(Some(Rc::new("Three".to_owned())));
        assert_eq!(g.inline.children().count(), 2);
        w.set_content(Some(Control::boxed(g.clone())));
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(g.inline.children().count(), 3);
        w.set_content(None);
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let weak = g.downgrade();
        drop(g);
        assert!(weak.upgrade().is_none());
        source.add_native(Some(Rc::new("After disposal".to_owned())));
        w.close();
    });
}
#[test]
fn popup_groups_filter_footer_identity_light_dismiss_and_selected_focus() {
    run(|| {
        let _culture = FilterCultureScope::new(true);
        let g = RibbonGallery::new();
        g.set_is_filter_enabled(true);
        for (label, category) in [("First", "A"), ("Second", "B"), ("Third", "A")] {
            let x = item(label, label);
            x.set_category(Some(category.into()));
            g.items().add_native(Some(Rc::new(x) as BoxedValue));
        }
        let footer = super::super::RibbonButton::new();
        footer.set_label(Some("Footer".into()));
        g.footer_items().add(Some(footer.clone().upcast()));
        let w = mount(&g);
        g.set_selected_item(Some(Rc::new("Second".to_owned())));
        g.open_drop_down();
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let popup = g.popup.borrow().clone().unwrap();
        assert!(popup.is_open());
        let tree = descendants(popup.child().unwrap()).collect::<Vec<_>>();
        assert!(tree.iter().any(|c| c.ptr_eq(&footer)));
        let panels = tree
            .iter()
            .filter_map(|c| c.cast::<RibbonUniformGridPanel>())
            .collect::<Vec<_>>();
        assert_eq!(panels.len(), 2);
        assert_eq!(
            panels.iter().map(|p| p.children().count()).sum::<usize>(),
            3
        );
        let focused = FocusManager::get_focus_manager(&g)
            .unwrap()
            .get_focused_element()
            .unwrap();
        assert!(focused.cast::<Button>().is_some());
        let filter = tree.iter().find_map(|c| c.cast::<TextBox>()).unwrap();
        filter.set_text(Some("thIRd"));
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let tree = descendants(popup.child().unwrap()).collect::<Vec<_>>();
        let panels = tree
            .iter()
            .filter_map(|c| c.cast::<RibbonUniformGridPanel>())
            .collect::<Vec<_>>();
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].children().count(), 1);
        popup.set_is_open(false);
        assert!(footer.parent().is_none());
        g.open_drop_down();
        assert!(g.is_drop_down_open());
        assert!(
            descendants(g.popup.borrow().as_ref().unwrap().child().unwrap())
                .any(|c| c.ptr_eq(&footer))
        );
        g.close_drop_down();
        w.close();
    });
}
#[test]
fn linked_copy_selection_two_way_source_items_and_single_command_invocation() {
    run(|| {
        let g = RibbonGallery::new();
        let x = item("Choice", "value");
        g.items().add_native(Some(Rc::new(x.clone()) as BoxedValue));
        let trace = Rc::new(RefCell::new(vec![]));
        g.set_command(Some(Rc::new(Command {
            trace: trace.clone(),
            name: "source",
        })));
        let copy = ribbon_item(&g)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonGallery>()
            .unwrap();
        assert!(copy.is_drop_down_only());
        assert_eq!(copy.effective_items().len(), 1);
        trace.borrow_mut().clear();
        copy.pick(Some(Rc::new(x) as BoxedValue));
        assert!(object_equals(&g.selected_item(), &copy.selected_item()));
        assert_eq!(
            trace
                .borrow()
                .iter()
                .filter(|v| v.starts_with("source:execute:"))
                .count(),
            1
        );
        g.set_selected_item(Some(Rc::new("other".to_owned())));
        assert!(object_equals(&g.selected_item(), &copy.selected_item()));
        g.items().add_native(Some(Rc::new("second".to_owned())));
        assert_eq!(copy.effective_items().len(), 2);
        RibbonItemHelper::unlink(&copy);
        assert!(copy.items_source().is_none());
    });
}
#[test]
fn original_method_layout_and_event_corpus() {
    run(|| {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        for case in data["corpus"]["layout"].as_array().unwrap() {
            let g = RibbonGallery::new();
            g.set_min_columns(case["min"].as_i64().unwrap() as i32);
            g.set_max_columns(case["max"].as_i64().unwrap() as i32);
            g.set_rows(case["rows"].as_i64().unwrap() as i32);
            g.set_item_height(case["height"].as_f64().unwrap());
            g.set_metrics(Some(g.metrics().unwrap().with_values(|m| {
                m.group_content_height = case["contentHeight"].as_f64().unwrap()
            })));
            g.set_is_drop_down_only(case["dropdown"].as_bool().unwrap());
            let mut layout = RibbonItemLayout::new(
                RibbonItemSize::from_raw(case["state"].as_i64().unwrap() as i32),
                g.metrics(),
            );
            layout.is_simplified = case["simplified"].as_bool().unwrap();
            g.apply_layout(layout);
            let columns = case["output"]["columns"].as_i64().unwrap() as usize;
            g.set_items_source(Some(Rc::new(RibbonFixedItems::new(
                (0..17 * columns)
                    .map(|i| Some(Rc::new(format!("Item{i}")) as BoxedValue))
                    .collect(),
            ))));
            let dropdown = RibbonButton::new();
            g.parts.borrow_mut().dropdown = Some(dropdown.clone());
            g.inline.set_first_row(30);
            g.apply_presentation();
            let output = &case["output"];
            assert_eq!(
                g.columns(),
                output["columns"].as_i64().unwrap() as i32,
                "{case}"
            );
            assert_eq!(g.is_inline(), output["inline"].as_bool().unwrap(), "{case}");
            assert_eq!(
                g.inline.item_height(),
                output["height"].as_f64().unwrap(),
                "{case}"
            );
            assert_eq!(
                g.inline.max_rows(),
                output["rows"].as_i64().unwrap() as i32,
                "{case}"
            );
            assert_eq!(
                g.first_row(),
                output["first"].as_i64().unwrap() as i32,
                "{case}"
            );
            assert_eq!(
                dropdown.current_size().raw(),
                output["buttonSize"].as_i64().unwrap() as i32,
                "{case}"
            );
        }
        let g = RibbonGallery::new();
        let trace = Rc::new(RefCell::new(vec![]));
        for (preview, name) in [(true, "preview"), (false, "click")] {
            let log = trace.clone();
            let handler = Some(MarkupDelegate::new(move |a| {
                let e = a[1]
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<Rc<RibbonGalleryItemEventArgs>>()
                    .unwrap();
                log.borrow_mut()
                    .push(format!("{name}:{}", display(e.value().as_ref())));
                None
            }));
            if preview {
                g.add_item_preview(handler)
            } else {
                g.add_item_click(handler)
            }
        }
        g.set_command(Some(Rc::new(Command {
            trace: trace.clone(),
            name: "pick",
        })));
        trace.borrow_mut().clear();
        let x = item("Choice", "chosen");
        let window = mount(&g);
        g.open_drop_down();
        let log = trace.clone();
        g.popup
            .borrow()
            .as_ref()
            .unwrap()
            .closed(move || log.borrow_mut().push("close:True".into()))
            .detach();
        g.preview(None);
        g.preview(Some(Rc::new(x.clone())));
        g.pick(Some(Rc::new(x.clone())));
        g.preview(None);
        let normalized = trace
            .borrow()
            .iter()
            .filter(|s| !s.starts_with("pick:can:"))
            .map(|s| s.replace("pick:execute:", "execute:"))
            .collect::<Vec<_>>();
        let expected = data["corpus"]["trace"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(normalized, expected);
        assert!(g.is_selected(&Some(Rc::new(x))));
        assert!(g.is_selected(&Some(Rc::new("chosen".to_owned()))));
        assert!(!g.is_selected(&None));
        window.close();
    });
}
#[test]
fn model_gallery_preview_and_pick_preserve_core_only_payload_identity() {
    run(|| {
        use crate::mvvm::{RibbonElementFactory, RibbonModelBindings};
        use ferroribbon_core::{commands::RibbonRelayCommand, model::*};

        struct StyleValue(u32);

        let model = RibbonGalleryModel::new(None, None);
        let item = RibbonGalleryItemModel::new(None, Some("Preview".into()));
        let payload = Rc::new(StyleValue(42));
        item.set_value(Some(RibbonObject::reference(payload.clone())));
        assert!(crate::mvvm::model_values::to_native(item.value()).is_err());
        model.items().add(Some(item.clone()));
        let bindings = RibbonModelBindings::new();
        let gallery = RibbonElementFactory::new()
            .create_item(Some(model.clone()), bindings.clone())
            .unwrap()
            .cast::<RibbonGallery>()
            .unwrap();
        let window = mount(&gallery);
        let preview = gallery.effective_items().get(0);
        gallery.preview(preview.clone());

        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let expected = payload.clone();
        let command = RibbonRelayCommand::new_with_parameter(
            move |value| {
                let value = value.unwrap().downcast_ref::<StyleValue>().unwrap();
                assert!(std::ptr::eq(value, payload.as_ref()));
                assert_eq!(value.0, 42);
                observed.set(observed.get() + 1);
            },
            Some(Rc::new(move |value| {
                if let Some(value) = value {
                    let value = value.downcast_ref::<StyleValue>().unwrap();
                    assert!(std::ptr::eq(value, expected.as_ref()));
                }
                true
            })),
        );
        model.set_preview_command(Some(command.clone()));
        gallery.preview(preview.clone());
        assert_eq!(calls.get(), 1);

        model.set_preview_command(None);
        model.set_command(Some(command.clone()));
        gallery.pick(preview.clone());
        assert_eq!(calls.get(), 2);
        assert!(Rc::ptr_eq(
            &model.selected_item().unwrap(),
            &(item as Rc<dyn RibbonGalleryItemModelImpl>),
        ));

        let raw = crate::commands::adapt_core_command(command);
        gallery.set_command(Some(raw.clone()));
        assert!(!Rc::ptr_eq(&gallery.command().unwrap(), &raw));
        gallery.pick(preview);
        assert_eq!(calls.get(), 3);

        bindings.dispose();
        window.close();
    });
}

#[test]
fn model_gallery_factory_two_way_collections_commands_replacement_and_disposal() {
    run(|| {
        use crate::mvvm::{RibbonElementFactory, RibbonModelBindings};
        use ferroribbon_core::commands::RibbonRelayCommand;
        use ferroribbon_core::model::*;
        let model = RibbonGalleryModel::new(Some("styles".into()), Some("Styles".into()));
        let x = RibbonGalleryItemModel::new(Some("one".into()), Some("Normal".into()));
        x.set_value(crate::metadata::model_objects::from_native(Some(Rc::new(
            "normal-style".to_owned(),
        ))));
        model.items().add(Some(x.clone()));
        let calls = Rc::new(RefCell::new(vec![]));
        let log = calls.clone();
        model.set_command(Some(RibbonRelayCommand::new_with_parameter(
            move |p| {
                log.borrow_mut()
                    .push(p.and_then(|p| p.downcast_ref::<String>()).unwrap().clone())
            },
            None,
        )));
        let preview = Rc::new(RefCell::new(vec![]));
        let log = preview.clone();
        model.set_preview_command(Some(RibbonRelayCommand::new_with_parameter(
            move |p| {
                log.borrow_mut()
                    .push(p.and_then(|p| p.downcast_ref::<String>()).cloned())
            },
            None,
        )));
        let factory = RibbonElementFactory::new();
        let bindings = RibbonModelBindings::new();
        let g = factory
            .create_item(Some(model.clone()), bindings.clone())
            .unwrap()
            .cast::<RibbonGallery>()
            .unwrap();
        assert_eq!(g.max_columns(), 7);
        assert_eq!(g.item_height(), 60.);
        let w = mount(&g);
        assert_eq!(g.inline.children().count(), 1);
        let button = g.inline.children().get(0).cast::<Button>().unwrap();
        assert_eq!(
            AutomationProperties::get_name(&button).as_deref(),
            Some("Normal")
        );
        let item = g.effective_items().get(0);
        g.preview(item.clone());
        assert_eq!(*preview.borrow(), vec![Some("normal-style".into())]);
        g.pick(item);
        assert_eq!(*calls.borrow(), vec!["normal-style"]);
        assert!(
            model.selected_item().is_some_and(|m| Rc::ptr_eq(
                &m,
                &(x.clone() as Rc<dyn RibbonGalleryItemModelImpl>)
            ))
        );
        assert_eq!(*preview.borrow(), vec![Some("normal-style".into()), None]);
        model.set_selected_item(None);
        assert!(g.selected_item().is_none());
        model.items().add(Some(RibbonGalleryItemModel::new(
            Some("two".into()),
            Some("Heading".into()),
        )));
        assert_eq!(g.inline.children().count(), 2);
        let trace = Rc::new(RefCell::new(vec![]));
        let raw: Rc<dyn ICommand> = Rc::new(Command {
            trace: trace.clone(),
            name: "raw",
        });
        g.set_command(Some(raw));
        trace.borrow_mut().clear();
        g.pick(g.effective_items().get(0));
        assert_eq!(
            trace
                .borrow()
                .iter()
                .filter(|s| s.as_str() == "raw:execute:normal-style")
                .count(),
            1
        );
        let footer = RibbonMenuItemModel::new(Some("footer".into()), Some("Footer".into()));
        model.menu_items().add(Some(footer));
        assert_eq!(g.footer_items().count(), 1);
        bindings.dispose();
        model.items().clear();
        assert_eq!(
            g.inline.children().count(),
            0,
            "loaded control keeps its own collection observation after factory bindings dispose"
        );
        model.set_max_columns(12);
        assert_eq!(
            g.max_columns(),
            7,
            "disposed factory no longer copies model property changes"
        ); // source remains shared, its native observation is independent of factory scope.
        w.close();
    });
}
struct LoadedCommand {
    gallery: ferroui_base::WeakRef<RibbonGallery>,
    seen: Rc<Cell<Option<bool>>>,
}
impl ICommand for LoadedCommand {
    fn can_execute(&self, _: Option<&BoxedValue>) -> bool {
        if let Some(g) = self.gallery.upgrade()
            && g.is_loaded()
            && self.seen.get().is_none()
        {
            self.seen.set(Some(g.observed.borrow().is_some()));
        }
        true
    }
    fn execute(&self, _: Option<&BoxedValue>) {}
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
#[test]
fn loaded_handler_order_keeps_base_command_before_gallery_source_observation() {
    run(|| {
        let g = RibbonGallery::new();
        g.set_items_source(Some(Rc::new(RibbonObjectItems::new_managed())));
        let seen = Rc::new(Cell::new(None));
        g.set_command(Some(Rc::new(LoadedCommand {
            gallery: g.downgrade(),
            seen: seen.clone(),
        })));
        let external = Rc::new(Cell::new(None));
        let observed = external.clone();
        let weak = g.downgrade();
        g.loaded(move |_, _| {
            observed.set(Some(weak.upgrade().unwrap().observed.borrow().is_some()))
        });
        let w = mount(&g);
        assert_eq!(
            external.get(),
            Some(true),
            "source Gallery Loaded observer is installed before external Loaded handlers"
        );
        assert_eq!(
            seen.get(),
            Some(false),
            "source base Loaded command handler runs before Gallery Loaded source observation"
        );
        assert!(g.observed.borrow().is_some());
        w.close();
    });
}
#[test]
fn repeated_presentation_preserves_containers_and_retained_parts_do_not_cycle_host() {
    run(|| {
        let g = RibbonGallery::new();
        g.set_items_source(Some(Rc::new(RibbonFixedItems::new(
            (0..256)
                .map(|i| Some(Rc::new(format!("Item {i}")) as BoxedValue))
                .collect(),
        ))));
        let w = mount(&g);
        let original = g.inline.children().to_vec();
        for step in 0..32 {
            g.set_rows(1 + step % 3);
            g.apply_layout(RibbonItemLayout::new(
                if step % 2 == 0 {
                    RibbonItemSize::Large
                } else {
                    RibbonItemSize::Medium
                },
                g.metrics(),
            ));
            g.scroll_rows(step);
            w.update_layout();
            assert!(
                g.inline
                    .children()
                    .to_vec()
                    .iter()
                    .zip(original.iter())
                    .all(|(a, b)| a.ptr_eq(b))
            );
        }
        g.open_drop_down();
        assert!(g.is_drop_down_open());
        g.close_drop_down();
        w.set_content(None);
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let weak = g.downgrade();
        drop(g);
        assert!(
            weak.upgrade().is_none(),
            "retained inline buttons and closed popup do not form a native reference cycle"
        );
        drop(original);
        w.close();
    });
}
#[test]
fn retemplate_removes_old_open_and_scroll_handlers_and_keeps_element_identity() {
    run(|| {
        let g = RibbonGallery::new();
        let element = Border::new();
        g.items().add_native(Some(Control::boxed(element.clone())));
        let w = mount(&g);
        let old_more = g.parts.borrow().more.clone().unwrap();
        let template = g.template().unwrap();
        g.set_template(None);
        g.apply_template();
        g.set_template(Some(template));
        g.apply_template();
        w.update_layout();
        old_more.raise_event(
            &ferroui_base::interactivity::RoutedEventArgs::with_event_and_source(
                Button::click_event(),
                old_more.clone(),
            ),
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!g.is_drop_down_open());
        let more = g.parts.borrow().more.clone().unwrap();
        assert!(!more.ptr_eq(&old_more));
        more.raise_event(
            &ferroui_base::interactivity::RoutedEventArgs::with_event_and_source(
                Button::click_event(),
                more.clone(),
            ),
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(g.is_drop_down_open());
        assert!(
            element
                .parent()
                .is_some_and(|p| p.cast::<Viewbox>().is_some())
        );
        g.close_drop_down();
        w.close();
    });
}
// These rules replay frozen CLR cases to test Gallery's framework dispatch only.
// They are not a general collation implementation or production fallback.
struct FixtureRules(String);
#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    target_os = "emscripten",
    target_os = "linux"
))]
#[test]
fn native_gallery_filter_matches_original_cases_without_a_fixture_provider() {
    run(|| {
        use ferroui_base::utilities::CultureInfo;
        let previous = CultureInfo::current_formatting();
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        let cases = fixture["corpus"]["filtering"].as_array().unwrap();
        for case in cases {
            CultureInfo::set_current_formatting(CultureInfo::get_culture_info(
                case["culture"].as_str().unwrap(),
            ));
            let label = Some(Rc::new(case["label"].as_str().unwrap().to_owned()) as BoxedValue);
            assert_eq!(
                filter_matches(&label, case["query"].as_str().unwrap()),
                case["matches"].as_bool().unwrap(),
                "{case}"
            );
            if case["label"] == "" {
                let item = RibbonGalleryItem::new();
                item.set_label(None);
                assert_eq!(
                    filter_matches(&Some(Rc::new(item)), case["query"].as_str().unwrap()),
                    case["matches"].as_bool().unwrap()
                );
            }
        }
        assert_eq!(cases.len(), 192);
        assert!(fixture["corpus"]["labelBoundary"]["missing"].is_null());
        assert_eq!(
            fixture["corpus"]["labelBoundary"]["nullItemError"],
            "NullReferenceException"
        );
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| filter_matches(&None, "x")))
                .is_err(),
            "a null item must not become an empty label"
        );
        CultureInfo::set_current_formatting(previous);
    });
}
impl ferroui_base::utilities::ICompareRules for FixtureRules {
    fn compare(&self, _: &str, _: &str, _: CompareOptions) -> std::cmp::Ordering {
        panic!("outside gallery fixture scope")
    }
    fn is_prefix(&self, _: &str, _: &str, _: CompareOptions) -> bool {
        panic!("outside gallery fixture scope")
    }
    fn index_of(&self, label: &str, query: &str, options: CompareOptions) -> i32 {
        assert_eq!(options, CompareOptions::IGNORE_CASE);
        let d: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        d["corpus"]["filtering"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["culture"] == self.0 && c["label"] == label && c["query"] == query)
            .expect("exact frozen CLR input")["index"]
            .as_i64()
            .unwrap() as i32
    }
}
struct FixtureProvider(bool);
impl ferroui_base::utilities::ICultureDataProvider for FixtureProvider {
    fn get_compare_rules(
        &self,
        culture: &str,
    ) -> Option<Rc<dyn ferroui_base::utilities::ICompareRules>> {
        self.0.then(|| {
            Rc::new(FixtureRules(culture.to_owned()))
                as Rc<dyn ferroui_base::utilities::ICompareRules>
        })
    }
}
struct FilterCultureScope {
    old: ferroui_base::utilities::CultureInfo,
    scope: Rc<dyn IDisposable>,
}
impl FilterCultureScope {
    fn new(enabled: bool) -> Self {
        use ferroui_base::{
            FerroLocator,
            utilities::{CultureInfo, ICultureDataProvider},
        };
        let scope = FerroLocator::enter_scope();
        let old = CultureInfo::current_formatting();
        CultureInfo::set_current_formatting(CultureInfo::get_culture_info("en-US"));
        FerroLocator::current_mutable()
            .bind::<dyn ICultureDataProvider>()
            .to_constant(Rc::new(FixtureProvider(enabled)));
        Self { old, scope }
    }
}
impl Drop for FilterCultureScope {
    fn drop(&mut self) {
        ferroui_base::utilities::CultureInfo::set_current_formatting(self.old.clone());
        self.scope.dispose();
    }
}
#[test]
fn filtering_dispatches_application_rules_before_native_matching() {
    run(|| {
        use ferroui_base::utilities::CultureInfo;
        let scope = FilterCultureScope::new(true);
        let d: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        for case in d["corpus"]["filtering"].as_array().unwrap() {
            // The framework treats invariant rules as built-in data rather than
            // consulting a registered provider. Native invariant matching is
            // covered by the separate whole-corpus test.
            if case["culture"] == "" {
                continue;
            }
            CultureInfo::set_current_formatting(CultureInfo::get_culture_info(
                case["culture"].as_str().unwrap(),
            ));
            let label = Some(Rc::new(case["label"].as_str().unwrap().to_owned()) as BoxedValue);
            assert_eq!(
                filter_matches(&label, case["query"].as_str().unwrap()),
                case["matches"].as_bool().unwrap(),
                "{case}"
            );
        }
        {
            use ferroui_base::{
                FerroLocator,
                utilities::{ICompareRules, ICultureDataProvider},
            };
            struct ApplicationRules;
            impl ICompareRules for ApplicationRules {
                fn compare(&self, _: &str, _: &str, _: CompareOptions) -> std::cmp::Ordering {
                    panic!("not used")
                }
                fn is_prefix(&self, _: &str, _: &str, _: CompareOptions) -> bool {
                    panic!("not used")
                }
                fn index_of(&self, label: &str, query: &str, options: CompareOptions) -> i32 {
                    assert_eq!(
                        (label, query, options),
                        ("unmatched", "APP-ONLY", CompareOptions::IGNORE_CASE)
                    );
                    0
                }
            }
            struct ApplicationProvider;
            impl ICultureDataProvider for ApplicationProvider {
                fn get_compare_rules(&self, _: &str) -> Option<Rc<dyn ICompareRules>> {
                    Some(Rc::new(ApplicationRules))
                }
            }
            let native_scope = FerroLocator::enter_scope();
            let previous = CultureInfo::current_formatting();
            CultureInfo::set_current_formatting(CultureInfo::get_culture_info("en-US"));
            FerroLocator::current_mutable()
                .bind::<dyn ICultureDataProvider>()
                .to_constant(Rc::new(ApplicationProvider));
            assert!(filter_matches(
                &Some(Rc::new("unmatched".to_owned())),
                "APP-ONLY"
            ));
            CultureInfo::set_current_formatting(previous);
            native_scope.dispose();
        }
        drop(scope);
        let _scope = FilterCultureScope::new(false);
        let g = RibbonGallery::new();
        g.items().add_native(Some(Rc::new("éclair".to_owned())));
        let host = StackPanel::new();
        g.fill_popup(&host, Some("\u{a0} "), 5);
        assert_eq!(
            host.children().count(),
            1,
            "blank query bypasses comparison exactly as source"
        );
        #[cfg(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        ))]
        {
            g.fill_popup(&host, Some("é"), 5);
            assert_eq!(host.children().count(), 1);
            g.fill_popup(&host, Some("absent"), 5);
            assert_eq!(host.children().count(), 0);
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            target_os = "emscripten",
            target_os = "linux"
        )))]
        {
            let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                g.fill_popup(&host, Some("é"), 5)
            }))
            .unwrap_err();
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(message.contains("requires framework culture data"));
        }
    });
}
struct NonReflexive;
impl PartialEq for NonReflexive {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
#[test]
fn selection_uses_static_object_equals_reference_shortcut_separately_from_sequence_equality() {
    run(|| {
        let d: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        let g = RibbonGallery::new();
        let value: BoxedValue = Rc::new(NonReflexive);
        g.set_selected_item(Some(value.clone()));
        assert_eq!(
            g.is_selected(&Some(value.clone())),
            d["corpus"]["objectSelection"][0].as_bool().unwrap(),
            "Object.Equals shortcuts identical references even if virtual Equals returns false"
        );
        assert_eq!(
            object_equals(&Some(value.clone()), &Some(value)),
            d["corpus"]["sequenceEquals"].as_bool().unwrap(),
            "SequenceEqual still calls the virtual comparer"
        );
        let other: BoxedValue = Rc::new(NonReflexive);
        assert_eq!(
            g.is_selected(&Some(other)),
            d["corpus"]["objectSelection"][1].as_bool().unwrap()
        );
        g.set_selected_item(Some(Rc::new(f64::NAN)));
        assert_eq!(
            g.is_selected(&Some(Rc::new(f64::NAN))),
            d["corpus"]["nanSelection"].as_bool().unwrap(),
            "boxed Double.Equals treats both NaNs as equal"
        );
    });
}
struct MutateOnEquality {
    gallery: ferroui_base::WeakRef<RibbonGallery>,
    calls: Rc<Cell<usize>>,
}
impl PartialEq for MutateOnEquality {
    fn eq(&self, _: &Self) -> bool {
        self.calls.set(self.calls.get() + 1);
        let gallery = self.gallery.upgrade().unwrap();
        let added = Button::new();
        gallery.inline.children().add(added.clone());
        gallery.inline.children().remove(added);
        false
    }
}
#[test]
fn selection_stops_when_callback_mutates_then_restores_inline_children() {
    run(|| {
        let d: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        let expected = &d["corpus"]["selectionMutation"];
        assert_eq!(expected["error"], "InvalidOperationException");
        let gallery = RibbonGallery::new();
        let calls = Rc::new(Cell::new(0));
        let value: BoxedValue = Rc::new(MutateOnEquality {
            gallery: gallery.downgrade(),
            calls: calls.clone(),
        });
        let selected: BoxedValue = Rc::new(MutateOnEquality {
            gallery: gallery.downgrade(),
            calls: calls.clone(),
        });
        let first = Button::new();
        first.set_tag(Some(value.clone()));
        let second = Button::new();
        second.set_tag(Some(value));
        gallery.inline.children().add(first);
        gallery.inline.children().add(second);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            gallery.set_selected_item(Some(selected));
        }));
        assert!(
            result.is_err(),
            "source enumeration rejects restored mutations"
        );
        assert_eq!(calls.get(), expected["calls"].as_u64().unwrap() as usize);
        assert_eq!(
            gallery.inline.children().count(),
            expected["count"].as_u64().unwrap() as usize
        );
    });
}
#[test]
fn source_deferred_open_retains_gallery_until_queued_callback_finishes() {
    run(|| {
        let g = RibbonGallery::new();
        let weak = g.downgrade();
        g.defer_open(Button::new().upcast());
        drop(g);
        assert!(
            weak.upgrade().is_some(),
            "source queued closure retains this until it runs"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "completed queued job releases the gallery"
        );
    });
}
#[test]
fn descendants_visit_lazily_after_yield_and_preserve_source_stack_order() {
    run(|| {
        let root = StackPanel::new();
        root.set_name(Some("root".into()));
        let old = Border::new();
        old.set_name(Some("old".into()));
        root.children().add(old);
        let mut walk = descendants(root.clone().upcast());
        let first = walk.next().unwrap();
        let added = Border::new();
        added.set_name(Some("new".into()));
        root.children().add(added);
        let mut names = vec![first.name().unwrap()];
        names.extend(walk.map(|v| v.name().unwrap()));
        let d: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/gallery.json")).unwrap();
        assert_eq!(serde_json::json!(names), d["corpus"]["traversal"]);
    });
}
