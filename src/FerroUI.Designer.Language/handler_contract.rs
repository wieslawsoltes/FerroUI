use super::{HandlerRequest, LanguageService};
use crate::handler_project::HandlerSource;
use crate::{LanguageError, Result, event_handler_name, event_object_span, require};
use ferroui_build_scan::model::TypeModel;
use ferroui_designer_core::{Document, Node};
use ferroui_designer_project::{CargoTarget, MarkupCatalog, MarkupEventSignature};
use std::path::Path;

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

pub(super) struct Contract<'a> {
    root: &'a Path,
    pub(super) catalog: &'a MarkupCatalog,
    pub(super) class: &'a TypeModel,
    crate_name: String,
    targets: Vec<CargoTarget>,
    pub(super) node: &'a Node,
    pub(super) event: String,
    pub(super) signature: MarkupEventSignature,
    pub(super) existing: Option<&'a str>,
}

impl Contract<'_> {
    pub(super) fn source(&self) -> Result<HandlerSource> {
        HandlerSource::locate(self.root, self.class, &self.crate_name, &self.targets)
    }
}

impl LanguageService {
    pub(super) fn handler_contract<'a>(
        &'a self,
        uri: &str,
        request: &HandlerRequest,
    ) -> Result<Contract<'a>> {
        let state = self.state(uri)?;
        require(
            state.version == request.version,
            "The XAML buffer changed. Prepare the handler again",
        )?;
        require(
            state.document.is_valid(),
            "Repair the XAML before editing an event",
        )?;
        let (root, project) = self
            .project_schema_for(uri)
            .ok_or_else(|| LanguageError("Build Project XAML Schema first".into()))?;
        let (node, event, existing) = event_at(
            &state.document,
            state.index.offset(request.position)?,
            request.event.as_deref(),
        )?;
        if request.event.is_none() {
            require(
                node.attribute_info(&event).is_some_and(|attribute| {
                    project
                        .effective
                        .attribute_property(&state.document, node, attribute)
                        .is_some_and(|info| info.event)
                }),
                "Choose an event attribute",
            )?;
        }
        let class = state
            .document
            .ast()
            .node(state.document.ast().root())
            .and_then(|node| {
                node.attributes.iter().find(|attribute| {
                    attribute.namespace_uri.as_deref() == Some(XAML)
                        && attribute.name.rsplit(':').next() == Some("Class")
                })
            })
            .ok_or_else(|| {
                LanguageError("This XAML view needs an x:Class with a registered Rust owner".into())
            })?;
        let (package, targets) = self
            .cargo_package_targets(root)?
            .ok_or_else(|| LanguageError("The code-behind package has no Cargo targets".into()))?;
        let catalog = &project.catalog;
        let crate_names = targets
            .iter()
            .map(|target| target.name.replace('-', "_"))
            .chain(std::iter::once(package.replace('-', "_")))
            .collect();
        let owner = catalog
            .code_behind_type(&class.value, &crate_names)
            .ok_or_else(|| {
                LanguageError(
                    "The project's build metadata has no unambiguous Rust owner for x:Class".into(),
                )
            })?;
        let target = catalog
            .xml_type(
                node.namespace_uri.as_deref().unwrap_or_default(),
                node.local_name(),
            )
            .ok_or_else(|| {
                LanguageError(
                    "This control is unavailable in the project's XAML type registry".into(),
                )
            })?;
        let namespaces = state.document.ast().namespaces(node.id);
        let (prefix, local) = event
            .split_once(':')
            .map_or(("", event.as_str()), |(prefix, local)| (prefix, local));
        let (declaring, member) = if let Some((declaring, member)) = local.split_once('.') {
            let namespace = namespaces
                .get(prefix)
                .map(String::as_str)
                .unwrap_or_default();
            let declaring = catalog
                .xml_type(namespace, declaring)
                .ok_or_else(|| LanguageError("The event's declaring type is unavailable".into()))?;
            (declaring.class, member)
        } else {
            require(
                prefix.is_empty(),
                "An event qualifier needs a declaring type",
            )?;
            (target.class, local)
        };
        require(
            event_object_span(&state.document, node, member).is_none(),
            "Edit the existing event object in source",
        )?;
        if existing.is_none() {
            require(
                !node.attributes.iter().any(|attribute| {
                    attribute.name != event
                        && attribute.name.rsplit([':', '.']).next() == Some(member)
                        && project
                            .effective
                            .attribute_property(&state.document, node, attribute)
                            .is_some_and(|property| property.event)
                }),
                "This event already has a qualified or unqualified declaration",
            )?;
        }
        let crate_name = owner.assembly.crate_name.clone();
        let signature = catalog
            .event_signature(target.class, declaring, member, &crate_name)
            .ok_or_else(|| {
                LanguageError("The build did not export a supported event signature".into())
            })?;
        Ok(Contract {
            root,
            catalog,
            class: owner.class,
            crate_name,
            targets,
            node,
            event,
            signature,
            existing,
        })
    }
}

pub(super) fn event_at<'a>(
    document: &'a Document,
    offset: usize,
    requested: Option<&str>,
) -> Result<(&'a Node, String, Option<&'a str>)> {
    let node = document
        .ast()
        .element_at(offset)
        .and_then(|id| document.ast().node(id))
        .ok_or_else(|| LanguageError("Select an authored event or control".into()))?;
    require(
        node.opening_span.contains(offset),
        "Select an event in an opening tag",
    )?;
    let event = requested
        .map(str::to_owned)
        .or_else(|| {
            node.attributes
                .iter()
                .find(|a| a.span.contains(offset))
                .map(|a| a.name.clone())
        })
        .ok_or_else(|| LanguageError("Choose an event attribute".into()))?;
    let existing = node.attribute(&event).filter(|value| !value.is_empty());
    require(
        existing.is_none_or(event_handler_name),
        "Edit the existing event expression in source",
    )?;
    Ok((node, event, existing))
}
