use super::{NativeSourceIndex, target};
use crate::AnimationProgram;
use ferroui_base::animation::{Animatable, Animation, IAnimation};
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::styling::{IStyle, Style, StyleBase, Styles};
use ferroui_base::{FerroObject, FerroProperty, Ref, StyledElement, TypeInfo};
use ferroui_designer_core::{Document, NodeId};
use ferroui_markup_xaml::{diagnostics::XamlSourceInfo, FromXamlObject};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// The framework objects loaded from the current document's animation declarations.
#[derive(Default)]
pub struct NativeProgramIndex {
    programs: BTreeMap<NodeId, LoadedProgram>,
}

struct LoadedProgram {
    animation: Ref<Animation>,
    style: Option<Ref<Style>>,
    setter_target: Option<&'static TypeInfo>,
    custom_animators: bool,
    transform_components: Vec<&'static FerroProperty>,
}

impl NativeProgramIndex {
    pub fn discover(
        document: &Document,
        source: &NativeSourceIndex,
        programs: &[AnimationProgram],
        controls: impl IntoIterator<Item = Ref<StyledElement>>,
    ) -> Self {
        let ids = programs
            .iter()
            .map(|program| program.id)
            .collect::<BTreeSet<_>>();
        if ids.is_empty() {
            return Self::default();
        }
        let resource_names = programs
            .iter()
            .filter_map(|program| {
                document
                    .ast()
                    .node(program.id)?
                    .attributes
                    .iter()
                    .find(|attribute| {
                        attribute.namespace_uri.as_deref()
                            == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                            && attribute.name.rsplit(':').next() == Some("Key")
                    })
                    .map(|attribute| attribute.value.as_str())
            })
            .collect::<BTreeSet<_>>();
        let mut index = Self::default();
        let mut styles = Vec::<Rc<dyn IStyle>>::new();
        let mut dictionaries = Vec::<Ref<ResourceDictionary>>::new();
        let mut seen = BTreeSet::new();
        for control in controls {
            if seen.insert(&*control as *const StyledElement as usize) {
                styles.extend(control.styles().snapshot().iter().cloned());
                dictionaries.extend([control.resources(), control.styles().resources()]);
                if let Some(theme) = control.theme() {
                    styles.push(theme.into());
                }
            }
        }
        seen.clear();
        while let Some(style) = styles.pop() {
            if let Some(object) = style.as_object() {
                if !seen.insert(object as *const FerroObject as usize) {
                    continue;
                }
                if let Some(base) = object.downcast_ref::<StyleBase>() {
                    for animation in base.animations().snapshot().iter() {
                        if let Some(animation) = native_object(animation) {
                            let info = XamlSourceInfo::for_object(&animation);
                            index.insert(
                                source,
                                &ids,
                                animation,
                                info,
                                object.to_ref().cast::<Style>(),
                            );
                        }
                    }
                    dictionaries.push(base.resources());
                } else if let Some(collection) = object.downcast_ref::<Styles>() {
                    dictionaries.push(collection.resources());
                }
            }
            styles.extend(style.children().iter().cloned());
        }
        seen.clear();
        while let Some(dictionary) = dictionaries.pop() {
            if !seen.insert(&*dictionary as *const ResourceDictionary as usize) {
                continue;
            }
            for key in dictionary.keys().into_iter().filter(|key| {
                key.as_str()
                    .is_some_and(|name| resource_names.contains(name))
            }) {
                let keyed = XamlSourceInfo::get_xaml_source_info_for_key(
                    &dictionary.clone().into(),
                    key.clone(),
                );
                // Only authored animation keys are materialized. A failing application
                // resource is left out of the catalog. A deferred resource can acquire
                // its own provenance on construction, so validate the object first.
                let value = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    ferroui_markup_xaml::xaml_error_scope::with_xaml_errors(|| {
                        Ok(dictionary.get(&key))
                    })
                    .ok()
                    .flatten()
                }))
                .ok()
                .flatten();
                let Some(value) = value else {
                    continue;
                };
                let animation = Ref::<Animation>::from_xaml_object(&value).or_else(|| {
                    value
                        .downcast_ref::<Rc<dyn IAnimation>>()
                        .and_then(native_object)
                });
                if let Some(animation) = animation {
                    let info = XamlSourceInfo::for_object(&animation).or(keyed);
                    index.insert(source, &ids, animation, info, None);
                }
            }
            dictionaries.extend(
                dictionary
                    .merged_dictionaries()
                    .snapshot()
                    .iter()
                    .filter_map(|provider| {
                        provider.as_object()?.to_ref().cast::<ResourceDictionary>()
                    }),
            );
            dictionaries.extend(dictionary.theme_dictionaries_snapshot().iter().filter_map(
                |(_, provider)| provider.as_object()?.to_ref().cast::<ResourceDictionary>(),
            ));
        }
        for (&id, program) in &mut index.programs {
            program.setter_target = target::declared_type(document, id);
        }
        index
    }

    pub fn animation(&self, id: NodeId) -> Option<&Ref<Animation>> {
        self.programs.get(&id).map(|program| &program.animation)
    }

    pub fn uses_custom_animators(&self, id: NodeId) -> bool {
        self.programs
            .get(&id)
            .is_some_and(|program| program.custom_animators)
    }

    pub fn uses_transform_components(&self, id: NodeId) -> bool {
        self.programs
            .get(&id)
            .is_some_and(|program| !program.transform_components.is_empty())
    }

    pub fn track_property(&self, id: NodeId, name: &str) -> Option<&'static FerroProperty> {
        let (property, _) = super::curve::track(self.animation(id)?, name).ok()?;
        Some(property)
    }

    pub fn transform_component(&self, id: NodeId, name: &str) -> Option<&'static FerroProperty> {
        let mut matches = self.programs.get(&id)?.transform_components
            .iter().copied().filter(|property| super::curve::matches_property(property, name));
        let property = matches.next()?;
        matches.next().is_none().then_some(property)
    }

    pub fn style_matches(&self, id: NodeId, element: &StyledElement) -> bool {
        let Some(style) = self
            .programs
            .get(&id)
            .and_then(|program| program.style.as_ref())
        else {
            return false;
        };
        style.selector().is_some_and(|selector| {
            selector
                .match_(element, style.parent().as_deref(), false)
                .is_match()
        })
    }

    pub fn validate_target(&self, id: NodeId, target: &Animatable, role: &str) -> Result<(), String> {
        let program = self.programs.get(&id).ok_or("The native animation retired")?;
        let compatible = target::validate_animation_target(&program.animation, target, role);
        if compatible.is_ok() {
            return compatible;
        }
        let in_scope = program.setter_target.map_or_else(
            || target.to_ref().cast::<StyledElement>().is_some_and(|control| self.style_matches(id, &control)),
            |type_| type_.is_assignable_from(target.get_type()),
        );
        if in_scope {
            target::validate(&program.animation, target, role, true)
        } else {
            compatible
        }
    }

    fn insert(
        &mut self,
        source: &NativeSourceIndex,
        ids: &BTreeSet<NodeId>,
        animation: Ref<Animation>,
        info: Option<XamlSourceInfo>,
        style: Option<Ref<Style>>,
    ) {
        if let Some(id) = info
            .as_ref()
            .and_then(|info| source.resolve(info, |id| ids.contains(&id)))
        {
            self.programs.entry(id).or_insert_with(|| LoadedProgram {
                custom_animators: animation.uses_custom_animators(),
                transform_components: animation.transform_components(),
                animation,
                style,
                setter_target: None,
            });
        }
    }
}

fn native_object(animation: &Rc<dyn IAnimation>) -> Option<Ref<Animation>> {
    animation.as_object()?.cast::<Animation>()
}
