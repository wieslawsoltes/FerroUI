//! An FXP archive indexes independent FXD syntax graphs by their original asset
//! paths. Opening the index borrows the image. Only a requested document needs
//! decompression, and it can also be given to a normal FXD hot-reload target.
use super::{decode, encode::encode_columnar, expanded_size};
use crate::compiler_extensions::FerroXamlIlCompiler;
use std::{fs, path::Path};
use xamlx::exceptions::{XamlError, XamlResult};

#[path = "solid.rs"]
mod solid;
pub use solid::{compile_solid_documents, expand_solid, is_solid};

const MAGIC: &[u8; 4] = b"FXP\0";
const PACK_VERSION: u8 = 1;
const MAX_DOCUMENTS: usize = 16_384;
const MAX_PATH: usize = 4096;
const MAX_PACK_BYTES: usize = 256 << 20;

fn invalid(message: &str) -> XamlError {
    XamlError::invalid_operation(format!("Invalid FXP archive: {message}"))
}

fn check_path(path: &str) -> XamlResult<()> {
    if path.len() > MAX_PATH
        || path.contains(['\\', ':', '?', '#'])
        || path.chars().any(char::is_control)
        || path.split('/').any(|s| matches!(s, "" | "." | ".."))
    {
        return Err(invalid("expected a relative, slash-separated asset path"));
    }
    Ok(())
}

fn take<'a>(bytes: &mut &'a [u8], n: usize) -> XamlResult<&'a [u8]> {
    let (value, rest) = bytes
        .split_at_checked(n)
        .ok_or_else(|| invalid("truncated archive"))?;
    *bytes = rest;
    Ok(value)
}

fn uint(bytes: &mut &[u8]) -> XamlResult<usize> {
    Ok(u32::from_le_bytes(take(bytes, 4)?.try_into().unwrap()) as usize)
}

fn account_image(total: &mut usize, image: &[u8]) -> XamlResult<()> {
    let expanded = expanded_size(image)?;
    *total = total
        .checked_add(expanded)
        .filter(|&n| n <= MAX_PACK_BYTES)
        .ok_or_else(|| invalid("expanded archive too large"))?;
    Ok(())
}

/// A directory of unique paths in lexical order. Each borrowed image is a
/// self-contained FXD stream. Receiving hosts supply the usual XAML base URI.
pub struct ThemePack<'a> {
    entries: Vec<(&'a str, &'a [u8])>,
}

impl<'a> ThemePack<'a> {
    /// Check the complete index, byte ranges, document envelopes, and aggregate
    /// expansion budget. Syntax validation and allocation are deferred to use.
    pub fn open(bytes: &'a [u8]) -> XamlResult<Self> {
        if bytes.len() > MAX_PACK_BYTES
            || !bytes.starts_with(MAGIC)
            || bytes.get(4) != Some(&PACK_VERSION)
        {
            return Err(invalid("unexpected signature, version or length"));
        }
        let mut tail = &bytes[5..];
        let count = uint(&mut tail)?;
        let index_len = uint(&mut tail)?;
        let mut index = take(&mut tail, index_len)?;
        if count == 0 || count > MAX_DOCUMENTS || count > index.len() / 7 {
            return Err(invalid("invalid document count"));
        }
        let mut entries = Vec::with_capacity(count);
        let mut expanded_total = 0usize;
        let mut previous = None;
        for _ in 0..count {
            let len = u16::from_le_bytes(take(&mut index, 2)?.try_into().unwrap()) as usize;
            let path = std::str::from_utf8(take(&mut index, len)?)
                .map_err(|_| invalid("invalid UTF-8 path"))?;
            check_path(path)?;
            if previous.is_some_and(|p| p >= path) {
                return Err(invalid("paths must be unique and sorted"));
            }
            previous = Some(path);
            let len = uint(&mut index)?;
            let image = take(&mut tail, len)?;
            account_image(&mut expanded_total, image)?;
            entries.push((path, image));
        }
        if !index.is_empty() || !tail.is_empty() {
            return Err(invalid("trailing archive bytes"));
        }
        Ok(Self { entries })
    }

    pub fn paths(&self) -> impl ExactSizeIterator<Item = &'a str> + '_ {
        self.entries.iter().map(|(path, _)| *path)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entries
            .binary_search_by(|(key, _)| key.cmp(&path))
            .is_ok()
    }

    pub fn document(&self, path: &str) -> XamlResult<&'a [u8]> {
        self.entries
            .binary_search_by(|(key, _)| key.cmp(&path))
            .map(|i| self.entries[i].1)
            .map_err(|_| invalid(&format!("no document at {path}")))
    }

    /// Validate every graph without running constructors or retaining prior
    /// graphs. A development host can do this before publishing a replacement.
    pub fn validate(&self) -> XamlResult<()> {
        for &(path, image) in &self.entries {
            decode(image).map_err(|e| invalid(&format!("{path}: {}", e.message())))?;
        }
        Ok(())
    }
}

/// Compile a reproducible archive from the host's selected document inventory.
/// Sorting the paths does not change XAML dictionary or include ordering.
pub fn compile_documents(documents: &[(&str, &str)]) -> XamlResult<Vec<u8>> {
    if documents.is_empty() || documents.len() > MAX_DOCUMENTS {
        return Err(invalid("invalid document count"));
    }
    let mut documents = documents.to_vec();
    documents.sort_unstable_by_key(|&(path, _)| path);
    let mut index = Vec::new();
    let mut payload = Vec::new();
    let mut expanded_total = 0usize;
    let mut last = None;
    for &(path, xml) in &documents {
        check_path(path)?;
        if last == Some(path) {
            return Err(invalid("duplicate path"));
        }
        last = Some(path);
        let image = FerroXamlIlCompiler::parse_source_syntax(xml)
            .and_then(|syntax| encode_columnar(&syntax))
            .map_err(|e| invalid(&format!("{path}: {}", e.message())))?;
        account_image(&mut expanded_total, &image)?;
        index.extend_from_slice(&(path.len() as u16).to_le_bytes());
        index.extend_from_slice(path.as_bytes());
        index.extend_from_slice(&(image.len() as u32).to_le_bytes());
        if 13 + index.len() + payload.len() + image.len() > MAX_PACK_BYTES {
            return Err(invalid("archive too large"));
        }
        payload.extend_from_slice(&image);
    }
    let mut output = Vec::with_capacity(13 + index.len() + payload.len());
    output.extend_from_slice(MAGIC);
    output.push(PACK_VERSION);
    output.extend_from_slice(&(documents.len() as u32).to_le_bytes());
    output.extend_from_slice(&(index.len() as u32).to_le_bytes());
    output.extend(index);
    output.extend(payload);
    Ok(output)
}

/// Package all .xaml files under a directory, ignoring filesystem symlinks.
pub fn compile_directory(root: &Path) -> XamlResult<Vec<u8>> {
    compile_directory_with(root, compile_documents)
}
/// Package a directory with cross-document compression for distribution.
pub fn compile_solid_directory(root: &Path) -> XamlResult<Vec<u8>> {
    compile_directory_with(root, compile_solid_documents)
}

fn compile_directory_with(root: &Path, encode: fn(&[(&str, &str)]) -> XamlResult<Vec<u8>>) -> XamlResult<Vec<u8>> {
    fn gather(dir: &Path, prefix: &str, docs: &mut Vec<(String, String)>) -> XamlResult<()> {
        let fs_error = |e: std::io::Error| invalid(&format!("{}: {e}", dir.display()));
        for entry in fs::read_dir(dir).map_err(fs_error)? {
            let entry = entry.map_err(fs_error)?;
            let kind = entry.file_type().map_err(fs_error)?;
            if !kind.is_dir()
                && (!kind.is_file()
                    || entry
                        .path()
                        .extension()
                        .is_none_or(|ext| !ext.eq_ignore_ascii_case("xaml")))
            {
                continue;
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| invalid("invalid UTF-8 filename"))?;
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            check_path(&path)?;
            if kind.is_dir() {
                gather(&entry.path(), &path, docs)?;
            } else {
                if docs.len() == MAX_DOCUMENTS {
                    return Err(invalid("too many documents"));
                }
                docs.push((path, fs::read_to_string(entry.path()).map_err(fs_error)?));
            }
        }
        Ok(())
    }
    let mut docs = Vec::new();
    gather(root, "", &mut docs)?;
    encode(
        &docs
            .iter()
            .map(|(p, s)| (p.as_str(), s.as_str()))
            .collect::<Vec<_>>(),
    )
}
