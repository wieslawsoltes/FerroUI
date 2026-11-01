//! Immutable application metadata and exact substring search. The index only
//! supplies candidates; the final string checks define matching semantics.
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct DesktopApplication {
    /// The XDG desktop file ID, including its `.desktop` suffix.
    pub id: String,
    pub name: String,
    pub description: String,
    /// Serialized GIcon, usually a single theme icon name.
    pub icon: String,
    search_text: String,
}

impl DesktopApplication {
    pub(super) fn new(id: String, name: String, description: String, icon: String) -> Self {
        let search_text = format!("{name}\n{id}\n{description}").to_lowercase();
        Self {
            id,
            name,
            description,
            icon,
            search_text,
        }
    }
}

#[derive(Default, Debug)]
pub struct ApplicationCatalog {
    applications: Vec<DesktopApplication>,
    ids: HashMap<String, usize>,
    /// Postings preserve catalog order and contain each application at most once.
    trigrams: HashMap<[u8; 3], Vec<usize>>,
    unavailable_reason: Option<String>,
}

impl ApplicationCatalog {
    pub(super) fn from_applications(mut applications: Vec<DesktopApplication>) -> Self {
        applications.sort_by_cached_key(|app| (app.name.to_lowercase(), app.id.clone()));
        let mut ids = HashMap::with_capacity(applications.len());
        let mut trigrams: HashMap<_, Vec<usize>> = HashMap::new();
        for (i, app) in applications.iter().enumerate() {
            ids.insert(app.id.clone(), i);
            // A linear scan wins for a small local menu. Larger development
            // and managed installations amortize the index over keystrokes.
            if applications.len() >= 256 {
                for gram in app.search_text.as_bytes().windows(3) {
                    let postings = trigrams.entry([gram[0], gram[1], gram[2]]).or_default();
                    if postings.last() != Some(&i) {
                        postings.push(i);
                    }
                }
            }
        }
        Self {
            applications,
            ids,
            trigrams,
            unavailable_reason: None,
        }
    }

    pub(super) fn unavailable(reason: String) -> Self {
        Self {
            unavailable_reason: Some(reason),
            ..Self::default()
        }
    }

    pub fn applications(&self) -> &[DesktopApplication] {
        &self.applications
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.ids.get(id).copied()
    }

    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }

    /// Pinned results follow the user's ordering even when a query is present.
    pub fn pinned_indices(&self, pins: &[String], query: &str) -> Vec<usize> {
        let query = query.to_lowercase();
        let words: Vec<_> = query.split_whitespace().collect();
        pins.iter()
            .filter_map(|id| self.index_of(id))
            .filter(|&index| {
                words
                    .iter()
                    .all(|word| self.applications[index].search_text.contains(*word))
            })
            .collect()
    }

    pub fn search_indices(&self, query: &str) -> Vec<usize> {
        let query = query.to_lowercase();
        let words: Vec<_> = query.split_whitespace().collect();
        if words.is_empty() {
            return (0..self.applications.len()).collect();
        }
        let matches = |&index: &usize| {
            let text = &self.applications[index].search_text;
            words.iter().all(|word| text.contains(*word))
        };
        if !self.trigrams.is_empty() {
            let mut rarest: Option<&[usize]> = None;
            for gram in words.iter().flat_map(|word| word.as_bytes().windows(3)) {
                let Some(postings) = self.trigrams.get(gram) else {
                    return Vec::new();
                };
                if rarest.is_none_or(|previous| postings.len() < previous.len()) {
                    rarest = Some(postings);
                }
            }
            if let Some(indices) = rarest {
                return indices.iter().copied().filter(matches).collect();
            }
        }
        (0..self.applications.len()).filter(matches).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERIES: &[&str] = &[
        "",
        "a",
        "ve",
        "vector",
        "audio",
        "427",
        "com.editor 29",
        "export telemetry",
        "télé",
        "é 工",
        "cad 021",
        "unavailable-application",
        "rev 00071",
    ];

    fn catalog(size: usize) -> ApplicationCatalog {
        const NAMES: [&str; 8] = [
            "Vector Studio",
            "Audio Mixer",
            "Télémetry",
            "CAD 工具",
            "Project Explorer",
            "Shader Editor",
            "Image Viewer",
            "Database Console",
        ];
        let apps = (0..size)
            .map(|i| {
                let name = NAMES[i % NAMES.len()];
                DesktopApplication::new(
                    format!("com.editor.{i:05}.desktop"),
                    format!("{name} {i:05}"),
                    format!(
                        "Inspect or export workspace documents, revision {}",
                        i % 491
                    ),
                    String::new(),
                )
            })
            .collect();
        ApplicationCatalog::from_applications(apps)
    }

    // This is the pre-index search implementation, also used as the parity oracle.
    fn linear(catalog: &ApplicationCatalog, query: &str) -> Vec<usize> {
        let query = query.to_lowercase();
        catalog
            .applications
            .iter()
            .enumerate()
            .filter_map(|(i, app)| {
                query
                    .split_whitespace()
                    .all(|word| app.search_text.contains(word))
                    .then_some(i)
            })
            .collect()
    }

    #[test]
    fn search_matches_the_linear_oracle_across_index_threshold_and_unicode() {
        for size in [0, 21, 255, 256, 1200] {
            let catalog = catalog(size);
            for query in QUERIES
                .iter()
                .copied()
                .chain(["  工具 télé", "\tSTUDIO 42", " É  "])
            {
                assert_eq!(
                    catalog.search_indices(query),
                    linear(&catalog, query),
                    "{size}: {query}"
                );
            }
        }
    }

    #[test]
    #[ignore = "manual timing fixture; run with the release profile and --nocapture"]
    fn benchmark_catalog_search() {
        use std::{hint::black_box, time::Instant};
        for size in [100, 1000, 10_000] {
            let start = Instant::now();
            let catalog = catalog(size);
            let creation = start.elapsed();
            let measure = |search: fn(&ApplicationCatalog, &str) -> Vec<usize>| {
                let mut samples = Vec::new();
                for _ in 0..31 {
                    let start = Instant::now();
                    for query in QUERIES {
                        black_box(search(black_box(&catalog), black_box(query)));
                    }
                    samples.push(start.elapsed());
                }
                samples.sort_unstable();
                samples[samples.len() / 2]
            };
            let before = measure(linear);
            let after = measure(ApplicationCatalog::search_indices);
            eprintln!("{size} apps, construction {creation:?}, median {}-query batch: linear {before:?}, indexed {after:?}", QUERIES.len());
        }
    }
}
