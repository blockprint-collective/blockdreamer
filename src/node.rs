use crate::config::Node as NodeConfig;
use eth2::{
    types::{BeaconBlock, EthSpec, Signature, SkipRandaoVerification, Slot},
    BeaconNodeHttpClient, Timeouts,
};
use sensitive_url::SensitiveUrl;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct Node {
    pub config: Arc<NodeConfig>,
    pub client: BeaconNodeHttpClient,
}

impl Node {
    pub fn new(config: Arc<NodeConfig>) -> Result<Self, String> {
        let url = SensitiveUrl::parse(&config.url).map_err(|e| format!("Invalid URL: {:?}", e))?;
        let client = BeaconNodeHttpClient::new(url, Timeouts::set_all(Duration::from_secs(6)));
        Ok(Self { config, client })
    }

    pub async fn get_block<E: EthSpec>(&self, slot: Slot) -> Result<BeaconBlock<E>, String> {
        let randao_reveal = Signature::infinity().unwrap().into();
        self.client
            .get_validator_blocks_modular::<E, _>(
                slot,
                &randao_reveal,
                None,
                if self.config.skip_randao_verification {
                    SkipRandaoVerification::Yes
                } else {
                    SkipRandaoVerification::No
                },
            )
            .await
            .map(|res| res.data)
            .map_err(|e| format!("Error fetching block from {}: {:?}", self.config.url, e))
    }
}
