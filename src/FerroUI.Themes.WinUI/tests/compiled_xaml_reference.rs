//! Storage of the checked-in reference only; the production compiler still emits one file.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

// Keep every published part strictly below 4 MiB. The final line may have no newline.
const MAX_PART_BYTES: usize = 4 * 1024 * 1024 - 1;
const MANIFEST: &str = "parts.txt";

fn part_name(index: usize) -> String {
    format!("part-{index:03}.rs.part")
}

fn split_source(source: &str, limit: usize) -> Result<Vec<&str>, String> {
    if source.is_empty() || limit == 0 {
        return Err("the reference and part size limit must be nonempty".into());
    }
    let mut parts = Vec::new();
    let mut start = 0;
    let mut length = 0;
    for line in source.split_inclusive('\n') {
        if line.len() > limit {
            return Err(format!(
                "a reference line exceeds the {limit}-byte part limit"
            ));
        }
        if length + line.len() > limit {
            parts.push(&source[start..start + length]);
            start += length;
            length = 0;
        }
        length += line.len();
    }
    parts.push(&source[start..]);
    Ok(parts)
}

fn parse_manifest(text: &str) -> Result<Vec<String>, String> {
    if text.is_empty() || !text.ends_with('\n') {
        return Err(
            "parts.txt must contain a nonempty, newline-terminated ordered inventory".into(),
        );
    }
    let mut names = Vec::new();
    for (index, name) in text.split_terminator('\n').enumerate() {
        let expected = part_name(index);
        if name != expected {
            return Err(format!(
                "parts.txt entry {index} must be {expected}, found {name:?}"
            ));
        }
        names.push(name.to_owned());
    }
    Ok(names)
}

fn read_inventory(directory: &Path) -> Result<Vec<String>, String> {
    let kind = fs::symlink_metadata(directory)
        .map_err(|error| error.to_string())?
        .file_type();
    if !kind.is_dir() {
        return Err(format!(
            "{} is not a regular reference directory",
            directory.display()
        ));
    }
    let manifest = fs::read_to_string(directory.join(MANIFEST))
        .map_err(|error| format!("parts.txt: {error}"))?;
    let names = parse_manifest(&manifest)?;
    let mut remaining: BTreeSet<_> = names.iter().cloned().collect();
    remaining.insert(MANIFEST.to_owned());
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "reference entry has a non-UTF-8 name")?;
        if !remaining.remove(&name) {
            return Err(format!(
                "unexpected reference entry {name:?}; only the manifest inventory is allowed"
            ));
        }
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            return Err(format!("reference entry {name:?} is not a regular file"));
        }
    }
    if !remaining.is_empty() {
        return Err(format!("reference inventory is missing: {remaining:?}"));
    }
    Ok(names)
}

fn read_with_limit(directory: &Path, limit: usize) -> Result<String, String> {
    let names = read_inventory(directory)?;
    let mut source = String::new();
    let mut lengths = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let part =
            fs::read_to_string(directory.join(name)).map_err(|error| format!("{name}: {error}"))?;
        if part.is_empty() || part.len() > limit {
            return Err(format!("{name} must contain between 1 and {limit} bytes"));
        }
        if index + 1 < names.len() && !part.ends_with('\n') {
            return Err(format!("{name} does not end at a newline boundary"));
        }
        lengths.push(part.len());
        source.push_str(&part);
    }
    let canonical: Vec<_> = split_source(&source, limit)?
        .iter()
        .map(|part| part.len())
        .collect();
    if lengths != canonical {
        return Err("reference parts do not use the deterministic whole-line partition".into());
    }
    Ok(source)
}

pub(super) fn read(directory: &Path) -> Result<String, String> {
    read_with_limit(directory, MAX_PART_BYTES)
}

fn write_with_limit(directory: &Path, source: &str, limit: usize) -> Result<(), String> {
    // Validate all source boundaries and the old inventory before overwriting anything.
    let parts = split_source(source, limit)?;
    if !directory.exists() {
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    }
    let kind = fs::symlink_metadata(directory)
        .map_err(|error| error.to_string())?
        .file_type();
    if !kind.is_dir() {
        return Err(format!(
            "{} is not a regular reference directory",
            directory.display()
        ));
    }
    let previous = if fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .next()
        .is_none()
    {
        Vec::new()
    } else {
        read_inventory(directory)?
    };
    let names: Vec<_> = (0..parts.len()).map(part_name).collect();
    for (name, part) in names.iter().zip(parts) {
        fs::write(directory.join(name), part).map_err(|error| format!("{name}: {error}"))?;
    }
    // Delete only files named by the previously validated inventory, never a glob.
    for name in previous.iter().skip(names.len()) {
        fs::remove_file(directory.join(name)).map_err(|error| format!("{name}: {error}"))?;
    }
    let manifest = names
        .iter()
        .map(|name| format!("{name}\n"))
        .collect::<String>();
    fs::write(directory.join(MANIFEST), manifest).map_err(|error| format!("parts.txt: {error}"))?;
    if read_with_limit(directory, limit)? != source {
        return Err("written reference does not match the generated source bytes".into());
    }
    Ok(())
}

pub(super) fn write(directory: &Path, source: &str) -> Result<(), String> {
    write_with_limit(directory, source, MAX_PART_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "ferroui-winui-reference-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn newline_chunks_round_trip_utf8_crlf_and_unterminated_last_line() {
        let directory = Scratch::new();
        let source = "ab\r\nλμ\nlast";
        assert_eq!(split_source(source, 6).unwrap(), ["ab\r\n", "λμ\n", "last"]);
        write_with_limit(&directory.0, source, 6).unwrap();
        assert_eq!(
            read_with_limit(&directory.0, 6).unwrap().as_bytes(),
            source.as_bytes()
        );
        assert_eq!(read_inventory(&directory.0).unwrap().len(), 3);

        write_with_limit(&directory.0, "tail", 6).unwrap();
        assert_eq!(read_with_limit(&directory.0, 6).unwrap(), "tail");
        assert_eq!(read_inventory(&directory.0).unwrap(), [part_name(0)]);
        assert!(!directory.0.join(part_name(1)).exists());
        assert!(!directory.0.join(part_name(2)).exists());
    }

    #[test]
    fn chunk_limits_count_bytes_and_reject_oversized_lines() {
        assert_eq!(split_source("é\né\nx", 6).unwrap(), ["é\né\n", "x"]);
        assert!(split_source("abcdef\n", 6).is_err());
        assert!(split_source("", 6).is_err());
        assert!(split_source("a", 0).is_err());

        let directory = Scratch::new();
        write_with_limit(&directory.0, "a\nb\nc\n", 4).unwrap();
        // The same concatenated bytes with a noncanonical boundary must fail too.
        fs::write(directory.0.join(part_name(0)), "a\n").unwrap();
        fs::write(directory.0.join(part_name(1)), "b\nc\n").unwrap();
        assert!(read_with_limit(&directory.0, 4)
            .unwrap_err()
            .contains("deterministic"));
    }

    #[test]
    fn inventory_rejects_missing_reordered_duplicate_and_malformed_entries() {
        for manifest in [
            "",
            "part-000.rs.part",
            "part-001.rs.part\npart-000.rs.part\n",
            "part-000.rs.part\npart-000.rs.part\n",
            "part-000.rs.part\npart-002.rs.part\n",
            "../part-000.rs.part\n",
            "part-000.rs.part\r\n",
            "part-000.rs.part\n\n",
        ] {
            assert!(parse_manifest(manifest).is_err(), "accepted {manifest:?}");
        }
        let directory = Scratch::new();
        write_with_limit(&directory.0, "a\nb\nc\n", 4).unwrap();
        fs::remove_file(directory.0.join(part_name(1))).unwrap();
        assert!(read_with_limit(&directory.0, 4)
            .unwrap_err()
            .contains("missing"));
        assert!(write_with_limit(&directory.0, "replacement", 20).is_err());
    }

    #[test]
    fn unexpected_files_are_rejected_and_never_deleted_by_regeneration() {
        let directory = Scratch::new();
        write_with_limit(&directory.0, "a\nb\nc\n", 4).unwrap();
        let unexpected = directory.0.join("part-099.rs.part");
        fs::write(&unexpected, "unlisted work").unwrap();
        assert!(read_with_limit(&directory.0, 4)
            .unwrap_err()
            .contains("unexpected"));
        assert!(write_with_limit(&directory.0, "tail", 4).is_err());
        assert_eq!(fs::read_to_string(unexpected).unwrap(), "unlisted work");
        assert_eq!(
            fs::read_to_string(directory.0.join(part_name(0))).unwrap(),
            "a\nb\n"
        );
    }
}
