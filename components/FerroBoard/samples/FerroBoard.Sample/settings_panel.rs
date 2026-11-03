//! Native public SettingsPanel (source SettingsPanel.cs), with passive compiled layout fragments.
//! Settings and widget callbacks use weak receivers; Drop removes owned subscriptions.
use crate::{
    enum_choices::EnumChoice,
    language_bindings::LanguagePanelBindings,
    languages::{LanguageCatalog, LanguagePanelViewModel},
    panel_context::SettingsPanelContext,
    settings_callback_owner::SettingsCallbackOwner,
    settings_field_view::SettingsFieldView,
    settings_fields::{
        BooleanField, FieldError, FieldKind, NumberField, SettingsFieldViewModel,
        SettingsRefreshers,
    },
    theme_choices::{ThemeChoices, ThemeProperty},
};
use ferroboard_controls::{OnScreenKeyboard, themes::KeyboardThemes};
use ferroboard_core::text::locale_text::LocaleText;
use ferroboard_core::{engine::KeyboardEngine, settings::KeyboardSettings};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, controls::NameScope,
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
    utilities::CultureInfo,
};
use ferroui_controls::{
    Button, ContentControlImpl, ControlImpl, ItemsControl, StackPanel, TextBox, UserControl,
    primitives::TemplatedControlImpl,
};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactiveui::{ICommand, ReactiveCommand};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{cell::RefCell, rc::Rc};
#[derive(Debug)]
pub enum SettingsPanelError {
    NullReference,
    Field(FieldError),
    Markup(String),
    Resource(String),
    Encoding,
}
impl std::fmt::Display for SettingsPanelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SettingsPanelError {}
impl From<FieldError> for SettingsPanelError {
    fn from(e: FieldError) -> Self {
        if e == FieldError::MissingTarget {
            Self::NullReference
        } else {
            Self::Field(e)
        }
    }
}
struct LanguageView {
    _bindings: Rc<LanguagePanelBindings>,
    _filter: Ref<TextBox>,
    _choices: Ref<ItemsControl>,
}
pub(crate) struct SettingsPanelPresentation {
    pub context: Rc<SettingsPanelContext>,
    pub themes: ThemeChoices,
    pub languages: Rc<LanguagePanelBindings>,
}
#[repr(C)]
pub struct SettingsPanel {
    base: UserControl,
    body: std::cell::OnceCell<Ref<StackPanel>>,
    keyboard: RefCell<Option<Ref<OnScreenKeyboard>>>,
    context: Rc<SettingsPanelContext>,
    refreshers: SettingsRefreshers,
    fields: RefCell<Vec<Rc<SettingsFieldView>>>,
    language: RefCell<Option<LanguageView>>,
    subscriptions: RefCell<Vec<(Rc<KeyboardSettings>, u64)>>,
    callback_owner: RefCell<std::rc::Weak<SettingsCallbackOwner>>,
}
ferroui_base::ferro_class!(SettingsPanel:UserControl);
ferroui_base::ferro_class_info!(SettingsPanel {new:SettingsPanel::new,markup:{methods:[try fn Attach(Option<Ref<OnScreenKeyboard>>)=>SettingsPanel::attach_nullable]}});
ferroui_base::ferro_impl_classes!(SettingsPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl SettingsPanel {
    pub fn new() -> Ref<Self> {
        let this = ferroui_base::instantiate(Self {
            base: UserControl::construct(),
            body: std::cell::OnceCell::new(),
            keyboard: RefCell::new(None),
            context: Rc::new(SettingsPanelContext::detached()),
            refreshers: SettingsRefreshers::new(),
            fields: RefCell::new(Vec::new()),
            language: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            callback_owner: RefCell::new(std::rc::Weak::new()),
        });
        crate::compiled_settings_panel::populate(None, &this)
            .expect("compiled SettingsPanel constructor");
        let body = NameScope::get_name_scope(&this)
            .unwrap()
            .find("Body")
            .unwrap()
            .cast::<StackPanel>()
            .unwrap();
        this.body.set(body).expect("SettingsPanel readonly body");
        this
    }
    pub fn attach(&self, keyboard: Ref<OnScreenKeyboard>) -> Result<(), SettingsPanelError> {
        self.attach_nullable(Some(keyboard))
    }
    pub fn attach_nullable(
        &self,
        keyboard: Option<Ref<OnScreenKeyboard>>,
    ) -> Result<(), SettingsPanelError> {
        let services = keyboard
            .as_ref()
            .map(|k| (k.settings(), k.engine().language_models()));
        let previous = self.keyboard.replace(keyboard);
        self.context.retarget(services);
        drop(previous);
        self.build()?;
        let settings = self
            .context
            .try_settings()
            .map_err(|_| SettingsPanelError::NullReference)?;
        let owner = self.to_ref().downgrade();
        // Preserve successful Attach notification multiplicity while the panel is alive.
        let token = settings.property_changed().add(Rc::new(move |_| {
            let Some(owner) = owner.upgrade() else {
                return;
            };
            owner
                .on_settings_changed()
                .expect("source SettingsPanel property refresh")
        }));
        self.subscriptions.borrow_mut().push((settings, token));
        Ok(())
    }
    fn body(&self) -> Ref<StackPanel> {
        self.body
            .get()
            .expect("SettingsPanel constructor body")
            .clone()
    }
    fn engine(&self) -> Result<Rc<KeyboardEngine>, SettingsPanelError> {
        self.keyboard
            .borrow()
            .as_ref()
            .map(|k| k.engine())
            .ok_or(SettingsPanelError::NullReference)
    }
    fn on_settings_changed(&self) -> Result<(), SettingsPanelError> {
        self.refreshers.refresh(&self.context).map_err(Into::into)
    }
    fn header(&self, text: &str) -> Result<(), SettingsPanelError> {
        let header = crate::compiled_settings_views::build_views_settings_header_xaml(Some(
            XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
        ))
        .map_err(|e| SettingsPanelError::Markup(e.to_string()))?;
        header.set_text(Some(text));
        self.body()
            .children()
            .add(header.upcast::<ferroui_controls::Control>());
        Ok(())
    }
    fn field(&self, kind: FieldKind) -> Result<(), SettingsPanelError> {
        let model = SettingsFieldViewModel::new(self.context.clone(), kind);
        let view = SettingsFieldView::new(model, self.callback_owner())
            .map_err(SettingsPanelError::Markup)?;
        let owned = view.clone();
        self.refreshers.add(Rc::new(move || owned.refresh()));
        ferroui_controls::automation::AutomationProperties::set_name(
            &view.input,
            Some(view.model.label()),
        );
        self.fields.borrow_mut().push(view.clone());
        self.body().children().add(view.root.clone());
        Ok(())
    }
    fn snapshot_themes(&self) -> Result<Rc<ThemeChoices>, SettingsPanelError> {
        let compare = LocaleText::new(CultureInfo::current_culture().name())
            .map_err(|e| SettingsPanelError::Resource(e.to_string()))?;
        ThemeChoices::snapshot(
            KeyboardThemes::all(),
            |t| t.name(),
            |t| t.id(),
            |a, b| compare.compare(a, b, false),
        )
        .map(Rc::new)
        .map_err(|e| SettingsPanelError::Resource(e.to_string()))
    }
    fn build_languages(&self) -> Result<(), SettingsPanelError> {
        let group = crate::compiled_settings_views::build_views_settings_languages_xaml(Some(
            XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
        ))
        .map_err(|e| SettingsPanelError::Markup(e.to_string()))?;
        let names = NameScope::get_name_scope(&group).unwrap();
        let filter = names.find("Filter").unwrap().cast::<TextBox>().unwrap();
        let choices = names
            .find("Choices")
            .unwrap()
            .cast::<ItemsControl>()
            .unwrap();
        let engine = self.engine()?;
        let compare = LocaleText::new(CultureInfo::current_culture().name())
            .map_err(|e| SettingsPanelError::Resource(e.to_string()))?;
        let catalog = LanguageCatalog::snapshot(&engine.resources(), |a, b| {
            compare.compare(a, b, true).map_err(|e| e.to_string())
        })
        .map_err(SettingsPanelError::Resource)?;
        // Resolve current culture for each Contains. Native handle cache is per language-view, synchronous.
        type SearchCache = Option<(String, Rc<LocaleText>)>;
        let cache = RefCell::<SearchCache>::new(None);
        let model = LanguagePanelViewModel::with_settings_source(
            self.context.settings_source(),
            catalog,
            Rc::new(move |a, b| {
                let name = CultureInfo::current_culture().name().to_string();
                let old = cache
                    .borrow()
                    .as_ref()
                    .filter(|(n, _)| n == &name)
                    .map(|(_, c)| c.clone());
                let compare = if let Some(c) = old {
                    c
                } else {
                    let c = Rc::new(LocaleText::new(&name).map_err(|e| e.to_string())?);
                    *cache.borrow_mut() = Some((name, c.clone()));
                    c
                };
                compare.contains(a, b, true).map_err(|e| e.to_string())
            }),
        )
        .map_err(SettingsPanelError::Resource)?;
        let bindings = LanguagePanelBindings::new_owned(model.clone(), self.callback_owner())
            .map_err(|_| SettingsPanelError::Encoding)?;
        choices.set_items_source(Some(bindings.items()));
        let weak = filter.downgrade();
        let retained_list = choices.clone();
        let retained_bindings = bindings.clone();
        let owner = self.callback_owner();
        filter.text_changed(move |_, _| {
            let Some(_owner) = owner.retain() else {
                return;
            };
            let _list = &retained_list;
            if let Some(filter) = weak.upgrade() {
                retained_bindings
                    .model()
                    .text_changed(filter.text().as_deref().map(Into::into))
                    .expect("source language filter callback");
            }
        });
        let children = group.children().to_vec();
        group.children().clear();
        for child in children {
            self.body().children().add(child);
        }
        *self.language.borrow_mut() = Some(LanguageView {
            _bindings: bindings,
            _filter: filter,
            _choices: choices,
        });
        Ok(())
    }
    fn build_data_buttons(&self) -> Result<(), SettingsPanelError> {
        let group = crate::compiled_settings_views::build_views_settings_databuttons_xaml(Some(
            XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
        ))
        .map_err(|e| SettingsPanelError::Markup(e.to_string()))?;
        let names = NameScope::get_name_scope(&group).unwrap();
        let context = self.context.clone();
        let reset = Rc::new(ReactiveCommand::create_with(
            move || context.reset_settings(),
            None,
            Some(ImmediateScheduler::instance()),
        ));
        let owner = self.callback_owner();
        names
            .find("Reset")
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .click(move |_, _| {
                let Some(_owner) = owner.retain() else {
                    return;
                };
                reset.execute(None);
            });
        let context = self.context.clone();
        let forget = Rc::new(ReactiveCommand::create_with(
            move || {
                context.forget_learned_words();
            },
            None,
            Some(ImmediateScheduler::instance()),
        ));
        let owner = self.callback_owner();
        names
            .find("Forget")
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .click(move |_, _| {
                let Some(_owner) = owner.retain() else {
                    return;
                };
                forget.execute(None);
            });
        self.body()
            .children()
            .add(group.upcast::<ferroui_controls::Control>());
        Ok(())
    }
    /// Private application adapter: reuse the successful Build snapshots, not another catalog/UI.
    pub(crate) fn presentation(&self) -> Result<SettingsPanelPresentation, SettingsPanelError> {
        let themes = self
            .fields
            .borrow()
            .iter()
            .find_map(|field| match field.model.kind() {
                FieldKind::Theme { choices, .. } => Some(choices.as_ref().clone()),
                _ => None,
            })
            .ok_or_else(|| {
                SettingsPanelError::Resource(
                    "panel theme snapshot is unavailable before a successful Attach".into(),
                )
            })?;
        let languages = self
            .language
            .borrow()
            .as_ref()
            .map(|view| view._bindings.clone())
            .ok_or(SettingsPanelError::NullReference)?;
        Ok(SettingsPanelPresentation {
            context: self.context.clone(),
            themes,
            languages,
        })
    }
    fn callback_owner(&self) -> Rc<SettingsCallbackOwner> {
        if let Some(owner) = self.callback_owner.borrow().upgrade() {
            return owner;
        }
        let owner = SettingsCallbackOwner::new(self.to_ref());
        *self.callback_owner.borrow_mut() = Rc::downgrade(&owner);
        owner
    }
    /// End this callback scope and remove registered settings hooks. Reattach creates a fresh scope.
    /// Drop performs the same cleanup; retained widgets from an ended scope become inactive.
    pub fn release_native_ownership(&self) {
        self.release_source_subscriptions();
        let owner = self.callback_owner.replace(std::rc::Weak::new()).upgrade();
        if let Some(owner) = owner {
            owner.release();
        }
    }
    /// Remove this panel's tracked settings hooks without ending its widget callback scope.
    pub fn release_source_subscriptions(&self) {
        for (settings, token) in self.subscriptions.take() {
            settings.property_changed().remove(token);
        }
    }
}
impl Drop for SettingsPanel {
    fn drop(&mut self) {
        self.release_native_ownership();
        self.context.retarget(None);
    }
}
include!("settings_panel_build.rs");

#[cfg(test)]
#[path = "settings_header_tests.rs"]
mod header_tests;
#[cfg(test)]
#[path = "settings_panel_tests.rs"]
mod tests;
