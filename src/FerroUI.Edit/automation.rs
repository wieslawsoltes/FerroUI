use crate::TextEditor;
use crate::text_editor::EditTarget;
use ferroui_base::{FerroObjectImpl, FerroObjectImplExt, Ref, ferro_class, instantiate};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{
    ITextProvider, ITextProvider2, IValueProvider, ProviderError, TextProviderError,
};
use ferroui_edit_core::document::DocumentEdit;
use std::rc::Rc;

mod text;
pub(crate) use text::merge_ranges;

#[repr(C)]
pub struct TextEditorAutomationPeer {
    base: ControlAutomationPeer,
    links: text::HyperlinkPeers,
}
ferro_class!(TextEditorAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TextEditorAutomationPeer {
    interfaces: [
        Rc<dyn IValueProvider> => TextEditorAutomationPeer::as_value_provider,
        Rc<dyn ITextProvider> => TextEditorAutomationPeer::as_text_provider,
        Rc<dyn ITextProvider2> => TextEditorAutomationPeer::as_text_provider2,
    ]
});
impl FerroObjectImpl for TextEditorAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        if let Some(editor) = this.try_editor() {
            let weak = this.to_ref().downgrade();
            let subscription = editor.text_area().visual_lines_changed(move |_, lines| {
                let Some(peer) = weak.upgrade() else { return };
                let Some(editor) = peer.try_editor() else { return };
                if peer.links.update_viewport(&peer, &editor, lines) {
                    peer.raise_children_changed_event();
                }
            });
            this.links.viewport_subscription.replace(Some(subscription));
        }
    }
}
impl ControlAutomationPeerImpl for TextEditorAutomationPeer {}
impl AutomationPeerImpl for TextEditorAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Edit
    }

    fn get_placeholder_text_core(this: &Self) -> Option<String> {
        this.try_editor().and_then(|editor| editor.watermark())
    }

    fn get_or_create_children_core(this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        let children = Self::parent_get_or_create_children_core(this);
        let Some(editor) = this.try_editor() else { return children };
        let Ok(lines) = editor.text_area().visible_editor_lines() else { return children };
        let links = this.links.peers_in_viewport(this, &editor, &lines);
        if links.is_empty() {
            children
        } else {
            Rc::new(children.iter().cloned().chain(links).collect())
        }
    }

    fn has_keyboard_focus_core(this: &Self) -> bool {
        this.try_editor().is_some_and(|editor| editor.text_area().is_focused())
    }

    fn is_keyboard_focusable_core(this: &Self) -> bool {
        this.try_editor().is_some_and(|editor| editor.text_area().focusable())
    }

    fn set_focus_core(this: &Self) {
        if let Some(editor) = this.try_editor() {
            editor.text_area().focus();
        }
    }
}
impl TextEditorAutomationPeer {
    pub fn new(owner: &TextEditor) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            links: text::HyperlinkPeers::default(),
        })
    }
    pub fn owner(&self) -> Ref<TextEditor> {
        self.try_editor().expect("editor peer owner")
    }
    fn try_editor(&self) -> Option<Ref<TextEditor>> {
        ControlAutomationPeer::try_owner(self)?.cast()
    }
    fn as_value_provider(peer: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(EditorProvider(peer))
    }
    fn as_text_provider(peer: Ref<Self>) -> Rc<dyn ITextProvider> {
        Rc::new(EditorProvider(peer))
    }
    fn as_text_provider2(peer: Ref<Self>) -> Rc<dyn ITextProvider2> {
        Rc::new(EditorProvider(peer))
    }
}
struct EditorProvider(Ref<TextEditorAutomationPeer>);
impl EditorProvider {
    fn editor(&self) -> Result<Ref<TextEditor>, TextProviderError> {
        self.0
            .try_editor()
            .ok_or(TextProviderError::ElementNotAvailable)
    }
}
impl IValueProvider for EditorProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        self.0.try_editor().is_none_or(|owner| owner.is_read_only() || owner.document().is_read_only())
    }
    fn value(&self) -> Option<String> {
        self.0.try_editor().map(|owner| owner.text())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        let owner = self.editor()?;
        if owner.is_read_only() || owner.document().is_read_only() || !owner.is_effectively_enabled() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Editor is read-only or disabled",
            )));
        }
        let edit = DocumentEdit::new(0, owner.document().text_length(), value.unwrap_or_default());
        EditTarget::new(&owner).apply(&owner, &[edit])?;
        Ok(())
    }
}
