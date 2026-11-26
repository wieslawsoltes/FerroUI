//! Check the active Cargo and Rust tooling from the selected project directory.
use crate::{
    project_templates, require, OutputStream, ProjectError, Result, TaskHandle, TaskStatus,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub directory: PathBuf,
    pub target: Option<String>,
    pub minimum_rust: Option<String>,
    /// A Cargo executable explicitly configured by the editor.
    pub cargo: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub directory: PathBuf,
    pub cargo: Probe,
    pub rustc: Probe,
    pub host: Option<String>,
    pub target: Option<String>,
    pub target_installed: Option<bool>,
    pub target_error: Option<String>,
    pub minimum_rust: String,
    pub meets_minimum: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct Probe {
    pub version: Option<String>,
    pub error: Option<String>,
}

pub fn inspect(request: &Request) -> Result<Report> {
    let directory = request.directory.canonicalize()?;
    require(directory.is_dir(), "Choose a project directory")?;
    if let Some(target) = &request.target {
        require(
            !target.is_empty()
                && target.len() <= 100
                && target.as_bytes()[0].is_ascii_alphanumeric()
                && target
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')),
            "Choose a Rust target triple",
        )?;
    }
    let minimum_rust = request
        .minimum_rust
        .as_deref()
        .unwrap_or(project_templates::RUST_VERSION);
    let minimum = version_parts(minimum_rust)
        .ok_or_else(|| ProjectError("Expected a numeric Rust version".into()))?;
    let cargo = request
        .cargo
        .as_deref()
        .unwrap_or_else(|| Path::new("cargo"));
    let rust_output = run(&directory, Path::new("rustc"), &["-vV"]);
    let host = rust_output
        .as_ref()
        .ok()
        .and_then(|output| named_line(output, "host: "));
    let release = rust_output
        .as_ref()
        .ok()
        .and_then(|output| named_line(output, "release: "));
    let target = request.target.clone().or_else(|| host.clone());
    let (target_installed, target_error) = match &target {
        Some(triple) if rust_output.is_ok() => {
            match run(
                &directory,
                Path::new("rustc"),
                &["--print", "target-libdir", "--target", triple],
            ) {
                Ok(output) => (Some(has_core_library(Path::new(output.trim()))), None),
                Err(error) => (None, Some(error)),
            }
        }
        _ => (None, None),
    };
    Ok(Report {
        cargo: version_probe(run(&directory, cargo, &["--version"])),
        rustc: version_probe(rust_output),
        directory,
        host,
        target,
        target_installed,
        target_error,
        minimum_rust: minimum_rust.into(),
        meets_minimum: release
            .and_then(|version| version_parts(&version))
            .map(|active| active >= minimum),
    })
}

fn version_probe(result: std::result::Result<String, String>) -> Probe {
    match result {
        Ok(output) => Probe {
            version: output.lines().next().map(str::to_owned),
            error: None,
        },
        Err(error) => Probe {
            version: None,
            error: Some(error),
        },
    }
}

fn named_line(output: &str, prefix: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.strip_prefix(prefix).map(str::to_owned))
}

fn version_parts(version: &str) -> Option<[u32; 3]> {
    let mut fields = version.split('-').next()?.split('.');
    let major = fields.next()?.parse().ok()?;
    let minor = fields.next()?.parse().ok()?;
    let patch = fields.next().unwrap_or("0").parse().ok()?;
    fields.next().is_none().then_some([major, minor, patch])
}

fn has_core_library(directory: &Path) -> bool {
    std::fs::read_dir(directory).is_ok_and(|mut entries| {
        entries.any(|entry| {
            entry.ok().is_some_and(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("libcore-") && name.ends_with(".rlib"))
            })
        })
    })
}

fn run(
    directory: &Path,
    program: &Path,
    arguments: &[&str],
) -> std::result::Result<String, String> {
    let mut command = Command::new(program);
    command
        .current_dir(directory)
        .args(arguments)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut probe = TaskHandle::spawn(&mut command, None, Default::default())
        .map_err(|error| format!("{}: {error}", program.display()))?;
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match probe.poll() {
            TaskStatus::Exited { code } => {
                let success = code == Some(0);
                let stream = if success {
                    OutputStream::Stdout
                } else {
                    OutputStream::Stderr
                };
                let output = probe
                    .take_output()
                    .into_iter()
                    .filter(|chunk| chunk.stream == stream)
                    .flat_map(|chunk| chunk.text.chars().take(2_048).collect::<Vec<_>>())
                    .take(2_048)
                    .collect::<String>();
                return if success {
                    Ok(output)
                } else {
                    Err(if output.trim().is_empty() {
                        format!("{} exited with {code:?}", program.display())
                    } else {
                        output.trim().to_owned()
                    })
                };
            }
            TaskStatus::Running if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(15))
            }
            TaskStatus::Running => {
                return Err(format!(
                    "{} did not respond within 3 seconds",
                    program.display()
                ))
            }
            TaskStatus::Failed { message } => return Err(message),
            TaskStatus::Cancelled => return Err("Toolchain check was cancelled".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_msrvs_and_requires_an_actual_target_library() {
        assert_eq!(version_parts("1.89"), Some([1, 89, 0]));
        assert_eq!(version_parts("1.91.0-nightly"), Some([1, 91, 0]));
        assert_eq!(version_parts("1.bad"), None);
        assert!(!has_core_library(Path::new(env!("CARGO_MANIFEST_DIR"))));
        let result = inspect(&Request {
            directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            target: Some("../target.json".into()),
            ..Default::default()
        });
        assert!(result.unwrap_err().to_string().contains("target triple"));
    }
}
