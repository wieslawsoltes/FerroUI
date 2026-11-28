use ferroribbon::controls::RibbonGallery;
use ferroribbon::mvvm::{RibbonElementFactory, RibbonModelBindings};
use ferroribbon_core::model::{RibbonGalleryModelExt, RibbonGalleryModelImpl};
use ferroui_base::{
    controls::NameScope,
    data::core::{ValueType, ValueTypes},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use std::rc::Rc;

#[test]
fn independent_preview_clipping_survives_application_styles_and_resize() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("GalleryPreviewStyle.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/GalleryPreviewStyle.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let names = NameScope::get_name_scope(&root).unwrap();
            let ordinary = names.find("Ordinary").unwrap().cast::<TextBlock>().unwrap();
            let direct = names
                .find("DirectPreview")
                .unwrap()
                .cast::<RibbonGallery>()
                .unwrap();
            let boxed = root
                .resources()
                .try_get_resource(&"PreviewModel".into(), None)
                .flatten()
                .unwrap();
            let model =
                ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn RibbonGalleryModelImpl>>())
                    .unwrap();
            let model = model
                .downcast_ref::<Rc<dyn RibbonGalleryModelImpl>>()
                .unwrap()
                .clone();
            let bindings = RibbonModelBindings::new();
            let generated = RibbonElementFactory::new()
                .create_item(Some(model.clone()), bindings.clone())
                .unwrap()
                .cast::<RibbonGallery>()
                .unwrap();
            root.children().add(generated.clone());
            let window = Window::new();
            window.set_width(320.);
            window.set_height(260.);
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            for width in [48., 91.25, 48.] {
                direct.set_item_width(width);
                model.set_item_width(width);
                window.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(ordinary.text_trimming().to_string(), "WordEllipsis");
                assert!(!ordinary.clip_to_bounds());
                let custom = root
                    .get_visual_descendants()
                    .filter_map(|v| v.cast::<TextBlock>())
                    .find(|v| v.text().as_deref() == Some("Template-owned preview"))
                    .unwrap();
                assert_eq!(custom.text_trimming().to_string(), "WordEllipsis");
                assert!(
                    !custom.clip_to_bounds(),
                    "the consumer owns its custom template"
                );
                for (gallery, prefix) in [(&direct, "Direct"), (&generated, "Model")] {
                    let texts = gallery
                        .get_visual_descendants()
                        .filter_map(|v| v.cast::<TextBlock>())
                        .collect::<Vec<_>>();
                    let text = texts
                        .iter()
                        .find(|t| {
                            t.text()
                                .is_some_and(|v| v.starts_with(&format!("{prefix} preview")))
                        })
                        .unwrap();
                    assert_eq!(
                        (text.text_trimming().to_string(), text.clip_to_bounds()),
                        ("None".into(), true),
                        "{prefix} preview: runtime={runtime}, width={width}"
                    );
                    assert!(text.bounds().width > 0. && text.bounds().width <= width);
                    assert!(
                        text.text_layout().width_including_trailing_whitespace()
                            > text.bounds().width,
                        "the test must exercise real text overflow"
                    );
                    assert!(
                        text.text_layout()
                            .text_lines()
                            .iter()
                            .all(|line| !line.has_collapsed())
                    );
                    let label = texts
                        .iter()
                        .find(|t| t.text().as_deref() == Some(&format!("{prefix} label")))
                        .unwrap();
                    assert_eq!(label.text_trimming().to_string(), "CharacterEllipsis");
                }
            }
            bindings.dispose();
            window.close();
            window.set_content(None);
        }
    });
}
