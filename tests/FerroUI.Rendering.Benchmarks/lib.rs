//! Backend adapters only. Workload code uses the same public contracts for every adapter.
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use std::rc::Rc;

pub struct Backend {
    pub name: String,
    pub renderer: Rc<dyn IPlatformRenderInterface>,
    pub fonts: Rc<dyn IFontManagerImpl>,
    pub shaper: Rc<dyn ITextShaperImpl>,
    pub text_backend: &'static str,
}
impl Backend {
    pub fn new(name: &str) -> Result<Self, String> {
        match name {
            #[cfg(feature = "skia")]
            "skia" => Ok(Self {
                name: name.into(),
                renderer: Rc::new(ferroui_skia::PlatformRenderInterface::new(None, None)),
                fonts: Rc::new(ferroui_skia::FontManagerImpl::new()),
                shaper: Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()),
                text_backend: "HarfBuzz",
            }),
            #[cfg(all(feature = "quartz", target_os = "macos"))]
            "quartz" => Ok(Self {
                name: name.into(),
                renderer: Rc::new(ferroui_quartz::QuartzRenderInterface),
                fonts: Rc::new(ferroui_quartz::QuartzFontManager::default()),
                shaper: Rc::new(ferroui_quartz::QuartzTextShaper),
                text_backend: "CoreText",
            }),
            #[cfg(all(
                feature = "quartz",
                target_os = "macos",
                any(feature = "skia", feature = "vello")
            ))]
            "quartz-harfbuzz" => {
                let mut backend = Self::new("quartz")?;
                backend.name = name.into();
                backend.shaper = Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new());
                backend.text_backend = "HarfBuzz";
                Ok(backend)
            }
            #[cfg(feature = "vello")]
            "vello-cpu" | "vello-hybrid" | "vello-gpu" => {
                use ferroui_vello::{VelloOptions, VelloRenderingMode};
                let mode = match name {
                    "vello-cpu" => VelloRenderingMode::Cpu,
                    "vello-hybrid" => VelloRenderingMode::Hybrid,
                    _ => VelloRenderingMode::Gpu,
                };
                Ok(Self {
                    name: name.into(),
                    renderer: Rc::new(ferroui_vello::PlatformRenderInterface::new(
                        VelloOptions::with_rendering_mode(mode),
                    )),
                    fonts: Rc::new(ferroui_vello::FontManagerImpl::new()),
                    shaper: Rc::new(ferroui_harfbuzz::HarfBuzzTextShaper::new()),
                    text_backend: "HarfBuzz",
                })
            }
            _ => Err(format!(
                "Backend {name:?} is unknown, disabled or unsupported on this platform"
            )),
        }
    }
    pub fn available() -> Vec<&'static str> {
        vec![
            #[cfg(feature = "skia")]
            "skia",
            #[cfg(all(feature = "quartz", target_os = "macos"))]
            "quartz",
            #[cfg(all(
                feature = "quartz",
                target_os = "macos",
                any(feature = "skia", feature = "vello")
            ))]
            "quartz-harfbuzz",
            #[cfg(feature = "vello")]
            "vello-cpu",
            #[cfg(feature = "vello")]
            "vello-hybrid",
            #[cfg(feature = "vello")]
            "vello-gpu",
        ]
    }
}

/// Nearest-rank percentiles over raw samples, retained in reports for independent analysis.
pub fn distribution(samples: &[f64]) -> serde_json::Value {
    assert!(!samples.is_empty());
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let at = |p: f64| sorted[((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1)];
    serde_json::json!({"median_ms": at(0.5), "p95_ms":at(0.95), "p99_ms":at(0.99), "mean_ms":samples.iter().sum::<f64>()/samples.len() as f64, "min_ms":sorted[0], "max_ms":sorted[sorted.len()-1]})
}
#[cfg(test)]
mod tests {
    #[test]
    fn rejects_unknown_backend_instead_of_silently_measuring_skia() {
        assert!(super::Backend::new("typo").is_err());
    }
    #[test]
    fn nearest_rank_percentiles_keep_the_tail() {
        let samples: Vec<_> = (1..=100).map(f64::from).collect();
        let result = super::distribution(&samples);
        assert_eq!(result["median_ms"], 50.);
        assert_eq!(result["p95_ms"], 95.);
        assert_eq!(result["p99_ms"], 99.);
    }
}
