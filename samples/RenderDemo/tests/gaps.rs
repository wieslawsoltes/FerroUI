//! The minimal reproductions of the gaps of the framework the sample found (`GAPS.md`): a
//! test of a gap that is fixed asserts what the framework does now.

use super::support::start_application_with_clock;
use crate::SAMPLE;
use ferroui_base::animation::TimeSpan;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::{Control, TextBlock, Window};
use sample_testing::documents::load_text;
use sample_testing::TestGlobalClock;
use std::rc::Rc;

/// R001: the attached property `Animation.Animator` gives the setter of a key frame a custom
/// animator (`Pages/CustomAnimatorPage.xaml`): markup resolves the property, and the
/// animation of the text interpolates with the animator of the sample.
#[test]
fn r001_a_setter_of_a_key_frame_takes_a_custom_animator() {
    let clock = Rc::new(TestGlobalClock::default());
    let _app = start_application_with_clock(clock.clone());
    let root = load_text(
        &SAMPLE,
        r#"<TextBlock xmlns="https://github.com/ferroui"
                      xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
                      xmlns:pages="using:RenderDemo.Pages">
             <TextBlock.Styles>
               <Style Selector="TextBlock">
                 <Style.Animations>
                   <Animation Duration="0:0:1" IterationCount="Infinite">
                     <KeyFrame Cue="0%">
                       <Setter Property="Text" Value="">
                         <Animation.Animator>
                           <pages:CustomStringAnimator/>
                         </Animation.Animator>
                       </Setter>
                     </KeyFrame>
                     <KeyFrame Cue="100%">
                       <Setter Property="Text" Value="0123456789">
                         <Animation.Animator>
                           <pages:CustomStringAnimator/>
                         </Animation.Animator>
                       </Setter>
                     </KeyFrame>
                   </Animation>
                 </Style.Animations>
               </Style>
             </TextBlock.Styles>
           </TextBlock>"#,
    );
    let text_block = from_markup_value::<Ref<TextBlock>>(&Some(root)).expect("a text block");
    let window = Window::new();
    window.set_content(Some(Control::boxed(&text_block)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    clock.pulse(TimeSpan::from_milliseconds(0.0));
    clock.pulse(TimeSpan::from_milliseconds(550.0));
    // A progress of 0.55 over ten characters: the first six.
    assert_eq!(text_block.text().as_deref(), Some("012345"));
    clock.pulse(TimeSpan::from_milliseconds(850.0));
    assert_eq!(text_block.text().as_deref(), Some("012345678"));
    window.close();
}
