//! Resolve runtime component URLs against Cargo's immutable, relocatable
//! catalog. Factories own their status, creation context and optional parent.
use super::*;
use crate::{
    dynamic::{Context, Template},
    transaction::Transaction,
};
use url::Url;

fn module(uri: &str) -> ir::Import {
    ir::Import {
        uri: uri.into(),
        is_path: false,
        version: None,
        alias: None,
        span: Default::default(),
    }
}
fn shape(ty: &str, url: &str, imports: Vec<ir::Import>) -> ir::Object {
    ir::Object {
        type_name: ty.into(),
        source_url: Some(url.into()),
        source_layers: vec![],
        shadows: vec![],
        composite_types: vec![],
        type_imports: Some(imports),
        members: vec![],
        span: Default::default(),
    }
}
impl Template {
    pub(crate) fn from_object(object: ir::Object, imports: &[ir::Import]) -> Result<Self, Error> {
        let name = object
            .source_url
            .as_deref()
            .and_then(|u| u.rsplit('/').next())
            .and_then(|n| n.strip_suffix(".qml"));
        Ok(Self {
            scopes: enums::Scopes::named(name, &object)?,
            imports: Rc::from(object.type_imports.as_deref().unwrap_or(imports)),
            object: Rc::new(object),
        })
    }
}
const ENGINE_BASE: &str = "https://qml.invalid/components/project/";

fn project_url(mut target: Url) -> Url {
    if target.host_str() == Some("qml.invalid") {
        if let Some(rest) = target
            .path()
            .strip_prefix("/modules/project/")
            .or_else(|| target.path().strip_prefix("/modules/classic/project/"))
        {
            let path = format!("/components/project/{rest}");
            target.set_path(&path);
        }
    }
    target
}

fn resolve(base: Option<&str>, spec: &str) -> Result<String, Error> {
    Url::parse(base.unwrap_or(ENGINE_BASE))
        .map(project_url)
        .and_then(|base| base.join(spec))
        .map(|url| project_url(url).to_string())
        .map_err(|e| Error::new(e.to_string()))
}

// QML's URL-to-string display preserves escaped delimiters (e.g. %2F)
// while showing spaces, unreserved ASCII and UTF-8 path text.
fn display_url(url: &str) -> String {
    let bytes = url.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            if let Some(byte) = url
                .get(at + 1..at + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                if !byte.is_ascii() || byte.is_ascii_alphanumeric() || b" -._~".contains(&byte) {
                    output.push(byte);
                } else {
                    output.push(b'%');
                    output.push(bytes[at + 1].to_ascii_uppercase());
                    output.push(bytes[at + 2].to_ascii_uppercase());
                }
                at += 3;
                continue;
            }
        }
        output.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(output).unwrap_or_else(|_| url.to_owned())
}

impl Node {
    fn scoped_url(&self, scope: Option<u32>) -> Option<&str> {
        let layer = |scope| self.source_layers.iter().find(|layer| layer.scope == scope);
        layer(scope)
            .or_else(|| layer(None))
            .map(|layer| layer.url.as_str())
            .or(self.source_url.as_deref())
    }
}

impl Instance {
    pub(crate) fn url_utility(
        &self,
        owner: ObjectId,
        scope: Option<u32>,
        origin: Option<&str>,
        name: &str,
        args: &[Value],
    ) -> Result<Value, Error> {
        let Some(Value::String(spec)) = args.first() else {
            return Err(Error::new(format!("Qt.{name} expects a URL")));
        };
        if name == "url" {
            return match args {
                [_] => Ok(Value::String(display_url(spec))),
                _ => Err(Error::new("Qt.url expects one URL")),
            };
        }
        let node_url = |id, scope| {
            self.node(id)
                .map(|node| node.scoped_url(scope))
                .ok_or_else(|| Error::new("invalid URL resolution context"))
        };
        let base = match args {
            [_] => origin.filter(|s| !s.is_empty()).or(node_url(owner, scope)?),
            [_, Value::Object(id)] => node_url(*id, None)?,
            [_, Value::Null | Value::Undefined] => Some(ENGINE_BASE),
            _ => {
                return Err(Error::new(
                    "Qt.resolvedUrl expects a URL and an optional object context",
                ))
            }
        };
        if spec.is_empty() && !matches!(args, [_, Value::Null | Value::Undefined]) {
            return Ok(Value::String(String::new()));
        }
        Ok(Value::String(display_url(&resolve(base, spec)?)))
    }

    pub fn create_component(
        &mut self,
        owner: ObjectId,
        url: &str,
        parent: Option<ObjectId>,
    ) -> Result<ObjectId, Error> {
        self.transaction(|tx| tx.url_factory(owner, None, url, false, parent))
    }
}
enum FactoryDefinition {
    Bundled(Result<Template, String>),
    Network(Url),
}

impl Transaction<'_> {
    pub(crate) fn component_call(
        &mut self,
        owner: ObjectId,
        origin: Option<&str>,
        args: &[Value],
    ) -> Result<Value, Error> {
        let Some(Value::String(name)) = args.first() else {
            return Err(Error::new(
                "Qt.createComponent expects a URL or module name",
            ));
        };
        let named = matches!(args.get(1), Some(Value::String(_)));
        let (later, parent) = match &args[if named { 2 } else { 1 }..] {
            [] | [Value::Null | Value::Undefined] => (false, None),
            [Value::Object(id)] => (false, Some(*id)),
            [Value::Number(n), rest @ ..] if matches!(*n, 0. | 1.) => {
                let parent = match rest {
                    [] | [Value::Null | Value::Undefined] => None,
                    [Value::Object(id)] => Some(*id),
                    _ => return Err(Error::new("invalid component parent")),
                };
                (*n == 1., parent)
            }
            _ => return Err(Error::new("invalid Qt.createComponent arguments")),
        };
        if name.is_empty() {
            return Ok(Value::Null);
        }
        if named {
            let Value::String(ty) = &args[1] else {
                unreachable!()
            };
            let imports = vec![module(name)];
            let definition = self
                .model
                .dynamic
                .registry
                .as_ref()
                .expect("type registry")
                .resolve(&imports, ty)
                .map_err(|e| e.message)
                .and_then(|_| {
                    Template::from_object(shape(ty, "", imports.clone()), &imports)
                        .map_err(|e| e.message)
                });
            self.make_factory(
                owner,
                parent,
                "",
                FactoryDefinition::Bundled(definition),
                later,
            )
            .map(Value::Object)
        } else {
            self.url_factory(owner, origin, name, later, parent)
                .map(Value::Object)
        }
    }
    pub(crate) fn url_factory(
        &mut self,
        owner: ObjectId,
        origin: Option<&str>,
        spec: &str,
        later: bool,
        parent: Option<ObjectId>,
    ) -> Result<ObjectId, Error> {
        let node = self
            .model
            .node(owner)
            .ok_or_else(|| Error::new("invalid declaration context"))?;
        let url = resolve(origin.or(node.source_url.as_deref()), spec)?;
        let parsed = Url::parse(&url).map_err(|e| Error::new(e.to_string()))?;
        let definition = if let Some(template) = self.model.dynamic.catalog.get(&url) {
            FactoryDefinition::Bundled(Ok(template.clone()))
        } else if matches!(parsed.scheme(), "https" | "http")
            && parsed.host_str() != Some("qml.invalid")
            && self
                .model
                .dynamic
                .registry
                .as_ref()
                .is_some_and(|r| r.component_transport.is_some())
        {
            network::allow_request(self.model.network.count() + self.network_loads.len())?;
            FactoryDefinition::Network(parsed)
        } else {
            FactoryDefinition::Bundled(Err(format!("{url}: outside the compiled QML catalog")))
        };
        self.make_factory(owner, parent, &url, definition, later)
    }
    fn make_factory(
        &mut self,
        owner: ObjectId,
        parent: Option<ObjectId>,
        url: &str,
        definition: FactoryDefinition,
        later: bool,
    ) -> Result<ObjectId, Error> {
        if self.model.node(owner).is_none() || parent.is_some_and(|p| self.model.node(p).is_none())
        {
            return Err(Error::new("invalid component parent or creation context"));
        }
        let id = self.model.next_object();
        let registry = self.model.dynamic.registry.clone().expect("type registry");
        self.model.dynamic.revision = self
            .model
            .dynamic
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::new("tree revision exhausted"))?;
        self.model.dynamic.contexts.insert(
            id,
            Context {
                declaration: owner,
                row_revision: None,
                delegate_inputs: None,
                ids: Default::default(),
            },
        );
        let fetching = matches!(definition, FactoryDefinition::Network(_));
        match definition {
            FactoryDefinition::Network(url) => {
                // A URL factory is a valid Component without an inline child.
                // errorString() is empty while the request is outstanding.
                self.model.dynamic.file_errors.insert(id, String::new());
                self.network_loads.push((id, url));
            }
            FactoryDefinition::Bundled(Ok(template)) => {
                self.model.dynamic.templates.insert(id, template);
            }
            FactoryDefinition::Bundled(Err(error)) => {
                self.model.dynamic.file_errors.insert(id, error);
            }
        }
        let shell = Template::from_object(shape("Component", url, vec![module("QtQml")]), &[])?;
        self.model
            .allocate_template(id, parent, &shell, &registry)?;
        self.set_value(self.model.key(id, "url")?, Value::String(url.into()));
        if later || fetching {
            self.set_value(self.model.key(id, "status")?, Value::Number(2.));
            self.set_value(self.model.key(id, "progress")?, Value::Number(0.));
            if !fetching {
                self.model.dynamic.loading.insert(id);
            }
        } else {
            self.finish_factory(id)?;
        }
        Ok(id)
    }
    pub(crate) fn finish_factory(&mut self, id: ObjectId) -> Result<(), Error> {
        let ready = self.model.dynamic.templates.contains_key(&id);
        self.set_value(
            self.model.key(id, "status")?,
            Value::Number(if ready { 1. } else { 3. }),
        );
        self.set_value(self.model.key(id, "progress")?, Value::Number(1.));
        Ok(())
    }
    pub(crate) fn text_component_call(
        &mut self,
        owner: ObjectId,
        args: &[Value],
    ) -> Result<Value, Error> {
        let (text, parent, file) =
            match args {
                [Value::String(s), Value::Object(p)] => (s, *p, "inline"),
                [Value::String(s), Value::Object(p), Value::String(f)] => (s, *p, f.as_str()),
                _ => return Err(Error::new(
                    "Qt.createQmlObject expects source, a parent and an optional diagnostic URL",
                )),
            };
        if self.in_binding() || self.model.node(parent).is_none() {
            return Err(Error::new("invalid dynamic creation context"));
        }
        let origin = resolve(
            self.model
                .node(owner)
                .and_then(|node| node.source_url.as_deref()),
            file,
        )?;
        #[cfg(feature = "javascript")]
        let doc = if self.model.javascript.engine.is_some() {
            ir::parse_external(text)
        } else {
            ir::parse(text)
        };
        #[cfg(not(feature = "javascript"))]
        let doc = ir::parse(text);
        let mut doc = doc.map_err(|e| Error::new(format!("{origin}:{e}")))?;
        doc.root.set_source_url(&origin);
        #[cfg(feature = "javascript")]
        if self.model.javascript.engine.is_some() {
            let mut imports = BTreeMap::new();
            for import in doc.imports.iter().filter(|i| i.is_script()) {
                let source = resolve(Some(&origin), &import.uri)?;
                let tail = source
                    .strip_prefix("https://qml.invalid/components/project/")
                    .ok_or_else(|| Error::new("script outside project"))?;
                let dir = if import.uri.ends_with(".mjs") {
                    "modules"
                } else {
                    "modules/classic"
                };
                let key = format!("https://qml.invalid/{dir}/project/{tail}");
                if !self.model.javascript.resources.contains_key(&key) {
                    return Err(Error::new(format!(
                        "{origin}:1:1: script was not embedded: {}",
                        import.uri
                    )));
                }
                imports.insert(
                    import
                        .alias
                        .clone()
                        .ok_or_else(|| Error::new("script import requires namespace"))?,
                    key,
                );
            }
            ir::javascript::visit_scripts(&mut doc.root, &mut |script| {
                script.origin.clone_from(&origin);
                script.imports.clone_from(&imports);
                Ok::<_, Error>(())
            })?;
            doc.imports.retain(|i| !i.is_script());
        }
        let source = ir::ComponentSource {
            key: origin.clone(),
            document: Rc::new(doc),
        };
        let doc = ir::link(source, &mut Resolver(&self.model.dynamic.catalog))
            .map_err(|e| Error::new(format!("{}:1:1: {}", e.origin, e.message)))?;
        let template = Template::from_object(doc.root, &doc.imports)?;
        self.build_object(parent, Some(parent), BTreeMap::new(), &template)
            .map(Value::Object)
            .map_err(|e| Error::new(format!("{origin}:1:1: {e}")))
    }
}
struct Resolver<'a>(&'a BTreeMap<String, Template>);
impl ir::ComponentResolver for Resolver<'_> {
    fn resolve(
        &mut self,
        from: &str,
        imports: &[ir::Import],
        ty: &str,
    ) -> Result<Option<ir::ComponentSource>, String> {
        let base = Url::parse(from).map_err(|e| e.to_string())?;
        let (ns, name) = ty
            .split_once('.')
            .map_or((None, ty), |(ns, name)| (Some(ns), name));
        let dirs = imports
            .iter()
            .filter(|i| i.is_path && !i.is_script() && i.alias.as_deref() == ns)
            .map(|i| format!("{}/", i.uri.trim_end_matches('/')));
        let mut matches = BTreeSet::new();
        for dir in ns
            .is_none()
            .then_some(".".to_owned())
            .into_iter()
            .chain(dirs)
        {
            let url = base
                .join(&dir)
                .and_then(|b| b.join(&format!("{name}.qml")))
                .map_err(|e| e.to_string())?
                .to_string();
            if self.0.contains_key(&url) {
                matches.insert(url);
            }
        }
        if matches.len() > 1 {
            return Err(format!("ambiguous QML component {ty}"));
        }
        Ok(matches.into_iter().next().map(|key| {
            let template = &self.0[&key];
            ir::ComponentSource {
                key,
                document: Rc::new(ir::Document {
                    root: (*template.object).clone(),
                    imports: template.imports.to_vec(),
                    components: Default::default(),
                    scripts: Default::default(),
                    singletons: vec![],
                    pragmas: vec![],
                }),
            }
        }))
    }
}
