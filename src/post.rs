use crate::Config;
use eth2::types::{BlindedBeaconBlock, EthSpec, Slot};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
pub struct PostEndpoint {
    client: Client,
    url: String,
    persistence_dir: Option<PathBuf>,
    compare_rewards: bool,
    require_all: bool,
    require_same_parent: bool,
    extra_data: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound = "E: EthSpec")]
pub struct PostPayload<E: EthSpec> {
    names: Vec<String>,
    labels: Vec<String>,
    blocks: Vec<BlindedBeaconBlock<E>>,
}

impl PostEndpoint {
    pub fn new(config: &Config) -> Option<Arc<Self>> {
        let client = Client::new();
        let url = config.post_endpoint.clone()?;
        Some(Arc::new(Self {
            client,
            url,
            persistence_dir: config.post_results_dir.clone(),
            compare_rewards: config.compare_rewards,
            require_all: config.post_require_all,
            require_same_parent: config.post_require_same_parent,
            extra_data: config.post_extra_data,
        }))
    }

    pub async fn post_blocks<E: EthSpec>(
        &self,
        names_and_labels: Vec<(String, String)>,
        blocks: Vec<BlindedBeaconBlock<E>>,
        slot: Slot,
    ) -> Result<(), String> {
        if self.require_all && names_and_labels.len() != blocks.len() {
            return Err(format!(
                "only got {}/{} blocks",
                blocks.len(),
                names_and_labels.len()
            ));
        }

        if self.require_same_parent
            && !blocks
                .iter()
                .all(|block| block.parent_root() == blocks[0].parent_root())
        {
            return Err(format!("not all blocks build on the same parent"));
        }

        let response = if self.extra_data {
            let (names, labels) = names_and_labels.iter().cloned().unzip();
            let payload = PostPayload {
                names,
                labels,
                blocks,
            };

            self.client.post(&self.url).json(&payload)
        } else {
            self.client.post(&self.url).json(&blocks)
        }
        .send()
        .await
        .map_err(|e| format!("POST error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "POST failed: {}",
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "<body garbled>".into())
            ));
        }

        let response_json: Vec<Value> = response
            .json()
            .await
            .map_err(|e| format!("invalid JSON from POST endpoint: {}", e))?;

        let mut max_reward = 0;
        let mut max_reward_nodes = vec![];

        for ((name, label), result) in names_and_labels.into_iter().zip(response_json) {
            if self.compare_rewards {
                let reward = result["attestation_rewards"]["total"].as_u64().unwrap();
                println!("reward from {name}: {reward} gwei");

                if reward > max_reward {
                    max_reward = reward;
                    max_reward_nodes = vec![name.clone()];
                } else if reward == max_reward {
                    max_reward_nodes.push(name.clone());
                }
            }

            if let Some(persistence_dir) = &self.persistence_dir {
                // Store results by client label (same format as blockprint training data).
                let label_dir = persistence_dir.join(label);
                create_dir_all(&label_dir)
                    .await
                    .map_err(|e| format!("unable to create {}: {}", label_dir.display(), e))?;

                // Name files by node name and slot.
                let result_path = label_dir.join(format!("{name}_{slot}.json"));
                let mut f = File::create(&result_path)
                    .await
                    .map_err(|e| format!("unable to create {}: {}", result_path.display(), e))?;

                let bytes =
                    serde_json::to_vec(&result).map_err(|e| format!("JSON error: {}", e))?;
                f.write_all(&bytes)
                    .await
                    .map_err(|e| format!("unable to write {}: {}", result_path.display(), e))?;
            }
        }

        if self.compare_rewards {
            println!("most profitable block from {max_reward_nodes:?}");
        }

        Ok(())
    }
}
