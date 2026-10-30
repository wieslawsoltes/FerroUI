//! `SystemBackdrop`, `MicaBackdrop` and `DesktopAcrylicBackdrop` of WinUI
//! (`dxaml/xcp/dxaml/lib/SystemBackdrop_Partial.cpp`,
//! `controls/dev/Materials/MicaBackdrop`, `DesktopAcrylicBackdrop` of
//! microsoft-ui-xaml), and the windows as their targets (winui-materials.md).

use crate::{TopLevel, WindowBase};
use ferroui_base::media::{
    BrushImpl, IBrush, XamlCompositionBrushBase, XamlCompositionBrushBaseImpl,
};
use ferroui_base::platform::{ColorContrastPreference, IPlatformSettings};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::system_backdrops::{
    DesktopAcrylicController, ICompositionSupportsSystemBackdrop, MicaController, MicaKind, PlatformSystemBackdrop,
    SystemBackdropConfiguration, SystemBackdropController, SystemBackdropSupport, SystemBackdropTheme,
};
use ferroui_base::rendering::composition::{CompositionBrush, Compositor};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroLocator, LocatorExtensions, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- the target -------------------------------------------------------------------------

/// The brush a window draws its system backdrop with: a XAML composition
/// brush whose composition brush the controller sets.
#[repr(C)]
pub(crate) struct SystemBackdropHostBrush {
    base: XamlCompositionBrushBase,
}

ferro_class!(SystemBackdropHostBrush: XamlCompositionBrushBase);
ferro_impl_classes!(SystemBackdropHostBrush: FerroObjectImpl, BrushImpl, XamlCompositionBrushBaseImpl);

impl SystemBackdropHostBrush {
    fn construct() -> Self {
        Self { base: XamlCompositionBrushBase::construct() }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// A window as the target of a system backdrop (WinUI's
/// `ICompositionSupportsSystemBackdrop` of a window or a popup window).
pub struct TopLevelBackdropTarget {
    top_level: WeakRef<TopLevel>,
    brush: RefCell<Option<Rc<CompositionBrush>>>,
    host_brush: Ref<SystemBackdropHostBrush>,
    /// Whether the target asked the platform for a material, and whether it
    /// changed the transparency of the window to have the platform blur
    /// what is behind it.
    platform_active: Cell<bool>,
    blur_requested: Cell<bool>,
}

impl TopLevelBackdropTarget {
    pub(crate) fn new(top_level: &TopLevel) -> Rc<TopLevelBackdropTarget> {
        Rc::new(TopLevelBackdropTarget {
            top_level: top_level.to_ref().downgrade(),
            brush: RefCell::new(None),
            host_brush: SystemBackdropHostBrush::new(),
            platform_active: Cell::new(false),
            blur_requested: Cell::new(false),
        })
    }

    /// The brush the window draws under its content: the XAML brush over
    /// the composition brush of the controller.
    pub(crate) fn host_brush(&self) -> Rc<dyn IBrush> {
        self.host_brush.clone().into()
    }

    /// Whether the platform draws a material behind the window.
    pub fn is_platform_active(&self) -> bool {
        self.platform_active.get()
    }

    fn restore_transparency(&self, top_level: &TopLevel) {
        if self.blur_requested.replace(false) {
            if let Some(platform_impl) = top_level.platform_impl() {
                platform_impl.set_transparency_level_hint(&top_level.transparency_level_hint());
            }
        }
    }
}

impl ICompositionSupportsSystemBackdrop for TopLevelBackdropTarget {
    fn system_backdrop(&self) -> Option<Rc<CompositionBrush>> {
        self.brush.borrow().clone()
    }

    fn set_system_backdrop(&self, brush: Option<Rc<CompositionBrush>>) {
        *self.brush.borrow_mut() = brush.clone();
        self.host_brush.set_composition_brush(brush);
        if let Some(top_level) = self.top_level.upgrade() {
            top_level.update_system_backdrop_visual();
        }
    }

    fn try_set_platform_system_backdrop(&self, backdrop: Option<&PlatformSystemBackdrop>) -> SystemBackdropSupport {
        let Some(top_level) = self.top_level.upgrade() else { return SystemBackdropSupport::None };
        let Some(platform_impl) = top_level.platform_impl() else { return SystemBackdropSupport::None };
        let Some(backdrop) = backdrop else {
            if self.platform_active.replace(false) {
                platform_impl.set_system_backdrop(None);
            }
            self.restore_transparency(&top_level);
            return SystemBackdropSupport::None;
        };
        let support = platform_impl.set_system_backdrop(Some(backdrop));
        if support == SystemBackdropSupport::Native {
            self.platform_active.set(true);
            self.restore_transparency(&top_level);
            return support;
        }
        if self.platform_active.replace(false) {
            platform_impl.set_system_backdrop(None);
        }
        // Where the platform blurs what is behind a transparent window, the
        // material is drawn over that blur.
        if matches!(backdrop.kind, ferroui_base::rendering::composition::system_backdrops::PlatformSystemBackdropKind::DesktopAcrylic(_)) {
            let blur = crate::WindowTransparencyLevel::blur();
            platform_impl.set_transparency_level_hint(&[blur]);
            if platform_impl.transparency_level() == blur {
                self.blur_requested.set(true);
                return SystemBackdropSupport::BlurBehind;
            }
            platform_impl.set_transparency_level_hint(&top_level.transparency_level_hint());
        }
        self.restore_transparency(&top_level);
        SystemBackdropSupport::None
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        self.top_level.upgrade().and_then(|top_level| top_level.renderer().compositor())
    }
}

/// An element of a window as the target of a system backdrop (FerroUI's
/// form of the system backdrop of a menu flyout presenter): the element
/// draws the brush of the controller as its background. Where the element
/// is in a popup window of its own the platform cannot draw the material
/// behind the shape of the element, so it shows the fallback colour; in a
/// popup of the window (an overlay) it shows the material as in-app acrylic
/// of the window behind it (winui-materials.md).
pub struct ElementBackdropTarget {
    element: WeakRef<crate::primitives::TemplatedControl>,
    brush: RefCell<Option<Rc<CompositionBrush>>>,
    host_brush: Ref<SystemBackdropHostBrush>,
    background: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl ElementBackdropTarget {
    pub(crate) fn new(element: &crate::primitives::TemplatedControl) -> Rc<ElementBackdropTarget> {
        Rc::new(ElementBackdropTarget {
            element: element.to_ref().downgrade(),
            brush: RefCell::new(None),
            host_brush: SystemBackdropHostBrush::new(),
            background: RefCell::new(None),
        })
    }
}

impl ICompositionSupportsSystemBackdrop for ElementBackdropTarget {
    fn system_backdrop(&self) -> Option<Rc<CompositionBrush>> {
        self.brush.borrow().clone()
    }

    fn set_system_backdrop(&self, brush: Option<Rc<CompositionBrush>>) {
        *self.brush.borrow_mut() = brush.clone();
        self.host_brush.set_composition_brush(brush.clone());
        if let Some(previous) = self.background.borrow_mut().take() {
            previous.dispose();
        }
        let Some(element) = self.element.upgrade() else { return };
        if brush.is_some() {
            let host: Rc<dyn IBrush> = self.host_brush.clone().into();
            *self.background.borrow_mut() = element.set_value_with_priority(
                crate::primitives::TemplatedControl::background_property(),
                Some(host),
                ferroui_base::data::BindingPriority::StyleTrigger,
            );
        }
    }

    fn try_set_platform_system_backdrop(&self, backdrop: Option<&PlatformSystemBackdrop>) -> SystemBackdropSupport {
        let Some(element) = self.element.upgrade() else { return SystemBackdropSupport::None };
        if backdrop.is_none() {
            return SystemBackdropSupport::None;
        }
        // In a popup window of its own: nothing of the window is behind it.
        match TopLevel::get_top_level(Some(&element)) {
            Some(top_level) if top_level.is::<crate::primitives::PopupRoot>() => SystemBackdropSupport::None,
            Some(_) => SystemBackdropSupport::InApp,
            None => SystemBackdropSupport::None,
        }
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        let element = self.element.upgrade()?;
        let top_level = TopLevel::get_top_level(Some(&element))?;
        top_level.renderer().compositor()
    }
}

/// Connects `backdrop` to `element` as its target while the element is in a
/// window (the system backdrop of a menu flyout presenter, of a popup in a
/// window).
pub(crate) fn connect_element_backdrop(
    element: &crate::primitives::TemplatedControl,
    backdrop: &Ref<SystemBackdrop>,
    target: &Rc<ElementBackdropTarget>,
) -> bool {
    let Some(top_level) = TopLevel::get_top_level(Some(element)) else { return false };
    let target: Rc<dyn ICompositionSupportsSystemBackdrop> = target.clone();
    backdrop.on_target_connected(&target, &top_level);
    true
}

pub(crate) fn disconnect_element_backdrop(backdrop: &Ref<SystemBackdrop>, target: &Rc<ElementBackdropTarget>) {
    let target: Rc<dyn ICompositionSupportsSystemBackdrop> = target.clone();
    backdrop.on_target_disconnected(&target);
    target.set_system_backdrop(None);
}

/// The system backdrop of an element (the `SystemBackdrop` of a menu flyout
/// presenter or of a context menu): the element as the target of the
/// backdrop, connected while the element is in a window.
#[derive(Default)]
pub(crate) struct ElementSystemBackdrop {
    target: RefCell<Option<Rc<ElementBackdropTarget>>>,
    connected: RefCell<Option<Ref<SystemBackdrop>>>,
}

impl ElementSystemBackdrop {
    /// Connects `backdrop` to `element` (after disconnecting the backdrop
    /// it was connected to).
    pub(crate) fn connect(&self, element: &crate::primitives::TemplatedControl, backdrop: Option<Ref<SystemBackdrop>>) {
        self.disconnect();
        let Some(backdrop) = backdrop else { return };
        let target = self.target.borrow_mut().get_or_insert_with(|| ElementBackdropTarget::new(element)).clone();
        if connect_element_backdrop(element, &backdrop, &target) {
            *self.connected.borrow_mut() = Some(backdrop);
        }
    }

    /// Disconnects the backdrop the element is connected to.
    pub(crate) fn disconnect(&self) {
        let backdrop = self.connected.borrow_mut().take();
        let target = self.target.borrow().clone();
        if let (Some(backdrop), Some(target)) = (backdrop, target) {
            disconnect_element_backdrop(&backdrop, &target);
        }
    }
}

// --- SystemBackdrop -----------------------------------------------------------------------

/// The configuration of one target and what keeps it up to date
/// (`PerTargetConfigurationEntry`).
struct PerTargetConfiguration {
    target: Rc<dyn ICompositionSupportsSystemBackdrop>,
    configuration: Rc<SystemBackdropConfiguration>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}

/// The base of the system backdrops of the object model (WinUI
/// `Microsoft.UI.Xaml.Media.SystemBackdrop`): set on a window
/// ([`TopLevel::system_backdrop`](crate::TopLevel::system_backdrop)), it
/// is connected to the window as its target
/// ([`on_target_connected`](SystemBackdropImpl::on_target_connected)) and
/// keeps a default configuration of the target up to date: the theme of
/// the window, the high contrast setting of the platform and whether the
/// window is active.
#[repr(C)]
pub struct SystemBackdrop {
    base: FerroObject,
    configurations: RefCell<Vec<PerTargetConfiguration>>,
}

ferro_class! {
    SystemBackdrop: FerroObject, virtuals SystemBackdropImpl: FerroObjectImpl {
        /// `OnTargetConnected`: the backdrop was set on `xaml_root`, whose
        /// target is `connected_target`.
        fn on_target_connected(this, connected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>, xaml_root: &TopLevel);

        /// `OnTargetDisconnected`.
        fn on_target_disconnected(this, disconnected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>);

        /// `OnDefaultSystemBackdropConfigurationChanged`: the theme, the
        /// high contrast or the activation of the target changed.
        fn on_default_system_backdrop_configuration_changed(this, target: &Rc<dyn ICompositionSupportsSystemBackdrop>, xaml_root: &TopLevel);
    }
}
ferroui_base::ferro_class_info!(SystemBackdrop {
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(SystemBackdrop: FerroObjectImpl);

fn same_target(a: &Rc<dyn ICompositionSupportsSystemBackdrop>, b: &Rc<dyn ICompositionSupportsSystemBackdrop>) -> bool {
    Rc::ptr_eq(a, b)
}

/// The window whose activation a target follows: the window itself, or the
/// window a popup window belongs to.
fn activation_window(top_level: &TopLevel) -> Option<Ref<WindowBase>> {
    if let Some(window) = top_level.to_ref().cast::<WindowBase>() {
        return Some(window);
    }
    let popup_root = top_level.to_ref().cast::<crate::primitives::PopupRoot>()?;
    popup_root.parent_top_level()?.cast::<WindowBase>()
}

fn is_high_contrast() -> bool {
    FerroLocator::current()
        .get_service::<dyn IPlatformSettings>()
        .is_some_and(|settings| settings.get_color_values().contrast_preference() == ColorContrastPreference::High)
}

fn theme_of(top_level: &TopLevel) -> SystemBackdropTheme {
    if top_level.actual_theme_variant() == Some(ThemeVariant::dark()) {
        SystemBackdropTheme::Dark
    } else {
        SystemBackdropTheme::Light
    }
}

impl SystemBackdropImpl for SystemBackdrop {
    fn on_target_connected(this: &Self, connected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>, xaml_root: &TopLevel) {
        if this.configurations.borrow().iter().any(|entry| same_target(&entry.target, connected_target)) {
            return;
        }
        let configuration = SystemBackdropConfiguration::new();
        configuration.set_theme(theme_of(xaml_root));
        configuration.set_is_high_contrast(is_high_contrast());
        let window = activation_window(xaml_root);
        configuration.set_is_input_active(window.as_ref().is_none_or(|window| window.is_active()));

        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();
        let notify = {
            let backdrop = this.to_ref().downgrade();
            let target = Rc::downgrade(connected_target);
            let root = xaml_root.to_ref().downgrade();
            Rc::new(move || {
                if let (Some(backdrop), Some(target), Some(root)) = (backdrop.upgrade(), target.upgrade(), root.upgrade()) {
                    backdrop.on_default_system_backdrop_configuration_changed(&target, &root);
                }
            })
        };
        {
            let configuration = configuration.clone();
            let root = xaml_root.to_ref().downgrade();
            let notify = notify.clone();
            subscriptions.push(xaml_root.actual_theme_variant_changed(move || {
                if let Some(root) = root.upgrade() {
                    configuration.set_theme(theme_of(&root));
                    notify();
                }
            }));
        }
        if let Some(settings) = FerroLocator::current().get_service::<dyn IPlatformSettings>() {
            let configuration = configuration.clone();
            let notify = notify.clone();
            subscriptions.push(settings.color_values_changed(Rc::new(move |values| {
                configuration.set_is_high_contrast(values.contrast_preference() == ColorContrastPreference::High);
                notify();
            })));
        }
        if let Some(window) = window {
            for active in [true, false] {
                let configuration = configuration.clone();
                let notify = notify.clone();
                let handler = move || {
                    configuration.set_is_input_active(active);
                    notify();
                };
                subscriptions.push(if active { window.activated(handler) } else { window.deactivated(handler) });
            }
        }
        this.configurations.borrow_mut().push(PerTargetConfiguration {
            target: connected_target.clone(),
            configuration,
            subscriptions,
        });
    }

    fn on_target_disconnected(this: &Self, disconnected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>) {
        let removed = {
            let mut configurations = this.configurations.borrow_mut();
            let index = configurations.iter().position(|entry| same_target(&entry.target, disconnected_target));
            index.map(|index| configurations.remove(index))
        };
        if let Some(entry) = removed {
            for subscription in entry.subscriptions {
                subscription.dispose();
            }
        }
    }

    fn on_default_system_backdrop_configuration_changed(
        _this: &Self,
        _target: &Rc<dyn ICompositionSupportsSystemBackdrop>,
        _xaml_root: &TopLevel,
    ) {
    }
}

impl SystemBackdrop {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), configurations: RefCell::new(Vec::new()) }
    }

    /// `GetDefaultSystemBackdropConfiguration`: the configuration the
    /// backdrop keeps for `target` (`None` for a target it is not
    /// connected to).
    pub fn get_default_system_backdrop_configuration(
        &self,
        target: &Rc<dyn ICompositionSupportsSystemBackdrop>,
        _xaml_root: Option<&TopLevel>,
    ) -> Option<Rc<SystemBackdropConfiguration>> {
        self.configurations
            .borrow()
            .iter()
            .find(|entry| same_target(&entry.target, target))
            .map(|entry| entry.configuration.clone())
    }
}

// --- MicaBackdrop and DesktopAcrylicBackdrop -------------------------------------------

/// A controller and its target (`ControllerEntry`).
struct ControllerEntry {
    target: Rc<dyn ICompositionSupportsSystemBackdrop>,
    controller: Rc<SystemBackdropController>,
    mica: Option<MicaController>,
}

impl Drop for ControllerEntry {
    fn drop(&mut self) {
        self.controller.remove_system_backdrop_target(&self.target);
        self.controller.close();
    }
}

/// The Mica backdrop of a window (WinUI `Microsoft.UI.Xaml.Media.MicaBackdrop`):
/// a [`MicaController`] of its [`kind`](Self::kind) for each target.
#[repr(C)]
pub struct MicaBackdrop {
    base: SystemBackdrop,
    controllers: RefCell<Vec<ControllerEntry>>,
}

ferro_class!(MicaBackdrop: SystemBackdrop);
ferroui_base::ferro_class_info!(MicaBackdrop {
    new: MicaBackdrop::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});

impl FerroObjectImpl for MicaBackdrop {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::kind_property().as_property() {
            for entry in this.controllers.borrow().iter() {
                if let Some(mica) = &entry.mica {
                    mica.set_kind(this.kind());
                }
            }
        }
    }
}

impl SystemBackdropImpl for MicaBackdrop {
    fn on_target_connected(this: &Self, connected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>, xaml_root: &TopLevel) {
        Self::parent_on_target_connected(this, connected_target, xaml_root);
        let controller = MicaController::new();
        controller.set_kind(this.kind());
        let configuration = this.get_default_system_backdrop_configuration(connected_target, Some(xaml_root));
        controller.add_system_backdrop_target(connected_target.clone());
        controller.set_system_backdrop_configuration(configuration);
        this.controllers.borrow_mut().push(ControllerEntry {
            target: connected_target.clone(),
            controller: controller.controller().clone(),
            mica: Some(controller),
        });
    }

    fn on_target_disconnected(this: &Self, disconnected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>) {
        Self::parent_on_target_disconnected(this, disconnected_target);
        this.controllers.borrow_mut().retain(|entry| !same_target(&entry.target, disconnected_target));
    }

    fn on_default_system_backdrop_configuration_changed(
        this: &Self,
        target: &Rc<dyn ICompositionSupportsSystemBackdrop>,
        xaml_root: &TopLevel,
    ) {
        Self::parent_on_default_system_backdrop_configuration_changed(this, target, xaml_root);
    }
}

ferroui_base::ferro_properties! { impl MicaBackdrop {
    ferro_property!(
        /// `Kind` (default `Base`).
        pub fn kind_property() -> StyledProperty<MicaKind> {
            FerroProperty::register::<MicaBackdrop, _>("Kind", MicaKind::Base)
        }
    );
} }

impl MicaBackdrop {
    pub fn construct() -> Self {
        Self { base: SystemBackdrop::construct(), controllers: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn kind(&self) -> MicaKind {
        self.get_value(Self::kind_property())
    }

    pub fn set_kind(&self, value: MicaKind) {
        self.set_value(Self::kind_property(), value)
    }

    /// The controller of `target`, while the backdrop is connected to it.
    pub fn controller_of(&self, target: &Rc<dyn ICompositionSupportsSystemBackdrop>) -> Option<Rc<SystemBackdropController>> {
        self.controllers.borrow().iter().find(|entry| same_target(&entry.target, target)).map(|entry| entry.controller.clone())
    }
}

/// The desktop acrylic backdrop of a window (WinUI
/// `Microsoft.UI.Xaml.Media.DesktopAcrylicBackdrop`): a
/// [`DesktopAcrylicController`] for each target.
#[repr(C)]
pub struct DesktopAcrylicBackdrop {
    base: SystemBackdrop,
    controllers: RefCell<Vec<ControllerEntry>>,
}

ferro_class!(DesktopAcrylicBackdrop: SystemBackdrop);
ferroui_base::ferro_class_info!(DesktopAcrylicBackdrop {
    new: DesktopAcrylicBackdrop::new,
    markup: {
        namespace: "FerroUI.Media",
    },
});
ferro_impl_classes!(DesktopAcrylicBackdrop: FerroObjectImpl);

impl SystemBackdropImpl for DesktopAcrylicBackdrop {
    fn on_target_connected(this: &Self, connected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>, xaml_root: &TopLevel) {
        Self::parent_on_target_connected(this, connected_target, xaml_root);
        let controller = DesktopAcrylicController::new();
        let configuration = this.get_default_system_backdrop_configuration(connected_target, Some(xaml_root));
        controller.add_system_backdrop_target(connected_target.clone());
        controller.set_system_backdrop_configuration(configuration);
        this.controllers.borrow_mut().push(ControllerEntry {
            target: connected_target.clone(),
            controller: controller.controller().clone(),
            mica: None,
        });
    }

    fn on_target_disconnected(this: &Self, disconnected_target: &Rc<dyn ICompositionSupportsSystemBackdrop>) {
        Self::parent_on_target_disconnected(this, disconnected_target);
        this.controllers.borrow_mut().retain(|entry| !same_target(&entry.target, disconnected_target));
    }

    fn on_default_system_backdrop_configuration_changed(
        this: &Self,
        target: &Rc<dyn ICompositionSupportsSystemBackdrop>,
        xaml_root: &TopLevel,
    ) {
        Self::parent_on_default_system_backdrop_configuration_changed(this, target, xaml_root);
    }
}

impl DesktopAcrylicBackdrop {
    pub fn construct() -> Self {
        Self { base: SystemBackdrop::construct(), controllers: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The controller of `target`, while the backdrop is connected to it.
    pub fn controller_of(&self, target: &Rc<dyn ICompositionSupportsSystemBackdrop>) -> Option<Rc<SystemBackdropController>> {
        self.controllers.borrow().iter().find(|entry| same_target(&entry.target, target)).map(|entry| entry.controller.clone())
    }
}
