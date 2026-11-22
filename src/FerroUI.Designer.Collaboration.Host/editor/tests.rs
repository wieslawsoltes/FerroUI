use super::*;
use std::path::PathBuf;

const SOURCE: &str = "<Panel>\r\n  <Button Content=\"Ready · 🪴\" />\r\n</Panel>";

struct RoomPair {
    root: PathBuf,
    owner: Option<EditorRoom>,
    guest: Option<EditorRoom>,
}

impl RoomPair {
    fn new(role: Role) -> Self {
        let root =
            std::env::temp_dir().join(format!("ferroui-editor-{}", Secret::generate().unwrap().0));
        let mut owner = Self::open(
            &root,
            "host",
            StartRoom::Host {
                title: "Review".into(),
                source: SOURCE.into(),
                display_name: "Owner".into(),
                public_url: None,
                bind: None,
            },
        );
        let invitation = owner.invite(role).unwrap();
        let guest = Self::open(
            &root,
            "guest",
            StartRoom::Join {
                invitation,
                display_name: "Reviewer".into(),
            },
        );
        Self {
            root,
            owner: Some(owner),
            guest: Some(guest),
        }
    }

    fn open(root: &std::path::Path, name: &str, start: StartRoom) -> EditorRoom {
        let mut room = OpenRoom {
            directory: root.join(name),
            uri: format!("file:///{name}.xaml"),
            start,
        }
        .connect()
        .unwrap();
        room.tick(true);
        room
    }

    fn owner(&mut self) -> &mut EditorRoom {
        self.owner.as_mut().unwrap()
    }
    fn guest(&mut self) -> &mut EditorRoom {
        self.guest.as_mut().unwrap()
    }
}

impl Drop for RoomPair {
    fn drop(&mut self) {
        self.guest.take();
        self.owner.take();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn revision(room: &EditorRoom) -> String {
    room.review_target().unwrap().revision()
}
fn source(room: &EditorRoom) -> &str {
    room.review_target().unwrap().replica.source()
}

#[test]
fn editor_room_checks_remote_versions_roles_and_source_backups() {
    let mut rooms = RoomPair::new(Role::Viewer);
    let stale = revision(rooms.guest());
    assert_eq!(
        rooms
            .guest()
            .publish(&stale, SOURCE.into())
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    let first = SOURCE.replace("Ready", "Reviewed");
    let owner = rooms.owner();
    owner.publish(&revision(owner), first.clone()).unwrap();
    assert_eq!(owner.status().pending, 1);
    owner.tick(true);
    let guest = rooms.guest();
    assert_eq!(
        guest.prepare_apply(&stale, SOURCE).unwrap_err().code,
        ErrorCode::Conflict
    );
    guest.tick(true);
    assert_eq!(source(guest), first);
    let local = SOURCE.replace("Ready", "Local editor draft");
    guest.prepare_apply(&revision(guest), &local).unwrap();
    assert_eq!(guest.recovery_source().unwrap(), local);
    let actor = guest.peer.document().actor().to_owned();
    rooms
        .owner()
        .set_member(actor, Role::Editor, false)
        .unwrap();
    let guest = rooms.guest();
    guest.tick(true);
    assert_eq!(guest.role(), Role::Editor);
    guest.publish(&revision(guest), local.clone()).unwrap();
    guest.tick(true);
    rooms.owner().tick(true);
    assert_eq!(source(rooms.owner()), local);
}

#[test]
fn saved_rooms_resume_an_outbox_and_require_review_after_downgrade_and_checkpoint() {
    let mut rooms = RoomPair::new(Role::Editor);
    let next = SOURCE.replace("Ready", "Offline draft");
    let guest = rooms.guest();
    guest.publish(&revision(guest), next.clone()).unwrap();
    rooms.owner.take();
    rooms.guest().tick(true);
    assert_eq!(rooms.guest().status().pending, 1);
    rooms.guest.take();
    rooms.owner = Some(RoomPair::open(&rooms.root, "host", StartRoom::Resume));
    rooms.guest = Some(RoomPair::open(&rooms.root, "guest", StartRoom::Resume));
    rooms.owner().tick(true);
    assert_eq!(source(rooms.owner()), next);

    let guest = rooms.guest();
    let private = SOURCE.replace("Ready", "Keep this unsubmitted draft");
    guest.publish(&revision(guest), private.clone()).unwrap();
    let actor = guest.peer.document().actor().to_owned();
    rooms
        .owner()
        .set_member(actor, Role::Viewer, false)
        .unwrap();
    let guest = rooms.guest();
    guest.tick(true);
    assert!(guest.review_target().unwrap().rebase);
    let review = revision(guest);
    assert_eq!(
        guest
            .resolve(&review, &private, Some(&private))
            .unwrap_err()
            .code,
        ErrorCode::Forbidden
    );
    guest.resolve(&review, &private, None).unwrap();
    assert_eq!(guest.recovery_source().unwrap(), private);
    assert_eq!(source(guest), next);
    assert!(guest.storage.directory.join("previous-peer.json").is_file());

    let owner = rooms.owner();
    owner.tick(true);
    let checkpoint = SOURCE.replace("Ready", "Reviewed baseline");
    owner
        .checkpoint(&revision(owner), &next, Some(checkpoint.clone()))
        .unwrap();
    let guest = rooms.guest();
    guest.tick(true);
    assert!(guest.review_target().unwrap().rebase);
    guest.resolve(&revision(guest), &next, None).unwrap();
    assert_eq!(source(guest), checkpoint);
}
