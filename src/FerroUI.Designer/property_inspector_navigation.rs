//! Source navigation from property rows, using the same identities as property edits.
use super::*;
use ferroui_designer_core::{Ast, MarkupValue, SourceSpan};

#[derive(Clone)]
struct PropertyNavigationSnapshot {
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    node: NodeId,
    scope: Option<NodeId>,
    scope_epoch: u64,
}

impl PropertyNavigationSnapshot {
    fn capture(workspace: &DesignerWorkspace) -> Result<Self, String> {
        if workspace.pending_source.get() {
            return Err(
                "Apply or cancel the source draft before navigating from a property".into(),
            );
        }
        if workspace.runtime_selection.get().is_some() {
            return Err("Runtime-only properties do not have authored source".into());
        }
        let session = workspace.session.borrow();
        if !session.document().is_valid() {
            return Err("Fix the XAML syntax before navigating from a property".into());
        }
        let [node] = session.selection() else {
            return Err("Select one authored element to inspect its property source".into());
        };
        Ok(Self {
            uri: session.document_uri().into(),
            revision: session.document().revision(),
            ast: session.document().ast_snapshot(),
            node: *node,
            scope: session.definition_scope().map(|scope| scope.node),
            scope_epoch: session.definition_scope_epoch(),
        })
    }

    fn check(&self, workspace: &DesignerWorkspace) -> Result<(), String> {
        let current = Self::capture(workspace)?;
        if self.uri == current.uri
            && self.revision == current.revision
            && Arc::ptr_eq(&self.ast, &current.ast)
            && self.node == current.node
            && self.scope == current.scope
            && self.scope_epoch == current.scope_epoch
        {
            Ok(())
        } else {
            Err(
                "The document, selection or definition scope changed; reopen the property menu"
                    .into(),
            )
        }
    }
}

enum PropertyReferenceTarget {
    Resource(NodeId),
    Name(usize),
}

fn authored_span(session: &DesignerSession, node: NodeId, property: &str) -> Option<SourceSpan> {
    let document = session.document();
    let node = document.ast().node(node)?;
    authored_property_attribute(document, node, property)
        .map(|attribute| attribute.span)
        .or_else(|| property_element(document, node, property).map(|element| element.span))
}

fn reference_target(
    session: &DesignerSession,
    node: NodeId,
    property: &str,
) -> Result<PropertyReferenceTarget, String> {
    let document = session.document();
    let element = document
        .ast()
        .node(node)
        .ok_or("The property target is unavailable")?;
    let attribute = authored_property_attribute(document, element, property)
        .ok_or("This property has no authored expression to navigate")?;
    let MarkupValue::Extension(extension) =
        attribute.markup().map_err(|error| error.to_string())?
    else {
        return Err("This property has no authored reference to navigate".into());
    };
    let extension_name =
        crate::property_inspector_view::framework_extension(document, node, &extension.name)
            .ok_or("This application expression does not declare a designer reference")?;
    match extension_name {
        "StaticResource" | "DynamicResource" => {
            let arguments = extension
                .arguments
                .iter()
                .filter(|argument| {
                    argument
                        .name
                        .as_deref()
                        .is_none_or(|name| name == "ResourceKey")
                })
                .collect::<Vec<_>>();
            let [argument] = arguments.as_slice() else {
                return Err("The resource expression must have one string key".into());
            };
            let MarkupValue::Literal(key) = &argument.value else {
                return Err("An application resource key must be inspected in XAML".into());
            };
            let context = session.component_resource_context(node);
            let resource = ferroui_designer_tools::component_resource_resolution(
                document,
                node,
                key,
                context.as_ref(),
            )
            .map_err(|error| error.to_string())?
            .resolved()
            .ok_or("The resource is external, missing or ambiguous in this preview context")?;
            Ok(PropertyReferenceTarget::Resource(resource))
        }
        "Binding" | "CompiledBinding" | "ReflectionBinding" => {
            let occurrences = ferroui_designer_language::name_occurrences(document);
            let mut references = occurrences
                .iter()
                .filter(|reference| {
                    !reference.declaration
                        && reference.node == node
                        && reference.span.start >= attribute.value_span.start
                        && reference.span.end <= attribute.value_span.end
                })
                .collect::<Vec<_>>();
            references.sort_by(|a, b| a.name.cmp(&b.name));
            references.dedup_by(|a, b| a.name == b.name && a.scope == b.scope);
            let [reference] = references.as_slice() else {
                return Err("This binding does not reference one unambiguous named element".into());
            };
            if occurrences
                .iter()
                .filter(|occurrence| {
                    occurrence.declaration
                        && occurrence.scope == reference.scope
                        && occurrence.name == reference.name
                })
                .count()
                != 1
            {
                return Err("The binding source is missing or ambiguous in this name scope".into());
            }
            Ok(PropertyReferenceTarget::Name(reference.span.start))
        }
        _ => Err("This expression has no authored resource or element reference".into()),
    }
}

impl DesignerWorkspace {
    /// Select the original authored attribute or property element in the XAML editor.
    /// Navigation preserves selection, history and authored values.
    pub fn reveal_property_source(self: &Rc<Self>, property: &str) -> Result<(), String> {
        let snapshot = PropertyNavigationSnapshot::capture(self)?;
        let span = authored_span(&self.session.borrow(), snapshot.node, property)
            .ok_or("This property has no authored value in the current document")?;
        self.select_source_span(span);
        Ok(())
    }

    /// Navigate to a uniquely resolved resource or named binding source. Theme and
    /// name scopes are respected; ambiguous or external targets remain explicit errors.
    pub fn go_to_property_definition(self: &Rc<Self>, property: &str) -> Result<(), String> {
        let snapshot = PropertyNavigationSnapshot::capture(self)?;
        let target = reference_target(&self.session.borrow(), snapshot.node, property)?;
        match target {
            PropertyReferenceTarget::Resource(node) => {
                let span = self
                    .session
                    .borrow()
                    .document()
                    .ast()
                    .node(node)
                    .map(|node| node.span)
                    .ok_or("The resource definition is unavailable")?;
                self.select_source_span(span);
            }
            PropertyReferenceTarget::Name(offset) => {
                self.synchronize_language_service()?;
                snapshot.check(self)?;
                let locations = self
                    .try_language_service()?
                    .definitions(&snapshot.uri, offset)
                    .map_err(|error| error.to_string())?;
                let [location] = locations.as_slice() else {
                    return Err(
                        "The binding source is missing or ambiguous in this name scope".into(),
                    );
                };
                self.navigate_to(location)?;
            }
        }
        Ok(())
    }

    pub(super) fn property_source_actions(
        self: &Rc<Self>,
        property: &str,
        source: InspectorSource,
    ) -> Vec<(String, Rc<dyn Fn()>)> {
        let Ok(snapshot) = PropertyNavigationSnapshot::capture(self) else {
            return Vec::new();
        };
        let session = self.session.borrow();
        if authored_span(&session, snapshot.node, property).is_none() {
            return Vec::new();
        }
        let has_reference = matches!(source, InspectorSource::Binding | InspectorSource::Resource)
            && reference_target(&session, snapshot.node, property).is_ok();
        drop(session);
        let mut commands = vec![("Show authored XAML", false)];
        if has_reference {
            commands.push(("Go to referenced definition", true));
        }
        commands
            .into_iter()
            .map(|(label, definition)| {
                let weak = Rc::downgrade(self);
                let snapshot = snapshot.clone();
                let property = property.to_owned();
                (
                    label.into(),
                    Rc::new(move || {
                        let Some(this) = weak.upgrade() else {
                            return;
                        };
                        let result = snapshot.check(&this).and_then(|_| {
                            if definition {
                                this.go_to_property_definition(&property)
                            } else {
                                this.reveal_property_source(&property)
                            }
                        });
                        this.report(result);
                    }) as Rc<dyn Fn()>,
                )
            })
            .collect()
    }
}
