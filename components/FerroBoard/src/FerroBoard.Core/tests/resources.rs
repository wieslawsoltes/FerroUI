use ferroboard_core::resources::EmbeddedResources;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
#[test]
fn unchanged_assembly_resource_manifest_and_every_decoded_payload() {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("../../../tests/fixtures/resources.json")).unwrap();
    assert_eq!(
        EmbeddedResources::paths().collect::<Vec<_>>(),
        rows.iter()
            .map(|r| r["path"].as_str().unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(rows.len(), 206);
    for row in rows {
        let path = row["path"].as_str().unwrap();
        assert_eq!(EmbeddedResources::logical_name(path), row["name"].as_str());
        let mut bytes = Vec::new();
        EmbeddedResources::open(path)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(sha(&bytes), row["decodedSha"].as_str().unwrap(), "{path}");
        assert_eq!(
            bytes.len() as u64,
            row["decodedBytes"].as_u64().unwrap(),
            "{path}"
        );
        let text = EmbeddedResources::read_text(path)
            .unwrap()
            .unwrap()
            .to_utf8()
            .unwrap();
        assert_eq!(
            sha(text.as_bytes()),
            row["textSha"].as_str().unwrap(),
            "{path}"
        );
        let alias = format!("///{}", path.replace('/', "\\"));
        assert!(EmbeddedResources::exists(&alias));
        if let Some(short) = path.strip_suffix(".gz") {
            assert_eq!(
                EmbeddedResources::read_text(short).unwrap(),
                EmbeddedResources::read_text(path).unwrap()
            );
        }
    }
    assert!(!EmbeddedResources::exists("no-resource"));
    assert!(EmbeddedResources::open("no-resource").is_none());
    assert_eq!(EmbeddedResources::read_text("no-resource").unwrap(), None);
}
#[test]
fn resource_index_is_shared_across_threads_and_stream_positions_are_independent() {
    let path = "keyboard/org.florisboard.layouts/extension.json";
    let expected = EmbeddedResources::read_text(path).unwrap().unwrap();
    let threads: Vec<_> = (0..8)
        .map(|_| std::thread::spawn(move || EmbeddedResources::read_text(path).unwrap().unwrap()))
        .collect();
    for thread in threads {
        assert_eq!(thread.join().unwrap(), expected);
    }
    let mut a = EmbeddedResources::open(path).unwrap();
    let mut b = EmbeddedResources::open(path).unwrap();
    let mut first = [0; 12];
    let mut second = [0; 12];
    a.read_exact(&mut first).unwrap();
    a.read_exact(&mut second).unwrap();
    b.read_exact(&mut second).unwrap();
    assert_eq!(first, second);
    assert!(!EmbeddedResources::exists(
        "Keyboard/org.florisboard.layouts/extension.json"
    ));
}
