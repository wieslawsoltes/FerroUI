use crate::{ProjectError, Result, file_path, file_uri, relative_asset_uri, require};
use std::{io, path::Path};

pub struct DictionaryDestination {
    pub uri: String,
    pub file_name: String,
    pub reference: String,
}

/// A sibling preserves the base directory of relative assets and nested includes.
pub fn dictionary_destination(source_uri: &str, name: &str) -> Result<DictionaryDestination> {
    let source = file_path(source_uri)?;
    let directory = source
        .parent()
        .ok_or_else(|| ProjectError("Open a project file first".into()))?;
    require(
        directory.is_dir(),
        "The source file's directory no longer exists",
    )?;
    validate_name(name)?;
    let path = directory.join(name);
    require(
        available(&path)?,
        "The dictionary destination already exists",
    )?;
    let reference = relative_asset_uri(name)
        .ok_or_else(|| ProjectError("Choose a relative XAML file name".into()))?;
    Ok(DictionaryDestination {
        uri: file_uri(path)?,
        file_name: name.into(),
        reference,
    })
}

pub fn suggested_dictionary_name(source_uri: &str) -> Result<String> {
    let path = file_path(source_uri)?;
    let directory = path
        .parent()
        .ok_or_else(|| ProjectError("Open a project file first".into()))?;
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("View");
    for ordinal in 1..=100 {
        let suffix = if ordinal == 1 {
            String::new()
        } else {
            format!(".{ordinal}")
        };
        let name = format!("{stem}.Resources{suffix}.xaml");
        if validate_name(&name).is_ok() && available(&directory.join(&name))? {
            return Ok(name);
        }
    }
    Ok("ExtractedResources.xaml".into())
}

fn validate_name(name: &str) -> Result<()> {
    let path = Path::new(name);
    require(
        name.trim() == name
            && name.len() <= 255
            && !name.chars().any(char::is_control)
            && !name.contains(['/', '\\', ':', '"', '<', '>', '|', '?', '*'])
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("xaml"))
            && path.file_stem().is_some_and(|stem| !stem.is_empty()),
        "Choose a sibling .xaml file name",
    )
}

fn available(path: &Path) -> Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}
