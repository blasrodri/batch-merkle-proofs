use near_primitives::{
    hash::CryptoHash,
    merkle::{Direction, MerklePath, MerklePathItem},
};

#[derive(Debug, PartialEq, Eq)]
pub struct NodeCoordinates {
    index: usize,
    level: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MerklePathWrapper {
    inner: MerklePath,
}

impl MerklePathWrapper {
    pub fn calculate_root_hash(&self, item: CryptoHash) {}
    pub fn get_coodinates_nodes(&self) -> (Vec<NodeCoordinates>, Vec<NodeCoordinates>) {
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
                            });
                            if depth == tree_depth {
                                node_coordinates_given.push(NodeCoordinates {
                                    index: idx_given ^ 1,
                                    level: depth,
                                });
                            } else {
                                node_coordinates_to_calculate.push(NodeCoordinates {
                                    index: idx_to_calculate,
                                    level: depth,
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
                            });
                            node_coordinates_given.push(NodeCoordinates {
                                index: idx_given ^ 1,
                                level: depth,
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
                            });
                            node_coordinates_to_calculate.push(NodeCoordinates {
                                index: idx_to_calculate,
                                level: depth,
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
                        NodeCoordinates { index: 0, level: 1 },
                        NodeCoordinates { index: 1, level: 1 },
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
                        NodeCoordinates { index: 1, level: 1 },
                        NodeCoordinates { index: 0, level: 2 },
                        NodeCoordinates { index: 1, level: 2 },
                    ],
                    node_coordinates_to_calculate: vec![NodeCoordinates { index: 0, level: 1 }],
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
                        NodeCoordinates { index: 1, level: 1 },
                        NodeCoordinates { index: 0, level: 2 },
                        NodeCoordinates { index: 1, level: 2 },
                    ],
                    node_coordinates_to_calculate: vec![NodeCoordinates { index: 0, level: 1 }],
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
                        NodeCoordinates { index: 0, level: 1 },
                        NodeCoordinates { index: 2, level: 2 },
                        NodeCoordinates { index: 3, level: 2 },
                    ],
                    node_coordinates_to_calculate: vec![NodeCoordinates { index: 1, level: 1 }],
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
                        NodeCoordinates { index: 0, level: 1 },
                        NodeCoordinates { index: 3, level: 2 },
                        NodeCoordinates { index: 4, level: 3 },
                        NodeCoordinates { index: 5, level: 3 },
                    ],
                    node_coordinates_to_calculate: vec![
                        NodeCoordinates { index: 1, level: 1 },
                        NodeCoordinates { index: 2, level: 2 },
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
                        NodeCoordinates { index: 1, level: 1 },
                        NodeCoordinates { index: 1, level: 2 },
                        NodeCoordinates { index: 0, level: 3 },
                        NodeCoordinates { index: 1, level: 3 },
                    ],
                    node_coordinates_to_calculate: vec![
                        NodeCoordinates { index: 0, level: 1 },
                        NodeCoordinates { index: 0, level: 2 },
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
                        NodeCoordinates { index: 1, level: 1 },
                        NodeCoordinates { index: 1, level: 2 },
                        NodeCoordinates { index: 0, level: 3 },
                        NodeCoordinates { index: 1, level: 3 },
                    ],
                    node_coordinates_to_calculate: vec![
                        NodeCoordinates { index: 0, level: 1 },
                        NodeCoordinates { index: 0, level: 2 },
                    ],
                }),
            ),
        ];

        for (mp, expected_result) in cases {
            assert_eq!(mp.get_coodinates_nodes(), expected_result.into());
        }
    }
}
