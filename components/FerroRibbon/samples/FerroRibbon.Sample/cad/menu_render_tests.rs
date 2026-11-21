//! The original rasterizer fixtures use actual SkiaSharp, not drawing descriptors.
use super::build;
use crate::cad::CadMenuIcons;
use ferroribbon::{
    compat::{ImageIcon, MenuFlyoutItem, MenuFlyoutSeparator, MenuFlyoutSubItem},
    metadata::control_items::RibbonControlItems,
};
use ferroribbon_core::model::RibbonIcon;
use ferroui_base::{
    PixelRect,
    media::{Color, imaging::Bitmap},
    platform::PixelFormat,
    threading::CancellationToken,
};
use ferroui_controls::{Control, StackPanel, Window};
use ferroui_headless::HeadlessUnitTestSession;
use serde_json::Value;
use std::{io::Cursor, path::PathBuf, rc::Rc};

fn capture_directory() -> PathBuf {
    let directory = std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ferroribbon"));
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).expect("create CAD capture directory");
    directory
}

fn source() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/cad-menu-icons.json")).unwrap()
}
fn png(row: &Value) -> Vec<u8> {
    row["png"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| u8::try_from(v.as_u64().unwrap()).unwrap())
        .collect()
}
fn pixels(data: &[u8]) -> Vec<u8> {
    let bitmap = Bitmap::from_stream(&mut Cursor::new(data)).unwrap();
    let size = bitmap.pixel_size();
    let mut bytes = vec![0; usize::try_from(size.width * size.height * 4).unwrap()];
    bitmap.copy_pixels(
        PixelRect::new(0, 0, size.width, size.height),
        &mut bytes,
        size.width * 4,
    );
    match bitmap.format().unwrap() {
        PixelFormat::RGBA8888 => {}
        PixelFormat::BGRA8888 => {
            for p in bytes.as_chunks_mut::<4>().0 {
                p.swap(0, 2)
            }
        }
        other => panic!("Unexpected native bitmap format {other:?}"),
    }
    bytes
}
#[test]
#[ignore = "Actual Skia reference cases; the separate full pixel comparison remains strict"]
fn source_exact_boundaries_and_svg_winding() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                crate::register_types();
                let source = source();
                let mut checked = 0;
                for row in source["custom"].as_array().unwrap() {
                    let name = row["name"].as_str().unwrap();
                    // These four cases have a separate, still-failing pixel assertion below.
                    // Direct Skia153 (without FerroUI or this port) reproduces the stroke/alpha differences.
                    if ["round-stroke", "opacity", "layers", "fixed-alpha"].contains(&name) {
                        continue;
                    }
                    let c = &row["color"];
                    let byte = |i: usize| c[i].as_u64().unwrap() as u8;
                    let actual = CadMenuIcons::render(
                        row["data"].as_str(),
                        f64::from_bits(row["viewBoxBits"].as_u64().unwrap()),
                        row["pixels"].as_i64().unwrap() as i32,
                        Color::from_argb(byte(3), byte(0), byte(1), byte(2)),
                    );
                    if name == "svg-nonzero" {
                        let actual = pixels(&actual.unwrap());
                        let expected = pixels(&png(&row["raster"]));
                        // Winding fills the nested center. Full edge pixels remain checked
                        // in original_cad_menu_rasters, which still reports a backend difference.
                        for (x, y) in [(8, 8), (20, 20), (32, 32)] {
                            let start = (y * 40 + x) * 4;
                            assert_eq!(
                                &actual[start..start + 4],
                                &expected[start..start + 4],
                                "{name} at {x},{y}"
                            );
                        }
                    } else if row["raster"]["error"].is_null() {
                        assert_eq!(
                            pixels(&actual.unwrap()),
                            pixels(&png(&row["raster"])),
                            "{name}"
                        );
                    } else {
                        assert!(
                            matches!(
                                actual,
                                Err(crate::cad::CadMenuIconError::InvalidPixelSize(_))
                            ),
                            "{name}"
                        );
                    }
                    checked += 1;
                }
                assert_eq!(checked, 17);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
#[test]
#[ignore = "Actual Skia SVG path rendering"]
fn original_cad_svg_coordinates_and_curves() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                // The original SkiaSharp SVG parser produced the same geometry
                // for each shorthand path and its explicit form.
                let pairs = [
                    (
                        "M+2+2 L2e1+2 L2E1 2e1 L+2 2e1 Z",
                        "M2,2 L20,2 L20,20 L2,20 Z",
                    ),
                    ("M2.,2. H20. V20. H2. Z", "M2,2 H20 V20 H2 Z"),
                    (
                        "M2,10 C3,3 7,3 9,10 L13,10 S20,3 22,10",
                        "M2,10 C3,3 7,3 9,10 L13,10 C13,10 20,3 22,10",
                    ),
                    (
                        "M2,10 Q5,2 8,10 S17,2 22,10",
                        "M2,10 Q5,2 8,10 C8,10 17,2 22,10",
                    ),
                    (
                        "M2,10 C3,3 7,3 9,10 T22,10",
                        "M2,10 C3,3 7,3 9,10 Q9,10 22,10",
                    ),
                    (
                        "M2,10 Q5,2 8,10 M12,10 T22,10",
                        "M2,10 Q5,2 8,10 M12,10 Q12,10 22,10",
                    ),
                    (
                        "M2,10 Q5,2 8,10 T14,10 20,10",
                        "M2,10 Q5,2 8,10 Q11,18 14,10 Q17,2 20,10",
                    ),
                ];
                let raster = |path: Option<&str>| {
                    let themed = Color::from_rgb(216, 222, 230);
                    pixels(&CadMenuIcons::render(path, 24., 40, themed).unwrap())
                };
                for (path, explicit) in pairs {
                    let expected = raster(Some(&format!("[stroke=2]{explicit}")));
                    assert!(expected.chunks_exact(4).any(|rgba| rgba[3] != 0), "{explicit}");
                    assert_eq!(raster(Some(&format!("[stroke=2]{path}"))), expected, "{path}");
                }
                let empty = raster(None);
                for invalid in ["M2,2 H20 V20 Z1", "M2,2 H2e V20 Z"] {
                    assert_eq!(raster(Some(invalid)), empty, "{invalid}");
                }
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Explicit actual Skia reference raster comparison"]
fn original_cad_menu_rasters() {
    let directory = capture_directory().join("cad-menu-native-frames");
    std::fs::create_dir_all(&directory).unwrap();
    let recorded = source();
    let source = if let Some(path) = std::env::var_os("FERRORIBBON_CAD_MENU_SOURCE") {
        let fresh: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        for key in ["revision", "files"] {
            assert_eq!(fresh[key], recorded[key], "CAD reference provenance: {key}");
        }
        for (table, keys) in [
            ("assets", &["owner", "name", "data", "viewbox"][..]),
            (
                "custom",
                &["name", "data", "viewBoxBits", "pixels", "color"][..],
            ),
        ] {
            let fresh_rows = fresh[table].as_array().unwrap();
            let recorded_rows = recorded[table].as_array().unwrap();
            assert_eq!(fresh_rows.len(), recorded_rows.len(), "CAD {table} count");
            for (fresh, recorded) in fresh_rows.iter().zip(recorded_rows) {
                for key in keys {
                    assert_eq!(fresh[key], recorded[key], "CAD {table} input: {key}");
                }
            }
        }
        fresh
    } else {
        recorded
    };
    let session = HeadlessUnitTestSession::start_new(build);
    let result=session.dispatch(move||{
        crate::register_types();
        let mut differences=Vec::new();
        let mut compare=|name:String,data:Option<&str>,box_size:f64,size:i32,color:Color,expected:&Value|{
            let actual=CadMenuIcons::render(data,box_size,size,color);
            if !expected["error"].is_null() {assert!(actual.is_err(),"{name}");return;}
            let actual=actual.unwrap();let expected=png(expected);
            let a=pixels(&actual);let b=pixels(&expected);assert_eq!(a.len(),b.len(),"{name}");
            let count=a.iter().zip(&b).filter(|(a,b)|a!=b).count();
            std::fs::write(directory.join(format!("{name}-native.png")),actual).unwrap();
            std::fs::write(directory.join(format!("{name}-source.png")),expected).unwrap();
            if count>0 {
                let max=a.iter().zip(&b).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
                differences.push(serde_json::json!({"name":name,"different_channels":count,"maximum_difference":max}));
            }
        };
        for row in source["custom"].as_array().unwrap() {
            let c=&row["color"];let byte=|i:usize|c[i].as_u64().unwrap() as u8;
            compare(row["name"].as_str().unwrap().to_owned(),row["data"].as_str(),f64::from_bits(row["viewBoxBits"].as_u64().unwrap()),row["pixels"].as_i64().unwrap() as i32,Color::from_argb(byte(3),byte(0),byte(1),byte(2)),&row["raster"]);
        }
        for row in source["assets"].as_array().unwrap() {for (theme,color) in [("dark",Color::from_rgb(216,222,230)),("light",Color::from_rgb(47,55,66))] {
            compare(format!("{}-{}-{theme}",row["owner"].as_str().unwrap(),row["name"].as_str().unwrap()),row["data"].as_str(),row["viewbox"].as_f64().unwrap(),40,color,&row[theme]);
        }}
        std::fs::write(directory.join("differences.json"),serde_json::to_vec_pretty(&differences).unwrap()).unwrap();
        assert!(differences.is_empty(),"{} source raster mismatches; see {directory:?}: {:?}",differences.len(),&differences[..differences.len().min(8)]);
    },CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
}
#[test]
#[ignore = "Actual native bitmap encoding/decoding and menu lifecycle"]
fn original_cache_recursion_and_native_image_lifetimes() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                crate::register_types();
                let source = source();
                let expected = &source["cache"];
                let path =
                    RibbonIcon::path_with_options(Some("M1,1 H12 V12 H1 Z".into()), 16., None);
                let first = CadMenuIcons::create(&path, true).unwrap().unwrap();
                let other_box = RibbonIcon::path_with_options(path.value.clone(), 32., None);
                let second = CadMenuIcons::create(&other_box, true).unwrap().unwrap();
                let light = CadMenuIcons::create(&path, false).unwrap().unwrap();
                let first_source = first.source().unwrap();
                assert_eq!(!first.ptr_eq(&second), expected["freshControls"]);
                assert_eq!(
                    Rc::ptr_eq(&first_source, &second.source().unwrap()),
                    expected["sharedSource"]
                );
                assert_eq!(
                    !Rc::ptr_eq(&first_source, &light.source().unwrap()),
                    expected["differentTheme"]
                );
                assert_eq!(first.width(), expected["width"].as_f64().unwrap());
                assert_eq!(first.height(), expected["height"].as_f64().unwrap());
                let child = MenuFlyoutItem::new();
                child.set_tag(ferroui_base::metadata::into_markup_value(path.clone()));
                let glyph = MenuFlyoutItem::new();
                let untouched = ImageIcon::new();
                glyph.set_icon(Some(Control::boxed(untouched.clone())));
                glyph.set_tag(ferroui_base::metadata::into_markup_value(
                    RibbonIcon::glyph(Some("g".into())),
                ));
                let sub = MenuFlyoutSubItem::new();
                sub.set_tag(ferroui_base::metadata::into_markup_value(path.clone()));
                sub.items().add(Some(child.clone().upcast()));
                sub.items().add(Some(glyph.clone().upcast()));
                let untagged = MenuFlyoutSubItem::new();
                let grand = MenuFlyoutItem::new();
                grand.set_tag(ferroui_base::metadata::into_markup_value(path.clone()));
                untagged.items().add(Some(grand.clone().upcast()));
                sub.items().add(Some(untagged.upcast()));
                let items = RibbonControlItems::new();
                items.add(Some(sub.clone().upcast()));
                let separator = MenuFlyoutSeparator::new();
                separator.set_tag(ferroui_base::metadata::into_markup_value(path.clone()));
                items.add(Some(separator.upcast()));
                items.add(None);
                CadMenuIcons::apply(items.iter(), true).unwrap();
                let image = |value: Option<ferroui_base::BoxedValue>| {
                    ferroui_base::data::core::ValueTypes::as_object(&*value.unwrap())
                        .unwrap()
                        .cast::<ImageIcon>()
                        .unwrap()
                };
                for item in [child.icon(), grand.icon(), sub.icon()] {
                    assert!(Rc::ptr_eq(&first_source, &image(item).source().unwrap()));
                }
                assert!(image(glyph.icon()).ptr_eq(&untouched));
                assert!(Rc::ptr_eq(
                    &ferroui_base::metadata::from_markup_value::<Rc<RibbonIcon>>(&child.tag())
                        .unwrap(),
                    &path
                ));
                assert!(
                    CadMenuIcons::create(&RibbonIcon::glyph(Some("x".into())), true)
                        .unwrap()
                        .is_none()
                );
                let old = image(child.icon());
                CadMenuIcons::apply(items.iter(), false).unwrap();
                let new = image(child.icon());
                assert!(!old.ptr_eq(&new));
                assert!(Rc::ptr_eq(&new.source().unwrap(), &light.source().unwrap()));
                let window = Window::new();
                let host = StackPanel::new();
                host.children().add(first.clone());
                host.children().add(second);
                window.set_content(Some(Control::boxed(host.clone())));
                window.show();
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                window.update_layout();
                assert!(
                    first
                        .get_visual_descendants()
                        .any(|v| v.cast::<ferroui_controls::Image>().is_some())
                );
                let weak = first.downgrade();
                host.children().clear();
                window.close();
                drop(first);
                ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
                assert!(
                    weak.upgrade().is_none(),
                    "Cache retains bitmaps, never icon controls"
                );
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
#[ignore = "Actual Skia compiled/runtime source CAD menu and linked split invocation"]
fn source_circle_menu_markup_and_linked_last_choice() {
    use ferroribbon::{
        controls::{RibbonItemHelper, RibbonSplitButton},
        primitives::RibbonItemContent,
    };
    use ferroui_base::{
        data::core::ValueTypes, metadata::from_markup_value, threading::Dispatcher,
    };
    use ferroui_controls::{Application, MenuFlyout, TopLevel};
    let directory = capture_directory();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                crate::register_types();
                crate::install_application_resources();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                let compiled = crate::generated::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_sample/CadMenus.xaml",
                )
                .unwrap()
                .unwrap();
                let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("../CadMenus.xaml"),
                    Some(&crate::ASSEMBLY),
                )
                .unwrap();
                for (mode, value) in [("compiled", compiled), ("runtime", runtime)] {
                    let host = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<StackPanel>()
                        .unwrap();
                    let split = host.children().get(0).cast::<RibbonSplitButton>().unwrap();
                    assert!(split.follow_last_choice());
                    let menu = split.flyout().unwrap().cast::<MenuFlyout>().unwrap();
                    assert_eq!(menu.items().count(), 8);
                    CadMenuIcons::apply(
                        menu.items().iter().map(|v| {
                            v.and_then(|v| ValueTypes::as_object(&*v))
                                .and_then(|v| v.cast::<Control>())
                        }),
                        true,
                    )
                    .unwrap();
                    let choice = ValueTypes::as_object(&*menu.items().get_at(3).unwrap())
                        .unwrap()
                        .cast::<MenuFlyoutItem>()
                        .unwrap();
                    assert_eq!(choice.text().as_deref(), Some("2-Point"));
                    let tag = from_markup_value::<Rc<RibbonIcon>>(&choice.tag()).unwrap();
                    assert!(Rc::ptr_eq(
                        &tag,
                        &crate::cad::CadIcon::get("CircleTwoPoint").unwrap()
                    ));
                    let image = ValueTypes::as_object(&*choice.icon().unwrap())
                        .unwrap()
                        .cast::<ImageIcon>()
                        .unwrap();
                    assert_eq!(
                        image.source().unwrap().size(),
                        ferroui_base::Size::new(40., 40.)
                    );
                    let converted = RibbonItemHelper::create_menu_icon(choice.icon())
                        .unwrap()
                        .unwrap()
                        .cast::<ImageIcon>()
                        .unwrap();
                    assert!(!converted.ptr_eq(&image));
                    assert!(Rc::ptr_eq(
                        &converted.source().unwrap(),
                        &image.source().unwrap()
                    ));
                    let calls = Rc::new(std::cell::Cell::new(0));
                    let observed = calls.clone();
                    choice.set_command(Some(ferroribbon::commands::adapt_core_command(
                        ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                            observed.set(observed.get() + 1)
                        }),
                    )));
                    let window = Window::new();
                    window.set_width(400.);
                    window.set_height(300.);
                    window.set_requested_theme_variant(Some(
                        ferroui_base::styling::ThemeVariant::dark(),
                    ));
                    window.set_content(Some(Control::boxed(host)));
                    window.show();
                    Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    let copy = split
                        .create_linked_copy_core()
                        .unwrap()
                        .cast::<RibbonSplitButton>()
                        .unwrap();
                    split.open_drop_down();
                    Dispatcher::ui_thread().run_jobs(None);
                    let popup = TopLevel::get_top_level(Some(&choice))
                        .expect("original source menu item reparented to actual popup");
                    popup.update_layout();
                    assert_eq!(
                        popup.actual_theme_variant(),
                        Some(ferroui_base::styling::ThemeVariant::dark()),
                        "Original CAD page requests Dark; popup follows its native owner"
                    );
                    assert!(choice.bounds().width > 0. && choice.bounds().height > 0.);
                    super::capture(&popup, &directory, &format!("cad-circle-menu-{mode}.png"));
                    choice.invoke_menu_core();
                    assert_eq!(calls.get(), 1);
                    assert!(split.last_choice().unwrap().ptr_eq(&choice));
                    assert!(copy.last_choice().unwrap().ptr_eq(&choice));
                    split.close_drop_down();
                    Dispatcher::ui_thread().run_jobs(None);
                    window.update_layout();
                    assert!(
                        split
                            .get_visual_descendants()
                            .filter_map(|v| v.cast::<RibbonItemContent>())
                            .any(|v| from_markup_value::<Rc<RibbonIcon>>(&v.icon())
                                .is_some_and(|i| Rc::ptr_eq(&i, &tag))),
                        "Last choice retains source vector Tag for primary presentation"
                    );
                    assert!(split.invoke_core());
                    assert_eq!(calls.get(), 2);
                    assert!(copy.invoke_core());
                    // Original InvokePrimary executes LastChoice before Click forwards
                    // to InvokePrimaryFromCopy on the source, which executes it again.
                    let split_reference: Value = serde_json::from_str(include_str!(
                        "../../../tests/fixtures/split-button.json"
                    ))
                    .unwrap();
                    let reference = split_reference["cases"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|c| {
                            c["name"] == "invoke"
                                && c["copied"] == true
                                && c["follow"] == true
                                && c["checkable"] == false
                                && c["last"] == "command"
                        })
                        .unwrap();
                    let copy_calls = reference["trace"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|s| s.as_str().unwrap().starts_with("choice:"))
                        .count();
                    assert_eq!(copy_calls, 2, "Original selected-source fixture");
                    assert_eq!(calls.get(), 2 + copy_calls);
                    RibbonItemHelper::unlink(&copy);
                    assert!(copy.flyout().is_none());
                    window.set_content(None);
                    window.close();
                }
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
