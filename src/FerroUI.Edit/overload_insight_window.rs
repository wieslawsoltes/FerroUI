use crate::completion_window_base::CompletionWindowBaseImpl;
use crate::completion_window_base::CompletionWindowBaseImplExt;
use crate::insight_window::InsightWindow;
use crate::overload_provider::{IOverloadProvider, OverloadProviderRef};
use crate::overload_viewer::OverloadViewer;
use crate::TextEditor;
use ferroui_base::input::{InputElementImpl, InputElementImplExt, Key, KeyEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_edit_core::document::DocumentError;
use std::cell::OnceCell;
use std::rc::Rc;

/// An insight popup with observable overload selection. Up/Down cycle signatures
/// while the editor retains keyboard focus; Escape closes through the base.
#[repr(C)]
pub struct OverloadInsightWindow {
    base: InsightWindow,
    viewer: OnceCell<Ref<OverloadViewer>>,
}
ferro_class!(OverloadInsightWindow: InsightWindow);
ferroui_base::ferro_class_info!(OverloadInsightWindow {
    new: OverloadInsightWindow::new
});
ferro_impl_classes!(OverloadInsightWindow: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);
ferro_properties! { impl OverloadInsightWindow {
    pub fn provider_property() -> StyledProperty<Option<OverloadProviderRef>> {
        FerroProperty::register::<OverloadInsightWindow, _>("Provider", None)
    }
} }
impl FerroObjectImpl for OverloadInsightWindow {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let viewer = OverloadViewer::new();
        this.set_content(Some(Rc::new(viewer.clone().upcast::<Control>())));
        this.viewer.set(viewer).ok();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::provider_property().as_property() {
            if let Some(viewer) = this.viewer.get() {
                viewer.set_provider(this.provider());
            }
            this.update_position();
        }
    }
}
impl InputElementImpl for OverloadInsightWindow {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if event.handled() || this.is_closed() {
            return;
        }
        if this.provider().is_some_and(|provider| provider.count() > 1) {
            if let Some(change) = match event.key {
                Key::Up => Some(-1),
                Key::Down => Some(1),
                _ => None,
            } {
                this.viewer().change_index(change);
                this.update_position();
                event.set_handled(true);
            }
        }
    }
}
impl CompletionWindowBaseImpl for OverloadInsightWindow {
    fn on_closed(this: &Self) {
        Self::parent_on_closed(this);
        this.set_provider(None);
    }
}
impl OverloadInsightWindow {
    pub fn construct() -> Self {
        Self {
            base: InsightWindow::construct(),
            viewer: OnceCell::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn show_for(
        editor: &Ref<TextEditor>,
        start: usize,
        end: usize,
        provider: Rc<dyn IOverloadProvider>,
    ) -> Result<Ref<Self>, DocumentError> {
        let window = Self::new();
        window.attach_to_editor(editor, start, end)?;
        window.set_provider(Some(provider));
        window.show();
        Ok(window)
    }
    pub fn viewer(&self) -> Ref<OverloadViewer> {
        self.viewer
            .get()
            .expect("constructed overload viewer")
            .clone()
    }
    pub fn provider(&self) -> Option<Rc<dyn IOverloadProvider>> {
        self.get_value(Self::provider_property())
            .map(|provider| provider.0)
    }
    pub fn set_provider(&self, provider: Option<Rc<dyn IOverloadProvider>>) {
        self.set_value(Self::provider_property(), provider.map(OverloadProviderRef));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overload_provider::{OverloadItem, OverloadProvider};
    use ferroui_base::data::model::INotifyPropertyChanged;
    use ferroui_base::input::InputElement;

    #[test]
    fn editor_keys_cycle_overloads_without_moving_caret_and_close_disposes_provider() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        editor.set_text("call()").unwrap();
        editor.set_caret_offset(5).unwrap();
        let provider = OverloadProvider::new(vec![
            OverloadItem::new("call()", "first"),
            OverloadItem::new("call(x)", "second"),
        ]);
        let window = OverloadInsightWindow::show_for(&editor, 0, 6, provider.clone()).unwrap();
        let mut down = KeyEventArgs::new();
        down.key = Key::Down;
        down.set_routed_event(Some(InputElement::key_down_event()));
        editor.text_area().raise_event(&down);
        assert!(down.handled());
        assert_eq!(provider.selected_index(), 1);
        assert_eq!(editor.caret_offset(), 5);
        assert_eq!(window.viewer().text(), "2 of 2");
        window.close_window();
        assert_eq!(provider.property_changed().handler_count(), 0);
    }
}
