use crate::{DesignerAsset, registered_designer_assets};
use ferroui_designer_language::Schema;
use ferroui_designer_project::{BuildMetadata, CargoBuildConfiguration, MarkupCatalog, file_path};
use ferroui_designer_rsx::RsxCatalog;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};

struct Package {
    assets: Vec<DesignerAsset>,
    rust: Arc<RsxCatalog>,
    catalog: Arc<MarkupCatalog>,
    #[cfg(not(target_arch = "wasm32"))]
    cargo: CargoBuildConfiguration,
}

/// Each accepted build replaces the selected package's controls. Older jobs
/// cannot publish after a newer build of the same package begins loading.
#[derive(Default)]
pub(super) struct ProjectMarkup {
    accepted: BTreeMap<PathBuf, Package>,
    loading: BTreeMap<PathBuf, Receiver<Result<Package, String>>>,
}

impl ProjectMarkup {
    pub(super) fn assets(&self, root: &Path) -> &[DesignerAsset] {
        self.accepted
            .get(root)
            .map_or(&[], |package| package.assets.as_slice())
    }

    pub(super) fn rust(&self, uri: &str) -> Option<Arc<RsxCatalog>> {
        let path = file_path(uri).ok()?;
        // The closest Cargo package owns the file, even when only an outer
        // package has been built in this solution.
        let root = path
            .ancestors()
            .skip(1)
            .find(|root| root.join("Cargo.toml").is_file())?;
        self.accepted.get(root).map(|package| package.rust.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn cargo(&self, root: &Path) -> CargoBuildConfiguration {
        self.accepted
            .get(root)
            .map(|package| package.cargo.clone())
            .unwrap_or_default()
    }

    pub(super) fn load(
        &mut self,
        metadata: BuildMetadata,
        cargo: CargoBuildConfiguration,
    ) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        drop(cargo);
        let root = metadata.package_root().map_err(|error| error.to_string())?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = metadata
                .catalog()
                .map(|catalog| {
                    let mut schema = Schema::ferroui();
                    schema.register_project(&catalog);
                    let package = metadata.package.as_deref();
                    Package {
                        assets: registered_designer_assets(&catalog, package),
                        rust: Arc::new(RsxCatalog::from_project(&catalog, &schema, package)),
                        catalog: Arc::new(catalog),
                        #[cfg(not(target_arch = "wasm32"))]
                        cargo,
                    }
                })
                .map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        self.loading.insert(root, receiver);
        Ok(())
    }

    pub(super) fn poll(&mut self) -> Vec<Result<(PathBuf, Arc<MarkupCatalog>), String>> {
        let mut results = Vec::new();
        self.loading
            .retain(|root, receiver| match receiver.try_recv() {
                Ok(Ok(package)) => {
                    results.push(Ok((root.clone(), package.catalog.clone())));
                    self.accepted.insert(root.clone(), package);
                    false
                }
                Ok(Err(error)) => {
                    results.push(Err(error));
                    false
                }
                Err(TryRecvError::Disconnected) => {
                    results.push(Err("The registered controls could not be loaded".into()));
                    false
                }
                Err(TryRecvError::Empty) => true,
            });
        results
    }
}
