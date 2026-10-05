//! The process-wide registry of assets embedded in the application binary.
//!
//! This takes the place of the resources a compiled assembly carries: every
//! crate that ships assets registers them once at startup, under its crate
//! name, as `(path, bytes)` pairs. The registry is shared by all threads so
//! that registration can happen before the UI thread exists.

use std::sync::{Mutex, MutexGuard, PoisonError};

type Assets = Vec<(String, &'static [u8])>;

/// The assets registered by one crate.
#[derive(Clone, Default)]
pub(crate) struct RegisteredAssembly {
    pub name: String,
    /// Assets addressed by rooted path through the asset scheme, in
    /// registration order. `None` when the crate registered none.
    pub assets: Option<Assets>,
    /// Embedded resources addressed by name through the `resm:` scheme, in
    /// registration order.
    pub manifest_resources: Assets,
}

static REGISTRY: Mutex<Vec<RegisteredAssembly>> = Mutex::new(Vec::new());

fn registry() -> MutexGuard<'static, Vec<RegisteredAssembly>> {
    REGISTRY.lock().unwrap_or_else(PoisonError::into_inner)
}

fn with_assembly(crate_name: &str, update: impl FnOnce(&mut RegisteredAssembly)) {
    let mut registry = registry();
    let position = match registry.iter().position(|assembly| assembly.name == crate_name) {
        Some(position) => position,
        None => {
            registry.push(RegisteredAssembly { name: crate_name.to_owned(), ..Default::default() });
            registry.len() - 1
        }
    };
    update(&mut registry[position]);
}

fn merge(target: &mut Assets, key: String, bytes: &'static [u8]) {
    match target.iter_mut().find(|(existing, _)| *existing == key) {
        Some(entry) => entry.1 = bytes,
        None => target.push((key, bytes)),
    }
}

/// Registers the assets embedded by a crate, addressable as
/// `<asset scheme>://<crate_name>/<path>`.
///
/// Paths are relative to the crate's asset root; a leading `/` is optional.
/// Registering a path again replaces its content. Asset loaders that already
/// cached the crate see the change after
/// [`IAssetLoader::invalidate_assembly_cache`](crate::platform::IAssetLoader::invalidate_assembly_cache).
pub fn register_assets(crate_name: &str, assets: &[(&str, &'static [u8])]) {
    with_assembly(crate_name, |assembly| {
        let target = assembly.assets.get_or_insert_with(Vec::new);
        for (path, bytes) in assets {
            let rooted = if path.starts_with('/') { (*path).to_owned() } else { format!("/{path}") };
            merge(target, rooted, bytes);
        }
    });
}

/// Registers embedded resources of a crate, addressable by name as
/// `resm:<name>?assembly=<crate_name>`.
pub fn register_manifest_resources(crate_name: &str, resources: &[(&str, &'static [u8])]) {
    with_assembly(crate_name, |assembly| {
        for (name, bytes) in resources {
            merge(&mut assembly.manifest_resources, (*name).to_owned(), bytes);
        }
    });
}

/// The first bytes of an asset bundle: the name of the format and its
/// version.
pub const ASSET_BUNDLE_MAGIC: &[u8; 8] = b"FUIASSB1";

/// Registers the assets of an asset bundle: assets of one or more crates in
/// one block of bytes, as [`register_assets`] registers assets embedded in
/// the binary. The registered assets are views into `bundle`, which is
/// therefore kept for the life of the process.
///
/// This is how an application whose binary has to stay small (a WebAssembly
/// module that is downloaded) ships its large assets in a separate file:
/// the host reads the file before the application starts and passes its
/// content here. The format is [`ASSET_BUNDLE_MAGIC`] followed by one record
/// per asset, all integers little-endian:
///
/// * the length of the crate name (`u16`) and the name, in UTF-8;
/// * the length of the rooted path (`u16`) and the path, in UTF-8;
/// * the length of the content (`u32`) and the content.
///
/// Returns the number of assets registered, or a description of the first
/// defect of the bundle; nothing is registered from a defective bundle.
pub fn register_asset_bundle(bundle: &'static [u8]) -> Result<usize, String> {
    let assets = read_asset_bundle(bundle)?;
    for (crate_name, path, content) in &assets {
        register_assets(crate_name, &[(*path, *content)]);
    }
    Ok(assets.len())
}

/// The records of an asset bundle: `(crate name, path, content)`.
fn read_asset_bundle(bundle: &[u8]) -> Result<Vec<(&str, &str, &[u8])>, String> {
    fn take<'a>(bundle: &'a [u8], at: &mut usize, length: usize, what: &str) -> Result<&'a [u8], String> {
        let end = at.checked_add(length).filter(|end| *end <= bundle.len());
        let end = end.ok_or_else(|| format!("the asset bundle ends inside {what} at byte {at}"))?;
        let bytes = &bundle[*at..end];
        *at = end;
        Ok(bytes)
    }
    fn text<'a>(bundle: &'a [u8], at: &mut usize, what: &str) -> Result<&'a str, String> {
        let length = u16::from_le_bytes(take(bundle, at, 2, what)?.try_into().expect("two bytes"));
        let bytes = take(bundle, at, usize::from(length), what)?;
        std::str::from_utf8(bytes).map_err(|_| format!("{what} before byte {at} of the asset bundle is not UTF-8"))
    }

    if !bundle.starts_with(ASSET_BUNDLE_MAGIC) {
        return Err("not an asset bundle: the first bytes are not the name of the format".to_owned());
    }
    let mut at = ASSET_BUNDLE_MAGIC.len();
    let mut assets = Vec::new();
    while at < bundle.len() {
        let crate_name = text(bundle, &mut at, "a crate name")?;
        let path = text(bundle, &mut at, "an asset path")?;
        let length = u32::from_le_bytes(take(bundle, &mut at, 4, "a content length")?.try_into().expect("four bytes"));
        let content = take(bundle, &mut at, length as usize, "the content of an asset")?;
        assets.push((crate_name, path, content));
    }
    Ok(assets)
}

/// The names of the crates that registered assets, in registration order.
pub(crate) fn registered_assembly_names() -> Vec<String> {
    registry().iter().map(|assembly| assembly.name.clone()).collect()
}

/// A snapshot of what the crate with exactly the given name registered.
pub(crate) fn registered_assembly(name: &str) -> Option<RegisteredAssembly> {
    registry().iter().find(|assembly| assembly.name == name).cloned()
}

// Not from upstream: the asset bundle is an addition of the port.
#[cfg(test)]
mod tests {
    use super::*;

    fn record(bundle: &mut Vec<u8>, crate_name: &str, path: &str, content: &[u8]) {
        bundle.extend_from_slice(&(crate_name.len() as u16).to_le_bytes());
        bundle.extend_from_slice(crate_name.as_bytes());
        bundle.extend_from_slice(&(path.len() as u16).to_le_bytes());
        bundle.extend_from_slice(path.as_bytes());
        bundle.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bundle.extend_from_slice(content);
    }

    #[test]
    fn a_bundle_registers_its_assets_under_their_crates() {
        let mut bundle = ASSET_BUNDLE_MAGIC.to_vec();
        record(&mut bundle, "BundleTestA", "/Assets/a.bin", b"first");
        record(&mut bundle, "BundleTestB", "/b.txt", b"");
        record(&mut bundle, "BundleTestA", "/Assets/c.bin", &[0, 1, 2]);
        let bundle: &'static [u8] = Box::leak(bundle.into_boxed_slice());

        assert_eq!(Ok(3), register_asset_bundle(bundle));

        let a = registered_assembly("BundleTestA").unwrap().assets.unwrap();
        assert_eq!(
            vec![("/Assets/a.bin".to_owned(), &b"first"[..]), ("/Assets/c.bin".to_owned(), &[0u8, 1, 2][..])],
            a
        );
        let b = registered_assembly("BundleTestB").unwrap().assets.unwrap();
        assert_eq!(vec![("/b.txt".to_owned(), &b""[..])], b);
    }

    #[test]
    fn an_empty_bundle_registers_nothing() {
        assert_eq!(Ok(0), register_asset_bundle(ASSET_BUNDLE_MAGIC));
    }

    #[test]
    fn a_defective_bundle_registers_nothing() {
        assert!(register_asset_bundle(b"PNG.....").is_err());

        let mut truncated = ASSET_BUNDLE_MAGIC.to_vec();
        record(&mut truncated, "BundleTestC", "/one.bin", b"one");
        record(&mut truncated, "BundleTestC", "/two.bin", b"two");
        truncated.pop();
        let truncated: &'static [u8] = Box::leak(truncated.into_boxed_slice());
        assert!(register_asset_bundle(truncated).unwrap_err().contains("ends inside the content of an asset"));
        assert!(registered_assembly("BundleTestC").is_none());
    }
}
