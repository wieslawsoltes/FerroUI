//! Shared Ribbon.Backstage.cs and Ribbon.cs application action. Copyright Wiesław Šoltés, MIT.
use super::super::{RibbonBackstage, RibbonHandledEventArgs, RibbonItemHelper, RibbonMenu};
use super::*;
use ferroui_base::{
    Rect, StyledElement,
    data::core::ValueTypes,
    input::{KeyModifiers, NavigationMethod},
    metadata::{MarkupDelegate, MarkupTyped},
    reactive::IDisposable,
    styling::ThemeVariant,
    utilities::EventArgs,
};
use ferroui_controls::{
    PlacementMode, TopLevel,
    primitives::{
        Popup,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};
#[derive(Default)]
pub(super) struct BackstageState {
    popup: RefCell<Option<Ref<Popup>>>,
    close_handler: RefCell<Option<MarkupDelegate>>,
    root_size: RefCell<Option<Rc<dyn IDisposable>>>,
    opened: RefCell<Option<MarkupDelegate>>,
    closed: RefCell<Option<MarkupDelegate>>,
    application_click: RefCell<Option<MarkupDelegate>>,
}
impl Drop for BackstageState {
    fn drop(&mut self) {
        if let Some(subscription) = self.root_size.get_mut().take() {
            subscription.dispose();
        }
        if let Some(popup) = self.popup.get_mut().take() {
            popup.set_is_open(false);
            popup.set_child(None);
        }
    }
}
struct CloseHandler(WeakRef<Ribbon>);
impl PartialEq for CloseHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class CloseHandler{this:Rc<CloseHandler>,handles:[Rc<CloseHandler>],methods:[fn Invoke(Option<BoxedValue>,EventArgs)=>|h:&Rc<CloseHandler>,_,_|{if let Some(r)=h.0.upgrade(){r.set_is_backstage_open(false);}}]});
impl Ribbon {
    pub(super) fn initialize_backstage() {
        Self::backstage_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (old, new) = e.get_old_and_new_value::<Option<Ref<RibbonBackstage>>>();
                this.on_backstage_changed(old, new);
            })
            .detach();
        Self::is_backstage_open_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (_, open) = e.get_old_and_new_value::<bool>();
                if open {
                    this.open_backstage_core();
                } else {
                    this.close_backstage_core();
                }
                this.on_model_backstage_changed(this.is_backstage_open());
            })
            .detach();
    }
    pub fn backstage(&self) -> Option<Ref<RibbonBackstage>> {
        self.get_value(Self::backstage_property())
    }
    pub fn set_backstage(&self, value: Option<Ref<RibbonBackstage>>) {
        self.set_value(Self::backstage_property(), value);
    }
    pub fn is_backstage_open(&self) -> bool {
        self.get_value(Self::is_backstage_open_property())
    }
    pub fn set_is_backstage_open(&self, value: bool) {
        self.set_value(Self::is_backstage_open_property(), value);
    }
    pub fn application_menu(&self) -> Option<Ref<ferroui_controls::primitives::FlyoutBase>> {
        self.get_value(Self::application_menu_property())
    }
    pub fn set_application_menu(
        &self,
        value: Option<Ref<ferroui_controls::primitives::FlyoutBase>>,
    ) {
        self.set_value(Self::application_menu_property(), value);
    }
    fn on_backstage_changed(
        &self,
        old: Option<Ref<RibbonBackstage>>,
        new: Option<Ref<RibbonBackstage>>,
    ) {
        let handler = self.backstage_state.close_handler.borrow().clone();
        if let Some(old) = old {
            old.remove_close_requested(handler.clone());
            old.set_ribbon(None);
        }
        if let Some(new) = new {
            let handler = handler.unwrap_or_else(|| {
                ValueTypes::register_reference::<CloseHandler>();
                let h = MarkupDelegate::for_method(
                    Some(Rc::new(CloseHandler(self.to_ref().downgrade()))),
                    CloseHandler::MARKUP,
                    &CloseHandler::MARKUP.methods[0],
                );
                self.backstage_state.close_handler.replace(Some(h.clone()));
                h
            });
            new.add_close_requested(Some(handler));
            new.set_ribbon(Some(self.to_ref()));
            RibbonItemHelper::set_owner(&new, Some(self.item_owner())).expect("backstage owner");
        }
        if self.is_backstage_open() {
            self.set_is_backstage_open(false);
        }
    }
    pub(crate) fn open_backstage_core(&self) {
        let Some(backstage) = self.backstage() else {
            self.set_is_backstage_open(false);
            return;
        };
        let Some(root) = TopLevel::get_top_level(Some(self)).filter(|_| self.is_loaded()) else {
            return;
        };
        let current = self.backstage_state.popup.borrow().clone();
        if current
            .as_ref()
            .is_some_and(|p| p.is_open() && p.child().is_some_and(|c| c.ptr_eq(&backstage)))
        {
            return;
        }
        self.hide_key_tips();
        self.close_minimized_popup();
        let popup = if let Some(p) = current {
            p
        } else {
            let p = Popup::new();
            self.backstage_state.popup.replace(Some(p.clone()));
            p
        };
        popup.set_child(Some(backstage.clone().upcast()));
        // XamlRoot maps to native owner plus an explicit window-origin placement,
        // not Popup's pointer default or the Ribbon's nested visual position.
        popup.set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
        popup.set_placement_target(root.clone().upcast::<Control>());
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_rect(Some(Rect::default()));
        popup.set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        popup.set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        backstage.set_value(
            ThemeVariant::requested_theme_variant_property(),
            self.actual_theme_variant(),
        );
        if backstage
            .get_local_value(StyledElement::data_context_property())
            .is_none()
        {
            backstage.set_data_context(self.data_context());
        }
        self.size_backstage();
        if let Some(subscription) = self.backstage_state.root_size.take() {
            subscription.dispose();
        }
        let weak = self.to_ref().downgrade();
        let subscription = root.property_changed(move |e| {
            if e.property() == TopLevel::client_size_property().as_property()
                && let Some(r) = weak.upgrade()
            {
                r.size_backstage();
            }
        });
        self.backstage_state.root_size.replace(Some(subscription));
        self.attach_popup_keyboard(Some(&backstage));
        popup.set_horizontal_offset(0.);
        popup.set_vertical_offset(0.);
        popup.set_is_open(true);
        backstage.ensure_selection();
        backstage.animate_content();
        backstage.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
        self.raise_backstage_event(true);
    }
    pub(super) fn refresh_backstage_popup_theme(&self) {
        let child = self
            .backstage_state
            .popup
            .borrow()
            .as_ref()
            .and_then(|popup| popup.child());
        if let Some(child) = child {
            child.set_value(
                ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
        }
    }
    fn size_backstage(&self) {
        if let Some(b) = self.backstage()
            && let Some(root) = TopLevel::get_top_level(Some(self))
        {
            b.set_width(root.client_size().width);
            b.set_height(root.client_size().height);
        }
    }
    fn close_backstage_core(&self) {
        if let Some(subscription) = self.backstage_state.root_size.take() {
            subscription.dispose();
        }
        self.hide_key_tips();
        let popup = self.backstage_state.popup.borrow().clone();
        if let Some(popup) = popup.filter(|p| p.is_open()) {
            popup.set_is_open(false);
            popup.set_child(None);
            self.raise_backstage_event(false);
            if let Some(button) = self.application_button() {
                button.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
            }
        }
    }
    pub fn invoke_application_button(&self) {
        self.invoke_application_button_at(None);
    }
    pub fn invoke_application_button_at(&self, anchor: Option<Ref<Control>>) {
        self.hide_key_tips();
        let args = RibbonHandledEventArgs::new();
        let handler = self.backstage_state.application_click.borrow().clone();
        if let Some(handler) = handler {
            handler
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(args.clone())),
                ])
                .expect("ApplicationButtonClick handler");
        }
        if args.handled() {
            return;
        }
        if self.backstage().is_some() {
            self.set_is_backstage_open(true);
        } else if let Some(menu) = self.application_menu()
            && let Some(target) = anchor.or_else(|| self.application_button().map(|b| b.upcast()))
        {
            RibbonMenu::apply_theme(Some(&menu));
            menu.show_at(&target);
        }
    }
    fn raise_backstage_event(&self, opened: bool) {
        let handler = if opened {
            self.backstage_state.opened.borrow().clone()
        } else {
            self.backstage_state.closed.borrow().clone()
        };
        if let Some(handler) = handler {
            handler
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(EventArgs::EMPTY)),
                ])
                .expect("backstage event handler");
        }
    }
    pub fn add_backstage_opened(&self, h: Option<MarkupDelegate>) {
        let next =
            MarkupDelegate::combine(self.backstage_state.opened.borrow().as_ref(), h.as_ref());
        self.backstage_state.opened.replace(next);
    }
    pub fn remove_backstage_opened(&self, h: Option<MarkupDelegate>) {
        let next =
            MarkupDelegate::remove(self.backstage_state.opened.borrow().as_ref(), h.as_ref());
        self.backstage_state.opened.replace(next);
    }
    pub fn add_backstage_closed(&self, h: Option<MarkupDelegate>) {
        let next =
            MarkupDelegate::combine(self.backstage_state.closed.borrow().as_ref(), h.as_ref());
        self.backstage_state.closed.replace(next);
    }
    pub fn remove_backstage_closed(&self, h: Option<MarkupDelegate>) {
        let next =
            MarkupDelegate::remove(self.backstage_state.closed.borrow().as_ref(), h.as_ref());
        self.backstage_state.closed.replace(next);
    }
    pub fn add_application_button_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(
            self.backstage_state.application_click.borrow().as_ref(),
            h.as_ref(),
        );
        self.backstage_state.application_click.replace(next);
    }
    pub fn remove_application_button_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(
            self.backstage_state.application_click.borrow().as_ref(),
            h.as_ref(),
        );
        self.backstage_state.application_click.replace(next);
    }
    #[cfg(test)]
    pub(crate) fn backstage_popup(&self) -> Option<Ref<Popup>> {
        self.backstage_state.popup.borrow().clone()
    }
}
