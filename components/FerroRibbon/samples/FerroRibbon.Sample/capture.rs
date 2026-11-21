// Copyright (c) 2026 Wiesław Šoltés. MIT. Original Capture.cs orchestration and compositor.
//! Application-specific actions and futures; no CLR task, reflection or stream runtime.
use std::{fmt, future::Future, time::Duration};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureError(pub String);
impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for CaptureError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureAction {
    ShowKeyTips,
    SelectTab(String),
    HideKeyTips,
    ActivateHomeHeader,
    Backstage,
    Simplified,
    Classic,
    Overflow,
    TabsOnly,
    OpenMinimized,
    FullScreen,
    Contextual,
    Gallery,
    ColorPicker,
    Paste,
    QatBelow,
    SearchFocus,
    SearchText,
    Palette,
    CustomizeRibbon,
    CustomizeQat,
    AddQat(String),
    CollapsedGroup,
    Touch,
    Compact,
    Colorful,
    SurfaceCad,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CadCaptureAction {
    SlideOut,
    ApplicationMenu,
    InvokeSaveAs,
    PanelButtons,
    DrawPopup,
    PanelTitles,
    LayersPopup,
    Float,
    Tooltip,
    Layers,
    Contextual,
    Hatch,
    Light,
    Command(String),
    Circle,
    Blocks,
    View,
    Workspace3d,
}
pub trait CaptureSurface {
    type Ribbon: Clone;
    type Cad: Clone;
    fn environment(&self, name: &str) -> Result<Option<String>, CaptureError>;
    fn wait(&self, duration: Duration) -> impl Future<Output = ()>;
    fn parse_integer(&self, value: Option<&str>) -> Option<i32>;
    fn resize(&self, width: i32, height: i32) -> Result<(), CaptureError>;
    fn dark(&self) -> Result<(), CaptureError>;
    fn show(&self, key: &str) -> Result<(), CaptureError>;
    fn find_ribbon(&self) -> Result<Option<Self::Ribbon>, CaptureError>;
    fn current_cad(&self) -> Result<Option<Self::Cad>, CaptureError>;
    fn action(&self, ribbon: &Self::Ribbon, action: CaptureAction) -> Result<(), CaptureError>;
    fn cad_action(
        &self,
        cad: &Self::Cad,
        ribbon: &Self::Ribbon,
        action: CadCaptureAction,
    ) -> Result<(), CaptureError>;
    fn action_failed(&self, error: &CaptureError);
    fn save(&self, path: &str) -> impl Future<Output = Result<(), CaptureError>>;
}
#[derive(Debug, Clone)]
pub struct CaptureRequest {
    pub path: String,
}
impl CaptureRequest {
    /// Read the path synchronously, as the source does before its first await.
    pub fn read(surface: &impl CaptureSurface) -> Result<Option<Self>, CaptureError> {
        Ok(surface
            .environment("RIBBONSPACE_CAPTURE")?
            .filter(|p| !p.is_empty())
            .map(|path| Self { path }))
    }
    pub async fn run(&self, surface: &impl CaptureSurface) -> Result<(), CaptureError> {
        surface.wait(Duration::from_millis(600)).await;
        let width = surface.environment("RIBBONSPACE_WIDTH")?;
        if let Some(width) = surface.parse_integer(width.as_deref()) {
            let height = surface.environment("RIBBONSPACE_HEIGHT")?;
            surface.resize(
                width,
                surface.parse_integer(height.as_deref()).unwrap_or(820),
            )?;
        }
        if surface.environment("RIBBONSPACE_THEME")?.as_deref() == Some("Dark") {
            surface.dark()?;
        }
        surface.wait(Duration::from_millis(400)).await;
        if let Some(page) = surface
            .environment("RIBBONSPACE_PAGE")?
            .filter(|p| !p.is_empty())
        {
            surface.show(&page)?;
        }
        surface.wait(Duration::from_millis(1400)).await;
        let ribbon = surface.find_ribbon()?;
        let actions = surface.environment("RIBBONSPACE_ACTION")?;
        if let Err(error) = run_actions(surface, actions.as_deref(), ribbon).await {
            surface.action_failed(&error);
        }
        surface.wait(Duration::from_millis(900)).await;
        surface.save(&self.path).await
    }
}
fn trim_step(value: &str) -> String {
    // Existing Core whitespace contract is UTF-16 based and source-tested.
    String::from_utf16(ferroribbon_core::text::trim(
        &value.encode_utf16().collect::<Vec<_>>(),
    ))
    .expect("valid UTF8 capture action")
}
pub async fn run_actions<S: CaptureSurface>(
    s: &S,
    actions: Option<&str>,
    ribbon: Option<S::Ribbon>,
) -> Result<(), CaptureError> {
    let Some(r) = ribbon else {
        return Ok(());
    };
    let Some(actions) = actions.filter(|a| !a.is_empty()) else {
        return Ok(());
    };
    for raw in actions.split(',') {
        let step = trim_step(raw);
        match step.as_str() {
            "keytips" => s.action(&r, CaptureAction::ShowKeyTips)?,
            "keytips-home" => {
                s.action(&r, CaptureAction::ShowKeyTips)?;
                s.wait(Duration::from_millis(300)).await;
                s.action(&r, CaptureAction::SelectTab("home".into()))?;
                s.action(&r, CaptureAction::HideKeyTips)?;
                s.action(&r, CaptureAction::ShowKeyTips)?;
                s.wait(Duration::from_millis(200)).await;
                s.action(&r, CaptureAction::ActivateHomeHeader)?;
            }
            "backstage" => s.action(&r, CaptureAction::Backstage)?,
            "simplified" => s.action(&r, CaptureAction::Simplified)?,
            "classic" => s.action(&r, CaptureAction::Classic)?,
            "overflow" => {
                s.wait(Duration::from_millis(400)).await;
                s.action(&r, CaptureAction::Overflow)?;
            }
            "minimized" => s.action(&r, CaptureAction::TabsOnly)?,
            "minimized-popup" => {
                s.action(&r, CaptureAction::TabsOnly)?;
                s.wait(Duration::from_millis(300)).await;
                s.action(&r, CaptureAction::OpenMinimized)?;
            }
            "fullscreen" => s.action(&r, CaptureAction::FullScreen)?,
            "contextual" => s.action(&r, CaptureAction::Contextual)?,
            "insert" | "design" | "layout" | "view" => {
                s.action(&r, CaptureAction::SelectTab(step.clone()))?
            }
            "gallery" => {
                s.wait(Duration::from_millis(300)).await;
                s.action(&r, CaptureAction::Gallery)?;
            }
            "colorpicker" => s.action(&r, CaptureAction::ColorPicker)?,
            "paste" => s.action(&r, CaptureAction::Paste)?,
            "qat-below" => s.action(&r, CaptureAction::QatBelow)?,
            "search" => {
                s.action(&r, CaptureAction::SearchFocus)?;
                s.wait(Duration::from_millis(200)).await;
                s.action(&r, CaptureAction::SearchText)?;
            }
            "palette" => s.action(&r, CaptureAction::Palette)?,
            "customize" => s.action(&r, CaptureAction::CustomizeRibbon)?,
            "customize-qat" => s.action(&r, CaptureAction::CustomizeQat)?,
            "add-qat" => {
                s.action(&r, CaptureAction::AddQat("bold".into()))?;
                s.action(&r, CaptureAction::AddQat("paste".into()))?;
            }
            "collapsed-group" => {
                s.wait(Duration::from_millis(300)).await;
                s.action(&r, CaptureAction::CollapsedGroup)?;
            }
            "touch" => s.action(&r, CaptureAction::Touch)?,
            "compact" => s.action(&r, CaptureAction::Compact)?,
            "colorful" => s.action(&r, CaptureAction::Colorful)?,
            "dark" => s.dark()?,
            "surface-cad" => s.action(&r, CaptureAction::SurfaceCad)?,
            key if key.starts_with("page:") => {
                s.show(&key[5..])?;
                s.wait(Duration::from_millis(800)).await;
            }
            _ => {
                if let Some(cad) = s.current_cad()? {
                    run_cad_action(s, &step, &cad, &r).await?;
                }
            }
        }
        s.wait(Duration::from_millis(500)).await;
    }
    Ok(())
}
async fn run_cad_action<S: CaptureSurface>(
    s: &S,
    step: &str,
    cad: &S::Cad,
    ribbon: &S::Ribbon,
) -> Result<(), CaptureError> {
    let action = match step {
        "cad-slideout" => CadCaptureAction::SlideOut,
        "cad-appmenu" => {
            s.cad_action(cad, ribbon, CadCaptureAction::ApplicationMenu)?;
            s.wait(Duration::from_millis(500)).await;
            CadCaptureAction::InvokeSaveAs
        }
        "cad-panelbuttons" => {
            s.cad_action(cad, ribbon, CadCaptureAction::PanelButtons)?;
            s.wait(Duration::from_millis(500)).await;
            CadCaptureAction::DrawPopup
        }
        "cad-paneltitles" => {
            s.cad_action(cad, ribbon, CadCaptureAction::PanelTitles)?;
            s.wait(Duration::from_millis(500)).await;
            CadCaptureAction::LayersPopup
        }
        "cad-float" => CadCaptureAction::Float,
        "cad-tooltip" => CadCaptureAction::Tooltip,
        "cad-layers" => CadCaptureAction::Layers,
        "cad-contextual" => CadCaptureAction::Contextual,
        "cad-hatch" => CadCaptureAction::Hatch,
        "cad-light" => CadCaptureAction::Light,
        command if command.starts_with("cad-cmd:") => {
            CadCaptureAction::Command(command[8..].to_owned())
        }
        "cad-circle" => CadCaptureAction::Circle,
        "cad-blocks" => CadCaptureAction::Blocks,
        "cad-view" => CadCaptureAction::View,
        "cad-3d" => CadCaptureAction::Workspace3d,
        _ => return Ok(()), // Unknown source steps are deliberately ignored, then delayed.
    };
    s.cad_action(cad, ribbon, action)
}
/// Source premultiplied BGRA overlay operation, including clipping, midpoint-even
/// rounding, forced alpha and lazy buffer errors with partial writes retained.
pub fn blend(
    target: &mut [u8],
    target_size: (i32, i32),
    source: &[u8],
    source_size: (i32, i32),
    offset: (i32, i32),
) -> Result<(), CaptureError> {
    let (tw, th) = target_size;
    let (sw, sh) = source_size;
    let (ox, oy) = offset;
    let read = |bytes: &[u8], index: i32| {
        bytes
            .get(index as usize)
            .copied()
            .ok_or_else(|| CaptureError("Pixel index out of range".into()))
    };
    for y in 0..sh {
        let ty = y.wrapping_add(oy);
        if ty < 0 || ty >= th {
            continue;
        }
        for x in 0..sw {
            let tx = x.wrapping_add(ox);
            if tx < 0 || tx >= tw {
                continue;
            }
            let si = y.wrapping_mul(sw).wrapping_add(x).wrapping_mul(4);
            let ti = ty.wrapping_mul(tw).wrapping_add(tx).wrapping_mul(4);
            let a = f64::from(read(source, si.wrapping_add(3))?) / 255.;
            if a <= 0. {
                continue;
            }
            for c in 0..3 {
                let sv = read(source, si.wrapping_add(c))?;
                let tv = read(target, ti.wrapping_add(c))?;
                let value =
                    (f64::from(sv) + f64::from(tv) * (1. - a)).round_ties_even() as i32 as u8;
                *target
                    .get_mut(ti.wrapping_add(c) as usize)
                    .ok_or_else(|| CaptureError("Pixel index out of range".into()))? = value;
            }
            *target
                .get_mut(ti.wrapping_add(3) as usize)
                .ok_or_else(|| CaptureError("Pixel index out of range".into()))? = 255;
        }
    }
    Ok(())
}

pub struct BgraFrame {
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
}
/// Capture-specific rendering boundary; concrete native visuals stay in the view.
pub trait CaptureRenderer {
    type Node: Clone;
    type Popup;
    fn root(&self) -> Result<Self::Node, CaptureError>;
    fn render(&self, node: &Self::Node) -> Result<BgraFrame, CaptureError>;
    fn actual_width(&self, node: &Self::Node) -> f64;
    fn popups(&self) -> Vec<Self::Popup>;
    fn child(&self, popup: &Self::Popup) -> Option<Self::Node>;
    fn candidates(&self, child: &Self::Node) -> [Option<Self::Node>; 3];
    fn origin(&self, target: &Self::Node, root: &Self::Node) -> Result<(f64, f64), CaptureError>;
    fn rendered(&self, child: &Self::Node, width: i32, height: i32);
    fn popup_failed(&self, error: &CaptureError);
}
/// Original popup-candidate loop and numeric composition, without a UI runtime.
pub fn compose<R: CaptureRenderer>(surface: &R) -> Result<BgraFrame, CaptureError> {
    let root = surface.root()?;
    let mut frame = surface.render(&root)?;
    // Source UIElement.ActualSize.X is a Single, not a Double.
    let actual_width = surface.actual_width(&root) as f32;
    let denominator = if actual_width < 1. { 1. } else { actual_width };
    let scale = f64::from(frame.width as f32 / denominator);
    for popup in surface.popups() {
        let Some(child) = surface.child(&popup).filter(|n| {
            let width = surface.actual_width(n);
            width.is_nan() || width >= 1.
        }) else {
            continue;
        };
        let result = (|| {
            let mut rendered = None;
            for node in surface.candidates(&child).into_iter().flatten() {
                let image = surface.render(&node)?;
                let width = image.width;
                rendered = Some((node, image));
                if width > 0 {
                    break;
                }
            }
            let (target, image) =
                rendered.ok_or_else(|| CaptureError("Popup has no render candidate".into()))?;
            surface.rendered(&child, image.width, image.height);
            let (x, y) = surface.origin(&target, &root)?;
            blend(
                &mut frame.pixels,
                (frame.width, frame.height),
                &image.pixels,
                (image.width, image.height),
                (
                    (x * scale).round_ties_even() as i32,
                    (y * scale).round_ties_even() as i32,
                ),
            )
        })();
        if let Err(error) = result {
            surface.popup_failed(&error);
        }
    }
    Ok(frame)
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
