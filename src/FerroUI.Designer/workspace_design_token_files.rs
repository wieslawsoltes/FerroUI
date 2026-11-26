use super::DesignerWorkspace;
use ferroui_base::{
    platform::storage::{
        FilePickerFileType, FilePickerOpenOptions, FilePickerSaveOptions, IStorageFile,
        IStorageItem,
    },
    Ref,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_designer_tools::token_handoff::Direction;
use std::{io::Read, path::Path, rc::Rc};

#[derive(Clone)]
pub(super) enum TokenSource {
    File {
        file: Rc<dyn IStorageFile>,
        text: Rc<str>,
    },
    Dictionary {
        uri: String,
        revision: u64,
        text: Rc<str>,
    },
}

impl TokenSource {
    pub fn current_dictionary(workspace: &DesignerWorkspace) -> Result<Self, String> {
        let session = workspace.session.borrow();
        if session.uses_rust_source() {
            return Err("Open an authored XAML resource dictionary".into());
        }
        Ok(Self::Dictionary {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            text: workspace.source_editor().text().unwrap_or_default().into(),
        })
    }

    pub fn text(&self) -> &str {
        match self {
            Self::File { text, .. } | Self::Dictionary { text, .. } => text,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::File { file, .. } => file.name(),
            Self::Dictionary { uri, .. } => ferroui_designer_project::file_path(uri)
                .ok()
                .and_then(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| uri.rsplit('/').next().unwrap_or(uri).to_owned()),
        }
    }

    pub fn direction(&self) -> Direction {
        match self {
            Self::File { .. } => Direction::Import,
            Self::Dictionary { .. } => Direction::Export,
        }
    }

    pub async fn verify(&self, workspace: &DesignerWorkspace) -> Result<(), String> {
        let current = match self {
            Self::File { file, text } => read_tokens(file).await? == text.as_ref(),
            Self::Dictionary {
                uri,
                revision,
                text,
            } => {
                let session = workspace.session.borrow();
                uri == session.document_uri()
                    && *revision == session.document().revision()
                    && workspace.source_editor().text().as_deref() == Some(text.as_ref())
            }
        };
        if !current {
            return Err("The source changed. Open it again to review the current bytes.".into());
        }
        Ok(())
    }
}

pub(super) async fn choose_tokens(root: &Ref<Control>) -> Result<Option<TokenSource>, String> {
    let provider = TopLevel::get_top_level(Some(root))
        .ok_or("Attach the token review to a window")?
        .storage_provider();
    if !provider.can_open() {
        return Err("This host has no file picker".into());
    }
    let mut options = FilePickerOpenOptions::new();
    options.set_title(Some("Open design tokens".into()));
    options.set_allow_multiple(false);
    options.set_file_type_filter(Some(vec![
        FilePickerFileType::new(Some("Design tokens")).with_patterns(&["*.tokens", "*.json"])
    ]));
    let files = provider
        .open_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?;
    let Some(file) = files.into_iter().next() else {
        return Ok(None);
    };
    let text = read_tokens(&file).await?.into();
    Ok(Some(TokenSource::File { file, text }))
}

async fn read_tokens(file: &Rc<dyn IStorageFile>) -> Result<String, String> {
    let stream = file
        .open_read_async()
        .await
        .map_err(|error| error.to_string())?;
    let mut text = String::new();
    stream
        .take(1_048_577)
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    if text.len() > 1_048_576 {
        return Err("The token handoff source exceeds 1 MiB".into());
    }
    Ok(text)
}

pub(super) async fn choose_destination(
    root: &Ref<Control>,
    direction: Direction,
) -> Result<Option<String>, String> {
    let provider = TopLevel::get_top_level(Some(root))
        .ok_or("Attach the token review to a window")?
        .storage_provider();
    if !provider.can_save() {
        return Err("This host has no save picker".into());
    }
    let (extension, name, label, pattern) = match direction {
        Direction::Import => ("xaml", "DesignTokens.xaml", "FerroUI dictionary", "*.xaml"),
        Direction::Export => ("json", "design.tokens.json", "Design tokens JSON", "*.json"),
    };
    let mut options = FilePickerSaveOptions::new();
    options.set_title(Some("Create the reviewed file".into()));
    options.set_default_extension(Some(extension.into()));
    options.set_suggested_file_name(Some(name.into()));
    options.set_file_type_choices(Some(vec![
        FilePickerFileType::new(Some(label)).with_patterns(&[pattern])
    ]));
    options.set_show_overwrite_prompt(Some(false));
    let Some(file) = provider
        .save_file_picker_async(options)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let item: Rc<dyn IStorageItem> = file;
    let path = item
        .try_get_local_path()
        .ok_or("Choose a local path for the reviewed file")?;
    if !Path::new(&path)
        .extension()
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case(extension))
    {
        return Err(format!("Choose a new .{extension} file"));
    }
    Ok(Some(path))
}
