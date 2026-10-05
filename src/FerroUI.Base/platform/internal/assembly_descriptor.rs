use super::{EmbeddedAssetDescriptor, IAssetDescriptor, RegisteredAssembly};
use crate::platform::AssetAssembly;
use std::collections::HashMap;
use std::rc::Rc;

/// A dictionary of asset descriptors that enumerates in insertion order.
#[derive(Default)]
pub(crate) struct AssetMap {
    order: Vec<String>,
    map: HashMap<String, Rc<dyn IAssetDescriptor>>,
}

impl AssetMap {
    pub fn insert(&mut self, key: String, value: Rc<dyn IAssetDescriptor>) {
        if self.map.insert(key.clone(), value).is_none() {
            self.order.push(key);
        }
    }

    pub fn get(&self, key: &str) -> Option<Rc<dyn IAssetDescriptor>> {
        self.map.get(key).cloned()
    }

    /// The keys, in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.order.iter().map(String::as_str)
    }
}

/// Describes the assets of one assembly.
pub(crate) trait IAssemblyDescriptor {
    /// The described assembly.
    fn assembly(&self) -> &AssetAssembly;

    /// The embedded resources, by name.
    fn resources(&self) -> Option<&AssetMap>;

    /// The assets addressed through the asset scheme, by rooted path.
    fn ferro_resources(&self) -> Option<&AssetMap>;

    /// The name of the assembly.
    fn name(&self) -> Option<&str>;
}

pub(crate) struct AssemblyDescriptor {
    assembly: AssetAssembly,
    resources: Option<AssetMap>,
    ferro_resources: Option<AssetMap>,
    name: Option<String>,
}

impl AssemblyDescriptor {
    /// Describes what the assembly has registered so far.
    pub fn new(assembly: &AssetAssembly) -> Self {
        let registered = super::registered_assembly(assembly.name())
            .unwrap_or_else(|| RegisteredAssembly { name: assembly.name().to_owned(), ..Default::default() });

        let mut resources = AssetMap::default();
        for (name, bytes) in &registered.manifest_resources {
            resources.insert(name.clone(), Rc::new(EmbeddedAssetDescriptor::new(assembly.clone(), bytes)));
        }

        let ferro_resources = registered.assets.as_ref().map(|assets| {
            let mut map = AssetMap::default();
            for (path, bytes) in assets {
                map.insert(Self::get_path_rooted(path), Rc::new(EmbeddedAssetDescriptor::new(assembly.clone(), bytes)));
            }
            map
        });

        Self {
            assembly: assembly.clone(),
            resources: Some(resources),
            ferro_resources,
            name: Some(assembly.name().to_owned()),
        }
    }

    fn get_path_rooted(path: &str) -> String {
        if path.starts_with('/') {
            path.to_owned()
        } else {
            format!("/{path}")
        }
    }
}

impl IAssemblyDescriptor for AssemblyDescriptor {
    fn assembly(&self) -> &AssetAssembly {
        &self.assembly
    }

    fn resources(&self) -> Option<&AssetMap> {
        self.resources.as_ref()
    }

    fn ferro_resources(&self) -> Option<&AssetMap> {
        self.ferro_resources.as_ref()
    }

    fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}
