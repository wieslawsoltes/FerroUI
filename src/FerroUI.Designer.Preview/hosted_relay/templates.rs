//! Resolve native template uses against the exact XAML accepted by App Preview.
use super::{
    authored::{AuthoredPreview, Request},
    Relay,
};
use ferroui_designer::ScopedControlTemplates;
use ferroui_designer_core::NodeId;
use ferroui_designer_language::{Range, XamlLocations};
use ferroui_designer_project::{resolve_uri, xaml_source_revision};
use ferroui_designer_tools::ItemTemplateMember;
use ferroui_remote_protocol::designer::{DesignTemplateCatalogMessage, DesignTemplateSource};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub(crate) enum Action {
    Catalog,
    PlanCopy { owner: NodeId, source: String },
}

pub(super) struct Templates(Vec<Template>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Template {
    owner: NodeId,
    label: String,
    type_name: String,
    definition: Location,
    can_copy: bool,
}

#[derive(PartialEq, Serialize)]
struct Location {
    address: String,
    uri: String,
    node: NodeId,
    range: Range,
}

struct AcceptedSource<'a> {
    address: &'a str,
    uri: &'a str,
    locations: XamlLocations<'a>,
}

impl Relay {
    pub(crate) fn templates(&self, request: Request<Action>) -> Result<Value, String> {
        let authored = self.authored(request.frame)?;
        let templates = self
            .templates
            .as_ref()
            .ok_or("Rebuild the application with App Preview template support")?;
        match request.action {
            Action::Catalog => Ok(json!({ "templates": templates.0 })),
            Action::PlanCopy { owner, source } => {
                let template = templates
                    .0
                    .iter()
                    .find(|entry| entry.owner == owner)
                    .filter(|entry| entry.can_copy)
                    .ok_or("Choose an inherited template from the accepted application preview")?;
                let address = &authored.sources.base_uri;
                let planner = authored.template_planner();
                let mut plan = authored.plan(address, &source, |_| {
                    planner.plan_copy(
                        owner,
                        &template.definition.address,
                        template.definition.node,
                    )
                })?;
                plan["includes"] = json!(authored.sources.includes.documents().map(|include| {
                    json!({ "uri": include.uri, "sourceRevision": format!("{:016x}", include.source_revision()) })
                }).collect::<Vec<_>>());
                Ok(plan)
            }
        }
    }
}

impl Templates {
    pub(super) fn capture(
        authored: &AuthoredPreview,
        catalog: DesignTemplateCatalogMessage,
    ) -> Self {
        let address = authored.sources.base_uri.as_str();
        let document = &authored.document;
        let planner = authored.template_planner();
        let mut sources = Sources {
            authored,
            cache: BTreeMap::new(),
        };
        let mut owners = BTreeMap::<NodeId, Option<Location>>::new();
        for usage in &catalog.templates {
            let Some(owner) = sources
                .locate(&usage.owner, &usage.type_name)
                .filter(|location| location.address == address)
            else {
                continue;
            };
            let definition = usage
                .definition
                .as_ref()
                .and_then(|stamp| sources.locate(stamp, "ControlTemplate"));
            owners
                .entry(owner.node)
                .and_modify(|previous| {
                    if *previous != definition {
                        *previous = None;
                    }
                })
                .or_insert(definition);
        }
        Self(
            owners
                .into_iter()
                .filter_map(|(owner, definition)| {
                    let definition = definition?;
                    let node = document.ast().node(owner)?;
                    Some(Template {
                        owner,
                        label: node
                            .attribute("x:Name")
                            .or_else(|| node.attribute("Name"))
                            .unwrap_or_else(|| node.local_name())
                            .to_owned(),
                        type_name: node.local_name().to_owned(),
                        definition,
                        can_copy: planner.can_copy(owner),
                    })
                })
                .collect(),
        )
    }
}

impl AuthoredPreview {
    fn template_planner(&self) -> ScopedControlTemplates<'_> {
        ScopedControlTemplates::new(
            &self.document,
            &self.sources.base_uri,
            &self.sources.includes,
            &self.schema,
        )
    }
}

struct Sources<'a> {
    authored: &'a AuthoredPreview,
    cache: BTreeMap<(&'a str, u64), Option<AcceptedSource<'a>>>,
}

impl<'a> Sources<'a> {
    fn locate(&mut self, stamp: &'a DesignTemplateSource, type_name: &str) -> Option<Location> {
        let fingerprint = stamp.fingerprint as u64;
        let authored = self.authored;
        let source = self
            .cache
            .entry((&stamp.address, fingerprint))
            .or_insert_with(|| accepted_source(authored, &stamp.address, fingerprint))
            .as_ref()?;
        let node = source
            .locations
            .element(stamp.line, stamp.column, type_name)?;
        if type_name == "ControlTemplate" && !ItemTemplateMember::Template.accepts(node) {
            return None;
        }
        Some(Location {
            address: source.address.to_owned(),
            uri: source.uri.to_owned(),
            node: node.id,
            range: source.locations.name_range(node)?,
        })
    }
}

fn accepted_source<'a>(
    authored: &'a AuthoredPreview,
    address: &str,
    fingerprint: u64,
) -> Option<AcceptedSource<'a>> {
    let base = authored.sources.base_uri.as_str();
    let (address, uri, document) =
        if address == base || resolve_uri(base, "").ok().as_deref() == Some(address) {
            let (uri, document) = authored.source_at(base).ok()?;
            if xaml_source_revision(document) != fingerprint {
                return None;
            }
            (base, uri, document)
        } else {
            let (address, include) = authored
                .sources
                .includes
                .authored_revision(address, fingerprint)?;
            (address, include.uri.as_str(), &include.document)
        };
    Some(AcceptedSource {
        address,
        uri,
        locations: XamlLocations::new(document),
    })
}
