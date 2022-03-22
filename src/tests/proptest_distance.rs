use crate::distance::Distance;
use eth2::types::{
    AggregateSignature, Attestation, AttestationData, BitList, Checkpoint, EthSpec, Hash256,
    MainnetEthSpec, Slot, Unsigned,
};
use proptest::prelude::*;

const MAX_SLOT: u64 = 8;
const MAX_SOURCE_LOOKBACK: u64 = MAX_SLOT;
const MAX_COMMITTEE_INDEX: u64 = 8;
const MAX_HASH256: u64 = 4;
const MAX_ATTESTATIONS: usize = 128;

type E = MainnetEthSpec;
type N = <E as EthSpec>::MaxValidatorsPerCommittee;

fn small_hash256() -> impl Strategy<Value = Hash256> {
    (0..MAX_HASH256).prop_map(Hash256::from_low_u64_be)
}

fn arb_checkpoint(slot: Slot) -> impl Strategy<Value = Checkpoint> {
    let epoch = slot.epoch(E::slots_per_epoch());
    small_hash256().prop_map(move |root| Checkpoint { epoch, root })
}

fn arb_attestation_data() -> impl Strategy<Value = AttestationData> {
    (
        ((0..MAX_SLOT), (0..MAX_SOURCE_LOOKBACK)).prop_flat_map(|(slot, lookback)| {
            let slot = Slot::new(slot);

            let source = arb_checkpoint(slot - lookback);
            let target = arb_checkpoint(slot);

            (Just(slot), source, target)
        }),
        (0..MAX_COMMITTEE_INDEX),
        small_hash256(),
    )
        .prop_map(
            |((slot, source, target), index, beacon_block_root)| AttestationData {
                slot,
                index,
                beacon_block_root,
                source,
                target,
            },
        )
}

fn arb_aggregation_bits() -> impl Strategy<Value = BitList<N>> {
    // Last byte of the bitfield needs at least one bit set for the length bit.
    let last_byte = proptest::bits::u8::sampled(1..8, 0..8);
    let max_bytes = N::to_usize() / 8;
    let leading_bytes =
        proptest::collection::vec(proptest::bits::u8::sampled(0..8, 0..8), 0..max_bytes - 1);

    (leading_bytes, last_byte).prop_map(|(leading_bytes, last_byte)| {
        let mut vec = leading_bytes;
        vec.push(last_byte);
        BitList::from_bytes(vec.into()).expect("valid bitfield by construction")
    })
}

fn arb_attestation() -> impl Strategy<Value = Attestation<E>> {
    (arb_aggregation_bits(), arb_attestation_data()).prop_map(|(aggregation_bits, data)| {
        Attestation {
            aggregation_bits,
            data,
            signature: AggregateSignature::empty(),
        }
    })
}

fn arb_attestations() -> impl Strategy<Value = Vec<Attestation<E>>> {
    proptest::collection::vec(arb_attestation(), 0..MAX_ATTESTATIONS)
}

// Test that the distance function is a metric:
//
// https://en.wikipedia.org/wiki/Metric_(mathematics)#Definition
proptest! {
    #[test]
    fn distance_symmetry_and_identity(
        atts1 in arb_attestations(),
        atts2 in arb_attestations(),
    ) {
        // Symmetry.
        let distance = atts1.as_slice().distance(&atts2.as_slice())
            .expect("distance is always defined");
        let distance_rev = atts2.as_slice().distance(&atts1.as_slice())
            .expect("distance is always defined");
        assert_eq!(distance, distance_rev);

        // Identity of indiscernibles: `atts1 == atts2 <--> distance == 0`
        if atts1 == atts2 {
            assert_eq!(distance, 0);
        } else {
            assert_ne!(distance, 0);
        }
    }

    #[test]
    fn distance_triangle_inequality(
        x in arb_attestations(),
        y in arb_attestations(),
        z in arb_attestations()
    ) {
        let x_y = x.as_slice().distance(&y.as_slice()).unwrap();
        let y_z = y.as_slice().distance(&z.as_slice()).unwrap();
        let x_z = x.as_slice().distance(&z.as_slice()).unwrap();

        assert!(x_z <= x_y + y_z);
    }
}
