//! Public native asset lookup assertions shared by library, consumer and real Skia.
use ferroribbon::{compat::ImageIcon, controls::RibbonItemHelper, primitives::RibbonIconPresenter};
use ferroribbon_core::model::RibbonIcon;
use ferroui_base::{
    BoxedValue,
    media::IImage,
    platform::{AssetLoader, register_assets},
};
use ferroui_controls::Application;
use std::rc::Rc;
#[path = "menu_image_uri_data.rs"]
mod source_data;
type ImageAssertion = fn(&Rc<dyn IImage>);

pub fn assert_source_resource_routes(verify: Option<ImageAssertion>) {
    const PNG: &[u8] = include_bytes!("images/bitmap-3x2.png");
    let app = Application::current().unwrap();
    let key = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
    let old = app.resources().try_get_resource(&key.into(), None);
    for (index, &(input, scheme, expected_path)) in source_data::CASES.iter().enumerate() {
        let authority = if scheme == "ferres" {
            "menu_uri_resource_probe".to_owned()
        } else {
            format!("ferroribbon_menu_uri_case_{index}")
        };
        // Only the exact source-selected file exists in this case's asset universe.
        // A global trim, or retaining the wrong suffix, cannot pass by opening a
        // different file that happens to contain the same PNG bytes.
        register_assets(&authority, &[(expected_path, PNG)]);
        AssetLoader::invalidate_assembly_cache(&authority);
        app.resources()
            .set(key, Some(Rc::new(format!("ferres://{authority}/"))));
        let descriptions: [BoxedValue; 2] = [
            Rc::new(input.to_owned()),
            Rc::new(RibbonIcon::image(Some(input.into()))),
        ];
        for description in descriptions {
            let icon = RibbonItemHelper::create_menu_icon(Some(description))
                .unwrap_or_else(|error| panic!("{input:?}: {error}"))
                .unwrap()
                .cast::<ImageIcon>()
                .unwrap();
            let image = icon.source().unwrap();
            if let Some(verify) = verify {
                verify(&image);
            }
        }
    }
    if let Some(value) = old {
        app.resources().set(key, value);
    } else {
        app.resources().remove(&key.into());
    }
}
