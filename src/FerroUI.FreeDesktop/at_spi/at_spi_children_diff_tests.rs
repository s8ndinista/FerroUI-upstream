//! The tests of `AtSpiChildrenDiffTests.cs` (a test file of the unit tests of the controls
//! upstream, to which the AT-SPI project makes its internals visible; here they are tests of the
//! crate that has the module).

use super::at_spi_node::AtSpiNode;

type Changes = Vec<(usize, &'static str)>;

fn diff(old_children: &[&'static str], new_children: &[&'static str]) -> (Changes, Changes) {
    let mut removed = Vec::new();
    let mut added = Vec::new();
    AtSpiNode::diff_children(old_children, new_children, |child| *child, &mut removed, &mut added);
    (removed, added)
}

#[test]
fn replaced_child_is_removed_then_added_at_index_0() {
    let (removed, added) = diff(&["old pane"], &["new pane"]);

    assert_eq!(vec![(0, "old pane")], removed);
    assert_eq!(vec![(0, "new pane")], added);
}

#[test]
fn unchanged_children_report_nothing() {
    let (removed, added) = diff(&["a", "b", "c"], &["a", "b", "c"]);

    assert!(removed.is_empty());
    assert!(added.is_empty());
}

#[test]
fn removed_children_use_old_indices_highest_first() {
    let (removed, added) = diff(&["a", "b", "c", "d"], &["a", "c"]);

    assert_eq!(vec![(3, "d"), (1, "b")], removed);
    assert!(added.is_empty());
}

#[test]
fn added_children_use_new_indices_lowest_first() {
    let (removed, added) = diff(&["b"], &["a", "b", "c"]);

    assert!(removed.is_empty());
    assert_eq!(vec![(0, "a"), (2, "c")], added);
}

#[test]
fn first_children_are_all_added() {
    let (removed, added) = diff(&[], &["a", "b"]);

    assert!(removed.is_empty());
    assert_eq!(vec![(0, "a"), (1, "b")], added);
}

#[test]
fn moved_child_is_removed_and_added() {
    let (removed, added) = diff(&["a", "b", "c"], &["c", "a", "b"]);

    assert_eq!(vec![(2, "c")], removed);
    assert_eq!(vec![(0, "c")], added);
}

fn applying_changes_in_order_produces_new_children(old_children: &[&'static str], new_children: &[&'static str]) {
    let (removed, added) = diff(old_children, new_children);
    let mut children: Vec<&'static str> = old_children.to_vec();

    for (index, child) in removed {
        assert_eq!(child, children[index]);
        children.remove(index);
    }

    for (index, child) in added {
        children.insert(index, child);
    }

    assert_eq!(new_children, children.as_slice());
}

#[test]
fn applying_changes_in_order_produces_new_children_rows() {
    applying_changes_in_order_produces_new_children(&["a", "b", "c"], &["c", "b", "a"]);
    applying_changes_in_order_produces_new_children(&["a", "b", "c", "d"], &["x", "b", "y", "d", "a"]);
    applying_changes_in_order_produces_new_children(&["a", "b"], &[]);
    applying_changes_in_order_produces_new_children(&["a", "b", "c"], &["b", "d", "a", "c", "e"]);
}
