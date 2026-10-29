//! The templates that let styles set behaviors, actions and other objects (`Templates/`): each
//! build creates new objects from deferred content.

mod action_collection_template;
mod behavior_collection_template;
mod notification_template;
mod object_template;

pub use action_collection_template::ActionCollectionTemplate;
pub use behavior_collection_template::BehaviorCollectionTemplate;
pub use notification_template::NotificationTemplate;
pub use object_template::ObjectTemplate;
