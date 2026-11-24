//! The groups of a list view in XAML (#1258): `ListView.GroupStyle` takes `GroupStyle` elements.

use crate::support::app::xaml_test_base;
use crate::support::loader::load_as;
use ferroui_base::Ref;
use ferroui_controls::ListView;

#[test]
fn a_list_view_takes_its_group_styles_from_xaml() {
    let _app = xaml_test_base();
    let list = load_as::<Ref<ListView>>(
        r#"<ListView xmlns='https://github.com/ferroui'>
             <ListView.GroupStyle>
               <GroupStyle HidesIfEmpty='True' />
             </ListView.GroupStyle>
           </ListView>"#,
    );
    let styles = list.group_style();
    assert_eq!(styles.count(), 1);
    assert!(styles.get(0).hides_if_empty());
}
