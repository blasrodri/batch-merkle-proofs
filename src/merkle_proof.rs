use std::collections::{BTreeMap, HashMap};

use near_primitives::{
    hash::CryptoHash,
    merkle::{Direction, MerklePath},
};

type Level = usize;
type Index = usize;
type LeafIndex = usize;

#[derive(Debug, PartialEq, Eq)]
pub struct NodeCoordinates {
    index: usize,
    level: usize,
    hash: Option<CryptoHash>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MerklePathWrapper {
    inner: MerklePath,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CachedNodes {
    inner: BTreeMap<(Level, Index), CryptoHash>,
    path_item_cache_mapping: HashMap<LeafIndex, Vec<(Level, Index)>>,
}

impl CachedNodes {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
            path_item_cache_mapping: HashMap::new(),
        }
    }

    pub fn extend_from_given(&mut self, given_nodes: &[NodeCoordinates], leaf_index: LeafIndex) {
        if given_nodes.len() == 0 {
            return;
        }

        given_nodes.iter().for_each(|node| {
            let NodeCoordinates { index, level, hash } = node;
            if self.inner.get(&(*level, *index)).is_some() {
                return;
            }
            self.inner.insert((*level, *index), hash.unwrap());
            let e = self
                .path_item_cache_mapping
                .entry(leaf_index)
                .or_insert_with(Vec::new);
            e.push((*level, *index));
        });
    }
}

impl MerklePathWrapper {
    pub fn new(inner: MerklePath) -> Self {
        Self { inner }
    }
    pub fn calculate_root_hash(
        &self,
        item: CryptoHash,
        cached_nodes: &mut CachedNodes,
    ) -> CryptoHash {
        if self.inner.len() == 0 {
            return CryptoHash::default();
        }

        let mut current_depth = self.inner.len();
        let (_, node_coordinates_to_calculate) = self.get_node_coordinates();
        let nodes_to_calculate = node_coordinates_to_calculate.len();
        self.inner.iter().enumerate().fold(
            CryptoHash::default(),
            |mut hash, (item_idx, merkle_path_item)| {
                current_depth -= 1;
                match current_depth {
                    cd if cd == self.inner.len() - 1 => match merkle_path_item.direction {
                        Direction::Left => {
                            hash = CryptoHash::hash_borsh(&(hash, merkle_path_item.hash))
                        }
                        Direction::Right => {
                            hash = CryptoHash::hash_borsh(&(merkle_path_item.hash, hash))
                        }
                    },
                    _ => {
                        dbg!(&item_idx);
                        let NodeCoordinates { level, index, .. } =
                            &node_coordinates_to_calculate[nodes_to_calculate - item_idx];
                        if let Some(cached_hash) = cached_nodes.inner.get(&(*level, *index)) {
                            hash = *cached_hash;
                        } else {
                            match merkle_path_item.direction {
                                Direction::Left => {
                                    hash = CryptoHash::hash_borsh(&(hash, merkle_path_item.hash))
                                }
                                Direction::Right => {
                                    hash = CryptoHash::hash_borsh(&(merkle_path_item.hash, hash))
                                }
                            }
                            cached_nodes.inner.insert((*level, *index), hash);
                        }
                    }
                }
                hash
            },
        )
    }

    pub fn update_cache(&self, cache: &mut CachedNodes) {
        let (given_nodes, _) = self.get_node_coordinates();
        let leaf_index = given_nodes.last().unwrap().index;
        cache.extend_from_given(&given_nodes[0..(given_nodes.len() - 1)], leaf_index);
    }

    fn get_node_coordinates(&self) -> (Vec<NodeCoordinates>, Vec<NodeCoordinates>) {
        let tree_depth = self.inner.len();
        self.inner
            .iter()
            .rev()
            .fold(
                ((vec![], vec![]), 0, 0, 0),
                |(
                    (mut node_coordinates_given, mut node_coordinates_to_calculate),
                    mut depth,
                    mut idx_given,
                    mut idx_to_calculate,
                ),
                 el| {
                    depth += 1;
                    match depth {
                        1 => {
                            match el.direction {
                                Direction::Left => {
                                    idx_to_calculate = 1;
                                }
                                Direction::Right => {
                                    idx_given = 1;
                                    idx_to_calculate = 0;
                                }
                            }
                            // edge case depth == 1
                            node_coordinates_given.push(NodeCoordinates {
                                index: idx_given,
                                level: depth,
                                hash: Some(el.hash),
                            });
                            if depth == tree_depth {
                                node_coordinates_given.push(NodeCoordinates {
                                    index: idx_given ^ 1,
                                    level: depth,
                                    hash: Some(el.hash),
                                });
                            } else {
                                node_coordinates_to_calculate.push(NodeCoordinates {
                                    index: idx_to_calculate,
                                    level: depth,
                                    hash: None,
                                });
                            }
                        }
                        depth if depth == tree_depth => {
                            idx_to_calculate *= 2;
                            idx_given = idx_to_calculate;
                            // both nodes are given on the leaf level
                            node_coordinates_given.push(NodeCoordinates {
                                index: idx_given,
                                level: depth,
                                hash: Some(el.hash),
                            });
                            node_coordinates_given.push(NodeCoordinates {
                                index: idx_given ^ 1,
                                level: depth,
                                hash: Some(el.hash),
                            })
                        }
                        depth => {
                            // move to the children
                            idx_to_calculate *= 2;
                            idx_given = idx_to_calculate;
                            match el.direction {
                                Direction::Left => {
                                    idx_to_calculate ^= 1;
                                }
                                Direction::Right => {
                                    idx_given ^= 1;
                                }
                            }
                            node_coordinates_given.push(NodeCoordinates {
                                index: idx_given,
                                level: depth,
                                hash: Some(el.hash),
                            });
                            node_coordinates_to_calculate.push(NodeCoordinates {
                                index: idx_to_calculate,
                                level: depth,
                                hash: None,
                            });
                        }
                    };
                    (
                        (node_coordinates_given, node_coordinates_to_calculate),
                        depth,
                        idx_given,
                        idx_to_calculate,
                    )
                },
            )
            .0
    }
}

#[cfg(test)]
mod tests {
    use near_primitives::merkle::{compute_root_from_path_and_item, merklize, MerklePathItem};

    use super::*;

    struct ExpectedResult {
        node_coordinates_given: Vec<NodeCoordinates>,
        node_coordinates_to_calculate: Vec<NodeCoordinates>,
    }

    impl From<ExpectedResult> for (Vec<NodeCoordinates>, Vec<NodeCoordinates>) {
        fn from(e: ExpectedResult) -> Self {
            (e.node_coordinates_given, e.node_coordinates_to_calculate)
        }
    }

    #[test]
    fn test_get_nodes_to_be_calculated() {
        let cases = vec![
            (
                MerklePathWrapper {
                    inner: vec![MerklePathItem {
                        direction: Direction::Left,
                        hash: CryptoHash::default(),
                    }],
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 0,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![],
                }),
            ),
            (
                MerklePathWrapper {
                    inner: vec![
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Left,
                            hash: CryptoHash::default(),
                        },
                    ]
                    .into_iter()
                    .rev()
                    .collect(),
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 1,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 0,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![NodeCoordinates {
                        index: 0,
                        level: 1,
                        hash: None,
                    }],
                }),
            ),
            (
                MerklePathWrapper {
                    inner: vec![
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                    ]
                    .into_iter()
                    .rev()
                    .collect(),
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 1,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 0,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![NodeCoordinates {
                        index: 0,
                        level: 1,
                        hash: None,
                    }],
                }),
            ),
            (
                MerklePathWrapper {
                    inner: vec![
                        MerklePathItem {
                            direction: Direction::Left,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                    ]
                    .into_iter()
                    .rev()
                    .collect(),
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 0,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 2,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 3,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![NodeCoordinates {
                        index: 1,
                        level: 1,
                        hash: None,
                    }],
                }),
            ),
            (
                MerklePathWrapper {
                    inner: vec![
                        MerklePathItem {
                            direction: Direction::Left,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                    ]
                    .into_iter()
                    .rev()
                    .collect(),
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 0,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 3,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 4,
                            level: 3,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 5,
                            level: 3,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![
                        NodeCoordinates {
                            index: 1,
                            level: 1,
                            hash: None,
                        },
                        NodeCoordinates {
                            index: 2,
                            level: 2,
                            hash: None,
                        },
                    ],
                }),
            ),
            (
                MerklePathWrapper {
                    inner: vec![
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                    ]
                    .into_iter()
                    .rev()
                    .collect(),
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 1,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 0,
                            level: 3,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 3,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![
                        NodeCoordinates {
                            index: 0,
                            level: 1,
                            hash: None,
                        },
                        NodeCoordinates {
                            index: 0,
                            level: 2,
                            hash: None,
                        },
                    ],
                }),
            ),
            (
                MerklePathWrapper {
                    inner: vec![
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Right,
                            hash: CryptoHash::default(),
                        },
                        MerklePathItem {
                            direction: Direction::Left,
                            hash: CryptoHash::default(),
                        },
                    ]
                    .into_iter()
                    .rev()
                    .collect(),
                },
                (ExpectedResult {
                    node_coordinates_given: vec![
                        NodeCoordinates {
                            index: 1,
                            level: 1,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 2,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 0,
                            level: 3,
                            hash: Some(CryptoHash::default()),
                        },
                        NodeCoordinates {
                            index: 1,
                            level: 3,
                            hash: Some(CryptoHash::default()),
                        },
                    ],
                    node_coordinates_to_calculate: vec![
                        NodeCoordinates {
                            index: 0,
                            level: 1,
                            hash: None,
                        },
                        NodeCoordinates {
                            index: 0,
                            level: 2,
                            hash: None,
                        },
                    ],
                }),
            ),
        ];

        for (mp, expected_result) in cases {
            assert_eq!(mp.get_node_coordinates(), expected_result.into());
        }
    }

    #[test]
    fn test_merklelize() {
        let (root_hash, merkle_proofs) = merklize(&[1, 2, 3, 4, 5]);
        let mp = &merkle_proofs[0];
        let mp2 = &merkle_proofs[1];
        assert_eq!(compute_root_from_path_and_item(mp, &1), root_hash);
        assert_eq!(compute_root_from_path_and_item(mp2, &2), root_hash);

        let merkle_proof = merkle_proofs[0].clone();
        let wrapper = MerklePathWrapper::new(merkle_proof);
        dbg!(&wrapper);
        let mut cached_nodes = CachedNodes::new();
        wrapper.update_cache(&mut cached_nodes);

        dbg!(&cached_nodes);
        assert_eq!(
            wrapper.calculate_root_hash(CryptoHash::hash_borsh(&1), &mut cached_nodes),
            root_hash
        );
        dbg!(&cached_nodes);
    }
}
