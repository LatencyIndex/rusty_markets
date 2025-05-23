use std::collections::VecDeque;

// Given two sorted lists, returns the first n elements of their union.
fn merge_sorted<T: Clone + Ord>(n: usize, l: &[T], r: &[T]) -> Vec<T> {
    let mut v = Vec::with_capacity(n);
    let mut i = 0;
    let mut j = 0;
    while v.len() < n {
        match (l.get(i), r.get(j)) {
            (Some(l), Some(r)) => {
                if l <= r {
                    v.push(l.clone());
                    i += 1;
                } else {
                    v.push(r.clone());
                    j += 1;
                }
            }
            (Some(l), None) => {
                v.push(l.clone());
                i += 1;
            }
            (None, Some(r)) => {
                v.push(r.clone());
                j += 1;
            }
            (None, None) => break,
        }
    }
    v
}

struct Family {
    sibling: usize,
    parent: usize,
}

struct Node<T> {
    family: Option<Family>,
    value: Vec<T>,
}

impl<T> Node<T> {
    fn new(k: usize) -> Self {
        Self {
            family: None,
            value: Vec::with_capacity(k),
        }
    }
}

pub struct KMinTree<T> {
    k: usize,
    nb_leaves: usize,
    nodes: Vec<Node<T>>,
}

impl<T: Clone + Ord> KMinTree<T> {
    /// Construct a new tree to track the min k values of the union of its leaves.
    pub fn new(k: usize, nb_leaves: usize) -> Self {
        // Fill level 0 nodes with leaves
        let mut nodes: Vec<Node<T>> = (0..nb_leaves).map(|_| Node::new(k)).collect();
        // Indexes of nodes without parents
        let mut orphans: VecDeque<usize> = (0..nb_leaves).collect();
        // Assign a parent to each pair of orphans, and place the parent into orphan queue.
        // Keep going until there is only one orphan left - the root node,
        // Or zero orphans - only possible if nb_leaves == 0.
        while let (Some(l), Some(r)) = (orphans.pop_front(), orphans.pop_front()) {
            // Create parent node
            let parent = nodes.len();
            nodes.push(Node::new(k));
            orphans.push_back(parent);
            // Link these adjacent orphans into a family
            nodes[l].family = Some(Family { sibling: r, parent });
            nodes[r].family = Some(Family { sibling: l, parent });
        }
        Self {
            k,
            nb_leaves,
            nodes,
        }
    }
    pub fn nb_leaves(&self) -> usize {
        self.nb_leaves
    }
    /// Set leaf to the new value, and return the new min k elements, if they have changed.
    /// Value does not have to be sorted.
    /// Runs in O(k*log(min(k, nb_leaves))) average time.
    /// Panics unless index < self.nb_leaves()
    pub fn update_leaf(&mut self, mut index: usize, mut value: Vec<T>) -> Option<&Vec<T>> {
        assert!(index < self.nb_leaves, "leaf index out of bounds");
        value.sort();
        value.truncate(self.k);
        // Propagate the changed value up the tree
        loop {
            if self.nodes[index].value == value {
                // Value did not change, so we can stop
                return None;
            } else {
                // Update current node's value
                self.nodes[index].value = value;
                if let Some(Family { sibling, parent }) = self.nodes[index].family {
                    // Derive what parent's value should be
                    value =
                        merge_sorted(self.k, &self.nodes[index].value, &self.nodes[sibling].value);
                    // Move to parent node
                    index = parent;
                } else {
                    // We just changed root's value, meaning some of the first n elements have changed,
                    // so we return the new values.
                    return Some(&self.nodes[index].value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn merge_sorted_test() {
        let a = [1, 3];
        let b = [2, 3, 4];
        let empty: [i32; 0] = [];
        assert_eq!(merge_sorted(0, &a, &b), empty);
        assert_eq!(merge_sorted(1, &a, &b), [1]);
        assert_eq!(merge_sorted(2, &a, &b), [1, 2]);
        assert_eq!(merge_sorted(3, &a, &b), [1, 2, 3]);
        assert_eq!(merge_sorted(4, &a, &b), [1, 2, 3, 3]);
        assert_eq!(merge_sorted(5, &a, &b), [1, 2, 3, 3, 4]);
        assert_eq!(merge_sorted(6, &a, &b), [1, 2, 3, 3, 4]);
    }

    #[test]
    fn first_n_test() {
        // The smallest n items in the collection. Slow & correct, for verification.
        fn smallest_n_simple<T: Ord + Clone>(n: usize, leaves: &[Vec<T>]) -> Vec<T> {
            let mut xs: Vec<T> = leaves.iter().flatten().cloned().collect();
            xs.sort();
            xs.truncate(n);
            xs
        }

        let mut rng = rand::rng();
        for k in 0..10 {
            for nb_leaves in 1..8 {
                let mut leaves: Vec<Vec<i32>> = vec![Vec::new(); nb_leaves];
                let mut tree: KMinTree<i32> = KMinTree::new(k, nb_leaves);
                for _ in 0..32 {
                    let first_before = smallest_n_simple(k, &leaves);
                    // Update a random leaf
                    let i: usize = rng.random_range(0..nb_leaves);
                    let len: usize = rng.random_range(0..=2 * k);
                    let v: Vec<i32> = (0..len).map(|_| rng.random()).collect();
                    leaves[i] = v.clone();
                    let first_after = smallest_n_simple(k, &leaves);

                    match tree.update_leaf(i, v) {
                        None => {
                            // First n have not changed
                            assert_eq!(first_before, first_after);
                        }
                        Some(first) => {
                            // First n have changed
                            assert_ne!(first_before, first_after);
                            assert_eq!(first, &first_after);
                        }
                    }
                }
            }
        }
    }
}
