//! Port of `Xaml/ShapeTests.cs`.

use ferroui_base::media::Pen;
use ferroui_base::Ref;

use crate::support::app::xaml_test_base;
use crate::support::loader::parse;

#[test]
fn can_specify_dash_style_in_xaml() {
    let _base = xaml_test_base();
    let xaml = "
<Pen xmlns='https://github.com/ferroui'>
    <Pen.DashStyle>
	    <DashStyle Offset='0' Dashes='1,3'/>
    </Pen.DashStyle>
</Pen>";

    // `Assert.NotNull(target)`: a parsed handle is never null.
    let _target: Ref<Pen> = parse::<Ref<Pen>>(xaml);
}
