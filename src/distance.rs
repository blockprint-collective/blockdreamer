use eth2::types::{Attestation, AttestationData, EthSpec};
use itertools::Itertools;
use pathfinding::{kuhn_munkres::kuhn_munkres_min, matrix::Matrix};
use std::collections::{HashMap, HashSet};

/// Cost of insertions and deletions (indels).
///
/// This is calibrated to equal the maximum possible `pos_distance`.
const INDEL_COST: usize = 128;

pub trait Distance {
    /// The type of intermediate data when computing the distance (mostly useful for diagnostics).
    type Delta;

    /// Distance between `self` and `other`, or `None` if incomparable.
    fn distance(&self, other: &Self) -> Option<usize> {
        self.delta(other).as_ref().map(Self::delta_to_distance)
    }

    /// Detailed delta between `self` and `other`, or `None` if incomparable.
    fn delta(&self, other: &Self) -> Option<Self::Delta>;

    /// Convert a delta for this type to a distance.
    fn delta_to_distance(delta: &Self::Delta) -> usize;

    /// Invert a delta converting it from left-right to right-left form.
    ///
    /// The default impl is a no-op (assumes delta has no handedness).
    fn invert_delta(delta: Self::Delta) -> Self::Delta {
        delta
    }
}

impl<E: EthSpec> Distance for Attestation<E> {
    type Delta = usize;

    fn delta(&self, other: &Self) -> Option<usize> {
        if self.data != other.data {
            return None;
        }
        let agg1_unique = self.aggregation_bits.difference(&other.aggregation_bits);
        let agg2_unique = other.aggregation_bits.difference(&self.aggregation_bits);
        Some(agg1_unique.num_set_bits() + agg2_unique.num_set_bits())
    }

    fn delta_to_distance(delta: &usize) -> usize {
        *delta
    }
}

type IndexMap<'a, E> = HashMap<AttestationData, Vec<(usize, &'a Attestation<E>)>>;

fn index_by_attestation_data<E: EthSpec>(atts: &[Attestation<E>]) -> IndexMap<E> {
    atts.iter()
        .enumerate()
        .into_group_map_by(|(_, att)| att.data.clone())
}

#[derive(Debug, Clone, Copy)]
pub enum Delta {
    /// Mutate an attestation on the `left` into `right` (and vice versa).
    ///
    /// This is a "perfect matching".
    Modify {
        /// The index of the attestation on the LHS.
        left: usize,
        /// The index of the closest matching attestation on the RHS.
        right: usize,
        /// The distance between `left` and `right`, i.e. `|left - right|`.
        pos_distance: usize,
        /// The distance between `left` and `right`'s attestations
        ///
        /// i.e. `atts1[left].distance(atts2[right])`
        bit_distance: usize,
    },
    /// Insert a new attestation on the left without matching it against any right attestation.
    InsertLeft { index: usize, num_set_bits: usize },
    /// Insert a new attestation on the right without matching it against any left attestation.
    InsertRight { index: usize, num_set_bits: usize },
}

impl Delta {
    fn total_distance(&self) -> usize {
        match self {
            Delta::Modify {
                pos_distance,
                bit_distance,
                ..
            } => *pos_distance + *bit_distance,
            Delta::InsertLeft { num_set_bits, .. } | Delta::InsertRight { num_set_bits, .. } => {
                *num_set_bits + INDEL_COST
            }
        }
    }
}

/// Compute `|x - y|`.
fn abs_diff(x: usize, y: usize) -> usize {
    let ix = isize::try_from(x).expect("x fits isize");
    let iy = isize::try_from(y).expect("y fits isize");
    ix.checked_sub(iy)
        .expect("no overflow")
        .abs()
        .try_into()
        .expect("abs value is positive")
}

fn compute_matching_att_deltas<E: EthSpec>(
    atts1: &[(usize, &Attestation<E>)],
    atts2: &[(usize, &Attestation<E>)],
) -> Vec<Delta> {
    // Create a matrix with one row for each member of `atts1` and one column
    // for each member of `atts2`.
    //
    // The weight of the edge is the distance between `att1` and `att2`.
    //
    // We make the matrix an n*n square by counting the distance of unmatched attestations
    // as their full weight plus the insertion/deletion cost.
    let n = std::cmp::max(atts1.len(), atts2.len());
    let dist_matrix = Matrix::from_rows((0..n).map(move |i| {
        (0..n)
            .map(move |j| {
                match (atts1.get(i), atts2.get(j)) {
                    // One side is out of bounds: this represents an insertion.
                    (Some((_, att)), None) | (None, Some((_, att))) => {
                        att.aggregation_bits.num_set_bits() + INDEL_COST
                    }
                    // Both sides are in bounds.
                    (Some((pos1, att1)), Some((pos2, att2))) => {
                        let pos_distance = abs_diff(*pos1, *pos2);
                        let bit_distance =
                            att1.distance(att2).expect("attestations are comparable");
                        pos_distance + bit_distance
                    }
                    // Neither side is in bounds.
                    (None, None) => unreachable!("at least one index must be less than slice len"),
                }
            })
            .map(|dist| dist as isize)
    }))
    .expect("matrix is valid by construction");

    assert!(dist_matrix.is_square());

    let (_, att1_to_att2_mapping) = kuhn_munkres_min(&dist_matrix);

    // Reconstruct the solution.
    let mut deltas = Vec::with_capacity(n);

    for (i, j) in att1_to_att2_mapping.into_iter().enumerate() {
        match (atts1.get(i), atts2.get(j)) {
            // Diff between two attestations, a modification.
            (Some((pos1, att1)), Some((pos2, att2))) => {
                let pos_distance = abs_diff(*pos1, *pos2);
                let bit_distance = att1.distance(att2).expect("attestations are comparable");

                deltas.push(Delta::Modify {
                    left: *pos1,
                    right: *pos2,
                    pos_distance,
                    bit_distance,
                });
            }
            // Insertion on the left.
            (Some((index, att)), None) => {
                deltas.push(Delta::InsertLeft {
                    index: *index,
                    num_set_bits: att.aggregation_bits.num_set_bits(),
                });
            }
            // Insertion on the right.
            (None, Some((index, att))) => {
                deltas.push(Delta::InsertRight {
                    index: *index,
                    num_set_bits: att.aggregation_bits.num_set_bits(),
                });
            }
            (None, None) => unreachable!("can't be out of bounds for both `atts1` and `atts2`"),
        }
    }

    deltas
}

fn sort_deltas(deltas: &mut Vec<Delta>) {
    // Sort by (left index, right index, handedness).
    deltas.sort_unstable_by_key(|delta| match delta {
        Delta::Modify { left, right, .. } => (*left, *right, 0u8),
        Delta::InsertLeft { index, .. } => (*index, *index, 0u8),
        Delta::InsertRight { index, .. } => (*index, *index, 1u8),
    });
}

impl<E: EthSpec> Distance for &[Attestation<E>] {
    type Delta = Vec<Delta>;

    fn delta(&self, other: &Self) -> Option<Self::Delta> {
        let left_index_map = index_by_attestation_data(self);
        let right_index_map = index_by_attestation_data(other);
        let empty = vec![];

        let mut deltas = Vec::with_capacity(std::cmp::max(self.len(), other.len()));

        let att_datas = left_index_map
            .keys()
            .chain(right_index_map.keys())
            .collect::<HashSet<_>>();

        for att_data in att_datas {
            let atts1 = left_index_map.get(att_data).unwrap_or(&empty);
            let atts2 = right_index_map.get(att_data).unwrap_or(&empty);
            assert!(!atts1.is_empty() || !atts2.is_empty());
            deltas.extend(compute_matching_att_deltas(atts1, atts2));
        }

        sort_deltas(&mut deltas);

        Some(deltas)
    }

    fn delta_to_distance(deltas: &Self::Delta) -> usize {
        deltas.iter().map(|delta| delta.total_distance()).sum()
    }

    fn invert_delta(mut deltas: Self::Delta) -> Self::Delta {
        for delta in &mut deltas {
            let new_delta = match *delta {
                Delta::InsertLeft {
                    index,
                    num_set_bits,
                } => Delta::InsertRight {
                    index,
                    num_set_bits,
                },
                Delta::InsertRight {
                    index,
                    num_set_bits,
                } => Delta::InsertLeft {
                    index,
                    num_set_bits,
                },
                Delta::Modify {
                    left,
                    right,
                    pos_distance,
                    bit_distance,
                } => Delta::Modify {
                    left: right,
                    right: left,
                    pos_distance,
                    bit_distance,
                },
            };
            *delta = new_delta;
        }
        sort_deltas(&mut deltas);

        deltas
    }
}
