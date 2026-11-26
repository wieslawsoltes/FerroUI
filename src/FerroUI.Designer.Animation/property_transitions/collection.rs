use super::{
    children, is_setters, is_style, native_animation_namespace, timing, PropertyTransitionList,
    PropertyTransitionTiming, PROPERTY_TRANSITION_TYPES,
};
use crate::programs::{core_error, native_fragment};
use crate::xaml::{escape, native_ticks, time_string};
use crate::{require, AnimationError, Result};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind, SourceSpan, TextPatch};

/// A property reference and transition class selected from the native target's registry.
pub struct XamlTransitionTarget {
    pub property: String,
    pub type_name: &'static str,
    pub namespaces: Vec<(String, String)>,
}

impl XamlTransitionTarget {
    /// Render a validated native transition for a designer projection or source insertion.
    pub fn to_xaml(&self, timing: &PropertyTransitionTiming) -> Result<String> {
        transition_markup(self, timing)
    }
}

pub enum PropertyTransitionCollectionEdit<'a> {
    Add {
        target: &'a XamlTransitionTarget,
        timing: &'a PropertyTransitionTiming,
    },
    Retarget {
        transition: NodeId,
        target: &'a XamlTransitionTarget,
    },
    Move {
        transition: NodeId,
        index: usize,
    },
    Remove {
        transition: NodeId,
    },
}

impl PropertyTransitionList {
    /// A resource assignment or ambiguous source stays in place.
    pub fn can_add(&self) -> bool {
        self.collection.is_some() || self.empty_container.is_some() || self.source.is_none()
    }

    pub fn plan_collection_edit(
        document: &Document,
        owner: NodeId,
        edit: PropertyTransitionCollectionEdit<'_>,
    ) -> Result<Vec<Edit>> {
        require(
            document.is_valid(),
            "Repair the XAML before editing its transition collection",
        )?;
        let list = Self::read(document, owner)?;
        let entry = |transition| {
            list.entries
                .iter()
                .find(|entry| entry.id == transition)
                .ok_or_else(|| {
                    AnimationError("The transition is outside the selected collection".into())
                })
        };
        let edit = match edit {
            PropertyTransitionCollectionEdit::Add { target, timing } => {
                list.add(document, target, timing)?
            }
            PropertyTransitionCollectionEdit::Retarget { transition, target } => {
                require(
                    entry(transition)?.can_retarget,
                    "Reveal the authored property before retargeting it",
                )?;
                let node = document
                    .ast()
                    .node(transition)
                    .expect("the authored entry exists");
                return retarget(document, node, target);
            }
            PropertyTransitionCollectionEdit::Move { transition, index } => {
                entry(transition)?;
                require(
                    index < list.entries.len(),
                    "Choose an index in the current transition collection",
                )?;
                Edit::MoveNode {
                    node: transition,
                    parent: list.collection.expect("the entry has a collection"),
                    index,
                }
            }
            PropertyTransitionCollectionEdit::Remove { transition } => {
                entry(transition)?;
                Edit::RemoveNode { node: transition }
            }
        };
        Ok(vec![edit])
    }

    fn add(
        &self,
        document: &Document,
        target: &XamlTransitionTarget,
        timing: &PropertyTransitionTiming,
    ) -> Result<Edit> {
        require(
            self.can_add(),
            "Open the authored transition collection before adding a property",
        )?;
        let transition = target.to_xaml(timing)?;
        if let Some(parent) = self.collection {
            return Ok(Edit::InsertChild {
                parent,
                index: self.entries.len(),
                source: native_fragment(document, parent, &transition)?,
            });
        }

        let collection = format!("<Transitions>{transition}</Transitions>");
        if let Some(parent) = self.empty_container {
            return Ok(Edit::InsertChild {
                parent,
                index: 0,
                source: native_fragment(document, parent, &collection)?,
            });
        }
        let owner = document
            .ast()
            .node(self.owner)
            .expect("the authored owner exists");
        let (parent, source) = if is_style(owner) {
            let wrappers: Vec<_> = children(document, owner)
                .filter(|node| is_setters(node))
                .collect();
            require(wrappers.len() <= 1, "Choose the authored setter collection")?;
            let parent = wrappers.first().map_or(owner.id, |node| node.id);
            (
                parent,
                format!("<Setter Property=\"Transitions\">{collection}</Setter>"),
            )
        } else {
            let property = format!("{}.Transitions", owner.name);
            // Qualify the nested object independently of the property wrapper.
            let collection = native_fragment(document, owner.id, &collection)?;
            return Ok(Edit::InsertChild {
                parent: owner.id,
                index: children(document, owner)
                    .take_while(|node| node.kind == NodeKind::PropertyElement)
                    .count(),
                source: format!("<{property}>{collection}</{property}>"),
            });
        };
        Ok(Edit::InsertChild {
            parent,
            index: document.ast().element_children(parent).len(),
            source: native_fragment(document, parent, &source)?,
        })
    }
}

fn transition_markup(
    target: &XamlTransitionTarget,
    timing: &PropertyTransitionTiming,
) -> Result<String> {
    validate_target(target)?;
    let duration = timing.duration.unwrap_or(0.2);
    let delay = timing.delay.unwrap_or(0.0);
    timing::validate_clock_range(native_ticks(duration)?, native_ticks(delay)?)?;
    let mut attributes = format!(
        " Property=\"{}\" Duration=\"{}\"",
        escape(&target.property),
        time_string(duration)
    );
    if timing.delay.is_some() {
        attributes.push_str(&format!(" Delay=\"{}\"", time_string(delay)));
    }
    for (prefix, uri) in &target.namespaces {
        attributes.push_str(&format!(" xmlns:{prefix}=\"{}\"", escape(uri)));
    }
    let kind = target.type_name;
    if let Some(easing) = &timing.easing {
        easing.validate()?;
        if let Some(spring) = timing::spring_markup(easing) {
            return Ok(format!(
                "<{kind}{attributes}><{kind}.Easing>{spring}</{kind}.Easing></{kind}>"
            ));
        }
        attributes.push_str(&format!(" Easing=\"{}\"", escape(&easing.to_xaml())));
    }
    Ok(format!("<{kind}{attributes}/>"))
}

fn validate_target(target: &XamlTransitionTarget) -> Result<()> {
    require(
        PROPERTY_TRANSITION_TYPES.contains(&target.type_name),
        "Choose a built-in property transition class",
    )?;
    require(!target.property.is_empty(), "Choose a registered property")?;
    require(
        target.namespaces.iter().all(|(prefix, _)| {
            !prefix.is_empty()
                && prefix
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        }),
        "The property namespace prefix is invalid",
    )
}

fn retarget(document: &Document, node: &Node, target: &XamlTransitionTarget) -> Result<Vec<Edit>> {
    validate_target(target)?;
    let attribute = node
        .attribute_info("Property")
        .expect("the entry has a literal property");
    let mut edits = Vec::new();
    if attribute.value != target.property {
        edits.push(Edit::SetAttribute {
            node: node.id,
            name: "Property".into(),
            value: target.property.clone(),
        });
    }
    for (prefix, uri) in &target.namespaces {
        edits.push(Edit::SetAttribute {
            node: node.id,
            name: format!("xmlns:{prefix}"),
            value: uri.clone(),
        });
    }
    if node.local_name() == target.type_name {
        return Ok(edits);
    }

    // Stage attribute updates first so name and property-element spans come
    // from the same candidate, including any namespace declarations.
    let mut candidate = document.clone();
    candidate
        .apply("Prepare transition property", edits.clone())
        .map_err(core_error)?;
    let node = candidate
        .ast()
        .node(node.id)
        .expect("attribute edits keep the entry identity");
    let source = candidate.valid_source();
    let mut patches = Vec::new();
    let rename = |name: &str, suffix: &str| {
        name.split_once(':').map_or_else(
            || format!("{}{suffix}", target.type_name),
            |(prefix, _)| format!("{prefix}:{}{suffix}", target.type_name),
        )
    };
    rename_node(node, rename(&node.name, ""), &mut patches);
    for property in children(&candidate, node) {
        if property.kind == NodeKind::PropertyElement && native_animation_namespace(property) {
            if let Some((owner, member)) = property.local_name().split_once('.') {
                if owner == node.local_name() {
                    rename_node(
                        property,
                        rename(&property.name, &format!(".{member}")),
                        &mut patches,
                    );
                }
            }
        }
    }
    let original = &source[node.span.range()];
    for patch in &mut patches {
        patch.span = SourceSpan::new(
            patch.span.start - node.span.start,
            patch.span.end - node.span.start,
        );
    }
    let replacement = TextPatch::apply_all(original, patches).map_err(core_error)?;
    edits.push(Edit::Text {
        span: node.span,
        replacement,
    });
    Ok(edits)
}

fn rename_node(node: &Node, name: String, patches: &mut Vec<TextPatch>) {
    if let Some(closing) = node.closing_span {
        let start = closing.start + 2;
        patches.push(TextPatch {
            span: SourceSpan::new(start, start + node.name.len()),
            replacement: name.clone(),
        });
    }
    patches.push(TextPatch {
        span: node.name_span,
        replacement: name,
    });
}
