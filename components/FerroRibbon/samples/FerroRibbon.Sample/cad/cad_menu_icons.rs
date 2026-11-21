// Copyright (c) 2026 Wiesław Šoltés. Licensed under the MIT License.
//! Native port of the original CadMenuIcons.cs
use ferroribbon::compat::{ImageIcon, MenuFlyoutItem, MenuFlyoutSubItem};
use ferroribbon_core::model::{RibbonIcon, RibbonIconKind, RibbonIconLayer};
use ferroui_base::{
    Matrix, PixelSize, Ref,
    media::{
        Color, FillRule, IBrush, IImage, IPen, PathMarkupParser, Pen, PenLineCap, PenLineJoin,
        SolidColorBrush, StreamGeometry,
        imaging::{Bitmap, PngBitmapEncoderOptions, RenderTargetBitmap},
    },
    metadata::from_markup_value,
    platform::IGeometryContext,
};
use ferroui_controls::Control;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Cursor, ErrorKind},
    rc::Rc,
};

type BitmapCache = HashMap<(Option<String>, bool), Rc<dyn IImage>>;
ferroui_base::ferro_thread_local! {
    static CACHE: RefCell<BitmapCache> = RefCell::new(HashMap::new());
}

#[derive(Debug)]
pub enum CadMenuIconError {
    /// The native text/geometry boundary requires valid UTF-8.
    UnpairedSurrogate,
    InvalidPixelSize(i32),
    BitmapIo(std::io::Error),
}
impl std::fmt::Display for CadMenuIconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnpairedSurrogate => {
                f.write_str("CAD icon text contains an unpaired UTF-16 surrogate")
            }
            Self::InvalidPixelSize(size) => {
                write!(f, "CAD icon bitmap size must be positive: {size}")
            }
            Self::BitmapIo(error) => write!(f, "CAD icon bitmap: {error}"),
        }
    }
}
impl std::error::Error for CadMenuIconError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BitmapIo(error) => Some(error),
            _ => None,
        }
    }
}
impl From<std::io::Error> for CadMenuIconError {
    fn from(error: std::io::Error) -> Self {
        Self::BitmapIo(error)
    }
}

pub struct CadMenuIcons;
impl CadMenuIcons {
    /// Update the original native menu objects, including nested untagged submenus.
    /// Leave an item's previous icon in place if its new raster is unsupported or invalid.
    pub fn apply(
        items: impl IntoIterator<Item = Option<Ref<Control>>>,
        dark: bool,
    ) -> Result<(), CadMenuIconError> {
        for item in items.into_iter().flatten() {
            if let Some(sub) = item.cast::<MenuFlyoutSubItem>() {
                if let Some(icon) = Self::tagged_icon(&sub, dark)? {
                    sub.set_icon(Some(Control::boxed(icon)));
                }
                Self::apply(sub.items().iter(), dark)?;
            } else if let Some(menu) = item.cast::<MenuFlyoutItem>()
                && let Some(icon) = Self::tagged_icon(&menu, dark)?
            {
                menu.set_icon(Some(Control::boxed(icon)));
            }
        }
        Ok(())
    }
    fn tagged_icon(item: &Control, dark: bool) -> Result<Option<Ref<ImageIcon>>, CadMenuIconError> {
        let Some(icon) = from_markup_value::<Rc<RibbonIcon>>(&item.tag()) else {
            return Ok(None);
        };
        Self::create(&icon, dark)
    }
    /// Fresh 20-DIP control, shared 40-pixel bitmap. The source cache excludes ViewBox.
    pub fn create(
        icon: &RibbonIcon,
        dark: bool,
    ) -> Result<Option<Ref<ImageIcon>>, CadMenuIconError> {
        Self::create_with_renderer(icon, dark, Self::render)
    }
    fn create_with_renderer(
        icon: &RibbonIcon,
        dark: bool,
        render: impl FnOnce(Option<&str>, f64, i32, Color) -> Result<Vec<u8>, CadMenuIconError>,
    ) -> Result<Option<Ref<ImageIcon>>, CadMenuIconError> {
        if icon.kind != RibbonIconKind::Path {
            return Ok(None);
        }
        let data = icon
            .value
            .as_ref()
            .map(|v| v.to_utf8())
            .transpose()
            .map_err(|_| CadMenuIconError::UnpairedSurrogate)?;
        let key = (data, dark);
        let cached = CACHE.with_borrow(|cache| cache.get(&key).cloned());
        let source = if let Some(bitmap) = cached {
            bitmap
        } else {
            let themed = if dark {
                Color::from_rgb(0xD8, 0xDE, 0xE6)
            } else {
                Color::from_rgb(0x2F, 0x37, 0x42)
            };
            let bitmap = render(key.0.as_deref(), icon.view_box_size, 40, themed)
                .and_then(|bytes| Bitmap::from_stream(&mut Cursor::new(bytes)).map_err(Into::into));
            let bitmap: Rc<dyn IImage> = match bitmap {
                Ok(bitmap) => Rc::new(bitmap),
                // BitmapImage.SetSource failures for invalid or unsupported
                // rasters leave the original icon in place. Retry on a later call.
                Err(CadMenuIconError::BitmapIo(error))
                    if matches!(
                        error.kind(),
                        ErrorKind::InvalidInput | ErrorKind::InvalidData | ErrorKind::Unsupported
                    ) =>
                {
                    return Ok(None);
                }
                Err(error) => return Err(error),
            };
            CACHE.with_borrow_mut(|cache| cache.insert(key, bitmap.clone()));
            bitmap
        };
        let image = ImageIcon::new();
        image.set_source(Some(source));
        image.set_width(20.);
        image.set_height(20.);
        Ok(Some(image))
    }
    /// Native geometry and render target services; PNG encoding is provided by FerroUI.
    pub fn render(
        layered: Option<&str>,
        view_box: f64,
        pixels: i32,
        themed: Color,
    ) -> Result<Vec<u8>, CadMenuIconError> {
        if pixels <= 0 {
            return Err(CadMenuIconError::InvalidPixelSize(pixels));
        }
        let target = RenderTargetBitmap::new(PixelSize::new(pixels, pixels));
        let mut context = target.create_drawing_context();
        {
            // The source explicitly casts the quotient to float before scaling.
            let scale = f64::from((f64::from(pixels) / view_box) as f32);
            let _transform = context.push_transform(Matrix::create_scale(scale, scale));
            let text = layered.map(Into::into);
            for layer in RibbonIconLayer::parse(text.as_ref()).iter().flatten() {
                let data = layer
                    .data()
                    .map(|v| v.to_utf8())
                    .transpose()
                    .map_err(|_| CadMenuIconError::UnpairedSurrogate)?
                    .unwrap_or_default();
                // SKPath's SVG grammar does not accept the XAML-only F command.
                if data.contains(['F', 'f']) {
                    continue;
                }
                let geometry = StreamGeometry::new();
                let parsed = {
                    let mut builder = geometry.open();
                    builder.set_fill_rule(FillRule::NonZero);
                    let mut parser = PathMarkupParser::new(&mut builder);
                    let result = parser.parse(&data);
                    parser.dispose();
                    result
                };
                if parsed.is_err() {
                    continue;
                }
                let geometry = geometry.upcast();
                let fixed = layer
                    .color()
                    .map(|v| v.to_utf8())
                    .transpose()
                    .map_err(|_| CadMenuIconError::UnpairedSurrogate)?;
                let mut color = fixed.as_deref().and_then(parse_color).unwrap_or(themed);
                color.a = (f64::from(color.a) * layer.opacity().clamp(0., 1.)) as u8;
                let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(color).into();
                if layer.stroke_thickness() > 0. {
                    let pen: Rc<dyn IPen> = Pen::with_all(
                        Some(brush),
                        f64::from(layer.stroke_thickness() as f32),
                        None,
                        PenLineCap::Round,
                        PenLineJoin::Round,
                        4.,
                    )
                    .into();
                    context.draw_geometry(None, Some(&pen), &geometry);
                } else {
                    context.draw_geometry(Some(&brush), None, &geometry);
                }
            }
        }
        context.dispose();
        let mut bytes = Vec::new();
        target.save(&mut bytes, &PngBitmapEncoderOptions::default().into())?;
        Ok(bytes)
    }
}

/// SKColor.TryParse accepts trimmed hex with optional '#', not named/CSS colors.
/// This local boundary serves this renderer only; it is not a replacement color runtime.
pub(super) fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim();
    let hex = text.trim_start_matches('#');
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    let expand = |v: u32| ((v & 15) * 17) as u8;
    match hex.len() {
        3 => Some(Color::from_rgb(
            expand(value >> 8),
            expand(value >> 4),
            expand(value),
        )),
        4 => Some(Color::from_argb(
            expand(value >> 12),
            expand(value >> 8),
            expand(value >> 4),
            expand(value),
        )),
        6 => Some(Color::from_rgb(
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        )),
        8 => Some(Color::from_argb(
            (value >> 24) as u8,
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "desktop")]
    #[test]
    #[ignore = "Uses the native Skia bitmap decoder"]
    fn invalid_native_menu_raster_is_absent_and_retried() {
        use super::{CadMenuIconError, CadMenuIcons};
        use ferroribbon_core::model::RibbonIcon;
        use ferroui_base::threading::CancellationToken;
        use ferroui_controls::{AppBuilder, Application};
        use ferroui_desktop::AppBuilderDesktopExtensions;
        use ferroui_headless::{
            FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
        };
        use std::{io, rc::Rc};

        let session = HeadlessUnitTestSession::start_new(|| {
            AppBuilder::configure::<Application>()
                .use_platform_detect()
                .use_headless(FerroHeadlessPlatformOptions {
                    use_headless_drawing: false,
                    ..Default::default()
                })
        });
        let result = session
            .dispatch(
                || {
                    let icon = RibbonIcon::path(Some("M2,2 L11,2 L7,12 Z".into()));
                    let attempt = CadMenuIcons::create_with_renderer(&icon, true, |_, _, _, _| {
                        Ok(b"invalid PNG".to_vec())
                    });
                    assert!(attempt.unwrap().is_none());

                    for kind in [io::ErrorKind::InvalidInput, io::ErrorKind::Unsupported] {
                        let attempt =
                            CadMenuIcons::create_with_renderer(&icon, true, |_, _, _, _| {
                                Err(io::Error::from(kind).into())
                            });
                        assert!(attempt.unwrap().is_none());
                    }

                    let attempt = CadMenuIcons::create_with_renderer(&icon, true, |_, _, _, _| {
                        Err(io::Error::from(io::ErrorKind::PermissionDenied).into())
                    });
                    assert!(matches!(attempt, Err(CadMenuIconError::BitmapIo(error))
                        if error.kind() == io::ErrorKind::PermissionDenied));

                    let first = CadMenuIcons::create(&icon, true).unwrap().unwrap();
                    let next = CadMenuIcons::create_with_renderer(&icon, true, |_, _, _, _| {
                        panic!("a successful decode must be cached")
                    })
                    .unwrap()
                    .unwrap();
                    assert!(!first.ptr_eq(&next));
                    assert!(Rc::ptr_eq(
                        &first.source().unwrap(),
                        &next.source().unwrap()
                    ));
                },
                CancellationToken::none(),
            )
            .wait();
        session.dispose();
        result.unwrap();
    }

    #[test]
    fn unpaired_utf16_geometry_input_has_an_explicit_native_boundary_error() {
        let text = ferroribbon_core::RibbonString::from_utf16(vec![0xd800]);
        let icon = ferroribbon_core::model::RibbonIcon::path(Some(text.clone()));
        assert!(matches!(
            super::CadMenuIcons::create(&icon, true),
            Err(super::CadMenuIconError::UnpairedSurrogate)
        ));
        let glyph = ferroribbon_core::model::RibbonIcon::glyph(Some(text));
        assert!(super::CadMenuIcons::create(&glyph, true).unwrap().is_none());
    }
    #[test]
    fn original_skia_hex_color_boundary() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/cad-menu-icons.json"))
                .unwrap();
        for row in fixture["colors"].as_array().unwrap() {
            let actual = super::parse_color(row["input"].as_str().unwrap());
            assert_eq!(
                actual.is_some(),
                row["success"].as_bool().unwrap(),
                "{}",
                row["input"]
            );
            if let Some(c) = actual {
                assert_eq!(serde_json::json!([c.r, c.g, c.b, c.a]), row["rgba"]);
            }
        }
    }
}
