use super::internal::{
    AssemblyDescriptor, AssemblyDescriptorResolver, IAssemblyDescriptor, IAssemblyDescriptorResolver, IAssetDescriptor,
};
use super::{AssetAssembly, AssetStream, IAssetLoader, ASSET_SCHEME};
use crate::utilities::{Uri, UriExtensions};
use std::cell::RefCell;
use std::rc::Rc;

/// Loads assets compiled into the application binary.
///
/// The type is considered unstable; use the functions of
/// [`AssetLoader`](super::AssetLoader) instead.
pub struct StandardAssetLoader {
    assembly_descriptor_resolver: Box<dyn IAssemblyDescriptorResolver>,
    default_resm_assembly: RefCell<Option<Rc<dyn IAssemblyDescriptor>>>,
}

impl Default for StandardAssetLoader {
    fn default() -> Self {
        Self::new(None)
    }
}

impl StandardAssetLoader {
    pub(crate) fn with_resolver(
        resolver: Box<dyn IAssemblyDescriptorResolver>,
        assembly: Option<&AssetAssembly>,
    ) -> Self {
        Self {
            assembly_descriptor_resolver: resolver,
            default_resm_assembly: RefCell::new(assembly.map(Self::describe)),
        }
    }

    /// Creates an asset loader. `assembly` is the default assembly from
    /// which to load embedded-resource (`resm:`) assets for which no
    /// assembly is specified.
    pub fn new(assembly: Option<&AssetAssembly>) -> Self {
        Self::with_resolver(Box::new(AssemblyDescriptorResolver::new()), assembly)
    }

    fn describe(assembly: &AssetAssembly) -> Rc<dyn IAssemblyDescriptor> {
        Rc::new(AssemblyDescriptor::new(assembly))
    }

    fn not_found(uri: &Uri) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, format!("The resource {uri} could not be found."))
    }

    fn try_get_asset(&self, uri: &Uri, base_uri: Option<&Uri>) -> Option<Rc<dyn IAssetDescriptor>> {
        if UriExtensions::is_absolute_resm(uri) {
            let assembly = self
                .try_get_assembly(Some(uri))
                .or_else(|| self.try_get_assembly(base_uri))
                .or_else(|| self.default_resm_assembly.borrow().clone());

            if let Some(resources) = assembly.as_ref().and_then(|assembly| assembly.resources()) {
                let resource_key = uri.absolute_path();

                if let Some(asset_descriptor) = resources.get(resource_key) {
                    return Some(asset_descriptor);
                }
            }
        }

        let uri = UriExtensions::ensure_absolute(uri, base_uri).ok()?;

        if UriExtensions::is_asset(&uri) {
            if let Some((assembly, path)) = self.try_get_res_asm_and_path(&uri) {
                let resources = assembly.ferro_resources()?;

                if let Some(asset_descriptor) = resources.get(&path) {
                    return Some(asset_descriptor);
                }
            }
        }

        None
    }

    fn try_get_res_asm_and_path(&self, uri: &Uri) -> Option<(Rc<dyn IAssemblyDescriptor>, String)> {
        let path = UriExtensions::get_unescape_absolute_path(uri);

        self.try_load_assembly(UriExtensions::authority(uri)).map(|assembly| (assembly, path))
    }

    fn try_get_assembly(&self, uri: Option<&Uri>) -> Option<Rc<dyn IAssemblyDescriptor>> {
        let uri = uri?;

        if !uri.is_absolute_uri() {
            return None;
        }

        if UriExtensions::is_asset(uri) {
            if let Some((assembly, _)) = self.try_get_res_asm_and_path(uri) {
                return Some(assembly);
            }
        }

        if UriExtensions::is_resm(uri) {
            let assembly_name = UriExtensions::get_assembly_name_from_query(uri);

            if !assembly_name.is_empty() {
                if let Some(assembly) = self.try_load_assembly(&assembly_name) {
                    return Some(assembly);
                }
            }
        }

        None
    }

    fn try_load_assembly(&self, assembly_name: &str) -> Option<Rc<dyn IAssemblyDescriptor>> {
        self.assembly_descriptor_resolver.get_assembly(assembly_name).ok()
    }
}

impl IAssetLoader for StandardAssetLoader {
    fn set_default_assembly(&self, assembly: &AssetAssembly) {
        *self.default_resm_assembly.borrow_mut() = Some(Self::describe(assembly));
    }

    fn exists(&self, uri: &Uri, base_uri: Option<&Uri>) -> bool {
        self.try_get_asset(uri, base_uri).is_some()
    }

    fn open(&self, uri: &Uri, base_uri: Option<&Uri>) -> std::io::Result<Box<dyn AssetStream>> {
        Ok(self.open_and_get_assembly(uri, base_uri)?.0)
    }

    fn open_and_get_assembly(
        &self,
        uri: &Uri,
        base_uri: Option<&Uri>,
    ) -> std::io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        match self.try_get_asset(uri, base_uri) {
            Some(asset_descriptor) => Ok((asset_descriptor.get_stream(), asset_descriptor.assembly().clone())),
            None => Err(Self::not_found(uri)),
        }
    }

    fn get_assembly(&self, uri: &Uri, base_uri: Option<&Uri>) -> Option<AssetAssembly> {
        let uri = match base_uri {
            Some(base_uri) if !uri.is_absolute_uri() => {
                if !base_uri.is_absolute_uri() {
                    return None;
                }
                Uri::combine(base_uri, uri)
            }
            _ => uri.clone(),
        };
        self.try_get_assembly(Some(&uri)).map(|descriptor| descriptor.assembly().clone())
    }

    fn get_assets(&self, uri: &Uri, base_uri: Option<&Uri>) -> Vec<Uri> {
        if UriExtensions::is_absolute_resm(uri) {
            let assembly = self.try_get_assembly(Some(uri)).or_else(|| self.default_resm_assembly.borrow().clone());

            let Some(assembly) = assembly else { return Vec::new() };
            let Some(resources) = assembly.resources() else { return Vec::new() };
            let path = UriExtensions::get_unescape_absolute_path(uri);
            let name = assembly.name().unwrap_or_default();
            return resources
                .keys()
                .filter(|key| key.contains(&path))
                .filter_map(|key| Uri::absolute(&format!("resm:{key}?assembly={name}")).ok())
                .collect();
        }

        let Ok(uri) = UriExtensions::ensure_absolute(uri, base_uri) else { return Vec::new() };
        if UriExtensions::is_asset(&uri) {
            let Some((assembly, mut path)) = self.try_get_res_asm_and_path(&uri) else { return Vec::new() };
            let Some(resources) = assembly.ferro_resources() else { return Vec::new() };

            if !path.is_empty() && !path.ends_with('/') {
                path.push('/');
            }

            let name = assembly.name().unwrap_or_default();
            return resources
                .keys()
                .filter(|key| key.starts_with(&path))
                .filter_map(|key| Uri::absolute(&format!("{ASSET_SCHEME}://{name}{key}")).ok())
                .collect();
        }

        Vec::new()
    }

    fn invalidate_assembly_cache(&self, name: &str) {
        self.assembly_descriptor_resolver.invalidate_assembly_cache(name);
    }

    fn invalidate_assembly_cache_all(&self) {
        self.assembly_descriptor_resolver.invalidate_assembly_cache_all();
    }
}

#[cfg(test)]
mod tests {
    use super::super::internal::AssetMap;
    use super::*;
    use crate::platform::{register_assets, register_manifest_resources};
    use crate::utilities::UriKind;
    use std::collections::HashMap;
    use std::io::Read;

    const ASSEMBLY_NAME_WITH_WHITESPACE: &str = "Awesome Library";
    const ASSEMBLY_NAME_WITH_NON_ASCII: &str = "Какое-то-название";

    struct MockAssemblyDescriptor(AssetAssembly);

    impl IAssemblyDescriptor for MockAssemblyDescriptor {
        fn assembly(&self) -> &AssetAssembly {
            &self.0
        }
        fn resources(&self) -> Option<&AssetMap> {
            None
        }
        fn ferro_resources(&self) -> Option<&AssetMap> {
            None
        }
        fn name(&self) -> Option<&str> {
            None
        }
    }

    struct MockResolver(HashMap<String, Rc<dyn IAssemblyDescriptor>>);

    impl IAssemblyDescriptorResolver for MockResolver {
        fn get_assembly(&self, name: &str) -> Result<Rc<dyn IAssemblyDescriptor>, String> {
            self.0.get(name).cloned().ok_or_else(|| format!("unknown assembly {name}"))
        }
        fn invalidate_assembly_cache(&self, _name: &str) {}
        fn invalidate_assembly_cache_all(&self) {}
    }

    fn resolver() -> Box<dyn IAssemblyDescriptorResolver> {
        let mut assemblies: HashMap<String, Rc<dyn IAssemblyDescriptor>> = HashMap::new();
        for name in [ASSEMBLY_NAME_WITH_WHITESPACE, ASSEMBLY_NAME_WITH_NON_ASCII] {
            assemblies.insert(name.to_owned(), Rc::new(MockAssemblyDescriptor(AssetAssembly::new(name))));
        }
        Box::new(MockResolver(assemblies))
    }

    #[test]
    fn assembly_name_with_whitespace_should_load_resm() {
        let uri =
            Uri::absolute(&format!("resm:Ferro.Base.UnitTests.Assets.something?assembly={ASSEMBLY_NAME_WITH_WHITESPACE}"))
                .unwrap();
        let loader = StandardAssetLoader::with_resolver(resolver(), None);

        let assembly_actual = loader.get_assembly(&uri, None);

        assert_eq!(Some(ASSEMBLY_NAME_WITH_WHITESPACE), assembly_actual.as_ref().map(AssetAssembly::name));
    }

    #[test]
    fn invalid_assembly_name_should_yield_empty_enumerable() {
        let uri = Uri::absolute(&format!("{ASSET_SCHEME}://InvalidAssembly")).unwrap();
        let loader = StandardAssetLoader::with_resolver(resolver(), None);

        let assembly_actual = loader.get_assets(&uri, None);

        assert!(assembly_actual.is_empty());
    }

    // --- not from upstream ---

    fn read(mut stream: Box<dyn AssetStream>) -> Vec<u8> {
        let mut content = Vec::new();
        stream.read_to_end(&mut content).unwrap();
        content
    }

    fn asset(path: &str) -> Uri {
        Uri::absolute(&format!("{ASSET_SCHEME}://{path}")).unwrap()
    }

    #[test]
    fn registered_assets_are_found_by_uri() {
        register_assets(
            "asset-loader-test-app",
            &[("Assets/logo.png", b"logo"), ("/Assets/Icons/a b.svg", b"icon"), ("Styles/Main.xaml", b"xaml")],
        );
        let loader = StandardAssetLoader::new(None);

        let logo = asset("asset-loader-test-app/Assets/logo.png");
        assert!(loader.exists(&logo, None));
        assert_eq!(b"logo".to_vec(), read(loader.open(&logo, None).unwrap()));

        // Paths are unescaped, the crate name is matched ignoring case.
        let icon = asset("Asset-Loader-Test-App/Assets/Icons/a%20b.svg");
        let (stream, assembly) = loader.open_and_get_assembly(&icon, None).unwrap();
        assert_eq!(b"icon".to_vec(), read(stream));
        assert_eq!("asset-loader-test-app", assembly.name());
        assert_eq!(Some(assembly), loader.get_assembly(&icon, None));

        // Relative URIs resolve against the base URI.
        let base = asset("asset-loader-test-app/Styles/Main.xaml");
        let relative = Uri::new("../Assets/logo.png", UriKind::Relative).unwrap();
        assert!(loader.exists(&relative, Some(&base)));
        assert!(!loader.exists(&relative, None));
        let rooted = Uri::new("/Assets/logo.png", UriKind::Relative).unwrap();
        assert_eq!(b"logo".to_vec(), read(loader.open(&rooted, Some(&base)).unwrap()));
        assert_eq!("asset-loader-test-app", loader.get_assembly(&rooted, Some(&base)).unwrap().name());

        let missing = asset("asset-loader-test-app/Assets/missing.png");
        assert!(!loader.exists(&missing, None));
        assert_eq!(std::io::ErrorKind::NotFound, loader.open(&missing, None).err().unwrap().kind());
        assert!(!loader.exists(&asset("asset-loader-unknown-crate/Assets/logo.png"), None));
        assert!(loader.open(&relative, None).is_err());
    }

    #[test]
    fn get_assets_lists_a_folder() {
        register_assets(
            "asset-loader-folder-app",
            &[("Assets/a.png", b"a"), ("Assets/Sub/b.png", b"b"), ("AssetsOther/c.png", b"c"), ("d.png", b"d")],
        );
        let loader = StandardAssetLoader::new(None);

        let list = |uri: &str| -> Vec<String> {
            loader.get_assets(&asset(uri), None).iter().map(|uri| uri.absolute_uri().to_owned()).collect()
        };

        assert_eq!(
            vec![
                format!("{ASSET_SCHEME}://asset-loader-folder-app/Assets/a.png"),
                format!("{ASSET_SCHEME}://asset-loader-folder-app/Assets/Sub/b.png"),
            ],
            list("asset-loader-folder-app/Assets")
        );
        assert_eq!(4, list("asset-loader-folder-app").len());
        assert_eq!(4, list("asset-loader-folder-app/").len());
        assert!(list("asset-loader-folder-app/Missing").is_empty());

        let relative = Uri::new("Assets/Sub", UriKind::Relative).unwrap();
        let base = asset("asset-loader-folder-app/d.png");
        assert_eq!(1, loader.get_assets(&relative, Some(&base)).len());
        assert!(loader.get_assets(&relative, None).is_empty());
    }

    #[test]
    fn manifest_resources_are_found_by_name() {
        register_manifest_resources(
            "asset-loader-resm-lib",
            &[("Lib.Assets.one.txt", b"one"), ("Lib.Assets.two.txt", b"two"), ("Lib.Other.txt", b"other")],
        );
        let lib = AssetAssembly::new("asset-loader-resm-lib");
        let loader = StandardAssetLoader::new(None);

        let qualified = Uri::absolute("resm:Lib.Assets.one.txt?assembly=asset-loader-resm-lib").unwrap();
        assert_eq!(b"one".to_vec(), read(loader.open(&qualified, None).unwrap()));
        assert_eq!(Some(lib.clone()), loader.get_assembly(&qualified, None));

        // Without an assembly in the query, the assembly of the base URI and
        // then the default assembly are used.
        let unqualified = Uri::absolute("resm:Lib.Assets.two.txt").unwrap();
        assert!(!loader.exists(&unqualified, None));
        assert!(loader.exists(&unqualified, Some(&qualified)));
        loader.set_default_assembly(&lib);
        assert_eq!(b"two".to_vec(), read(loader.open(&unqualified, None).unwrap()));

        let found: Vec<String> = loader
            .get_assets(&Uri::absolute("resm:Lib.Assets").unwrap(), None)
            .iter()
            .map(|uri| uri.absolute_uri().to_owned())
            .collect();
        assert_eq!(
            vec![
                "resm:Lib.Assets.one.txt?assembly=asset-loader-resm-lib".to_owned(),
                "resm:Lib.Assets.two.txt?assembly=asset-loader-resm-lib".to_owned(),
            ],
            found
        );

        // A crate that registered only embedded resources has no assets of
        // the asset scheme.
        assert!(loader.get_assets(&asset("asset-loader-resm-lib/"), None).is_empty());
    }

    #[test]
    fn assembly_cache_is_invalidated_explicitly() {
        register_assets("asset-loader-cache-app", &[("a.txt", b"1")]);
        let loader = StandardAssetLoader::new(None);
        let a = asset("asset-loader-cache-app/a.txt");
        let b = asset("asset-loader-cache-app/b.txt");
        assert!(loader.exists(&a, None));
        assert!(!loader.exists(&b, None));

        register_assets("asset-loader-cache-app", &[("b.txt", b"2"), ("a.txt", b"3")]);
        assert!(!loader.exists(&b, None));
        loader.invalidate_assembly_cache("asset-loader-cache-app");
        assert!(loader.exists(&b, None));
        assert_eq!(b"3".to_vec(), read(loader.open(&a, None).unwrap()));

        register_assets("asset-loader-cache-app", &[("c.txt", b"4")]);
        loader.invalidate_assembly_cache_all();
        assert!(loader.exists(&asset("asset-loader-cache-app/c.txt"), None));
    }

    #[test]
    fn shortest_matching_crate_name_wins() {
        register_assets("asset-loader-prefix-long", &[("x", b"long")]);
        register_assets("asset-loader-prefix", &[("x", b"short")]);
        let loader = StandardAssetLoader::new(None);
        assert_eq!(b"short".to_vec(), read(loader.open(&asset("asset-loader-prefix/x"), None).unwrap()));
        assert_eq!(b"long".to_vec(), read(loader.open(&asset("asset-loader-prefix-long/x"), None).unwrap()));
    }
}
