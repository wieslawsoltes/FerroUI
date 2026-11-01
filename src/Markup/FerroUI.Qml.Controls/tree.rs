//! Maintain a retained native projection of live QML object identities.
use super::*;

pub(super) type Peers = BTreeMap<ObjectId, Rc<dyn NativeObject>>;
pub(super) type Factories = BTreeMap<String, BTreeMap<String, Factory>>;

pub(super) fn prepare(
    instance: &Instance,
    objects: impl IntoIterator<Item = ObjectId>,
    current: &Peers,
    factories: &Factories,
) -> Result<Peers, Error> {
    let mut peers = Peers::new();
    for object in objects {
        if let Some(peer) = current.get(&object) {
            peers.insert(object, peer.clone());
            continue;
        }
        let Some(factory) = instance
            .module(object)
            .and_then(|module| factories.get(module))
            .and_then(|types| types.get(instance.type_name(object)?))
        else {
            continue;
        };
        let peer = factory();
        for name in instance.property_names(object) {
            let value = instance.get(object, &name)?;
            peer.validate(&name, &value)?;
            peer.apply_initial(&name, &value, instance.is_assigned(object, &name))?;
        }
        peers.insert(object, Rc::from(peer));
    }
    Ok(peers)
}

#[derive(Default)]
pub(super) struct Subscriptions(pub Vec<Rc<dyn IDisposable>>);

impl Drop for Subscriptions {
    fn drop(&mut self) {
        for subscription in self.0.drain(..) {
            subscription.dispose();
        }
    }
}

pub(super) struct Candidate {
    pub revision: u64,
    pub peers: Peers,
}

pub(super) fn affects_layout(peer: &dyn NativeObject, property: &str) -> bool {
    peer.children_property() == Some(property)
        || (peer.uses_item_anchors()
            && (property.starts_with("anchors.")
                || property == "baselineOffset"
                || property == "LayoutMirroring.enabled"))
}

impl Inner {
    pub(super) fn validate(
        &self,
        model: &Instance,
        changes: &[ferroui_qml::Change],
    ) -> Result<(), Error> {
        let current = self.nodes.borrow();
        let topology_changed = self.revision.get() != model.structure_revision();
        let next = if topology_changed {
            Some(prepare(model, model.objects(), &current, &self.factories)?)
        } else {
            None
        };
        let peers = next.as_ref().unwrap_or(&current);
        let graph = self.visual_graph.borrow();
        // A nested native notification may arrive in the middle of applying a
        // previous plan. In that case validate against the complete model.
        let mut full =
            topology_changed || self.flushing.get() || self.validated_layout.borrow().is_some();
        let mut local = std::collections::BTreeSet::new();
        let mut layout_changed = topology_changed;
        for change in changes {
            if let Some(peer) = peers.get(&change.object) {
                peer.validate(&change.property, &change.value)?;
                if affects_layout(&**peer, &change.property) {
                    layout_changed = true;
                    if peer.children_property() == Some(&change.property)
                        || matches!(
                            change.property.as_str(),
                            "baselineOffset" | "anchors.alignWhenCentered"
                        )
                    {
                        full = true;
                    } else {
                        local.insert(change.object);
                    }
                }
            }
        }
        if layout_changed {
            let plan = if full || !graph.is_current() {
                composition::plan(model, peers)?
            } else {
                composition::anchor_changes(model, peers, &graph, local)?
            };
            // Custom peers may replace their native hosts during apply. They
            // still re-plan after applying the property notifications.
            let stable = plan.graph.as_ref().unwrap_or(&graph).stable_hosts;
            self.validated_layout.replace(stable.then_some(plan));
        }
        if let Some(peers) = next {
            self.candidate.replace(Some(Candidate {
                revision: model.structure_revision(),
                peers,
            }));
        }
        Ok(())
    }
}

impl QmlView {
    pub(super) fn accept_topology(&self) -> bool {
        let revision = self.0.instance.borrow().structure_revision();
        if revision == self.0.revision.get() {
            return false;
        }
        let next = self
            .0
            .candidate
            .take()
            .expect("committed topology was validated");
        debug_assert_eq!(next.revision, revision);
        let added: Vec<_> = {
            let old = self.0.nodes.borrow();
            next.peers
                .iter()
                .filter(|(id, _)| !old.contains_key(id))
                .map(|(&id, peer)| (id, peer.clone()))
                .collect()
        };
        let old = self.0.nodes.replace(next.peers);
        self.0.revision.set(revision);
        for (id, peer) in old {
            if self.0.nodes.borrow().contains_key(&id) {
                continue;
            }
            let subscriptions = self.0.subscriptions.borrow_mut().remove(&id);
            drop(subscriptions);
            if let Some((_, children)) = peer.content() {
                children.clear();
            }
            if let Some(canvas) = peer
                .anchor_host()
                .and_then(|host| host.cast::<anchors::AnchorCanvas>())
            {
                canvas.update(Default::default());
            }
        }
        for (id, peer) in added {
            self.connect_peer(id, &*peer);
        }
        true
    }
}
