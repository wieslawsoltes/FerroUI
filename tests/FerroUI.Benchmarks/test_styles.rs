//! A collection of styles with nested style collections, styles, resources
//! and theme resources, for the benchmarks of resource lookups.

use ferroui_base::controls::ResourceDictionary;
use ferroui_base::styling::{Style, Styles, ThemeVariant};
use ferroui_base::Ref;

pub struct TestStyles;

impl TestStyles {
    /// Styles with `child_styles_count` style collections of
    /// `child_inner_style_count` styles each; every style has
    /// `child_resource_count` resources and, when
    /// `child_theme_resources_count` is positive, a dark and a light theme
    /// dictionary with that many resources each. Every resource is null.
    pub fn new(
        child_styles_count: i32,
        child_inner_style_count: i32,
        child_resource_count: i32,
        child_theme_resources_count: i32,
    ) -> Ref<Styles> {
        let styles = Styles::new();

        for i in 0..child_styles_count {
            let child_styles = Styles::new();

            for j in 0..child_inner_style_count {
                let child_style = Style::new();

                for k in 0..child_resource_count {
                    child_style.resources().add(format!("resource.{i}.{j}.{k}"), None);
                }

                if child_theme_resources_count > 0 {
                    let dark_theme = ResourceDictionary::new();
                    let light_theme = ResourceDictionary::new();
                    child_style.resources().set_theme_dictionary(ThemeVariant::dark(), &dark_theme);
                    child_style.resources().set_theme_dictionary(ThemeVariant::light(), &light_theme);
                    for k in 0..child_theme_resources_count {
                        dark_theme.add(format!("resource.theme.{i}.{j}.{k}"), None);
                        light_theme.add(format!("resource.theme.{i}.{j}.{k}"), None);
                    }
                }

                child_styles.add(&child_style);
            }

            styles.add(&child_styles);
        }

        styles
    }
}
