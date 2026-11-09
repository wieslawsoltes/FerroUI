//! SettingsPanel.Build's theme snapshot and Combo<string> selection, independent of native controls.
use crate::enum_choices::ChoiceIndexError;
use ferroboard_core::{settings::KeyboardSettings, text::Utf16Text};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeChoice {
    pub name: Option<Utf16Text>,
    pub id: Option<Utf16Text>,
}
#[derive(Clone, Debug, Default)]
pub struct ThemeChoices {
    items: Vec<ThemeChoice>,
}
#[derive(Clone, Copy, Debug)]
pub enum ThemeProperty {
    LightTheme,
    DarkTheme,
}
#[derive(Debug)]
pub enum ThemeSelectionError {
    Index(ChoiceIndexError),
    Encoding,
}
impl std::fmt::Display for ThemeSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Index(e) => e.fmt(f),
            Self::Encoding => write!(
                f,
                "theme id contains unpaired UTF-16 surrogates; native settings cannot represent it"
            ),
        }
    }
}
impl std::error::Error for ThemeSelectionError {}
impl ThemeChoices {
    pub fn snapshot<T, E>(
        themes: impl IntoIterator<Item = T>,
        mut name: impl FnMut(&T) -> Option<Utf16Text>,
        mut id: impl FnMut(&T) -> Option<Utf16Text>,
        mut compare: impl FnMut(&Utf16Text, &Utf16Text) -> Result<Ordering, E>,
    ) -> Result<Self, E> {
        // LINQ evaluates name keys once, performs a stable sort, then reads the projected names and ids.
        let mut keys = themes
            .into_iter()
            .map(|t| (name(&t), t))
            .collect::<Vec<_>>();
        let mut failure = None;
        keys.sort_by(|(left, _), (right, _)| {
            if failure.is_some() {
                return Ordering::Equal;
            }
            match (left, right) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Less,
                (Some(_), None) => Ordering::Greater,
                (Some(left), Some(right)) => match compare(left, right) {
                    Ok(order) => order,
                    Err(error) => {
                        failure = Some(error);
                        Ordering::Equal
                    }
                },
            }
        });
        // No partially sorted list, id projection or successful result escapes a comparison error.
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(Self {
            items: keys
                .into_iter()
                .map(|(_, theme)| ThemeChoice {
                    name: name(&theme),
                    id: id(&theme),
                })
                .collect(),
        })
    }
    pub fn items(&self) -> &[ThemeChoice] {
        &self.items
    }
    pub fn selected_index(&self, settings: &KeyboardSettings, property: ThemeProperty) -> i32 {
        let current = Some(Utf16Text::from(property.get(settings).as_str()));
        self.items
            .iter()
            .position(|item| item.id == current)
            .map_or(-1, |i| i as i32)
    }
    pub fn select(
        &self,
        settings: &KeyboardSettings,
        property: ThemeProperty,
        index: i32,
        refreshing: bool,
    ) -> Result<(), ThemeSelectionError> {
        if refreshing || index < 0 {
            return Ok(());
        }
        let item = self
            .items
            .get(index as usize)
            .ok_or(ThemeSelectionError::Index(ChoiceIndexError {
                index,
                count: self.items.len(),
            }))?;
        let id = item
            .id
            .as_ref()
            .map(Utf16Text::to_utf8)
            .transpose()
            .map_err(|_| ThemeSelectionError::Encoding)?;
        property.set(settings, id);
        Ok(())
    }
}
impl ThemeProperty {
    pub fn get(self, settings: &KeyboardSettings) -> String {
        match self {
            Self::LightTheme => settings.light_theme(),
            Self::DarkTheme => settings.dark_theme(),
        }
    }
    pub fn set(self, settings: &KeyboardSettings, value: Option<String>) {
        match self {
            Self::LightTheme => settings.set_light_theme(value),
            Self::DarkTheme => settings.set_dark_theme(value),
        }
    }
}
