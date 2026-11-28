use super::units::{element_groups, link};
use super::{TextContext, TextRange};
use crate::link_element_generator::builtin_link_markers;
use crate::{TextDocument, TextEditor, VisibleEditorLine, VisualLine, VisualLineLinkText};
use ferroui_base::reactive::Subscription;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Rect, Ref, WeakRef};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl,
};
use ferroui_controls::automation::provider::{
    IInvokeProvider, ITextChildProvider, ProviderError, TextProviderError,
};
use ferroui_controls::automation::ElementNotEnabledError;
use ferroui_edit_core::document::AnchorSegment;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use url::Url;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Link {
    span: Range<usize>,
    uri: Url,
    target: Option<String>,
}

fn links_on_line(line: &VisualLine) -> impl Iterator<Item = Link> + '_ {
    let offset = line.first_document_line().offset();
    element_groups(line)
        .filter_map(move |group| {
            let first = link(group.first()?.as_ref())?;
            let start = offset + group.first()?.data().relative_text_offset();
            let end = offset + group.last()?.data().end_relative_text_offset();
            Some(Link {
                span: start..end,
                uri: first.navigate_uri.clone()?,
                target: first.target_name.clone(),
            })
        })
        .filter(|link| !link.span.is_empty())
}

pub(super) fn link_spans_on_line(line: &VisualLine) -> impl Iterator<Item = Range<usize>> + '_ {
    links_on_line(line).map(|link| link.span)
}

fn can_generate_links(editor: &TextEditor) -> bool {
    let options = editor.options();
    options.enable_hyperlinks || options.enable_email_hyperlinks
        || editor.text_area().has_custom_visual_line_construction()
}

#[derive(Default)]
pub(in crate::automation) struct HyperlinkPeers {
    known: RefCell<Vec<WeakRef<EditorHyperlinkAutomationPeer>>>,
    visible: RefCell<Vec<Ref<AutomationPeer>>>,
    pub(in crate::automation) viewport_subscription: RefCell<Option<Subscription>>,
}

impl HyperlinkPeers {
    fn resolve(
        &self,
        context: &TextContext,
        document: &TextDocument,
        links: impl IntoIterator<Item = Link>,
    ) -> Result<Vec<(Ref<EditorHyperlinkAutomationPeer>, Range<usize>)>, ProviderError> {
        let mut indexed = HashMap::new();
        self.known.borrow_mut().retain(|weak| {
            let Some(peer) = weak
                .upgrade()
                .filter(|peer| peer.context.epoch == context.epoch)
            else {
                return false;
            };
            let Ok(key) = peer.anchored_link() else {
                return false;
            };
            indexed.insert(key, peer);
            true
        });

        let mut result = Vec::new();
        for link in links {
            let span = link.span.clone();
            let peer = if let Some(peer) = indexed.get(&link) {
                peer.clone()
            } else {
                let peer =
                    EditorHyperlinkAutomationPeer::new(context.clone(), document, link.clone())?;
                self.known.borrow_mut().push(peer.downgrade());
                indexed.insert(link, peer.clone());
                peer
            };
            result.push((peer, span));
        }
        Ok(result)
    }

    pub(in crate::automation) fn update_viewport(
        &self,
        owner: &super::TextEditorAutomationPeer,
        editor: &TextEditor,
        lines: &[VisibleEditorLine],
    ) -> bool {
        let peers = self.peers_in_viewport(owner, editor, lines);
        if *self.visible.borrow() == peers {
            return false;
        }
        self.visible.replace(peers);
        true
    }

    pub(in crate::automation) fn peers_in_viewport(
        &self,
        owner: &super::TextEditorAutomationPeer,
        editor: &TextEditor,
        lines: &[VisibleEditorLine],
    ) -> Vec<Ref<AutomationPeer>> {
        let context = TextContext {
            peer: owner.to_ref().downgrade(),
            epoch: editor.document_epoch(),
        };
        let links = lines.iter().flat_map(|row| links_on_line(&row.line));
        self.resolve(&context, &editor.document(), links)
            .unwrap_or_default()
            .into_iter()
            .map(|(peer, _)| peer.upcast())
            .collect()
    }
}

impl TextContext {
    fn resolve_links(
        &self,
        document: &TextDocument,
        links: impl IntoIterator<Item = Link>,
    ) -> Result<Vec<(Ref<AutomationPeer>, Range<usize>)>, ProviderError> {
        self.editor()?;
        let owner = self
            .peer
            .upgrade()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        Ok(owner
            .links
            .resolve(self, document, links)?
            .into_iter()
            .map(|(peer, span)| (peer.upcast(), span))
            .collect())
    }

    pub(super) fn links_in_range(
        &self,
        editor: &TextEditor,
        span: &Range<usize>,
        document_scope: bool,
    ) -> Result<Vec<(Ref<AutomationPeer>, Range<usize>)>, ProviderError> {
        if span.is_empty() {
            return Ok(Vec::new());
        }
        let area = editor.text_area();
        let custom = area.has_custom_visual_line_construction();
        let options = editor.options();
        if !custom && !options.enable_hyperlinks && !options.enable_email_hyperlinks {
            return Ok(Vec::new());
        }
        let document = editor.document();
        let version = document.version();
        let mut markers = if custom {
            None
        } else {
            Some(builtin_link_markers(
                &document.create_snapshot(), span.clone(),
                options.enable_hyperlinks, options.enable_email_hyperlinks,
            )?)
        };
        let mut links = Vec::new();
        let mut next = span.start;
        while next < span.end {
            let offset = if let Some(markers) = &mut markers {
                let Some(offset) = markers.find(|&marker| marker >= next) else { break };
                offset
            } else {
                next
            };
            let (line, successor) = area.automation_scan_line(&document, offset)?;
            super::range::check_version(&document, version.clone())?;
            self.editor()?;
            if let Some(line) = line {
                links.extend(links_on_line(&line).filter(|link| {
                    span.start <= link.span.start
                        && link.span.end <= span.end
                        && (document_scope || link.span != *span)
                }));
            }
            if successor <= offset {
                return Err(TextProviderError::ElementNotAvailable.into());
            }
            next = successor;
        }
        if links.is_empty() {
            Ok(Vec::new())
        } else {
            self.resolve_links(&document, links)
        }
    }

    pub(super) fn enclosing_link(
        &self,
        editor: &TextEditor,
        span: &Range<usize>,
    ) -> Result<Option<Ref<AutomationPeer>>, ProviderError> {
        let document = editor.document();
        if span.start >= document.text_length() || !can_generate_links(editor) {
            return Ok(None);
        }
        let version = document.version();
        let (line, _) = editor.text_area().automation_scan_line(&document, span.start)?;
        super::range::check_version(&document, version)?;
        self.editor()?;
        let Some(line) = line else { return Ok(None) };
        let candidate = links_on_line(&line).find(|link| {
            if span.is_empty() {
                link.span.start < span.start && span.start < link.span.end
            } else {
                link.span.start <= span.start && span.end <= link.span.end
            }
        });
        let Some(link) = candidate else {
            return Ok(None);
        };
        Ok(self
            .resolve_links(&document, [link])?
            .pop()
            .map(|(peer, _)| peer))
    }
}

/// The peer of one logical link generated by a text editor visual line.
#[repr(C)]
pub(super) struct EditorHyperlinkAutomationPeer {
    base: AutomationPeer,
    context: TextContext,
    anchors: AnchorSegment,
    uri: Url,
    target: Option<String>,
}

ferro_class!(EditorHyperlinkAutomationPeer: AutomationPeer);
ferroui_base::ferro_class_info!(EditorHyperlinkAutomationPeer {
    interfaces: [
        Rc<dyn IInvokeProvider> => EditorHyperlinkAutomationPeer::as_invoke_provider,
        Rc<dyn ITextChildProvider> => EditorHyperlinkAutomationPeer::as_text_child_provider,
    ]
});

impl FerroObjectImpl for EditorHyperlinkAutomationPeer {}

impl EditorHyperlinkAutomationPeer {
    fn new(
        context: TextContext,
        document: &TextDocument,
        link: Link,
    ) -> Result<Ref<Self>, ProviderError> {
        Ok(instantiate(Self {
            base: AutomationPeer::construct(),
            context,
            anchors: AnchorSegment::new(document, link.span.start, link.span.len())?,
            uri: link.uri,
            target: link.target,
        }))
    }

    fn anchored_link(&self) -> Result<Link, ProviderError> {
        let span = self.anchors.offset()?..self.anchors.end_offset()?;
        if span.is_empty() {
            return Err(TextProviderError::ElementNotAvailable.into());
        }
        Ok(Link {
            span,
            uri: self.uri.clone(),
            target: self.target.clone(),
        })
    }

    fn current(&self) -> Result<(Ref<TextEditor>, Link), ProviderError> {
        let editor = self.context.editor()?;
        let document = editor.document();
        let version = document.version();
        let key = self.anchored_link()?;
        let (line, _) = editor.text_area().automation_scan_line(&document, key.span.start)?;
        super::range::check_version(&document, version)?;
        self.context.editor()?;
        if !line.is_some_and(|line| links_on_line(&line).any(|link| link == key)) {
            return Err(TextProviderError::ElementNotAvailable.into());
        }
        Ok((editor, key))
    }

    pub(super) fn belongs_to(&self, owner: &Ref<super::TextEditorAutomationPeer>) -> bool {
        self.context.peer.upgrade().as_ref() == Some(owner)
    }

    pub(super) fn text_range(&self) -> Result<TextRange, ProviderError> {
        let (editor, link) = self.current()?;
        self.context.make_range(&editor.document(), link.span)
    }

    fn as_invoke_provider(peer: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(HyperlinkProvider(peer))
    }

    fn as_text_child_provider(peer: Ref<Self>) -> Rc<dyn ITextChildProvider> {
        Rc::new(HyperlinkProvider(peer))
    }
}

impl AutomationPeerImpl for EditorHyperlinkAutomationPeer {
    fn is_element_available_core(this: &Self) -> bool {
        this.current().is_ok()
    }
    fn bring_into_view_core(this: &Self) {
        if let Ok(range) = this.text_range() {
            let _ = range.scroll_into_view(true);
        }
    }
    fn get_accelerator_key_core(_this: &Self) -> Option<String> {
        None
    }
    fn get_access_key_core(_this: &Self) -> Option<String> {
        None
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Hyperlink
    }
    fn get_automation_id_core(_this: &Self) -> Option<String> {
        None
    }
    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        this.text_range()
            .and_then(|range| range.get_bounding_rectangles())
            .unwrap_or_default()
            .into_iter()
            .reduce(|a, b| a.union(b))
            .unwrap_or_default()
    }
    fn get_or_create_children_core(_this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        Rc::default()
    }
    fn get_class_name_core(_this: &Self) -> String {
        "Hyperlink".to_owned()
    }
    fn get_labeled_by_core(_this: &Self) -> Option<Ref<AutomationPeer>> {
        None
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let (editor, link) = this.current().ok()?;
        editor
            .document()
            .get_text(link.span.start, link.span.len())
            .ok()
    }
    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.context.peer.upgrade().map(|peer| peer.upcast())
    }
    fn get_automation_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.get_parent()?.get_automation_root()
    }
    fn get_visual_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.get_parent()?.get_visual_root()
    }
    fn to_screen_core(this: &Self, rect: Rect) -> Option<Rect> {
        this.get_parent()?.to_screen(rect)
    }
    fn has_keyboard_focus_core(_this: &Self) -> bool {
        false
    }
    fn is_content_element_core(_this: &Self) -> bool {
        true
    }
    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
    fn is_enabled_core(this: &Self) -> bool {
        this.current()
            .is_ok_and(|(editor, _)| editor.is_effectively_enabled())
    }
    fn is_keyboard_focusable_core(_this: &Self) -> bool {
        false
    }
    fn is_offscreen_core(this: &Self) -> bool {
        this.get_bounding_rectangle().is_empty()
    }
    fn set_focus_core(_this: &Self) {}
    fn show_context_menu_core(this: &Self) -> bool {
        this.get_parent()
            .is_some_and(|parent| parent.show_context_menu())
    }
    fn try_set_parent(_this: &Self, _parent: Option<Ref<AutomationPeer>>) -> bool {
        false
    }
}

struct HyperlinkProvider(Ref<EditorHyperlinkAutomationPeer>);

impl IInvokeProvider for HyperlinkProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        let (editor, link) = self
            .0
            .current()
            .map_err(|_| ElementNotEnabledError::new())?;
        if !editor.is_effectively_enabled() {
            return Err(ElementNotEnabledError::new());
        }
        VisualLineLinkText::open_uri(&editor.text_area(), link.uri, link.target);
        Ok(())
    }
}

impl ITextChildProvider for HyperlinkProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn text_container(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.0.current()?;
        self.0
            .get_parent()
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    fn text_range(&self) -> Result<TextRange, ProviderError> {
        self.0.text_range()
    }
}
