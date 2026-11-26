//! Write a portable review that can be opened from a downloaded CI artifact.
use ferroui_designer_tools::scenario::{ScenarioStepReport, ScenarioSuiteReport, VisualReview};
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

const PAGE_START: &str = r#"<!doctype html><html lang="en"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>FerroUI scenario review</title><style>
:root{color-scheme:light dark;font:15px/1.55 system-ui;color:CanvasText;background:Canvas}
body{max-width:1200px;margin:0 auto;padding:24px}h1{line-height:1.2;margin:0 0 .4em}
p{margin:.45em 0}details{border-block-start:1px solid color-mix(in srgb,CanvasText 20%,Canvas);padding:12px 0}
summary{cursor:pointer;font-weight:650}.pass{color:#168063}.fail{color:#c43b42}
@media(prefers-color-scheme:dark){.pass{color:#62d3ae}.fail{color:#ff8794}}
.gallery{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(240px,100%),1fr));gap:12px;margin:12px 0}
figure{margin:0;padding:10px;border:1px solid color-mix(in srgb,CanvasText 20%,Canvas);border-radius:5px}
figcaption{margin-block-end:6px;font-weight:600}img{display:block;max-width:100%;height:auto}
li{margin-block:4px}code{overflow-wrap:anywhere}
</style><main><h1>FerroUI scenario review</h1>"#;

pub(super) fn write(root: &Path, workspace: &Path, report: &ScenarioSuiteReport) -> io::Result<()> {
    fs::create_dir_all(root)?;
    let mut output = BufWriter::new(File::create(&report.review_report)?);
    output.write_all(PAGE_START.as_bytes())?;
    let failed = report
        .scenarios
        .iter()
        .filter(|entry| entry.report.as_ref().is_none_or(|result| !result.passed))
        .count();
    writeln!(
        output,
        "<p>{} scenarios passed · {failed} failed</p>",
        report.scenarios.len() - failed
    )?;
    for entry in &report.scenarios {
        let relative = entry.path.strip_prefix(workspace).unwrap_or(&entry.path);
        let name = entry
            .report
            .as_ref()
            .map(|report| report.name.clone())
            .unwrap_or_else(|| relative.display().to_string());
        let passed = entry.report.as_ref().is_some_and(|result| result.passed);
        let visual = entry
            .report
            .as_ref()
            .is_some_and(|result| !result.visuals.is_empty());
        let expanded = if !passed || visual { " open" } else { "" };
        writeln!(
            output,
            "<details{expanded}><summary>{} · {}</summary><p><code>{}</code></p>",
            escape(&name),
            badge(passed),
            escape(&relative.display().to_string())
        )?;
        if let Some(error) = &entry.error {
            writeln!(output, "<p class=fail>{}</p>", escape(error))?;
        }
        if let Some(result) = &entry.report {
            if let Some(reference) = &result.reference {
                writeln!(
                    output,
                    "<p>Merge base of {} · <code>{}</code></p>",
                    escape(&reference.name),
                    escape(&reference.commit)
                )?;
            }
            output.write_all(b"<ol>")?;
            for step in &result.steps {
                step_row(
                    &mut output,
                    root,
                    step,
                    result.visuals.iter().find(|item| item.step == step.step),
                )?;
            }
            output.write_all(b"</ol>")?;
        }
        output.write_all(b"</details>\n")?;
    }
    output.write_all(b"</main></html>\n")?;
    output.flush()
}

fn step_row(
    out: &mut impl Write,
    root: &Path,
    step: &ScenarioStepReport,
    review: Option<&VisualReview>,
) -> io::Result<()> {
    write!(
        out,
        "<li>{} · {}",
        badge(step.passed),
        escape(&step.message)
    )?;
    if let Some(review) = review {
        if let Some(changed) = review.comparison.changed_pixels {
            write!(out, "<p>{changed} pixels differ from the review image.</p>")?;
        }
    }
    if !step.artifacts.is_empty() {
        out.write_all(b"<div class=gallery>")?;
        for artifact in &step.artifacts {
            let Ok(relative) = artifact.strip_prefix(root) else {
                continue;
            };
            let Some(name) = artifact.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let label = if name.ends_with("-expected.png") {
                "Expected"
            } else if name.ends_with("-diff.png") {
                "Difference"
            } else {
                "Actual"
            };
            let url = escape(&relative.display().to_string().replace('\\', "/"));
            writeln!(out, "<figure><figcaption>{label}</figcaption><a href=\"{url}\"><img src=\"{url}\" alt=\"{label} screenshot\" loading=lazy></a></figure>")?;
        }
        out.write_all(b"</div>")?;
    }
    out.write_all(b"</li>\n")
}

fn badge(passed: bool) -> &'static str {
    if passed {
        "<span class=pass>Passed</span>"
    } else {
        "<span class=fail>Failed</span>"
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
