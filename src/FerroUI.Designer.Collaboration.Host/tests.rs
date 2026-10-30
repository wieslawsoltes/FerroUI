use super::*;
use ferroui_designer_collaboration::{DocumentReplica, SharedEdit, VersionVector};
use std::path::PathBuf;
use std::sync::Arc;

struct Fixture {
    path: PathBuf,
    admin: Secret,
    store: Arc<RoomStore>,
    owner: JoinedRoom,
}
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("ferroui-rooms-{}", Secret::generate().unwrap().0));
        let admin = Secret::generate().unwrap();
        let store = Arc::new(RoomStore::open(&path, &admin).unwrap());
        // Match RoomStore's canonical path on platforms where the temporary directory's parent
        // is an alias (for example /var -> /private/var on macOS).
        let path = std::fs::canonicalize(path).unwrap();
        let owner = store
            .create(CreateRequest {
                admin: admin.clone(),
                owner_token: Secret::generate().unwrap(),
                title: "Design review".into(),
                document_id: "app/Main.xaml".into(),
                source: "<Panel><Button Name=\"a\"/><Button Name=\"b\"/></Panel>".into(),
                display_name: "Owner".into(),
            })
            .unwrap();
        Self {
            path,
            admin,
            store,
            owner,
        }
    }
    fn join(&self, role: Role) -> JoinedRoom {
        let invite = Secret::generate().unwrap();
        self.store
            .invite(&self.owner.credentials, invite.clone(), role, 60, 1)
            .unwrap();
        self.store
            .join(JoinRequest {
                room: self.owner.state.room.clone(),
                invite,
                member_token: Secret::generate().unwrap(),
                display_name: "Peer".into(),
            })
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
fn replica(joined: &JoinedRoom) -> DocumentReplica {
    DocumentReplica::from_snapshot(
        joined.credentials.actor.clone(),
        joined.state.snapshot.clone(),
    )
    .unwrap()
}
fn exchange(
    store: &RoomStore,
    joined: &JoinedRoom,
    replica: &DocumentReplica,
) -> Result<ExchangeReply> {
    store.exchange(
        &joined.credentials,
        Exchange {
            epoch: joined.state.epoch,
            known: VersionVector::new(),
            message: replica.message(
                replica
                    .operations()
                    .filter(|op| op.id.actor == joined.credentials.actor)
                    .cloned()
                    .collect(),
            ),
            presence: None,
        },
    )
}

#[test]
fn server_authenticates_writers_and_rejects_forged_actor_atomically() {
    let fixture = Fixture::new();
    let editor = fixture.join(Role::Editor);
    let mut forged = replica(&fixture.owner);
    forged.set_attribute(forged.root(), "Width", "42").unwrap();
    let result = fixture.store.exchange(
        &editor.credentials,
        Exchange {
            epoch: 1,
            known: VersionVector::new(),
            message: forged.message(forged.operations().cloned().collect()),
            presence: None,
        },
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::Forbidden);
    assert!(fixture
        .store
        .state(&editor.credentials)
        .unwrap()
        .snapshot
        .operations
        .is_empty());
}
#[test]
fn viewers_receive_changes_but_cannot_publish_or_invite() {
    let fixture = Fixture::new();
    let viewer = fixture.join(Role::Viewer);
    let mut owner = replica(&fixture.owner);
    owner.set_attribute(owner.root(), "Width", "100").unwrap();
    exchange(&fixture.store, &fixture.owner, &owner).unwrap();
    let mut local = replica(&viewer);
    let response = exchange(&fixture.store, &viewer, &local).unwrap();
    local.merge(response.message).unwrap();
    assert!(local.source().contains("100"));
    local.set_attribute(local.root(), "Width", "10").unwrap();
    assert_eq!(
        exchange(&fixture.store, &viewer, &local).unwrap_err().code,
        ErrorCode::Forbidden
    );
    assert!(fixture
        .store
        .invite(
            &viewer.credentials,
            Secret::generate().unwrap(),
            Role::Editor,
            60,
            1
        )
        .is_err());
}
#[test]
fn lost_join_response_is_idempotent_and_invitation_exhaustion_is_enforced() {
    let fixture = Fixture::new();
    let token = Secret::generate().unwrap();
    fixture
        .store
        .invite(
            &fixture.owner.credentials,
            token.clone(),
            Role::Editor,
            60,
            1,
        )
        .unwrap();
    let request = JoinRequest {
        room: fixture.owner.state.room.clone(),
        invite: token,
        member_token: Secret::generate().unwrap(),
        display_name: "Peer".into(),
    };
    let first = fixture.store.join(request.clone()).unwrap();
    let retry = fixture.store.join(request.clone()).unwrap();
    assert_eq!(first.credentials.actor, retry.credentials.actor);
    assert_eq!(first.state.members.len(), 2);
    let mut other = request;
    other.member_token = Secret::generate().unwrap();
    assert_eq!(
        fixture.store.join(other).unwrap_err().code,
        ErrorCode::Unauthorized
    );
}
#[test]
fn durable_acknowledgements_survive_restart_and_duplicate_exchange() {
    let fixture = Fixture::new();
    let mut local = replica(&fixture.owner);
    local.set_attribute(local.root(), "Height", "200").unwrap();
    exchange(&fixture.store, &fixture.owner, &local).unwrap();
    exchange(&fixture.store, &fixture.owner, &local).unwrap();
    let path = fixture.path.clone();
    let admin = fixture.admin.clone();
    let credentials = fixture.owner.credentials.clone();
    let bytes = std::fs::read_dir(&path)
        .unwrap()
        .filter_map(|x| {
            let p = x.unwrap().path();
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("room-")
                .then(|| std::fs::read(&p).unwrap())
        })
        .next()
        .unwrap();
    assert!(!String::from_utf8(bytes)
        .unwrap()
        .contains(credentials.token.expose()));
    // Copy a coherent durable checkpoint into a separate store to simulate process restart.
    let copy = path.with_extension("restart");
    std::fs::create_dir(&copy).unwrap();
    for entry in std::fs::read_dir(&path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != "server.lock" {
            std::fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
        }
    }
    let restarted = RoomStore::open(&copy, &admin).unwrap();
    let state = restarted.state(&credentials).unwrap();
    assert_eq!(state.snapshot.operations.len(), 1);
    assert!(DocumentReplica::from_snapshot("reader", state.snapshot)
        .unwrap()
        .source()
        .contains("200"));
    drop(restarted);
    std::fs::remove_dir_all(copy).unwrap();
}

#[test]
fn multi_batch_room_log_can_restart_and_download_in_bounded_pages() {
    use ferroui_designer_collaboration::{Operation, OperationId, SharedNodeId, MAX_MESSAGE_BYTES};
    let fixture = Fixture::new();
    let mut snapshot = fixture.owner.state.snapshot.clone();
    let payload = "x".repeat(4 * 1024 * 1024);
    for sequence in 1..=9 {
        snapshot.operations.push(Operation {
            id: OperationId {
                actor: fixture.owner.credentials.actor.clone(),
                lamport: sequence,
                sequence,
            },
            document_id: snapshot.document_id.clone(),
            label: "Durable conflict".into(),
            edits: vec![SharedEdit::SetAttribute {
                node: SharedNodeId::Baseline(999_999),
                name: "Text".into(),
                value: payload.clone(),
            }],
        });
    }
    let local =
        DocumentReplica::from_snapshot(fixture.owner.credentials.actor.clone(), snapshot).unwrap();
    let mut acknowledged = VersionVector::new();
    let mut uploads = 0;
    while acknowledged != local.version() {
        let page = local
            .changes_since_bounded(&acknowledged, Some(local.actor()), MAX_MESSAGE_BYTES)
            .unwrap();
        let reply = fixture
            .store
            .exchange(
                &fixture.owner.credentials,
                Exchange {
                    epoch: 1,
                    known: local.version(),
                    message: page,
                    presence: None,
                },
            )
            .unwrap();
        acknowledged = reply.version;
        uploads += 1;
        assert!(uploads <= 2);
    }
    assert_eq!(uploads, 2);
    let copy = fixture.path.with_extension("large-restart");
    std::fs::create_dir(&copy).unwrap();
    for entry in std::fs::read_dir(&fixture.path).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != "server.lock" {
            std::fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
        }
    }
    let restarted = RoomStore::open(&copy, &fixture.admin).unwrap();
    let mut receiver = DocumentReplica::new(
        "receiver",
        local.document_id(),
        &fixture.owner.state.snapshot.baseline,
    )
    .unwrap();
    for _ in 0..2 {
        let reply = restarted
            .exchange(
                &fixture.owner.credentials,
                Exchange {
                    epoch: 1,
                    known: receiver.version(),
                    message: receiver.message(Vec::new()),
                    presence: None,
                },
            )
            .unwrap();
        assert!(reply.message.to_json().unwrap().len() <= MAX_MESSAGE_BYTES);
        assert!(receiver.merge(reply.message).unwrap().received > 0);
    }
    assert_eq!(receiver.version(), acknowledged);
    assert_eq!(receiver.conflicts(), local.conflicts());
    drop(restarted);
    std::fs::remove_dir_all(copy).unwrap();
}
#[test]
fn cancelled_commit_changes_neither_memory_nor_disk() {
    let fixture = Fixture::new();
    let mut local = replica(&fixture.owner);
    local.set_attribute(local.root(), "Width", "42").unwrap();
    let message = local.message(local.operations().cloned().collect());
    let result = fixture.store.exchange_checked(
        &fixture.owner.credentials,
        Exchange {
            epoch: 1,
            known: VersionVector::new(),
            message,
            presence: None,
        },
        &|| Err(HostError::new(ErrorCode::Cancelled, "Cancelled")),
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::Cancelled);
    assert!(fixture
        .store
        .state(&fixture.owner.credentials)
        .unwrap()
        .snapshot
        .operations
        .is_empty());
}
#[test]
fn concurrent_peers_merge_without_lost_acknowledged_updates() {
    let fixture = Fixture::new();
    let a = fixture.join(Role::Editor);
    let b = fixture.join(Role::Editor);
    let mut one = replica(&a);
    let mut two = replica(&b);
    one.set_attribute(one.root(), "Width", "42").unwrap();
    two.set_attribute(two.root(), "Height", "84").unwrap();
    let store = fixture.store.clone();
    let t1 = std::thread::spawn(move || exchange(&store, &a, &one).unwrap());
    let store = fixture.store.clone();
    let t2 = std::thread::spawn(move || exchange(&store, &b, &two).unwrap());
    t1.join().unwrap();
    t2.join().unwrap();
    let state = fixture.store.state(&fixture.owner.credentials).unwrap();
    let replica = DocumentReplica::from_snapshot("reader", state.snapshot).unwrap();
    assert!(
        replica.source().contains("Width=\"42\"") && replica.source().contains("Height=\"84\"")
    );
}
#[test]
fn revocation_is_immediate_and_owner_cannot_be_removed() {
    let fixture = Fixture::new();
    let editor = fixture.join(Role::Editor);
    fixture
        .store
        .set_member(
            &fixture.owner.credentials,
            &editor.credentials.actor,
            Role::Viewer,
            true,
        )
        .unwrap();
    assert_eq!(
        fixture.store.state(&editor.credentials).unwrap_err().code,
        ErrorCode::Unauthorized
    );
    assert!(fixture
        .store
        .set_member(
            &fixture.owner.credentials,
            &fixture.owner.credentials.actor,
            Role::Viewer,
            true
        )
        .is_err());
}
#[test]
fn checkpoint_requires_exact_version_and_fences_old_offline_peers() {
    let fixture = Fixture::new();
    let editor = fixture.join(Role::Editor);
    let mut local = replica(&fixture.owner);
    local.set_attribute(local.root(), "Width", "42").unwrap();
    exchange(&fixture.store, &fixture.owner, &local).unwrap();
    assert!(fixture
        .store
        .checkpoint(&fixture.owner.credentials, 1, &VersionVector::new())
        .is_err());
    let state = fixture
        .store
        .checkpoint(&fixture.owner.credentials, 1, &local.version())
        .unwrap();
    assert_eq!(state.epoch, 2);
    assert!(state.snapshot.operations.is_empty());
    let mut offline = replica(&editor);
    offline
        .set_attribute(offline.root(), "Height", "55")
        .unwrap();
    assert_eq!(
        exchange(&fixture.store, &editor, &offline)
            .unwrap_err()
            .code,
        ErrorCode::EpochChanged
    );
    assert!(offline.source().contains("55"));
}

#[test]
fn checkpoint_cannot_discard_unseen_operations_beyond_a_sequence_hole() {
    let fixture = Fixture::new();
    let mut local = replica(&fixture.owner);
    let first = local.set_attribute(local.root(), "Width", "10").unwrap();
    let second = local.set_attribute(local.root(), "Width", "20").unwrap();
    let third = local.set_attribute(local.root(), "Width", "30").unwrap();
    let send = |operations| {
        fixture
            .store
            .exchange(
                &fixture.owner.credentials,
                Exchange {
                    epoch: 1,
                    known: VersionVector::new(),
                    message: local.message(operations),
                    presence: None,
                },
            )
            .unwrap()
    };
    let observed = send(vec![first]);
    let with_hole = send(vec![third]);
    assert_eq!(
        observed.version, with_hole.version,
        "the contiguous vector cannot reveal sequence 3"
    );
    assert_eq!(
        fixture
            .store
            .checkpoint(&fixture.owner.credentials, 1, &observed.version)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        fixture
            .store
            .checkpoint_resolved(&fixture.owner.credentials, 1, &observed.version, "<Panel/>")
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let complete = send(vec![second]);
    assert_eq!(complete.version[&fixture.owner.credentials.actor], 3);
    let state = fixture
        .store
        .checkpoint(&fixture.owner.credentials, 1, &complete.version)
        .unwrap();
    assert_eq!(state.epoch, 2);
    assert!(state.snapshot.baseline.contains("30"));
}

#[test]
fn owner_can_explicitly_resolve_historical_text_conflicts_at_an_exact_version() {
    let fixture = Fixture::new();
    let editor = fixture.join(Role::Editor);
    let mut a = replica(&fixture.owner);
    let mut b = replica(&editor);
    let baseline = a.source().to_owned();
    a.submit(
        "First source",
        vec![ferroui_designer_collaboration::SharedEdit::ReplaceSource {
            expected_source: baseline.clone(),
            source: "<Panel Width=\"10\"/>".into(),
        }],
    )
    .unwrap();
    b.submit(
        "Second source",
        vec![ferroui_designer_collaboration::SharedEdit::ReplaceSource {
            expected_source: baseline,
            source: "<Panel Height=\"20\"/>".into(),
        }],
    )
    .unwrap();
    exchange(&fixture.store, &fixture.owner, &a).unwrap();
    let reply = exchange(&fixture.store, &editor, &b).unwrap();
    assert_eq!(reply.conflicts.len(), 1);
    assert!(fixture
        .store
        .checkpoint(&fixture.owner.credentials, 1, &reply.version)
        .is_err());
    assert!(fixture
        .store
        .checkpoint_resolved(
            &fixture.owner.credentials,
            1,
            &VersionVector::new(),
            "<Panel/>"
        )
        .is_err());
    let state = fixture
        .store
        .checkpoint_resolved(
            &fixture.owner.credentials,
            1,
            &reply.version,
            "<Panel Width=\"10\" Height=\"20\"/>",
        )
        .unwrap();
    assert_eq!(state.epoch, 2);
    assert!(state.snapshot.operations.is_empty());
    assert!(
        state.snapshot.baseline.contains("Width") && state.snapshot.baseline.contains("Height")
    );
}
#[test]
fn storage_lock_and_checksum_protect_room_integrity() {
    let fixture = Fixture::new();
    assert!(RoomStore::open(&fixture.path, &fixture.admin).is_err());
    let path = fixture.path.join("checkpoint.json");
    save_private(&path, &fixture.owner.state).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let offset = bytes.iter().position(|b| *b == b'a').unwrap();
    bytes[offset] = b'b';
    std::fs::write(&path, bytes).unwrap();
    assert!(load_private::<RoomState>(&path).is_err());
}
#[test]
fn credentials_are_redacted_and_plaintext_never_enters_room_state() {
    let fixture = Fixture::new();
    let secret = fixture.owner.credentials.token.expose();
    assert!(!format!("{:?}", fixture.owner.credentials).contains(secret));
    let json = serde_json::to_string(&fixture.owner.state).unwrap();
    assert!(!json.contains(secret) && !json.contains("token_hash"));
    let wrong = Secret::generate().unwrap();
    assert_eq!(
        fixture.store.authorize_admin(&wrong).unwrap_err().code,
        ErrorCode::Unauthorized
    );
}

#[test]
fn retrying_redeemed_invite_returns_current_role_after_owner_downgrade() {
    let fixture = Fixture::new();
    let invite = Secret::generate().unwrap();
    fixture
        .store
        .invite(
            &fixture.owner.credentials,
            invite.clone(),
            Role::Editor,
            60,
            1,
        )
        .unwrap();
    let request = JoinRequest {
        room: fixture.owner.state.room.clone(),
        invite,
        member_token: Secret::generate().unwrap(),
        display_name: "Peer".into(),
    };
    let joined = fixture.store.join(request.clone()).unwrap();
    fixture
        .store
        .set_member(
            &fixture.owner.credentials,
            &joined.credentials.actor,
            Role::Viewer,
            false,
        )
        .unwrap();
    assert_eq!(
        fixture.store.join(request).unwrap().state.role,
        Role::Viewer
    );
}
#[test]
fn indeterminate_post_rename_durability_blocks_stale_memory_writes() {
    let fixture = Fixture::new();
    let mut local = replica(&fixture.owner);
    local.set_attribute(local.root(), "Width", "99").unwrap();
    crate::storage::fail_next_directory_sync(fixture.path.join(format!(
        "room-{}.json",
        crate::model::hash(fixture.owner.credentials.room.as_bytes())
    )));
    assert_eq!(
        exchange(&fixture.store, &fixture.owner, &local)
            .unwrap_err()
            .code,
        ErrorCode::Indeterminate
    );
    assert_eq!(
        fixture
            .store
            .state(&fixture.owner.credentials)
            .unwrap_err()
            .code,
        ErrorCode::Indeterminate
    );
    assert_eq!(
        fixture
            .store
            .invite(
                &fixture.owner.credentials,
                Secret::generate().unwrap(),
                Role::Editor,
                60,
                1
            )
            .unwrap_err()
            .code,
        ErrorCode::Indeterminate
    );
    let saved = std::fs::read_dir(&fixture.path)
        .unwrap()
        .find_map(|entry| {
            let entry = entry.unwrap();
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("room-")
                .then(|| crate::storage::read::<crate::model::StoredRoom>(&entry.path()).unwrap())
        })
        .unwrap();
    assert_eq!(saved.snapshot.operations.len(), 1);
    assert!(DocumentReplica::from_snapshot("reader", saved.snapshot)
        .unwrap()
        .source()
        .contains("99"));
}

#[cfg(feature = "client")]
mod peer_tests {
    use super::*;
    use serde_json::Value;
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Transport {
        store: Arc<RoomStore>,
        lose_ack: Arc<AtomicBool>,
    }
    impl PeerTransport for Transport {
        fn request(&mut self, request: Request) -> Result<Value> {
            let Request::Exchange {
                credentials,
                exchange,
            } = request
            else {
                panic!("exchange expected")
            };
            let result = self.store.exchange(&credentials, exchange)?;
            if self.lose_ack.swap(false, Ordering::SeqCst) {
                Err(HostError::new(
                    ErrorCode::Transport,
                    "connection lost after durable commit",
                ))
            } else {
                Ok(serde_json::to_value(result).unwrap())
            }
        }
    }
    #[test]
    fn reconnect_resends_lost_ack_and_restores_persisted_offline_queue() {
        let fixture = Fixture::new();
        let joined = fixture.join(Role::Editor);
        let loss = Arc::new(AtomicBool::new(true));
        let mut peer = ReconnectingPeer::with_transport(
            Endpoint {
                url: "http://127.0.0.1:1/mcp".into(),
                bearer: Secret::generate().unwrap(),
            },
            joined,
            Box::new(Transport {
                store: fixture.store.clone(),
                lose_ack: loss,
            }),
        )
        .unwrap();
        let path = fixture.path.join("peer.json");
        peer.set_checkpoint_path(&path).unwrap();
        peer.submit(
            "Offline change",
            vec![SharedEdit::SetAttribute {
                node: peer.document().root(),
                name: "Width".into(),
                value: "77".into(),
            }],
        )
        .unwrap();
        assert_eq!(peer.pending_operations(), 1);
        assert!(peer.sync(None).is_err());
        let restored = ReconnectingPeer::restore(&path).unwrap();
        assert_eq!(restored.pending_operations(), 1);
        peer.sync(None).unwrap();
        assert_eq!(peer.pending_operations(), 0);
        assert_eq!(
            fixture
                .store
                .state(&fixture.owner.credentials)
                .unwrap()
                .snapshot
                .operations
                .len(),
            1
        );
    }
    #[test]
    fn indeterminate_outbox_commit_fences_the_peer_until_restored() {
        let fixture = Fixture::new();
        let joined = fixture.join(Role::Editor);
        let mut peer = ReconnectingPeer::with_transport(
            Endpoint {
                url: "http://127.0.0.1:1/mcp".into(),
                bearer: Secret::generate().unwrap(),
            },
            joined,
            Box::new(Transport {
                store: fixture.store.clone(),
                lose_ack: Arc::new(AtomicBool::new(false)),
            }),
        )
        .unwrap();
        let path = fixture.path.join("peer.json");
        peer.set_checkpoint_path(&path).unwrap();
        let edit = SharedEdit::SetAttribute {
            node: peer.document().root(),
            name: "Width".into(),
            value: "99".into(),
        };
        crate::storage::fail_next_directory_sync(path.clone());
        assert_eq!(
            peer.submit("Uncertain durability", vec![edit.clone()])
                .unwrap_err()
                .code,
            ErrorCode::Indeterminate
        );
        let replaced = std::fs::read(&path).unwrap();
        assert_eq!(peer.sync(None).unwrap_err().code, ErrorCode::Indeterminate);
        assert_eq!(
            peer.submit("Must not overwrite", vec![edit])
                .unwrap_err()
                .code,
            ErrorCode::Indeterminate
        );
        assert_eq!(
            peer.set_checkpoint_path(fixture.path.join("other.json"))
                .unwrap_err()
                .code,
            ErrorCode::Indeterminate
        );
        assert_eq!(std::fs::read(&path).unwrap(), replaced);
        let restored = ReconnectingPeer::restore(&path).unwrap();
        assert!(restored.document().source().contains("99"));
        assert_eq!(restored.pending_operations(), 1);
    }
    #[test]
    fn stale_epoch_rebase_keeps_offline_changes_until_explicit_resolution() {
        let fixture = Fixture::new();
        let joined = fixture.join(Role::Editor);
        let credentials = joined.credentials.clone();
        let mut peer = ReconnectingPeer::with_transport(
            Endpoint {
                url: "http://127.0.0.1:1/mcp".into(),
                bearer: Secret::generate().unwrap(),
            },
            joined,
            Box::new(Transport {
                store: fixture.store.clone(),
                lose_ack: Arc::new(AtomicBool::new(false)),
            }),
        )
        .unwrap();
        peer.submit(
            "Offline",
            vec![SharedEdit::SetAttribute {
                node: peer.document().root(),
                name: "Width".into(),
                value: "9".into(),
            }],
        )
        .unwrap();
        fixture
            .store
            .checkpoint(&fixture.owner.credentials, 1, &VersionVector::new())
            .unwrap();
        assert_eq!(peer.sync(None).unwrap_err().code, ErrorCode::EpochChanged);
        assert_eq!(peer.pending_operations(), 1);
        let state = fixture.store.state(&credentials).unwrap();
        let remote = state.snapshot.baseline.clone();
        let resolved = peer.document().source().to_owned();
        assert!(peer.rebase(state.clone(), "stale", &resolved).is_err());
        peer.rebase(state, &remote, &resolved).unwrap();
        peer.sync(None).unwrap();
        assert_eq!(peer.pending_operations(), 0);
        assert!(peer.document().source().contains("Width=\"9\""));
    }
    #[test]
    fn network_client_rejects_cleartext_remote_urls_and_credentials_in_urls() {
        for url in [
            "http://example.com/mcp",
            "https://user:secret@example.com/mcp",
            "https://example.com/mcp?token=x",
        ] {
            assert!(McpPeer::new(Endpoint {
                url: url.into(),
                bearer: Secret::generate().unwrap()
            })
            .is_err());
        }
    }
}

#[cfg(all(feature = "client", feature = "server"))]
mod network_tests {
    use super::*;
    #[test]
    fn actual_mcp_transport_hosts_two_peers_and_large_unicode_documents() {
        let path = std::env::temp_dir().join(format!(
            "ferroui-network-room-{}",
            Secret::generate().unwrap().0
        ));
        let admin = Secret::generate().unwrap();
        let host = HostedServer::start(&path, &admin, None).unwrap();
        let endpoint = host.endpoint();
        let mut owner_transport = McpPeer::new(endpoint.clone()).unwrap();
        let mut source = "<Panel><!--".to_owned();
        source.push_str(&"界".repeat(180_000));
        source.push_str("--><Button Name=\"a\"/></Panel>");
        let owner: JoinedRoom = serde_json::from_value(
            owner_transport
                .request(Request::Create {
                    request: CreateRequest {
                        admin: admin.clone(),
                        owner_token: Secret::generate().unwrap(),
                        title: "Shared large document".into(),
                        document_id: "large.xaml".into(),
                        source,
                        display_name: "Owner".into(),
                    },
                })
                .unwrap(),
        )
        .unwrap();
        let invite = Secret::generate().unwrap();
        owner_transport
            .request(Request::Invite {
                credentials: owner.credentials.clone(),
                token: invite.clone(),
                role: Role::Editor,
                expires_in: 60,
                uses: 1,
            })
            .unwrap();
        let mut guest_transport = McpPeer::new(endpoint.clone()).unwrap();
        let guest: JoinedRoom = serde_json::from_value(
            guest_transport
                .request(Request::Join {
                    request: JoinRequest {
                        room: owner.state.room.clone(),
                        invite,
                        member_token: Secret::generate().unwrap(),
                        display_name: "Peer".into(),
                    },
                })
                .unwrap(),
        )
        .unwrap();
        let mut one = ReconnectingPeer::new(endpoint.clone(), owner.clone()).unwrap();
        let mut two = ReconnectingPeer::new(endpoint, guest).unwrap();
        one.submit(
            "Owner width",
            vec![SharedEdit::SetAttribute {
                node: one.document().root(),
                name: "Width".into(),
                value: "800".into(),
            }],
        )
        .unwrap();
        two.submit(
            "Peer height",
            vec![SharedEdit::SetAttribute {
                node: two.document().root(),
                name: "Height".into(),
                value: "600".into(),
            }],
        )
        .unwrap();
        one.sync(None).unwrap();
        two.sync(None).unwrap();
        one.sync(None).unwrap();
        assert_eq!(one.document().source(), two.document().source());
        assert!(one.document().source().contains("800") && one.document().source().contains("600"));
        // Exercise pagination over the real MCP transport with a deliberately small wire page.
        // The final reply must acknowledge only received writes; a lost/partial outbox cannot
        // be mistaken for the complete local operation set.
        let mut paged = DocumentReplica::from_snapshot(
            owner.credentials.actor.clone(),
            one.document().snapshot(),
        )
        .unwrap();
        let mut known = one.document().version();
        for index in 0..6 {
            paged
                .set_attribute(
                    paged.root(),
                    "Tag",
                    format!("{index}: {}", "\"界\\".repeat(20)),
                )
                .unwrap();
        }
        let mut pages = 0;
        while paged.pending_operation_count(&known, paged.actor()) > 0 {
            let message = paged
                .changes_since_bounded(&known, Some(paged.actor()), 1200)
                .unwrap();
            assert!(message.to_json().unwrap().len() <= 1200);
            let reply: ExchangeReply = serde_json::from_value(
                owner_transport
                    .request(Request::Exchange {
                        credentials: owner.credentials.clone(),
                        exchange: Exchange {
                            epoch: owner.state.epoch,
                            known: paged.version(),
                            message,
                            presence: None,
                        },
                    })
                    .unwrap(),
            )
            .unwrap();
            assert!(reply.version[&owner.credentials.actor] > known[&owner.credentials.actor]);
            known = reply.version;
            pages += 1;
            assert!(pages <= 6);
        }
        assert!(pages > 1);
        two.sync(None).unwrap();
        assert_eq!(two.document().source(), paged.source());
        let old_endpoint = host.endpoint();
        drop(host);
        std::thread::sleep(std::time::Duration::from_millis(100));
        let restarted = HostedServer::start(&path, &admin, None).unwrap();
        assert_eq!(old_endpoint.bearer, restarted.endpoint().bearer);
        assert_eq!(
            restarted
                .store
                .state(&owner.credentials)
                .unwrap()
                .snapshot
                .operations
                .len(),
            8
        );
        drop(restarted);
        let _ = std::fs::remove_dir_all(path);
    }
}
