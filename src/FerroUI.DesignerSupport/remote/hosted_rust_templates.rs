//! Read template properties of the actual app-hosted RSX controls.
use super::{data_slots, MAX_USES, MAX_VISUALS};
use crate::design_window_loader::LoadedDesign;
use ferroui_base::diagnostics::RsxSourceInfo;
use ferroui_controls::{diagnostics::column_templates, templates::IDataTemplate};
use ferroui_remote_protocol::designer::{
    DesignTemplateCatalogMessage, DesignTemplateSource, DesignTemplateUse,
};
use std::rc::Rc;
use url::Url;

pub(super) fn capture(
    revision: i64,
    design: &LoadedDesign,
) -> Option<DesignTemplateCatalogMessage> {
    let mut templates = Vec::new();
    for (index, visual) in design.root.get_self_and_visual_descendants().enumerate() {
        if index == MAX_VISUALS {
            return None;
        }
        let Some(info) = RsxSourceInfo::get(&visual) else {
            continue;
        };
        let Some(owner) = editor_location(info, false) else {
            continue;
        };
        let type_name = visual.get_type().name();
        for (member, template) in data_slots(&visual) {
            templates.push(usage(info, &owner, type_name, member, None, template));
        }
        for slot in column_templates(&visual, MAX_USES.saturating_sub(templates.len()))? {
            templates.push(usage(
                info,
                &owner,
                type_name,
                slot.member.name(),
                Some((i32::try_from(slot.index).ok()?, slot.expander_inner)),
                slot.template,
            ));
        }
        if templates.len() >= MAX_USES {
            return None;
        }
    }
    Some(DesignTemplateCatalogMessage {
        revision,
        templates,
    })
}

fn usage(
    info: RsxSourceInfo,
    owner: &DesignTemplateSource,
    type_name: &str,
    member: &str,
    column: Option<(i32, bool)>,
    template: Option<Rc<dyn IDataTemplate>>,
) -> DesignTemplateUse {
    let provenance = template.as_ref().and_then(|item| item.rust_editor_source());
    let body = provenance.and_then(|source| source.rendered_body);
    let definition = body.and_then(|body| editor_location(body, true));
    let rust_factory = provenance.and_then(|source| {
        let constructor = source.constructor;
        let path = RsxSourceInfo::resolve_file(constructor.file(), info.manifest)?;
        let address = Url::from_file_path(path).ok()?;
        let fingerprint = [Some(info), body].into_iter().flatten().find_map(|proof| {
            (proof.source_uri()? == address)
                .then_some(proof.compiled.revision)
                .flatten()
        })?;
        Some(DesignTemplateSource {
            address: address.into(),
            fingerprint: fingerprint as i64,
            line: i32::try_from(constructor.line()).ok()?,
            column: i32::try_from(constructor.column()).ok()?,
        })
    });
    DesignTemplateUse {
        owner: owner.clone(),
        type_name: type_name.into(),
        definition,
        member: Some(member.into()),
        column: column.map(|(index, _)| index),
        column_inner: column.filter(|(_, inner)| *inner).map(|(_, inner)| inner),
        rust_factory,
    }
}

fn editor_location(info: RsxSourceInfo, invocation: bool) -> Option<DesignTemplateSource> {
    let location = info.location();
    let (line, column) = if invocation {
        location.invocation
    } else {
        location.element
    };
    Some(DesignTemplateSource {
        address: info.source_uri()?.into(),
        fingerprint: location.revision? as i64,
        line: i32::try_from(line).ok()?,
        column: i32::try_from(column).ok()?,
    })
}
