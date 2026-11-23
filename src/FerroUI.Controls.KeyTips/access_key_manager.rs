//! WinUI access keys: `AccessKey`, scopes, the Alt activation and invoking a control by its key.
//!
//! Nothing runs until an element has an access key. From then on one class handler on the roots
//! looks at Alt and, while the Key Tips are showing, at what is typed.

use crate::key_tip_math::{match_keys, normalize_key, KeyMatch, KeyTipPlacementMode};
use crate::key_tip_overlay::{KeyTipOverlay, ShownTip};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, PointerPressedEventArgs};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_markup_enum, ferro_properties, ferro_routed_event, ferro_routed_event_args,
    ferro_static_type, AttachedProperty, FerroProperty, FerroPropertyChangedEventArgs, Ref, Visual,
    WeakRef,
};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, ISelectionItemProvider, IToggleProvider,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Control, MenuItem, TopLevel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

ferro_markup_enum!(
    KeyTipPlacementMode { Auto, Bottom, Top, Left, Right, Center, Hidden },
    { namespace: "FerroUI.Controls.KeyTips" }
);

/// Raised on an element when its access key is typed. A handler that sets it handled replaces
/// what the element would do.
#[derive(Clone)]
pub struct AccessKeyInvokedEventArgs {
    base: RoutedEventArgs,
}

ferro_routed_event_args!(AccessKeyInvokedEventArgs: RoutedEventArgs);

impl AccessKeyInvokedEventArgs {
    pub fn new() -> Self {
        Self {
            base: RoutedEventArgs::with_event(AccessKeyManager::access_key_invoked_event()),
        }
    }
}

impl Default for AccessKeyInvokedEventArgs {
    fn default() -> Self {
        Self::new()
    }
}

/// The access keys of an application: the attached properties, and the Key Tips display mode.
pub struct AccessKeyManager;

ferro_static_type!(AccessKeyManager);

impl AccessKeyManager {
    ferro_routed_event!(
        /// Defines the `AccessKeyInvoked` event.
        pub fn access_key_invoked_event() -> RoutedEvent<AccessKeyInvokedEventArgs> {
            RoutedEvent::register::<AccessKeyManager, _>(
                "AccessKeyInvoked",
                RoutingStrategies::BUBBLE,
            )
        }
    );
}

ferro_properties! {
    impl AccessKeyManager {
        /// Defines the `AccessKey` attached property: the key, of one or more characters.
        pub fn access_key_property() -> AttachedProperty<Option<String>> {
            let property =
                FerroProperty::register_attached::<AccessKeyManager, Control, _>("AccessKey", None);
            property.changed().subscribe(on_access_key_changed).detach();
            property
        }

        /// Defines the `IsAccessKeyScope` attached property: the keys inside the element are
        /// shown once its own key was typed.
        pub fn is_access_key_scope_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<AccessKeyManager, Control, _>("IsAccessKeyScope", false)
        }

        /// Defines the `AccessKeyScopeOwner` attached property: the scope of the element, where
        /// it is not the nearest ancestor that is a scope.
        pub fn access_key_scope_owner_property() -> AttachedProperty<Option<Ref<Control>>> {
            FerroProperty::register_attached::<AccessKeyManager, Control, _>("AccessKeyScopeOwner", None)
        }

        /// Defines the `ExitDisplayModeOnAccessKeyInvoked` attached property.
        pub fn exit_display_mode_on_access_key_invoked_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<AccessKeyManager, Control, _>(
                "ExitDisplayModeOnAccessKeyInvoked",
                true,
            )
        }

        /// Defines the `KeyTipPlacementMode` attached property.
        pub fn key_tip_placement_mode_property() -> AttachedProperty<KeyTipPlacementMode> {
            FerroProperty::register_attached::<AccessKeyManager, Control, _>(
                "KeyTipPlacementMode",
                KeyTipPlacementMode::Auto,
            )
        }

        /// Defines the `KeyTipHorizontalOffset` attached property.
        pub fn key_tip_horizontal_offset_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<AccessKeyManager, Control, _>("KeyTipHorizontalOffset", 0.0)
        }

        /// Defines the `KeyTipVerticalOffset` attached property.
        pub fn key_tip_vertical_offset_property() -> AttachedProperty<f64> {
            FerroProperty::register_attached::<AccessKeyManager, Control, _>("KeyTipVerticalOffset", 0.0)
        }
    }
}

/// The display mode of the Key Tips.
struct Display {
    /// The window the keys of the outermost scope belong to.
    root: WeakRef<Visual>,
    /// The scopes entered, innermost last.
    scopes: Vec<WeakRef<Control>>,
    typed: String,
    overlays: Vec<(WeakRef<AdornerLayer>, Ref<KeyTipOverlay>)>,
}

struct Session {
    targets: RefCell<Vec<WeakRef<Control>>>,
    display: RefCell<Option<Display>>,
    is_alt_down: Cell<bool>,
    /// Another key went down with Alt: its release does not show the tips.
    is_alt_chord: Cell<bool>,
    /// Whether Alt and a key together can invoke. Not for AltGr or the Option key of a Mac, with
    /// which people type characters.
    can_chord: Cell<bool>,
    are_key_tips_enabled: Cell<bool>,
    changed: HandlerList<dyn Fn()>,
}

ferroui_base::ferro_thread_local! {
    static SESSION: RefCell<Option<Rc<Session>>> = const { RefCell::new(None) };
}

fn session() -> Option<Rc<Session>> {
    SESSION.with(|session| session.borrow().clone())
}

/// The session of this thread; the first access key installs the handlers on the roots.
fn ensure_session() -> Rc<Session> {
    if let Some(session) = session() {
        return session;
    }
    let created = Rc::new(Session {
        targets: RefCell::new(Vec::new()),
        display: RefCell::new(None),
        is_alt_down: Cell::new(false),
        is_alt_chord: Cell::new(false),
        can_chord: Cell::new(false),
        are_key_tips_enabled: Cell::new(true),
        changed: HandlerList::new(),
    });
    SESSION.with(|session| *session.borrow_mut() = Some(created.clone()));
    InputElement::key_down_event()
        .add_class_handler_with::<TopLevel>(on_key_down, RoutingStrategies::TUNNEL, false)
        .detach();
    InputElement::key_up_event()
        .add_class_handler_with::<TopLevel>(on_key_up, RoutingStrategies::TUNNEL, false)
        .detach();
    InputElement::pointer_pressed_event()
        .add_class_handler_with::<TopLevel>(on_pointer_pressed, RoutingStrategies::TUNNEL, true)
        .detach();
    created
}

fn on_access_key_changed(e: &FerroPropertyChangedEventArgs<'_>) {
    let Some(control) = e.sender().downcast_ref::<Control>() else {
        return;
    };
    let session = ensure_session();
    let key = e.get_new_value::<Option<String>>();
    {
        let mut targets = session.targets.borrow_mut();
        targets.retain(|target| target.upgrade().is_some());
        let this = control.to_ref();
        if !targets
            .iter()
            .any(|target| target.upgrade().as_ref() == Some(&this))
        {
            targets.push(this.downgrade());
        }
    }
    // Assistive technology reads the same key.
    if let Some(key) = key.filter(|key| !key.trim().is_empty()) {
        if AutomationProperties::get_access_key(control).is_none() {
            AutomationProperties::set_access_key(control, &key);
        }
    }
    session.refresh();
}

fn root_of(sender: &TopLevel) -> Ref<Visual> {
    sender.to_ref().upcast()
}

/// The top-level of a visual: a window, or the root of a popup.
fn top_level_of(visual: &Visual) -> Option<Ref<Visual>> {
    TopLevel::get_top_level(Some(visual)).map(|top_level| top_level.upcast())
}

fn on_key_down(sender: &TopLevel, e: &KeyEventArgs) {
    let Some(session) = session() else { return };
    let is_alt = matches!(e.key, Key::LeftAlt | Key::RightAlt);
    if session.is_showing() {
        if is_alt {
            session.is_alt_down.set(true);
            session.is_alt_chord.set(false);
            e.set_handled(true);
        } else if e.key == Key::Escape {
            session.leave_scope();
            e.set_handled(true);
        } else if let Some(text) = typed_text(e) {
            session.type_text(&text);
            e.set_handled(true);
        } else if !is_modifier(e.key) {
            // Another key goes on to its target and ends the display mode.
            session.exit();
        }
        return;
    }
    if !session.are_key_tips_enabled.get() || !session.has_keys_in(&root_of(sender)) {
        return;
    }
    if is_alt {
        // Used here, so that Alt does not also open the menu of the window.
        session.is_alt_down.set(true);
        session.is_alt_chord.set(false);
        session
            .can_chord
            .set(e.key == Key::LeftAlt && !cfg!(target_vendor = "apple"));
        e.set_handled(true);
    } else if session.is_alt_down.get() {
        session.is_alt_chord.set(true);
        if !session.can_chord.get() {
            return;
        }
        if let Some(text) = typed_text(e) {
            // Alt and a key: the same as Alt, then the key, when it is a key of this window.
            if session.starts_a_key(&root_of(sender), &text) {
                session.enter(&root_of(sender));
                session.type_text(&text);
                e.set_handled(true);
            }
        }
    }
}

fn on_key_up(sender: &TopLevel, e: &KeyEventArgs) {
    let Some(session) = session() else { return };
    if !matches!(e.key, Key::LeftAlt | Key::RightAlt) || !session.is_alt_down.replace(false) {
        return;
    }
    let was_chord = session.is_alt_chord.replace(false);
    e.set_handled(true);
    if was_chord {
        return;
    }
    if session.is_showing() {
        session.exit();
    } else if session.are_key_tips_enabled.get() {
        session.enter(&root_of(sender));
    }
}

fn on_pointer_pressed(_sender: &TopLevel, _e: &PointerPressedEventArgs) {
    if let Some(session) = session() {
        session.exit();
    }
}

fn is_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::LeftShift | Key::RightShift | Key::LeftCtrl | Key::RightCtrl | Key::LWin | Key::RWin
    )
}

/// The text of a key that can be part of an access key.
fn typed_text(e: &KeyEventArgs) -> Option<String> {
    if e.key_modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
    {
        return None;
    }
    let symbol = e.key_symbol.as_deref()?;
    let is_text = !symbol.is_empty()
        && symbol
            .chars()
            .all(|c| !c.is_control() && !c.is_whitespace());
    is_text.then(|| normalize_key(symbol))
}

fn is_scope(control: &Control) -> bool {
    AccessKeyManager::get_is_access_key_scope(control)
        || control
            .to_ref()
            .cast::<MenuItem>()
            .is_some_and(|item| item.has_sub_menu())
}

/// The scope an element belongs to: its owner, or its nearest ancestor that is a scope.
fn scope_of(control: &Control) -> Option<Ref<Control>> {
    if let Some(owner) = AccessKeyManager::get_access_key_scope_owner(control) {
        return Some(owner);
    }
    let mut parent = control.parent();
    while let Some(element) = parent {
        if let Some(candidate) = element.cast::<Control>() {
            if is_scope(&candidate) {
                return Some(candidate);
            }
        }
        parent = element.parent();
    }
    None
}

fn can_show(control: &Control) -> bool {
    control.is_effectively_enabled()
        && control.is_effectively_visible()
        && control.visual_root().is_some()
}

fn key_of(control: &Control) -> Option<String> {
    AccessKeyManager::get_access_key(control)
        .map(|key| normalize_key(&key))
        .filter(|key| !key.is_empty())
}

/// What the key of an element does when nothing handled it.
fn run_default_action(control: &Ref<Control>) {
    let peer = ControlAutomationPeer::create_peer_for_element(control);
    if let Some(expand) = peer.get_provider::<dyn IExpandCollapseProvider>() {
        if is_scope(control) {
            let _ = expand.expand();
            return;
        }
    }
    if let Some(invoke) = peer.get_provider::<dyn IInvokeProvider>() {
        let _ = invoke.invoke();
    } else if let Some(toggle) = peer.get_provider::<dyn IToggleProvider>() {
        let _ = toggle.toggle();
    } else if let Some(select) = peer.get_provider::<dyn ISelectionItemProvider>() {
        let _ = select.select();
    } else if let Some(expand) = peer.get_provider::<dyn IExpandCollapseProvider>() {
        let _ = expand.expand();
    } else {
        control.focus();
    }
}

impl Session {
    fn is_showing(&self) -> bool {
        self.display.borrow().is_some()
    }

    fn live_targets(&self) -> Vec<Ref<Control>> {
        self.targets
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .collect()
    }

    fn has_keys_in(&self, root: &Ref<Visual>) -> bool {
        self.targets
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .any(|target| {
                top_level_of(&target).as_ref() == Some(root)
                    && key_of(&target).is_some()
                    && can_show(&target)
            })
    }

    /// The elements of a scope that can show a key, with their keys.
    fn shown_in(
        &self,
        root: &Ref<Visual>,
        scope: Option<&Ref<Control>>,
    ) -> Vec<(Ref<Control>, String)> {
        self.live_targets()
            .into_iter()
            .filter(|target| can_show(target))
            .filter(|target| scope_of(target).as_ref() == scope)
            // The outermost scope is one window; a scope's own elements may be in its popup.
            .filter(|target| scope.is_some() || top_level_of(target).as_ref() == Some(root))
            .filter_map(|target| key_of(&target).map(|key| (target, key)))
            .collect()
    }

    fn current(&self) -> Vec<(Ref<Control>, String)> {
        let (root, scope) = {
            let display = self.display.borrow();
            let Some(display) = display.as_ref() else {
                return Vec::new();
            };
            let scope = display.scopes.last().and_then(WeakRef::upgrade);
            (display.root.upgrade(), scope)
        };
        root.map_or_else(Vec::new, |root| self.shown_in(&root, scope.as_ref()))
    }

    fn starts_a_key(&self, root: &Ref<Visual>, text: &str) -> bool {
        let keys: Vec<String> = self
            .shown_in(root, None)
            .into_iter()
            .map(|(_, key)| key)
            .collect();
        !matches!(match_keys(&keys, text), KeyMatch::None)
    }

    fn enter(self: &Rc<Self>, root: &Ref<Visual>) {
        if self.is_showing() || self.shown_in(root, None).is_empty() {
            return;
        }
        *self.display.borrow_mut() = Some(Display {
            root: root.downgrade(),
            scopes: Vec::new(),
            typed: String::new(),
            overlays: Vec::new(),
        });
        self.refresh();
        self.raise_changed();
    }

    fn exit(&self) {
        let display = self.display.borrow_mut().take();
        let Some(display) = display else { return };
        for (layer, overlay) in display.overlays {
            if let Some(layer) = layer.upgrade() {
                layer
                    .children()
                    .remove(&overlay.clone().upcast::<Control>());
            }
        }
        self.raise_changed();
    }

    fn raise_changed(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }

    /// Escape: out of the innermost scope, or out of the display mode.
    fn leave_scope(self: &Rc<Self>) {
        let left = {
            let mut display = self.display.borrow_mut();
            let Some(display) = display.as_mut() else {
                return;
            };
            display.typed.clear();
            display.scopes.pop()
        };
        match left {
            Some(scope) => {
                // A menu that was opened by its key closes with its scope.
                if let Some(item) = scope.upgrade().and_then(|scope| scope.cast::<MenuItem>()) {
                    item.close();
                }
                self.refresh();
            }
            None => self.exit(),
        }
    }

    fn type_text(self: &Rc<Self>, text: &str) {
        let typed = {
            let mut display = self.display.borrow_mut();
            let Some(display) = display.as_mut() else {
                return;
            };
            display.typed.push_str(text);
            display.typed.clone()
        };
        let shown = self.current();
        let keys: Vec<&str> = shown.iter().map(|(_, key)| key.as_str()).collect();
        match match_keys(&keys, &typed) {
            KeyMatch::Exact(index) => {
                let target = shown[index].0.clone();
                self.invoke(&target);
            }
            KeyMatch::Partial => self.refresh(),
            KeyMatch::None => {
                // A key that matches nothing starts over.
                if let Some(display) = self.display.borrow_mut().as_mut() {
                    display.typed.clear();
                }
                self.refresh();
            }
        }
    }

    fn invoke(self: &Rc<Self>, target: &Ref<Control>) {
        let enters_scope = is_scope(target);
        let args = AccessKeyInvokedEventArgs::new();
        target.raise_event(&args);
        if !args.handled() {
            run_default_action(target);
        }
        if enters_scope {
            if let Some(display) = self.display.borrow_mut().as_mut() {
                display.scopes.push(target.downgrade());
                display.typed.clear();
            }
            self.refresh();
            // A menu places its items after it opens: their tips follow.
            let session = Rc::downgrade(self);
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(session) = session.upgrade() {
                        session.refresh();
                    }
                },
                DispatcherPriority::LOADED,
            );
        } else if AccessKeyManager::get_exit_display_mode_on_access_key_invoked(target) {
            self.exit();
        } else {
            if let Some(display) = self.display.borrow_mut().as_mut() {
                display.typed.clear();
            }
            self.refresh();
        }
    }

    /// Shows what is current: each window that has tips gets one overlay.
    fn refresh(&self) {
        if !self.is_showing() {
            return;
        }
        let typed = self
            .display
            .borrow()
            .as_ref()
            .map_or_else(String::new, |d| d.typed.clone());
        let shown = self.current();
        let mut by_layer: Vec<(Ref<AdornerLayer>, Vec<ShownTip>)> = Vec::new();
        for (target, key) in &shown {
            if !key.starts_with(&typed) {
                continue;
            }
            let Some(layer) = AdornerLayer::get_adorner_layer(target) else {
                continue;
            };
            let tip = ShownTip {
                target: target.downgrade(),
                text: key.clone(),
            };
            match by_layer.iter_mut().find(|(known, _)| *known == layer) {
                Some((_, tips)) => tips.push(tip),
                None => by_layer.push((layer, vec![tip])),
            }
        }
        let mut display = self.display.borrow_mut();
        let Some(display) = display.as_mut() else {
            return;
        };
        display.overlays.retain(|(layer, overlay)| {
            let is_used = layer
                .upgrade()
                .is_some_and(|layer| by_layer.iter().any(|(known, _)| *known == layer));
            if !is_used {
                if let Some(layer) = layer.upgrade() {
                    layer
                        .children()
                        .remove(&overlay.clone().upcast::<Control>());
                }
            }
            is_used
        });
        for (layer, tips) in by_layer {
            let known = display
                .overlays
                .iter()
                .find(|(held, _)| held.upgrade().as_ref() == Some(&layer))
                .map(|(_, overlay)| overlay.clone());
            let overlay = known.unwrap_or_else(|| {
                let overlay = KeyTipOverlay::new();
                layer.children().add(overlay.clone().upcast::<Control>());
                display.overlays.push((layer.downgrade(), overlay.clone()));
                overlay
            });
            overlay.set_tips(tips);
        }
    }
}

impl AccessKeyManager {
    pub fn get_access_key(element: &Control) -> Option<String> {
        element.get_value(Self::access_key_property())
    }

    pub fn set_access_key(element: &Control, value: Option<&str>) {
        element.set_value(Self::access_key_property(), value.map(str::to_owned));
    }

    pub fn get_is_access_key_scope(element: &Control) -> bool {
        element.get_value(Self::is_access_key_scope_property())
    }

    pub fn set_is_access_key_scope(element: &Control, value: bool) {
        element.set_value(Self::is_access_key_scope_property(), value);
    }

    pub fn get_access_key_scope_owner(element: &Control) -> Option<Ref<Control>> {
        element.get_value(Self::access_key_scope_owner_property())
    }

    pub fn set_access_key_scope_owner(element: &Control, value: Option<Ref<Control>>) {
        element.set_value(Self::access_key_scope_owner_property(), value);
    }

    pub fn get_exit_display_mode_on_access_key_invoked(element: &Control) -> bool {
        element.get_value(Self::exit_display_mode_on_access_key_invoked_property())
    }

    pub fn set_exit_display_mode_on_access_key_invoked(element: &Control, value: bool) {
        element.set_value(
            Self::exit_display_mode_on_access_key_invoked_property(),
            value,
        );
    }

    pub fn get_key_tip_placement_mode(element: &Control) -> KeyTipPlacementMode {
        element.get_value(Self::key_tip_placement_mode_property())
    }

    pub fn set_key_tip_placement_mode(element: &Control, value: KeyTipPlacementMode) {
        element.set_value(Self::key_tip_placement_mode_property(), value);
    }

    pub fn get_key_tip_horizontal_offset(element: &Control) -> f64 {
        element.get_value(Self::key_tip_horizontal_offset_property())
    }

    pub fn set_key_tip_horizontal_offset(element: &Control, value: f64) {
        element.set_value(Self::key_tip_horizontal_offset_property(), value);
    }

    pub fn get_key_tip_vertical_offset(element: &Control) -> f64 {
        element.get_value(Self::key_tip_vertical_offset_property())
    }

    pub fn set_key_tip_vertical_offset(element: &Control, value: f64) {
        element.set_value(Self::key_tip_vertical_offset_property(), value);
    }

    /// Whether the Key Tips are showing.
    pub fn is_display_mode_enabled() -> bool {
        session().is_some_and(|session| session.is_showing())
    }

    /// Whether Alt shows the Key Tips.
    pub fn are_key_tips_enabled() -> bool {
        session().is_none_or(|session| session.are_key_tips_enabled.get())
    }

    /// Sets whether Alt shows the Key Tips. Turning them off leaves the display mode.
    pub fn set_are_key_tips_enabled(value: bool) {
        let session = ensure_session();
        session.are_key_tips_enabled.set(value);
        if !value {
            session.exit();
        }
    }

    /// Shows the Key Tips of the window of `element`.
    pub fn enter_display_mode(element: &Visual) {
        let session = ensure_session();
        if !session.are_key_tips_enabled.get() {
            return;
        }
        if let Some(root) = top_level_of(element) {
            session.enter(&root);
        }
    }

    /// Hides the Key Tips.
    pub fn exit_display_mode() {
        if let Some(session) = session() {
            session.exit();
        }
    }

    /// Calls `handler` when the Key Tips show or hide.
    pub fn is_display_mode_enabled_changed(handler: impl Fn() + 'static) -> Subscription {
        let session = ensure_session();
        let token = session.changed.add(Rc::new(handler));
        let weak = Rc::downgrade(&session);
        Subscription::new(Disposable::create(move || {
            if let Some(session) = weak.upgrade() {
                session.changed.remove(token);
            }
        }))
    }

    /// The keys showing now, in the current scope and matching what was typed.
    pub fn visible_key_tips() -> Vec<String> {
        let Some(session) = session() else {
            return Vec::new();
        };
        let typed = session
            .display
            .borrow()
            .as_ref()
            .map_or_else(String::new, |d| d.typed.clone());
        let mut keys: Vec<String> = session
            .current()
            .into_iter()
            .map(|(_, key)| key)
            .filter(|key| key.starts_with(&typed))
            .collect();
        keys.sort();
        keys
    }
}
