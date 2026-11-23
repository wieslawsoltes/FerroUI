use super::{fingerprint, paths, ManifestFile, Transfer};
use crate::{
    is_packaged_asset_uri, is_raster_asset, require, workspace::publish_bytes, PackagedAssets,
    ProjectError, Result,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const MAX_ASSET_BYTES: u64 = 64 * 1024 * 1024;
const OTHER_FORMATS: &[&str] = &[
    "svg", "ttf", "otf", "woff", "woff2", "wav", "mp3", "ogg", "mp4", "webm", "xaml", "fxaml",
];

pub struct Asset {
    pub file: PathBuf,
    pub available: bool,
    pub raster: bool,
    pub copy: Option<AssetCopy>,
}

pub struct AssetCopy {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub bytes: usize,
    digest: String,
    manifest_id: String,
}

impl ManifestFile {
    pub fn project_asset(&self, path: &str, transfer: Option<&Transfer>) -> Result<Asset> {
        let file = self.project_path(path)?;
        format(&file)?;
        let raster = is_raster_asset(&file);
        let (available, copy) = match transfer {
            Some(Transfer::Import { file: input }) => {
                let input = paths::confined_path(&self.directory, paths::relative_path(input)?)?;
                let copy = AssetCopy::prepare(self, &input, file.clone())?;
                (copy.is_none(), copy)
            }
            Some(Transfer::Export { file: output }) => (true, self.export(&file, output)?),
            None => {
                paths::existing_file(&file)?;
                (true, None)
            }
        };
        Ok(Asset {
            file,
            available,
            raster,
            copy,
        })
    }

    pub fn published_asset(
        &self,
        catalog: Option<&PackagedAssets>,
        address: &str,
        export: Option<&str>,
    ) -> Result<Asset> {
        require(
            address.len() <= 4096 && is_packaged_asset_uri(address),
            "Use an exact ferres:// address from the Cargo asset catalog",
        )?;
        let catalog = catalog.ok_or_else(|| {
            ProjectError("Build the owning Cargo package to resolve published assets".into())
        })?;
        let asset = catalog.resolve(address)?.ok_or_else(|| {
            ProjectError("The asset is absent from the active Cargo export table".into())
        })?;
        let raster = is_raster_asset(&asset.file);
        let copy = export
            .map(|output| self.export(&asset.file, output))
            .transpose()?
            .flatten();
        Ok(Asset {
            file: asset.file,
            available: true,
            raster,
            copy,
        })
    }

    fn export(&self, source: &Path, output: &str) -> Result<Option<AssetCopy>> {
        let destination = paths::confined_path(&self.directory, paths::relative_path(output)?)?;
        AssetCopy::prepare(self, source, destination)
    }
}

impl AssetCopy {
    fn prepare(
        manifest: &ManifestFile,
        source: &Path,
        destination: PathBuf,
    ) -> Result<Option<Self>> {
        let source = paths::existing_file(source)?;
        require(
            format(&source)? == format(&destination)?,
            "Preserve the asset file extension when copying",
        )?;
        let bytes = paths::read_bounded(&source, MAX_ASSET_BYTES)?;
        let digest = fingerprint(&bytes);
        match fs::symlink_metadata(&destination) {
            Ok(metadata) => {
                require(
                    metadata.is_file()
                        && metadata.len() == bytes.len() as u64
                        && fingerprint(&paths::read_bounded(&destination, MAX_ASSET_BYTES)?)
                            == digest,
                    "The destination already contains a different file",
                )?;
                Ok(None)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Some(Self {
                source,
                destination,
                bytes: bytes.len(),
                digest,
                manifest_id: manifest.identity(),
            })),
            Err(error) => Err(error.into()),
        }
    }

    pub fn fingerprint(&self) -> String {
        fingerprint(&(&self.source, &self.destination, self.bytes, &self.digest))
    }

    /// Publish the checked bytes to the reviewed pathname, with no overwrite.
    pub fn apply(&self, manifest: &ManifestFile) -> Result<()> {
        require(
            self.manifest_id == manifest.identity(),
            "The handoff file changed. Refresh the review",
        )?;
        manifest.verify()?;
        require(
            paths::existing_file(&self.source)? == self.source,
            "The asset source was redirected",
        )?;
        let bytes = paths::read_bounded(&self.source, MAX_ASSET_BYTES)?;
        require(
            fingerprint(&bytes) == self.digest,
            "The asset changed. Refresh the review",
        )?;
        paths::with_destination(&self.destination, || {
            publish_bytes(&self.destination, &bytes, false, || {
                manifest.verify()?;
                require(
                    self.destination.parent().is_some_and(|parent| {
                        parent.canonicalize().is_ok_and(|current| current == parent)
                    }),
                    "The asset destination was redirected",
                )
            })?;
            Ok(())
        })
    }
}

fn format(path: &Path) -> Result<String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    require(
        is_raster_asset(path) || OTHER_FORMATS.contains(&extension.as_str()),
        "Choose a supported image, font, media, or XAML file",
    )?;
    Ok(extension)
}
