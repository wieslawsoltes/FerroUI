//! Regressions of the pending-disposal queue and its index by server-object id.
use super::{Compositor, ServerObjectId};
use crate::rendering::composition::server::IServerObject;
use crate::rendering::composition::test_compositor::TestCompositor;
use std::any::Any;
use std::rc::Rc;

struct Server;
impl IServerObject for Server {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

fn create_objects(compositor: &Compositor, count: usize) -> Vec<ServerObjectId> {
    (0..count)
        .map(|_| compositor.create_server_object(|_, _| Rc::new(Server)))
        .collect()
}

fn queued(compositor: &Compositor) -> Vec<(ServerObjectId, bool, bool)> {
    compositor
        .dispose_on_next_batch
        .borrow()
        .iter()
        .map(|entry| (entry.id, entry.dispose, entry.release))
        .collect()
}

#[test]
fn disposal_requests_coalesce_without_changing_order() {
    let fixture = TestCompositor::new();
    let c = &fixture.compositor;
    let ids = create_objects(c, 6);
    let (first, second) = (ids[5], ids[2]);
    assert!(c.queue_disposal(first, true, false));
    assert!(c.queue_disposal(second, false, true));
    assert!(!c.queue_disposal(first, true, false));
    assert!(c.queue_disposal(first, false, true));
    assert!(!c.queue_disposal(first, true, true));
    assert_eq!(vec![(first, true, true), (second, false, true)], queued(c));
}

#[test]
fn out_of_band_disposal_keeps_releases_and_reindexes_remaining_entries() {
    let fixture = TestCompositor::new();
    let c = &fixture.compositor;
    let ids = create_objects(c, 4);
    for (index, id) in ids.iter().enumerate() {
        c.queue_disposal(*id, true, index == 2);
    }
    c.oob_dispose(ids[1]);
    c.oob_dispose(ids[2]);
    assert!(!c.queue_disposal(ids[3], true, false));
    assert!(c.queue_disposal(ids[3], false, true));
    assert!(c.queue_disposal(ids[1], false, true));
    assert_eq!(
        vec![
            (ids[0], true, false),
            (ids[2], false, true),
            (ids[3], true, true),
            (ids[1], false, true)
        ],
        queued(c)
    );
    fixture.run_jobs();
}

#[test]
fn object_ids_can_be_queued_again_after_a_batch_is_committed() {
    let fixture = TestCompositor::new();
    let c = &fixture.compositor;
    for _ in 0..3 {
        let ids = create_objects(c, 10_000);
        for id in ids.iter().rev() {
            assert!(c.queue_disposal(*id, true, false));
        }
        for id in &ids {
            assert!(c.queue_disposal(*id, false, true));
            assert!(!c.queue_disposal(*id, true, true));
        }
        assert_eq!(10_000, queued(c).len());
        assert_eq!(ids[9_999], queued(c)[0].0);
        fixture.run_jobs();
        assert!(c.dispose_on_next_batch.borrow().is_empty());
        assert!(c.dispose_on_next_batch_index.borrow().iter().all(|slot| *slot == 0));
        assert_eq!(0, c.server().object_count());
    }
}
