//! The provider contracts automation peers implement to expose the
//! behaviour of their elements (invoking, toggling, values, selection,
//! scrolling, ...) to UI automation clients.
//!
//! A peer class implements a contract on [`ProviderAdapter`] of itself and
//! lists the handle (`Rc<dyn IInvokeProvider>`) in the `interfaces` of its
//! class information; `AutomationPeer::get_provider` then finds it. (A peer
//! class of another crate uses an adapter type of its own.)

mod i_embedded_root_provider;
mod i_expand_collapse_provider;
mod i_invoke_provider;
mod i_range_value_provider;
mod i_root_provider;
mod i_scroll_provider;
mod i_selection_item_provider;
mod i_selection_provider;
mod i_toggle_provider;
mod i_value_provider;

pub use i_embedded_root_provider::IEmbeddedRootProvider;
pub use i_expand_collapse_provider::IExpandCollapseProvider;
pub use i_invoke_provider::IInvokeProvider;
pub use i_range_value_provider::IRangeValueProvider;
pub use i_root_provider::IRootProvider;
pub use i_scroll_provider::{IScrollProvider, ScrollAmount};
pub use i_selection_item_provider::ISelectionItemProvider;
pub use i_selection_provider::ISelectionProvider;
pub use i_toggle_provider::{IToggleProvider, ToggleState};
pub use i_value_provider::{IValueProvider, ProviderError};

use ferroui_base::{ObjectType, Ref};
use std::rc::Rc;

/// The handle through which an automation peer of class `T` implements a
/// provider contract: `impl IInvokeProvider for ProviderAdapter<MyPeer>`.
///
/// The peer class then lists the handle in the `interfaces` of its class
/// information, with the conversion of this type:
/// `interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider]`.
pub struct ProviderAdapter<T: ObjectType>(pub Ref<T>);

macro_rules! conversions {
    ($($name:ident: $provider:ident),* $(,)?) => {
        impl<T: ObjectType> ProviderAdapter<T> {
            $(
                /// Converts the handle of a peer to the handle of the provider
                /// contract it implements on this type.
                pub fn $name(peer: Ref<T>) -> Rc<dyn $provider>
                where
                    Self: $provider,
                {
                    Rc::new(Self(peer))
                }
            )*
        }
    };
}

conversions! {
    as_embedded_root_provider: IEmbeddedRootProvider,
    as_expand_collapse_provider: IExpandCollapseProvider,
    as_invoke_provider: IInvokeProvider,
    as_range_value_provider: IRangeValueProvider,
    as_root_provider: IRootProvider,
    as_scroll_provider: IScrollProvider,
    as_selection_item_provider: ISelectionItemProvider,
    as_selection_provider: ISelectionProvider,
    as_toggle_provider: IToggleProvider,
    as_value_provider: IValueProvider,
}
