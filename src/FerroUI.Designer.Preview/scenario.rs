//! A native preview test uses the same renderer, source projections and input as the editor.
use crate::protocol::{Image, Language, Render, Source, MAX_SOURCE_BYTES};
use crate::renderer::Renderer;
use crate::scenario_review::{Inputs, Options, Review};
use crate::scenario_stages::PrototypeRun;
use base64::{engine::general_purpose::STANDARD, Engine};
use ferroui_designer_language::Position;
use ferroui_designer_tools::scenario::{
    Scenario, ScenarioPosition, ScenarioReport, ScenarioStep, ScenarioStepReport,
};
use ferroui_reload_protocol::fingerprint;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub(crate) fn read_text(path: &Path) -> Result<String, String> {
    let error = |error: io::Error| format!("{}: {error}", path.display());
    let mut text = String::new();
    File::open(path)
        .map_err(error)?
        .take((MAX_SOURCE_BYTES + 1) as u64)
        .read_to_string(&mut text)
        .map_err(error)?;
    if text.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "{} exceeds the 1 MiB preview limit",
            path.display()
        ));
    }
    Ok(text)
}

pub(super) fn run(
    path: &Path,
    workspace: &Path,
    artifacts: &Path,
    options: &Options,
) -> Result<ScenarioReport, String> {
    let base = path
        .parent()
        .ok_or("The scenario needs a containing folder")?;
    let source = read_text(path)?;
    let scenario: Scenario = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .map_err(|error| error.to_string())?;
    scenario.validate()?;
    let document_path = existing_workspace_path(base.join(&scenario.document), workspace)?;
    let document = read_source(&document_path, scenario.caret)?;
    let mut review = Review::new(
        Inputs {
            scenario: path,
            scenario_text: &source,
            document: &document_path,
            document_hash: fingerprint(document.source.as_bytes()),
            workspace,
            artifacts,
        },
        options,
    )?;
    let (mut surface, notices) = match &scenario.prototype {
        Some(prototype) => (
            ScenarioSurface::Prototype(PrototypeRun::start(document, prototype, base, workspace)?),
            Vec::new(),
        ),
        None => {
            let input = Render {
                rust_project: None,
                document,
                width: scenario.viewport.width,
                height: scenario.viewport.height,
                dark: scenario.viewport.dark,
                theme: None,
                frame: 1,
                workspace_root: Some(workspace.into()),
                local_sources: Vec::new(),
                design_data: None,
                layer_view: None,
                timeline_pose: None,
                timeline_targets: None,
            };
            let mut renderer = Renderer::new();
            let preview = crate::snapshot::render(&mut renderer, &input, None)?;
            if preview.image.is_none() {
                return Err(preview
                    .render_error
                    .unwrap_or_else(|| "No accepted native preview".into()));
            }
            (
                ScenarioSurface::Design {
                    renderer,
                    frame: input.frame,
                },
                preview.notices,
            )
        }
    };
    let mut report = ScenarioReport {
        name: scenario.name,
        document: document_path.clone(),
        passed: true,
        notices,
        steps: Vec::new(),
        reference: review.reference(),
        visuals: Vec::new(),
        approved: None,
    };
    let mut assertions_passed = true;
    for (index, step) in scenario.steps.into_iter().enumerate() {
        let step_number = index + 1;
        let screenshot = matches!(step, ScenarioStep::ExpectScreenshot { .. });
        let result = match step {
            ScenarioStep::ExpectScreenshot {
                baseline,
                channel_tolerance,
                max_changed_pixels,
            } => surface.capture().and_then(|Image { png, .. }| {
                let image = STANDARD.decode(png).map_err(|error| error.to_string())?;
                review.capture(
                    &baseline,
                    step_number,
                    image,
                    channel_tolerance,
                    max_changed_pixels,
                )
            }),
            action => surface.inspect(action).map(|message| ScenarioStepReport {
                step: step_number,
                passed: true,
                message,
                artifacts: Vec::new(),
            }),
        };
        let may_continue = screenshot && result.is_ok() && review.continues_after_difference();
        let result = result.unwrap_or_else(|message| ScenarioStepReport {
            step: step_number,
            passed: false,
            message,
            artifacts: Vec::new(),
        });
        let passed = result.passed;
        report.passed &= passed;
        report.steps.push(result);
        if !passed && !may_continue {
            assertions_passed = false;
            break;
        }
    }
    review.finish(&mut report, assertions_passed)?;
    Ok(report)
}

pub(crate) fn existing_workspace_path(path: PathBuf, workspace: &Path) -> Result<PathBuf, String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    if !path.starts_with(workspace) || !path.is_file() {
        return Err("Scenario inputs must be files in the selected workspace".into());
    }
    Ok(path)
}

pub(crate) fn read_source(path: &Path, caret: ScenarioPosition) -> Result<Source, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let language = match extension.as_str() {
        "xaml" => Language::Xaml,
        "rs" => Language::Rust,
        _ => return Err("Choose a FerroUI XAML or Rust view".into()),
    };
    Ok(Source {
        language,
        uri: ferroui_designer_project::file_uri(path).map_err(|error| error.to_string())?,
        source: read_text(path)?,
        caret: Position {
            line: caret.line,
            character: caret.character,
        },
        project: None,
    })
}

enum ScenarioSurface {
    Design { renderer: Renderer, frame: u64 },
    Prototype(PrototypeRun),
}

impl ScenarioSurface {
    fn capture(&mut self) -> Result<Image, String> {
        match self {
            Self::Design { renderer, frame } => {
                let session = &renderer.stage(*frame)?.session;
                Ok(crate::renderer::capture(&mut session.borrow_mut())?.image)
            }
            Self::Prototype(prototype) => prototype.capture(),
        }
    }

    fn inspect(&mut self, step: ScenarioStep) -> Result<String, String> {
        match self {
            Self::Design { renderer, frame } => {
                let host = renderer
                    .stage(*frame)?
                    .session
                    .borrow()
                    .preview_host()
                    .upcast();
                crate::scenario_input::execute(&host, step)
            }
            Self::Prototype(prototype) => prototype.inspect(step),
        }
    }
}
