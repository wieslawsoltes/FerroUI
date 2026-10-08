/// The names, units and descriptions of the meters of the framework (the
/// original's nested class `Diagnostic.Meters`).
pub struct DiagnosticMeters;

impl DiagnosticMeters {
    pub const SECONDS_UNIT: &'static str = "s";
    pub const MILLISECONDS_UNIT: &'static str = "ms";

    pub const COMPOSITOR_RENDER_PASS_NAME: &'static str = "ferroui.comp.render.time";
    pub const COMPOSITOR_RENDER_PASS_DESCRIPTION: &'static str = "Duration of the compositor render pass on render thread";
    pub const COMPOSITOR_UPDATE_PASS_NAME: &'static str = "ferroui.comp.update.time";
    pub const COMPOSITOR_UPDATE_PASS_DESCRIPTION: &'static str = "Duration of the compositor update pass on render thread";

    pub const LAYOUT_MEASURE_PASS_NAME: &'static str = "ferroui.ui.measure.time";
    pub const LAYOUT_MEASURE_PASS_DESCRIPTION: &'static str = "Duration of layout measurement pass on UI thread";
    pub const LAYOUT_ARRANGE_PASS_NAME: &'static str = "ferroui.ui.arrange.time";
    pub const LAYOUT_ARRANGE_PASS_DESCRIPTION: &'static str = "Duration of layout arrangement pass on UI thread";
    pub const LAYOUT_RENDER_PASS_NAME: &'static str = "ferroui.ui.render.time";
    pub const LAYOUT_RENDER_PASS_DESCRIPTION: &'static str = "Duration of render recording pass on UI thread";
    pub const LAYOUT_INPUT_PASS_NAME: &'static str = "ferroui.ui.input.time";
    pub const LAYOUT_INPUT_PASS_DESCRIPTION: &'static str = "Duration of input processing on UI thread";

    pub const TOTAL_EVENT_HANDLE_COUNT_NAME: &'static str = "ferroui.ui.event.handler.count";
    pub const TOTAL_EVENT_HANDLE_COUNT_DESCRIPTION: &'static str =
        "Number of event handlers currently registered in the application";
    pub const TOTAL_EVENT_HANDLE_COUNT_UNIT: &'static str = "{handler}";
    pub const TOTAL_VISUAL_COUNT_NAME: &'static str = "ferroui.ui.visual.count";
    pub const TOTAL_VISUAL_COUNT_DESCRIPTION: &'static str =
        "Number of visual elements currently present in the visual tree";
    pub const TOTAL_VISUAL_COUNT_UNIT: &'static str = "{visual}";
    pub const TOTAL_DISPATCHER_TIMER_COUNT_NAME: &'static str = "ferroui.ui.dispatcher.timer.count";
    pub const TOTAL_DISPATCHER_TIMER_COUNT_DESCRIPTION: &'static str =
        "Number of active dispatcher timers in the application";
    pub const TOTAL_DISPATCHER_TIMER_COUNT_UNIT: &'static str = "{timer}";
}

/// The names of the tags of the activities of the framework (the original's
/// nested class `Diagnostic.Tags`).
pub struct DiagnosticTags;

impl DiagnosticTags {
    pub const STYLE: &'static str = "Style";
    pub const SELECTOR_RESULT: &'static str = "SelectorResult";
    pub const KEY: &'static str = "Key";
    pub const THEME_VARIANT: &'static str = "ThemeVariant";
    pub const RESULT: &'static str = "Result";
    pub const ACTIVATOR: &'static str = "Activator";
    pub const IS_ACTIVE: &'static str = "IsActive";
    pub const SELECTOR: &'static str = "Selector";
    pub const CONTROL: &'static str = "Control";
    pub const ROUTED_EVENT: &'static str = "RoutedEvent";
}
