use crate::{DesignerAsset, registered_designer_assets};
use ferroui_designer_project::BuildMetadata;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

type CatalogResult = Result<Vec<DesignerAsset>, String>;

/// Each accepted build replaces the selected package's controls. Older jobs
/// cannot publish after a newer build of the same package begins loading.
#[derive(Default)]
pub(super) struct ProjectMarkup {
    accepted: BTreeMap<PathBuf, Vec<DesignerAsset>>,
    loading: BTreeMap<PathBuf, Receiver<CatalogResult>>,
}

impl ProjectMarkup {
    pub(super) fn assets(&self, root: &Path) -> &[DesignerAsset] {
        self.accepted.get(root).map_or(&[], Vec::as_slice)
    }

    pub(super) fn load(&mut self, metadata: BuildMetadata) -> Result<(), String> {
        let root = metadata.package_root().map_err(|error| error.to_string())?;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = metadata
                .catalog()
                .map(|catalog| registered_designer_assets(&catalog, metadata.package.as_deref()))
                .map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        self.loading.insert(root, receiver);
        Ok(())
    }

    pub(super) fn poll(&mut self) -> Vec<Result<(), String>> {
        let mut results = Vec::new();
        self.loading
            .retain(|root, receiver| match receiver.try_recv() {
                Ok(Ok(assets)) => {
                    self.accepted.insert(root.clone(), assets);
                    results.push(Ok(()));
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
