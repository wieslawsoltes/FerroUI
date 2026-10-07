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

/// The names of the crates that registered assets, in registration order.
pub(crate) fn registered_assembly_names() -> Vec<String> {
    registry().iter().map(|assembly| assembly.name.clone()).collect()
}

/// A snapshot of what the crate with exactly the given name registered.
pub(crate) fn registered_assembly(name: &str) -> Option<RegisteredAssembly> {
    registry().iter().find(|assembly| assembly.name == name).cloned()
}
