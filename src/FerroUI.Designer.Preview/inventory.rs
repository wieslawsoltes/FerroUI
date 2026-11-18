use ferroui_designer_project::{workspace_documents, DocumentEntry, DocumentKind};
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_ENTRIES: usize = 4096;

#[derive(Serialize)]
pub(crate) struct Inventory {
    files: Vec<WorkspaceFile>,
    truncated: bool,
}

#[derive(Serialize)]
struct WorkspaceFile {
    path: String,
    kind: FileKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    invocations: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum FileKind {
    Cargo,
    Rsx,
    Xaml,
    Stages,
}

pub(crate) fn scan(root: &Path) -> Result<Inventory, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let documents = workspace_documents(&root).map_err(|error| error.to_string())?;
    let mut files: Vec<_> = documents
        .into_iter()
        .filter_map(|entry| classify(&root, &entry))
        .take(MAX_ENTRIES + 1)
        .collect();
    let truncated = files.len() > MAX_ENTRIES;
    files.truncate(MAX_ENTRIES);
    Ok(Inventory { files, truncated })
}

fn classify(root: &Path, entry: &DocumentEntry) -> Option<WorkspaceFile> {
    let relative = entry.path.strip_prefix(root).ok()?;
    let path = relative
        .iter()
        .map(|part| part.to_str())
        .collect::<Option<Vec<_>>>()?
        .join("/");
    let (kind, invocations) = match entry.kind {
        DocumentKind::Manifest if relative.file_name()?.to_str()? == "Cargo.toml" => {
            (FileKind::Cargo, None)
        }
        DocumentKind::Xaml => (FileKind::Xaml, None),
        DocumentKind::Rust => {
            let count = count_invocations(&entry.path)?;
            if count == 0 {
                return None;
            }
            (FileKind::Rsx, Some(count))
        }
        DocumentKind::Asset
            if relative
                .extension()?
                .to_str()?
                .eq_ignore_ascii_case("ferrostages") =>
        {
            (FileKind::Stages, None)
        }
        _ => return None,
    };
    Some(WorkspaceFile {
        path,
        kind,
        invocations,
    })
}

fn count_invocations(path: &Path) -> Option<usize> {
    let file = File::open(path).ok()?;
    let max = crate::protocol::MAX_SOURCE_BYTES;
    if file.metadata().ok()?.len() > max as u64 {
        return None;
    }
    let mut source = String::new();
    file.take(max as u64 + 1).read_to_string(&mut source).ok()?;
    (source.len() <= max).then(|| ferroui_designer_rsx::invocation_count(&source))
}
