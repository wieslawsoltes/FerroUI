//! A bounded async script catalog for one transport-backed QML component.
//! The compiler rewrites imports against immutable in-memory module keys; the
//! transport alone can read a remote URL. V8 never fetches on the network.
use super::*;
use std::fmt::Write;
use url::Url;

pub(crate) const PREFIX: &str = "https://qml.invalid/modules/remote/";
type Catalog = BTreeMap<String, ir::ScriptResource>;

#[derive(Default)]
pub(crate) struct Scripts {
    sources: BTreeMap<Url, String>,
    queued: BTreeSet<(Url, bool)>,
    preload: BTreeSet<String>,
    resources: Catalog,
}
impl Scripts {
    fn request(&mut self, owner: &Url, spec: &str, module: bool) -> Result<String, String> {
        let url = owner.join(spec).map_err(|e| format!("{owner}: {e}"))?;
        if url.origin() != owner.origin()
            || !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(format!("{owner}: disallowed script dependency {url}"));
        }
        if url.as_str().len() > 2048
            || !(url.path().ends_with(".js") || url.path().ends_with(".mjs"))
        {
            return Err(format!("{owner}: unsupported script URL {url}"));
        }
        let module = module || url.path().ends_with(".mjs");
        let mut key = format!("{PREFIX}{}/", if module { "module" } else { "classic" });
        for byte in url.as_str().as_bytes() {
            write!(key, "{byte:02x}").expect("string formatting");
        }
        if !self.resources.contains_key(&key) {
            self.queued.insert((url, module));
            if self.queued.len() + self.resources.len() > 64 {
                return Err(format!("{owner}: too many remote script resources"));
            }
        }
        Ok(key)
    }
    pub(crate) fn attach(&mut self, doc: &mut ir::Document, url: &Url) -> Result<(), String> {
        let mut aliases = BTreeMap::new();
        for import in doc.imports.iter().filter(|i| i.is_script()) {
            let key = self.request(url, &import.uri, false)?;
            self.preload.insert(key.clone());
            aliases.insert(
                import
                    .alias
                    .clone()
                    .ok_or("script import requires a namespace")?,
                key,
            );
        }
        ir::javascript::visit_scripts(&mut doc.root, &mut |script| {
            ir::script_resource::attach(script, url.as_str(), &aliases, |dep| {
                self.request(url, dep, true)
            })
        })
    }
    pub(crate) fn waiting(&self, url: &Url) -> bool {
        self.queued.iter().any(|(u, _)| u == url)
    }
    pub(crate) fn receive(&mut self, url: &Url, text: Option<&str>) -> Result<(), String> {
        let text = text.ok_or_else(|| format!("{url}: script resource not found"))?;
        self.sources.insert(url.clone(), text.into());
        Ok(())
    }
    pub(crate) fn source_count(&self) -> usize {
        self.sources.len()
    }
    pub(crate) fn next(&mut self) -> Result<Option<Url>, String> {
        while let Some((url, module)) = self.queued.first().cloned() {
            let Some(source) = self.sources.get(&url).cloned() else {
                return Ok(Some(url));
            };
            let key = self.request(&url, url.as_str(), module)?;
            let mut resource = ir::script_resource::compile(&key, &source, module, |dep, esm| {
                self.request(&url, dep, esm)
            })
            .map_err(|e| format!("{url}: {e}"))?;
            resource.preload = self.preload.contains(&key);
            self.queued.remove(&(url, module));
            self.resources.insert(key, resource);
        }
        Ok(None)
    }
    pub(crate) fn finish(&mut self) -> Catalog {
        for key in &self.preload {
            if let Some(resource) = self.resources.get_mut(key) {
                resource.preload = true;
            }
        }
        std::mem::take(&mut self.resources)
    }
}

impl Instance {
    pub(crate) fn install_remote_scripts(
        &mut self,
        owner: ObjectId,
        catalog: Catalog,
    ) -> Result<(), String> {
        if catalog.is_empty() {
            return Ok(());
        }
        let previous = self.javascript.resources.clone();
        let resources = &mut self.javascript.resources;
        let mut count = 0usize;
        let mut bytes = 0usize;
        for (url, script) in resources.iter().filter(|(k, _)| k.starts_with(PREFIX)) {
            count += 1;
            bytes += url.len() + script.source.len() + script.executable.len();
        }
        let (mut added, mut added_bytes) = (0, 0);
        let mut changed = false;
        for (key, new) in &catalog {
            if let Some(old) = resources.get(key) {
                if old.source != new.source
                    || old.executable != new.executable
                    || old.dependencies != new.dependencies
                {
                    return Err("conflicting remote script at an already loaded URL".into());
                }
                changed |= new.preload && !old.preload;
            } else {
                let cost = key.len() + new.source.len() + new.executable.len();
                added += 1;
                added_bytes += cost;
                count += 1;
                bytes += cost;
                changed = true;
            }
        }
        if count + self.network.failed_script_cost.0 > 256
            || bytes + self.network.failed_script_cost.1 > 8 * 1024 * 1024
        {
            return Err("instance remote script catalog exceeds its limit".into());
        }
        if changed {
            let all = std::sync::Arc::make_mut(resources);
            for (key, new) in catalog {
                let alias = new.preload;
                all.entry(key).or_insert(new).preload |= alias;
            }
            let result = self.transaction(|tx| {
                let engine = tx.script_engine()?;
                tx.with_receiver_context(owner, &mut BTreeSet::new(), |ctx| {
                    engine.prepare_script_resources(ctx)
                })
            });
            if let Err(error) = result {
                // The VM may have installed source bytes before an ES module threw.
                // Charge failed installs too, and restore the usable prior manifest.
                self.network.failed_script_cost.0 += added;
                self.network.failed_script_cost.1 += added_bytes;
                self.javascript.resources = previous;
                let _ = self.transaction(|tx| {
                    let engine = tx.script_engine()?;
                    tx.with_receiver_context(owner, &mut BTreeSet::new(), |ctx| {
                        engine.prepare_script_resources(ctx)
                    })
                });
                return Err(error.to_string());
            }
        }
        Ok(())
    }
}
