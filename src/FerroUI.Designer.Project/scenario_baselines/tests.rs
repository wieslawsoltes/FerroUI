use super::*;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferroui-baselines-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(&self.0)
            .args([
                "-c",
                "user.name=FerroUI Test",
                "-c",
                "user.email=tests@example.invalid",
                "-c",
                "commit.gpgSign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn compares_at_the_fork_without_switching_the_checkout() {
    let fixture = Fixture::new();
    let root = &fixture.0;
    fixture.git(&["init", "-q", "--initial-branch=main"]);
    let baseline = root.join("base image.png");
    fs::write(&baseline, b"shared ancestor").unwrap();
    fixture.git(&["add", "--", "base image.png"]);
    fixture.git(&["commit", "-qm", "first approved image"]);
    fixture.git(&["branch", "work"]);
    fs::write(&baseline, b"trunk moved on").unwrap();
    fs::write(root.join("new.png"), b"trunk only").unwrap();
    fixture.git(&["add", "--", "base image.png", "new.png"]);
    fixture.git(&["commit", "-qm", "new trunk images"]);
    fixture.git(&["switch", "-q", "work"]);
    fs::write(&baseline, b"feature working file").unwrap();
    let repo = Repository::open(root, Some("main")).unwrap().unwrap();
    assert_eq!(
        repo.baseline(&baseline).unwrap().as_deref(),
        Some(b"shared ancestor".as_slice())
    );
    assert_eq!(repo.baseline(&root.join("new.png")).unwrap(), None);
    assert_eq!(fs::read(&baseline).unwrap(), b"feature working file");
    assert!(Repository::open(root, Some("--help")).is_err());
}

#[test]
fn serializes_approval_and_rejects_a_stale_or_escaping_destination() {
    let fixture = Fixture::new();
    let root = &fixture.0;
    let path = &destination(root, root, "expected/render.png").unwrap();
    publish(root, path, None, b"original", || Ok(())).unwrap();
    let results = std::thread::scope(|threads| {
        let first = threads.spawn(|| publish(root, path, Some(b"original"), b"one", || Ok(())));
        let second = threads.spawn(|| publish(root, path, Some(b"original"), b"two", || Ok(())));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(matches!(
        read(path).unwrap().as_deref(),
        Some(b"one" | b"two")
    ));
    assert!(destination(root, root, "../escaped.png").is_err());
    assert!(publish(root, path, None, b"unreviewed", || Ok(())).is_err());
}
