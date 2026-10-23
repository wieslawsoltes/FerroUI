//! ferroui-view
//!
//! A declarative way to write FerroUI user interfaces in Rust, without XAML:
//! elements built with chained setters (`button().content("Add").on_click(..)`)
//! or with the [`rsx!`] macro, state held in signals that the properties of
//! controls follow, and children that follow state.
//!
//! The pieces (`docs/declarative-ui.md` has the design):
//!
//! - [`Element`]: a description of controls, their property values,
//!   handlers and children. It is data, compared by value, so that the
//!   reconciler can compare two trees and patch controls; mounting makes the
//!   controls ([`Element::mount`]).
//! - Builders: a function per control class (`button()`, `stack_panel()`)
//!   and a trait per class with a setter per property and a handler per routed
//!   event, generated from the sources (`scripts/generate_view_builders.py`).
//! - Reactivity: [`Signal`], [`Memo`], [`effect`], owned by an [`Owner`].
//!   A setter given a signal or a closure makes the property follow it.
//! - Children that follow state: [`for_each`] (keyed), [`show`], [`dynamic`].
//! - Components that run again: [`rerender`] and `#[component(rerender)]`,
//!   with their state in [hooks](hooks) ([`use_signal`], [`use_effect`], ...);
//!   the reconciler compares what they return with what they returned before
//!   and patches the controls.
//! - Patterns of larger applications: futures run by a component
//!   ([`use_future`], [`suspense`], [`sleep`]), error boundaries
//!   ([`error_boundary()`]), and navigation between views ([`router()`],
//!   [`route`], [`use_router`]).
//! - With the feature `desktop`: [`launch`] runs an application whose main
//!   window is an element.
//!
//! ```ignore
//! use ferroui_view::prelude::*;
//!
//! fn counter() -> Element {
//!     let count = signal(0);
//!     stack_panel()
//!         .spacing(8.0)
//!         .child(text_block().text(move || format!("Count: {}", count.get())))
//!         .child(button().content("Increment").on_click(move |_| count.update(|c| *c += 1)))
//!         .into()
//! }
//! ```

// The expansions of the macros name the crate; its own tests use them too.
extern crate self as ferroui_view;

mod animation;
pub mod async_data;
pub mod builder;
mod component_host;
mod control_flow;
pub mod diagnostics;
mod conversions;
pub mod element;
pub mod error_boundary;
mod extras;
pub mod hooks;
pub mod hot;
pub mod inspect;
mod mount;
pub mod reactive;
mod register_types;
pub mod router;
mod rust_paths;
mod style;
mod template;
pub mod xaml;

#[cfg(feature = "desktop")]
mod app;

/// The generated builders (see the module documentation).
pub mod generated {
    pub mod builders;
    #[cfg(feature = "color-picker")]
    pub mod color_picker;
    #[cfg(feature = "dialogs")]
    pub mod dialogs;
    #[cfg(feature = "reactiveui")]
    pub mod reactiveui;
    #[cfg(feature = "treedatagrid")]
    pub mod treedatagrid;
}

pub use builder::{Bind, Builder, El, ElementBuilder, FromValue, IntoProp, IntoValue, TypedProperty, ViewClass};
pub use control_flow::{dynamic, for_each, show, show_else, NodeRef};
pub use diagnostics::{clear_diagnostics_handler, set_diagnostics_handler, Diagnostic};
pub use element::{rerender, ComponentElement, Element, IntoChild, IntoChildren, IntoElement, Key};
pub use component_host::{register_component, ComponentHost, View};
pub use hooks::{
    use_data_context, use_data_context_value, use_effect, use_effect_with, use_hook, use_memo, use_parent_control,
    use_parent_property, use_resource, use_resource_as, use_signal, use_theme_variant,
};
pub use animation::{
    animation, ms, transition, transition_eased, transitions, use_animation, AnimationBuilder, KeyFrameBuilder,
    Transitionable,
};
pub use style::{parse_selector, style, style_of, StyleBuilder};
pub use inspect::{inspector, mounted_trees, InstanceKind, InstanceNode, MountedTree, PropertyInfo};
#[cfg(feature = "desktop")]
pub use inspect::{attach_inspector, open_inspector};
pub use mount::{view_stats, ViewStats};
pub use template::template;
pub use register_types::{register_types, ASSEMBLY};
#[cfg(feature = "xaml")]
pub use xaml::{load_xaml, load_xaml_uri, use_xaml};
pub use extras::GridDefinitions;
pub use async_data::{sleep, suspense, use_future, AsyncState};
pub use error_boundary::{error_boundary, ErrorInfo};
pub use router::{route, router, try_use_router, use_router, RouteDef, RouteMatch, Router};
pub use reactive::{
    batch, effect, memo, on_cleanup, provide_context, signal, untrack, use_context, Memo, Owner, Signal, TwoWay,
};

#[cfg(feature = "desktop")]
pub use app::{launch, launch_themed, launch_with, mount_window, open_window};

pub use ferroui_view_macros::{component, rsx};

/// Everything an application written with the builders or the macro uses.
pub mod prelude {
    pub use crate::generated::builders::*;
    #[cfg(feature = "color-picker")]
    pub use crate::generated::color_picker::*;
    #[cfg(feature = "dialogs")]
    pub use crate::generated::dialogs::*;
    #[cfg(feature = "reactiveui")]
    pub use crate::generated::reactiveui::*;
    #[cfg(feature = "treedatagrid")]
    pub use crate::generated::treedatagrid::*;
    pub use crate::{
        animation, batch, component, dynamic, effect, for_each, memo, ms, on_cleanup, provide_context, rerender, rsx, show,
        show_else, style, transition, transitions, use_animation, use_resource, use_resource_as, use_theme_variant,
        signal, template, untrack, use_context, use_data_context, use_effect, use_effect_with, use_hook, use_memo,
        use_signal, Bind, ComponentHost, El, Element, ElementBuilder, GridDefinitions, IntoChild, IntoChildren, IntoElement, Key, Memo, NodeRef, Owner, Signal, TwoWay,
    };
    pub use crate::{
        error_boundary, route, router, sleep, suspense, try_use_router, use_future, use_router, AsyncState, ErrorInfo, RouteDef,
        RouteMatch, Router,
    };
    #[cfg(feature = "desktop")]
    pub use crate::{launch, launch_themed, launch_with, mount_window, open_window};
}

#[cfg(test)]
mod tests;
