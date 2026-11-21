//! Capture source provenance from the actual loaded app, including custom themes.
use crate::design_window_loader::LoadedDesign;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use ferroui_remote_protocol::designer::{
    DesignTemplateCatalogMessage, DesignTemplateSource, DesignTemplateUse, DESIGN_TEMPLATES_QUERY,
    DESIGN_TEMPLATES_VERSION,
};
use std::cell::Cell;
use url::Url;

const MAX_VISUALS: usize = 8_192;
const MAX_USES: usize = 2_000;

ferroui_base::ferro_thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn configure(transport: Option<&Url>, remote: bool) {
    ENABLED.set(
        super::hosted_sources::requested(transport, remote)
            && transport.is_some_and(|uri| {
                uri.query_pairs().any(|(key, version)| {
                    key == DESIGN_TEMPLATES_QUERY && version == DESIGN_TEMPLATES_VERSION
                })
            }),
    );
}

pub(super) fn enabled() -> bool {
    ENABLED.get()
}

pub(super) fn capture(
    revision: i64,
    design: &LoadedDesign,
) -> Option<DesignTemplateCatalogMessage> {
    if !enabled() || design.rust_fixture {
        return None;
    }
    let root = XamlSourceInfo::for_object(&design.root).and_then(source)?;
    let mut templates = Vec::new();
    for (index, visual) in design.root.get_self_and_visual_descendants().enumerate() {
        // Partial traversal could miss another live instance of the same authored control.
        if index == MAX_VISUALS || templates.len() == MAX_USES {
            return None;
        }
        let Some(control) = visual.cast::<TemplatedControl>() else {
            continue;
        };
        let Some(owner) = XamlSourceInfo::for_object(&control).and_then(source) else {
            continue;
        };
        if owner.address != root.address || owner.fingerprint != root.fingerprint {
            continue;
        }
        let definition = control
            .template()
            .and_then(|template| XamlSourceInfo::for_control_template(&template).and_then(source));
        templates.push(DesignTemplateUse {
            owner,
            type_name: control.get_type().name().into(),
            definition,
        });
    }
    Some(DesignTemplateCatalogMessage {
        revision,
        templates,
    })
}

fn source(info: XamlSourceInfo) -> Option<DesignTemplateSource> {
    let address = info
        .loader_uri()
        .or_else(|| info.source_uri().map(Url::as_str))?;
    Some(DesignTemplateSource {
        address: address.to_owned(),
        fingerprint: info.source_revision()? as i64,
        line: info.line_number(),
        column: info.line_position(),
    })
}
