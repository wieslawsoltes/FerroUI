use crate::{HotReloadSession, ReloadError};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
/// Development manifest emitted by `ferroui_build::Build::hot_reload_manifest`.
pub struct ReloadManifest {
    pub files: BTreeMap<String, PathBuf>,
    pub sources: BTreeMap<String, String>,
    pub dependencies: BTreeMap<String, Vec<String>>,
    /// Whether the build compiled `{Binding}` as a compiled binding by default (#603): the
    /// default of the build, unless the crate opts out with `[package.metadata.ferroui]
    /// compile-bindings = false` (or `Build::default_compile_bindings(false)`). `true` for a
    /// manifest that does not state it. [`configuration`](Self::configuration) reloads the
    /// documents the same way (#604).
    pub compile_bindings: bool,
    contracts: Option<std::rc::Rc<crate::contract::BuildContracts>>,
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    fn manifest(compile_bindings: Option<serde_json::Value>) -> Result<ReloadManifest, ReloadError> {
        let mut value = serde_json::json!({"version": 1, "documents": []});
        if let Some(compile_bindings) = compile_bindings {
            value["compile_bindings"] = compile_bindings;
        }
        ReloadManifest::parse(&value.to_string())
    }
    /// Not from upstream (#604): the documents reload with compiled bindings, the default of
    /// the build, unless the manifest says the build opted out.
    #[test]
    fn documents_reload_with_the_binding_default_of_the_build() {
        static ASSEMBLY: ferroui_base::metadata::MarkupAssembly = ferroui_base::metadata::MarkupAssembly {
            name: "Manifest.Bindings",
            crate_name: "manifest_bindings",
            xmlns_definitions: &[],
            xmlns_prefixes: &[],
            metadata: &[],
        };
        for (stated, compiled) in [(None, true), (Some(serde_json::json!(true)), true), (Some(serde_json::json!(false)), false)] {
            let manifest = manifest(stated).unwrap();
            assert_eq!(manifest.compile_bindings, compiled);
            let configuration = manifest.configuration(Some(&ASSEMBLY));
            assert_eq!(configuration.use_compiled_bindings_by_default, compiled);
            assert!(configuration.local_assembly.is_some_and(|assembly| std::ptr::eq(assembly, &ASSEMBLY)));
            assert!(manifest.configuration(None).local_assembly.is_none());
        }
        assert!(manifest(Some(serde_json::json!("false"))).is_err());
    }
    /// Not from upstream (#604): a reloaded document that the default of the build compiles is
    /// refused at run time where it lacks `x:DataType`, as the build refuses it, and binds by
    /// reflection where the build opted out.
    #[test]
    fn a_reloaded_document_binds_as_the_build_compiled_it() {
        use ferroui_controls::testing::{TestServices, UnitTestApplication};
        use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
        use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
        let _application = UnitTestApplication::start(TestServices::styled_window());
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        FerroRuntimeXamlLoader::register();
        let load = |xaml: &str, configuration: RuntimeXamlLoaderConfiguration| {
            FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        };
        let untyped = "<TextBlock xmlns='https://github.com/ferroui' Text='{Binding Title}'/>";
        let typed = "<TextBlock xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:DataType='TextBlock' Text='{Binding Text}'/>";
        let compiled = manifest(None).unwrap().configuration(None);
        assert!(load(untyped, compiled.clone()).is_err(), "a compiled binding needs x:DataType");
        assert!(load(typed, compiled).is_ok());
        let opted_out = manifest(Some(serde_json::json!(false))).unwrap().configuration(None);
        assert!(load(untyped, opted_out).is_ok(), "a reflection binding needs no data type");
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::{PreparedReload, ReloadStatus, ReloadTarget, Sources};
    use std::{cell::Cell, rc::Rc};
    struct Target(Rc<Cell<usize>>);
    impl ReloadTarget for Target {
        fn documents(&self) -> Vec<String> {
            vec!["ferres://manifest/View.xaml".into()]
        }
        fn prepare(&self, _: Rc<Sources>) -> Result<Box<dyn PreparedReload>, ReloadError> {
            self.0.set(self.0.get() + 1);
            struct Change;
            impl PreparedReload for Change {
                fn commit(&mut self) -> Result<(), ReloadError> {
                    Ok(())
                }
                fn rollback(&mut self) {}
            }
            Ok(Box::new(Change))
        }
    }
    #[test]
    fn frozen_build_events_and_unknown_types_are_checked_before_preparation() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let source =
            r#"<Custom xmlns="urn:unregistered-build-type" Recomputed="Before" Width="10"/>"#;
        let manifest = |known| {
            ReloadManifest::parse(&serde_json::json!({
            "version": 1,
            "type_contracts": [{"namespace": "urn:unregistered-build-type", "name": "Custom",
                "known": known, "events": {"Recomputed": ["Action<Boolean>"]}}],
            "documents": [{
                "uri": "ferres://manifest/View.xaml", "source_path": "View.xaml", "source": source
            }]
        }).to_string()).unwrap()
        };
        for known in [true, false] {
            let manifest = manifest(known);
            let mut session = manifest.session().unwrap();
            let prepares = Rc::new(Cell::new(0));
            let target: Rc<dyn ReloadTarget> = Rc::new(Target(prepares.clone()));
            session.register_target(&target);
            for edited in [
                source.replace("Before", "After"),
                source.replace(" Recomputed=\"Before\"", ""),
                source.replace("Recomputed", "AnotherEvent"),
            ] {
                session.submit("ferres://manifest/View.xaml".into(), edited);
                assert_eq!(
                    session.apply_pending().unwrap().status,
                    ReloadStatus::RestartRequired
                );
                assert_eq!(prepares.get(), 0);
            }
            session.submit(
                "ferres://manifest/View.xaml".into(),
                source.replace("10", "20"),
            );
            assert_eq!(
                session.apply_pending().unwrap().status,
                if known {
                    ReloadStatus::Applied
                } else {
                    ReloadStatus::RestartRequired
                }
            );
            assert_eq!(prepares.get(), usize::from(known));
        }
    }
}
impl ReloadManifest {
    pub fn read(path: &Path) -> Result<Self, ReloadError> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| ReloadError::Failed(error.to_string()))?;
        Self::parse(&text)
    }
    pub fn parse(text: &str) -> Result<Self, ReloadError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| ReloadError::Failed(error.to_string()))?;
        if value["version"].as_u64() != Some(1) {
            return Err(ReloadError::Failed(
                "Unsupported reload manifest version".into(),
            ));
        }
        let documents = value["documents"]
            .as_array()
            .ok_or_else(|| ReloadError::Failed("Missing documents".into()))?;
        let compile_bindings = match value.get("compile_bindings") {
            None => true,
            Some(value) => value
                .as_bool()
                .ok_or_else(|| ReloadError::Failed("compile_bindings must be a boolean".into()))?,
        };
        let mut manifest = Self {
            files: BTreeMap::new(),
            sources: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            compile_bindings,
            contracts: None,
        };
        if let Some(contracts) = value.get("type_contracts") {
            let contracts = contracts
                .as_array()
                .ok_or_else(|| ReloadError::Failed("Invalid type contracts".into()))?;
            let catalog = manifest.contracts.get_or_insert_with(Default::default);
            let catalog =
                std::rc::Rc::get_mut(catalog).expect("unshared manifest under construction");
            for contract in contracts {
                catalog.insert(contract)?;
            }
        }
        for document in documents {
            let string = |key: &str| {
                document[key]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| ReloadError::Failed(format!("Missing document {key}")))
            };
            let uri = crate::document_key(&string("uri")?);
            let base = crate::document_url(&uri)
                .ok_or_else(|| ReloadError::Failed(format!("Invalid document URI: {uri}")))?;
            if manifest
                .files
                .insert(uri.clone(), PathBuf::from(string("source_path")?))
                .is_some()
            {
                return Err(ReloadError::Failed(format!(
                    "Duplicate document URI: {uri}"
                )));
            }
            manifest.sources.insert(uri.clone(), string("source")?);
            let mut includes = Vec::new();
            if let Some(values) = document["includes"].as_array() {
                for value in values {
                    let text = value
                        .as_str()
                        .ok_or_else(|| ReloadError::Failed("Include must be a URI".into()))?;
                    let resolved = crate::include_key(&base, text)
                        .ok_or_else(|| ReloadError::Failed(format!("Invalid include: {text}")))?;
                    includes.push(resolved);
                }
            }
            manifest.dependencies.insert(uri, includes);
        }
        Ok(manifest)
    }
    /// The configuration of the run-time loader for the documents of the manifest: `{Binding}`
    /// compiles as the build compiled it ([`compile_bindings`](Self::compile_bindings)), so a
    /// reloaded document binds the way the built application does (#604). `assembly` is the
    /// assembly of the documents, the local assembly of the loader.
    pub fn configuration(
        &self,
        assembly: Option<&'static ferroui_base::metadata::MarkupAssembly>,
    ) -> ferroui_markup_xaml::RuntimeXamlLoaderConfiguration {
        let mut configuration = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::new();
        configuration.local_assembly = assembly;
        configuration.use_compiled_bindings_by_default = self.compile_bindings;
        configuration
    }
    pub fn session(&self) -> Result<HotReloadSession, ReloadError> {
        let mut session = HotReloadSession::new();
        session.build_contracts = self.contracts.clone();
        for (uri, source) in &self.sources {
            session.register_document(uri.clone(), source.clone())?;
        }
        Ok(session)
    }
    /// Watch manifest document directories recursively from the build's source
    /// baseline. Newly discovered documents are reported for conservative admission.
    pub fn watcher(&self, interval: std::time::Duration) -> crate::FileWatcher {
        let mut directories = std::collections::BTreeSet::new();
        for (uri, path) in &self.files {
            if let (Some(parent), Some((prefix, _))) = (path.parent(), uri.rsplit_once('/')) {
                directories.insert((parent.to_path_buf(), format!("{prefix}/")));
            }
        }
        crate::FileWatcher::with_directories(
            self.files.clone(),
            directories
                .into_iter()
                .map(|(path, uri_prefix)| crate::WatchDirectory { path, uri_prefix })
                .collect(),
            interval,
            self.sources.clone(),
        )
    }
    pub fn transitive_dependencies(&self, uri: &str) -> Vec<String> {
        let root = crate::document_key(uri);
        let mut seen = std::collections::BTreeSet::from([root.clone()]);
        let mut pending = vec![root.clone()];
        while let Some(uri) = pending.pop() {
            for included in self.dependencies.get(&uri).into_iter().flatten() {
                if seen.insert(included.clone()) {
                    pending.push(included.clone());
                }
            }
        }
        seen.remove(&root);
        seen.into_iter().collect()
    }
}
