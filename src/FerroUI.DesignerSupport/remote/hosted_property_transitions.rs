//! Resolve and plan authored property transitions beside the application's controls.
use super::{HostedAnimations, ACTIVE, PROPERTY_TRANSITIONS};
use ferroui_base::{
    animation::Animatable,
    styling::{ControlTheme, Style},
    Ref, StyledElement,
};
use ferroui_designer_animation::native::{
    element_type, plan_xaml_property_transition, transition_choices, transition_target_type,
};
use ferroui_designer_animation::{PropertyTransitionCommand, PropertyTransitionList};
use ferroui_designer_core::{Document, NodeId, TextPatch};
use ferroui_remote_protocol::designer::{
    DesignAnimationCatalogMessage, DesignAnimationSourcePatch, DesignAnimationTarget,
    DesignPropertyTransitionsRequest, DesignPropertyTransitionsResult, DesignTransitionProperty,
};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub(in crate::remote) fn request(
    request: &DesignPropertyTransitionsRequest,
) -> DesignPropertyTransitionsResult {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !PROPERTY_TRANSITIONS.get() {
            return Err("The app preview did not negotiate property transitions".into());
        }
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            let host = active
                .as_mut()
                .ok_or("Open the current application preview first")?;
            host.claim(request.revision, request.request)?;
            host.property_transitions(request)
        })
    }));
    match result {
        Ok(Ok(result)) => result,
        failure => DesignPropertyTransitionsResult {
            revision: request.revision,
            request: request.request,
            error: Some(failure.ok().and_then(Result::err).unwrap_or_else(|| {
                "The app preview could not resolve the transition collection".into()
            })),
            ..Default::default()
        },
    }
}

pub(super) struct HostedPropertyTransitions {
    document: Rc<Document>,
    owners: BTreeSet<NodeId>,
}

impl HostedPropertyTransitions {
    pub(super) fn mount(
        document: Rc<Document>,
        targets: &BTreeMap<i64, Ref<StyledElement>>,
        catalog: &mut DesignAnimationCatalogMessage,
    ) -> Self {
        let offered = document
            .ast()
            .elements()
            .filter_map(|node| {
                let id = i64::try_from(node.id).ok()?;
                if let Some(control) = targets
                    .get(&id)
                    .filter(|control| control.cast::<Animatable>().is_some())
                {
                    let type_name = control.get_type().name().to_owned();
                    return Some(DesignAnimationTarget {
                        id,
                        label: control.name().unwrap_or_else(|| type_name.clone()),
                        type_name,
                    });
                }
                let local = node.local_name();
                if !matches!(local, "Style" | "ControlTheme" | "Transitions") {
                    return None;
                }
                let list = PropertyTransitionList::read(&document, node.id).ok()?;
                let label = match local {
                    "Style" | "ControlTheme"
                        if element_type(&document, node.id).is_some_and(|type_| {
                            std::ptr::eq(type_, Style::TYPE)
                                || std::ptr::eq(type_, ControlTheme::TYPE)
                        }) =>
                    {
                        if list.source.is_none()
                            && transition_target_type(&document, &list, None, |_| None)
                                .ok()?
                                .is_none()
                        {
                            return None;
                        }
                        node.attribute("Selector")
                            .or_else(|| node.attribute("TargetType"))
                            .unwrap_or(local)
                    }
                    "Transitions"
                        if list.target_owner.is_none() && list.collection == Some(node.id) =>
                    {
                        node.attributes
                            .iter()
                            .find(|attribute| {
                                attribute.namespace_uri.as_deref()
                                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                                    && attribute.name.rsplit(':').next() == Some("Key")
                            })
                            .map_or(local, |attribute| attribute.value.as_str())
                    }
                    _ => return None,
                };
                Some(DesignAnimationTarget {
                    id,
                    label: label.into(),
                    type_name: local.into(),
                })
            })
            .collect::<Vec<_>>();
        let owners = offered.iter().map(|target| target.id as NodeId).collect();
        catalog.property_transition_owners = Some(offered);
        Self { document, owners }
    }
}

impl HostedAnimations {
    fn property_transitions(
        &self,
        request: &DesignPropertyTransitionsRequest,
    ) -> Result<DesignPropertyTransitionsResult, String> {
        let hosted = self
            .property_transitions
            .as_ref()
            .ok_or("Read the accepted app preview first")?;
        let owner = NodeId::try_from(request.owner)
            .ok()
            .filter(|owner| hosted.owners.contains(owner))
            .ok_or("Choose an authored transition owner in the current App Preview")?;
        let document = hosted.document.as_ref();
        let list =
            PropertyTransitionList::read(document, owner).map_err(|error| error.to_string())?;
        let resource_target = request.target_type.as_deref();
        let live_type = |node| {
            self.targets
                .get(&i64::try_from(node).ok()?)
                .map(|control| control.get_type())
        };
        let mut result = DesignPropertyTransitionsResult {
            revision: request.revision,
            request: request.request,
            ..Default::default()
        };
        if let Some(command) = &request.command {
            let command: PropertyTransitionCommand =
                serde_json::from_str(command).map_err(|error| error.to_string())?;
            let edits = plan_xaml_property_transition(
                document,
                &list,
                resource_target,
                live_type,
                &command,
            )?;
            let mut updated = document.clone();
            updated
                .apply(command.label(), edits)
                .map_err(|error| error.to_string())?;
            result.patch = TextPatch::between(document.source(), updated.source())
                .map(|patch| {
                    DesignAnimationSourcePatch::from_utf8(
                        document.source(),
                        patch.span.range(),
                        patch.replacement,
                    )
                })
                .transpose()?;
        } else {
            result.properties =
                transition_target_type(document, &list, resource_target, live_type)?
                    .into_iter()
                    .flat_map(transition_choices)
                    .map(|choice| DesignTransitionProperty {
                        property: choice.description.property,
                        label: choice.description.label,
                        type_name: choice.description.type_name.into(),
                    })
                    .collect();
        }
        Ok(result)
    }
}
