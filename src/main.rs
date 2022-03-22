use crate::distance::Distance;
use config::Config;
use eth2::types::{BeaconBlock, MainnetEthSpec};
use eth2_network_config::Eth2NetworkConfig;
use futures::future::join_all;
use node::Node;
use slot_clock::{SlotClock, SystemTimeSlotClock};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

mod config;
mod distance;
mod node;
mod tests;

type E = MainnetEthSpec;

// FIXME: add to config
const VERBOSE: bool = false;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    run().await.unwrap();
}

async fn run() -> Result<(), String> {
    // Load config.
    let config = Config::from_file(Path::new("config.toml")).unwrap();
    eprintln!("{:#?}", config);

    // Get network config and slot clock.
    let network_config = Eth2NetworkConfig::constant(&config.network)?
        .ok_or_else(|| format!("Unknown network `{}`", config.network))?;
    let spec = network_config.chain_spec::<E>()?;
    let genesis_state = network_config.beacon_state::<E>()?;
    let slot_clock = SystemTimeSlotClock::new(
        spec.genesis_slot,
        Duration::from_secs(genesis_state.genesis_time()),
        Duration::from_secs(spec.seconds_per_slot),
    );

    // Establish connections to beacon nodes.
    let nodes = config
        .nodes
        .into_iter()
        .map(|config| Node::new(config))
        .collect::<Result<Vec<_>, String>>()?;

    // Main loop.
    let mut all_blocks = HashMap::new();

    loop {
        let wait = slot_clock.duration_to_next_slot().expect("post genesis");
        tokio::time::sleep(wait).await;

        let slot = slot_clock.now().unwrap();

        // Dispatch requests in parallel to all dreaming nodes.
        let handles = nodes
            .iter()
            .map(move |node| {
                let inner = node.clone();
                tokio::spawn(async move { inner.get_block::<E>(slot).await })
            })
            .collect::<Vec<_>>();

        for (result, node) in join_all(handles).await.into_iter().zip(&nodes) {
            let name = node.config.name.clone();

            let block = match result.map_err(|e| format!("Task panicked: {:?}", e))? {
                Ok(block) => block,
                Err(e) => {
                    eprintln!("{} failed to produce a block: {}", name, e);
                    continue;
                }
            };
            eprintln!(
                "slot {}: block from {} with {} attestations",
                slot,
                name,
                block.body().attestations().len()
            );

            all_blocks
                .entry(slot)
                .or_insert_with(HashMap::new)
                .insert(node.config.name.clone(), block);
        }

        if let Some(blocks) = all_blocks.get(&slot) {
            for (name1, block1) in blocks {
                for (name2, block2) in blocks {
                    // Use lexicographic name ordering to establish order.
                    if name1 >= name2 {
                        continue;
                    }

                    let delta = block1.delta(block2).unwrap();
                    if VERBOSE {
                        eprintln!("{}-{} delta: {:#?}", name1, name2, delta);
                    }
                    eprintln!(
                        "{}-{} distance: {}",
                        name1,
                        name2,
                        BeaconBlock::<E>::delta_to_distance(&delta)
                    );
                }
            }
        }
    }
}
