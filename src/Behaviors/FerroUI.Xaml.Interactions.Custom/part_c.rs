//! Part C of the crate: the folders of the reference listed below, one module per folder.
//!
//! - `Animations/` ([`animations`](self::animations)): key-frame animations;
//! - `SplitView/` ([`split_view`](self::split_view)): the split view;
//! - `Notifications/` ([`notifications`](self::notifications)): the notifications of a top level;
//! - `RenderTarget/` ([`render_target`](self::render_target)): drawing into a render target bitmap, with its contracts;
//! - `WriteableBitmap/` ([`writeable_bitmap`](self::writeable_bitmap)): rendering into a writeable bitmap, with its contract;
//! - `Window/` ([`window`](self::window)): the window of the associated control;
//! - `Button/` ([`button`](self::button)): buttons;
//! - `Cursor/` ([`cursor`](self::cursor)): the cursor of an input element, with its contract;
//! - `ScrollViewer/` ([`scroll_viewer`](self::scroll_viewer)): the scroll viewer;
//! - `ViewModel/` ([`view_model`](self::view_model)): the properties of the view model (the data context) of a control;
//! - `ContextDialogs/` ([`context_dialogs`](self::context_dialogs)): a context dialog shown in a popup;
//! - `Transitions/` ([`transitions`](self::transitions)): the transitions of an element;
//! - `ToolTips/` ([`tool_tips`](self::tool_tips)): the tool tip of a control.
//!
//! The part owns its classes ([`TYPES`]), their public Rust paths and its markup types
//! ([`register`]); `lib.rs` re-exports everything public of it.

use ferroui_base::animation::Animatable;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, ObjectType, Ref, StaticType, TypeInfo, Visual};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use std::rc::Rc;

#[path = "animations/mod.rs"]
mod animations;
#[path = "button/mod.rs"]
mod button;
#[path = "context_dialogs/mod.rs"]
mod context_dialogs;
#[path = "cursor/mod.rs"]
mod cursor;
#[path = "notifications/mod.rs"]
mod notifications;
#[path = "render_target/mod.rs"]
mod render_target;
#[path = "scroll_viewer/mod.rs"]
mod scroll_viewer;
#[path = "split_view/mod.rs"]
mod split_view;
#[path = "tool_tips/mod.rs"]
mod tool_tips;
#[path = "transitions/mod.rs"]
mod transitions;
#[path = "view_model/mod.rs"]
mod view_model;
#[path = "window/mod.rs"]
mod window;
#[path = "writeable_bitmap/mod.rs"]
mod writeable_bitmap;

pub use self::animations::*;
pub use self::button::*;
pub use self::context_dialogs::*;
pub use self::cursor::*;
pub use self::notifications::*;
pub use self::render_target::*;
pub use self::scroll_viewer::*;
pub use self::split_view::*;
pub use self::tool_tips::*;
pub use self::transitions::*;
pub use self::view_model::*;
pub use self::window::*;
pub use self::writeable_bitmap::*;

macro_rules! types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as StaticType>::TYPE),*]
    };
}

/// The classes of this part, registered by `register_types`.
pub(crate) const TYPES: &[&TypeInfo] = types![
    // Animations/
    crate::AnimateOnAttachedBehavior,
    crate::AnimationCompletedTrigger,
    crate::BeginAnimationAction,
    crate::FadeInBehavior,
    crate::PlayAnimationBehavior,
    crate::RunAnimationTrigger,
    crate::StartAnimationAction,
    crate::StartBuiltAnimationAction,
    // SplitView/
    crate::SplitViewPaneClosedTrigger,
    crate::SplitViewPaneClosingTrigger,
    crate::SplitViewPaneOpenedTrigger,
    crate::SplitViewPaneOpeningTrigger,
    crate::SplitViewStateBehavior,
    crate::SplitViewStateSetter,
    crate::SplitViewTogglePaneAction,
    // Notifications/
    crate::CloseNotificationAction,
    crate::NotificationManagerBehavior,
    crate::ShowErrorNotificationAction,
    crate::ShowInformationNotificationAction,
    crate::ShowNotificationAction,
    crate::ShowSuccessNotificationAction,
    crate::ShowWarningNotificationAction,
    // RenderTarget/
    crate::RenderRenderTargetBitmapAction,
    crate::RenderTargetBitmapBehavior,
    crate::RenderTargetBitmapTrigger,
    crate::StaticRenderTargetBitmapBehavior,
    // WriteableBitmap/
    crate::WriteableBitmapBehavior,
    crate::WriteableBitmapRenderAction,
    crate::WriteableBitmapRenderBehavior,
    crate::WriteableBitmapTimerTrigger,
    crate::WriteableBitmapTrigger,
    // Window/
    crate::CenterWindowBehavior,
    crate::CloseWindowAction,
    crate::ShowWindowAction,
    crate::WindowAction,
    crate::WindowDragMoveBehavior,
    crate::WindowStateTrigger,
    // Button/
    crate::ButtonClickEventTriggerBehavior,
    crate::ButtonExecuteCommandOnKeyDownBehavior,
    crate::ButtonHideFlyoutBehavior,
    crate::ButtonHideFlyoutOnClickBehavior,
    crate::ButtonHidePopupOnClickBehavior,
    crate::InvokeButtonClickAction,
    // Cursor/
    crate::PointerOverCursorBehavior,
    crate::SetCursorAction,
    crate::SetCursorBehavior,
    crate::SetCursorFromProviderAction,
    crate::SetCursorFromProviderBehavior,
    // ScrollViewer/
    crate::HorizontalScrollViewerBehavior,
    crate::ScrollChangedTrigger,
    crate::ScrollToOffsetAction,
    crate::ScrollViewerOffsetBehavior,
    crate::ViewportBehavior,
    // ViewModel/
    crate::IncrementViewModelPropertyAction,
    crate::SetViewModelPropertyAction,
    crate::SetViewModelPropertyOnLoadBehavior,
    crate::ToggleViewModelBooleanAction,
    crate::ViewModelPropertyChangedTrigger,
    // ContextDialogs/
    crate::ContextDialogBehavior,
    crate::ContextDialogClosedTrigger,
    crate::ContextDialogOpenedTrigger,
    crate::HideContextDialogAction,
    crate::ShowContextDialogAction,
    // Transitions/
    crate::AddTransitionAction,
    crate::ClearTransitionsAction,
    crate::RemoveTransitionAction,
    crate::TransitionsBehavior,
    crate::TransitionsChangedTrigger,
    // ToolTips/
    crate::HideToolTipAction,
    crate::SetToolTipTipAction,
    crate::ShowToolTipAction,
    crate::ToolTipClosingTrigger,
    crate::ToolTipOpeningTrigger,
];

/// The types of this part that are not classes (lists, enumerations and contracts), declared for
/// markup.
const MARKUP_TYPES: &[&MarkupType] = &[
    // SplitView/
    <crate::SplitViewStateSetters as MarkupTyped>::MARKUP,
    // RenderTarget/
    <dyn crate::IRenderTargetBitmapRenderHost as MarkupTyped>::MARKUP,
    <dyn crate::IRenderTargetBitmapRenderer as MarkupTyped>::MARKUP,
    <dyn crate::IRenderTargetBitmapSimpleRenderer as MarkupTyped>::MARKUP,
    // WriteableBitmap/
    <dyn crate::IWriteableBitmapRenderer as MarkupTyped>::MARKUP,
    // Window/
    <crate::WindowActionType as MarkupTyped>::MARKUP,
    // Cursor/
    <dyn crate::ICursorProvider as MarkupTyped>::MARKUP,
    // ScrollViewer/
    <crate::ChangeSize as MarkupTyped>::MARKUP,
];

/// The public Rust paths of the classes of this part, by the shortest path another crate names
/// them by; the emitter of Rust source of the markup compiler reads them.
mod rust_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
            // Animations/
            crate::AnimateOnAttachedBehavior,
            crate::AnimationCompletedTrigger,
            crate::BeginAnimationAction,
            crate::FadeInBehavior,
            crate::PlayAnimationBehavior,
            crate::RunAnimationTrigger,
            crate::StartAnimationAction,
            crate::StartBuiltAnimationAction,
            // SplitView/
            crate::SplitViewPaneClosedTrigger,
            crate::SplitViewPaneClosingTrigger,
            crate::SplitViewPaneOpenedTrigger,
            crate::SplitViewPaneOpeningTrigger,
            crate::SplitViewStateBehavior,
            crate::SplitViewStateSetter,
            crate::SplitViewTogglePaneAction,
            // Notifications/
            crate::CloseNotificationAction,
            crate::NotificationManagerBehavior,
            crate::ShowErrorNotificationAction,
            crate::ShowInformationNotificationAction,
            crate::ShowNotificationAction,
            crate::ShowSuccessNotificationAction,
            crate::ShowWarningNotificationAction,
            // RenderTarget/
            crate::RenderRenderTargetBitmapAction,
            crate::RenderTargetBitmapBehavior,
            crate::RenderTargetBitmapTrigger,
            crate::StaticRenderTargetBitmapBehavior,
            // WriteableBitmap/
            crate::WriteableBitmapBehavior,
            crate::WriteableBitmapRenderAction,
            crate::WriteableBitmapRenderBehavior,
            crate::WriteableBitmapTimerTrigger,
            crate::WriteableBitmapTrigger,
            // Window/
            crate::CenterWindowBehavior,
            crate::CloseWindowAction,
            crate::ShowWindowAction,
            crate::WindowAction,
            crate::WindowDragMoveBehavior,
            crate::WindowStateTrigger,
            // Button/
            crate::ButtonClickEventTriggerBehavior,
            crate::ButtonExecuteCommandOnKeyDownBehavior,
            crate::ButtonHideFlyoutBehavior,
            crate::ButtonHideFlyoutOnClickBehavior,
            crate::ButtonHidePopupOnClickBehavior,
            crate::InvokeButtonClickAction,
            // Cursor/
            crate::PointerOverCursorBehavior,
            crate::SetCursorAction,
            crate::SetCursorBehavior,
            crate::SetCursorFromProviderAction,
            crate::SetCursorFromProviderBehavior,
            // ScrollViewer/
            crate::HorizontalScrollViewerBehavior,
            crate::ScrollChangedTrigger,
            crate::ScrollToOffsetAction,
            crate::ScrollViewerOffsetBehavior,
            crate::ViewportBehavior,
            // ViewModel/
            crate::IncrementViewModelPropertyAction,
            crate::SetViewModelPropertyAction,
            crate::SetViewModelPropertyOnLoadBehavior,
            crate::ToggleViewModelBooleanAction,
            crate::ViewModelPropertyChangedTrigger,
            // ContextDialogs/
            crate::ContextDialogBehavior,
            crate::ContextDialogClosedTrigger,
            crate::ContextDialogOpenedTrigger,
            crate::HideContextDialogAction,
            crate::ShowContextDialogAction,
            // Transitions/
            crate::AddTransitionAction,
            crate::ClearTransitionsAction,
            crate::RemoveTransitionAction,
            crate::TransitionsBehavior,
            crate::TransitionsChangedTrigger,
            // ToolTips/
            crate::HideToolTipAction,
            crate::SetToolTipTipAction,
            crate::ShowToolTipAction,
            crate::ToolTipClosingTrigger,
            crate::ToolTipOpeningTrigger,
        ],
        types: [
            // SplitView/
            crate::SplitViewStateSetters,
            // Window/
            crate::WindowActionType,
            // ScrollViewer/
            crate::ChangeSize,
        ],
        contracts: [
            // RenderTarget/
            crate::IRenderTargetBitmapRenderHost,
            crate::IRenderTargetBitmapRenderer,
            crate::IRenderTargetBitmapSimpleRenderer,
            // WriteableBitmap/
            crate::IWriteableBitmapRenderer,
            // Cursor/
            crate::ICursorProvider,
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

/// Registers the Rust paths (`ferroui_base::ferro_rust_paths!`) and the markup types
/// (`MarkupType::register_all`) of this part. Called once by `register_types`.
pub(crate) fn register() {
    rust_paths::register_rust_paths();
    MarkupType::register_all(MARKUP_TYPES);
    ValueTypes::register_global(register_value_types);
}

/// What the untyped value conversions of markup and bindings must know about the types of this
/// part: the nullable forms of the handles, contracts, lists and enumerations its properties hold,
/// and the element references of the properties that name an element.
fn register_value_types() {
    // Animations/
    ValueTypes::register_nullable::<crate::AnimationBuilderRef>();
    ValueTypes::register_element_ref::<Control>();
    // SplitView/
    ValueTypes::register_nullable::<crate::SplitViewStateSetters>();
    ValueTypes::register_element_ref::<ferroui_controls::SplitView>();
    // Notifications/
    ValueTypes::register_nullable::<Rc<dyn ferroui_controls::INotificationManager>>();
    ValueTypes::register_nullable::<Rc<dyn ferroui_controls::INotification>>();
    ValueTypes::register_element_ref::<ferroui_controls::NotificationCard>();
    // RenderTarget/
    ValueTypes::register_nullable::<Rc<dyn crate::IRenderTargetBitmapRenderHost>>();
    ValueTypes::register_nullable::<Rc<dyn crate::IRenderTargetBitmapRenderer>>();
    ValueTypes::register_nullable::<Rc<dyn crate::IRenderTargetBitmapSimpleRenderer>>();
    // WriteableBitmap/
    ValueTypes::register_nullable::<crate::WriteableBitmapRef>();
    ValueTypes::register_nullable::<Rc<dyn crate::IWriteableBitmapRenderer>>();
    // Window/
    ValueTypes::register_nullable::<crate::WindowActionType>();
    ValueTypes::register_element_ref::<ferroui_controls::Window>();
    // Button/
    ValueTypes::register_element_ref::<ferroui_controls::Button>();
    // Cursor/
    ValueTypes::register_nullable::<Rc<dyn crate::ICursorProvider>>();
    ValueTypes::register_element_ref::<ferroui_base::input::InputElement>();
    // ScrollViewer/
    ValueTypes::register_nullable::<crate::ChangeSize>();
    ValueTypes::register_element_ref::<ferroui_controls::ScrollViewer>();
    // ContextDialogs/
    ValueTypes::register_element_ref::<crate::ContextDialogBehavior>();
    // Transitions/
    ValueTypes::register_element_ref::<ferroui_base::StyledElement>();
}

/// The sender of an action as an object of class `T` (`sender as T` of the reference).
pub(crate) fn sender_as<T: ObjectType>(sender: Option<&BoxedValue>) -> Option<Ref<T>> {
    sender.and_then(object_of).and_then(|object| object.cast::<T>())
}

/// A control as the animatable target of an animation.
pub(crate) fn as_animatable(control: &Control) -> &Animatable {
    control
}

/// A control as a visual.
pub(crate) fn control_as_visual(control: &Control) -> &Visual {
    control
}

/// A visual as the animatable target of an animation.
pub(crate) fn visual_as_animatable(visual: &Visual) -> &Animatable {
    visual
}
