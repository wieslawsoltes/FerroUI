//! Native control recycling and recycling data templates for FerroDock.

mod control_recycling;
mod detach;
mod recycling_data_template;
mod register_types;
mod value_bridge;
mod value_key;
pub use control_recycling::ControlRecycling;
pub use ferrodock_controls_recycling_model::{IControlRecycling, IControlRecyclingIdProvider};
pub use recycling_data_template::ControlRecyclingDataTemplate;
pub use register_types::{register_types, ASSEMBLY};
pub use value_bridge::{from_recycling_value, register_value_type, to_recycling_value};

#[cfg(test)]
mod tests;

#[doc(hidden)]
pub use ferrodock_controls_recycling_model_ferro::{
    IControlRecyclingIdProviderMetadata, IControlRecyclingMetadata,
};
