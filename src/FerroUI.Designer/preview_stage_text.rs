//! Text settings persisted with an independent native preview stage.
use ferroui_base::{diagnostics::DesignTextEnvironment, utilities::Locale};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PreviewStageText {
    /// BCP-47 locale, or the application's normal language when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Font size in percent of the authored size.
    pub scale_percent: u16,
}

impl Default for PreviewStageText {
    fn default() -> Self {
        Self {
            locale: None,
            scale_percent: 100,
        }
    }
}

impl PreviewStageText {
    pub(crate) fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        self.environment().map(|_| ())
    }

    /// The native settings for the accepted stage's subtree.
    pub fn environment(&self) -> Result<Option<DesignTextEnvironment>, String> {
        if !(50..=300).contains(&self.scale_percent) {
            return Err("Text size must be between 50% and 300%".into());
        }
        let locale = self
            .locale
            .as_deref()
            .map(|tag| {
                if tag.is_empty() || tag.len() > 128 {
                    return Err("Enter a locale such as fr-CA or ja-JP".to_owned());
                }
                Locale::try_from_str(tag)
                    .map_err(|_| "Enter a locale such as fr-CA or ja-JP".to_owned())
            })
            .transpose()?;
        Ok(DesignTextEnvironment::new(
            locale,
            f64::from(self.scale_percent) / 100.0,
        ))
    }
}
