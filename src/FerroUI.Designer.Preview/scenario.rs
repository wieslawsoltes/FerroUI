//! A native preview test uses the same renderer, source projections and input as the editor.
use crate::protocol::{Image, Language, Render, Source, MAX_SOURCE_BYTES};
use crate::renderer::Renderer;
use base64::{engine::general_purpose::STANDARD, Engine};
use ferroui_designer_language::Position;
use ferroui_designer_tools::scenario::{Scenario, ScenarioStep};
use serde::Serialize;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Report {
    name: String,
    document: PathBuf,
    passed: bool,
    notices: Vec<String>,
    steps: Vec<StepResult>,
}

#[derive(Serialize)]
struct StepResult {
    step: usize,
    passed: bool,
    message: String,
    artifacts: Vec<PathBuf>,
}

pub(crate) fn command() -> io::Result<()> {
    let mut args = std::env::args_os().skip(2);
    let scenario = PathBuf::from(
        args.next()
            .ok_or_else(|| io::Error::other("Pass a FerroUI scenario file"))?,
    )
    .canonicalize()?;
    let mut accept = false;
    let mut artifacts = std::env::temp_dir()
        .join("ferroui-scenario-results")
        .join(std::process::id().to_string());
    let mut workspace = scenario
        .parent()
        .ok_or_else(|| io::Error::other("The scenario needs a containing folder"))?
        .to_owned();
    while let Some(arg) = args.next() {
        if arg == "--accept-baselines" {
            accept = true;
        } else if arg == "--artifacts" {
            artifacts = args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| io::Error::other("Pass an artifact directory"))?;
        } else if arg == "--workspace" {
            workspace = PathBuf::from(
                args.next()
                    .ok_or_else(|| io::Error::other("Pass the workspace directory"))?,
            )
            .canonicalize()?;
        } else {
            return Err(io::Error::other(
                "Use --accept-baselines, --artifacts, or --workspace with a scenario file",
            ));
        }
    }
    if !scenario.starts_with(&workspace) || !workspace.is_dir() {
        return Err(io::Error::other(
            "The scenario must belong to the selected workspace",
        ));
    }
    let mut output = ferroui_mcp::reserve_stdout()?;
    let report = run(&scenario, &workspace, &artifacts, accept).map_err(io::Error::other)?;
    serde_json::to_writer(&mut output, &report)?;
    output.write_all(b"\n")?;
    output.flush()?;
    if report.passed {
        Ok(())
    } else {
        Err(io::Error::other("Scenario failed"))
    }
}

fn read_text(path: &Path) -> Result<String, String> {
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

fn run(path: &Path, workspace: &Path, artifacts: &Path, accept: bool) -> Result<Report, String> {
    let base = path
        .parent()
        .ok_or("The scenario needs a containing folder")?;
    let source = read_text(path)?;
    let scenario: Scenario = serde_json::from_str(source.trim_start_matches('\u{feff}'))
        .map_err(|error| error.to_string())?;
    scenario.validate()?;
    let document_path = base
        .join(&scenario.document)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !document_path.starts_with(workspace) {
        return Err("Scenario source must belong to the selected workspace".into());
    }
    let extension = document_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let language = match extension.as_str() {
        "xaml" => Language::Xaml,
        "rs" => Language::Rust,
        _ => return Err("Choose a FerroUI XAML or Rust view".into()),
    };
    let input = Render {
        document: Source {
            language,
            uri: ferroui_designer_project::file_uri(&document_path)
                .map_err(|error| error.to_string())?,
            source: read_text(&document_path)?,
            caret: Position {
                line: scenario.caret.line,
                character: scenario.caret.character,
            },
            project: None,
        },
        width: scenario.viewport.width,
        height: scenario.viewport.height,
        dark: scenario.viewport.dark,
        frame: 1,
        workspace_root: None,
    };
    let mut renderer = Renderer::new();
    let preview = crate::snapshot::render(&mut renderer, &input, None)?;
    if preview.image.is_none() {
        return Err(preview
            .render_error
            .unwrap_or_else(|| "No accepted native preview".into()));
    }
    let mut runner = Run {
        renderer,
        frame: input.frame,
        base,
        workspace,
        artifacts,
        accept,
        baselines: Vec::new(),
    };
    let mut report = Report {
        name: scenario.name,
        document: document_path,
        passed: true,
        notices: preview.notices,
        steps: Vec::new(),
    };
    for (index, step) in scenario.steps.into_iter().enumerate() {
        let result = runner.step(step, index + 1);
        report.passed = result.passed;
        report.steps.push(result);
        if !report.passed {
            break;
        }
    }
    if report.passed {
        for (index, path, image) in runner.baselines {
            crate::scenario_images::write_baseline(&path, &image)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            report.steps[index - 1].message = format!("Accepted {}", path.display());
        }
    }
    Ok(report)
}

struct Run<'a> {
    renderer: Renderer,
    frame: u64,
    base: &'a Path,
    workspace: &'a Path,
    artifacts: &'a Path,
    accept: bool,
    baselines: Vec<(usize, PathBuf, Vec<u8>)>,
}

impl Run<'_> {
    fn step(&mut self, step: ScenarioStep, number: usize) -> StepResult {
        let result = match step {
            ScenarioStep::ExpectScreenshot {
                baseline,
                max_changed_pixels,
                channel_tolerance,
            } => self.screenshot(&baseline, number, channel_tolerance, max_changed_pixels),
            step => self
                .inspect(step)
                .map(|message| (true, message, Vec::new())),
        };
        let (passed, message, artifacts) =
            result.unwrap_or_else(|error| (false, error, Vec::new()));
        StepResult {
            step: number,
            passed,
            message,
            artifacts,
        }
    }

    fn screenshot(
        &mut self,
        baseline: &str,
        step: usize,
        tolerance: u8,
        maximum: u32,
    ) -> Result<(bool, String, Vec<PathBuf>), String> {
        if !Path::new(baseline)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
        {
            return Err("Screenshot baselines require a .png path".into());
        }
        let session = &self.renderer.stage(self.frame)?.session;
        let Image { png, .. } = crate::renderer::capture(&mut session.borrow_mut())?.image;
        let image = STANDARD.decode(png).map_err(|error| error.to_string())?;
        let baseline = workspace_path(self.base, self.workspace, baseline)?;
        if self.accept {
            if self.baselines.iter().any(|(_, path, _)| path == &baseline) {
                return Err("Give each captured screenshot its own baseline file".into());
            }
            self.baselines.push((step, baseline.clone(), image));
            return Ok((
                true,
                "Captured screenshot for acceptance".into(),
                vec![baseline],
            ));
        }
        let result = crate::scenario_images::check(
            &image,
            &baseline,
            self.artifacts,
            step,
            tolerance,
            maximum,
        )?;
        Ok((!result.failure, result.message, result.artifacts))
    }

    fn inspect(&self, step: ScenarioStep) -> Result<String, String> {
        let host = self
            .renderer
            .stage(self.frame)?
            .session
            .borrow()
            .preview_host();
        crate::scenario_input::execute(&host, step)
    }
}

fn workspace_path(base: &Path, workspace: &Path, name: &str) -> Result<PathBuf, String> {
    use ferroui_designer_project::{file_path, file_uri};

    let path = file_uri(base.join(name))
        .and_then(|uri| file_path(&uri))
        .map_err(|error| error.to_string())?;
    let parent = path
        .ancestors()
        .find(|entry| entry.exists())
        .ok_or("Baseline directory is unavailable")?;
    let resolved = parent.canonicalize().map_err(|error| error.to_string())?;
    if !resolved.starts_with(workspace) {
        return Err("Screenshot baselines must be inside the workspace".into());
    }
    let tail = path
        .strip_prefix(parent)
        .map_err(|error| error.to_string())?;
    Ok(if tail.as_os_str().is_empty() {
        resolved
    } else {
        resolved.join(tail)
    })
}
