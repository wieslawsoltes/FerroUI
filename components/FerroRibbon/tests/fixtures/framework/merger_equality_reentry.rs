//! Regression for a merge removal whose equality callback appends to the collection.
use ferroribbon_core::{ObservableObjectImpl, model::*};
use ferroui_system::{component_model::INotifyPropertyChanged, events::Event};
use std::{
    any::Any,
    rc::{Rc, Weak},
};
struct MutatingItem {
    base: RibbonButtonModel,
    owner: Weak<RibbonModel>,
}
impl INotifyPropertyChanged for MutatingItem {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for MutatingItem {}
impl RibbonNodeModelImpl for MutatingItem {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
    fn equals_node(&self, other: &dyn RibbonNodeModelImpl) -> bool {
        self.owner
            .upgrade()
            .unwrap()
            .tab_strip_items()
            .add(Some(RibbonButtonModel::new(
                Some("side".into()),
                Some("side".into()),
            )));
        (other as &dyn Any).is::<Self>()
    }
}
impl RibbonItemModelImpl for MutatingItem {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonModelImpl for MutatingItem {
    fn button_model(&self) -> &RibbonButtonModel {
        &self.base
    }
}
#[test]
fn merge_remove_permits_equals_to_append_before_removing_the_match() {
    register_ribbon_button_model::<MutatingItem>().unwrap();
    let host = RibbonModel::new();
    let source = RibbonModel::new();
    host.tab_strip_items().add(Some(Rc::new(MutatingItem {
        base: RibbonButtonModel::construct(Some("id".into()), Some("matched".into())),
        owner: Rc::downgrade(&host),
    })));
    let remove = RibbonButtonModel::new(Some("id".into()), Some("remove".into()));
    remove.set_merge_action(ferroribbon_core::RibbonMergeAction::Remove);
    source.tab_strip_items().add(Some(remove));
    let merger = RibbonModelMerger::merge(Some(host.clone()), Some(source));
    // Frozen C# MergerCorpus.collectionGap: merged=["side"], restored=["id","side"].
    let ids = || {
        host.tab_strip_items()
            .iter()
            .map(|v| v.unwrap().id().unwrap().to_utf8().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(), ["side"]);
    merger.unmerge();
    assert_eq!(ids(), ["id", "side"]);
}
