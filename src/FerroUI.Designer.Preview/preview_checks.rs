//! Translate findings from an accepted design into the UTF-16 coordinates of its editor buffer.
use crate::renderer::Stage;
use ferroui_designer::PreviewFinding;
use ferroui_designer_core::Document;
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_rsx::RsxSourceMap;
use ferroui_mcp::UiToolProvider;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
pub(crate) struct SourceFinding {
    #[serde(flatten)]
    finding: PreviewFinding,
    range: Option<Range>,
}

pub(crate) fn source_findings(
    findings: Vec<PreviewFinding>,
    document: &Document,
    source: &str,
    rsx: Option<&RsxSourceMap>,
) -> Vec<SourceFinding> {
    let index = LineIndex::new(source);
    findings
        .into_iter()
        .map(|mut finding| {
            if let Some(map) = rsx {
                finding.map_rsx_source(document, map);
            }
            let range = finding.source.and_then(|span| index.range(span).ok());
            SourceFinding { finding, range }
        })
        .collect()
}

pub(crate) fn query(stage: &Stage, arguments: &Value) -> Result<Value, String> {
    stage
        .studio
        .validate("designer_preview_checks", arguments)?;
    let session = stage.session.borrow();
    let document = session.document();
    let findings = session.preview_checks()?;
    let findings =
        if let Some(authored) = stage.authored_rsx.as_ref().filter(|_| !findings.is_empty()) {
            let projection = authored.projection()?;
            let (_, map) = projection.mapped_markup();
            source_findings(findings, document, authored.source(), Some(&map))
        } else {
            source_findings(findings, document, document.source(), None)
        };
    Ok(json!({"revision":document.revision(),"findings":findings}))
}
