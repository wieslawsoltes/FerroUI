use ferroribbon::{
    compat::{
        FontIcon, FontIconSource, ImageIcon, ImageIconSource, MenuFlyoutItem, PathIconSource,
    },
    primitives::RibbonIconPresenter,
};
use ferroui_base::{
    data::core::ValueTypes,
    media::{FontFamily, FontStyle, FontWeight},
};
use ferroui_controls::{Image, PathIcon, StackPanel, Viewbox};
use std::rc::Rc;

#[test]
fn independent_nonvisual_image_source_compiled_bindings_keep_presenter_snapshots() {
    super::tests::run(|| {
        use crate::image_icon_consumer::ImageViewModel;
        use ferroui_base::media::{DrawingImage, IImage};
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::image_icon_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[ImageViewModel::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("IconSourceBinding.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/IconSourceBinding.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let model = panel
                .resources()
                .try_get_value(&"Model".into())
                .unwrap()
                .unwrap();
            let model = ValueTypes::as_object(&*model)
                .unwrap()
                .cast::<ImageViewModel>()
                .unwrap();
            let presenter = panel
                .children()
                .get(0)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let source = ValueTypes::as_object(&*presenter.icon().unwrap())
                .unwrap()
                .cast::<ImageIconSource>()
                .unwrap();
            let first: Rc<dyn IImage> = DrawingImage::new().into();
            let second: Rc<dyn IImage> = DrawingImage::new().into();
            let original = presenter.children().get(0).cast::<Image>().unwrap();
            model.set_image(Some(first.clone()));
            assert!(Rc::ptr_eq(source.image_source().as_ref().unwrap(), &first));
            assert!(original.source().is_none());
            presenter.set_icon_size(24.);
            let captured = presenter.children().get(0).cast::<Image>().unwrap();
            assert!(Rc::ptr_eq(captured.source().as_ref().unwrap(), &first));
            source.set_image_source(Some(second.clone()));
            assert!(Rc::ptr_eq(model.image().as_ref().unwrap(), &second));
            assert!(Rc::ptr_eq(captured.source().as_ref().unwrap(), &first));
            presenter.set_icon_size(32.);
            assert!(Rc::ptr_eq(
                presenter
                    .children()
                    .get(0)
                    .cast::<Image>()
                    .unwrap()
                    .source()
                    .as_ref()
                    .unwrap(),
                &second
            ));
            model.set_image(None);
            assert!(source.image_source().is_none());
        }
    });
}
#[test]
fn independent_icon_sources_initialize_before_snapshots_in_compiled_and_runtime_markup() {
    super::tests::run(|| {
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("IconSources.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/IconSources.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let presenter = panel
                .children()
                .get(0)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let source = ValueTypes::as_object(&*presenter.icon().unwrap())
                .unwrap()
                .cast::<FontIconSource>()
                .unwrap();
            let font = presenter.children().get(0).cast::<FontIcon>().unwrap();
            assert_eq!(font.glyph().as_deref(), Some("Source"));
            assert_eq!(font.font_family(), FontFamily::new("Consumer Symbols"));
            assert_eq!(font.font_size(), 23.5);
            assert_eq!(font.font_weight(), FontWeight::Bold);
            assert_eq!(font.font_style(), FontStyle::Normal);
            assert!(!font.mirrored_when_right_to_left());
            assert_eq!(font.foreground(), source.foreground());
            source.set_glyph(Some("Later".into()));
            assert_eq!(font.glyph().as_deref(), Some("Source"));
            presenter.set_icon_size(32.);
            let updated = presenter.children().get(0).cast::<FontIcon>().unwrap();
            assert_eq!(updated.glyph().as_deref(), Some("Later"));
            assert!(!updated.ptr_eq(&font));
            let p = panel
                .children()
                .get(1)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let path_source = ValueTypes::as_object(&*p.icon().unwrap())
                .unwrap()
                .cast::<PathIconSource>()
                .unwrap();
            let box_ = p.children().get(0).cast::<Viewbox>().unwrap();
            let path = box_.child().unwrap().cast::<PathIcon>().unwrap();
            assert!(path.data().unwrap().ptr_eq(&path_source.data().unwrap()));
            assert_eq!(path.width(), 23.5);
            assert_eq!(box_.width(), 23.5);
            let p = panel
                .children()
                .get(2)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let image_source = ValueTypes::as_object(&*p.icon().unwrap())
                .unwrap()
                .cast::<ImageIconSource>()
                .unwrap();
            assert!(image_source.image_source().is_none());
            assert!(
                p.children()
                    .get(0)
                    .cast::<Image>()
                    .unwrap()
                    .source()
                    .is_none()
            );
            let inline = panel
                .children()
                .get(3)
                .cast::<RibbonIconPresenter>()
                .unwrap()
                .children()
                .get(0)
                .cast::<FontIcon>()
                .unwrap();
            assert_eq!(inline.glyph().as_deref(), Some("Inline"));
            let menu = panel.children().get(4).cast::<MenuFlyoutItem>().unwrap();
            let menu_font = ValueTypes::as_object(&*menu.icon().unwrap())
                .unwrap()
                .cast::<FontIcon>()
                .unwrap();
            assert_eq!(menu_font.glyph().as_deref(), Some("Source"));
            assert_eq!(menu_font.font_size(), 20.);
            assert_eq!(menu_font.font_weight(), FontWeight::Normal);
            assert_eq!(menu_font.font_family(), FontIcon::new().font_family());
            let menu = panel.children().get(5).cast::<MenuFlyoutItem>().unwrap();
            let menu_path = ValueTypes::as_object(&*menu.icon().unwrap())
                .unwrap()
                .cast::<PathIcon>()
                .unwrap();
            assert!(
                menu_path
                    .data()
                    .unwrap()
                    .ptr_eq(&path_source.data().unwrap())
            );
            let menu = panel.children().get(6).cast::<MenuFlyoutItem>().unwrap();
            assert!(
                ValueTypes::as_object(&*menu.icon().unwrap())
                    .unwrap()
                    .cast::<ImageIcon>()
                    .unwrap()
                    .source()
                    .is_none()
            );
        }
    });
}
