use crate::design_window_loader::LoadedDesign;
use ferroui_base::{Ref, StyledElement};
use ferroui_designer_animation::StatePreviewGroup;
use ferroui_designer_animation::native::{NativeSourceIndex, NativeStateRoute, NativeStateStyles};
use ferroui_designer_core::{Document, NodeId};
use ferroui_remote_protocol::designer::{DesignAnimationCatalogMessage, DesignAnimationProgram};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) struct HostedStateRoutes(BTreeMap<i64, NativeStateRoute>);

impl HostedStateRoutes {
    pub(super) fn mount(
        document: &Document,
        groups: Vec<StatePreviewGroup>,
        locations: &NativeSourceIndex,
        design: &LoadedDesign,
        targets: &mut BTreeMap<i64, Ref<StyledElement>>,
        catalog: &mut DesignAnimationCatalogMessage,
    ) -> Self {
        let fixture = design
            .rust_fixture
            .then(|| design.root.clone().upcast::<StyledElement>());
        let mut bindings = BTreeMap::new();
        for group in groups {
            let owner = group.model.owner;
            if let Some(root) = &fixture {
                let Some(control) = fixture_control(document, owner, root) else {
                    continue;
                };
                if catalog.state_authoring == Some(true) {
                    if let Ok(id) = i64::try_from(owner) {
                        targets.insert(id, control);
                    }
                }
            }
            let styles = if fixture.is_some() {
                NativeStateStyles::default()
            } else {
                i64::try_from(owner).ok().and_then(|id| targets.get(&id))
                    .and_then(|owner| catch_unwind(AssertUnwindSafe(|| {
                        NativeStateStyles::discover(locations, owner, &group)
                    })).ok()).unwrap_or_default()
            };
            for route in &group.routes {
                let Ok(id) = i64::try_from(route.id) else {
                    continue;
                };
                let mut matched = BTreeMap::new();
                // Property conversion and animator registration can invoke application code.
                let native = catch_unwind(AssertUnwindSafe(|| {
                    NativeStateRoute::bind_loaded(document, &group, &route.transition, &styles, |target| {
                        let id = i64::try_from(target).ok()?;
                        let control = if let Some(root) = &fixture {
                            fixture_control(document, target, root)?
                        } else {
                            targets.get(&id)?.clone()
                        };
                        matched.insert(id, control.clone());
                        Some(control)
                    })
                }));
                let Ok(Ok(native)) = native else { continue };
                let targets_in_route = native
                    .targets()
                    .filter_map(|id| i64::try_from(id).ok())
                    .collect::<Vec<_>>();
                targets.extend(matched);
                let transition = &route.transition;
                let from = transition.from.as_deref().unwrap_or("Base");
                let name = if route.endpoint {
                    format!("{} · {}", group.model.name, transition.to)
                } else {
                    format!("{} · {from} → {}", group.model.name, transition.to)
                };
                catalog.programs.push(DesignAnimationProgram {
                    id,
                    name,
                    selector: String::new(),
                    duration: transition.duration,
                    total_duration: Some(transition.duration),
                    default_targets: targets_in_route.clone(),
                    compatible_targets: targets_in_route,
                    source_utf16: -1,
                    ..DesignAnimationProgram::default()
                });
                bindings.insert(id, native);
            }
        }
        Self(bindings)
    }

    pub(super) fn get(&self, id: i64) -> Option<&NativeStateRoute> {
        self.0.get(&id)
    }
}

fn fixture_control(
    document: &Document,
    target: NodeId,
    root: &Ref<StyledElement>,
) -> Option<Ref<StyledElement>> {
    let ast = document.ast();
    let mut levels = Vec::new();
    let mut id = Some(target);
    while let Some(current) = id {
        let node = ast.node(current)?;
        if !node.local_name().contains('.') {
            levels.push(node);
        }
        id = node.parent;
    }
    let matches = |control: &StyledElement, node: &ferroui_designer_core::Node| {
        control.get_type().name() == node.local_name()
            && control.name().as_deref() == node.xaml_name()
    };
    if !matches(root, levels.pop()?) {
        return None;
    }
    let mut control = root.clone();
    for node in levels.into_iter().rev() {
        // A logical path is only useful as a fixture source map while every
        // step has one match. Templates and dynamically repeated controls stay live.
        let children = control.logical_children().snapshot();
        let mut matches = children.iter().filter(|child| matches(child, node));
        control = matches.next().filter(|_| matches.next().is_none())?.clone();
    }
    Some(control)
}
