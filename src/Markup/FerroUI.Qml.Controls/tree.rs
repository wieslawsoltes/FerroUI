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
