//! Styling: styles, selectors, setters, control themes, theme variants and
//! container queries.

pub(crate) mod activators;

mod and_query;
mod child_selector;
mod container;
mod container_query;
mod container_sizing;
mod control_theme;
mod descendent_selector;
mod direct_property_setter_binding_instance;
mod direct_property_setter_instance;
mod i_global_styles;
mod i_setter_instance;
mod i_setter_value;
mod i_style;
mod i_style_host;
mod i_style_instance;
mod i_template;
mod i_theme_variant_host;
mod i_theme_variant_root;
mod nesting_selector;
mod not_selector;
mod nth_child_selector;
mod nth_last_child_selector;
mod or_query;
mod or_selector;
mod property_equals_selector;
mod property_setter_template_instance;
mod screen_queries;
mod selector;
mod selector_match;
mod selectors;
mod setter;
mod setter_base;
mod style;
mod style_base;
mod style_children;
mod style_instance;
mod style_queries;
mod style_query;
mod style_query_comparison_operator;
mod styles;
mod template_selector;
mod theme_variant;
mod type_name_and_class_selector;
mod value_style_query;
mod visual_query_provider;

pub(crate) use and_query::AndQuery;
pub(crate) use child_selector::ChildSelector;
pub use container::Container;
pub use container_query::ContainerQuery;
pub use container_sizing::ContainerSizing;
pub use control_theme::ControlTheme;
pub(crate) use descendent_selector::DescendantSelector;
pub(crate) use direct_property_setter_binding_instance::DirectPropertySetterBindingInstance;
pub(crate) use direct_property_setter_instance::DirectPropertySetterInstance;
pub use i_global_styles::{GlobalStylesHandler, IGlobalStyles};
pub use i_setter_instance::ISetterInstance;
pub use i_setter_value::ISetterValue;
pub(crate) use i_style::empty_styles;
pub use i_style::{style_ptr_eq, IStyle};
pub use i_style_host::{IStyleHost, StyleHostRef};
pub use i_style_instance::IStyleInstance;
pub use i_template::ITemplate;
pub use i_theme_variant_host::IThemeVariantHost;
pub use i_theme_variant_root::IThemeVariantRoot;
pub(crate) use nesting_selector::NestingSelector;
pub(crate) use not_selector::NotSelector;
pub(crate) use nth_child_selector::NthChildSelector;
pub(crate) use nth_last_child_selector::NthLastChildSelector;
pub(crate) use or_query::OrQuery;
pub(crate) use or_selector::OrSelector;
pub(crate) use property_equals_selector::PropertyEqualsSelector;
pub(crate) use property_setter_template_instance::PropertySetterTemplateInstance;
pub(crate) use screen_queries::{HeightQuery, WidthQuery};
pub use selector::Selector;
pub(crate) use selector::SelectorNode;
pub use selector_match::{SelectorMatch, SelectorMatchResult};
pub use selectors::Selectors;
pub use setter::{Setter, SetterValue};
pub use setter_base::{SetterBase, SetterInstance};
pub(crate) use setter_base::SetterInstanceKind;
pub use style::Style;
pub use style_base::{StyleBase, StyleBaseImpl, StyleBaseImplExt, StyleBaseVTable};
pub use style_children::StyleChildren;
pub use style_instance::{DuplicateSetterError, StyleInstance};
pub use style_queries::StyleQueries;
pub use style_query::StyleQuery;
pub(crate) use style_query::StyleQueryNode;
pub use style_query_comparison_operator::StyleQueryComparisonOperator;
pub use styles::{styles_as_style, Styles};
pub(crate) use template_selector::TemplateSelector;
pub use theme_variant::{ThemeVariant, ThemeVariantParseError};
pub(crate) use type_name_and_class_selector::TypeNameAndClassSelector;
pub(crate) use value_style_query::ValueStyleQuery;
pub(crate) use visual_query_provider::VisualQueryProvider;

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

#[cfg(test)]
mod selector_tests;

#[cfg(test)]
mod style_tests;

#[cfg(test)]
mod container_tests;
