use crate::animation::RsxAnimationIssue;
use crate::behaviors::BehaviorFields;
use crate::blocks::{blocks, contains_trivia, keep_comments, SourceDelimiters};
use crate::catalog::Children;
use crate::child_collection::{ChildEnvelope, CollectionSource, InlineChildren, control_expression};
use crate::columns::{ColumnSource, collection_member, table_kind};
use crate::commands::CommandFields;
use crate::item_templates::TemplateFields;
use crate::literal;
use crate::local_rust::LocalRust;
use crate::markup::{RsxPreviewMarkup, serialize};
use crate::property_transitions::RsxPropertyTransitions;
use crate::source_array::ArraySlot;
use crate::registered::{Class, RsxCatalog};
use crate::source_edits::RsxEvent;
use crate::states::{self, ProjectedStates};
use crate::styles::{RsxStyle, RsxStyleIssue, projected_styles};
use crate::tree::{Element, MAX_DEPTH, replacement};
use crate::xaml::XamlTree;
use ferroui_designer_animation::BehaviorModel;
use ferroui_designer_core::{Document, NodeId, NodeKind, SourceSpan, TextPatch};
use ferroui_designer_project::RustModuleSources;
use ferroui_designer_tools::TableKind;
use ferroui_view_syntax::{Attribute, Node};
use proc_macro2::TokenStream;
use std::collections::{BTreeMap, BTreeSet};
use std::cell::OnceCell;
use syn::ext::IdentExt;
use syn::spanned::Spanned;

mod children;
mod property_fields;
use property_fields::PropertyField;

/// The original source ranges of one projected control, in XAML element order.
pub struct RsxNode {
    pub span: SourceSpan,
    pub name: SourceSpan,
    pub properties: BTreeMap<String, SourceSpan>,
    pub expressions: BTreeMap<String, SourceSpan>,
    pub events: BTreeMap<String, RsxEvent>,
    pub(crate) fields: BTreeMap<String, SourceSpan>,
    pub(crate) commands: CommandFields,
    pub(crate) class: Class,
    pub(crate) open: usize,
    pub(crate) close: usize,
    pub(crate) child_indices: Vec<usize>,
    /// Token starts of children omitted from the static projection, in source order.
    pub(crate) opaque_children: Vec<usize>,
    pub(crate) computed_child_setter: bool,
    pub(crate) child_collection: Option<CollectionSource>,
    pub(crate) envelope: Option<ChildEnvelope>,
    pub(crate) style_slot: ArraySlot,
    pub(crate) state_slot: ArraySlot,
    pub(crate) behaviors: BehaviorFields,
    pub(crate) state_triggers: crate::state_triggers::StateTriggers,
    pub(crate) property_transitions: Option<RsxPropertyTransitions>,
    pub(crate) columns: Option<ColumnSource>,
    pub(crate) templates: TemplateFields,
    pub(crate) subtree: ProjectedSubtree,
}

#[derive(Default)]
pub(crate) struct ProjectedSubtree {
    /// Exclusive end of this control's subtree in the projection's preorder.
    pub(crate) end: usize,
    pub(crate) omitted: usize,
    pub(crate) animation_issues: usize,
    pub(crate) style_issues: usize,
}

impl RsxNode {
    /// Whether a property has a static representation in both source languages.
    pub fn supports_literal(&self, name: &str) -> bool {
        self.class.literal(name).is_some()
    }

    pub fn can_set_image_source(&self) -> bool {
        self.class.literal("Source") == Some(literal::Literal::EmbeddedImage)
            && !self.expressions.contains_key("Source")
    }

    /// Registered raster properties with a missing or mapped authored value.
    pub fn editable_image_properties(&self) -> Vec<String> {
        self.class.image_property_names()
            .filter(|name| !self.expressions.contains_key(*name))
            .map(str::to_owned)
            .collect()
    }

    pub fn supports_events(&self) -> bool {
        self.class.mapped_events()
    }

    pub fn editable_children(&self) -> bool {
        let slot = self.class.children();
        slot != Children::None
            && (slot == Children::Panel || self.opaque_children.is_empty())
            && !self.expressions.contains_key("Content")
            && !self.computed_child_setter
    }

    pub(crate) fn child_span(&self) -> SourceSpan {
        self.envelope.map_or(self.span, ChildEnvelope::span)
    }
}

/// Static markup plus the source anchors needed to edit its original Rust block.
pub struct RsxProjection<'a> {
    pub(crate) source: &'a str,
    pub(crate) catalog: &'a RsxCatalog,
    pub(crate) modules: Option<&'a RustModuleSources>,
    pub(crate) tree: Element,
    pub nodes: Vec<RsxNode>,
    pub blocks: Vec<SourceSpan>,
    /// The selected invocation, including its name and delimiters.
    pub invocation: SourceSpan,
    /// An incomplete projection can still edit its mapped literals in place.
    pub omissions: Vec<String>,
    pub omitted_count: usize,
    pub animation_issues: Vec<RsxAnimationIssue>,
    pub style_issues: Vec<RsxStyleIssue>,
    inner_trivia: bool,
    preserve_asset_expressions: bool,
    pub(crate) styles: BTreeMap<usize, Vec<RsxStyle>>,
    pub(crate) states: BTreeMap<usize, ProjectedStates>,
}

impl RsxProjection<'_> {
    pub fn module_sources(&self) -> Option<&RustModuleSources> {
        self.modules
    }

    /// Revalidate the captured Cargo target against a draft of the active buffer.
    pub fn draft_module_sources(&self, source: &str) -> Result<Option<RustModuleSources>, String> {
        self.modules.map(|modules| {
            modules.with_file_text(&modules.active().uri, source)
                .ok_or_else(|| "The Rust edit changes the accepted Cargo module layout".into())
        }).transpose()
    }

    /// Carry the active invocation's start across checked edits to this file.
    pub fn invocation_start_after(&self, patches: &[TextPatch]) -> Result<usize, String> {
        let before = self.invocation.start;
        let mut after = before;
        for patch in patches.iter().filter(|patch| patch.span.start < before) {
            if patch.span.end > before {
                return Err("The edit crosses the selected RSX invocation".into());
            }
            after = after.checked_sub(patch.span.len()).and_then(|anchor| anchor.checked_add(patch.replacement.len()))
                .ok_or("The selected RSX invocation moved outside the buffer")?;
        }
        Ok(after)
    }

    pub fn xaml(&self) -> String {
        self.preview_markup().source
    }

    pub fn preview_markup(&self) -> RsxPreviewMarkup<'_> {
        serialize(&self.tree, &self.nodes, &self.styles, &self.states, &crate::state_triggers::TriggerMarkup::project(self))
    }

    pub fn has_projected_animations(&self) -> bool {
        self.style_definitions().iter().any(|(_, style)| !style.clips.is_empty())
    }

    pub fn has_projected_styles(&self) -> bool {
        !self.styles.is_empty()
    }

    pub fn has_projected_states(&self) -> bool {
        !self.states.is_empty()
    }

    fn has_state_triggers(&self) -> bool {
        self.nodes.iter().any(|node| !node.state_triggers.is_empty())
    }

    pub fn property_transitions(&self, owner: usize) -> Option<&RsxPropertyTransitions> {
        self.nodes.get(owner)?.property_transitions.as_ref()
    }

    /// Resolve an authored control ID from this projection's accepted XAML document.
    pub fn control_index(&self, document: &Document, node: NodeId) -> Option<usize> {
        let owner = document.ast().node(node)?;
        self.preview_markup().controls.binary_search(&owner.span.start).ok()
    }

    fn has_property_transitions(&self) -> bool {
        self.nodes.iter().any(|node| node.property_transitions.is_some())
    }

    pub fn has_projected_columns(&self) -> bool {
        self.nodes.iter().any(|node| node.columns.as_ref().is_some_and(ColumnSource::projected))
    }

    pub fn matches_xaml(&self, source: &str) -> Result<bool, String> {
        let document = Document::new(source).map_err(|error| error.to_string())?;
        let counterpart = XamlTree::counterpart(&document, self.catalog)?;
        Ok(self.tree == counterpart.tree && self.matches_behavior_models(&counterpart.behavior_models()))
    }

    /// Structural conversion can replace a complete tree with no interior comments.
    pub fn can_replace_structure(&self) -> bool {
        self.omitted_count == 0 && !self.inner_trivia && !self.has_projected_styles() && !self.has_projected_states()
            && !self.has_state_triggers() && !self.has_projected_columns() && !self.has_property_transitions()
            && !self.has_projected_behaviors()
            && !self.has_explicit_child_lists() && !self.preserve_asset_expressions
    }

    pub fn set_property(&self, index: usize, name: &str, value: &str) -> Result<TextPatch, String> {
        let node = self.nodes.get(index).ok_or("Unknown RSX element")?;
        let span = *node
            .properties
            .get(name)
            .ok_or("Edit an authored literal property")?;
        let outside_static_values = || format!("{value:?} is outside the static RSX values for {name}");
        let codec = node.class.literal(name).ok_or_else(outside_static_values)?;
        if codec == literal::Literal::EmbeddedImage {
            return crate::image_source::edit(self.source, span, value, self.modules)
                .ok_or_else(outside_static_values);
        }
        let original = &self.source[span.range()];
        if let Some(shape) = codec.native_list_shape() {
            return Ok(TextPatch { span, replacement: literal::edit_native_list(shape, original, value)? });
        }
        let mut replacement = codec.rust(value).ok_or_else(outside_static_values)?;
        if original.contains('/') {
            let comments = keep_comments(original)?;
            replacement.insert_str(0, &comments);
        }
        Ok(TextPatch { span, replacement })
    }

    /// Apply edited XAML to the selected RSX tree. Changes to mapped properties keep
    /// the original child declarations. Structural edits retain the rest of the file.
    pub fn apply_xaml(&self, draft: &str) -> Result<Vec<TextPatch>, String> {
        if self.has_projected_columns() {
            return Err("Edit authored Rust column collections in Columns or in source".into());
        }
        if self.has_property_transitions() {
            return Err("Edit the authored transition collection in Rust source".into());
        }
        if self.has_projected_states() || self.has_state_triggers() {
            return Err("Edit mapped Rust state declarations in States or in the source editor".into());
        }
        if self.has_projected_animations() {
            return Err("Edit mapped animation keys in Timeline and other Rust values in source or Properties".into());
        }
        if self.has_projected_styles() {
            return Err("Edit mapped Rust style builders in Styles & Themes or in source".into());
        }
        if self.omitted_count != 0 {
            return Err(
                "Use the property inspector to edit literals in a partial RSX preview".into(),
            );
        }
        let document = Document::new(draft).map_err(|e| e.to_string())?;
        if document.ast().nodes().any(|node| {
            matches!(
                node.kind,
                NodeKind::Comment | NodeKind::ProcessingInstruction
            )
        }) {
            return Err("Keep comments and processing instructions in the authored format".into());
        }
        let draft = XamlTree::counterpart(&document, self.catalog)?;
        let models = draft.behavior_models();
        if let Some(changes) = self.tree.changes(&draft.tree) {
            let mut patches = changes
                .iter()
                .map(|change| self.edit_property(change.index, change.name, change.value))
                .collect::<Result<Vec<_>, _>>()?;
            patches.extend(self.patch_behavior_models(&models)?);
            crate::source_edits::merge_insertions(&mut patches);
            if !patches.is_empty() {
                let source = TextPatch::apply_all(self.source, patches.clone())
                    .map_err(|error| error.to_string())?;
                let modules = self.draft_module_sources(&source)?;
                let projected = self.catalog.project_with_modules(
                    &source,
                    self.invocation.start,
                    modules.as_ref(),
                )?;
                if projected.omitted_count != 0 || projected.tree != draft.tree || !projected.matches_behavior_models(&models) {
                    return Err("The property edits do not match the authored Rust view".into());
                }
            }
            return Ok(patches);
        }
        if self.has_projected_behaviors() || models.iter().any(|items| !items.is_empty()) {
            return Err("Use the canvas or Outline to arrange a view with behaviors".into());
        }
        if self.has_explicit_child_lists() {
            return Err("Use the canvas or Outline to edit the authored Rust child list".into());
        }
        if self.preserve_asset_expressions {
            return Err("Use the canvas, Outline, or Rust editor to keep the authored image expressions".into());
        }
        if self.inner_trivia {
            return Err("Edit RSX structure in the Rust editor to keep its comments".into());
        }
        Ok(vec![replacement(
            self.source,
            self.nodes[0].span,
            &draft.tree.rsx_body(),
        )])
    }

    /// Whether structural edits must keep explicit Rust child expressions or collections.
    pub fn has_explicit_child_lists(&self) -> bool {
        self.nodes.iter().any(|node| node.child_collection.is_some() || node.envelope.is_some())
    }
}

/// Selects the innermost `rsx!` enclosing the UTF-8 caret, or the first invocation.
pub fn project_rust(source: &str, caret: usize) -> Result<RsxProjection<'_>, String> {
    RsxCatalog::built_in().project(source, caret)
}

impl RsxCatalog {
    /// Project the selected invocation using the imported registry for its Cargo package.
    pub fn project<'a>(
        &'a self,
        source: &'a str,
        caret: usize,
    ) -> Result<RsxProjection<'a>, String> {
        self.project_with_modules(source, caret, None)
    }

    /// Use an accepted Cargo target snapshot to resolve authored declarations
    /// in the other modules of that crate.
    pub fn project_with_modules<'a>(
        &'a self,
        source: &'a str,
        caret: usize,
        modules: Option<&'a RustModuleSources>,
    ) -> Result<RsxProjection<'a>, String> {
        let modules = modules.filter(|modules| modules.active().source.as_ref() == source);
        let blocks = blocks(source)?;
        let block = blocks
            .iter()
            .filter(|b| b.span.start <= caret && caret <= b.span.end)
            .min_by_key(|b| b.span.len())
            .or_else(|| blocks.first())
            .ok_or("Open a Rust file containing a ferroui_view::rsx! invocation")?;
        let stream = block.group.stream();
        let (tree, state) = parse_body(source, stream.clone(), self, modules)?;
        let inner_trivia = contains_trivia(source, stream, state.nodes[0].span);
        Ok(RsxProjection {
            source,
            catalog: self,
            modules,
            tree,
            nodes: state.nodes,
            blocks: blocks.iter().map(|block| block.span).collect(),
            invocation: block.span,
            omissions: state.omissions,
            omitted_count: state.omitted_count,
            animation_issues: state.animation_issues,
            style_issues: state.style_issues,
            inner_trivia,
            preserve_asset_expressions: state.preserve_asset_expressions,
            styles: state.styles,
            states: state.states,
        })
    }
}

pub(crate) fn parse_draft(source: &str, catalog: &RsxCatalog) -> Result<(Element, Vec<Vec<BehaviorModel>>), String> {
    let tokens: TokenStream = source
        .parse()
        .map_err(|e| format!("Invalid RSX tokens: {e}"))?;
    let draft: syn::ExprMacro =
        syn::parse2(tokens.clone()).map_err(|e| format!("Expected an rsx! expression: {e}"))?;
    if draft
        .mac
        .path
        .segments
        .last()
        .is_none_or(|segment| segment.ident != "rsx")
    {
        return Err("Expected an rsx! expression".into());
    }
    let (tree, state) = parse_body(source, draft.mac.tokens, catalog, None)?;
    if !state.styles.is_empty() || !state.states.is_empty() || state.nodes.iter().any(|node| {
        node.property_transitions.is_some() || !node.state_triggers.is_empty()
            || node.columns.as_ref().is_some_and(ColumnSource::projected)
    }) {
        return Err("Edit native styles, transitions, animation, states, and columns in their authored scope".into());
    }
    if state.omitted_count != 0 {
        return Err(format!(
            "The RSX draft must be fully convertible: {}",
            state.omissions.join("; ")
        ));
    }
    if contains_trivia(source, tokens, SourceSpan::new(0, source.len())) {
        return Err("Keep comments in the authored format".into());
    }
    let models = state.nodes.iter().map(|node| node.behaviors.entries.iter().map(|entry| entry.model().clone()).collect()).collect();
    Ok((tree, models))
}

#[derive(Default)]
struct State<'a> {
    source: &'a str,
    nodes: Vec<RsxNode>,
    delimiters: SourceDelimiters,
    omissions: Vec<String>,
    omitted_count: usize,
    styles: BTreeMap<usize, Vec<RsxStyle>>,
    states: BTreeMap<usize, ProjectedStates>,
    animation_issues: Vec<RsxAnimationIssue>,
    style_issues: Vec<RsxStyleIssue>,
    local_rust: Option<LocalRust>,
    modules: Option<&'a RustModuleSources>,
    image_includes: OnceCell<Option<ferroui_designer_project::RustIncludeSyntax>>,
    preserve_asset_expressions: bool,
}

impl State<'_> {
    fn omit(&mut self, description: String) {
        self.omitted_count += 1;
        if self.omissions.len() < 8 && !self.omissions.contains(&description) {
            self.omissions.push(description);
        }
    }

    fn project_styles(&mut self, source: &str, index: usize, value: &syn::Expr, catalog: &RsxCatalog) {
        let slot = &mut self.nodes[index].style_slot;
        *slot = if matches!(slot, ArraySlot::Missing) {
            ArraySlot::from_expression(value)
        } else {
            ArraySlot::Unavailable
        };
        let modules = self.modules;
        let bindings = self.local_rust.get_or_insert_with(|| {
            LocalRust::parse(source, value.span().byte_range().start, modules)
        });
        let projected = projected_styles(source, value, catalog, bindings);
        let partial = !projected.issues.is_empty() || !projected.animation_issues.is_empty();
        self.style_issues.extend(projected.issues);
        self.animation_issues.extend(projected.animation_issues);
        if !projected.styles.is_empty() {
            self.styles.entry(index).or_default().extend(projected.styles);
        }
        if partial {
            let class = &self.nodes[index].class;
            self.omit(format!("{class}.styles expression"));
        }
    }

    fn project_property_transitions(
        &mut self,
        source: &str,
        index: usize,
        value: Option<&syn::Expr>,
        field: SourceSpan,
        catalog: &RsxCatalog,
    ) {
        let node = &mut self.nodes[index];
        let duplicate = node.expressions.insert(
            "Transitions".into(), value.map_or(field, crate::animation::syntax::span),
        ).is_some();
        node.property_transitions = value.filter(|_| !duplicate).and_then(|value| {
            let bindings = self.local_rust.get_or_insert_with(|| {
                LocalRust::parse(source, value.span().byte_range().start, self.modules)
            });
            RsxPropertyTransitions::read(value, &node.class, catalog, bindings)
        });
        if node.property_transitions.is_none() {
            let description = format!("{}.transitions expression", node.class);
            self.omit(description);
        }
    }

    fn element(
        &mut self,
        source: &str,
        node: &Node,
        depth: usize,
        catalog: &RsxCatalog,
    ) -> Result<Element, String> {
        if depth > MAX_DEPTH || self.nodes.len() >= 2000 {
            return Err("RSX previews support up to 2000 controls and 96 levels".into());
        }
        let Node::Element(node) = node else {
            return Err("A preview root must be a static FerroUI control".into());
        };
        let name = node.path.span().byte_range();
        let class = catalog
            .rust(&node.path)
            .ok_or("Use a FerroUI control or a registered RSX mapping")?;
        let mut result = Element::new(class.clone());
        let table = table_kind(&class);
        let collection = InlineChildren::read(node, &class);
        let index = self.nodes.len();
        let omitted_before = self.omitted_count;
        let animation_issues_before = self.animation_issues.len();
        let style_issues_before = self.style_issues.len();
        self.nodes.push(RsxNode {
            span: SourceSpan::new(name.start, node.braces.close().byte_range().end),
            name: SourceSpan::new(name.start, name.end),
            properties: BTreeMap::new(),
            expressions: BTreeMap::new(),
            events: BTreeMap::new(),
            fields: BTreeMap::new(),
            commands: CommandFields::default(),
            class: class.clone(),
            open: node.braces.open().byte_range().end,
            close: node.braces.close().byte_range().start,
            child_indices: Vec::new(),
            opaque_children: Vec::new(),
            computed_child_setter: false,
            child_collection: None,
            envelope: None,
            state_slot: if class.xaml_property(None, "visual_states").is_none() {
                ArraySlot::Missing
            } else {
                ArraySlot::Unavailable
            },
            state_triggers: crate::state_triggers::StateTriggers::default(),
            behaviors: BehaviorFields::new(class.xaml_property(None, "behaviors").is_none()),
            property_transitions: None,
            style_slot: if class.xaml_property(None, "styles").is_none() {
                ArraySlot::Missing
            } else {
                ArraySlot::Unavailable
            },
            columns: table.map(|_| ColumnSource::Missing),
            templates: TemplateFields::for_class(&class),
            subtree: ProjectedSubtree::default(),
        });
        let mut seen = BTreeSet::new();
        let mut state_fields = Vec::new();
        for (attribute_index, attribute) in node.attributes.iter().enumerate() {
            if collection.as_ref().is_some_and(|list| {
                list.attribute == attribute_index && self.set_child_collection(index, list, attribute)
            }) {
                continue;
            }
            let (owner, name, value) = match attribute {
                Attribute::Value {
                    owner, name, value, ..
                } => (owner, name, value),
                Attribute::Call { name, arguments } => {
                    let setter = name.unraw().to_string();
                    let argument = arguments.first().filter(|_| arguments.len() == 1);
                    if setter == "behaviors" && class.xaml_property(None, &setter).is_none() {
                        self.project_behaviors(index, argument, &class);
                        continue;
                    }
                    let name = crate::animation::syntax::span(name);
                    let field = self.delimiters.calls.get(&name.start).copied();
                    if setter == "state_trigger" && class.xaml_property(None, &setter).is_none() {
                        if !self.nodes[index].state_triggers.read(arguments.iter(), field) {
                            self.omit(format!("{class}.state_trigger expression"));
                        }
                        continue;
                    }
                    if self.project_command(index, &setter, false, PropertyField {
                        field, fallback: field.unwrap_or(name), expression: argument,
                    }) {
                        continue;
                    }
                    if let Some(styles) = argument.filter(|_| {
                        setter == "styles" && class.xaml_property(None, "styles").is_none()
                    }) {
                        self.project_styles(source, index, styles, catalog);
                        continue;
                    }
                    if setter == "transitions" && class.xaml_property(None, &setter).is_none() {
                        self.project_property_transitions(source, index, argument, field.unwrap_or(name), catalog);
                        continue;
                    }
                    self.project_template(index, &setter, false, argument, field);
                    if let Some(table) = table {
                        if self.project_columns(index, table, &setter, false, argument, name) {
                            continue;
                        }
                    }
                    self.nodes[index].computed_child_setter |= matches!(
                        setter.as_str(), "children" | "child" | "content" | "items" | "items_source"
                    );
                    if setter == "visual_states" {
                        self.nodes[index].state_slot = ArraySlot::Unavailable;
                    }
                    if setter == "styles" {
                        self.nodes[index].style_slot = ArraySlot::Unavailable;
                    }
                    if setter.starts_with("on_") {
                        let value = arguments.first().map_or(name, crate::animation::syntax::span);
                        self.nodes[index].events.insert(
                            setter,
                            RsxEvent {
                                value,
                                field: None,
                            },
                        );
                    } else if let Some(key) = class.xaml_property(None, &setter) {
                        self.project_property(
                            index, &mut result, &mut seen, key, source,
                            PropertyField { field, fallback: field.unwrap_or(name), expression: argument },
                        );
                        continue;
                    }
                    self.omit(format!("{class} builder call"));
                    continue;
                }
            };
            let setter = name.unraw().to_string();
            let owner = owner.as_ref().map(|owner| owner.unraw().to_string());
            if let Some(table) = table {
                if self.project_columns(index, table, &setter, owner.is_some(), Some(value), crate::animation::syntax::span(name)) {
                    continue;
                }
            }
            self.nodes[index].computed_child_setter |= owner.is_none()
                && matches!(setter.as_str(), "children" | "child" | "items" | "items_source");
            if owner.is_none() && setter == "visual_states" && class.xaml_property(None, &setter).is_none() {
                let slot = &mut self.nodes[index].state_slot;
                *slot = if matches!(slot, ArraySlot::Missing) {
                    ArraySlot::from_expression(value)
                } else {
                    ArraySlot::Unavailable
                };
                state_fields.push(value.as_ref());
                continue;
            }
            if owner.is_none() && setter == "behaviors" && class.xaml_property(None, &setter).is_none() {
                self.project_behaviors(index, Some(value), &class);
                continue;
            }
            let value_span = value.span().byte_range();
            let value_span = SourceSpan::new(value_span.start, value_span.end);
            let field = SourceSpan::new(name.span().byte_range().start, value_span.end);
            if owner.is_none() && setter == "transitions" && class.xaml_property(None, &setter).is_none() {
                self.project_property_transitions(source, index, Some(value), field, catalog);
                continue;
            }
            if self.project_command(index, &setter, owner.is_some(), PropertyField {
                field: Some(field), fallback: field, expression: Some(value),
            }) {
                continue;
            }
            if self.project_template(index, &setter, owner.is_some(), Some(value), owner.is_none().then_some(field)) {
                self.omit(format!("{class}.{setter} template"));
                continue;
            }
            if owner.is_none() && setter == "styles" && class.xaml_property(None, &setter).is_none() {
                self.project_styles(source, index, value, catalog);
                continue;
            }
            if class.mapped_events() && owner.is_none() && setter.starts_with("on_") {
                let field = (!self.nodes[index].events.contains_key(&setter)).then_some(field);
                self.nodes[index].events.insert(
                    setter.clone(),
                    RsxEvent {
                        value: value_span,
                        field,
                    },
                );
                self.omit(format!("{class}.{setter}"));
                continue;
            }
            let Some(key) = class.xaml_property(owner.as_deref(), &setter) else {
                self.omit(format!("{class}.{setter}"));
                continue;
            };
            let start = match attribute {
                Attribute::Value { owner: Some(owner), .. } => owner.span().byte_range().start,
                _ => field.start,
            };
            self.project_property(
                index, &mut result, &mut seen, key, source,
                PropertyField {
                    field: Some(SourceSpan::new(start, field.end)),
                    fallback: field,
                    expression: Some(value),
                },
            );
        }
        let placement = class.children();
        if let Some(collection) = collection.filter(|_| self.nodes[index].child_collection.is_some()) {
            self.project_child_collection(source, index, &mut result, collection, depth, catalog)?;
        }
        let text_property = class
            .text_property()
            .filter(|name| node.children.len() == 1 && !seen.contains(*name));
        for child in &node.children {
            if let (Some(name), Node::Text(value, _)) = (text_property, child) {
                let span = value.span().byte_range();
                let span = SourceSpan::new(span.start, span.end);
                self.nodes[index].fields.insert(name.into(), span);
                self.nodes[index].properties.insert(name.into(), span);
                result
                    .properties
                    .insert(name.into(), literal::text_value(&value.value()));
                continue;
            }
            let static_expression = match child {
                Node::Expr(value, start) => self.delimiters.blocks.get(&start.byte_range().start)
                    .copied()
                    .zip(control_expression(value)),
                _ => None,
            };
            match static_expression.as_ref().map_or(child, |(_, control)| control) {
                control @ Node::Element(element) if catalog.rust(&element.path).is_some()
                    && accepts_control(placement, &result) => {
                    let child_index = self.nodes.len();
                    result
                        .children
                        .push(self.element(source, control, depth + 1, catalog)?);
                    self.nodes[index].child_indices.push(child_index);
                    if let Some((span, _)) = static_expression {
                        self.nodes[child_index].envelope = Some(ChildEnvelope::Block(span));
                    }
                }
                _ => {
                    self.nodes[index].opaque_children.push(child_start(child));
                    self.omit(format!("{class} child"));
                }
            }
        }
        if !state_fields.is_empty() {
            let (groups, issues) = states::project(&state_fields, &result, index, catalog);
            if let Some(groups) = groups {
                self.states.insert(index, groups);
            }
            if !issues.is_empty() {
                self.nodes[index].state_slot = ArraySlot::Unavailable;
                self.animation_issues.extend(issues);
                self.omit(format!("{class}.visual_states expression"));
            }
        }
        self.nodes[index].subtree = ProjectedSubtree {
            end: self.nodes.len(),
            omitted: self.omitted_count - omitted_before,
            animation_issues: self.animation_issues.len() - animation_issues_before,
            style_issues: self.style_issues.len() - style_issues_before,
        };
        Ok(result)
    }

    fn project_behaviors(&mut self, index: usize, value: Option<&syn::Expr>, class: &Class) {
        let position = self.nodes[index].name.start;
        let rust = self.local_rust.get_or_insert_with(|| LocalRust::parse(self.source, position, self.modules));
        if !self.nodes[index].behaviors.read(value, rust) {
            self.omit(format!("{class}.behaviors expression"));
        }
    }

    fn project_template(
        &mut self,
        index: usize,
        setter: &str,
        qualified: bool,
        argument: Option<&syn::Expr>,
        field: Option<SourceSpan>,
    ) -> bool {
        let templates = &mut self.nodes[index].templates;
        templates.note_setter(setter, qualified);
        let Some(property) = templates.member(setter) else {
            return false;
        };
        let source = self.source;
        let modules = self.modules;
        let scope = argument.and_then(|value| self.local_rust.get_or_insert_with(|| {
            LocalRust::parse(source, value.span().byte_range().start, modules)
        }).source_scope(value));
        templates.record(property, argument, field, scope);
        true
    }

    fn project_command(
        &mut self,
        index: usize,
        setter: &str,
        qualified: bool,
        source: PropertyField<'_>,
    ) -> bool {
        let node = &mut self.nodes[index];
        let slots = node.class.commands();
        if slots.is_empty() {
            return false;
        }
        let Some(member) = slots.iter().flat_map(|slot| slot.members())
            .find(|member| member.setter.trim_start_matches("r#") == setter) else {
            node.commands.note_setter(setter);
            return false;
        };
        let expression = source.expression.map(crate::animation::syntax::span);
        let field = if qualified { None } else { source.field };
        node.commands.record(&member.property, expression, field);
        if !qualified && node.class.literal(&member.property).is_some() {
            return false;
        }
        node.expressions.insert(member.property.clone(), expression.unwrap_or(source.fallback));
        let reason = format!("{}.{setter} command expression", node.class);
        self.omit(reason);
        true
    }

    fn project_columns(
        &mut self, index: usize, table: TableKind, setter: &str, qualified: bool,
        value: Option<&syn::Expr>, name: SourceSpan,
    ) -> bool {
        let (member, property) = collection_member(table);
        let node = &mut self.nodes[index];
        let columns = node.columns.as_mut().expect("Mapped native table");
        let source = value.map_or(name, crate::animation::syntax::span);
        if setter == member {
            node.expressions.insert(property.into(), source);
            let projected = if let (false, Some(value)) = (qualified, value) {
                let source = self.source;
                let modules = self.modules;
                let scope = self.local_rust.get_or_insert_with(|| {
                    LocalRust::parse(source, value.span().byte_range().start, modules)
                }).source_scope(value);
                columns.read(table, value, scope)
            } else {
                *columns = ColumnSource::Computed(source);
                false
            };
            if !projected {
                let message = format!("{}.{property} expression", node.class);
                self.omit(message);
            }
            return true;
        }
        if matches!(setter, "prop" | "set" | "with")
            || (table == TableKind::TreeDataGrid && matches!(setter, "source" | "model"))
        {
            *columns = ColumnSource::Computed(source);
        }
        false
    }
}

fn child_start(child: &Node) -> usize {
    match child {
        Node::Element(element) => element.path.span(),
        Node::Text(value, _) => value.span(),
        Node::Expr(_, open) | Node::If(_, _, _, open) | Node::For(_, _, _, open) => *open,
    }.byte_range().start
}

pub(crate) fn accepts_control(placement: Children, element: &Element) -> bool {
    placement == Children::Panel
        || (matches!(placement, Children::Single | Children::Content)
            && element.children.is_empty()
            && !element.properties.contains_key("Content"))
}

fn parse_body<'a>(
    source: &'a str,
    tokens: TokenStream,
    catalog: &RsxCatalog,
    modules: Option<&'a RustModuleSources>,
) -> Result<(Element, State<'a>), String> {
    let nodes = ferroui_view_syntax::parse(tokens.clone()).map_err(|e| format!("Invalid RSX: {e}"))?;
    let [root] = nodes.as_slice() else {
        return Err("RSX preview needs one root control; wrap fragments in a panel".into());
    };
    let mut state = State {
        source,
        modules,
        delimiters: SourceDelimiters::read(tokens),
        ..State::default()
    };
    let tree = state.element(source, root, 0, catalog)?;
    if state.nodes.iter().any(|node| !node.behaviors.entries.is_empty()) {
        let mut names = BTreeMap::new();
        let mut pending = vec![&tree];
        while let Some(control) = pending.pop() {
            if let Some(name) = control.properties.get("Name") {
                *names.entry(name.as_str()).or_insert(0) += 1;
            }
            pending.extend(&control.children);
        }
        for index in 0..state.nodes.len() {
            if !state.nodes[index].behaviors.retain_resolved_targets(&names) {
                state.omit("Behavior with a missing or ambiguous control name".into());
            }
        }
    }
    Ok((tree, state))
}
