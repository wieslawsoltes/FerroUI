//! Independent compiled/runtime ImageIcon consumer; decoding is tested by real Skia.
use ferroribbon::{
    compat::{ImageIcon, MenuFlyoutItem},
    primitives::RibbonIconPresenter,
};
use ferroui_base::{
    data::core::ValueTypes,
    platform::{AssetLoader, register_assets},
};
use ferroui_controls::{Application, Control, StackPanel, Viewbox};
use std::rc::Rc;
#[test]
fn compiled_runtime_image_icons_and_attached_menu_conversion_share_native_sources() {
    crate::tests::run(|| {
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::image_icon_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[
            crate::image_icon_consumer::ConsumerImageIcon::TYPE,
            crate::image_icon_consumer::ImageViewModel::TYPE,
        ]);
        register_assets(
            "ferroribbon_image_icon_consumer",
            &[(
                "Assets/cut.png",
                include_bytes!("../fixtures/images/bitmap-3x2.png"),
            )],
        );
        AssetLoader::invalidate_assembly_cache("ferroribbon_image_icon_consumer");
        let app = Application::current().unwrap();
        let base = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
        let old_base = app.resources().try_get_resource(&base.into(), None);
        let old_bitmap = app
            .resources()
            .try_get_resource(&"ConsumerBitmap".into(), None);
        app.resources().set(
            base,
            Some(Rc::new(
                "ferres://ferroribbon_image_icon_consumer/".to_owned(),
            )),
        );
        let bitmap: Rc<dyn ferroui_base::media::IImage> =
            RibbonIconPresenter::try_load_bitmap_uri(&"Assets/cut.png".into()).unwrap();
        app.resources()
            .set("ConsumerBitmap", Some(Rc::new(bitmap.clone())));
        let compiled = crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ImageIcon.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("ImageIcon.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            assert_eq!(host.children().count(), 4);
            assert!(
                host.children()
                    .get(3)
                    .cast::<ImageIcon>()
                    .unwrap()
                    .source()
                    .is_none()
            );
            let presenter = host
                .children()
                .get(0)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let original = ValueTypes::as_object(&*presenter.icon().unwrap())
                .unwrap()
                .cast::<ImageIcon>()
                .unwrap();
            let clone = presenter
                .children()
                .get(0)
                .cast::<Viewbox>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<ImageIcon>()
                .unwrap();
            assert!(!clone.ptr_eq(&original));
            assert!(Rc::ptr_eq(&clone.source().unwrap(), &bitmap));
            assert_eq!((clone.width(), clone.height()), (22., 22.));
            let menu = host.children().get(1).cast::<MenuFlyoutItem>().unwrap();
            let menu_icon = Control::from_boxed(&menu.icon().unwrap())
                .unwrap()
                .cast::<ImageIcon>()
                .unwrap();
            assert!(menu_icon.source().is_some());
            let empty = host.children().get(2).cast::<ImageIcon>().unwrap();
            assert!(empty.source().is_none());
            let subclass = empty
                .cast::<crate::image_icon_consumer::ConsumerImageIcon>()
                .unwrap();
            subclass.measure(ferroui_base::Size::INFINITY);
            assert_eq!(subclass.measures(), 1);
            let vm = ValueTypes::as_object(&*subclass.data_context().unwrap())
                .unwrap()
                .cast::<crate::image_icon_consumer::ImageViewModel>()
                .unwrap();
            vm.set_image(Some(bitmap.clone()));
            assert!(Rc::ptr_eq(&subclass.source().unwrap(), &bitmap));
            subclass.set_source(None);
            assert!(vm.image().is_none());
            original.set_source(None);
            assert!(clone.source().is_some());
            presenter.set_icon_size(24.);
            assert!(
                presenter
                    .children()
                    .get(0)
                    .cast::<Viewbox>()
                    .unwrap()
                    .child()
                    .unwrap()
                    .cast::<ImageIcon>()
                    .unwrap()
                    .source()
                    .is_none()
            );
        }
        for (key, previous) in [(base, old_base), ("ConsumerBitmap", old_bitmap)] {
            if let Some(value) = previous {
                app.resources().set(key, value);
            } else {
                app.resources().remove(&key.into());
            }
        }
    });
}

#[test]
fn independent_dependency_preserves_source_menu_uri_resource_boundaries() {
    crate::tests::run(|| crate::menu_image_uri_cases::assert_source_resource_routes(None));
}
