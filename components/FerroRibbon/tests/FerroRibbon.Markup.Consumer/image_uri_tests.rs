//! Public compiled/runtime icon URI integration. Decoding is verified separately with Skia.
use ferroribbon::{
    compat::ImageIcon,
    controls::RibbonItemHelper,
    primitives::{RibbonIconPresenter, RibbonImageLoadError, observe_image_loads},
};
use ferroui_base::{
    TypeInfo,
    data::core::ValueTypes,
    platform::{AssetLoader, register_assets},
    threading::Dispatcher,
};
use ferroui_controls::{Application, Control, Image, StackPanel, Window};
use std::{cell::RefCell, rc::Rc};

#[test]
fn native_image_events_fire_through_compiled_and_runtime_markup() {
    use crate::image_event_consumer::ImageEventsView;

    crate::tests::run(|| {
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::image_event_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[ImageEventsView::TYPE]);
        register_assets(
            "ferroribbon_image_events",
            &[(
                "cut.png",
                include_bytes!("../fixtures/images/bitmap-3x2.png"),
            )],
        );
        AssetLoader::invalidate_assembly_cache("ferroribbon_image_events");
        let compiled = crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ImageEvents.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("ImageEvents.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ImageEventsView>()
                .unwrap();
            let image = view.children().get(0).cast::<Image>().unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(view.clone())));
            window.show();
            let presenter = RibbonIconPresenter::new();
            let load = |asset: &str| {
                presenter.set_icon(Some(Rc::new(format!(
                    "ferres://ferroribbon_image_events/{asset}"
                ))));
                presenter
                    .children()
                    .get(0)
                    .cast::<Image>()
                    .unwrap()
                    .source()
            };
            let failed = load("missing.png");
            image.set_source(failed.clone());
            assert!(view.events().is_empty());
            Dispatcher::ui_thread().run_jobs(None);
            let first = view.events();
            assert!(matches!(first.as_slice(), [Err(message)] if !message.is_empty()));
            let opened = load("cut.png");
            image.set_source(opened.clone());
            assert_eq!(view.events(), first);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(view.events(), [first[0].clone(), Ok(())]);
            image.set_source(failed);
            image.set_source(opened);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(view.events(), [first[0].clone(), Ok(())]);
            window.set_content(None);
            window.close();
        }
    });
}

#[test]
fn compiled_runtime_image_icons_use_native_assets_and_application_resource_mapping() {
    crate::tests::run(|| {
        register_assets(
            "ferroribbon_image_consumer",
            &[(
                "Assets/cut.png",
                include_bytes!("../fixtures/images/bitmap-3x2.png"),
            )],
        );
        AssetLoader::invalidate_assembly_cache("ferroribbon_image_consumer");
        let app = Application::current().unwrap();
        let key = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
        let previous = app.resources().try_get_resource(&key.into(), None);
        app.resources().set(
            key,
            Some(Rc::new("ferres://ferroribbon_image_consumer/".to_owned())),
        );
        let compiled = crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ImageUris.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("ImageUris.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap();
        for value in [compiled, runtime] {
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            assert_eq!(host.children().count(), 3);
            for (index, child) in host.children().snapshot().iter().enumerate() {
                let presenter = child.cast::<RibbonIconPresenter>().unwrap();
                assert!(presenter.has_icon());
                let image = presenter.children().get(0).cast::<Image>().unwrap();
                assert!(image.source().is_some());
                assert_eq!(image.width(), if index == 1 { 24. } else { 16. });
                if index == 0 {
                    assert_image_load_observation(&image);
                }
                presenter.set_icon(None);
                assert!(!presenter.has_icon());
                assert!(presenter.children().is_empty());
            }
        }
        if let Some(value) = previous {
            app.resources().set(key, value);
        } else {
            app.resources().remove(&key.into());
        }
    });
}

fn assert_image_load_observation(image: &Image) {
    let ready = image.source().unwrap();
    let failed_source = || {
        RibbonItemHelper::create_menu_icon(Some(Rc::new(
            "ferres://ferroribbon_image_consumer/Assets/missing.png".to_owned(),
        )))
        .unwrap()
        .unwrap()
        .cast::<ImageIcon>()
        .unwrap()
        .source()
        .unwrap()
    };
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let subscription = observe_image_loads(image, move |result| {
        if let Err(error) = result {
            assert!(matches!(error.as_ref(), RibbonImageLoadError::Load(_)));
        }
        observed.borrow_mut().push(result.is_ok());
    });
    let settle = || Dispatcher::ui_thread().run_jobs(None);
    settle();
    assert!(events.borrow().is_empty());

    let failed = failed_source();
    image.set_source(None);
    image.set_source(Some(ready.clone()));
    image.set_source(Some(failed.clone()));
    assert!(events.borrow().is_empty());
    let attached_after_assignment = Rc::new(RefCell::new(Vec::new()));
    let observed = attached_after_assignment.clone();
    let late_subscription = observe_image_loads(image, move |result| {
        observed.borrow_mut().push(result.is_ok());
    });
    settle();
    assert_eq!(&*events.borrow(), &[false]);
    assert_eq!(&*attached_after_assignment.borrow(), &[false]);
    drop(late_subscription);

    image.set_source(Some(ready));
    settle();
    assert_eq!(&*events.borrow(), &[false, true]);
    image.set_source(Some(failed));
    settle();
    assert_eq!(&*events.borrow(), &[false, true]);
    image.set_source(Some(failed_source()));
    drop(subscription);
    settle();
    assert_eq!(&*events.borrow(), &[false, true]);
}
