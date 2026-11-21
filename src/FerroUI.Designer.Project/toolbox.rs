use crate::{Result, require};
use ferroui_designer_core::Document;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const MAX_SNIPPET_BYTES: u64 = 64 * 1024;
const MAX_PROJECT_SNIPPETS: usize = 128;

/// A checked, single-root snippet contributed by a workspace or Cargo package.
pub struct ToolboxSnippet {
    pub title: String,
    pub path: PathBuf,
    pub source: String,
}

/// Read shared workspace entries, then let the nearest Cargo package replace a
/// snippet with the same filename. The active document can be a Cargo manifest.
pub fn toolbox_snippets(root: &Path, document: &Path) -> Result<Vec<ToolboxSnippet>> {
    let root = root.canonicalize()?;
    let document = document.canonicalize()?;
    require(
        document.starts_with(&root),
        "The design is outside the workspace",
    )?;
    let package = document
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .take_while(|parent| *parent != root)
        .find(|parent| parent.join("Cargo.toml").is_file());

    let mut snippets = BTreeMap::new();
    for scope in std::iter::once(root.as_path()).chain(package) {
        let directory = scope.join(".ferroui/toolbox");
        let directory = match directory.canonicalize() {
            Ok(directory) if directory.starts_with(scope) => directory,
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let mut items = directory
            .read_dir()?
            .filter_map(std::result::Result::ok)
            .filter(|entry| {
                entry.path().extension().is_some_and(|ext| ext == "xaml")
                    && entry.file_type().is_ok_and(|kind| kind.is_file())
            })
            .take(MAX_PROJECT_SNIPPETS + 1)
            .collect::<Vec<_>>();
        require(
            items.len() <= MAX_PROJECT_SNIPPETS,
            "Too many project Toolbox snippets",
        )?;
        items.sort_by_key(|item| item.file_name());
        for item in items {
            let path = item.path();
            if !path
                .canonicalize()
                .is_ok_and(|file| file.starts_with(&directory))
            {
                continue;
            }
            let Some(title) = path
                .file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
            else {
                continue;
            };
            let mut source = String::new();
            if File::open(&path)
                .and_then(|file| file.take(MAX_SNIPPET_BYTES + 1).read_to_string(&mut source))
                .is_err()
                || source.len() as u64 > MAX_SNIPPET_BYTES
            {
                continue;
            }
            let Ok(fragment) = Document::new(source) else {
                continue;
            };
            let node = fragment
                .ast()
                .node(fragment.ast().root())
                .expect("valid fragment root");
            let source = fragment.source()[node.span.range()].to_owned();
            let relative = path
                .strip_prefix(&root)
                .expect("bounded snippet path")
                .to_owned();
            snippets.insert(
                title.clone(),
                ToolboxSnippet {
                    title,
                    source,
                    path: relative,
                },
            );
        }
    }
    Ok(snippets.into_values().collect())
}
