use eth2::types::{Attestation, BeaconBlock, EthSpec};

pub trait Distance {
    /// Distance between `self` and `other`, or `None` if incomparable.
    fn distance(&self, other: &Self) -> Option<usize>;
}

impl<E: EthSpec> Distance for Attestation<E> {
    fn distance(&self, other: &Self) -> Option<usize> {
        if self.data != other.data {
            return None;
        }
        let agg1_unique = self.aggregation_bits.difference(&other.aggregation_bits);
        let agg2_unique = other.aggregation_bits.difference(&self.aggregation_bits);
        Some(agg1_unique.num_set_bits() + agg2_unique.num_set_bits())
    }
}

impl<E: EthSpec> Distance for &[Attestation<E>] {
    fn distance(&self, other: &self) -> Option<usize> {
        // Transform any list of attestations into any other using these operations:
        // - Add attestation aggregation bit
        // - Remove attestatin aggregation bit
        // - Swap attestations
        // - Add attestation (with empty bitfield?)
        // - Delete attestation
        None
    }
}
