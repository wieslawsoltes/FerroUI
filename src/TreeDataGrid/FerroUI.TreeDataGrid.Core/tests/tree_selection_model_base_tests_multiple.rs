//! `TreeSelectionModelBaseTests_Multiple.cs`: the tree selection model in multiple selection mode.

use super::support::{single, ResettingCollection, TestCollection};
use crate::model_collection::{Models, ModelsChangedArgs};
use crate::selection::{
    IReadOnlyList, TreeSelectionModelBase, TreeSelectionModelHooks, TreeSelectionModelIndexesChangedEventArgs,
    TreeSelectionModelSelectionChangedEventArgsOf, TreeSelectionModelSourceResetEventArgs,
};
use crate::utils::{CollectionChangedEventManager, ICollectionChangedListener};
use crate::{index_path, IndexPath};
use ferroui_base::data::model::INotifyPropertyChanged;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

type Args = TreeSelectionModelSelectionChangedEventArgsOf<Node>;

mod selected_index {
    use super::*;

    #[test]
    fn can_set_selected_index() {
        let target = create_target(None);
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn can_set_grandchild_selected_index() {
        let data = create_data(3);
        let target = create_target_with(&data);
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![0, 0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 0, 2], target.selected_index());
        assert_eq!(index_path![0, 0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn setting_selected_index_clears_old_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 1]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 1], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-1", single(e.deselected_items().to_vec()).unwrap().caption());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn can_set_selected_index_to_empty() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 2], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.deselected_items().to_vec()).unwrap().caption());
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.set_selected_index(IndexPath::UNSELECTED);

        assert_eq!(1, raised.get());
        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
    }

    #[test]
    fn out_of_range_selected_index_clears_selection() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(index_path![0, 2], single(e.deselected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.deselected_items().to_vec()).unwrap().caption());
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![5, 10, 250]);

        assert_eq!(1, raised.get());
        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
    }

    #[test]
    fn can_select_unexpanded_item() {
        let target = create_target(None);
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![1, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 1-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.set_selected_index(index_path![1, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 2], target.selected_index());
        assert_eq!(index_path![1, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 1-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 1-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn setting_selected_index_during_collection_changed_results_in_correct_selection() {
        let data = TestCollection::<Node>::new();
        let target = create_target_with(&data);
        // The collection manager holds its listeners weakly: the binding is kept alive for the change
        // (the reference discards it and relies on it not being collected yet).
        let _binding = MockBinding::new(target.clone(), &data);

        data.add(Node::new(None));

        assert_eq!(IndexPath::new(0), target.selected_index());
    }

    #[test]
    fn property_changed_is_raised() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedIndex");

        target.set_selected_index(index_path![0, 2]);

        assert_eq!(1, raised.get());
    }

    struct MockBinding {
        target: Rc<TestTreeSelectionModel>,
    }

    impl MockBinding {
        fn new(target: Rc<TestTreeSelectionModel>, data: &Rc<TestCollection<Node>>) -> Rc<Self> {
            let this = Rc::new(Self { target });
            let listener: Weak<dyn ICollectionChangedListener<Node>> = Rc::downgrade(&this) as Weak<dyn ICollectionChangedListener<Node>>;
            CollectionChangedEventManager::add_listener(&(data.clone() as Models<Node>), listener);
            this
        }
    }

    impl ICollectionChangedListener<Node> for MockBinding {
        fn changed(&self, _collection: &Models<Node>, _e: &ModelsChangedArgs<'_, Node>) {
            self.target.select(&IndexPath::new(0));
        }

        fn post_changed(&self, _collection: &Models<Node>, _e: &ModelsChangedArgs<'_, Node>) {}

        fn pre_changed(&self, _collection: &Models<Node>, _e: &ModelsChangedArgs<'_, Node>) {}
    }
}

mod selected_item {
    use super::*;

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedItem");

        target.set_selected_index(IndexPath::new(1));

        assert_eq!(1, raised.get());
    }
}

mod selected_indexes {
    use super::*;

    #[test]
    fn can_get_items_via_indexer() {
        let target = create_target(None);

        target.select(&IndexPath::new(0));
        target.select(&IndexPath::new(1));
        target.select(&index_path![1, 2]);
        target.select(&index_path![2, 3]);

        assert_eq!(4, target.count());
        assert_eq!(4, target.selected_indexes().count());
        assert_eq!(IndexPath::new(0), target.selected_indexes().get(0));
        assert_eq!(IndexPath::new(1), target.selected_indexes().get(1));
        assert_eq!(index_path![1, 2], target.selected_indexes().get(2));
        assert_eq!(index_path![2, 3], target.selected_indexes().get(3));
    }

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedIndexes");

        target.set_selected_index(IndexPath::new(1));

        assert_eq!(1, raised.get());
    }
}

mod selected_items {
    use super::*;

    #[test]
    fn can_get_items_via_indexer() {
        let target = create_target(None);

        target.select(&IndexPath::new(0));
        target.select(&IndexPath::new(1));
        target.select(&index_path![1, 2]);
        target.select(&index_path![2, 3]);

        assert_eq!(4, target.count());
        assert_eq!(4, target.selected_items().count());
        assert_eq!("Node 0", target.selected_items().get(0).unwrap().caption());
        assert_eq!("Node 1", target.selected_items().get(1).unwrap().caption());
        assert_eq!("Node 1-2", target.selected_items().get(2).unwrap().caption());
        assert_eq!("Node 2-3", target.selected_items().get(3).unwrap().caption());
    }

    #[test]
    fn property_changed_is_raised_when_selected_index_changes() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SelectedItems");

        target.set_selected_index(IndexPath::new(1));

        assert_eq!(1, raised.get());
    }
}

mod select {
    use super::*;

    #[test]
    fn select_sets_selected_index_if_previously_unset() {
        let target = create_target(None);
        let raised = counter();

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.select(&index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn select_adds_to_selection() {
        let target = create_target(None);
        let raised = counter();

        target.select(&IndexPath::new(0));

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert!(e.deselected_items().to_vec().is_empty());
                assert_eq!(index_path![0, 2], single(e.selected_indexes().to_vec()));
                assert_eq!("Node 0-2", single(e.selected_items().to_vec()).unwrap().caption());
                bump(&raised);
            }));
        }

        target.select(&index_path![0, 2]);

        assert_eq!(1, raised.get());
        assert_eq!(2, target.count());
        assert_eq!(IndexPath::new(0), target.selected_index());
        assert_eq!(vec![IndexPath::new(0), index_path![0, 2]], target.selected_indexes().to_vec());
        assert_eq!("Node 0", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0", "Node 0-2"], captions(target.selected_items().to_vec()));
    }

    #[test]
    fn select_with_invalid_index_does_nothing() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |_e: &Args| bump(&raised)));
        }

        target.select(&index_path![5, 10, 250]);

        assert_eq!(0, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(index_path![0, 2], single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!("Node 0-2", single(target.selected_items().to_vec()).unwrap().caption());
    }

    #[test]
    fn selecting_already_selected_item_doesnt_raise_selection_changed() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(index_path![0, 2]);
        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |_e: &Args| bump(&raised)));
        }

        target.select(&index_path![0, 2]);

        assert_eq!(0, raised.get());
    }
}

mod deselect {
    use super::*;

    #[test]
    fn deselect_clears_selected_item() {
        let target = create_target(None);
        let raised = counter();

        target.set_selected_index(IndexPath::new(0));
        target.select(&index_path![0, 1]);

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(vec![index_path![0, 1]], e.deselected_indexes().to_vec());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.deselect(&index_path![0, 1]);

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(0), target.selected_index());
        assert_eq!(IndexPath::new(0), single(target.selected_indexes().to_vec()));
        assert_eq!("Node 0", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0"], captions(target.selected_items().to_vec()));
        assert_eq!(1, raised.get());
    }

    #[test]
    fn deselect_updates_selected_item_to_first_selected_item() {
        let target = create_target(None);

        target.select(&index_path![0, 2]);
        target.select(&index_path![0, 3]);
        target.select(&index_path![0, 4]);
        target.deselect(&index_path![0, 2]);

        assert_eq!(2, target.count());
        assert_eq!(index_path![0, 3], target.selected_index());
    }
}

mod clear {
    use super::*;

    #[test]
    fn clear_raises_selection_changed() {
        let target = create_target(None);
        let raised = counter();

        target.select(&index_path![0, 1]);
        target.select(&index_path![0, 2]);

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(vec![index_path![0, 1], index_path![0, 2]], e.deselected_indexes().to_vec());
                assert_eq!(vec!["Node 0-1", "Node 0-2"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.clear();

        assert_eq!(1, raised.get());
    }
}

mod anchor_index {
    use super::*;

    #[test]
    fn setting_selected_index_sets_anchor_index() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "AnchorIndex");

        target.set_selected_index(index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn setting_selected_index_to_empty_doesnt_clear_anchor_index() {
        let target = create_target(None);

        target.set_selected_index(index_path![0, 1]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.set_selected_index(IndexPath::UNSELECTED);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn select_sets_anchor_index() {
        let target = create_target(None);

        target.set_selected_index(index_path![0, 0]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.select(&index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn deselect_doesnt_clear_anchor_index() {
        let target = create_target(None);

        target.select(&index_path![0, 0]);
        target.select(&index_path![0, 1]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.deselect(&index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(0, raised.get());
    }
}

mod range_anchor_index {
    use super::*;

    #[test]
    fn setting_selected_index_sets_range_anchor_index() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "RangeAnchorIndex");

        target.set_selected_index(index_path![0, 1]);

        assert_eq!(index_path![0, 1], target.range_anchor_index());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn setting_selected_index_to_empty_doesnt_clear_range_anchor_index() {
        let target = create_target(None);

        target.set_selected_index(index_path![0, 1]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.set_selected_index(IndexPath::UNSELECTED);

        assert_eq!(index_path![0, 1], target.range_anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn select_doesnt_set_range_anchor_index() {
        let target = create_target(None);

        target.set_selected_index(index_path![0, 0]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.select(&index_path![0, 1]);

        assert_eq!(index_path![0, 0], target.range_anchor_index());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn deselect_doesnt_clear_range_anchor_index() {
        let target = create_target(None);

        target.set_selected_index(index_path![0, 0]);

        let raised = count_property_changes(&target, "AnchorIndex");

        target.deselect(&index_path![0, 0]);

        assert_eq!(index_path![0, 0], target.range_anchor_index());
        assert_eq!(0, raised.get());
    }
}

mod single_select {
    use super::*;

    #[test]
    fn converting_to_single_selection_removes_multiple_selection() {
        let target = create_target(None);
        let raised = counter();

        target.select(&index_path![0, 1]);
        target.select(&index_path![0, 2]);
        target.select(&index_path![0, 3]);

        {
            let raised = raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert_eq!(vec![index_path![0, 2], index_path![0, 3]], e.deselected_indexes().to_vec());
                assert_eq!(vec!["Node 0-2", "Node 0-3"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        target.set_single_select(true);

        assert_eq!(1, raised.get());
        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 1], target.selected_index());
        assert_eq!(vec![index_path![0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(1, raised.get());
    }

    #[test]
    fn raises_property_changed() {
        let target = create_target(None);
        let raised = count_property_changes(&target, "SingleSelect");

        target.set_single_select(true);

        assert_eq!(1, raised.get());
    }
}

mod collection_changes {
    use super::*;

    #[test]
    fn adding_root_item_before_selected_root_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(IndexPath::new(1));

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(2), target.selected_index());
        assert_eq!(vec![IndexPath::new(2)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(2), target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_child_item_before_selected_child_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![0, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::new(0), *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.get(0).children().insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 2], target.selected_index());
        assert_eq!(vec![index_path![0, 2]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![0, 2], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_root_item_before_selected_child_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![0, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 1], target.selected_index());
        assert_eq!(vec![index_path![1, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_root_item_before_selected_grandchild_item_updates_indexes() {
        let data = create_data(3);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![0, 0, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                assert_eq!(0, e.start_index());
                assert_eq!(1, e.delta());
                bump(&raised);
            }));
        }

        data.insert(0, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 0, 1], target.selected_index());
        assert_eq!(vec![index_path![1, 0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-0-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-0-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 0, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn adding_root_item_after_selected_root_item_doesnt_raise_events() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let raised = counter();

        target.set_selected_index(IndexPath::new(1));

        {
            let raised = raised.clone();
            target.property_changed().add(Rc::new(move |_name: &str| bump(&raised)));
        }
        count_selection_changes(&target, &raised);
        {
            let raised = raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }

        data.insert(2, Node::new(Some("new")));

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(1), target.selected_index());
        assert_eq!(vec![IndexPath::new(1)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(1), target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn removing_root_selected_item_updates_state() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();

        target.select(&IndexPath::new(1));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.remove_at(1);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn removing_child_selected_item_updates_state() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();

        target.select(&index_path![0, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.get(0).children().remove_at(1);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn removing_parent_of_selected_item_updates_state() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();

        target.select(&index_path![0, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.remove_at(0);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(IndexPath::UNSELECTED, target.anchor_index());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
    }

    #[test]
    fn removing_root_item_before_selected_root_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(IndexPath::new(1));

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }

        data.remove_at(0);

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(0), target.selected_index());
        assert_eq!(vec![IndexPath::new(0)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(0), target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn removing_root_item_before_selected_child_item_updates_indexes() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![1, 1]);

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }

        data.remove_at(0);

        assert_eq!(1, target.count());
        assert_eq!(index_path![0, 1], target.selected_index());
        assert_eq!(vec![index_path![0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 1-1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1-1"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn removing_child_item_before_selected_grandhild_item_updates_indexes() {
        let data = create_data(3);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.set_selected_index(index_path![1, 1, 2]);

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-1, e.delta());
                bump(&raised);
            }));
        }

        data.get(1).children().remove_at(0);

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 0, 2], target.selected_index());
        assert_eq!(vec![index_path![1, 0, 2]], target.selected_indexes().to_vec());
        assert_eq!("Node 1-1-2", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1-1-2"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 0, 2], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
    }

    #[test]
    fn removing_child_range_updates_state() {
        let data = create_data(3);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let indexes_changed_raised = counter();

        target.select(&index_path![0, 1]);
        target.select(&index_path![0, 2]);
        target.select(&index_path![0, 3]);

        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 0-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |e: &TreeSelectionModelIndexesChangedEventArgs| {
                assert_eq!(0, e.start_index());
                assert_eq!(-2, e.delta());
                bump(&raised);
            }));
        }

        data.get(0).children().remove_range(0, 2);

        assert_eq!(2, target.count());
        assert_eq!(index_path![0, 0], target.selected_index());
        assert_eq!(vec![index_path![0, 0], index_path![0, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 0-2", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 0-2", "Node 0-3"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![0, 1], target.anchor_index());
        assert_eq!(1, indexes_changed_raised.get());
        assert_eq!(1, selection_changed_raised.get());
    }

    #[test]
    fn removing_root_item_after_selected_root_item_doesnt_raise_events() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let raised = counter();

        target.set_selected_index(IndexPath::new(1));

        {
            let raised = raised.clone();
            target.property_changed().add(Rc::new(move |_name: &str| bump(&raised)));
        }
        count_selection_changes(&target, &raised);
        {
            let raised = raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }

        data.remove_at(2);

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(1), target.selected_index());
        assert_eq!(vec![IndexPath::new(1)], target.selected_indexes().to_vec());
        assert_eq!("Node 1", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(1), target.anchor_index());
        assert_eq!(0, raised.get());
    }

    #[test]
    fn replacing_selected_root_item_updates_state() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();

        target.select(&IndexPath::new(1));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.set(1, Node::new(Some("new")));

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn replacing_selected_child_item_updates_state() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();

        target.select(&index_path![1, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        {
            let raised = selection_changed_raised.clone();
            target.add_selection_changed(Rc::new(move |e: &Args| {
                assert!(e.deselected_indexes().to_vec().is_empty());
                assert_eq!(vec!["Node 1-1"], captions(e.deselected_items().to_vec()));
                assert!(e.selected_indexes().to_vec().is_empty());
                assert!(e.selected_items().to_vec().is_empty());
                bump(&raised);
            }));
        }

        data.get(1).children().set(1, Node::new(Some("new")));

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(1, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn resetting_root_items_clears_selection() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let source_reset_raised = counter();

        target.select(&IndexPath::new(1));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = source_reset_raised.clone();
            target.add_source_reset(Rc::new(move |e: &TreeSelectionModelSourceResetEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                bump(&raised);
            }));
        }

        data.clear();

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn resetting_child_items_clears_selection() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let source_reset_raised = counter();

        target.select(&index_path![1, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = source_reset_raised.clone();
            target.add_source_reset(Rc::new(move |e: &TreeSelectionModelSourceResetEventArgs| {
                assert_eq!(IndexPath::new(1), *e.parent_index());
                bump(&raised);
            }));
        }

        data.get(1).children().clear();

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn resetting_child_items_updates_selected_item_to_first_selected_item() {
        let data = create_data(2);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let source_reset_raised = counter();

        target.select(&index_path![1, 1]);
        target.select(&index_path![2, 1]);

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = source_reset_raised.clone();
            target.add_source_reset(Rc::new(move |e: &TreeSelectionModelSourceResetEventArgs| {
                assert_eq!(IndexPath::new(1), *e.parent_index());
                bump(&raised);
            }));
        }

        data.get(1).children().clear();

        assert_eq!(1, target.count());
        assert_eq!(index_path![2, 1], target.selected_index());
        assert_eq!(vec![index_path![2, 1]], target.selected_indexes().to_vec());
        assert_eq!("Node 2-1", target.selected_item().unwrap().caption());
        assert_eq!("Node 2-1", single(target.selected_items().to_vec()).unwrap().caption());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn resetting_root_items_to_non_empty_collection_clears_selection() {
        let data = ResettingCollection::new(create_data(2).to_vec());
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let source_reset_raised = counter();

        target.select(&IndexPath::new(0));

        let selected_index_raised = count_property_changes(&target, "SelectedIndex");
        let selected_item_raised = count_property_changes(&target, "SelectedItem");
        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = source_reset_raised.clone();
            target.add_source_reset(Rc::new(move |e: &TreeSelectionModelSourceResetEventArgs| {
                assert_eq!(IndexPath::UNSELECTED, *e.parent_index());
                bump(&raised);
            }));
        }

        data.reset(vec![data.get(0)]);

        assert_eq!(0, target.count());
        assert_eq!(IndexPath::UNSELECTED, target.selected_index());
        assert!(target.selected_indexes().to_vec().is_empty());
        assert!(target.selected_item().is_none());
        assert!(target.selected_items().to_vec().is_empty());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, selected_index_raised.get());
        assert_eq!(1, selected_item_raised.get());
    }

    #[test]
    fn doesnt_crash_on_removing_last_item_after_resetting_to_larger_collection() {
        let data = ResettingCollection::new(create_data(4).to_vec());
        let target = create_target_with(&data);

        target.select(&index_path![0, 1, 2]);
        let mut larger = data.to_vec();
        larger.push(Node::new(None));
        data.reset(larger);
        data.remove_at(data.count() - 1);
    }

    #[test]
    fn handles_selection_made_in_collection_changed() {
        // Tests the following scenario:
        //
        // - Items changes from empty to having 1 item
        // - ViewModel auto-selects item 0 in CollectionChanged
        // - SelectionModel receives CollectionChanged
        // - And so adjusts the selected item from 0 to 1, which is past the end of the items.
        //
        // There's not much we can do about this situation because the order in which
        // CollectionChanged handlers are called can't be known (the problem also exists with
        // WPF). The best we can do is not select an invalid index.
        let data = TestCollection::<Node>::new();
        let target = create_target_with(&data);

        {
            let target = target.clone();
            data.on_collection_changed(Rc::new(move |_e: &ModelsChangedArgs<'_, Node>| {
                target.select(&IndexPath::new(0));
            }));
        }

        data.add(Node::new(Some("foo")));

        assert_eq!(1, target.count());
        assert_eq!(IndexPath::new(0), target.selected_index());
        assert_eq!(vec![IndexPath::new(0)], target.selected_indexes().to_vec());
        assert_eq!("foo", target.selected_item().unwrap().caption());
        assert_eq!(vec!["foo"], captions(target.selected_items().to_vec()));
        assert_eq!(IndexPath::new(0), target.anchor_index());
    }

    #[test]
    fn clearing_node_selection_unsubscribes_from_collection_changed() {
        let data = create_data(2);
        let target = create_target_with(&data);

        target.select(&index_path![1, 1]);

        let debug = data.get(1).children();
        assert_eq!(1, debug.collection_changed_subscriber_count());

        target.deselect(&index_path![1, 1]);

        assert_eq!(0, debug.collection_changed_subscriber_count());
    }

    #[test]
    fn clearing_children_updates_state() {
        let data = create_data(4);
        let target = create_target_with(&data);
        let selection_changed_raised = counter();
        let source_reset_raised = counter();
        let indexes_changed_raised = counter();

        target.select(&index_path![0, 1]);
        target.select(&index_path![0, 1, 0]);
        target.select(&index_path![0, 1, 0, 1]);
        target.select(&index_path![0, 2]);
        target.select(&index_path![0, 3]);
        target.select(&index_path![1, 3]);

        count_selection_changes(&target, &selection_changed_raised);
        {
            let raised = source_reset_raised.clone();
            target.add_source_reset(Rc::new(move |_e: &TreeSelectionModelSourceResetEventArgs| bump(&raised)));
        }
        {
            let raised = indexes_changed_raised.clone();
            target.add_indexes_changed(Rc::new(move |_e: &TreeSelectionModelIndexesChangedEventArgs| bump(&raised)));
        }

        data.get(0).children().clear();

        assert_eq!(1, target.count());
        assert_eq!(index_path![1, 3], target.selected_index());
        assert_eq!(vec![index_path![1, 3]], target.selected_indexes().to_vec());
        assert_eq!("Node 1-3", target.selected_item().unwrap().caption());
        assert_eq!(vec!["Node 1-3"], captions(target.selected_items().to_vec()));
        assert_eq!(index_path![1, 3], target.anchor_index());
        assert_eq!(0, indexes_changed_raised.get());
        assert_eq!(0, selection_changed_raised.get());
        assert_eq!(1, source_reset_raised.get());
    }
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn bump(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

/// `target.PropertyChanged += (s, e) => { if (e.PropertyName == name) ++raised; }`.
fn count_property_changes(target: &TestTreeSelectionModel, name: &'static str) -> Rc<Cell<i32>> {
    let raised = counter();
    let r = raised.clone();
    target.property_changed().add(Rc::new(move |property: &str| {
        if property == name {
            bump(&r);
        }
    }));
    raised
}

/// `target.SelectionChanged += (s, e) => ++raised`.
fn count_selection_changes(target: &TestTreeSelectionModel, raised: &Rc<Cell<i32>>) {
    let raised = raised.clone();
    target.add_selection_changed(Rc::new(move |_e: &Args| bump(&raised)));
}

/// The captions of selected items (`items.Select(x => x?.Caption)`).
fn captions(items: Vec<Option<Rc<Node>>>) -> Vec<String> {
    items.into_iter().map(|x| x.map(|x| x.caption()).unwrap_or_default()).collect()
}

fn create_nodes(parent_id: &IndexPath, depth: i32) -> Rc<TestCollection<Node>> {
    let result = TestCollection::new();

    for i in 0..5 {
        let id = parent_id.append(i);
        let caption = format!("Node {}", id.to_vec().iter().map(|x| x.to_string()).collect::<Vec<_>>().join("-"));

        let node = Rc::new(Node { id, caption: Some(caption), children: RefCell::new(None), target_depth: depth });

        result.add(node);
    }

    result
}

fn create_data(depth: i32) -> Rc<TestCollection<Node>> {
    create_nodes(&IndexPath::UNSELECTED, depth)
}

fn create_target(data: Option<Models<Node>>) -> Rc<TestTreeSelectionModel> {
    let target = TestTreeSelectionModel::new(data.unwrap_or_else(|| create_data(2) as Models<Node>));
    target.set_single_select(false);
    target
}

fn create_target_with<C: crate::model_collection::ModelCollection<Node> + 'static>(data: &Rc<C>) -> Rc<TestTreeSelectionModel> {
    create_target(Some(data.clone() as Models<Node>))
}

struct Node {
    id: IndexPath,
    caption: Option<String>,
    children: RefCell<Option<Rc<TestCollection<Node>>>>,
    target_depth: i32,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    /// `new Node { Caption = caption }`.
    fn new(caption: Option<&str>) -> Rc<Self> {
        Rc::new(Self {
            id: IndexPath::UNSELECTED,
            caption: caption.map(str::to_string),
            children: RefCell::new(None),
            target_depth: 0,
        })
    }

    fn caption(&self) -> String {
        self.caption.clone().unwrap_or_default()
    }

    /// `Children!`.
    fn children(&self) -> Rc<TestCollection<Node>> {
        self.children.borrow().clone().expect("the children of the node")
    }
}

struct TestTreeSelectionModel {
    base: Rc<TreeSelectionModelBase<Node>>,
}

impl TestTreeSelectionModel {
    fn new(data: Models<Node>) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| {
            let base = TreeSelectionModelBase::with_source(Some(data));
            base.set_hooks(this.clone() as Weak<dyn TreeSelectionModelHooks<Node>>);
            Self { base }
        })
    }
}

impl Deref for TestTreeSelectionModel {
    type Target = TreeSelectionModelBase<Node>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl TreeSelectionModelHooks<Node> for TestTreeSelectionModel {
    fn get_children(&self, _model: &TreeSelectionModelBase<Node>, node: &Rc<Node>) -> Option<Models<Node>> {
        if node.children.borrow().is_none() && (node.id.count() as i32) < node.target_depth {
            *node.children.borrow_mut() = Some(create_nodes(&node.id, node.target_depth));
        }
        node.children.borrow().clone().map(|children| children as Models<Node>)
    }
}
