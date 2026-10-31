//! Resolve composite types without consulting a filesystem or a global type registry.
use crate::*;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct ComponentSource {
    /// A stable identity, e.g. a canonical filesystem path or resource URL.
    pub key: String,
    pub document: Rc<Document>,
}

/// Native types are left for the host registry. Returning a component instead
/// gives this instantiation a fresh private id scope.
pub trait ComponentResolver {
    /// Imports that are to be resolved by the host, after consuming source modules.
    fn native_imports(&mut self, _from: &str, imports: &[Import]) -> Result<Vec<Import>, String> {
        Ok(imports.iter().filter(|i| !i.is_path).cloned().collect())
    }

    fn resolve(
        &mut self,
        from: &str,
        imports: &[Import],
        ty: &str,
    ) -> Result<Option<ComponentSource>, String>;
}

#[derive(Debug)]
pub struct LinkError {
    pub origin: String,
    pub span: Span,
    pub message: String,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (byte {}): {}",
            self.origin, self.span.start, self.message
        )
    }
}
impl std::error::Error for LinkError {}

/// Expand file and inline components and retain the defining document's native
/// type imports on each object. File imports are consumed by the resolver.
pub fn link(
    source: ComponentSource,
    resolver: &mut impl ComponentResolver,
) -> Result<Document, LinkError> {
    let mut linker = Linker {
        resolver,
        stack: vec![source.key.clone()],
        serial: 0,
        nodes: 0,
        types: HashMap::new(),
    };
    let context = Context::new(source.key, &source.document, linker.resolver)?;
    let mut root = source.document.root.clone();
    linker.object(&context, &mut root)?;
    Ok(Document {
        root,
        pragmas: source.document.pragmas.clone(),
        imports: context.native_imports,
    })
}

struct Context {
    origin: String,
    imports: Vec<Import>,
    native_imports: Vec<Import>,
    inline: BTreeMap<String, Object>,
}

impl Context {
    fn new(
        origin: String,
        document: &Document,
        resolver: &mut impl ComponentResolver,
    ) -> Result<Self, LinkError> {
        let native_imports = resolver
            .native_imports(&origin, &document.imports)
            .map_err(|message| LinkError {
                origin: origin.clone(),
                span: document.root.span,
                message,
            })?;
        let inline = document
            .root
            .members
            .iter()
            .filter_map(|m| match &m.kind {
                MemberKind::Component { name, object } => Some((name.clone(), object.clone())),
                _ => None,
            })
            .collect();
        Ok(Self {
            origin,
            imports: document.imports.clone(),
            native_imports,
            inline,
        })
    }
    fn error(&self, span: Span, message: impl Into<String>) -> LinkError {
        LinkError {
            origin: self.origin.clone(),
            span,
            message: message.into(),
        }
    }
}

struct Linker<'a, R> {
    resolver: &'a mut R,
    stack: Vec<String>,
    serial: usize,
    nodes: usize,
    types: HashMap<String, String>,
}

impl<R: ComponentResolver> Linker<'_, R> {
    fn object(&mut self, context: &Context, site: &mut Object) -> Result<(), LinkError> {
        self.nodes += 1;
        if self.nodes > 65_536 || self.stack.len() > 48 {
            return Err(context.error(site.span, "component expansion limit exceeded"));
        }
        site.members
            .retain(|m| !matches!(m.kind, MemberKind::Component { .. }));
        self.members(context, &mut site.members)?;

        if let Some(template) = context.inline.get(&site.type_name) {
            return self.expand(
                context,
                format!("{}#{}", context.origin, site.type_name),
                template,
                site,
            );
        }
        if let Some(source) = self
            .resolver
            .resolve(&context.origin, &context.imports, &site.type_name)
            .map_err(|message| context.error(site.span, message))?
        {
            if !source.document.pragmas.is_empty() {
                return Err(context.error(
                    site.span,
                    format!(
                        "pragmas on imported component '{}' are unsupported",
                        source.key
                    ),
                ));
            }
            let defining = Context::new(source.key.clone(), &source.document, self.resolver)?;
            return self.expand(&defining, source.key, &source.document.root, site);
        }
        site.type_imports = Some(context.native_imports.clone());
        Ok(())
    }

    fn type_id(&mut self, key: &str) -> String {
        let index = self.types.len();
        self.types
            .entry(key.to_owned())
            .or_insert_with(|| format!("Qml$Type{index}"))
            .clone()
    }

    fn annotation(
        &mut self,
        context: &Context,
        ty: &mut String,
        span: Span,
    ) -> Result<(), LinkError> {
        let list = ty.strip_prefix("list<").and_then(|ty| ty.strip_suffix('>'));
        let name = list.unwrap_or(ty);
        if !name.chars().next().is_some_and(char::is_uppercase) {
            return Ok(());
        }
        let key = if context.inline.contains_key(name) {
            Some(format!("{}#{name}", context.origin))
        } else {
            self.resolver
                .resolve(&context.origin, &context.imports, name)
                .map_err(|message| context.error(span, message))?
                .map(|s| s.key)
        };
        if let Some(key) = key {
            let id = self.type_id(&key);
            *ty = if list.is_some() {
                format!("list<{id}>")
            } else {
                id
            };
        }
        Ok(())
    }

    fn members(&mut self, context: &Context, members: &mut [Member]) -> Result<(), LinkError> {
        for member in members {
            match &mut member.kind {
                MemberKind::Property { ty, .. } => self.annotation(context, ty, member.span)?,
                MemberKind::Function {
                    parameters,
                    return_type,
                    ..
                } => {
                    if let Some(ty) = return_type {
                        self.annotation(context, ty, member.span)?;
                    }
                    for param in parameters {
                        if let Some(ty) = &mut param.ty {
                            self.annotation(context, ty, member.span)?;
                        }
                    }
                }
                MemberKind::Signal { parameters, .. } => {
                    for param in parameters {
                        if let Some(ty) = &mut param.ty {
                            self.annotation(context, ty, member.span)?;
                        }
                    }
                }
                _ => (),
            }
            match &mut member.kind {
                MemberKind::Child(object) => self.object(context, object)?,
                MemberKind::Group { members, .. } => self.members(context, members)?,
                MemberKind::Binding { value, .. }
                | MemberKind::Property {
                    value: Some(value), ..
                } => match value {
                    Binding::Object(object) => self.object(context, object)?,
                    Binding::Objects(objects) => {
                        for object in objects {
                            self.object(context, object)?;
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        }
        Ok(())
    }

    fn expand(
        &mut self,
        context: &Context,
        key: String,
        template: &Object,
        site: &mut Object,
    ) -> Result<(), LinkError> {
        if self.stack.contains(&key) {
            return Err(context.error(
                site.span,
                format!("recursive component: {} -> {key}", self.stack.join(" -> ")),
            ));
        }
        let type_id = self.type_id(&key);
        self.stack.push(key);
        self.serial += 1;
        let mut base = template.clone();
        let root_id = object_id(site)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("$qml:{}:root", self.serial));
        super::scope::isolate(&mut base, self.serial, &root_id);
        self.object(context, &mut base)?;
        self.stack.pop();
        merge_site(&mut base, site, context)?;
        base.composite_types.push(type_id);
        *site = base;
        Ok(())
    }
}

pub(crate) fn object_id(object: &Object) -> Option<&str> {
    object.members.iter().find_map(|m| match &m.kind {
        MemberKind::Id(id) => Some(id.as_str()),
        _ => None,
    })
}

fn merge_site(base: &mut Object, site: &mut Object, context: &Context) -> Result<(), LinkError> {
    let assigned: HashMap<_, _> = site
        .members
        .iter()
        .filter_map(|m| match &m.kind {
            MemberKind::Binding { name, .. } => Some((name.as_str(), m.span)),
            _ => None,
        })
        .collect();
    for member in &mut base.members {
        if let MemberKind::Property {
            name,
            ty,
            modifiers,
            value,
        } = &mut member.kind
        {
            if let Some(&span) = assigned.get(name.as_str()) {
                if modifiers.readonly {
                    return Err(context.error(
                        span,
                        format!("cannot initialize inherited readonly property '{name}'"),
                    ));
                }
                // An alias declaration identifies the destination of the assignment.
                if ty != "alias" {
                    *value = None;
                }
            }
        }
    }
    base.members.retain(|m| !matches!(&m.kind, MemberKind::Binding { name, .. } if assigned.contains_key(name.as_str())));
    base.members.extend(
        site.members
            .drain(..)
            .filter(|m| !matches!(m.kind, MemberKind::Id(_))),
    );
    Ok(())
}
