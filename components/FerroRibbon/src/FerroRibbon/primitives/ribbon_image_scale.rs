//! Resolves Windows-style display, theme and contrast qualifiers on packaged images.
use ferroui_base::{
    FerroObject, Rect, Size,
    media::{
        DrawingContext, IAffectsRender, IImage, ImageScaleVariant, ImageVariantContext,
        imaging::{Bitmap, IBitmap},
    },
    platform::{AssetLoader, PlatformThemeVariant},
    styling::ThemeVariant,
    utilities::{UriExtensions, UriRef},
};
use std::{any::Any, cmp::Reverse, rc::Rc};

const SCALES: [u16; 16] = [
    80, 100, 120, 125, 140, 150, 160, 175, 180, 200, 225, 250, 300, 350, 400, 450,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contrast {
    Standard,
    High,
    Black,
    White,
}

#[derive(Clone, Copy, Debug, Default)]
struct Qualifiers {
    scale: Option<u16>,
    theme: Option<PlatformThemeVariant>,
    contrast: Option<Contrast>,
}

impl Qualifiers {
    fn read(&mut self, group: &str) -> Option<()> {
        for qualifier in group.split('_') {
            let (name, value) = qualifier.split_once('-')?;
            match name.to_ascii_lowercase().as_str() {
                "scale" if self.scale.is_none() => {
                    let scale = value.parse().ok()?;
                    self.scale = SCALES.contains(&scale).then_some(scale);
                    self.scale?;
                }
                "theme" if self.theme.is_none() => {
                    self.theme = match value.to_ascii_lowercase().as_str() {
                        "light" => Some(PlatformThemeVariant::Light),
                        "dark" => Some(PlatformThemeVariant::Dark),
                        _ => return None,
                    };
                }
                "contrast" if self.contrast.is_none() => {
                    self.contrast = match value.to_ascii_lowercase().as_str() {
                        "standard" => Some(Contrast::Standard),
                        "high" => Some(Contrast::High),
                        "black" => Some(Contrast::Black),
                        "white" => Some(Contrast::White),
                        _ => return None,
                    };
                }
                _ => return None,
            }
        }
        Some(())
    }

    fn any(self) -> bool {
        self.scale.is_some() || self.theme.is_some() || self.contrast.is_some()
    }
}

pub(super) struct ImageAsset {
    pub uri: UriRef,
    qualifiers: Qualifiers,
    first_qualifier: usize,
}

impl ImageAsset {
    pub fn percent(&self) -> Option<u16> {
        self.qualifiers.scale
    }

    fn parse(uri: UriRef, logical: &str) -> Option<Self> {
        let physical = UriExtensions::get_unescape_absolute_path(uri.as_url()?);
        let (folder, name) = physical.rsplit_once('/')?;
        let (logical_folder, logical_name) = logical.rsplit_once('/')?;
        let mut expected = logical_folder.split('/').peekable();
        let mut qualifiers = Qualifiers::default();
        let mut first_qualifier = usize::MAX;
        for (index, part) in folder.split('/').enumerate() {
            if expected.peek() == Some(&part) {
                expected.next();
            } else {
                qualifiers.read(part)?;
                first_qualifier = first_qualifier.min(index);
            }
        }
        if expected.next().is_some() {
            return None;
        }

        if name != logical_name {
            let suffix = if let Some((stem, extension)) = logical_name.rsplit_once('.') {
                let (qualified, actual_extension) = name.rsplit_once('.')?;
                if extension != actual_extension {
                    return None;
                }
                qualified.strip_prefix(stem)?.strip_prefix('.')?
            } else {
                name.strip_prefix(logical_name)?.strip_prefix('.')?
            };
            qualifiers.read(suffix)?;
        }
        Some(Self {
            uri,
            qualifiers,
            first_qualifier,
        })
    }
}

pub(super) fn qualified_assets(reference: &UriRef) -> Vec<ImageAsset> {
    let Some(url) = reference.as_url().filter(|url| url.scheme() == "ferres") else {
        return Vec::new();
    };
    let logical = UriExtensions::get_unescape_absolute_path(url);
    if logical.ends_with('/') {
        return Vec::new();
    }
    let mut root = url.clone();
    root.set_path("/");
    root.set_query(None);
    root.set_fragment(None);
    let mut assets: Vec<_> = AssetLoader::get_assets(&UriRef::from(root), None)
        .into_iter()
        .filter_map(|mut candidate| {
            candidate.set_query(url.query());
            candidate.set_fragment(url.fragment());
            ImageAsset::parse(UriRef::from(candidate), &logical)
        })
        .collect();
    if !assets.iter().any(|asset| asset.qualifiers.any()) {
        assets.clear();
    }
    assets
}

pub(super) fn selected_asset(
    assets: &[ImageAsset],
    context: &ImageVariantContext,
) -> Option<usize> {
    let colors = context.platform_colors;
    let high_theme =
        context.theme.is_some() && context.theme == ThemeVariant::high_contrast_variant();
    let system_colors = colors.system_colors();
    let theme =
        ThemeVariant::to_platform_theme_variant(context.theme.as_ref()).unwrap_or_else(|| {
            if high_theme || colors.is_high_contrast() {
                system_colors.map_or(colors.theme_variant(), |system| system.theme_variant())
            } else {
                colors.theme_variant()
            }
        });
    let contrast = if high_theme || colors.is_high_contrast() {
        match system_colors.map(|system| system.window()) {
            Some(background) if (background.r, background.g, background.b) == (0, 0, 0) => {
                Contrast::Black
            }
            Some(background) if (background.r, background.g, background.b) == (255, 255, 255) => {
                Contrast::White
            }
            _ => Contrast::High,
        }
    } else {
        Contrast::Standard
    };
    select(assets, context.scaling, theme, contrast)
}

fn select(
    assets: &[ImageAsset],
    scaling: f64,
    theme: PlatformThemeVariant,
    contrast: Contrast,
) -> Option<usize> {
    assets
        .iter()
        .enumerate()
        .filter_map(|(index, asset)| {
            let q = asset.qualifiers;
            if q.theme.is_some_and(|value| value != theme)
                || q.contrast.is_some_and(|value| value != contrast)
            {
                return None;
            }
            let scale = q.scale.unwrap_or(100);
            let below = f64::from(scale) / 100. < scaling;
            let size_order = if below { u16::MAX - scale } else { scale };
            let rank = (
                q.contrast.is_none(),
                q.theme.is_none(),
                q.scale.is_none(),
                below,
                size_order,
                Reverse(asset.first_qualifier),
            );
            Some((rank, index))
        })
        .min()
        .map(|(_, index)| index)
}

type LoadAsset = dyn Fn(Option<&ImageAsset>) -> Rc<dyn IImage>;

/// The logical source retains independent selected images for native controls.
pub(super) struct QualifiedImageSource {
    assets: Vec<ImageAsset>,
    initial_index: Option<usize>,
    initial_context: ImageVariantContext,
    initial: Rc<dyn IImage>,
    load: Box<LoadAsset>,
}

impl QualifiedImageSource {
    pub fn new(
        assets: Vec<ImageAsset>,
        context: ImageVariantContext,
        load: impl Fn(Option<&ImageAsset>) -> Rc<dyn IImage> + 'static,
    ) -> Self {
        let initial_index = selected_asset(&assets, &context);
        let initial = load(initial_index.map(|index| &assets[index]));
        Self {
            assets,
            initial_index,
            initial_context: context,
            initial,
            load: Box::new(load),
        }
    }
}

impl IImage for QualifiedImageSource {
    fn size(&self) -> Size {
        self.initial.size()
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source: Rect, destination: Rect) {
        self.initial.draw(context, source, destination);
    }

    fn as_any(&self) -> &dyn Any {
        self.initial.as_any()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        self.initial.as_object()
    }

    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        self.initial.as_bitmap()
    }

    fn as_affects_render(&self) -> Option<&dyn IAffectsRender> {
        self.initial.as_affects_render()
    }

    fn resolve_for_scaling(
        &self,
        scaling: f64,
        current: Option<usize>,
    ) -> Option<ImageScaleVariant> {
        let mut context = self.initial_context.clone();
        context.scaling = scaling;
        self.resolve_for_context(&context, current)
    }

    fn resolve_for_context(
        &self,
        context: &ImageVariantContext,
        current: Option<usize>,
    ) -> Option<ImageScaleVariant> {
        let selection = selected_asset(&self.assets, context);
        let id = selection.unwrap_or(self.assets.len());
        if current == Some(id) {
            return None;
        }
        let image = if current.is_none() && selection == self.initial_index {
            self.initial.clone()
        } else {
            (self.load)(selection.map(|index| &self.assets[index]))
        };
        Some(ImageScaleVariant { id, image })
    }
}

/// Keeps decoded pixels at their native resolution and measures them in DIPs.
pub(super) struct ScaleBitmap {
    pub bitmap: Rc<Bitmap>,
    pub percent: u16,
}

impl IImage for ScaleBitmap {
    fn size(&self) -> Size {
        let size = self.bitmap.pixel_size();
        let scale = f64::from(self.percent) / 100.;
        Size::new(
            f64::from(size.width) / scale,
            f64::from(size.height) / scale,
        )
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source: Rect, destination: Rect) {
        let scale = f64::from(self.percent) / 100.;
        let source = Rect::new(
            source.x * scale,
            source.y * scale,
            source.width * scale,
            source.height * scale,
        );
        self.bitmap.draw(context, source, destination);
    }

    fn as_any(&self) -> &dyn Any {
        self.bitmap.as_any()
    }

    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        self.bitmap.as_bitmap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_image_qualifiers_match_windows_resource_manager() {
        let fixture = include_str!("../../../tests/fixtures/windows-image-qualifiers.txt");
        let (files, cases) = fixture.split_once("\n\n").unwrap();
        let files: Vec<_> = files
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|path| UriRef::parse_absolute(&format!("ferres://test/{path}")).unwrap())
            .map(UriRef::from)
            .collect();

        assert_eq!(cases.lines().count(), 400);
        for case in cases.lines() {
            let [contrast, theme, scale, name, expected]: [&str; 5] =
                case.split(';').collect::<Vec<_>>().try_into().unwrap();
            let contrast = match contrast {
                "standard" => Contrast::Standard,
                "high" => Contrast::High,
                "black" => Contrast::Black,
                "white" => Contrast::White,
                _ => panic!("unknown contrast in {case}"),
            };
            let theme = match theme {
                "light" => PlatformThemeVariant::Light,
                "dark" => PlatformThemeVariant::Dark,
                _ => panic!("unknown theme in {case}"),
            };
            let logical = format!("/Assets/{name}.png");
            let assets: Vec<_> = files
                .iter()
                .filter_map(|uri| ImageAsset::parse(uri.clone(), &logical))
                .collect();
            let scaling = scale.parse::<f64>().unwrap() / 100.;
            let selected = select(&assets, scaling, theme, contrast)
                .map(|index| &assets[index].uri)
                .map_or("-", |uri| {
                    uri.as_url().unwrap().path().trim_start_matches('/')
                });
            assert_eq!(selected, expected, "{case}");
        }
    }
}
