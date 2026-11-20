//! Carry the compiler's target type through each branch of the selector.
use super::{PropertyUse, References, Source, TypeUse};
use crate::{Schema, TypeInfo, FRAMEWORK_NAMESPACE};
use ferroui_designer_core::Document;
use ferroui_markup_syntax::selector_grammar::{SelectorOperand, SelectorSyntax};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

pub(super) fn references(
    schema: &Schema,
    document: &Document,
    source: &Source<'_>,
    nesting: Option<&TypeInfo>,
    syntax: &[SelectorSyntax],
    operands: Vec<SelectorOperand>,
    output: &mut References,
) -> bool {
    let scope = Scope::new(schema, document, source, nesting);
    let mut capture = Some(Capture {
        operands: operands.into_iter(),
        output,
        source,
    });
    if scope.resolve(syntax, &mut capture).is_err() {
        return false;
    }
    capture.unwrap().operands.next().is_none()
}

pub(super) fn target_type<'s>(
    schema: &'s Schema,
    document: &Document,
    source: &Source<'_>,
    nesting: Option<&'s TypeInfo>,
    syntax: &[SelectorSyntax],
) -> Option<&'s TypeInfo> {
    Scope::new(schema, document, source, nesting)
        .resolve(syntax, &mut None)
        .ok()
        .flatten()
}

pub(super) fn contains_nesting(syntax: &[SelectorSyntax]) -> bool {
    syntax.iter().any(|item| match item {
        SelectorSyntax::Nesting => true,
        SelectorSyntax::Not { argument } => contains_nesting(argument),
        _ => false,
    })
}

struct Scope<'s> {
    schema: &'s Schema,
    namespaces: BTreeMap<String, String>,
    nesting: Option<&'s TypeInfo>,
}

impl<'s> Scope<'s> {
    fn new(
        schema: &'s Schema,
        document: &Document,
        source: &Source<'_>,
        nesting: Option<&'s TypeInfo>,
    ) -> Self {
        Self {
            schema,
            namespaces: document.ast().namespaces(source.scope.id),
            nesting,
        }
    }

    fn namespace(&self, prefix: &str) -> Option<&str> {
        self.namespaces
            .get(prefix)
            .map(String::as_str)
            .or_else(|| prefix.is_empty().then_some(FRAMEWORK_NAMESPACE))
    }

    fn resolve(
        &self,
        syntax: &[SelectorSyntax],
        capture: &mut Option<Capture<'_, '_>>,
    ) -> Result<Option<&'s TypeInfo>, ()> {
        let mut branches = syntax.split(|item| matches!(item, SelectorSyntax::Comma));
        let first = self.branch(branches.next().unwrap_or_default(), capture)?;
        branches.try_fold(first, |left, branch| {
            let right = self.branch(branch, capture)?;
            Ok(left
                .zip(right)
                .and_then(|(left, right)| common_base(self.schema, left, right)))
        })
    }

    fn branch(
        &self,
        syntax: &[SelectorSyntax],
        capture: &mut Option<Capture<'_, '_>>,
    ) -> Result<Option<&'s TypeInfo>, ()> {
        let mut subject = None;
        for item in syntax {
            match item {
                SelectorSyntax::OfType { type_name, xmlns }
                | SelectorSyntax::Is { type_name, xmlns } => {
                    let namespace = self.namespace(xmlns);
                    subject =
                        namespace.and_then(|ns| self.schema.exact_type_info(type_name, Some(ns)));
                    if let Some(capture) = capture {
                        let SelectorOperand::Type(span) = capture.next()? else {
                            return Err(());
                        };
                        capture.type_use(type_name, namespace, span)?;
                    }
                }
                SelectorSyntax::Nesting => subject = self.nesting,
                SelectorSyntax::Template | SelectorSyntax::Child | SelectorSyntax::Descendant => {
                    subject = None
                }
                SelectorSyntax::Not { argument } => {
                    if capture.is_some() {
                        self.resolve(argument, capture)?;
                    }
                }
                SelectorSyntax::Property { property, .. } => {
                    if let Some(capture) = capture {
                        let SelectorOperand::Property(span) = capture.next()? else {
                            return Err(());
                        };
                        capture.property_use(property, span, self.schema, subject, false)?;
                    }
                }
                SelectorSyntax::AttachedProperty {
                    xmlns,
                    type_name,
                    property,
                    ..
                } => {
                    if let Some(capture) = capture {
                        let SelectorOperand::AttachedProperty {
                            owner,
                            property: span,
                        } = capture.next()?
                        else {
                            return Err(());
                        };
                        let namespace = self.namespace(xmlns);
                        capture.type_use(type_name, namespace, owner)?;
                        let owner = subject
                            .and(namespace)
                            .and_then(|ns| self.schema.exact_type_info(type_name, Some(ns)));
                        capture.property_use(property, span, self.schema, owner, true)?;
                    }
                }
                _ => {}
            }
        }
        Ok(subject)
    }
}

fn common_base<'s>(
    schema: &'s Schema,
    left: &'s TypeInfo,
    right: &'s TypeInfo,
) -> Option<&'s TypeInfo> {
    let ancestors: Vec<_> = bases(schema, right).collect();
    bases(schema, left).find(|ty| ancestors.iter().any(|other| same_type(ty, other)))
}

fn same_type(left: &TypeInfo, right: &TypeInfo) -> bool {
    left.name == right.name
        && (left.namespace == right.namespace
            || left
                .authored
                .as_ref()
                .zip(right.authored.as_ref())
                .is_some_and(|(left, right)| left.same_registration(right)))
}

fn bases<'s>(schema: &'s Schema, ty: &'s TypeInfo) -> impl Iterator<Item = &'s TypeInfo> {
    let mut seen = BTreeSet::new();
    std::iter::successors(Some(ty), |ty| schema.base_type(ty))
        .take_while(move |ty| seen.insert((&ty.namespace, &ty.name)))
}

struct Capture<'a, 's> {
    operands: std::vec::IntoIter<SelectorOperand>,
    output: &'a mut References,
    source: &'a Source<'s>,
}

impl Capture<'_, '_> {
    fn next(&mut self) -> Result<SelectorOperand, ()> {
        self.operands.next().ok_or(())
    }

    fn type_use(
        &mut self,
        spelling: &str,
        namespace: Option<&str>,
        range: Range<usize>,
    ) -> Result<(), ()> {
        if !range.is_empty() {
            self.output.types.push(TypeUse {
                span: self.source.operand_span(range).ok_or(())?,
                spelling: spelling.into(),
                namespace: namespace.map(str::to_owned),
            });
        }
        Ok(())
    }

    fn property_use(
        &mut self,
        spelling: &str,
        range: Range<usize>,
        schema: &Schema,
        owner: Option<&TypeInfo>,
        attached: bool,
    ) -> Result<(), ()> {
        if !range.is_empty() {
            self.output.properties.push(PropertyUse {
                span: self.source.operand_span(range).ok_or(())?,
                spelling: spelling.into(),
                member: owner
                    .and_then(|ty| schema.property(&ty.name, Some(&ty.namespace), spelling))
                    .filter(|member| member.attached == attached && !member.event),
            });
        }
        Ok(())
    }
}
