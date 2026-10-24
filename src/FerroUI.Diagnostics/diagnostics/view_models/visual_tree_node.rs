//! Port of `Diagnostics/ViewModels/VisualTreeNode.cs`.

use super::{TreeNode, TreeNodeCollection, TreeNodeCollectionImpl, TreeNodeKind};
use crate::diagnostics::controls::Application;
use crate::diagnostics::views::MainWindow;
use ferroui_base::collections::{FerroList, FerroListExtensions};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::reactive::{CompositeDisposable, IDisposable, IObservable, IObserver, Observable, ObservableExt};
use ferroui_base::{FerroObject, FerroObjectExtensions, Nullable, Ref, StyledElement, Visual};
use ferroui_controls::diagnostics::{IPopupHostProvider, ToolTipDiagnostics};
use ferroui_controls::primitives::{FlyoutBase, IPopupHost, Popup, PopupFlyoutBase};
use ferroui_controls::{Button, ContextMenu, Control, Window};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A node of the visual tree.
///
/// The original derives the node class from [`TreeNode`]; here a visual
/// tree node is a [`TreeNode`] of the kind [`TreeNodeKind::Visual`], made by
/// the functions of this type.
pub struct VisualTreeNode;

impl VisualTreeNode {
    /// `VisualTreeNode(FerroObject ferroObject, TreeNode? parent, string? customName = null)`:
    /// the node of an object of the visual tree, whose children are the
    /// visual children of the visual and the root of the popup it hosts, or
    /// the top levels of an application.
    pub fn new(ferro_object: Ref<FerroObject>, parent: Option<&Rc<TreeNode>>, custom_name: Option<String>) -> Rc<TreeNode> {
        let visual = ferro_object.cast::<Visual>();
        let host = ferro_object.cast::<Application>();
        let is_in_template = ferro_object
            .cast::<StyledElement>()
            .is_some_and(|styleable| styleable.templated_parent().is_some());
        TreeNode::new_node(ferro_object, parent, custom_name, TreeNodeKind::Visual { is_in_template }, move |this| {
            if let Some(visual) = visual {
                VisualTreeNodeCollection::new(this, visual)
            } else if let Some(host) = host {
                ApplicationHostVisuals::new(this, host)
            } else {
                TreeNodeCollection::empty()
            }
        })
    }

    /// `Create(object control)`: the root node of the visual tree of
    /// `control`, or no node when it is not an object of the framework.
    pub fn create(control: impl Into<Nullable<FerroObject>>) -> Vec<Rc<TreeNode>> {
        match control.into().0 {
            Some(visual) => vec![VisualTreeNode::new(visual, None, None)],
            None => Vec::new(),
        }
    }
}

/// The root of a popup that a visual hosts: the control at the root of the
/// popup and the name its node is shown with.
#[derive(Clone)]
struct PopupRoot {
    root: Ref<Control>,
    custom_name: Option<String>,
}

impl PopupRoot {
    fn new(root: Ref<Control>, custom_name: Option<String>) -> Self {
        Self { root, custom_name }
    }
}

/// `VisualTreeNode.VisualTreeNodeCollection`: the nodes of the visual
/// children of a visual, and of the root of the popup it hosts, following
/// them.
pub struct VisualTreeNodeCollection {
    control: Ref<Visual>,
    subscriptions: CompositeDisposable,
}

impl VisualTreeNodeCollection {
    pub fn new(owner: Weak<TreeNode>, control: Ref<Visual>) -> Rc<TreeNodeCollection> {
        TreeNodeCollection::new(owner, Box::new(Self { control, subscriptions: CompositeDisposable::with_capacity(2) }))
    }

    /// The roots of the popups a visual hosts: the host of the popup of a
    /// popup; for a control, the context menu itself, or the host of its
    /// context flyout, attached flyout, tooltip or button flyout, whichever
    /// is set first in that order. `None` for any other visual.
    fn get_hosted_popup_root_observable(visual: &Ref<Visual>) -> Option<Rc<dyn IObservable<Option<PopupRoot>>>> {
        fn get_popup_host_observable(
            popup_host_provider: Rc<dyn IPopupHostProvider>,
            provider_name: Option<&'static str>,
        ) -> Rc<dyn IObservable<Option<PopupRoot>>> {
            let provider = popup_host_provider.clone();
            Observable::create(move |observer: Rc<dyn IObserver<Option<Rc<dyn IPopupHost>>>>| {
                let handler: Rc<dyn Fn(Option<&Rc<dyn IPopupHost>>)> =
                    Rc::new(move |args: Option<&Rc<dyn IPopupHost>>| observer.on_next(args.cloned()));
                provider.popup_host_changed(handler)
            })
            .start_with(popup_host_provider.popup_host())
            .select(move |popup_host: Option<Rc<dyn IPopupHost>>| {
                // Every popup host of the framework is a control.
                popup_host.map(|popup_host| {
                    let control = popup_host.as_control();
                    let custom_name =
                        provider_name.map(|provider_name| format!("{provider_name} ({})", control.get_type().name()));
                    PopupRoot::new(control, custom_name)
                })
            })
        }

        if let Some(p) = visual.cast::<Popup>() {
            return Some(get_popup_host_observable(p.to_popup_host_provider(), None));
        }

        let c = visual.cast::<Control>()?;

        fn object<T: ferroui_base::ObjectType + ferroui_base::Upcast<FerroObject>>(
            value: Option<Ref<T>>,
        ) -> Option<Ref<FerroObject>> {
            value.map(|value| value.upcast())
        }

        // The property observables of the object (`Interactive` has a `get_observable` of its own,
        // for routed events).
        let o: &FerroObject = &c;
        let observables: Vec<Rc<dyn IObservable<Option<Ref<FerroObject>>>>> = vec![
            FerroObjectExtensions::get_observable(o, Control::context_flyout_property()).select(object),
            FerroObjectExtensions::get_observable(o, Control::context_menu_property()).select(object),
            FerroObjectExtensions::get_observable(o, FlyoutBase::attached_flyout_property()).select(object),
            FerroObjectExtensions::get_observable(o, ToolTipDiagnostics::tool_tip_property()).select(object),
            FerroObjectExtensions::get_observable(o, Button::flyout_property()).select(object),
        ];

        let as_popup_host_provider = |item: &Option<Ref<FerroObject>>| -> Option<Rc<dyn IPopupHostProvider>> {
            item.as_ref().and_then(|item| item.cast::<PopupFlyoutBase>()).map(|flyout| flyout.to_popup_host_provider())
        };

        let popup_roots = Observable::combine_latest_all(observables).select(
            move |items: Vec<Option<Ref<FerroObject>>>| -> Rc<dyn IObservable<Option<PopupRoot>>> {
                let context_flyout = as_popup_host_provider(&items[0]);
                let context_menu = items[1].as_ref().and_then(|item| item.cast::<ContextMenu>());
                let attached_flyout = as_popup_host_provider(&items[2]);
                let tool_tip = items[3]
                    .as_ref()
                    .and_then(|item| item.cast::<ferroui_controls::ToolTip>())
                    .map(|tool_tip| tool_tip.to_popup_host_provider());
                let button_flyout = as_popup_host_provider(&items[4]);

                if let Some(context_menu) = context_menu {
                    // Note: ContextMenus are special since all the items are added as visual children.
                    // So we don't need to go via Popup
                    return Observable::return_(Some(PopupRoot::new(context_menu.upcast(), None)));
                }

                if let Some(context_flyout) = context_flyout {
                    return get_popup_host_observable(context_flyout, Some("ContextFlyout"));
                }

                if let Some(attached_flyout) = attached_flyout {
                    return get_popup_host_observable(attached_flyout, Some("AttachedFlyout"));
                }

                if let Some(tool_tip) = tool_tip {
                    return get_popup_host_observable(tool_tip, Some("ToolTip"));
                }

                if let Some(button_flyout) = button_flyout {
                    return get_popup_host_observable(button_flyout, Some("Flyout"));
                }

                Observable::return_(None)
            },
        );

        Some(Observable::switch(popup_roots))
    }
}

impl TreeNodeCollectionImpl for VisualTreeNodeCollection {
    /// The original disposes the subscriptions only, not the nodes.
    fn dispose(&self, _this: &TreeNodeCollection) {
        self.subscriptions.dispose();
    }

    fn initialize(&self, this: &TreeNodeCollection, nodes: &FerroList<Rc<TreeNode>>) {
        self.subscriptions.clear();

        let owner = this.owner();
        let owner = owner.as_ref().map(Rc::downgrade).unwrap_or_default();

        if let Some(popup_root_observable) = Self::get_hosted_popup_root_observable(&self.control) {
            let child_node: RefCell<Option<Rc<TreeNode>>> = RefCell::new(None);
            let nodes = nodes.clone();
            let owner = owner.clone();

            self.subscriptions.add(popup_root_observable.subscribe_fn(move |popup_root: Option<PopupRoot>| {
                if let Some(popup_root) = popup_root {
                    let node = VisualTreeNode::new(
                        popup_root.root.upcast(),
                        owner.upgrade().as_ref(),
                        popup_root.custom_name,
                    );
                    *child_node.borrow_mut() = Some(node.clone());

                    nodes.add(node);
                } else {
                    let child = child_node.borrow().clone();
                    if let Some(child) = child {
                        nodes.remove(&child);
                    }
                }
            }));
        }

        let (added, removed, reset) = (nodes.clone(), nodes.clone(), nodes.clone());
        self.subscriptions.add(FerroListExtensions::for_each_item_indexed(
            self.control.visual_children(),
            move |i, item: &Ref<Visual>| {
                added.insert(i, VisualTreeNode::new(item.clone().upcast(), owner.upgrade().as_ref(), None));
            },
            move |i, _| removed.remove_at(i),
            move || reset.clear(),
            false,
        ));
    }
}

/// `VisualTreeNode.ApplicationHostVisuals`: the nodes of the top levels of
/// an application (its main view, or its windows except the window of the
/// developer tools), following the windows that open and close.
pub struct ApplicationHostVisuals {
    application: Ref<Application>,
    subscriptions: RefCell<Rc<CompositeDisposable>>,
}

impl ApplicationHostVisuals {
    pub fn new(owner: Weak<TreeNode>, host: Ref<Application>) -> Rc<TreeNodeCollection> {
        TreeNodeCollection::new(
            owner,
            Box::new(Self {
                application: host,
                subscriptions: RefCell::new(Rc::new(CompositeDisposable::with_capacity(2))),
            }),
        )
    }
}

impl TreeNodeCollectionImpl for ApplicationHostVisuals {
    fn initialize(&self, this: &TreeNodeCollection, nodes: &FerroList<Rc<TreeNode>>) {
        let owner = this.owner();
        let application_lifetime = self.application.application_lifetime();

        if let Some(single) =
            application_lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            if let Some(main_view) = single.main_view() {
                nodes.add(VisualTreeNode::new(main_view.upcast(), owner.as_ref(), None));
            }
        }

        if let Some(classic) =
            application_lifetime.as_ref().and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            for window in classic.windows() {
                if window.is::<MainWindow>() {
                    continue;
                }
                nodes.add(VisualTreeNode::new(window.upcast(), owner.as_ref(), None));
            }

            let owner = owner.as_ref().map(Rc::downgrade).unwrap_or_default();

            let opened = {
                let nodes = nodes.clone();
                let owner = owner.clone();
                Window::window_opened_event().add_class_handler::<Window>(move |s: &Window, _e: &RoutedEventArgs| {
                    let s = s.to_ref();
                    if s.is::<MainWindow>() {
                        return;
                    }
                    nodes.add(VisualTreeNode::new(s.upcast(), owner.upgrade().as_ref(), None));
                })
            };
            let closed = {
                let nodes = nodes.clone();
                Window::window_closed_event().add_class_handler::<Window>(move |s: &Window, _e: &RoutedEventArgs| {
                    let s = s.to_ref();
                    if s.is::<MainWindow>() {
                        return;
                    }
                    let item = nodes.snapshot().iter().find(|node| node.visual().ptr_eq(&s)).cloned();
                    if let Some(item) = item {
                        nodes.remove(&item);
                    }
                })
            };

            *self.subscriptions.borrow_mut() = Rc::new(CompositeDisposable::from_disposables([opened, closed]));
        }
    }

    fn dispose(&self, this: &TreeNodeCollection) {
        let subscriptions = self.subscriptions.borrow().clone();
        subscriptions.dispose();
        this.base_dispose();
    }
}
