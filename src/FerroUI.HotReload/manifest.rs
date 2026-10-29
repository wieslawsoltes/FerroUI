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
    contracts: Option<std::rc::Rc<crate::contract::BuildContracts>>,
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
        let mut manifest = Self {
            files: BTreeMap::new(),
            sources: BTreeMap::new(),
            dependencies: BTreeMap::new(),
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
            let base = ferroui_base::utilities::Uri::try_create(
                &uri,
                ferroui_base::utilities::UriKind::Absolute,
            )
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
                    let relative = ferroui_base::utilities::Uri::try_create(
                        text,
                        ferroui_base::utilities::UriKind::RelativeOrAbsolute,
                    )
                    .ok_or_else(|| ReloadError::Failed(format!("Invalid include: {text}")))?;
                    let resolved = if relative.is_absolute_uri() {
                        relative
                    } else {
                        ferroui_base::utilities::Uri::combine(&base, &relative)
                    };
                    includes.push(resolved.absolute_uri().to_string());
                }
            }
            manifest.dependencies.insert(uri, includes);
        }
        Ok(manifest)
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
