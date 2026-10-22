use crate::completion_tip_content_control::CompletionTipContentControl;
use crate::overload_provider::{IOverloadProvider, OverloadProviderRef};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControl, ContentControlImpl, Control, ControlImpl, StackPanel, TextBlock,
};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// Native content presentation for the selected signature and its documentation.
/// Navigation buttons are hidden when the provider contains fewer than two items.
#[repr(C)]
pub struct OverloadViewer {
    base: ContentControl,
    header: OnceCell<Ref<CompletionTipContentControl>>,
    documentation: OnceCell<Ref<CompletionTipContentControl>>,
    navigation: OnceCell<Ref<StackPanel>>,
    index_text: OnceCell<Ref<TextBlock>>,
    provider_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(OverloadViewer: ContentControl);
ferroui_base::ferro_class_info!(OverloadViewer {
    new: OverloadViewer::new
});
ferro_impl_classes!(OverloadViewer:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl,
    ControlImpl, TemplatedControlImpl, ContentControlImpl
);
ferro_properties! { impl OverloadViewer {
    pub fn provider_property() -> StyledProperty<Option<OverloadProviderRef>> {
        FerroProperty::register::<OverloadViewer, _>("Provider", None)
    }
    pub fn text_property() -> StyledProperty<String> {
        FerroProperty::register::<OverloadViewer, _>("Text", String::new())
    }
} }
impl FerroObjectImpl for OverloadViewer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.build_content();
        this.provider_changed();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::provider_property().as_property() {
            this.provider_changed();
        } else if change.property() == Self::text_property().as_property() {
            if let Some(text) = this.index_text.get() {
                text.set_text(Some(&this.text()));
            }
        }
    }
}
impl OverloadViewer {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            header: OnceCell::new(),
            documentation: OnceCell::new(),
            navigation: OnceCell::new(),
            index_text: OnceCell::new(),
            provider_subscription: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn provider(&self) -> Option<Rc<dyn IOverloadProvider>> {
        self.get_value(Self::provider_property())
            .map(|value| value.0)
    }
    pub fn set_provider(&self, value: Option<Rc<dyn IOverloadProvider>>) {
        self.set_value(Self::provider_property(), value.map(OverloadProviderRef));
    }
    pub fn text(&self) -> String {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, value: impl Into<String>) {
        self.set_value(Self::text_property(), value.into());
    }
    pub fn header_presenter(&self) -> Ref<CompletionTipContentControl> {
        self.header.get().expect("constructed header").clone()
    }
    pub fn content_presenter(&self) -> Ref<CompletionTipContentControl> {
        self.documentation
            .get()
            .expect("constructed documentation")
            .clone()
    }
    pub fn navigation_is_visible(&self) -> bool {
        self.navigation
            .get()
            .is_some_and(|navigation| navigation.is_visible())
    }
    pub fn change_index(&self, change: isize) -> bool {
        let Some(provider) = self.provider() else {
            return false;
        };
        let count = provider.count();
        if count == 0 || change == 0 {
            return false;
        }
        // Arithmetic remains valid even for an external provider whose index
        // became stale when its collection changed.
        let current = provider.selected_index().min(count - 1);
        let next = if change >= 0 {
            (current as u128 + change as u128) % count as u128
        } else {
            (current as u128 + count as u128 - change.unsigned_abs() as u128 % count as u128)
                % count as u128
        } as usize;
        provider.set_selected_index(next);
        self.refresh();
        current != next
    }
    fn provider_changed(&self) {
        if let Some(previous) = self.provider_subscription.borrow_mut().take() {
            previous.dispose();
        }
        if let Some(provider) = self.provider() {
            let weak = self.to_ref().downgrade();
            let token = provider.property_changed().add(Rc::new(move |_| {
                if let Some(viewer) = weak.upgrade() {
                    viewer.refresh();
                }
            }));
            let weak_provider = Rc::downgrade(&provider);
            *self.provider_subscription.borrow_mut() = Some(Disposable::create(move || {
                if let Some(provider) = weak_provider.upgrade() {
                    provider.property_changed().remove(token);
                }
            }));
        }
        self.refresh();
    }
    pub fn refresh(&self) {
        let provider = self.provider();
        self.set_text(
            provider
                .as_ref()
                .map_or_else(String::new, |provider| provider.current_index_text()),
        );
        if let Some(navigation) = self.navigation.get() {
            navigation.set_is_visible(
                provider
                    .as_ref()
                    .is_some_and(|provider| provider.count() > 1),
            );
        }
        if let Some(header) = self.header.get() {
            header.set_content(
                provider
                    .as_ref()
                    .and_then(|provider| provider.current_header()),
            );
        }
        if let Some(content) = self.documentation.get() {
            let value = provider
                .as_ref()
                .and_then(|provider| provider.current_content());
            content.set_is_visible(value.is_some());
            content.set_content(value);
        }
    }
    fn build_content(&self) {
        let panel = StackPanel::new();
        panel.set_spacing(4.0);
        let heading = StackPanel::new();
        heading.set_orientation(Orientation::Horizontal);
        heading.set_spacing(4.0);
        let navigation = StackPanel::new();
        navigation.set_orientation(Orientation::Horizontal);
        navigation.set_spacing(3.0);
        let index_text = TextBlock::new();
        for (label, change) in [("▲", -1), ("▼", 1)] {
            let button = Button::new();
            button.set_content(Some(Rc::new(label.to_owned())));
            button.set_focusable(false);
            let weak = self.to_ref().downgrade();
            self.subscriptions
                .borrow_mut()
                .push(
                    button.add_disposable_handler(Button::click_event(), move |_, event| {
                        if let Some(viewer) = weak.upgrade() {
                            viewer.change_index(change);
                            event.set_handled(true);
                        }
                    }),
                );
            navigation.children().add(button.upcast::<Control>());
            if change == -1 {
                navigation
                    .children()
                    .add(index_text.clone().upcast::<Control>());
            }
        }
        heading
            .children()
            .add(navigation.clone().upcast::<Control>());
        let header = CompletionTipContentControl::new();
        heading.children().add(header.clone().upcast::<Control>());
        let documentation = CompletionTipContentControl::new();
        panel.children().add(heading.upcast::<Control>());
        panel
            .children()
            .add(documentation.clone().upcast::<Control>());
        self.set_content(Some(Rc::new(panel.upcast::<Control>())));
        self.header.set(header).ok();
        self.documentation.set(documentation).ok();
        self.navigation.set(navigation).ok();
        self.index_text.set(index_text).ok();
    }
}
impl Drop for OverloadViewer {
    fn drop(&mut self) {
        if let Some(subscription) = self.provider_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overload_provider::{OverloadItem, OverloadProvider};
    use ferroui_base::data::model::INotifyPropertyChanged;

    #[test]
    fn provider_notifications_navigation_rich_content_and_disposal() {
        let _scope = crate::tests::test_scope();
        let first = OverloadProvider::new(vec![
            OverloadItem::new("f(x)", "one"),
            OverloadItem::new("f(x,y)", "two"),
        ]);
        let second = OverloadProvider::new(vec![OverloadItem::new("g()", "three")]);
        let viewer = OverloadViewer::new();
        viewer.set_provider(Some(first.clone()));
        assert_eq!(first.property_changed().handler_count(), 1);
        assert_eq!(viewer.text(), "1 of 2");
        assert!(viewer.navigation_is_visible());
        assert!(viewer.change_index(-1));
        assert_eq!(first.selected_index(), 1);
        assert_eq!(viewer.text(), "2 of 2");
        first.set_selected_index(0);
        assert_eq!(viewer.text(), "1 of 2");
        let rich = TextBlock::new();
        rich.set_text(Some("rich signature"));
        first.set_items(vec![OverloadItem {
            header: Some(Rc::new(rich.clone().upcast::<Control>())),
            content: None,
        }]);
        assert!(!viewer.navigation_is_visible());
        assert_eq!(
            viewer
                .header_presenter()
                .content()
                .unwrap()
                .downcast_ref::<Ref<Control>>(),
            Some(&rich.upcast::<Control>())
        );
        viewer.set_provider(Some(second.clone()));
        assert_eq!(first.property_changed().handler_count(), 0);
        assert_eq!(second.property_changed().handler_count(), 1);
        second.set_items(Vec::new());
        assert_eq!(viewer.text(), "0 of 0");
        assert!(!viewer.change_index(1));
        drop(viewer);
        assert_eq!(second.property_changed().handler_count(), 0);
    }
}
