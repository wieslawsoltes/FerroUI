//! Where tiles come from. No provider is built in: the application gives the address template
//! and the attribution its provider requires, or its own loader.

use crate::projection::TileId;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// Why a tile could not be loaded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TileError {
    /// The source has no such tile; asking again will not help.
    NotFound,
    /// The load failed and may work later.
    Failed(String),
}

impl fmt::Display for TileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("the source has no such tile"),
            Self::Failed(message) => write!(f, "the tile could not be loaded: {message}"),
        }
    }
}

impl std::error::Error for TileError {}

/// The encoded image of a tile.
pub type TileFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, TileError>>>>;

/// Loads the encoded images of tiles. Tests and offline applications implement it directly.
pub trait ITileLoader {
    fn load(&self, address: &str, tile: TileId) -> TileFuture;
}

/// Why an address template was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TileTemplateError {
    /// One of `{x}`, `{y}` or `{-y}`, and `{z}`, or else `{quadkey}`, is missing.
    MissingCoordinates,
    /// A `{...}` this template language does not have.
    UnknownPlaceholder(String),
    /// A `{` without its `}`.
    UnclosedPlaceholder,
    /// `{s}` without any subdomains.
    NoSubdomains,
    /// Not an `http`, `https` or application-defined `tile` address.
    UnsupportedScheme,
}

impl fmt::Display for TileTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCoordinates => {
                f.write_str("the template needs {z}, {x} and {y} or {-y}, or {quadkey}")
            }
            Self::UnknownPlaceholder(name) => {
                write!(f, "the template has an unknown placeholder {{{name}}}")
            }
            Self::UnclosedPlaceholder => f.write_str("the template has a { without a }"),
            Self::NoSubdomains => f.write_str("the template uses {s} but no subdomains were given"),
            Self::UnsupportedScheme => {
                f.write_str("the template must start with http://, https:// or tile://")
            }
        }
    }
}

impl std::error::Error for TileTemplateError {}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Part {
    Text(String),
    Zoom,
    X,
    Y,
    InvertedY,
    Quadkey,
    Subdomain,
}

/// A raster tile source: an address template and what the provider requires to be shown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RasterTileSource {
    template: String,
    parts: Vec<Part>,
    subdomains: Vec<String>,
    attribution: String,
    min_zoom: u8,
    max_zoom: u8,
}

impl RasterTileSource {
    /// A source from a template with `{z}`, `{x}`, `{y}`, `{-y}`, `{quadkey}` and `{s}`.
    pub fn new(template: &str, attribution: impl Into<String>) -> Result<Self, TileTemplateError> {
        Self::with_subdomains(template, attribution, &[])
    }

    /// A source whose `{s}` takes one of `subdomains`, the same one for the same tile.
    pub fn with_subdomains(
        template: &str,
        attribution: impl Into<String>,
        subdomains: &[&str],
    ) -> Result<Self, TileTemplateError> {
        let lower = template.to_ascii_lowercase();
        if !["http://", "https://", "tile://"]
            .iter()
            .any(|scheme| lower.starts_with(scheme))
        {
            return Err(TileTemplateError::UnsupportedScheme);
        }
        let parts = parse(template)?;
        let has = |part: &Part| parts.contains(part);
        let has_xyz = has(&Part::Zoom) && has(&Part::X) && (has(&Part::Y) || has(&Part::InvertedY));
        if !has_xyz && !has(&Part::Quadkey) {
            return Err(TileTemplateError::MissingCoordinates);
        }
        if has(&Part::Subdomain) && subdomains.is_empty() {
            return Err(TileTemplateError::NoSubdomains);
        }
        Ok(Self {
            template: template.to_owned(),
            parts,
            subdomains: subdomains.iter().map(|name| (*name).to_owned()).collect(),
            attribution: attribution.into(),
            min_zoom: 0,
            max_zoom: 19,
        })
    }

    /// The zoom levels the source has. Past the deepest one its tiles are scaled up.
    pub fn with_zoom_range(mut self, min_zoom: u8, max_zoom: u8) -> Self {
        self.min_zoom = min_zoom.min(max_zoom);
        self.max_zoom = max_zoom;
        self
    }

    /// The name of the folder of this source in a disk cache.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn cache_key(&self) -> String {
        crate::tile_cache::source_key(&self.template)
    }

    /// The text the provider requires next to the map.
    pub fn attribution(&self) -> &str {
        &self.attribution
    }

    pub fn min_zoom(&self) -> u8 {
        self.min_zoom
    }

    pub fn max_zoom(&self) -> u8 {
        self.max_zoom
    }

    /// The address of a tile.
    pub fn address(&self, tile: TileId) -> String {
        let mut address = String::new();
        for part in &self.parts {
            match part {
                Part::Text(text) => address.push_str(text),
                Part::Zoom => address.push_str(&tile.zoom.to_string()),
                Part::X => address.push_str(&tile.x.to_string()),
                Part::Y => address.push_str(&tile.y.to_string()),
                Part::InvertedY => {
                    let last = (1u64 << tile.zoom) - 1;
                    address.push_str(&(last - u64::from(tile.y)).to_string());
                }
                Part::Quadkey => address.push_str(&tile.quadkey()),
                Part::Subdomain => {
                    let index =
                        (u64::from(tile.x) + u64::from(tile.y)) % self.subdomains.len() as u64;
                    address.push_str(&self.subdomains[index as usize]);
                }
            }
        }
        address
    }
}

fn parse(template: &str) -> Result<Vec<Part>, TileTemplateError> {
    let mut parts = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        if open > 0 {
            parts.push(Part::Text(rest[..open].to_owned()));
        }
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or(TileTemplateError::UnclosedPlaceholder)?;
        parts.push(match &after[..close] {
            "z" => Part::Zoom,
            "x" => Part::X,
            "y" => Part::Y,
            "-y" => Part::InvertedY,
            "quadkey" => Part::Quadkey,
            "s" => Part::Subdomain,
            other => return Err(TileTemplateError::UnknownPlaceholder(other.to_owned())),
        });
        rest = &after[close + 1..];
    }
    if !rest.is_empty() {
        parts.push(Part::Text(rest.to_owned()));
    }
    Ok(parts)
}

/// A loader shared by the map and its cache.
pub type SharedTileLoader = Rc<dyn ITileLoader>;

#[cfg(test)]
mod tests {
    use super::*;

    const TILE: TileId = TileId {
        zoom: 3,
        x: 5,
        y: 2,
    };

    #[test]
    fn an_address_is_filled_in() {
        let source =
            RasterTileSource::new("https://tiles.example.test/{z}/{x}/{y}.png?v=1", "Example")
                .unwrap();
        assert_eq!(
            source.address(TILE),
            "https://tiles.example.test/3/5/2.png?v=1"
        );
        assert_eq!(source.attribution(), "Example");
    }

    #[test]
    fn an_inverted_row_counts_from_the_bottom() {
        let source = RasterTileSource::new("https://t.example.test/{z}/{x}/{-y}", "").unwrap();
        assert_eq!(source.address(TILE), "https://t.example.test/3/5/5");
    }

    #[test]
    fn a_quadkey_is_enough() {
        let source = RasterTileSource::new("tile://local/{quadkey}.png", "").unwrap();
        assert_eq!(
            source.address(TileId {
                zoom: 3,
                x: 3,
                y: 5
            }),
            "tile://local/213.png"
        );
    }

    #[test]
    fn a_tile_keeps_its_subdomain() {
        let source = RasterTileSource::with_subdomains(
            "https://{s}.example.test/{z}/{x}/{y}",
            "",
            &["a", "b", "c"],
        )
        .unwrap();
        assert_eq!(source.address(TILE), source.address(TILE));
        assert_eq!(source.address(TILE), "https://b.example.test/3/5/2");
    }

    #[test]
    fn bad_templates_are_refused() {
        use TileTemplateError as E;
        let error = |template: &str| RasterTileSource::new(template, "").unwrap_err();
        assert_eq!(error("https://e.test/{z}/{x}"), E::MissingCoordinates);
        assert_eq!(
            error("https://e.test/{z}/{x}/{y}/{key}"),
            E::UnknownPlaceholder("key".to_owned())
        );
        assert_eq!(error("https://e.test/{z}/{x}/{y"), E::UnclosedPlaceholder);
        assert_eq!(error("https://{s}.e.test/{z}/{x}/{y}"), E::NoSubdomains);
        assert_eq!(error("file:///{z}/{x}/{y}"), E::UnsupportedScheme);
    }

    #[test]
    fn the_zoom_range_is_kept_in_order() {
        let source = RasterTileSource::new("https://e.test/{z}/{x}/{y}", "")
            .unwrap()
            .with_zoom_range(9, 4);
        assert_eq!((source.min_zoom(), source.max_zoom()), (4, 4));
    }
}
