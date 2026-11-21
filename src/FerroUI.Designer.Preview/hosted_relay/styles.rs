//! Scoped styles over the same accepted frame and XAML buffers as palettes.
use super::{
    authored::{AuthoredPreview, Request},
    Relay,
};
use ferroui_designer::{ScopedStyleEdit, ScopedStyles};
use ferroui_designer_core::{Document, NodeId};
use ferroui_designer_language::Range;
use ferroui_designer_rsx::RsxStyleEdit;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub(crate) enum Action {
    Files,
    Catalog {
        address: String,
    },
    Read {
        target: Target,
    },
    Resources {
        target: Target,
        property: String,
    },
    Plan {
        target: Target,
        edit: Edit,
        source: String,
    },
    Source { address: String },
    Locate { address: String, range: Range, source: String },
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum Edit {
    Rust(RustEdit),
    Xaml(ScopedStyleEdit),
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RustEdit {
    Rust { edit: RsxStyleEdit },
}

#[derive(Deserialize)]
pub(crate) struct Target {
    pub(super) address: String,
    pub(super) node: NodeId,
}

impl Relay {
    pub(crate) fn styles(&self, request: Request<Action>) -> Result<Value, String> {
        if let Some(rust) = &self.authored_rsx {
            self.require_authored_frame(request.frame)?;
            return rust.style_action(request.action);
        }
        self.authored(request.frame)?.style_action(request.action)
    }
}

impl AuthoredPreview {
    fn style_action(&self, action: Action) -> Result<Value, String> {
        match action {
            Action::Files => Ok(
                json!({"documents":self.documents().map(|(address, uri, _)| json!({"address":address,"uri":uri}))
                .collect::<Vec<_>>()}),
            ),
            Action::Catalog { address } => {
                let (uri, document) = self.source_at(&address)?;
                Ok(json!({"uri":uri,"catalog":self.style_scope(document,&address).catalog()?}))
            }
            Action::Read { target } => {
                let (uri, document) = self.source_at(&target.address)?;
                Ok(
                    json!({"uri":uri,"definition":self.style_scope(document,&target.address).definition(target.node)?}),
                )
            }
            Action::Resources { target, property } => {
                let (_, document) = self.source_at(&target.address)?;
                self.style_scope(document, &target.address)
                    .resources(target.node, &property)
            }
            Action::Plan {
                target,
                edit: Edit::Xaml(edit),
                source,
            } => self.plan(&target.address, &source, |document| {
                self.style_scope(document, &target.address)
                    .plan(target.node, &edit)
            }),
            Action::Plan { .. } => Err("Choose an XAML style edit for this preview".into()),
            Action::Source { address } => Ok(json!({"uri":self.source_at(&address)?.0})),
            Action::Locate { address, range, source } => self.locate(&address, range, &source),
        }
    }

    fn style_scope<'a>(&'a self, document: &'a Document, address: &'a str) -> ScopedStyles<'a> {
        ScopedStyles::new(document, address, &self.sources.includes, &self.schema)
    }
}
